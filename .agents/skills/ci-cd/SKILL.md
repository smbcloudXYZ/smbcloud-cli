---
name: ci-cd
description: Use when creating, debugging, or improving GitHub Actions workflows for this repo — especially the crates.io release orchestrator and the downstream CLI and SDK release workflows for PyPI, npm, GitHub Releases, Homebrew tap, and NuGet. Also covers SDK distribution README branding alignment across npm, PyPI, and the main repo. Covers cargo-workspaces publish flow, release orchestration, OpenSSL vendoring, Windows vs non-Windows platform-conditional dependencies, manylinux containers, Perl module gaps, workspace scoping, native vs QEMU arm64 runners, trusted publishing, artifact actions, toolchain parity, idempotency patterns, Homebrew formula generation, and cross-workflow chaining.
---

# CI/CD

## Source of truth for release orchestration

| Role                                | Workflow file                            |
| ----------------------------------- | ---------------------------------------- |
| Workspace crates publish entrypoint | `.github/workflows/release-crate.yml`    |
| GitHub Release binaries             | `.github/workflows/release-github.yml`   |
| Homebrew tap update                 | `.github/workflows/release-homebrew.yml` |
| CLI npm release                     | `.github/workflows/release-npm.yml`      |
| CLI PyPI release                    | `.github/workflows/release-pypi.yml`     |
| CLI NuGet release                   | `.github/workflows/release-nuget.yml`    |
| SDK npm release                     | `.github/workflows/release-sdk-npm.yml`  |
| SDK PyPI release                    | `.github/workflows/release-sdk-pypi.yml` |
| SDK Ruby gem release                | `.github/workflows/release-sdk-gem.yml`  |

`release-crate.yml` is the top of the chain. It publishes Rust crates first, then fans out to the CLI distribution workflows, and then conditionally triggers SDK workflows when the published crate set says they need to run.

## Source of truth for distribution channel READMEs

| File                         | Published as                                      |
| ---------------------------- | ------------------------------------------------- |
| `README.MD`                  | GitHub repo — canonical brand voice               |
| `npm/smbcloud-cli/README.md` | npmjs.com — `@smbcloud/cli`                       |
| `pypi/README.md`             | pypi.org — `smbcloud-cli`                         |
| `npm/README.md.tmpl`         | npmjs.com — `@smbcloud/cli-*` (platform packages) |

When the main `README.MD` changes its tagline, logo, quick start, or value proposition, update all three distribution READMEs to match.

---

## Universal workflow patterns

The repo now has two kinds of release workflows.

### Trigger shape

**Orchestrator workflow:**

`release-crate.yml` is the only release workflow that should run directly from a tag push.

```yaml
on:
  push:
    tags:
      - "v*.*.*"
  workflow_dispatch:
    inputs:
      tag:
        description: "Release tag (e.g. v0.3.35)"
        required: true
```

**Downstream workflows:**

Everything else in the release chain should be `workflow_dispatch` only. The orchestrator passes the tag in explicitly.

```yaml
on:
  workflow_dispatch:
    inputs:
      tag:
        description: "Release tag (e.g. v0.3.35)"
        required: true
```

### Checkout — always include the ref override

```yaml
- name: Checkout
  uses: actions/checkout@v6
  with:
    ref: ${{ github.event.inputs.tag || github.ref }}
```

Without `ref:`, a `workflow_dispatch` run checks out the default branch HEAD, not the tagged commit. The source code and the version would silently diverge.

### Version resolution

For simple downstream workflows, resolving `RELEASE_VERSION` is enough:

```yaml
- name: Set the release version
  shell: bash
  run: |
    if [[ "${GITHUB_REF_TYPE}" == "tag" ]]; then
      release_version="${GITHUB_REF_NAME#v}"
    else
      release_version="${{ github.event.inputs.tag }}"
      release_version="${release_version#v}"
    fi

    echo "RELEASE_VERSION=${release_version}" >> "$GITHUB_ENV"
```

For the orchestrator, resolve both the full tag and the stripped version:

```yaml
- name: Set the release version
  id: release
  shell: bash
  run: |
    if [[ "${GITHUB_REF_TYPE}" == "tag" ]]; then
      release_tag="${GITHUB_REF_NAME}"
      release_version="${GITHUB_REF_NAME#v}"
    else
      release_tag="${{ github.event.inputs.tag }}"
      release_tag="${release_tag#refs/tags/}"
      release_version="${release_tag#v}"
    fi

    echo "RELEASE_TAG=${release_tag}" >> "$GITHUB_ENV"
    echo "RELEASE_VERSION=${release_version}" >> "$GITHUB_ENV"
    echo "tag=${release_tag}" >> "$GITHUB_OUTPUT"
```

Never read the version from `Cargo.toml` with `sed` in the `else` branch. That quietly turns a tag-driven release into a whatever-is-on-branch release.

### Toolchain — always read from `rust-toolchain.toml`

```yaml
- name: Read Rust toolchain
  shell: bash
  run: |
    rust_toolchain="$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml | head -n 1)"
    if [ -z "$rust_toolchain" ]; then
      echo "Failed to read Rust toolchain from rust-toolchain.toml" >&2
      exit 1
    fi

    echo "RUST_TOOLCHAIN=${rust_toolchain}" >> "$GITHUB_ENV"

- name: Install toolkit
  uses: dtolnay/rust-toolchain@3c5f7ea28cd621ae0bf5283f0e981fb97b8a7af9
  with:
    toolchain: ${{ env.RUST_TOOLCHAIN }}
```

Never hardcode the toolchain version or use `@stable`. The pinned SHA on `dtolnay/rust-toolchain` is required because the action does not publish semver tags — its only named ref is `@master`. Pinning to a commit SHA makes the action immutable: it will always refer to that exact tree regardless of what the maintainer pushes later, and it cannot be silently replaced if the account were ever compromised. For first-party actions (`actions/checkout`, `actions/upload-artifact`, etc.) the `@v6`, `@v7` tags are maintained under a strict semver contract by GitHub, so floating major-version tags are safe there.

### Rust cache — use a per-target key

```yaml
- name: Setup Rust cache
  uses: Swatinem/rust-cache@v2
  with:
    key: <job-name>-${{ matrix.target }}
```

Without a unique key, cache entries from different targets can collide and corrupt each other.

### Idempotency — check before publishing

Every publish step must be safe to rerun.

**crates.io workspace release:**

This repo no longer publishes a single crate with `cargo publish --package ...` in CI. The crates workflow uses `cargo workspaces plan --skip-published --json` to decide whether there is anything left to do.

```yaml
- name: Build unpublished crate plan
  id: plan
  shell: bash
  env:
    CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
  run: |
    cargo workspaces plan --skip-published --json --token "${CARGO_REGISTRY_TOKEN}" > plan.json
    plan_count="$(jq 'length' plan.json)"
    echo "count=${plan_count}" >> "$GITHUB_OUTPUT"

- name: Publish workspace crates to crates.io
  if: steps.plan.outputs.count != '0'
  run: |
    cargo workspaces publish \
      --publish-as-is \
      --yes \
      --locked \
      --allow-branch "*" \
      --publish-interval 15 \
      --token "${CARGO_REGISTRY_TOKEN}"
```

If the plan is empty, stop cleanly and let the workflow say so.

**PyPI:**

```yaml
- name: Check whether release already exists on PyPI
  id: pypi-check
  shell: bash
  run: |
    if curl -fsS "https://pypi.org/pypi/smbcloud-cli/${RELEASE_VERSION}/json" >/dev/null 2>&1; then
      echo "exists=true" >> "$GITHUB_OUTPUT"
    else
      echo "exists=false" >> "$GITHUB_OUTPUT"
    fi

- name: Publish distribution
  if: steps.pypi-check.outputs.exists != 'true'
  uses: pypa/gh-action-pypi-publish@release/v1
  with:
    skip-existing: true
```

`skip-existing: true` is a safety net for partial re-runs where some wheels already exist but others do not.

**npm:** The check is inline in the publish shell script:

```bash
if npm view "${npm_package_name}@${node_version}" version >/dev/null 2>&1; then
  echo "${npm_package_name}@${node_version} already exists on npm, skipping publish"
  exit 0
fi
npm publish --access public
```

The `exit 0` inside the `run` block is correct here because it prevents the `npm publish` line below it in the same script from executing. This is different from the PyPI anti-pattern where `exit 0` in a separate step does not prevent the next step from running.

### Version guard — match the check to the workflow

For single-package workflows, checking one package against the tag is still fine.

For the crates orchestrator, check the workspace members under `/crates` as a group and be honest about what you are guarding. The point is to catch a bad tag early, not to pretend this is a single-package publish flow.

### Artifact actions

Use `actions/upload-artifact@v7` and `actions/download-artifact@v7` consistently across all workflows.

---

## Workspace scoping — use `--package` for build jobs, not for the crates orchestrator

For normal build jobs, do not build the whole workspace by accident.

This workspace includes `smbcloud-auth-sdk-py`, a PyO3 `cdylib` that requires Python symbols at link time. Building the full workspace on platforms without a matching Python interpreter causes:

```
ld: symbol(s) not found for architecture arm64
clang: error: linker command failed with exit code 1
```

Scope cargo build commands like this:

```bash
cargo build --release --locked --target <target> --package smbcloud-cli
```

The crates release orchestrator is the exception. That workflow is supposed to operate at the workspace root with `cargo workspaces plan` and `cargo workspaces publish`.

The PyPI workflow is also exempt because maturin is already pointed at the right crate via `manifest-path`.

---

## Platform matrix

### Current runners and targets

| Platform      | Runner             | Target                      | Notes                      |
| ------------- | ------------------ | --------------------------- | -------------------------- |
| Linux x86_64  | `ubuntu-latest`    | `x86_64-unknown-linux-gnu`  | Native                     |
| Linux arm64   | `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | Native — do not use QEMU   |
| Windows x64   | `windows-2022`     | `x86_64-pc-windows-msvc`    | Native                     |
| Windows arm64 | `windows-2022`     | `aarch64-pc-windows-msvc`   | Do not use for PyPI wheels |
| macOS x64     | `macos-15-intel`   | `x86_64-apple-darwin`       | Native Intel runner        |
| macOS arm64   | `macos-latest`     | `aarch64-apple-darwin`      | Native Apple Silicon       |

For npm and GitHub Release, use native runners. For PyPI (maturin), the macOS x64 build uses `macos-latest` with cross-compilation because maturin handles it transparently inside its container. Windows arm64 is the exception: GitHub's hosted Windows runners only provide x64 Python, so maturin will refuse to build an `aarch64-pc-windows-msvc` wheel when the interpreter reports `win-amd64`.

### Do not use QEMU for arm64 Linux

When `ubuntu-latest` (x86_64) targets `aarch64-unknown-linux-gnu` via maturin, the manylinux container runs under QEMU emulation. This is slow and causes container toolchain mismatches. Use `ubuntu-24.04-arm` (native arm64 runner) instead.

---

## OpenSSL vendoring

This repo uses `git2` which depends on `libgit2-sys` → `openssl-sys`. On non-Windows platforms without system OpenSSL headers (manylinux containers, macOS cross-compilation), the build fails with:

```
Could not find directory of OpenSSL installation
```

On Windows, `libgit2-sys` uses **WinHTTP** (built into Windows) for HTTPS — OpenSSL is not needed at all. Enabling `vendored` on Windows causes `openssl-src` to invoke Perl to compile OpenSSL from source, which fails because the Windows runner Perl is missing required modules.

### Fix applied

`Cargo.toml` (workspace) — declares the vendored feature once:

```toml
openssl = { version = "0.10", features = ["vendored"] }
```

`crates/cli/Cargo.toml` — activates it only on non-Windows targets:

```toml
[target.'cfg(not(windows))'.dependencies]
openssl = { workspace = true }
```

This activates `openssl-sys/vendored` via Cargo feature unification on Linux and macOS (where it is needed), and leaves the `openssl` crate completely absent from the Windows dependency graph (where WinHTTP handles SSL natively).

### Per-platform behaviour

| Platform      | SSL backend         | `openssl-sys/vendored` | OpenSSL compiled from source |
| ------------- | ------------------- | ---------------------- | ---------------------------- |
| Linux (gnu)   | vendored OpenSSL    | ✅                     | ✅                           |
| macOS arm64   | vendored OpenSSL    | ✅                     | ✅                           |
| macOS x64     | vendored OpenSSL    | ✅                     | ✅                           |
| Windows x64   | WinHTTP (OS native) | ❌                     | ❌                           |
| Windows arm64 | WinHTTP (OS native) | ❌                     | ❌                           |

---

## manylinux containers and Perl

The vendored OpenSSL build compiles OpenSSL from source by running `perl ./Configure`. The manylinux2014 container (CentOS 7) has a stripped Perl installation missing core modules.

### Symptom

```
Can't locate IPC/Cmd.pm in @INC
Can't locate Time/Piece.pm in @INC
```

### Fix

```yaml
- name: Build wheel
  uses: PyO3/maturin-action@v1
  with:
    before-script-linux: yum install -y perl-core
```

`perl-core` is the CentOS 7 meta-package that installs all standard Perl core modules. Install it once rather than chasing individual missing modules — each new OpenSSL version can require additional ones.

`before-script-linux` runs inside the manylinux Docker container, not the host runner. It is ignored on macOS and Windows runners.

### manylinux2014 uses `yum`, not `apt`

The manylinux2014 image is CentOS 7 based. Use `yum`. Using `apt` fails with exit code 127 ("command not found").

### manylinux target per architecture

| Architecture | manylinux target | Container base               | Package manager |
| ------------ | ---------------- | ---------------------------- | --------------- |
| x86_64       | `2014`           | CentOS 7                     | `yum`           |
| aarch64      | `2014`           | CentOS 7 (via native runner) | `yum`           |

Do not use `manylinux: "2014"` for aarch64 with an x86_64 host — the cross-compilation container does not have `yum`. Use a native `ubuntu-24.04-arm` runner with `manylinux: "2014"` instead.

---

## Trusted publishing

### PyPI — uses OIDC trusted publishing

```yaml
permissions:
  id-token: write
  contents: read

- name: Publish distribution
  uses: pypa/gh-action-pypi-publish@release/v1
```

No `PYPI_TOKEN` secret needed. The action exchanges the GitHub OIDC JWT for a short-lived PyPI token automatically.

### npm — does NOT support trusted publishing natively

npm has no OIDC-based trusted publishing equivalent to PyPI. The `id-token: write` permission in the npm workflow is unused for auth.

Authentication requires a stored secret:

```yaml
- name: Publish to NPM
  env:
    NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

`setup-node` with `registry-url` writes an `.npmrc` referencing `${NODE_AUTH_TOKEN}`. That variable must be in the step's `env` block at publish time. If it is missing, `npm publish` silently has no auth and returns 404 for new packages (existing packages that pass the `npm view` idempotency check are never attempted, masking the missing token).

The 404 on `npm publish` does not mean the package does not exist. npm returns 404 for auth failures to avoid leaking package existence.

**npm trusted publishing (future):** npm does support OIDC trusted publishing via `npm trust github`. It requires the package to exist on npm first. Configure it per-package after the first manual publish. See `smbcloud-cli-release` skill for details.

### crates.io — uses `CARGO_REGISTRY_TOKEN`

```yaml
- name: Publish workspace crates to crates.io
  env:
    CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
  run: |
    cargo workspaces publish \
      --publish-as-is \
      --yes \
      --locked \
      --allow-branch "*" \
      --publish-interval 15 \
      --token "${CARGO_REGISTRY_TOKEN}"
```

This repo uses `cargo-workspaces` for the Rust release flow. CI is not doing the interactive version bump step. It is publishing the already-prepared tagged workspace state.

crates.io also supports OIDC trusted publishing, but that is separate from the current workflow.

---

## Split build + release job pattern

For the GitHub Release workflow, separate the build matrix from the release step:

```yaml
jobs:
  build:
    strategy:
      matrix: ...
    steps:
      - name: Build binary
        run: cargo build ...
      - name: Upload artifact
        uses: actions/upload-artifact@v7

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Download all artifacts
        uses: actions/download-artifact@v7
        with:
          pattern: "binary-*"
          merge-multiple: true
      - name: Release
        uses: softprops/action-gh-release@v2
        with:
          files: release/*
```

This ensures all platform binaries are collected before any are attached to the GitHub Release. If a single platform build fails, no partial release is created.

---

## Chaining workflows — use explicit dispatches

Do not rely on GitHub events magically fanning out through the release chain. Keep the chain explicit.

GitHub does not fire downstream repository events the way people expect when the upstream action used `GITHUB_TOKEN`. If you want one release workflow to kick off another, dispatch it yourself.

The current release chain is:

- `release-crate.yml` publishes workspace crates
- then dispatches CLI distribution workflows:
  - `release-github.yml`
  - `release-npm.yml`
  - `release-nuget.yml`
  - `release-pypi.yml`
- then conditionally dispatches SDK workflows based on the crate publish plan:
  - `release-sdk-pypi.yml`
  - `release-sdk-npm.yml`
  - `release-sdk-gem.yml`
- `release-github.yml` dispatches `release-homebrew.yml` after the GitHub Release assets exist

This pattern needs `actions: write` in the upstream workflow permissions.

```yaml
permissions:
  contents: write
  actions: write

- name: Trigger Homebrew release
  uses: actions/github-script@v7
  with:
    script: |
      await github.rest.actions.createWorkflowDispatch({
        owner: context.repo.owner,
        repo: context.repo.repo,
        workflow_id: 'release-homebrew.yml',
        ref: context.payload.repository.default_branch,
        inputs: {
          tag: '${{ steps.tag.outputs.tag }}'
        }
      })
```

A few practical rules:

- downstream workflows must expose `workflow_dispatch` with a `tag` input
- dispatch from the repo default branch so the workflow definition comes from the current release lane
- still check out the requested tag inside the downstream workflow so the built code matches the release
- do not dispatch Homebrew until the GitHub Release job has uploaded the assets it needs

This is a fire-and-forget chain. If you want one green check to mean the whole release tree passed, you need a more involved callback or polling design. Right now the pattern is good enough, but it is not magic.

---

## Homebrew tap release pattern

`release-homebrew.yml` is triggered by `release-github.yml` via `workflow_dispatch` (see above). It runs a single cheap `ubuntu-latest` job — no Rust compilation, no macOS runners.

### Key principle: compute SHA256 once, at build time

The Homebrew formula's `sha256` field is the checksum of the tarball that Homebrew downloads at `brew install` time. The tarball lives at the GitHub Release URL. CI only needs to _write_ that checksum into `cli.rb` — it does not need to download the tarball again.

**In `release-github.yml` (build job, macOS legs only):**

```yaml
- name: Package macOS tarball for Homebrew
  if: contains(matrix.target, 'apple-darwin')
  shell: bash
  run: |
    ARCHIVE_NAME="${PROJECT_NAME}-${{ matrix.name }}.tar.gz"

    mkdir -p staging-brew
    cp "./release/${PROJECT_NAME}-${{ matrix.name }}" "staging-brew/${PROJECT_NAME}"
    strip "staging-brew/${PROJECT_NAME}"

    tar -C staging-brew -czf "${ARCHIVE_NAME}" "${PROJECT_NAME}"

    # Compute and save the checksum alongside the archive
    shasum -a 256 "${ARCHIVE_NAME}" | awk '{print $1}' > "${ARCHIVE_NAME}.sha256"

- name: Upload macOS Homebrew artifacts
  if: contains(matrix.target, 'apple-darwin')
  uses: actions/upload-artifact@v7
  with:
    name: homebrew-${{ matrix.name }}
    path: |
      ${{ env.PROJECT_NAME }}-${{ matrix.name }}.tar.gz
      ${{ env.PROJECT_NAME }}-${{ matrix.name }}.tar.gz.sha256
```

Both the `.tar.gz` and the `.sha256` file are downloaded in the release job and attached to the GitHub Release via `files: release/*`.

**In `release-homebrew.yml` (single job):**

```yaml
- name: Read SHA256 checksums from release
  id: sha
  env:
    GH_TOKEN: ${{ github.token }}
  shell: bash
  run: |
    mkdir -p artifacts

    for attempt in 1 2 3 4 5; do
      if gh release download "${{ steps.release.outputs.tag }}" \
        --repo "${{ env.REPO }}" \
        --pattern "smb-macos-*.tar.gz.sha256" \
        --dir artifacts/; then
        break
      fi

      if [ "$attempt" -eq 5 ]; then
        echo "Failed to download Homebrew checksum artifacts from the GitHub release." >&2
        exit 1
      fi

      echo "Release assets are not visible yet. Waiting 10 seconds..."
      sleep 10
    done
```

`gh` is pre-installed on GitHub-hosted runners. Add a small retry loop here because the GitHub Release exists first and the Homebrew workflow arrives a moment later. Usually that gap is tiny. Sometimes it is not.

### Formula URL convention

The formula `url:` references the release asset directly. Homebrew fetches it at install time — CI never downloads it:

```ruby
on_macos do
  on_arm do
    url 'https://github.com/smbcloudXYZ/smbcloud-cli/releases/download/v0.3.36/smb-macos-arm64.tar.gz'
    sha256 '<computed during build>'
  end
  on_intel do
    url 'https://github.com/smbcloudXYZ/smbcloud-cli/releases/download/v0.3.36/smb-macos-amd64.tar.gz'
    sha256 '<computed during build>'
  end
end
```

The tarball must contain the binary named `smb` (not `smb-macos-arm64`) because the formula's install step is `bin.install 'smb'`.

### Homebrew tap permissions

`release-homebrew.yml` only needs `contents: read` on `smbcloud-cli`. Writing to the tap repo (`smbcloudXYZ/homebrew-tap`) is authenticated via `secrets.HOMEBREW_TAP_TOKEN` in the `actions/checkout` step, not via workflow-level permissions.

---

## `fail-fast: false`

Always set `fail-fast: false` on release build matrices:

```yaml
strategy:
  fail-fast: false
  matrix: ...
```

The default `fail-fast: true` cancels all in-progress jobs as soon as one fails. For release builds, you want every platform to complete so you can diagnose all failures in a single run rather than fixing them one at a time.

---

## SDK distribution README branding

Every distribution channel README must stay aligned with the main `README.MD`. The three elements that must always match exactly:

- **Logo** — `https://avatars.githubusercontent.com/u/89791739?s=200&v=4`, width 128
- **Tagline** — "Deploy to the cloud in one command."
- **Quick start** — `smb login` → `smb init` → `smb deploy`

### Structure per README type

**Wrapper package** (`npm/smbcloud-cli/README.md` → `@smbcloud/cli`, `pypi/README.md` → `smbcloud-cli`):

1. Logo + tagline + nav links + badges
2. About section (copy from main README verbatim)
3. Primary install method for that channel first (`npm install -g` or `pip install`)
4. Quick start
5. All other install methods (Homebrew, npm, pip, shell, PowerShell)
6. Documentation link
7. Platform support table
8. Source & Issues pointer to GitHub
9. License

The PyPI README leads with the PyPI badge; the npm README leads with the npm badge. Both include the full badge row.

**Platform binary package** (`npm/README.md.tmpl` → `@smbcloud/cli-darwin-arm64`, etc.):

Keep it minimal. Show the logo and tagline for brand recognition, then immediately redirect to `@smbcloud/cli`. Do not repeat install instructions — the user landed on the wrong package.

### Platform support table

Keep this table consistent across all READMEs:

```md
| Platform      | Architecture |
| ------------- | ------------ |
| macOS         | arm64, x64   |
| Linux (glibc) | arm64, x64   |
| Windows       | arm64, x64   |
```

Update it whenever a new platform is added to the release matrix.

### PyPI-specific copy

Add this line to distinguish the pip install from npm:

> This package installs the native `smb` executable for your platform directly — no Node.js, no Docker, no runtime dependencies.

### Template variable in `README.md.tmpl`

The platform package template uses `${node_pkg}` as a shell-style variable. This is substituted at release time by the npm publish script. Do not replace it with a hardcoded package name.

---

## SDK trigger rules

Use the crates publish plan to decide whether an SDK release should run.

Current mappings:

- **SDK PyPI** runs when the plan includes any of:
  - `smbcloud-auth-sdk-py`
  - `smbcloud-auth-sdk`
  - `smbcloud-model`
  - `smbcloud-network`
- **SDK npm** runs when the plan includes any of:
  - `smbcloud-auth-sdk-wasm`
  - `smbcloud-auth-sdk`
  - `smbcloud-network`
  - `smbcloud-networking`
- **SDK Ruby gem** runs when the plan includes any of:
  - `smbcloud-auth-sdk`
  - `smbcloud-model`
  - `smbcloud-network`

These are based on the actual wrappers in `sdk/`:

- `sdk/python/pyproject.toml` points maturin at `crates/smbcloud-auth-sdk-py/Cargo.toml`
- `sdk/npm/smbcloud-auth/prepare-package.mjs` builds from `crates/smbcloud-auth-sdk-wasm`
- `sdk/gems/auth/ext/auth/Cargo.toml` pulls published crates from crates.io

One practical warning: the Ruby gem release depends on crates.io propagation. If crates were just published, wait for them to appear before building the gem.

## Common mistakes

**Building the full workspace in release workflows**
Always pass `--package smbcloud-cli` to `cargo build` and `cargo publish`. Omitting it builds `smbcloud-auth-sdk-py` (PyO3 cdylib) which fails on platforms without a matching Python interpreter.

**`exit 0` in a separate step does not skip the next step**
`exit 0` only terminates the current `run` shell. The following step runs regardless. Use `$GITHUB_OUTPUT` + `if:` conditions for inter-step flow control.

**Hardcoded toolchain version**
Never hardcode `toolchain: 1.92` or use `@stable`. Always read from `rust-toolchain.toml`.

**Missing `ref:` on checkout for `workflow_dispatch`**
Without `ref: ${{ github.event.inputs.tag || github.ref }}`, a manual dispatch checks out the default branch, not the tag. The compiled binary will be from the wrong commit.

**Using `sed` on `Cargo.toml` as the version fallback**
This reads whatever version is currently in `Cargo.toml` on the checked-out branch, which may not match the tag being re-published. Use the `tag` input directly.

**QEMU for arm64 Linux**
Using `ubuntu-latest` (x86_64) to build `aarch64-unknown-linux-gnu` via QEMU inside manylinux is slow and causes container mismatch errors. Use `ubuntu-24.04-arm`.

**Windows arm64 PyPI wheels on hosted runners**
Maturin needs a matching Python interpreter for wheel generation. On GitHub-hosted Windows runners, the available interpreter reports `win-amd64`, so an `aarch64-pc-windows-msvc` wheel build gets skipped and then fails. Unless you have an arm64 Windows runner with arm64 Python installed, leave Windows arm64 out of the PyPI wheel matrix.

**`yum` vs `apt` in manylinux**
manylinux2014 is CentOS 7. Use `yum`, not `apt`. For manylinux_2_28 (AlmaLinux 8) use `dnf`.

**Missing `User-Agent` on crates.io API requests**
crates.io rejects curl requests without a `User-Agent` header. Always include `-H "User-Agent: smbcloud-cli-release-workflow"`.

**Enabling vendored OpenSSL unconditionally on Windows**
`openssl = { features = ["vendored"] }` as an unconditional dependency forces `openssl-src` to compile OpenSSL from source on Windows. Windows builds fail because the runner Perl is missing modules required by OpenSSL's `Configure` script. Use `[target.'cfg(not(windows))'.dependencies]` so the `openssl` crate is never pulled into the Windows dependency graph. `libgit2-sys` uses WinHTTP on Windows and does not need OpenSSL.

**Distribution README out of sync with main README**
When the main `README.MD` changes its tagline, logo URL, quick start commands, or value proposition, the npm and PyPI READMEs must be updated in the same commit. Stale distribution READMEs show outdated branding on npmjs.com and pypi.org — the highest-visibility surfaces for new users discovering the CLI.

**Platform support table missing new targets**
When a new platform is added to any release matrix (e.g. `linux-arm64`), update the platform support table in `npm/smbcloud-cli/README.md` and `pypi/README.md` in the same PR.

**Use workflow-level `env:` for compile-time secrets, not `$GITHUB_ENV` export steps**
Compile-time secrets like `CLI_CLIENT_SECRET` (read by `env!()`) must be available to every `cargo` invocation across all platforms. Writing to `$GITHUB_ENV` in a separate step is unreliable: it does not propagate into Docker containers, and on Windows runners the default PowerShell shell does not expand `"$GITHUB_ENV"` correctly.

Declare the secret in the workflow-level `env:` block:

```yaml
env:
  CLI_CLIENT_SECRET: ${{ secrets.CLI_CLIENT_SECRET }}
  CARGO_TERM_COLOR: always
```

Never add a dedicated "Export build secret" step for this purpose.

**manylinux Docker containers require `docker-options` to forward env vars**
Even with the secret in the workflow-level `env:` block, maturin-action's manylinux builds run inside a Docker container that does not automatically inherit the runner's environment. The secret is visible to the action process (shown as `CLI_CLIENT_SECRET: ***` in the step log) but is not forwarded into the container unless explicitly requested.

Fix: add `docker-options: -e CLI_CLIENT_SECRET` to the `Build wheel` step:

```yaml
- name: Build wheel
  uses: PyO3/maturin-action@v1
  with:
    target: ${{ matrix.build.TARGET }}
    manylinux: ${{ matrix.build.MANYLINUX || 'off' }}
    working-directory: pypi
    args: --release --locked --compatibility pypi --out dist
    before-script-linux: yum install -y perl-core
    docker-options: -e CLI_CLIENT_SECRET
```

`-e CLI_CLIENT_SECRET` tells `docker run` to forward that variable from the runner process into the container. Windows and macOS legs are unaffected (they do not use Docker). Without this, `env!("CLI_CLIENT_SECRET")` fails only on Linux manylinux legs while all other platforms succeed — making the root cause easy to miss.

**PyPI blocks short and protocol-reserved package names**
PyPI maintains a blocklist of names that are too generic or conflict with well-known protocols and tools. `smb` is blocked because it is the name of the Windows SMB/CIFS file sharing protocol. Attempting to publish a package named `smb` returns `400 The name 'smb' isn't allowed`. There is no workaround — the name cannot be registered regardless of who owns it. If you need `uvx <name>` to work without `--from`, the package name and binary name must match AND the package name must not be on the blocklist. For this repo, `uvx --from smbcloud-cli smb` is the correct form.

**Multiple release workflows all listening to tag pushes**
Only `release-crate.yml` should run directly from a release tag. The downstream workflows should be `workflow_dispatch` only. If they all listen to `push.tags`, they race each other and you lose control of the order.

**`on: release: types: [published]` does not fire when the release is created by automation**
When `softprops/action-gh-release` creates a GitHub Release using the implicit `GITHUB_TOKEN`, GitHub does not dispatch the `release` event to other workflows. The downstream workflow just never runs. Use `workflow_dispatch` via `actions/github-script@v7` instead, and give the upstream workflow `actions: write`.

**Downloading full tarballs in the Homebrew workflow to compute SHA256**
The Homebrew formula only needs the SHA256 hash. Compute it during the build in `release-github.yml`, attach the `.sha256` files to the GitHub Release, and have `release-homebrew.yml` download only those tiny files. Do not re-download full binaries and do not rebuild Rust code there.

**Forgetting that Homebrew depends on the GitHub Release assets**
Homebrew cannot run first. It needs the `.sha256` files attached by `release-github.yml`. Keep that dependency explicit.

**Starting the Ruby gem release before crates.io has caught up**
`sdk/gems/auth/ext/auth/Cargo.toml` pulls `smbcloud-auth-sdk`, `smbcloud-model`, and `smbcloud-network` from crates.io. If the gem workflow starts immediately after the crates publish job, the index may not be ready yet. Add a small wait-and-retry loop against the crates.io API before the gem build starts.

**Rebuilding macOS binaries in `release-homebrew.yml`**
`release-github.yml` already builds macOS binaries. Running a second Rust compilation in `release-homebrew.yml` wastes two macOS runner slots and risks producing binaries with different checksums if the environment differs. Build once, reuse the output.

**Homebrew tarball contains wrong binary name**
The formula uses `bin.install 'smb'`. The tarball must contain the binary named `smb`, not `smb-macos-arm64`. Strip and repackage correctly in the "Package macOS tarball for Homebrew" step.

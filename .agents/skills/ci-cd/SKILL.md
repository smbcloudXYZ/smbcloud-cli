---
name: ci-cd
description: Use when creating, debugging, or improving GitHub Actions workflows for this repo — especially release workflows for crates.io, PyPI, npm, GitHub Releases, or Debian packages. Also covers SDK distribution README branding alignment across npm, PyPI, and the main repo. Covers OpenSSL vendoring, Windows vs non-Windows platform-conditional dependencies, manylinux containers, Perl module gaps, workspace scoping, native vs QEMU arm64 runners, trusted publishing, artifact actions, toolchain parity, and idempotency patterns.
---

# CI/CD

## Source of truth for all release workflows

| Distribution            | Workflow file                          |
| ----------------------- | -------------------------------------- |
| crates.io               | `.github/workflows/release-crate.yml`  |
| PyPI                    | `.github/workflows/release-pypi.yml`   |
| npm                     | `.github/workflows/release-npm.yml`    |
| GitHub Release binaries | `.github/workflows/release-github.yml` |

All four workflows share the same structural patterns. When editing one, check the others for consistency.

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

Every release workflow must follow these patterns identically.

### Trigger

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

### Checkout — always include the ref override

```yaml
- name: Checkout
  uses: actions/checkout@v6
  with:
    ref: ${{ github.event.inputs.tag || github.ref }}
```

Without `ref:`, a `workflow_dispatch` run checks out the default branch HEAD, not the tagged commit. The source code and the version would silently diverge.

### Version resolution

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

Never read the version from `Cargo.toml` with `sed` in the `else` branch. That silently publishes the current `Cargo.toml` version regardless of the tag input.

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

Every publish step must be gated so re-runs do not fail on already-published versions.

**crates.io:**

```yaml
- name: Check whether release already exists on crates.io
  id: crates-check
  shell: bash
  run: |
    if curl -fsS \
      -H "User-Agent: smbcloud-cli-release-workflow" \
      "https://crates.io/api/v1/crates/smbcloud-cli/${RELEASE_VERSION}" \
      >/dev/null 2>&1; then
      echo "exists=true" >> "$GITHUB_OUTPUT"
    else
      echo "exists=false" >> "$GITHUB_OUTPUT"
    fi

- name: Publish to crates.io
  if: steps.crates-check.outputs.exists != 'true'
  ...
```

The `User-Agent` header is required — crates.io rejects requests without one.

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

### Version guard — verify Cargo.toml matches the tag

```yaml
- name: Verify Cargo.toml version matches tag
  shell: bash
  run: |
    CARGO_VERSION=$(cargo metadata --no-deps --format-version 1 \
      | jq -r '.packages[] | select(.name == "smbcloud-cli") | .version')

    if [[ "${CARGO_VERSION}" != "${RELEASE_VERSION}" ]]; then
      echo "::error::Version mismatch — bump version in Cargo.toml before releasing."
      exit 1
    fi
```

### Artifact actions

Use `actions/upload-artifact@v7` and `actions/download-artifact@v7` consistently across all workflows.

---

## Workspace scoping — always use `--package`

The workspace root `Cargo.toml` is a **virtual manifest** (no `[package]` section). Running `cargo build` or `cargo publish` from the workspace root without `--package` will either fail or build every workspace member.

This workspace includes `smbcloud-auth-py`, a PyO3 `cdylib` that requires Python symbols at link time. Building the full workspace on platforms without a matching Python interpreter causes:

```
ld: symbol(s) not found for architecture arm64
clang: error: linker command failed with exit code 1
```

Always scope cargo commands:

```bash
cargo build --release --locked --target <target> --package smbcloud-cli
cargo publish --locked --package smbcloud-cli
```

The PyPI workflow is exempt because maturin is already pointed at `crates/cli/Cargo.toml` via `manifest-path` in `pypi/pyproject.toml`.

---

## Platform matrix

### Current runners and targets

| Platform      | Runner             | Target                      | Notes                     |
| ------------- | ------------------ | --------------------------- | ------------------------- |
| Linux x86_64  | `ubuntu-latest`    | `x86_64-unknown-linux-gnu`  | Native                    |
| Linux arm64   | `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | Native — do not use QEMU  |
| Windows x64   | `windows-2022`     | `x86_64-pc-windows-msvc`    | Native                    |
| Windows arm64 | `windows-2022`     | `aarch64-pc-windows-msvc`   | Cross-compile on x64 host |
| macOS x64     | `macos-15-intel`   | `x86_64-apple-darwin`       | Native Intel runner       |
| macOS arm64   | `macos-latest`     | `aarch64-apple-darwin`      | Native Apple Silicon      |

For npm and GitHub Release, use native runners. For PyPI (maturin), the macOS x64 build uses `macos-latest` with cross-compilation because maturin handles it transparently inside its container.

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
- name: Publish to crates.io
  env:
    CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
  run: cargo publish --locked --package smbcloud-cli
```

crates.io also supports OIDC trusted publishing but the setup is separate from this workflow.

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

## Common mistakes

**Building the full workspace in release workflows**
Always pass `--package smbcloud-cli` to `cargo build` and `cargo publish`. Omitting it builds `smbcloud-auth-py` (PyO3 cdylib) which fails on platforms without a matching Python interpreter.

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

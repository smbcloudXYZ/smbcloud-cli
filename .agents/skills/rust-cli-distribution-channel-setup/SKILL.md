---
name: rust-cli-distribution-channel-setup
description: How to ship a Rust CLI binary to every package channel (crates.io, GitHub Releases, Homebrew, Scoop, winget, npm, PyPI, NuGet .NET tool, RubyGems, MCP Registry) from one tag push, using this repo's workflows as the reference implementation. Use this whenever the user wants to add a distribution channel to a Rust CLI ("add a NuGet channel", "publish my binary on npm/PyPI/winget/Homebrew"), set up or fix release automation that fans out from a tag, configure trusted publishing (OIDC) on any registry, wire per-platform build matrices, or debug a failed publish step, even if they only paste an error such as "403 API key has expired", "invalid-publisher", "No trusted publisher configured", "no matching trusted publishing policy", or an npm 404 on publish. Also use it when deciding the layout of a `packaging/` directory or how version numbers stay in sync across manifests.
---

# Shipping a Rust CLI to every channel

This skill describes the release system of this repo (a Rust CLI named `smb`, plus a second binary, `xcrs`) as a template for other Rust CLIs. Workflow file names below are the real ones under `.github/workflows/`. Anything account-specific is written as a placeholder: `<owner>/<repo>`, `<package>`, `<tag>`. Do not paste real account IDs, policy IDs, tokens, or customer details into a public repo; keep those in a private place.

For repo-specific detail, read the neighbouring skills instead of duplicating them here:

- `ci-cd`: workflow patterns, trusted-publishing notes per registry, idempotency patterns.
- `smbcloud-cli-release`: the release runbook, version-sync rules, npm and PyPI details.

## The shape of the system

One idea runs through everything: **a tag is the single input, and every channel is an independent, idempotent workflow that takes that tag.** Independence means a failure in one registry never blocks the others. Idempotency means any failed channel can be re-dispatched without double-publishing.

```text
git tag vX.Y.Z (on main) ──push──▶ release-crate.yml
                                     │  cargo workspaces publish (--skip-published)
                                     ▼
                         dispatch-distributions  (workflow_dispatch -f tag=vX.Y.Z)
        ┌──────────┬──────────┬──────────┬──────────┬───────────────┐
        ▼          ▼          ▼          ▼          ▼               ▼
  release-github  release-npm  release-pypi  release-nuget  release-xcrs-*   (SDK workflows,
        │                                        │                            gated on which
        ▼ (after assets exist)                   ▼ (after publish succeeds)   crates changed)
  release-homebrew                       release-mcp-registry
  release-scoop
  release-winget
```

### Why dispatch instead of `on: push: tags`

A workflow started by `GITHUB_TOKEN` events (a release created by another workflow, for example) does not trigger further workflows, but `workflow_dispatch` through the REST API does, provided the dispatching job has `permissions: actions: write`. So the orchestrator fans out with `createWorkflowDispatch`, passing `ref: <tag>` and `inputs: { tag }`. Chained follow-ups use the same trick: `release-github.yml` dispatches Homebrew, Scoop and winget once the release assets exist, and `release-nuget.yml` dispatches `release-mcp-registry.yml` once `publish-nuget` succeeds.

### Why every workflow checks out `inputs.tag`

Each job starts with `actions/checkout` and `ref: ${{ github.event.inputs.tag || github.ref }}`, and derives `RELEASE_VERSION` from the tag. This is what makes republishing safe:

- The build uses the sources that were tagged, even when the workflow is dispatched from another branch.
- Re-dispatching `-f tag=v1.1.1` rebuilds 1.1.1, not whatever is on `main` today.
- A workflow *run* always replays the workflow file from the ref it was started on. "Re-run failed jobs" on an old run reuses the **old** workflow file. After fixing a workflow, start a fresh run from the branch that has the fix: `gh workflow run <file>.yml --ref <fixed-branch> -f tag=<tag>`. The tag still selects the sources.

Add a version guard early in each channel: fail if the manifest version differs from the tag (the crates.io workflow does this with `cargo metadata` and `jq`; the gem workflow checks `version.rb`). A mismatch otherwise surfaces as a confusing registry rejection.

## Repository layout

```text
crates/<cli>/                  the Rust binary crate (the version source of truth)
packaging/
  npm/<package>/               wrapper package + templates for per-platform packages
  npm/scripts/                 render-platform-package.cjs, render-main-package.cjs
  nuget/<package>/             .csproj + Program.cs launcher for a .NET tool
  pypi/<package>/              pyproject.toml for maturin `bin` wheels
  winget/                      *.yaml.in manifest templates
  windows/                     check-static-crt.ps1
  mcp/server*.json             MCP Registry listings
scripts/
  sync-release-version.mjs     copies the crate version into every manifest above
  check-release-versions.mjs   fails if anything is out of sync
Makefile                       make patch|minor|major|custom VERSION=x.y.z
```

Keep one directory per channel and, where a channel ships several packages, one subdirectory per package. `make release` bumps every workspace crate in lockstep (`cargo workspaces version`), runs `sync-release-version.mjs` so npm, NuGet, PyPI, gem and MCP manifests carry the same version, regenerates lockfiles, runs `check-release-versions.mjs`, and commits `Release <version>` **without tagging**. Tagging is a separate, deliberate step: merge the release branch with `--no-ff`, then create an annotated tag on the merge commit and push it.

Make the sync script throw when a pattern does not match (`replaceOrThrow`). A silent no-op in a version regex is how a registry ends up with a stale version.

The MCP Registry listings (`packaging/mcp/*.json`) are part of this sync because the registry validates them against the *published* npm and NuGet packages, so those must be live at the recorded version first. That ordering is why MCP Registry hangs off the NuGet workflow instead of the parallel fan-out.

## Auth policy

Prefer **trusted publishing (OIDC)** wherever the registry offers it. There is no secret to leak, rotate or find expired at release time, and the failure mode becomes a policy mismatch you can read in the log.

Rules that hold for every registry:

1. The publish job needs `permissions: id-token: write` (and `contents: read`). Scope it to the single publishing job so build jobs never get an OIDC token.
2. The policy names the **workflow file that contains the publish job**. That is the claim the registry matches (`job_workflow_ref`). A policy registered for the orchestrator, `release-crate.yml`, which only dispatches the real workflow, never matches.
3. One policy per package per workflow file. If two workflows publish (CLI and a second binary), register both.
4. A package that does not exist yet usually needs a **pending publisher** (PyPI, RubyGems, crates.io) or a manual first publish (npm). Do that before the first tag.
5. Do not also pass the old token to the publish command. Mixed config makes failures ambiguous.

| Channel | Auth | Secret that remains |
|---|---|---|
| crates.io | OIDC supported (`rust-lang/crates-io-auth-action`); this repo still uses a token | `CARGO_REGISTRY_TOKEN` (until migrated) |
| GitHub Releases | `GITHUB_TOKEN` | none |
| Homebrew tap | PAT with write access to the tap repo | tap token |
| Scoop bucket | PAT with write access to the bucket repo | bucket token |
| winget | two classic PATs (see below) | issue token, PR token |
| npm | trusted publisher per package | none |
| PyPI | trusted publisher per project | none |
| NuGet | trusted publishing policy + `NuGet/login` | `NUGET_USER` (a name, not a credential) |
| RubyGems | trusted publisher per gem | none |
| MCP Registry | GitHub OIDC (`mcp-publisher login github-oidc`) | none |

Cross-repo writes (tap, bucket, winget-pkgs) cannot use OIDC because the target is another repository, so those tokens stay. Give each the minimum scopes and name them for what they do.

## Build matrix basics

- **Native runners, not QEMU.** Use `ubuntu-24.04-arm` (or `ubuntu-22.04-arm`) for aarch64 Linux. Emulated builds are slow and flaky for large Rust builds.
- **Pick the oldest reasonable glibc.** Building Linux binaries on `ubuntu-22.04` keeps them running on Debian 12 and Ubuntu 22.04. Building on the newest image produces binaries that fail with `GLIBC_2.xx not found` on older hosts.
- **Static CRT on Windows.** Set `rustflags = ["-C", "target-feature=+crt-static"]` for the MSVC targets in `.cargo/config.toml`, then gate the release with a script that runs `dumpbin /dependents` and fails if `VCRUNTIME140.dll` or `MSVCP140.dll` appears. Without it the exe works on CI runners (which have the redistributable) and fails to start on a clean machine. This is also a winget requirement in practice.
- **Windows arm64 is a cross-compile** on an x64 runner (`aarch64-pc-windows-msvc`); `maturin` also refuses to build an arm64 wheel under x64 Python, so check the PyPI workflow header for how that case is handled.
- **macOS:** `macos-15-intel` for x86_64 and `macos-latest` for arm64. Build each natively instead of cross-compiling.
- **OpenSSL:** depend on `openssl = { version = "0.10", features = ["vendored"] }` for targets where system OpenSSL differs (manylinux containers, musl, cross builds), or use `rustls`. Vendored builds need Perl in manylinux (CentOS-based) containers: `before-script-linux: yum install -y perl-core`.
- **Compile-time secrets** (for example a client secret read with `env!`/`option_env!`) belong in workflow-level `env:` so every build job sees them. If a job builds without it, the binary is produced and silently broken.
- Use `cargo build --locked --release --target <triple>` and a per-target `Swatinem/rust-cache` key.

## Per-channel setup

For each channel: what it publishes, the auth, the one-time registry setup, and how it stays idempotent.

### crates.io (`release-crate.yml`)

- Publishes all workspace crates with `cargo workspaces publish --publish-as-is --yes --locked --allow-branch "*" --publish-interval 15`. The interval gives the index time to propagate between dependent crates.
- Idempotency: `cargo workspaces plan --skip-published --json` builds the list of unpublished crates. If it is empty the publish step is skipped, and the plan also decides which SDK workflows to dispatch.
- Setup: create the token once, or configure a trusted publisher on crates.io for `<owner>/<repo>` and the workflow file, then use `rust-lang/crates-io-auth-action` and pass its token. A new crate name must be published once with a token before a trusted publisher can be attached.

### GitHub Releases (`release-github.yml`)

- Builds the six binaries (linux amd64/arm64, windows amd64/arm64, macOS amd64/arm64), runs the static-CRT check on Windows, renames them to `<bin>-<platform>[.exe]`, and attaches them with `softprops/action-gh-release`. Release notes are generated from merged PRs since the previous tag.
- Auth: `GITHUB_TOKEN` with `contents: write`.
- Idempotency: re-uploading assets to an existing tag release overwrites them.
- After the assets exist it dispatches Homebrew, Scoop and winget, which all consume the release assets by URL and checksum.

### Homebrew tap (`release-homebrew.yml`)

- Reads the SHA-256 of the macOS tarballs from the release, renders a formula, commits to the tap repo. The formula's `test do` block runs `<bin> --help`.
- Auth: a PAT stored as a secret, scoped to the tap repository only.
- Setup: create `<owner>/homebrew-<tap>` with a `Formula/` directory.
- Idempotency: skip the commit when the rendered formula is identical to the one in the tap.

### Scoop (`release-scoop.yml`)

- Same pattern as Homebrew for a bucket repo: read the `.sha256` files the GitHub release publishes next to the Windows assets, write the manifest JSON with hashes, commit. Use Scoop's `#/<bin>.exe` URL fragment to rename `<bin>-win-amd64.exe` to the name the shim needs on `PATH`.
- Add a `concurrency` group keyed on the tag. A re-pushed or re-dispatched tag then queues behind the first run, hits the "already published" check, and exits clean instead of racing.

### winget (`release-winget.yml`)

- Runs on `windows-latest` because `wingetcreate` is Windows-only. Manifests are templated in `packaging/winget/*.yaml.in` (installer, locale, version).
- The workflow branches: if the package already exists in `microsoft/winget-pkgs`, use `wingetcreate update`; otherwise render the manifests and `wingetcreate submit` them as a new package. It also skips when an open PR for that package already exists.
- Package identifiers have the form `Publisher.Package` and cannot be renamed later without a new package, so choose them once.
- Auth, two classic PATs: an issue token (`public_repo`) used to file the Package Request issue, and a PR token with `public_repo`, `read:org`, `read:discussion` and **`workflow`**. `wingetcreate` fast-forwards the token owner's fork before branching, and GitHub refuses that sync once an upstream commit touches `.github/workflows`. "The forked repository could not be synced with the upstream commits" means the PR token lacks `workflow`.
- The first submission goes through Microsoft's moderation, so the PR can sit open for days. That is expected.

### npm (`release-npm.yml`)

- Layout: one wrapper package (`<scope>/<cli>`) whose `optionalDependencies` list one package per platform (`<scope>/<cli>-linux-x64`, `-darwin-arm64`, `-win32-x64`, and so on). Each platform package declares `os` and `cpu`, so npm installs only the matching binary. The wrapper has a tiny launcher that resolves and `spawn`s the right one. Platform packages are rendered from templates at release time (`render-platform-package.cjs`), the wrapper by `render-main-package.cjs`.
- Order: platform packages publish first, the wrapper `needs` all of them. One failed platform job therefore blocks the wrapper. That is deliberate, because a wrapper pointing at a missing version is a broken install.
- Auth: trusted publisher, set **per package**, not per scope: `npm trust github <package> --repo <owner>/<repo> --file release-npm.yml`. Needs npm >= 11.5.1 and Node >= 22.14.0 on the runner. Do not set `NODE_AUTH_TOKEN`.
- Setup: `npm trust` only works on packages that already exist, so the first version of every new package name is published by hand. Remember each new platform package needs its own trust entry.
- Idempotency: `npm view <package>@<version>` before publishing, skip when it exists.
- A 404 on `npm publish` is an auth failure in disguise (npm hides package existence). Look for a missing trust entry first.

### PyPI (`release-pypi.yml`)

- Ships the Rust binary as `maturin` `bindings = "bin"` wheels, so `pip install <package>` or `uvx <package>` puts the native executable on `PATH`. Build one wheel per target with `PyO3/maturin-action`, plus an sdist, on native runners; manylinux builds run in a container.
- Auth: `pypa/gh-action-pypi-publish@release/v1` with `id-token: write`. No token.
- Setup: on pypi.org add a trusted publisher (owner, repo, workflow file name, environment blank) for each project. For a project that does not exist yet, create a **pending publisher** under your account's publishing settings; the first upload creates the project.
- Idempotency: query the PyPI JSON API for the version and set `skip-existing: true`.

### NuGet .NET tool (`release-nuget.yml`)

- Packs the native binaries into a .NET global tool, one folder per runtime identifier (`linux-x64`, `linux-arm64`, `osx-x64`, `osx-arm64`, `win-x64`, `win-arm64`), with a small `Program.cs` that locates the right binary and starts it. Users run `dotnet tool install -g <PackageId>`.
- Jobs: `build-native-binaries` (matrix) → `pack-nuget` (downloads all binaries, runs `dotnet pack`) → `smoke-test` (installs the produced `.nupkg` from a local source on Linux, Windows and macOS and runs `<bin> --version`) → `publish-nuget`. The smoke test is what catches a missing RID folder or a non-executable file before the package is public. NuGet versions cannot be unpublished, only unlisted.
- Auth: trusted publishing. `publish-nuget` has `id-token: write`, runs `NuGet/login@v1` with `user: ${{ secrets.NUGET_USER }}`, and passes `steps.nuget-login.outputs.NUGET_API_KEY` to `dotnet nuget push`. The key lasts about an hour and publishing starts seconds later.
- Setup on nuget.org (Trusted Publishing, under your profile): add a policy with the owner, `<owner>/<repo>`, and the workflow file name. **One policy per workflow file**; use the profile name (not email) as `NUGET_USER`.
- Idempotency: check `https://api.nuget.org/v3-flatcontainer/<id-lowercase>/index.json` for the version, skip login and push when present; `--skip-duplicate` backs it up.
- After a successful push it dispatches the MCP Registry workflow.

### RubyGems and other SDK bindings (`release-sdk-gem.yml`, `release-sdk-npm.yml`, `release-sdk-pypi.yml`)

Library bindings (WASM for npm, PyO3 for PyPI, a native-extension gem) follow the same rules but are **gated**: the orchestrator derives from the crates.io plan which SDK-facing crates were actually published and dispatches only the SDK workflows that need it.

- RubyGems auth is a trusted publisher per gem (owner, repo, workflow file, environment blank). `rubygems/configure-rubygems-credentials` exchanges the token. A gem that does not exist yet needs a pending publisher (about 12 hours of validity), created right before the first release.
- Push each `.gem` individually and treat "already published" as success, so a partial release can be completed by re-running.
- Native-extension gems resolve crates.io dependencies at build time, so the workflow has to wait for the crates to be published first.

### MCP Registry (`release-mcp-registry.yml`)

- Publishes `server.json` listings with `mcp-publisher` using GitHub OIDC (`id-token: write`).
- The registry checks each listed package (npm, NuGet, Cargo) against the real registry at the recorded version, so it must run **after** those are live. Chain it from the slowest dependency (NuGet here) rather than the fan-out.
- Idempotency: the publish helper treats "version already published" as a skip.

## Adding a channel to an existing Rust CLI

1. Decide the delivery model: binary-in-package (npm optional deps, PyPI `bin` wheels, NuGet RIDs) or metadata-pointing-at-release-assets (Homebrew, Scoop, winget). The second kind is a few dozen lines of YAML; the first needs a build matrix.
2. Create `packaging/<channel>/<package>/` with the manifest and a launcher if the ecosystem needs one.
3. Add the version to `scripts/sync-release-version.mjs` and `check-release-versions.mjs`.
4. Write `release-<channel>.yml` with a `workflow_dispatch` `tag` input, the tag checkout, the version guard, the build, a smoke test where the artifact is runnable, an existence check, and the publish step.
5. Do the registry-side setup (policy or pending publisher naming that exact file) **before** the first tag.
6. Add a dispatch step to `release-crate.yml` (or chain it from the workflow it depends on).
7. Dry-run: dispatch the workflow against an existing tag. The existence check should skip publishing and everything before it should pass.
8. Document the channel in `ci-cd` and the runbook so the next failure is quick to diagnose.

## Republishing a version

```sh
gh workflow run release-<channel>.yml --ref <branch-with-fix> -f tag=<tag>
```

Confirm with whoever owns the release first: registry publishes are public and mostly irreversible. The existence checks make it safe to re-dispatch every channel, since finished ones skip.

## Troubleshooting

| Error text | Channel | Cause and fix |
|---|---|---|
| `403 The specified API key has expired` | NuGet | The workflow still pushes with a stored key. Move to `NuGet/login` and a trusted publishing policy. |
| `no matching trusted publishing policy` (at `NuGet/login`) | NuGet | Policy missing, or it names another workflow file, repo or owner. Add one policy per workflow file; check `NUGET_USER` is the profile name. |
| `invalid-publisher: valid token, but no corresponding publisher` | PyPI | No trusted publisher for this project and workflow file, or the file name differs. Fix the publisher; for a new project, create a pending publisher. |
| `No trusted publisher configured for this workflow` | RubyGems | Publisher registered to the orchestrator or another file. Register the workflow file that contains the push job. |
| `npm error 404 Not Found - PUT https://registry.npmjs.org/<pkg>` | npm | Usually missing trust entry for that package (npm returns 404 for auth failures). Run `npm trust github ...` per package. A wrapper that `needs` it is skipped too. |
| `npm error code EOTP` | npm | Token path with 2FA. Switch to trusted publishing; do not pass `NODE_AUTH_TOKEN`. |
| `The forked repository could not be synced with the upstream commits` | winget | PR token lacks the `workflow` scope. Add it, or sync the fork by hand. |
| App fails to start on a clean Windows install, works on CI | Windows binaries | Dynamic CRT. Enable `+crt-static` and run the `dumpbin` dependency check. |
| `GLIBC_2.xx not found` | Linux binaries | Built on a runner image newer than the target. Build on the oldest supported image. |
| `Resource not accessible by integration` on `createWorkflowDispatch` | orchestrator | The dispatching job lacks `permissions: actions: write`. |
| Downstream workflow never starts after a release | chaining | Events created with `GITHUB_TOKEN` do not trigger workflows. Dispatch explicitly through the API. |
| Re-run of a failed run still shows the old behaviour | any | Re-runs reuse the original workflow file. Start a new run from the fixed branch with `-f tag=<tag>`. |
| MCP Registry run never happened for a release | MCP | It is chained from NuGet. A failed `publish-nuget` stops the chain; fix NuGet, then re-dispatch both. |
| `cargo workspaces` fails on index lag between crates | crates.io | Raise `--publish-interval`; re-run, since `--skip-published` resumes where it stopped. |
| Registry rejects a manifest with a stale version | any | `sync-release-version.mjs` missed a file. Make the pattern throw, and run `check-release-versions.mjs` in CI. |

## Checklist before the first tag

- [ ] Crate versions match the tag; `make check-release-versions` passes.
- [ ] Each registry has its trusted publisher or policy naming the exact publish workflow file; new package names have a pending publisher or a manual first publish.
- [ ] Secrets exist for the remaining token channels (crates.io until migrated, tap, bucket, winget PATs) and `NUGET_USER`.
- [ ] Compile-time secrets are in workflow-level `env`.
- [ ] Windows static-CRT check and the NuGet smoke test pass in a dry run.
- [ ] The orchestrator job has `actions: write`; chained workflows dispatch with `ref: <tag>`.

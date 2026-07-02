---
name: smbcloud-cli-release
description: Use when building, packaging, or releasing smbCloud CLI binaries through npm, PyPI, or GitHub Actions CI/CD. Covers the Rust binary release source of truth, the npm wrapper and platform packages, the PyPI `maturin` package under `pypi/`, local publishing with `uv tool install maturin`, and fixes to `.github/workflows/release-npm.yml` or `.github/workflows/release-pypi.yml`.
---

# smbcloud CLI Release

Use this skill when work touches any part of the smbCloud CLI distribution flow:

- local npm release for `smbcloud-cli`
- local PyPI release for `smbcloud-cli`
- GitHub Actions npm release automation
- GitHub Actions PyPI release automation
- Rust binary packaging into npm platform packages or PyPI wheels
- release failures caused by macOS cross-compilation, missing package metadata, npm 2FA, or PyPI trusted publishing setup

## Source of truth

Use these files as the release source of truth:

- Rust crate version and binary name: `crates/cli/Cargo.toml`
- Workspace crate dependency versions: `Cargo.toml` under `[workspace.dependencies]`
- SDK WASM crate version: `crates/smbcloud-auth-sdk-wasm/Cargo.toml`
- SDK npm package version: `sdk/npm/smbcloud-auth/package.json`
- SDK npm build script: `sdk/npm/smbcloud-auth/prepare-package.mjs`
- npm release workflow: `.github/workflows/release-npm.yml`
- PyPI release workflow: `.github/workflows/release-pypi.yml`
- npm platform package generator: `npm/scripts/render-platform-package.cjs`
- npm wrapper package generator: `npm/scripts/render-main-package.cjs`
- npm wrapper launcher: `npm/smbcloud-cli/src/index.ts`
- npm wrapper committed package metadata: `npm/smbcloud-cli/package.json`
- npm wrapper committed lockfile: `npm/smbcloud-cli/package-lock.json`
- PyPI package metadata: `pypi/pyproject.toml`
- PyPI package README: `pypi/README.md`
- SDK PyPI package metadata: `sdk/python/pyproject.toml`
- SDK Ruby gem auth version: `sdk/gems/auth/lib/auth/version.rb`
- SDK Ruby gem auth native extension: `sdk/gems/auth/ext/auth/Cargo.toml`
- SDK Ruby gem model version: `sdk/gems/model/lib/model/version.rb`
- SDK Ruby gem model native extension: `sdk/gems/model/ext/model/Cargo.toml`

## Version sync rules

The SDK npm package `@smbcloud/sdk-auth` must have its version in `sdk/npm/smbcloud-auth/package.json` match the version in `crates/smbcloud-auth-sdk-wasm/Cargo.toml` exactly. The `prepare-package.mjs` script enforces this at build time and will fail CI if they diverge.

When bumping workspace crate versions for a release, also update the version constraints in the root `Cargo.toml` under `[workspace.dependencies]` so every `smbcloud-*` path dependency points at the same release version.

When bumping workspace crate versions for a release, always update `sdk/npm/smbcloud-auth/package.json` in the same commit.

The npm wrapper package is generated, but its checked-in files still need to match the release version before tagging:

- rerender `npm/smbcloud-cli/package.json` with `node ../scripts/render-main-package.cjs ./package.json <version>`
- refresh `npm/smbcloud-cli/package-lock.json` with `npm install`
- commit both files so CI and local packaging agree on the wrapper version and optional dependency versions

The same applies to the Ruby gems in `sdk/gems/`. For each gem (`auth`, `model`):

- `lib/<gem>/version.rb` — the gem version constant
- `ext/<gem>/Cargo.toml` — the native extension crate version AND the `smbcloud-*` dependency version constraints (e.g. `"0.3"` → `"0.4"`)
- Regenerate `Cargo.lock` with `cargo generate-lockfile` and `Gemfile.lock` with `bundle lock` inside the gem directory

## Release branch convention

Every release is prepared on a dedicated branch named **`release/v<version>`**, branched
off `development`. Never prepare a release directly on `development`, and never tag a
release on the `release/*` (or any feature) branch.

Reasoning:

- The release-prep commits (version bump, lockfiles, generated metadata) can be reviewed
  and run through CI in isolation before they touch the mainline.
- The release commit history stays clean and linear on the default branch.
- `cargo workspaces publish --allow-branch "*"` accepts any branch, but downstream
  workflows dispatch from the tag ref, so the tagged commit must contain all intended changes.
- Tagging on a feature branch leaves `development` without the release commit and makes git
  history confusing.

Workflow:

1. Branch off an up-to-date `development`: `git checkout development && git pull && git checkout -b release/v<version>`.
2. Prepare the release on that branch — `make patch|minor|major|custom VERSION=<version>`
   (see the version-sync rules above; `make` commits `Release <version>` on the branch).
3. Push the branch and let CI run: `git push origin release/v<version>`.
4. **Only when CI is green**, merge into `development` with `--no-ff`:
   `git checkout development && git merge --no-ff release/v<version>`.
5. Delete the release branch (local and remote):
   `git branch -d release/v<version> && git push origin --delete release/v<version>`.
6. Tag on `development`: `git tag v<version>`.
7. Push both: `git push origin development && git push origin v<version>` — the tag push
   triggers `release-crate.yml` and the full publish chain.

If a tag was placed on the wrong commit (e.g. before a last-minute fix), move it:

1. Merge the fix into `development`.
2. Recreate the annotated tag on the correct commit: `git tag -fa v<version> -m "v<version>"`.
3. Force-push the tag: `git push origin v<version> --force`.

When verifying the remote tag, remember that annotated tags have two refs. `git ls-remote --tags origin refs/tags/v<version>*` should show:

- `refs/tags/v<version>` — the tag object
- `refs/tags/v<version>^{}` — the peeled commit

The peeled `^{}` ref is the one that must match the intended release commit.

The `release-crate.yml` orchestrator triggers on `push.tags: "v*.*.*"`, so the force-push will re-trigger the full release chain.

## Release model

There are two public distribution channels.

### 1. npm

npm has two package layers. Publish them in order.

Platform binary packages:

- `@smbcloud/cli-darwin-arm64`
- `@smbcloud/cli-darwin-x64`
- `@smbcloud/cli-linux-x64`
- `@smbcloud/cli-windows-arm64`
- `@smbcloud/cli-windows-x64`

Wrapper package:

- `@smbcloud/cli`

The wrapper package resolves the right platform package with `require.resolve(...)`.
Do not publish `@smbcloud/cli` before the required platform packages for that version exist.

### 2. PyPI

PyPI uses one package:

- `smbcloud-cli`

It is built from `pypi/pyproject.toml` with `maturin` and `bindings = "bin"`, so the published wheel installs the native `smb` executable directly.

## Local npm release workflow

### Preflight

1. Confirm npm auth with `npm whoami`.
2. Confirm the crate version in `crates/cli/Cargo.toml`.
3. Use the same version string for every npm package in that release.
4. Rerender `npm/smbcloud-cli/package.json` and refresh `npm/smbcloud-cli/package-lock.json` before tagging if the wrapper version changed.

### Build platform binaries

Build each target first:

- `cargo build --release --locked --target aarch64-apple-darwin`
- `cargo build --release --locked --target x86_64-apple-darwin`
- or the matching Linux or Windows target in CI

### Generate platform package

From `npm/`:

- run `node ./scripts/render-platform-package.cjs <pkg> <version> <os> <arch>`
- create `<pkg>/bin`
- copy the built `smb` binary into `bin/`
- on Windows, use `smb.exe`

Example:

- `node ./scripts/render-platform-package.cjs cli-darwin-arm64 0.3.39 darwin arm64`

### Publish order

1. Publish each platform package with `npm publish --access public`
2. Generate `npm/smbcloud-cli/package.json` with `render-main-package.cjs`
3. Run `npm install`
4. Run `npm run build`
5. Publish `@smbcloud/cli`

## Local PyPI release workflow

### Tooling

Use `uv` for local `maturin` installation in this repo.

Install:

- `uv tool install maturin`

Upgrade later if needed:

- `uv tool upgrade maturin`

If the shell cannot find `maturin`, use:

- `uv tool run maturin --version`

### Preflight

1. Confirm the crate version in `crates/cli/Cargo.toml`.
2. Make sure `.env` exists with `CLI_CLIENT_SECRET=...` because the Rust build reads it at compile time.
3. If publishing from a local machine, use a PyPI API token. Trusted publishing is for CI.

### Build and upload

From `pypi/`:

- `maturin build --release --locked --compatibility pypi --out dist`
- `maturin upload dist/*`

Or publish in one step:

- `maturin publish --release --locked --compatibility pypi`

For local uploads, export `MATURIN_PYPI_TOKEN` before running `maturin upload` or `maturin publish`.

### Publishing the smb stub (one-time)

The stub lives in `pypi/smb-stub/` and is built with `hatchling`, not `maturin`. It only needs to be published once — it never changes.

```sh
cd pypi/smb-stub
pip install hatchling build twine
python -m build
twine upload dist/*
```

Or with uv:

```sh
cd pypi/smb-stub
uv run --with build python -m build
uv run --with twine twine upload dist/*
```

Use a PyPI API token with upload scope for `smb`. Trusted publishing is not required for a static stub.

Do not republish the stub on every CLI release. It permanently depends on the unpinned `smbcloud-cli`, so uv always resolves the latest version when the tool environment is created or upgraded.

### Local publishing constraint

A local publish normally builds only for the current platform.

Do not claim a full PyPI release is complete from one machine unless you intentionally published only one platform or separately produced all required wheels.

## npm 2FA

Publishing may fail with `EOTP`.

When that happens, retry the exact `npm publish` command with:

- `--otp=<code>`

Do not assume the first publish failed before the package upload step. Check the exit result or npm registry state before retrying repeatedly.

## Trusted publishing

### npm

Prefer npm trusted publishers for CI over `NPM_TOKEN`.

Official references:

- https://docs.npmjs.com/trusted-publishers/
- https://docs.npmjs.com/cli/v11/commands/npm-trust/

### npm-side constraints

Trusted publishing can only be configured for packages that already exist on npm.

That means:

- publish a package once manually if it does not exist yet
- then configure trust for that package
- each package can have only one trusted publisher configuration at a time

For `smbcloud-cli`, configure trust per package, not only for `@smbcloud/cli`.

Example packages:

- `@smbcloud/cli`
- `@smbcloud/cli-darwin-arm64`
- later, each additional published `@smbcloud/cli-*` package

### GitHub Actions setup

The workflow must include:

- `permissions:`
- `id-token: write`
- `contents: read`

Do not pass `NODE_AUTH_TOKEN` to `npm publish` when using trusted publishing.

The npm CLI detects the GitHub OIDC environment automatically and exchanges it for a short-lived publish credential.

### Trusted publisher command

For this repo, the trust relationship should point at:

- repository: `smbcloudXYZ/smbcloud-cli`
- workflow file: `release-npm.yml`

Example:

- `npm trust github @smbcloud/cli --repo smbcloudXYZ/smbcloud-cli --file release-npm.yml --yes`
- `npm trust github @smbcloud/cli-darwin-arm64 --repo smbcloudXYZ/smbcloud-cli --file release-npm.yml --yes`

These commands may require npm 2FA and support `--otp=<code>`.

### Migration guidance

Recommended order:

1. enable trusted publishing for existing packages
2. verify a CI publish works through OIDC
3. then remove or stop using `NPM_TOKEN`
4. after the migration is proven, tighten npm package publishing access if desired

Do not remove the manual publish path for brand-new package names, because npm trust cannot be configured before the first publish.

### PyPI

Use trusted publishing in GitHub Actions through `pypa/gh-action-pypi-publish@release/v1`.
Do not use trusted publishing as the default local-machine publish path.

## CI/CD workflow rules

### Shared rules

For both release workflows:

- `runs-on` must use `matrix.build.OS` when the matrix uses upper-case keys
- derive `RELEASE_VERSION` from the git tag when `GITHUB_REF_TYPE=tag`
- for `workflow_dispatch`, fall back to the version in `crates/cli/Cargo.toml`
- read the Rust toolchain from `rust-toolchain.toml`
- pass that exact toolchain into `dtolnay/rust-toolchain`
- run `rustup target add <target> --toolchain ${{ env.RUST_TOOLCHAIN }}` before cross-target builds

### npm workflow

The npm workflow lives in `.github/workflows/release-npm.yml`.

Important rules:

- avoid `envsubst` for package generation across runners
- use the Node generator scripts instead
- use `npm install` and `npm run build` for the wrapper package job

### PyPI workflow

The PyPI workflow lives in `.github/workflows/release-pypi.yml`.

Important rules:

- build wheels with `PyO3/maturin-action@v1`
- use `working-directory: pypi`
- pass `target: ${{ matrix.build.TARGET }}`
- use `manylinux: ${{ matrix.build.MANYLINUX || 'off' }}`
- build the sdist in a separate job
- publish only after wheel and sdist artifacts are downloaded into one directory

## Rust toolchain consistency

When cross-compiling in CI, the Rust toolchain used for `rustup target add` must match the toolchain used by `cargo build` or `maturin`.

For this repo, `rust-toolchain.toml` is the only Rust version source of truth.

If the workflow installs a target for one toolchain but builds with another, CI can fail with:

- `error[E0463]: can't find crate for core`

## macOS cross-build constraint

On an Apple Silicon host, `x86_64-apple-darwin` may fail due to `openssl-sys` if only arm64 Homebrew OpenSSL exists under `/opt/homebrew`.

If that happens:

- do not claim the x64 package is releasable from that machine
- either build on an Intel Mac
- or use a Rosetta or x86 Homebrew toolchain with Intel OpenSSL under `/usr/local/opt/openssl@3`
- or publish only the arm64 platform package and explicitly avoid calling the release complete

Do not publish `@smbcloud/cli` for a version unless you accept that missing platform packages for that version can break installs on those platforms.

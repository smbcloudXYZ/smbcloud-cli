---
name: smbcloud-cli-release
description: Use when building, packaging, or releasing smbCloud CLI binaries through npm, PyPI, or GitHub Actions CI/CD. Covers the Rust binary release source of truth, the npm wrapper and platform packages, the PyPI `maturin` package under `packaging/pypi/smbcloud-cli/`, local publishing with `uv tool install maturin`, and fixes to `.github/workflows/release-npm.yml` or `.github/workflows/release-pypi.yml`.
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
- npm platform package generator: `packaging/npm/scripts/render-platform-package.cjs`
- npm wrapper package generator: `packaging/npm/scripts/render-main-package.cjs`
- npm wrapper launcher: `packaging/npm/smbcloud-cli/src/index.ts`
- npm wrapper committed package metadata: `packaging/npm/smbcloud-cli/package.json`
- npm wrapper committed lockfile: `packaging/npm/smbcloud-cli/package-lock.json`
- PyPI package metadata: `packaging/pypi/smbcloud-cli/pyproject.toml`
- PyPI package README: `packaging/pypi/smbcloud-cli/README.md`
- SDK PyPI package metadata: `sdk/python/pyproject.toml`
- SDK Ruby gem auth version: `sdk/gems/auth/lib/auth/version.rb`
- SDK Ruby gem auth native extension: `sdk/gems/auth/ext/auth/Cargo.toml`
- SDK Ruby gem model version: `sdk/gems/model/lib/model/version.rb`
- SDK Ruby gem model native extension: `sdk/gems/model/ext/model/Cargo.toml`
- SDK Ruby gem email version: `sdk/gems/email/lib/email/version.rb`
- SDK Ruby gem email native extension: `sdk/gems/email/ext/email/Cargo.toml`

## Version sync rules

The SDK npm package `@smbcloud/sdk-auth` must have its version in `sdk/npm/smbcloud-auth/package.json` match the version in `crates/smbcloud-auth-sdk-wasm/Cargo.toml` exactly. The `prepare-package.mjs` script enforces this at build time and will fail CI if they diverge.

When bumping workspace crate versions for a release, also update the version constraints in the root `Cargo.toml` under `[workspace.dependencies]` so every `smbcloud-*` path dependency points at the same release version.

When bumping workspace crate versions for a release, always update `sdk/npm/smbcloud-auth/package.json` in the same commit.

The npm wrapper package is generated, but its checked-in files still need to match the release version before tagging:

- rerender `packaging/npm/smbcloud-cli/package.json` with `node ../scripts/render-main-package.cjs ./package.json <version>`
- refresh `packaging/npm/smbcloud-cli/package-lock.json` with `npm install`
- commit both files so CI and local packaging agree on the wrapper version and optional dependency versions

The same applies to the Ruby gems in `sdk/gems/`. For each gem (`auth`, `model`, `email`):

- `lib/<gem>/version.rb` — the gem version constant
- `ext/<gem>/Cargo.toml` — the native extension crate version AND the `smbcloud-*` dependency version constraints (e.g. `"0.3"` → `"0.4"`)
- Regenerate `Cargo.lock` with `cargo generate-lockfile` and `Gemfile.lock` with `bundle lock` inside the gem directory

## Branches and releases

Branch roles:

- `main` is the default branch. It only receives release merges and patch cherry-picks,
  and every release tag lives on a `main` commit.
- `development` is the integration branch. Feature and fix branches start from it and
  pull requests target it.
- `feature/<slug>` and `fix/<slug>` hold day-to-day work.
- `release/v<version>` holds a single release. `make patch|minor|major|custom` only runs
  on these branches.

Never prepare a release directly on `development` or `main`, and never tag on a
`release/*` or feature branch. The release-prep commits (version bump, lockfiles,
generated metadata) are reviewed and run through CI on their own branch first, and
downstream workflows dispatch from the tag ref, so the tagged commit must contain every
intended change.

### Feature flow

1. `git checkout development && git pull && git checkout -b feature/<slug>`.
2. Open the pull request against `development`.

### Minor or major release

1. Branch off an up-to-date `development`:
   `git checkout development && git pull && git checkout -b release/v<version>`.
2. Prepare the release: `make minor|major|custom VERSION=<version>` (see the version-sync
   rules above; `make` commits `Release <version>` on the branch). Run it through `rv`
   if the Ruby gem lockfiles are regenerated: `rv run make minor`.
3. Push the branch and let CI run: `git push origin release/v<version>`.
4. **Only when CI is green**, merge into both branches with `--no-ff`
   (open one pull request per target and use the merge-commit method, or merge locally):
   `development` first, then `main`.
5. Tag the `main` merge commit with an annotated tag and push it:
   `git checkout main && git pull && git tag -a v<version> -m "v<version>" && git push origin v<version>`.
   The tag push triggers `release-crate.yml` and the full publish chain.
6. Delete the release branch (local and remote):
   `git branch -d release/v<version> && git push origin --delete release/v<version>`.

### Patch release

1. Land the fix on `development` through a normal pull request.
2. Branch off `main`: `git checkout main && git pull && git checkout -b release/v<version>`.
3. Cherry-pick the fix: `git cherry-pick -x <fix-commit>`.
4. `make patch`, push the branch and wait for CI.
5. Merge the release branch into `main` with `--no-ff`, then tag `main` as above.
6. Merge the release branch back into `development` with `--no-ff`. This carries the
   version bump; the fix is already there.
7. Delete the release branch.

### Moving a misplaced tag

If a tag was placed on the wrong commit (for example before a last-minute fix):

1. Merge the fix into `main`.
2. Recreate the annotated tag on the correct commit: `git tag -fa v<version> -m "v<version>"`.
3. Force-push the tag: `git push origin v<version> --force`.

When verifying the remote tag, remember that annotated tags have two refs. `git ls-remote --tags origin refs/tags/v<version>*` should show:

- `refs/tags/v<version>` — the tag object
- `refs/tags/v<version>^{}` — the peeled commit

The peeled `^{}` ref is the one that must match the intended release commit, which is the
`main` merge commit.

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

It is built from `packaging/pypi/smbcloud-cli/pyproject.toml` with `maturin` and `bindings = "bin"`, so the published wheel installs the native `smb` executable directly.

## Local npm release workflow

### Preflight

1. Confirm npm auth with `npm whoami`.
2. Confirm the crate version in `crates/cli/Cargo.toml`.
3. Use the same version string for every npm package in that release.
4. Rerender `packaging/npm/smbcloud-cli/package.json` and refresh `packaging/npm/smbcloud-cli/package-lock.json` before tagging if the wrapper version changed.

### Build platform binaries

Build each target first:

- `cargo build --release --locked --target aarch64-apple-darwin`
- `cargo build --release --locked --target x86_64-apple-darwin`
- or the matching Linux or Windows target in CI

### Generate platform package

From `packaging/npm/`:

- run `node ./scripts/render-platform-package.cjs <pkg> <version> <os> <arch>`
- create `<pkg>/bin`
- copy the built `smb` binary into `bin/`
- on Windows, use `smb.exe`

Example:

- `node ./scripts/render-platform-package.cjs cli-darwin-arm64 0.3.39 darwin arm64`

### Publish order

1. Publish each platform package with `npm publish --access public`
2. Generate `packaging/npm/smbcloud-cli/package.json` with `render-main-package.cjs`
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

From `packaging/pypi/smbcloud-cli/`:

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

### RubyGems

The gems publish from `release-sdk-gem.yml` over OIDC, with no API key. Each gem needs a trusted publisher on rubygems.org for repository `smbcloudXYZ/smbcloud-cli` and workflow `release-sdk-gem.yml`, environment blank. The filename must be the workflow that contains the push job: a publisher registered to `release-crate.yml`, which only dispatches it, fails with `No trusted publisher configured for this workflow`.

A new gem needs a pending publisher (12-hour expiry) created right before the release. If a push job fails, fix the publisher and run `gh run rerun <run-id> --failed`. See `docs/gems.md`.

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

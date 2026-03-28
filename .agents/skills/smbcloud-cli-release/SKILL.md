---
name: smbcloud-cli-release
description: Use when building, packaging, or releasing smbCloud CLI binaries through npm or GitHub Actions CI/CD. Covers the Rust binary packages under `npm/cli-*`, the `@smbcloud/cli` wrapper package, macOS arm64 vs x64 constraints, npm 2FA publishing, and fixes to `.github/workflows/release-npm.yml`.
---

# smbcloud CLI Release

Use this skill when work touches any part of the smbCloud CLI distribution flow:

- local npm release for `smbcloud-cli`
- GitHub Actions npm release automation
- Rust binary packaging into scoped npm platform packages
- release failures caused by macOS cross-compilation, missing package metadata, or npm 2FA

## Release model

There are two npm package layers. Publish them in order.

### 1. Platform binary packages

These packages contain the compiled Rust binary only:

- `@smbcloud/cli-darwin-arm64`
- `@smbcloud/cli-darwin-x64`
- `@smbcloud/cli-linux-x64`
- `@smbcloud/cli-windows-arm64`
- `@smbcloud/cli-windows-x64`

Their package directories are generated under `npm/cli-*/` and should be treated as release artifacts, not source.

### 2. Wrapper package

The public package is:

- `@smbcloud/cli`

It contains the Node launcher in `npm/smbcloud-cli/src/index.ts` and resolves the correct platform package with `require.resolve(...)`.

Do not publish `@smbcloud/cli` before the required platform packages for that version exist.

## Source of truth

Use these files as the release source of truth:

- Rust crate version: `crates/cli/Cargo.toml`
- npm release workflow: `.github/workflows/release-npm.yml`
- platform package generator: `npm/scripts/render-platform-package.cjs`
- wrapper package generator: `npm/scripts/render-main-package.cjs`
- wrapper launcher: `npm/smbcloud-cli/src/index.ts`

Do not hand-edit generated `npm/cli-*/package.json` files unless debugging a broken release artifact.

## Local release workflow

### Preflight

1. Confirm npm auth with `npm whoami`.
2. Confirm the crate version in `crates/cli/Cargo.toml`.
3. Use the same version string for every npm package in that release.

### Build platform binaries

Build each target first:

- `cargo build --release --locked --target aarch64-apple-darwin`
- `cargo build --release --locked --target x86_64-apple-darwin`
- or the matching Linux/Windows target in CI

### Generate platform package

From `npm/`:

- run `node ./scripts/render-platform-package.cjs <pkg> <version> <os> <arch>`
- create `<pkg>/bin`
- copy the built `smb` binary into `bin/`
- on Windows, use `smb.exe`

Example:

- `node ./scripts/render-platform-package.cjs cli-darwin-arm64 0.3.33 darwin arm64`

### Publish order

1. Publish each platform package with `npm publish --access public`
2. Generate `npm/smbcloud-cli/package.json` with `render-main-package.cjs`
3. Run `npm install`
4. Run `npm run build`
5. Publish `@smbcloud/cli`

## npm 2FA

Publishing may fail with `EOTP`.

When that happens, retry the exact `npm publish` command with:

- `--otp=<code>`

Do not assume the first publish failed before the package upload step. Check the exit result or npm registry state before retrying repeatedly.

## Trusted publishing

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

### Rust toolchain consistency

When cross-compiling in CI, the Rust toolchain used for `rustup target add` must match the toolchain used by `cargo build`.

For this repo, check `rust-toolchain.toml` first and keep the workflow matrix aligned with it.

If the workflow installs a target for one toolchain but Cargo builds with another, CI can fail with:

- `error[E0463]: can't find crate for core`
- note that the target may not be installed

This can happen even when `rustup target add <target>` already ran successfully.

Preferred pattern:

- install the requested toolchain explicitly
- run `rustup target add <target> --toolchain <toolchain>`
- run `cargo +<toolchain> build --target <target>`

Do not rely on plain `cargo build` if the repo pin in `rust-toolchain.toml` can differ from the matrix toolchain version.

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

## CI/CD workflow rules

The release workflow lives in `.github/workflows/release-npm.yml`.

Important rules:

- `runs-on` must use `matrix.build.OS`, not `matrix.build.os`
- derive `RELEASE_VERSION` from the git tag when `GITHUB_REF_TYPE=tag`
- for `workflow_dispatch`, fall back to the version in `crates/cli/Cargo.toml`
- avoid `envsubst` for package generation across runners
- use the Node generator scripts instead
- use `npm install` and `npm run build` for the wrapper package job

### Package generation

Prefer deterministic Node scripts over shell templating.

Reasons:

- `envsubst` is not guaranteed across macOS and Windows runners
- package metadata must stay identical between local release and CI
- the generated wrapper package must point optional dependencies at the exact release version

## macOS cross-build constraint

On an Apple Silicon host, `x86_64-apple-darwin` may fail due to `openssl-sys` if only arm64 Homebrew OpenSSL exists under `/opt/homebrew`.

Typical symptom:

- `openssl-sys` cannot find an `x86_64-apple-darwin` OpenSSL

If that happens:

- do not claim the x64 package is releasable from that machine
- either build on an Intel Mac
- or use a Rosetta/x86 Homebrew toolchain with Intel OpenSSL under `/usr/local/opt/openssl@3`
- or publish only the arm64 platform package and explicitly avoid calling the release complete

Do not publish `@smbcloud/cli` for a version unless you accept that missing platform packages for that version can break installs on those platforms.

## Generated artifacts

Generated platform package directories should be git-ignored:

- `/npm/cli-*/`

These are build artifacts and should not be committed.

## Validation

Use the smallest checks that match the change.

### Local

- `npm whoami`
- `cargo build --release --locked --target <target>`
- `node npm/scripts/render-platform-package.cjs ...`
- `node npm/scripts/render-main-package.cjs ...`
- `npm install`
- `npm run build`

### CI/CD edits

After changing the workflow:

- inspect `.github/workflows/release-npm.yml`
- verify the package generators still output the expected names and versions
- confirm `@smbcloud/cli` optional dependencies match the platform package names exactly

## Common mistakes

- publishing `@smbcloud/cli` before the matching platform packages exist
- assuming `workflow_dispatch` has a tag-derived version
- relying on `envsubst` in a cross-platform GitHub Actions workflow
- forgetting that Windows binaries need `.exe`
- treating generated `npm/cli-*` directories as source files
- trying to cross-build macOS x64 on Apple Silicon without an Intel OpenSSL toolchain

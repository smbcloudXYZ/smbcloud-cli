# ADR

This repo uses [Architecture Decision Record](https://github.com/joelparkerhenderson/architecture-decision-record) in the `adr` folder.

# Releasing

Releases are **automated** via GitHub Actions. The full source of truth for the
release process is the `smbcloud-cli-release` skill under `.agents/skills/` — read
it before cutting a release. The short version:

1. Prepare the bump on a `release/v<version>` branch off `development` with the
   Makefile — `make patch | minor | major | custom VERSION=<version>`. This bumps
   every workspace crate in lockstep, syncs the npm/PyPI/gem manifests and
   lockfiles, and commits `Release <version>`.
2. Once CI is green, merge into `development` with `--no-ff`, then tag on
   `development`: `git tag v<version>`.
3. Push both: `git push origin development && git push origin v<version>`.

Pushing the `v*.*.*` tag triggers `.github/workflows/release-crate.yml`, which
publishes the workspace crates to crates.io and fans out to the npm, PyPI, NuGet,
and Ruby-gem distribution workflows. **You do not publish by hand** — the tag push
is the trigger.

A local publish (`cargo workspaces publish --publish-as-is`) exists only as a
fallback for when CI is unavailable; it is not the normal path. See the
`smbcloud-cli-release` skill for per-channel details, version-sync rules, and
trusted-publishing setup.

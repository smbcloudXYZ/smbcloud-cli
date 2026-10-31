# ADR

This repo uses [Architecture Decision Record](https://github.com/joelparkerhenderson/architecture-decision-record) in the `adr` folder.

# Releasing

Releases are **automated** via GitHub Actions. The full source of truth for the
release process is the `smbcloud-cli-release` skill under `.agents/skills/` — read
it before cutting a release. The short version:

1. Prepare the bump on a `release/v<version>` branch with the Makefile:
   `make patch | minor | major | custom VERSION=<version>`. Branch off `development`
   for a minor or major release. For a patch, branch off `main` and
   `git cherry-pick -x` the fix, which already landed on `development`. The Makefile
   bumps every workspace crate in lockstep, syncs the npm/PyPI/gem manifests and
   lockfiles, and commits `Release <version>` without tagging.
2. Push the branch. Once CI is green, merge it into `development` and `main` with
   `--no-ff`.
3. Tag the `main` merge commit with an annotated tag and push it:
   `git checkout main && git pull && git tag -a v<version> -m "v<version>" && git push origin v<version>`.
4. For a patch release, the `development` merge carries the version bump back; the fix
   is already there. Delete the release branch.

Pushing the `v*.*.*` tag triggers `.github/workflows/release-crate.yml`, which
publishes the workspace crates to crates.io and fans out to the npm, PyPI, NuGet,
and Ruby-gem distribution workflows. **You do not publish by hand** — the tag push
is the trigger.

A local publish (`cargo workspaces publish --publish-as-is`) exists only as a
fallback for when CI is unavailable; it is not the normal path. See the
`smbcloud-cli-release` skill for per-channel details, version-sync rules, and
trusted-publishing setup.

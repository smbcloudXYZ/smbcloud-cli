.PHONY: help release patch minor major custom sync-release-metadata regenerate-release-lockfiles check-release-versions

help:
	@echo "Release commands:"
	@echo "  make patch"
	@echo "  make minor"
	@echo "  make major"
	@echo "  make custom VERSION=0.4.14"

release:
	@test -n "$(BUMP)" || (echo "BUMP is required" && exit 1)
	@if [ "$(BUMP)" = "custom" ] && [ -z "$(VERSION)" ]; then echo "VERSION is required for custom releases"; exit 1; fi
	@case "$$(git branch --show-current)" in release/v*) ;; *) echo "Releases must be prepared on a release/v<version> branch. Branch off development for a minor or major release, or off main for a patch."; exit 1;; esac
	@if [ -n "$$(git status --short --untracked-files=all)" ]; then echo "Working tree must be clean before running a release"; exit 1; fi
	@if [ "$(BUMP)" = "custom" ]; then \
		cargo workspaces version custom "$(VERSION)" --yes --no-git-commit --force "*"; \
	else \
		cargo workspaces version "$(BUMP)" --yes --no-git-commit --force "*"; \
	fi
	@$(MAKE) sync-release-metadata
	@$(MAKE) regenerate-release-lockfiles
	@$(MAKE) check-release-versions
	@release_version=$$(sed -n 's/^version = "\(.*\)"/\1/p' crates/cli/Cargo.toml | head -n 1); \
	if [ -z "$$release_version" ]; then echo "Unable to determine release version from crates/cli/Cargo.toml"; exit 1; fi; \
	release_branch=$$(git branch --show-current); \
	git add -A; \
	git commit -m "Release $$release_version"; \
	echo "Release $$release_version prepared on $$release_branch (not tagged)."; \
	echo "Next: git push origin $$release_branch"; \
	echo "  CI green: merge $$release_branch into development and main with --no-ff"; \
	echo "  then:     git checkout main && git pull && git tag -a v$$release_version -m \"v$$release_version\" && git push origin v$$release_version"

sync-release-metadata:
	@node ./scripts/sync-release-version.mjs

regenerate-release-lockfiles:
	@# Ruby extension Cargo.lock files resolve crates.io dependencies that are
	@# published only after this release tag is pushed. The SDK gem workflow waits
	@# for those crates and resolves them during its build, so do not attempt to
	@# regenerate the locks while preparing the tag locally.
	@BUNDLE_GEMFILE=sdk/gems/auth/Gemfile bundle lock
	@BUNDLE_GEMFILE=sdk/gems/email/Gemfile bundle lock
	@BUNDLE_GEMFILE=sdk/gems/model/Gemfile bundle lock

check-release-versions:
	@node ./scripts/check-release-versions.mjs

patch:
	@$(MAKE) release BUMP=patch

minor:
	@$(MAKE) release BUMP=minor

major:
	@$(MAKE) release BUMP=major

custom:
	@$(MAKE) release BUMP=custom VERSION="$(VERSION)"

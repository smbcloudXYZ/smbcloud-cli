.PHONY: help release patch minor major custom sync-release-metadata regenerate-release-lockfiles

help:
	@echo "Release commands:"
	@echo "  make patch"
	@echo "  make minor"
	@echo "  make major"
	@echo "  make custom VERSION=0.3.40"

release:
	@test -n "$(BUMP)" || (echo "BUMP is required" && exit 1)
	@if [ "$(BUMP)" = "custom" ] && [ -z "$(VERSION)" ]; then echo "VERSION is required for custom releases"; exit 1; fi
	@if [ "$$(git branch --show-current)" != "development" ]; then echo "Releases must be prepared on the development branch"; exit 1; fi
	@if [ -n "$$(git status --short --untracked-files=all)" ]; then echo "Working tree must be clean before running a release"; exit 1; fi
	@if [ "$(BUMP)" = "custom" ]; then \
		cargo workspaces version custom "$(VERSION)" --yes --no-git-commit --no-git-tag --no-git-push --allow-branch "*" --force "*"; \
	else \
		cargo workspaces version "$(BUMP)" --yes --no-git-commit --no-git-tag --no-git-push --allow-branch "*" --force "*"; \
	fi
	@$(MAKE) sync-release-metadata
	@$(MAKE) regenerate-release-lockfiles
	@release_version=$$(sed -n 's/^version = "\(.*\)"/\1/p' crates/cli/Cargo.toml | head -n 1); \
	if [ -z "$$release_version" ]; then echo "Unable to determine release version from crates/cli/Cargo.toml"; exit 1; fi; \
	git add -A; \
	git commit -m "Release $$release_version"; \
	git tag "v$$release_version"; \
	echo "Release $$release_version prepared locally."; \
	echo "Next: git push origin development && git push origin v$$release_version"

sync-release-metadata:
	@node ./scripts/sync-release-version.mjs

regenerate-release-lockfiles:
	@cargo generate-lockfile --manifest-path sdk/gems/auth/Cargo.toml
	@cargo generate-lockfile --manifest-path sdk/gems/model/Cargo.toml
	@bundle lock --gemfile sdk/gems/auth/Gemfile
	@bundle lock --gemfile sdk/gems/model/Gemfile

patch:
	@$(MAKE) release BUMP=patch

minor:
	@$(MAKE) release BUMP=minor

major:
	@$(MAKE) release BUMP=major

custom:
	@$(MAKE) release BUMP=custom VERSION="$(VERSION)"

.PHONY: help release patch minor major custom

help:
	@echo "Release commands:"
	@echo "  make patch"
	@echo "  make minor"
	@echo "  make major"
	@echo "  make custom VERSION=0.3.40"

release:
	@test -n "$(BUMP)" || (echo "BUMP is required" && exit 1)
	@if [ "$(BUMP)" = "custom" ] && [ -z "$(VERSION)" ]; then echo "VERSION is required for custom releases"; exit 1; fi
	@if [ "$(BUMP)" = "custom" ]; then \
		cargo workspaces version custom "$(VERSION)" --yes --no-git-push --allow-branch "*" --force "*"; \
	else \
		cargo workspaces version "$(BUMP)" --yes --no-git-push --allow-branch "*" --force "*"; \
	fi
	@echo "Release version prepared locally."
	@echo "Next: git push origin $$(git branch --show-current) && git push origin --tags"

patch:
	@$(MAKE) release BUMP=patch

minor:
	@$(MAKE) release BUMP=minor

major:
	@$(MAKE) release BUMP=major

custom:
	@$(MAKE) release BUMP=custom VERSION="$(VERSION)"

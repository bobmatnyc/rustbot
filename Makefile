.PHONY: help build run release version-show version-patch version-minor version-major version-build clean test

help:
	@echo "Rustbot Development Commands"
	@echo ""
	@echo "Building:"
	@echo "  make build          - Build debug version"
	@echo "  make run            - Build and run application"
	@echo "  make release        - Build release version (optimized)"
	@echo ""
	@echo "Version Management:"
	@echo "  make version-show   - Show current version"
	@echo "  make version-patch  - Bump patch version (0.2.6 -> 0.2.7)"
	@echo "  make version-minor  - Bump minor version (0.2.6 -> 0.3.0)"
	@echo "  make version-major  - Bump major version (0.2.6 -> 1.0.0)"
	@echo "  make version-build  - Bump build number (0001 -> 0002)"
	@echo ""
	@echo "Development:"
	@echo "  make test           - Run tests"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make watch          - Auto-rebuild on changes"
	@echo ""
	@echo "Git Workflow:"
	@echo "  make commit MSG='your message'  - Add, commit, and show status"
	@echo "  make push           - Push to origin/main"
	@echo "  make release-patch  - Full workflow: patch bump + commit + push"

# Building
build:
	cargo build

run:
	cargo run --bin rustbot

release:
	cargo build --release

# Version Management
version-show:
	@python3 scripts/manage_version.py show

version-patch:
	@python3 scripts/manage_version.py bump patch

version-minor:
	@python3 scripts/manage_version.py bump minor

version-major:
	@python3 scripts/manage_version.py bump major

version-build:
	@python3 scripts/manage_version.py bump build

# Development
test:
	cargo test

clean:
	cargo clean

watch:
	cargo watch -x run -w agents

# Git Workflow
commit:
ifndef MSG
	@echo "Error: MSG is required. Usage: make commit MSG='your commit message'"
	@exit 1
endif
	@git add -A
	@git commit -m "$(MSG)"
	@git status

push:
	git push origin main

# Combined workflows
release-patch: version-patch
	@echo "\n📦 Creating release commit..."
	@VERSION=$$(grep 'pub const VERSION' src/version.rs | sed 's/.*"\([^"]*\)".*/\1/'); \
	git add -A && \
	git commit -m "chore: release v$$VERSION" && \
	git push origin main && \
	echo "\n✅ Released v$$VERSION and pushed to origin/main"

release-minor: version-minor
	@echo "\n📦 Creating release commit..."
	@VERSION=$$(grep 'pub const VERSION' src/version.rs | sed 's/.*"\([^"]*\)".*/\1/'); \
	git add -A && \
	git commit -m "chore: release v$$VERSION" && \
	git push origin main && \
	echo "\n✅ Released v$$VERSION and pushed to origin/main"

release-major: version-major
	@echo "\n📦 Creating release commit..."
	@VERSION=$$(grep 'pub const VERSION' src/version.rs | sed 's/.*"\([^"]*\)".*/\1/'); \
	git add -A && \
	git commit -m "chore: release v$$VERSION" && \
	git push origin main && \
	echo "\n✅ Released v$$VERSION and pushed to origin/main"

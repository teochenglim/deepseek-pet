.DEFAULT_GOAL := help
NAME := deepseek-pet
# Single source of truth for the version.
VERSION_CURRENT := $(shell cat VERSION 2>/dev/null || echo 0.0.0)
APP := desktop/cross-platform
TAURI := $(APP)/src-tauri
MANIFEST := $(TAURI)/Cargo.toml
# WebM source (read-only reference clone); override with FISH_DIR=...
FISH_DIR ?= big-blue-fish-desktop-pet/code/BigBlueFish.Windows/assets/fish

.PHONY: help
help: ## Show this menu
	@echo "$(NAME) $(VERSION_CURRENT) - available targets:"
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Release cycle (USER ONLY): make release VERSION=x.y.z"

.PHONY: setup
setup: ## Install frontend deps (npm ci in desktop/)
	cd $(APP) && npm ci

.PHONY: build
build: ## Build the macOS .app / .dmg bundle (needs npm + tauri CLI)
	cd $(APP) && npx tauri build

.PHONY: run
run: ## Run the pet in dev mode
	cd $(APP) && npx tauri dev

.PHONY: test
test: ## Rust tests (AC config tests + unit tests) and hook tests
	cargo test --locked --manifest-path $(MANIFEST)
	cd $(APP) && npm test --silent
	python3 -m unittest tools/test_convert_assets.py

.PHONY: ac-map
ac-map: ## Traceability gate: every requirement -> AC -> >=2 tests
	./scripts/ac-map.sh

.PHONY: fmt
fmt: ## Format Rust
	cargo fmt --manifest-path $(MANIFEST) --all

.PHONY: fmt-check
fmt-check: ## Check Rust formatting
	cargo fmt --manifest-path $(MANIFEST) --all -- --check

.PHONY: clippy
clippy: ## Lint (warnings are errors)
	cargo clippy --manifest-path $(MANIFEST) --all-targets --locked -- -D warnings

.PHONY: lint-frontend
lint-frontend: ## eslint + prettier on desktop/
	cd $(APP) && npx prettier --check . && npx eslint src

.PHONY: check
check: fmt-check clippy lint-frontend ac-map test version-check ## Everything CI runs; green before claiming anything works

.PHONY: convert-assets
convert-assets: ## (0.2.0) WebM -> frames; needs ffmpeg + Pillow
	python3 tools/convert-assets.py --input $(FISH_DIR) --output assets/big-blue-fish

.PHONY: clean
clean: ## Remove cargo build artifacts
	cargo clean --manifest-path $(MANIFEST)

.PHONY: github-action-bump
github-action-bump: ## Pin workflow actions to full SHAs
	python3 scripts/pin-actions.py

.PHONY: audit
audit: ## cargo-audit + cargo-deny (install separately)
	cd $(TAURI) && cargo audit && cargo deny --manifest-path Cargo.toml check

.PHONY: version
version: ## Print VERSION
	@echo $(VERSION_CURRENT)

.PHONY: version-check
version-check: ## VERSION must equal Cargo.toml and tauri.conf.json versions
	@test "$$(grep -m1 '^version' $(MANIFEST) | sed 's/.*"\(.*\)"/\1/')" = "$(VERSION_CURRENT)" || { echo "VERSION != Cargo.toml"; exit 1; }
	@grep -q '"version": "$(VERSION_CURRENT)"' $(TAURI)/tauri.conf.json || { echo "VERSION != tauri.conf.json"; exit 1; }

.PHONY: bump
bump: ## Rewrite VERSION, Cargo.toml, tauri.conf.json (VERSION=x.y.z)
	@if [ -z "$(VERSION)" ]; then echo "Usage: make bump VERSION=x.y.z"; exit 1; fi
	@echo "$(VERSION)" > VERSION
	@perl -0pi -e 's/^version = ".*?"/version = "$(VERSION)"/m' $(MANIFEST)
	@perl -0pi -e 's/"version": ".*?"/"version": "$(VERSION)"/' $(TAURI)/tauri.conf.json
	@cargo update --manifest-path $(MANIFEST) -w >/dev/null 2>&1 || true
	@echo "VERSION -> $(VERSION)"

.PHONY: release
release: ## Bump, amend HEAD, push, tag, push tag -> CI (USER ONLY)
	@if [ -z "$(VERSION)" ]; then echo "Usage: make release VERSION=x.y.z"; exit 1; fi
	$(MAKE) bump VERSION=$(VERSION)
	git add VERSION $(MANIFEST) $(TAURI)/Cargo.lock $(TAURI)/tauri.conf.json
	git commit --amend --no-edit
	git push origin HEAD
	git tag v$(VERSION)
	git push origin v$(VERSION)

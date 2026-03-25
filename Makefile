LAURUS_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="laurus") | .version')

USER_AGENT ?= $(shell curl --version | head -n1 | awk '{print $1"/"$2}')
USER ?= $(shell whoami)
HOSTNAME ?= $(shell hostname)

# ── Python venv ─────────────────────────────────────────────────────────────
PYTHON_VENV_DIR := laurus-python/.venv
PYTHON          := $(PYTHON_VENV_DIR)/bin/python
PIP             := $(PYTHON_VENV_DIR)/bin/pip
MATURIN         := $(PYTHON_VENV_DIR)/bin/maturin
PYTEST          := $(PYTHON_VENV_DIR)/bin/pytest

.DEFAULT_GOAL := help

help: ## Show help
	@echo "Available targets:"
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-30s %s\n", $$1, $$2}'

# ── Python venv setup ───────────────────────────────────────────────────────

$(PYTHON_VENV_DIR):
	python3 -m venv $(PYTHON_VENV_DIR)
	$(PIP) install --quiet --upgrade pip

venv: $(PYTHON_VENV_DIR) ## Create laurus-python venv and install dev dependencies
	$(PIP) install --quiet maturin pytest

venv-clean: ## Remove the laurus-python venv
	rm -rf $(PYTHON_VENV_DIR)

# ── Clean ──────────────────────────────────────────────────────────────────

clean: venv-clean ## Clean all build artifacts (including venv)
	cargo clean

# ── Format ─────────────────────────────────────────────────────────────────

format: ## Format all crates
	cargo fmt

format-laurus: ## Format laurus
	cargo fmt -p laurus

format-laurus-cli: ## Format laurus-cli
	cargo fmt -p laurus-cli

format-laurus-server: ## Format laurus-server
	cargo fmt -p laurus-server

format-laurus-mcp: ## Format laurus-mcp
	cargo fmt -p laurus-mcp

format-laurus-python: ## Format laurus-python
	cargo fmt -p laurus-python

# ── Lint ───────────────────────────────────────────────────────────────────

lint: ## Lint all crates
	cargo clippy --workspace --all-targets -- -D warnings

lint-laurus: ## Lint laurus
	cargo clippy -p laurus --all-targets -- -D warnings

lint-laurus-cli: ## Lint laurus-cli
	cargo clippy -p laurus-cli --all-targets -- -D warnings

lint-laurus-server: ## Lint laurus-server
	cargo clippy -p laurus-server --all-targets -- -D warnings

lint-laurus-mcp: ## Lint laurus-mcp
	cargo clippy -p laurus-mcp --all-targets -- -D warnings

lint-laurus-python: ## Lint laurus-python
	cargo clippy -p laurus-python -- -D warnings

# ── Test ───────────────────────────────────────────────────────────────────

test: ## Test all crates
	cargo test --workspace

test-laurus: ## Test laurus
	cargo test -p laurus

test-laurus-cli: ## Test laurus-cli
	cargo test -p laurus-cli

test-laurus-server: ## Test laurus-server
	cargo test -p laurus-server

test-laurus-mcp: ## Test laurus-mcp
	cargo test -p laurus-mcp

test-laurus-python: venv ## Test laurus-python (Rust unit tests + Python pytest)
	cargo test -p laurus-python
	cd laurus-python && VIRTUAL_ENV=$(abspath $(PYTHON_VENV_DIR)) $(abspath $(MATURIN)) develop --quiet && $(abspath $(PYTEST)) tests/ -v

# ── Build ──────────────────────────────────────────────────────────────────

build: ## Build all crates (release, all features)
	cargo build --release --all-features

build-laurus: ## Build laurus (release)
	cargo build -p laurus --release

build-laurus-cli: ## Build laurus-cli (release)
	cargo build -p laurus-cli --release

build-laurus-server: ## Build laurus-server (release)
	cargo build -p laurus-server --release

build-laurus-mcp: ## Build laurus-mcp (release)
	cargo build -p laurus-mcp --release

build-laurus-python: venv ## Build laurus-python wheel (release)
	cd laurus-python && VIRTUAL_ENV=$(abspath $(PYTHON_VENV_DIR)) $(abspath $(MATURIN)) build --release

# ── Benchmark ──────────────────────────────────────────────────────────────

bench: ## Benchmark the project
	cargo bench --bench bench

# ── Tag & Publish ──────────────────────────────────────────────────────────

tag: ## Make a new tag for the current version
	git tag v$(LAURUS_VERSION)
	git push origin v$(LAURUS_VERSION)

publish: ## Publish the crate to crates.io
ifeq ($(shell curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/laurus | jq -r 'select(.versions != null) | .versions[].num' 2>/dev/null | grep -Fx "$(LAURUS_VERSION)"),)
	(cd laurus && cargo package && cargo publish)
endif

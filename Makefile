IRIS_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="iris") | .version')

USER_AGENT ?= $(shell curl --version | head -n1 | awk '{print $1"/"$2}')
USER ?= $(shell whoami)
HOSTNAME ?= $(shell hostname)

.DEFAULT_GOAL := help

help: ## Show help
	@echo "Available targets:"
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-15s %s\n", $$1, $$2}'

clean: ## Clean the project
	cargo clean

format: ## Format the project
	cargo fmt

lint: ## Lint the project
	cargo clippy --workspace --all-targets -- -D warnings

test: ## Test the project
	cargo test --workspace

build: ## Build the project
	cargo build --release

bench: ## Benchmark the project
	cargo bench --bench bench

tag: ## Make a new tag for the current version
	git tag v$(IRIS_VERSION)
	git push origin v$(IRIS_VERSION)

publish: ## Publish the crate to crates.io
ifeq ($(shell curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/iris | jq -r 'select(.versions != null) | .versions[].num' 2>/dev/null | grep -Fx "$(IRIS_VERSION)"),)
	(cd iris && cargo package && cargo publish)
endif

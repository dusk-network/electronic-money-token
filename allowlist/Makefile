ALLOWLIST_WASM:="../target/wasm64-unknown-unknown/release/emt_allowlist.wasm"
TOKEN_WASM:="../target/wasm64-unknown-unknown/release/emt_token.wasm"
HOLDER_WASM:="../target/wasm64-unknown-unknown/release/emt_holder_contract.wasm"

all: ## Build the token-contract
	@cargo build --release

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

$(ALLOWLIST_WASM): ## Build allowlist-contract wasm file if it doesn't exist
	$(MAKE) -C ../ allowlist

$(HOLDER_WASM): ## Build token-holder-contract wasm file if it doesn't exist
	$(MAKE) -C ../ holder-contract

test: $(ALLOWLIST_WASM) $(HOLDER_WASM) ## Run the token-contract tests
	@cargo test --release -- --test-threads=1 # piecrust throws persistence error when using more threads

clippy: ## Run clippy
	@cargo +dusk clippy -Z build-std=core,alloc --release --target wasm64-unknown-unknown -- -D warnings

doc: ## Build the docs
	@cargo doc --release

.PHONY: all test clippy doc

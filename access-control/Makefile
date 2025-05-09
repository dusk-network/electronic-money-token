TOKEN_WASM:="../target/wasm64-unknown-unknown/release/emt_token.wasm"
ACCESS_CONTROL_WASM:="../target/wasm64-unknown-unknown/release/emt_access_control.wasm"

all: ## Build the token contract
	@cargo build --release

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

$(TOKEN_WASM): ## Build token contract wasm file if it doesn't exist
	$(MAKE) -C ../ token

$(ACCESS_CONTROL_WASM): ## Build access-control-contract wasm file if it doesn't exist
	$(MAKE) -C ../ access-control

test: $(TOKEN_WASM) $(ACCESS_CONTROL_WASM) ## Run the token contract tests
	@cargo test --release -- --test-threads=1 # piecrust throws persistence error when using more threads

clippy: ## Run clippy
	@cargo +dusk clippy -Z build-std=core,alloc --release --target wasm64-unknown-unknown -- -D warnings

doc: ## Build the docs
	@cargo doc --release

.PHONY: all test clippy doc

COMPILER_VERSION=v0.2.0

all: contract

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

test: contract holder-contract ## Run the tests
	@cargo test --release --manifest-path=tests/Cargo.toml

contract: setup-compiler ## Compile the token contract
	@RUSTFLAGS="-C link-args=-zstack-size=65536" \
	cargo +dusk build \
	  --release \
	  --manifest-path=contract/Cargo.toml \
	  --color=always \
	  -Z build-std=core,alloc \
	  --target wasm64-unknown-unknown
	@mkdir -p build
	@find target/wasm64-unknown-unknown/release -maxdepth 1 -name "*.wasm" \
	    | xargs -I % basename % \
	    | xargs -I % ./scripts/strip.sh \
		target/wasm64-unknown-unknown/release/% \
		build/%

holder-contract: setup-compiler ## Compile the holder-contract used for testing
	@RUSTFLAGS="-C link-args=-zstack-size=65536" \
	cargo +dusk build \
	  --release \
	  --manifest-path=tests/holder/Cargo.toml \
	  --color=always \
	  -Z build-std=core,alloc \
	  --target wasm64-unknown-unknown
	@mkdir -p build
	@find target/wasm64-unknown-unknown/release -maxdepth 1 -name "*.wasm" \
	    | xargs -I % basename % \
	    | xargs -I % ./scripts/strip.sh \
		target/wasm64-unknown-unknown/release/% \
		build/%


clippy: ## Run clippy
	# @cargo clippy --all-features --release -- -D warnings
	@cargo +dusk clippy -Z build-std=core,alloc --manifest-path=contract/Cargo.toml --release --target wasm64-unknown-unknown -- -D warnings

setup-compiler: ## Run the setup-compiler script
	@./scripts/setup-compiler.sh $(COMPILER_VERSION)

clean: ## Clean the build artifacts
	@cargo clean
	@rm -rf build

.PHONY: all test contract holder-contract clean setup-compiler

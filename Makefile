COMPILER_VERSION=v0.2.0

all: token

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

test: ## Run the tests
	$(MAKE) -C ./token/ $@
	$(MAKE) -C ./access-control/ $@

token: setup-compiler ## Compile the token-contract
	@RUSTFLAGS="-C link-args=-zstack-size=65536" \
	cargo +dusk build \
	  --release \
	  --manifest-path=token/Cargo.toml \
	  --color=always \
	  -Z build-std=core,alloc \
	  --target wasm64-unknown-unknown
	@mkdir -p build
	@find target/wasm64-unknown-unknown/release -maxdepth 1 -name "*.wasm" \
	    | xargs -I % basename % \
	    | xargs -I % ./scripts/strip.sh \
		target/wasm64-unknown-unknown/release/% \
		build/%

access-control: setup-compiler ## Compile the contract
	@RUSTFLAGS="-C link-args=-zstack-size=65536" \
	cargo +dusk build \
	  --release \
	  --manifest-path=access-control/Cargo.toml \
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
	$(MAKE) -C ./token/ $@
	$(MAKE) -C ./access-control/ $@

setup-compiler: ## Run the setup-compiler script
	@./scripts/setup-compiler.sh $(COMPILER_VERSION)

clean: ## Clean the build artifacts
	@cargo clean
	@rm -rf build

.PHONY: all test token access-control holder-contract clean setup-compiler

CARGO_BUILD_ARGS ?=

# build
.PHONY: build
build:
	cargo build --release $(CARGO_BUILD_ARGS)

.PHONY: build-quick
build-quick:
	cargo build --profile quick-release $(CARGO_BUILD_ARGS)

# lint
.PHONY: lint
lint:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings

.PHONY: fix-lint
fix-lint:
	cargo clippy --fix --allow-staged --allow-dirty --allow-no-vcs
	cargo fmt

.PHONY: format
format:
	cargo fmt

# test
.PHONY: test
test:
	cargo test --all-targets

.PHONY: test-all
test-all:
	cargo test --all-targets --workspace

# docs
.PHONY: doc
doc:
	cargo doc --no-deps --workspace

# clean
.PHONY: clean
clean:
	cargo clean

# Publish dry run — validates crates.io readiness in dependency order
PUBLISHABLE_CRATES = converge-traits converge-core converge-provider converge-experience converge-knowledge ortools-sys converge-optimization converge-domain converge-analytics converge-axiom
.PHONY: publish-dry-run
publish-dry-run:
	@for crate in $(PUBLISHABLE_CRATES); do \
		echo "--- dry-run: $$crate ---"; \
		cargo publish --dry-run -p $$crate || exit 1; \
	done

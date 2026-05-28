REPO ?= spalencsar/deskify

.PHONY: help fmt check clippy test build release clean labels doctor

help:
	@echo "Deskify development targets:"
	@echo ""
	@echo "  make fmt       - Format all code with cargo fmt"
	@echo "  make check     - Run cargo check"
	@echo "  make clippy    - Run clippy with -D warnings (as required by AGENTS.md)"
	@echo "  make test      - Run all unit tests"
	@echo "  make build     - Debug build"
	@echo "  make release   - Optimized release build"
	@echo "  make clean     - Remove build artifacts"
	@echo "  make doctor    - Run deskify doctor (prefers debug binary)"
	@echo "  make labels    - Sync GitHub issue labels (requires gh + jq)"
	@echo ""

fmt:
	cargo fmt

check:
	cargo check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test

build:
	cargo build

release:
	cargo build --release

clean:
	cargo clean

labels:
	./scripts/setup-github-labels.sh $(REPO)

doctor:
	@if [ -f target/debug/deskify ]; then \
		echo "Using debug binary..."; \
		target/debug/deskify doctor; \
	elif [ -f target/release/deskify ]; then \
		echo "Using release binary..."; \
		target/release/deskify doctor; \
	else \
		echo "No deskify binary found. Run 'make build' or 'make release' first."; \
		exit 1; \
	fi

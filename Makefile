.PHONY: help setup test check fmt clippy clean doc coverage bench precommit audit

# Default target
help:
	@echo "Available targets:"
	@echo "  setup      - Set up development environment including pre-commit hooks"
	@echo "  test       - Run all tests"
	@echo "  check      - Run fmt check, clippy, and tests"
	@echo "  fmt        - Format code with rustfmt"
	@echo "  clippy     - Run clippy linter"
	@echo "  clean      - Clean build artifacts"
	@echo "  doc        - Build documentation"
	@echo "  coverage   - Generate test coverage report"
	@echo "  bench      - Run benchmarks"
	@echo "  precommit  - Run pre-commit hooks on all files"
	@echo "  audit      - Run security audit"

# Set up development environment
setup:
	@echo "Setting up development environment..."
	./scripts/setup-pre-commit.sh

# Run all tests
test:
	cargo test --all-features

# Run all checks (fmt, clippy, test)
check:
	cargo fmt --all -- --check
	cargo clippy --all-features --all-targets -- -D warnings
	cargo test --all-features

# Format code
fmt:
	cargo fmt --all

# Run clippy
clippy:
	cargo clippy --all-features --all-targets -- -D warnings

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/
	rm -f coverage.info lcov.info

# Build documentation
doc:
	cargo doc --all-features --no-deps --open

# Generate coverage report
coverage:
	@command -v cargo-llvm-cov >/dev/null 2>&1 || cargo install cargo-llvm-cov
	cargo llvm-cov --all-features --lcov --output-path coverage.info
	cargo llvm-cov --all-features --summary-only

# Run benchmarks
bench:
	cargo bench --features tools

# Run pre-commit hooks on all files
precommit:
	pre-commit run --all-files

# Run security audit
audit:
	@command -v cargo-audit >/dev/null 2>&1 || cargo install cargo-audit
	cargo audit
# Cogni Development Tasks

<!-- A task runner for the Cogni project -->

## test

> Run all tests

```bash
cargo test --all-features
```

## test-verbose

> Run all tests with output

```bash
cargo test --all-features -- --nocapture
```

## clippy

> Run clippy linter

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## fmt

> Format code

```bash
cargo fmt --all
```

## fmt-check

> Check code formatting

```bash
cargo fmt --all -- --check
```

## build

> Build all packages

```bash
cargo build --all-features
```

## build-release

> Build in release mode

```bash
cargo build --all-features --release
```

## coverage

> Run test coverage

```bash
cargo llvm-cov --all-features
```

## coverage-html

> Generate HTML coverage report

```bash
cargo llvm-cov --all-features --html
```

## coverage-json

> Generate JSON coverage report

```bash
cargo llvm-cov --all-features --json
```

## docs

> Build documentation

```bash
cargo doc --all-features --no-deps --open
```

## clean

> Clean build artifacts

```bash
cargo clean
```

## check

> Run all checks (fmt, clippy, test)

```bash
set -e
echo "Checking formatting..."
cargo fmt --all -- --check
echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings
echo "Running tests..."
cargo test --all-features
echo "All checks passed!"
```

## example

> Run an example

**OPTIONS**
* name
  * flags: -n --name
  * desc: Name of the example to run
  * required

```bash
cargo run --example $name --all-features
```

## bench

> Run benchmarks

```bash
cargo bench --all-features
```

## watch

> Watch for changes and run tests

```bash
cargo watch -x "test --all-features"
```

## dev

> Start development mode (watch and run tests)

```bash
mprocs "bacon clippy-all" "bacon test"
```

## todo

> Show project TODOs

```bash
grep -r "TODO\|FIXME\|HACK" --include="*.rs" --include="*.toml" --include="*.md" . | grep -v target | grep -v .git
```

## deps

> Check for outdated dependencies

```bash
cargo outdated
```

## audit

> Security audit dependencies

```bash
cargo audit
```

## release

> Prepare for release (run all checks)

```bash
set -e
echo "Running release checks..."
mask check
echo "Checking for uncommitted changes..."
git diff --exit-code
echo "Building in release mode..."
cargo build --all-features --release
echo "Generating documentation..."
cargo doc --all-features --no-deps
echo "Release checks complete!"
```

## mcp-http

> Run MCP HTTP example server

```bash
cd examples/mcp && python mock_mcp_http_server.py
```

## mcp-stdio

> Run MCP stdio example server

```bash
cd examples/mcp && python mock_mcp_stdio_server.py
```

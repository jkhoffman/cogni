# Contributing to Cogni

Thank you for your interest in contributing to Cogni! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct: be respectful, inclusive, and constructive in all interactions.

## How to Contribute

### Reporting Issues

1. Check if the issue already exists
2. Create a new issue with a clear title and description
3. Include:
   - Cogni version
   - Rust version (`rustc --version`)
   - Operating system
   - Minimal code example reproducing the issue
   - Error messages or unexpected behavior

### Suggesting Features

1. Open a discussion first for major features
2. Explain the use case and benefits
3. Consider implementation complexity
4. Be open to feedback and alternatives

### Submitting Code

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass (`cargo test --all-features`)
6. Run clippy (`cargo clippy --all-features`)
7. Format code (`cargo fmt`)
8. Commit with clear messages (pre-commit hooks will run automatically)
9. Push and create a Pull Request

## Development Setup

### Prerequisites

- Rust 1.75 or later
- API keys for testing (optional):
  - `OPENAI_API_KEY`
  - `ANTHROPIC_API_KEY`
  - Ollama running locally

### Setting up Pre-commit Hooks

We use pre-commit hooks to ensure code quality. To set them up:

```bash
# Using mask (recommended)
mask setup

# Or run the setup script directly
./scripts/setup-pre-commit.sh

# Or manually:
pip install pre-commit
cargo install taplo-cli cargo-audit
pre-commit install
```

Pre-commit will automatically:
- Format Rust code with `cargo fmt`
- Run `cargo clippy` with strict warnings
- Check for trailing whitespace and file issues
- Verify TOML file formatting
- Check for typos in code and documentation
- Detect accidentally committed secrets

To run hooks manually:
```bash
pre-commit run --all-files
```

To skip hooks temporarily (not recommended):
```bash
git commit --no-verify
```

### Building

```bash
# Clone the repository
git clone https://github.com/yourusername/cogni.git
cd cogni

# Build all packages
cargo build --all-features

# Run tests
cargo test --all-features

# Run specific test
cargo test test_name

# Run benchmarks
cargo bench --features tools
```

### Project Structure

```
cogni/
├── cogni-core/       # Core types and traits
├── cogni-providers/  # Provider implementations
├── cogni-tools/      # Tool execution framework
├── cogni-middleware/ # Middleware system
├── cogni-client/     # High-level client API
├── cogni/           # Main crate re-exporting everything
├── examples/        # Example code
├── benches/         # Benchmarks
└── tests/           # Integration tests
```

## Coding Guidelines

### Style

- Follow Rust standard style guidelines
- Use `cargo fmt` before committing
- Keep lines under 100 characters when practical
- Use meaningful variable and function names

### Documentation

- Add doc comments to all public items
- Include examples in doc comments
- Update README.md if adding features
- Add entries to CHANGELOG.md

### Testing

- Write unit tests for new functionality
- Add integration tests for cross-module features
- Test error cases, not just happy paths
- Mock external APIs in tests

### Performance

- Avoid unnecessary allocations
- Use `Cow<str>` instead of cloning strings
- Pre-allocate collections when size is known
- Run benchmarks for performance-critical code

## Pull Request Process

1. **Title**: Use a clear, descriptive title
2. **Description**: Explain what and why
3. **Testing**: Describe how you tested
4. **Breaking Changes**: Clearly mark any
5. **Documentation**: Update as needed
6. **Review**: Address feedback promptly

### PR Title Format

- `feat: Add new feature`
- `fix: Fix specific bug`
- `docs: Update documentation`
- `perf: Improve performance`
- `refactor: Refactor code`
- `test: Add tests`
- `chore: Update dependencies`

## Architecture Decisions

### Middleware

We use a Tower-inspired Service/Layer pattern for middleware because:
- Composable and type-safe
- Works with Rust's async trait limitations
- Allows zero-cost abstractions

### Error Handling

- Use `thiserror` for error definitions
- Make errors non-exhaustive
- Include context in error messages
- Provide recovery hints when possible

### Async Design

- All provider methods are async
- Use `BoxFuture` for type erasure when needed
- Avoid blocking operations
- Support cancellation via dropping futures

## Adding a New Provider

1. Create module in `cogni-providers/src/`
2. Implement `Provider` trait
3. Add configuration struct
4. Support streaming if possible
5. Add integration tests
6. Document provider-specific features
7. Add example in `examples/`

Example structure:
```rust
// cogni-providers/src/newprovider/mod.rs
mod config;
mod converter;
mod stream;

pub use config::NewProviderConfig;

pub struct NewProvider { ... }

impl Provider for NewProvider {
    type Stream = NewProviderStream;
    
    async fn request(&self, request: Request) -> Result<Response, Error> {
        // Implementation
    }
    
    async fn stream(&self, request: Request) -> Result<Self::Stream, Error> {
        // Implementation
    }
}
```

## Adding Middleware

1. Create module in `cogni-middleware/src/`
2. Implement `Service` and `Layer` traits
3. Handle both request and streaming
4. Add configuration options
5. Write comprehensive tests
6. Document behavior and options

## Release Process

See [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md) for detailed release procedures.

## Getting Help

- Open an issue for bugs
- Start a discussion for questions
- Check existing issues and discussions first
- Join our community chat (if applicable)

## Recognition

Contributors will be recognized in:
- GitHub contributors page
- CHANGELOG.md for significant contributions
- Special thanks in release notes

Thank you for contributing to Cogni!
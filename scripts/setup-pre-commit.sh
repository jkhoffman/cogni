#!/bin/bash
set -e

echo "Setting up pre-commit hooks for Cogni..."

# Check if pre-commit is installed
if ! command -v pre-commit &> /dev/null; then
    echo "Installing pre-commit..."
    if command -v pip &> /dev/null; then
        pip install pre-commit
    elif command -v brew &> /dev/null; then
        brew install pre-commit
    else
        echo "Error: Neither pip nor brew found. Please install pre-commit manually."
        echo "Visit: https://pre-commit.com/#installation"
        exit 1
    fi
fi

# Check if taplo is installed (for TOML formatting)
if ! command -v taplo &> /dev/null; then
    echo "Installing taplo for TOML formatting..."
    if command -v cargo &> /dev/null; then
        cargo install taplo-cli
    else
        echo "Warning: taplo not found and cargo not available. Skipping TOML formatter."
        echo "To install: cargo install taplo-cli"
    fi
fi

# Check if cargo-audit is installed
if ! command -v cargo-audit &> /dev/null; then
    echo "Installing cargo-audit for security auditing..."
    if command -v cargo &> /dev/null; then
        cargo install cargo-audit
    else
        echo "Warning: cargo-audit not found and cargo not available."
        echo "To install: cargo install cargo-audit"
    fi
fi

# Install pre-commit hooks
echo "Installing pre-commit hooks..."
pre-commit install

# Run hooks on all files to ensure everything is clean
echo "Running pre-commit on all files..."
pre-commit run --all-files || true

echo "✅ Pre-commit setup complete!"
echo ""
echo "Pre-commit will now run automatically on git commit."
echo "To run manually: pre-commit run --all-files"
echo "To run specific hooks: pre-commit run <hook-id>"
echo "To skip hooks temporarily: git commit --no-verify"
echo ""
echo "Optional: Run 'pre-commit run --all-files --hook-stage manual' to run manual checks (tests, audit)"

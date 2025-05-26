# Pre-commit Setup for Cogni

This document describes the pre-commit hooks configured for the Cogni project.

## Quick Start

```bash
# Run the setup script
./scripts/setup-pre-commit.sh

# Or use mask
mask setup
```

## What Gets Checked

### On Every Commit

1. **Rust Formatting** (`cargo fmt`)
   - Ensures consistent code style
   - Automatically fixable with `cargo fmt`

2. **Rust Linting** (`cargo clippy`)
   - Catches common mistakes and anti-patterns
   - Configured with `-D warnings` (warnings as errors)

3. **File Hygiene**
   - No trailing whitespace
   - Files end with newline
   - No mixed line endings (enforces LF)
   - No case conflicts in filenames
   - No merge conflict markers

4. **TOML Formatting** (`taplo`)
   - Formats Cargo.toml files consistently
   - Validates TOML syntax

5. **Spell Check** (`typos`)
   - Catches common typos in code and docs
   - Configurable via `.typos.toml`

6. **Security Checks**
   - No accidentally committed private keys
   - File size limits (1MB default)

### Manual/Staged Checks

These checks can be run manually but don't run on every commit:

```bash
# Run security audit
pre-commit run cargo-audit --all-files --hook-stage manual

# Run tests (if enabled)
pre-commit run cargo-test --all-files --hook-stage manual
```

## Configuration Files

- `.pre-commit-config.yaml` - Main configuration
- `.typos.toml` - Spell checker configuration
- `.githooks/pre-commit` - Alternative git hook (optional)

## Bypassing Checks

In rare cases where you need to bypass checks:

```bash
# Skip all hooks for one commit
git commit --no-verify -m "Emergency fix"

# Skip specific hooks
SKIP=cargo-clippy git commit -m "WIP commit"
```

## Troubleshooting

### Hook Installation Issues

```bash
# Reinstall hooks
pre-commit uninstall
pre-commit install

# Update hooks to latest versions
pre-commit autoupdate
```

### Performance Issues

If hooks are too slow:

1. Use `--no-verify` for WIP commits
2. Run checks manually before final commit
3. Consider disabling slow hooks locally

### Tool Installation

Required tools:
- `pre-commit` (Python package)
- `taplo` (Cargo package)
- `cargo-audit` (Cargo package)

All are installed by the setup script.

## CI Integration

The same checks run in CI via:
- `.github/workflows/pre-commit.yml` - Pre-commit specific
- `.github/workflows/ci.yml` - Full CI pipeline

This ensures that even if developers bypass local hooks, CI will catch issues.

## Customization

To add custom project-specific checks:

1. Edit `.pre-commit-config.yaml`
2. Add new hooks under the `local` repository
3. Test with `pre-commit run --all-files`

Example custom hook:

```yaml
- repo: local
  hooks:
    - id: no-todo
      name: Check for TODO comments
      entry: 'TODO|FIXME|XXX'
      language: pygrep
      types: [rust]
```

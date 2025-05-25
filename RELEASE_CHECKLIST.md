# Release Checklist for v0.1.0

## Pre-Release

### Code Quality
- [x] All tests passing
- [x] No clippy warnings
- [x] Documentation complete
- [x] Examples working
- [x] Benchmarks running

### Documentation
- [x] README.md updated
- [x] CHANGELOG.md created
- [x] USAGE_GUIDE.md complete
- [x] API documentation reviewed
- [x] Migration guide included

### Version Bumps
- [ ] Update version in root Cargo.toml
- [ ] Update version in all workspace Cargo.toml files
- [ ] Update version in README.md examples
- [ ] Update version in documentation

### Final Checks
- [ ] Run full test suite: `cargo test --all-features`
- [ ] Run benchmarks: `cargo bench --features tools`
- [ ] Check documentation: `cargo doc --all-features --open`
- [ ] Verify examples: `cargo run --example hello_world`
- [ ] License headers in all source files
- [ ] No sensitive information in code

## Release Process

### 1. Create Release Branch
```bash
git checkout -b release-0.1.0
git add .
git commit -m "Prepare for v0.1.0 release"
```

### 2. Tag Release
```bash
git tag -a v0.1.0 -m "Release version 0.1.0"
git push origin release-0.1.0
git push origin v0.1.0
```

### 3. Publish to crates.io
```bash
# Publish in dependency order
cargo publish -p cogni-core
cargo publish -p cogni-providers
cargo publish -p cogni-tools
cargo publish -p cogni-middleware
cargo publish -p cogni-client
cargo publish -p cogni
```

### 4. Create GitHub Release
- Go to GitHub releases page
- Create release from tag v0.1.0
- Copy CHANGELOG.md content for v0.1.0
- Attach any binary artifacts if applicable

### 5. Post-Release
- [ ] Announce on relevant forums/social media
- [ ] Update project website/blog
- [ ] Create issues for v0.2.0 features
- [ ] Merge release branch to main
- [ ] Update version to 0.2.0-dev

## Rollback Plan

If issues are discovered post-release:

1. Yank affected crates: `cargo yank --vers 0.1.0 -p cogni`
2. Fix issues in hotfix branch
3. Release as v0.1.1 following same process
4. Update changelog with fixes

## Communication

### Release Announcement Template

```
🎉 Cogni v0.1.0 Released! 

Cogni is a unified Rust library for interacting with multiple LLM providers.

✨ Highlights:
- Multi-provider support (OpenAI, Anthropic, Ollama)
- Streaming responses
- Tool/function calling
- Composable middleware
- High-level client API

📚 Get started: https://github.com/yourusername/cogni
📖 Documentation: https://docs.rs/cogni

#rustlang #ai #llm
```

## Notes

- Ensure all environment variables are documented
- Test on different platforms (Linux, macOS, Windows)
- Consider creating Docker images for examples
- Plan for backwards compatibility in future releases
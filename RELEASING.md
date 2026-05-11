# Release Process

## Overview

LocalWebs uses GitHub Actions to automatically build and publish releases for multiple platforms when you push a version tag.

## Supported Platforms

- **Linux**: x86_64, ARM64 (musl static binaries)
- **macOS**: x86_64 (Intel), ARM64 (Apple Silicon)
- **Windows**: x86_64

## Release Steps

### 1. Prepare Release

```bash
# Make sure main is up to date
git checkout master
git pull origin master

# Run tests
cargo test

# Update version in Cargo.toml
vim Cargo.toml  # Change version = "0.1.0" to "0.2.0"

# Update CHANGELOG.md (optional but recommended)
vim CHANGELOG.md

# Commit version bump
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to 0.2.0"
git push origin master
```

### 2. Create and Push Tag

```bash
# Create annotated tag
git tag -a v0.2.0 -m "Release v0.2.0"

# Push tag to trigger release workflow
git push origin v0.2.0
```

### 3. GitHub Actions Workflow

The `.github/workflows/release.yml` will automatically:
1. Build for all platforms
2. Create `.tar.gz` archives (Unix) and `.zip` (Windows)
3. Create a GitHub Release
4. Attach all binaries to the release
5. Generate release notes from commits

### 4. Verify Release

1. Go to https://github.com/nvquanghuy/localwebs/releases
2. Check that the new release is published
3. Verify all platform binaries are attached
4. Test installation script:

```bash
# Test on Linux/macOS
curl -fsSL https://raw.githubusercontent.com/nvquanghuy/localwebs/master/install.sh | bash

# Test manual download
wget https://github.com/nvquanghuy/localwebs/releases/latest/download/localwebs-x86_64-unknown-linux-musl.tar.gz
tar xzf localwebs-x86_64-unknown-linux-musl.tar.gz
./localwebs --version
```

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** (1.0.0): Breaking changes
- **MINOR** (0.1.0): New features, backward compatible
- **PATCH** (0.0.1): Bug fixes

Examples:
- `v0.1.0` - Initial release
- `v0.2.0` - Add new sort feature
- `v0.2.1` - Fix sorting bug
- `v1.0.0` - Stable API, ready for production

## Manual Build (for testing)

### Cross-compile for all platforms

```bash
# Install cross (one-time setup)
cargo install cross

# Build all platforms
make cross

# Package binaries
make package

# Result: dist/ folder with all platform archives
ls -lh dist/
```

### Test specific platform

```bash
# Linux x86_64
cross build --release --target x86_64-unknown-linux-musl

# macOS ARM64
cross build --release --target aarch64-apple-darwin

# Windows
cross build --release --target x86_64-pc-windows-msvc
```

## Troubleshooting

### Build fails for ARM64 Linux

Install cross-compilation tools:
```bash
sudo apt-get install -y musl-tools gcc-aarch64-linux-gnu
rustup target add aarch64-unknown-linux-musl
```

### macOS builds fail on Linux

Use `cross` instead of `cargo`:
```bash
cargo install cross
cross build --release --target x86_64-apple-darwin
```

### Release workflow fails

1. Check GitHub Actions logs
2. Verify secrets are set (usually not needed for public repos)
3. Ensure tag follows `v*` pattern

## Post-Release

### Announce

- [ ] Update repository README if needed
- [ ] Post on social media / forums
- [ ] Update documentation site
- [ ] Notify users via Discord / Slack

### Verify Installation

Test on clean machines:
```bash
# Fresh Ubuntu
docker run -it ubuntu:latest bash
curl -fsSL https://raw.githubusercontent.com/nvquanghuy/localwebs/master/install.sh | bash
localwebs --version

# Fresh Fedora
docker run -it fedora:latest bash
curl -fsSL https://raw.githubusercontent.com/nvquanghuy/localwebs/master/install.sh | bash
localwebs --version
```

## Quick Reference

```bash
# Full release process (one-liner after version bump)
git tag -a v0.2.0 -m "Release v0.2.0" && git push origin v0.2.0

# Delete tag (if you made a mistake)
git tag -d v0.2.0
git push origin :refs/tags/v0.2.0

# Create pre-release
git tag -a v0.2.0-beta.1 -m "Beta release"
git push origin v0.2.0-beta.1
```

## Binary Size Optimization

Current sizes (typical):
- Linux (musl): ~5-8MB
- macOS: ~5-8MB  
- Windows: ~6-9MB

To reduce further, add to `Cargo.toml`:
```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization
strip = true        # Strip symbols
```

## Future: Publish to Package Managers

### Cargo (crates.io)

```bash
cargo login <token>
cargo publish
```

### Homebrew

Create a tap:
```bash
# Create nvquanghuy/homebrew-tap
# Add Formula/localwebs.rb
```

### AUR (Arch Linux)

Create PKGBUILD and publish to AUR.

### Snap / Flatpak

Create snap.yaml or flatpak manifest.

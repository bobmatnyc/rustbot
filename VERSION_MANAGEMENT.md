# Version Management Guide

This document explains how to manage semantic versioning and build numbers for Rustbot.

## Version Format

Rustbot uses semantic versioning with build tracking:

```
v{MAJOR}.{MINOR}.{PATCH}-{BUILD}
```

Example: `v0.0.1-0001`

## Current Version

- **Version**: 0.2.0
- **Build**: 0001

## Updating Versions

### 1. Update Build Number (Most Common)

For routine builds and minor changes:

1. Edit `src/version.rs`
2. Increment the `BUILD` constant: `"0001"` → `"0002"`
3. Build and test
4. Commit with message: `build: Bump build to 0002`

### 2. Update Patch Version

For bug fixes and small improvements:

1. Edit `src/version.rs`
2. Increment `VERSION`: `"0.0.1"` → `"0.0.2"`
3. Reset `BUILD` to `"0001"`
4. Update `Cargo.toml` version to match
5. Commit with message: `chore: Release v0.0.2`

### 3. Update Minor Version

For new features (backward compatible):

1. Edit `src/version.rs`
2. Increment minor: `"0.0.1"` → `"0.1.0"`
3. Reset `BUILD` to `"0001"`
4. Update `Cargo.toml` version to match
5. Commit with message: `feat: Release v0.1.0`

### 4. Update Major Version

For breaking changes:

1. Edit `src/version.rs`
2. Increment major: `"0.1.0"` → `"1.0.0"`
3. Reset `BUILD` to `"0001"`
4. Update `Cargo.toml` version to match
5. Commit with message: `feat!: Release v1.0.0`

## Semantic Versioning Rules

- **MAJOR**: Breaking changes (incompatible API changes)
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)
- **BUILD**: Sequential build number (4 digits)

## Example Workflow

### Daily Development
```bash
# Make changes
vim src/main.rs

# Increment build
vim src/version.rs  # Change BUILD from "0001" to "0002"

# Test
cargo build && cargo test

# Commit
git add -A
git commit -m "build: Bump to v0.0.1-0002"
git push
```

### Bug Fix Release
```bash
# Fix bug
vim src/main.rs

# Update version
vim src/version.rs  # Change VERSION to "0.0.2", BUILD to "0001"
vim Cargo.toml     # Change version to "0.0.2"

# Test
cargo build && cargo test

# Commit
git add -A
git commit -m "fix: Resolve token calculation issue - v0.0.2"
git push
```

### Feature Release
```bash
# Add feature
vim src/main.rs

# Update version
vim src/version.rs  # Change VERSION to "0.1.0", BUILD to "0001"
vim Cargo.toml     # Change version to "0.1.0"

# Test
cargo build && cargo test

# Commit
git add -A
git commit -m "feat: Add streaming response support - v0.1.0"
git push
```

## Version Display

The version is displayed in the application header:

```
Rustbot - AI Assistant    v0.0.1-0001
```

## Files to Update

When changing versions:

1. `src/version.rs` - Update VERSION and/or BUILD constants
2. `Cargo.toml` - Update package version (major.minor.patch only)
3. This file - Update "Current Version" section

## Automated Future Enhancement

Consider automating this with:
- Build scripts that auto-increment BUILD
- CI/CD pipelines that tag releases
- Pre-commit hooks that validate version consistency

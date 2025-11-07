# CI/CD Pipeline Documentation

This document describes the automated Continuous Integration and Continuous Deployment (CI/CD) pipelines configured for this project.

## Overview

The project uses GitHub Actions for automated testing, quality checks, and releases. Three main workflows are configured:

1. **CI Workflow** (`ci.yml`) - Runs on every push and pull request
2. **Release Workflow** (`release.yml`) - Triggered by version tags
3. **Dependabot Auto-merge** (`dependabot.yml`) - Automatically merges dependency updates

## CI Workflow (`.github/workflows/ci.yml`)

### Triggers
- Push to `main` or `develop` branches
- Pull requests to `main` or `develop` branches

### Jobs

#### 1. Code Formatting (`fmt`)
- **Runs on**: Ubuntu Latest
- **Purpose**: Ensures code follows Rust formatting standards
- **Command**: `cargo fmt --all -- --check`
- **Fails if**: Code is not properly formatted

#### 2. Linting (`clippy`)
- **Runs on**: Ubuntu Latest
- **Purpose**: Catches common mistakes and enforces best practices
- **Command**: `cargo clippy --all-targets --all-features -- -D warnings`
- **Features**: Caching for faster builds
- **Fails if**: Clippy warnings are found

#### 3. Test Suite (`test`)
- **Runs on**: Ubuntu, Windows, macOS
- **Rust versions**: Stable and Beta
- **Matrix strategy**: 
  - Full testing on all platforms with stable Rust
  - Beta testing only on Ubuntu (to save CI time)
- **Steps**:
  1. Build all targets
  2. Run all tests (unit + integration)
  3. Build examples
  4. Run documentation tests
- **Features**: Build caching for faster execution

#### 4. Documentation (`docs`)
- **Runs on**: Ubuntu Latest
- **Purpose**: Verifies documentation builds without errors
- **Command**: `cargo doc --no-deps --all-features`
- **Env**: `RUSTDOCFLAGS=-D warnings` (treats doc warnings as errors)

#### 5. Security Audit (`security_audit`)
- **Runs on**: Ubuntu Latest
- **Purpose**: Checks for known security vulnerabilities in dependencies
- **Tool**: `cargo-audit`

#### 6. Outdated Dependencies (`outdated`)
- **Runs on**: Ubuntu Latest
- **Purpose**: Checks for outdated dependencies
- **Tool**: `cargo-outdated`
- **Note**: Allowed to fail (doesn't block PR)

#### 7. CI Success (`ci-success`)
- **Purpose**: Overall status check for branch protection
- **Depends on**: All other jobs
- **Fails if**: Any required job fails

### Caching Strategy

The CI workflow uses GitHub Actions cache for:
- **Cargo registry** (`~/.cargo/registry`)
- **Cargo git** (`~/.cargo/git`)
- **Build artifacts** (`target/`)

Cache keys include:
- Operating system
- Rust toolchain version
- `Cargo.lock` hash

This significantly speeds up build times by reusing previously downloaded dependencies and compiled artifacts.

## Release Workflow (`.github/workflows/release.yml`)

### Triggers
- Push of tags matching pattern: `v*.*.*` (e.g., `v0.1.0`, `v1.2.3`)

### Jobs

#### 1. Create Release (`create-release`)
- Creates a GitHub release for the tagged version
- Generates upload URL for binary artifacts

#### 2. Publish to crates.io (`publish`)
- **Runs on**: Ubuntu Latest
- **Purpose**: Publishes the crate to crates.io
- **Requirements**: `CRATES_IO_TOKEN` secret must be configured
- **Command**: `cargo publish`

#### 3. Build Release Binaries (`build-release`)
- **Matrix**:
  - **Linux** (x86_64-unknown-linux-gnu)
  - **Windows** (x86_64-pc-windows-msvc)
  - **macOS** (x86_64-apple-darwin)
- **Artifacts**: All example binaries
- **Packaging**:
  - Linux/macOS: `.tar.gz`
  - Windows: `.zip`
- **Upload**: Binaries attached to GitHub release

### Creating a Release

To create a new release:

```bash
# Tag the commit
git tag v0.1.0

# Push the tag
git push origin v0.1.0
```

The CI will automatically:
1. Build release binaries for all platforms
2. Create a GitHub release
3. Upload binaries to the release
4. Publish to crates.io (if configured)

## Dependabot Configuration

### Auto-Updates (`.github/dependabot.yml`)

Two package ecosystems are monitored:

#### 1. Cargo Dependencies
- **Schedule**: Weekly (Mondays)
- **PR Limit**: 10 open PRs max
- **Labels**: `dependencies`, `rust`
- **Commit prefix**: `chore`

#### 2. GitHub Actions
- **Schedule**: Weekly (Mondays)
- **PR Limit**: 5 open PRs max
- **Labels**: `dependencies`, `ci`
- **Commit prefix**: `ci`

### Auto-Merge (`.github/workflows/dependabot.yml`)

- **Trigger**: Dependabot PRs
- **Auto-merges**: Minor and patch version updates only
- **Requires**: All CI checks to pass
- **Merge strategy**: Squash merge

## Branch Protection Recommendations

For production use, configure branch protection on `main`:

1. **Required status checks**:
   - `Rustfmt`
   - `Clippy`
   - `Test Suite`
   - `Documentation`
   - `Security Audit`

2. **Required reviews**: 1 approval minimum

3. **Require branches to be up to date**: Yes

4. **Include administrators**: No (allow emergency fixes)

## Secrets Configuration

Required secrets for full CI/CD functionality:

### For crates.io Publishing
- `CRATES_IO_TOKEN`: API token from crates.io
  - Get from: https://crates.io/me
  - Settings → API Tokens → New Token
  - Scope: `publish-update`

### Setting Secrets

Go to repository Settings → Secrets and variables → Actions → New repository secret

## Local Testing

Before pushing, run the same checks locally:

```bash
# Format check
cargo fmt --all -- --check

# Linting
cargo clippy --all-targets --all-features

# Tests
cargo test --all-targets

# Documentation
cargo doc --no-deps --all-features

# Security audit
cargo install cargo-audit
cargo audit

# Outdated dependencies
cargo install cargo-outdated
cargo outdated
```

## Troubleshooting

### CI Fails on Formatting
**Solution**: Run `cargo fmt --all` and commit the changes

### CI Fails on Clippy
**Solution**: Fix the warnings reported by `cargo clippy` or add `#[allow(...)]` attributes if intentional

### Tests Fail on Specific Platform
**Solution**: 
1. Check platform-specific code (e.g., PEAK CAN on Windows)
2. Add platform-specific conditional compilation if needed
3. Consider using feature flags for platform-specific functionality

### Cache Issues
**Solution**: 
1. Clear cache from Actions tab → Caches
2. Or update cache key in workflow file

### Release Fails
**Common issues**:
1. Version in `Cargo.toml` doesn't match tag
2. `CRATES_IO_TOKEN` not configured
3. Crate name already taken on crates.io

## Performance

Typical CI run times (with cache):
- **Formatting**: ~30 seconds
- **Clippy**: ~2-3 minutes
- **Tests** (per platform): ~3-5 minutes
- **Documentation**: ~2 minutes
- **Security Audit**: ~1 minute

Total CI time: ~10-15 minutes for full matrix

## Future Enhancements

Potential improvements to consider:

1. **Code Coverage**:
   - Add `tarpaulin` or `grcov` for coverage reporting
   - Upload to Codecov or Coveralls

2. **Benchmarks**:
   - Add `cargo bench` step
   - Track performance over time

3. **Additional Platforms**:
   - ARM architectures
   - 32-bit builds

4. **Nightly Builds**:
   - Test against Rust nightly
   - Early warning for upcoming changes

5. **Integration Tests**:
   - Test with actual CAN hardware (if available in CI)
   - Mock hardware testing

6. **Documentation Deployment**:
   - Auto-deploy docs to GitHub Pages
   - Version-specific documentation

## Maintenance

- **Review Dependabot PRs** weekly
- **Update CI workflow** when:
  - Adding new lint rules
  - Changing supported platforms
  - Adding new test categories
- **Rotate crates.io tokens** annually

# Contributing to libCANopen Client

Thank you for your interest in contributing to libCANopen Client! This document provides guidelines and information for contributors.

## Code of Conduct

Be respectful, professional, and constructive in all interactions. We're here to build great software together.

## How to Contribute

### Reporting Bugs

Before creating a bug report, please check existing issues. When creating a bug report, include:

- **Clear title** describing the issue
- **Detailed description** of the problem
- **Steps to reproduce** the issue
- **Expected vs actual behavior**
- **Environment details** (OS, Rust version, hardware)
- **Code samples** if applicable
- **Log output** with `RUST_LOG=debug` enabled

Example:
```markdown
## Bug: SDO timeout when reading from node 5

**Environment:**
- OS: Windows 11
- Rust: 1.70
- Hardware: PEAK PCAN-USB

**Steps to reproduce:**
1. Connect to CAN bus at 1 Mbps
2. Attempt to read 0x1000:0 from node 5
3. Timeout occurs after 1 second

**Expected:** Value should be returned
**Actual:** CANopenError::Timeout

**Logs:**
[DEBUG] Sending SDO read request...
[ERROR] SDO timeout after 1000ms
```

### Suggesting Features

Feature requests are welcome! Please include:

- **Use case** - Why is this feature needed?
- **Proposed solution** - How should it work?
- **Alternatives considered** - What other approaches did you think about?
- **Additional context** - Any relevant specifications or examples

### Pull Requests

1. **Fork** the repository
2. **Create a branch** from `main`
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes** following the code style guidelines
4. **Add tests** for new functionality
5. **Update documentation** as needed
6. **Ensure all tests pass**
   ```bash
   cargo test
   cargo clippy
   cargo fmt --check
   ```
7. **Commit** with clear messages
8. **Push** to your fork
9. **Submit a pull request**

#### Pull Request Guidelines

- **One feature per PR** - Keep changes focused
- **Write clear commit messages**
  ```
  feat: Add TIME protocol support
  
  - Implement TIME message handling
  - Add timestamp synchronization
  - Add example and tests
  ```
- **Reference related issues** - Use "Fixes #123" or "Relates to #456"
- **Include tests** - New code should have test coverage
- **Update CHANGELOG.md** - Add entry in [Unreleased] section

## Development Setup

### Prerequisites

- **Rust 1.70+** (install via [rustup](https://rustup.rs/))
- **PEAK CAN drivers** (for hardware testing)
- **Git**

### Setup

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/libcanopen-client-rs.git
cd libcanopen-client-rs

# Build the project
cargo build

# Run tests
cargo test

# Run clippy (linter)
cargo clippy

# Format code
cargo fmt
```

### Running Examples

```bash
# Debug mode (faster compilation)
cargo run --example sdo_test

# Release mode (optimized)
cargo run --release --example sdo_test
```

### Enable Logging

```powershell
# Windows PowerShell
$env:RUST_LOG="debug"
cargo run --release --example sdo_test
```

```bash
# Linux/macOS
RUST_LOG=debug cargo run --release --example sdo_test
```

## Code Style

### Rust Guidelines

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting (enforced in CI)
- Use `cargo clippy` and fix all warnings
- Write rustdoc comments for public APIs
- Use descriptive variable names
- Keep functions focused and short
- Prefer explicit error handling over `unwrap()`

### Documentation

- **Public APIs** must have rustdoc comments
- **Examples** in doc comments should compile (use `no_run` if hardware needed)
- **Explain parameters** and return values
- **Document errors** that can be returned
- **Add usage examples** for complex features

Example:
```rust
/// Reads a 32-bit unsigned integer from the object dictionary.
///
/// # Arguments
///
/// * `node_id` - CANopen node ID (1-127)
/// * `index` - Object dictionary index (0x0000-0xFFFF)
/// * `subindex` - Object dictionary subindex (0x00-0xFF)
///
/// # Returns
///
/// The 32-bit value read from the device.
///
/// # Errors
///
/// - `CANopenError::Timeout` - Node did not respond within timeout
/// - `CANopenError::Sdo { code }` - SDO abort with error code
/// - `CANopenError::InvalidLength` - Returned data is not 4 bytes
///
/// # Examples
///
/// ```no_run
/// # use libcanopen_client::*;
/// # async fn example(canopen: &CANopenSimple) -> Result<()> {
/// // Read device type from node 5
/// let device_type = canopen.sdo_read_u32(5, 0x1000, 0).await?;
/// println!("Device type: 0x{:08X}", device_type);
/// # Ok(())
/// # }
/// ```
pub async fn sdo_read_u32(&self, node_id: u8, index: u16, subindex: u8) -> Result<u32> {
    // Implementation...
}
```

### Async Code

- Use `async fn` for I/O operations
- Use `tokio::spawn` for background tasks
- Use channels for message passing
- Avoid blocking operations in async context
- Use `Arc` for shared ownership
- Use `RwLock` or `Mutex` for shared mutable state

### Error Handling

- Return `Result<T>` for fallible operations
- Use `thiserror` for error types
- Provide descriptive error messages
- Log errors appropriately

## Testing

### Unit Tests

Place unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_detection() {
        let msg = CanMessage::new(0x601, vec![0x40, 0x00, 0x10, 0x00]).unwrap();
        assert_eq!(msg.message_type(), MessageType::Sdo);
    }

    #[tokio::test]
    async fn test_sdo_request() {
        // Async test
    }
}
```

### Integration Tests

Place in `tests/` directory:

```rust
// tests/integration_test.rs
use libcanopen_client::*;

#[tokio::test]
async fn test_full_workflow() {
    // Test that requires full library
}
```

### Test Coverage

- Write tests for new features
- Test edge cases and error conditions
- Test async behavior
- Use mock hardware when possible

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `build`: Build system changes
- `ci`: CI/CD changes
- `chore`: Other changes

### Examples

```
feat(lss): Add LSS Fastscan protocol support

Implements the LSS Fastscan protocol for fast node identification.
Includes unit tests and example code.

Closes #123
```

```
fix(sdo): Correct segmented transfer toggle bit handling

The toggle bit was not being inverted correctly in some cases,
causing transfer failures. Fixed by ensuring proper state tracking.

Fixes #456
```

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create git tag: `git tag -a v0.2.0 -m "Release v0.2.0"`
4. Push tag: `git push origin v0.2.0`
5. Publish to crates.io: `cargo publish`

## Questions?

- **Issues**: For bugs and features
- **Discussions**: For questions and ideas
- **Email**: For private concerns

Thank you for contributing! 🎉

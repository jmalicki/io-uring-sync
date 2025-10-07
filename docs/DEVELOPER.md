# Developer Documentation

This document outlines the development practices and guidelines for arsync.

## Development Standards

### Semantic Versioning (SemVer)

We follow [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html):

- **MAJOR** version when you make incompatible API changes
- **MINOR** version when you add functionality in a backwards compatible manner  
- **PATCH** version when you make backwards compatible bug fixes

**Examples:**
- `0.1.0` → `0.1.1`: Bug fixes
- `0.1.1` → `0.2.0`: New features (backwards compatible)
- `0.2.0` → `1.0.0`: Breaking changes or stable release

### Conventional Commits

We use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification:

**Format:** `<type>[optional scope]: <description>`

**Types:**
- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation only changes
- `style`: Changes that do not affect the meaning of the code
- `refactor`: A code change that neither fixes a bug nor adds a feature
- `perf`: A code change that improves performance
- `test`: Adding missing tests or correcting existing tests
- `chore`: Changes to the build process or auxiliary tools
- `ci`: Changes to CI configuration files and scripts

**Examples:**
```bash
feat(copy): add copy_file_range support for same-filesystem operations
fix(xattr): handle missing extended attributes gracefully
docs(api): update copy function documentation
perf(io): optimize buffer management for large files
test(integration): add end-to-end copy verification tests
```

### Branch Management

We follow [GitFlow](https://nvie.com/posts/a-successful-git-branching-model/):

- `main`: Production-ready code
- `develop`: Integration branch for features
- `feature/*`: Feature branches (e.g., `feature/copy-file-range`)
- `bugfix/*`: Bug fix branches (e.g., `bugfix/xattr-handling`)
- `release/*`: Release preparation branches
- `hotfix/*`: Critical bug fixes for production

### Code Quality

#### Pre-commit Hooks

Pre-commit hooks ensure code quality before commits:

```bash
# Install pre-commit
pip install pre-commit

# Install hooks
pre-commit install

# Run manually
pre-commit run --all-files
```

#### Rustfmt and Clippy

All code must pass `rustfmt` and `clippy`:

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
```

#### Testing Requirements

- **Unit Tests**: Required for all public functions
- **Integration Tests**: Required for end-to-end workflows
- **Property Tests**: Encouraged for complex algorithms
- **Benchmark Tests**: Required for performance-critical code

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Run benchmarks
cargo bench
```

## Development Workflow

### 1. Setup Development Environment

```bash
# Clone repository
git clone https://github.com/yourusername/arsync.git
cd arsync

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development dependencies
cargo install cargo-tarpaulin cargo-criterion

# Install pre-commit hooks
pre-commit install
```

### 2. Making Changes

```bash
# Create feature branch
git checkout -b feature/your-feature-name

# Make changes and commit
git add .
git commit -m "feat(copy): add your feature description"

# Push and create PR
git push origin feature/your-feature-name
```

### 3. Pull Request Process

1. **Create PR**: Use the provided template
2. **CI Checks**: All CI checks must pass
3. **Code Review**: At least one approval required
4. **Merge**: Squash and merge to maintain clean history

### 4. Release Process

```bash
# Update version in Cargo.toml
# Update CHANGELOG.md
# Create release branch
git checkout -b release/v0.1.0

# Tag release
git tag v0.1.0
git push origin v0.1.0

# GitHub Actions will automatically build and release
```

## Architecture Guidelines

### Module Organization

```
src/
├── main.rs           # CLI entry point
├── cli.rs            # Command-line interface
├── error.rs          # Error types and handling
├── sync.rs           # Main synchronization logic
├── copy/             # File copying operations
│   ├── mod.rs
│   ├── copy_file_range.rs
│   ├── splice.rs
│   └── read_write.rs
├── metadata/         # Metadata handling
│   ├── mod.rs
│   ├── xattr.rs
│   └── permissions.rs
├── progress/         # Progress tracking
└── queue/            # Queue management
    ├── mod.rs
    └── per_cpu.rs
```

### Error Handling

Use `thiserror` for error types:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CopyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },
}
```

### Async/Await Patterns

Prefer async/await over manual futures:

```rust
// Good
async fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    let data = read_file(src).await?;
    write_file(dst, data).await?;
    Ok(())
}

// Avoid
fn copy_file(src: &Path, dst: &Path) -> impl Future<Output = Result<()>> {
    read_file(src)
        .and_then(|data| write_file(dst, data))
        .map(|_| Ok(()))
}
```

### Documentation

#### Code Documentation

Use rustdoc for all public APIs:

```rust
/// Copies a file using the specified method.
///
/// # Arguments
/// * `src` - Source file path
/// * `dst` - Destination file path  
/// * `method` - Copy method to use
///
/// # Examples
/// ```rust
/// # use arsync::copy::copy_file;
/// # use std::path::Path;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let src = Path::new("source.txt");
/// let dst = Path::new("dest.txt");
/// copy_file(src, dst, CopyMethod::Auto).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns an error if the copy operation fails.
pub async fn copy_file(src: &Path, dst: &Path, method: CopyMethod) -> Result<()> {
    // Implementation
}
```

#### README and Documentation

- Keep `README.md` up to date with usage examples
- Use `docs/` directory for detailed documentation
- Include performance benchmarks and comparisons
- Document all command-line options

## Testing Guidelines

### Test Categories

1. **Unit Tests**: Test individual functions in isolation
2. **Integration Tests**: Test component interactions
3. **End-to-End Tests**: Test complete workflows
4. **Performance Tests**: Benchmark critical paths
5. **Property Tests**: Test invariants with random data

### Test Data Management

```rust
// Use tempfile for test directories
use tempfile::TempDir;

#[test]
fn test_file_copy() {
    let temp_dir = TempDir::new().unwrap();
    let src = temp_dir.path().join("source.txt");
    let dst = temp_dir.path().join("dest.txt");
    
    // Test implementation
}
```

### Mocking and Stubs

Use dependency injection for testability:

```rust
pub trait FileSystem {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<()>;
}

// Production implementation
pub struct RealFileSystem;

// Test implementation
pub struct MockFileSystem {
    // Mock data
}
```

## Performance Guidelines

### Benchmarking

Use `criterion` for performance benchmarks:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_copy_methods(c: &mut Criterion) {
    let mut group = c.benchmark_group("copy_methods");
    
    group.bench_function("copy_file_range", |b| {
        b.iter(|| copy_file_range(black_box(&src), black_box(&dst)))
    });
    
    group.finish();
}

criterion_group!(benches, benchmark_copy_methods);
criterion_main!(benches);
```

### Memory Management

- Use `Bytes` for zero-copy operations where possible
- Implement buffer pooling for high-throughput scenarios
- Monitor memory usage with `cargo bench`

### CPU Optimization

- Use per-CPU queues to avoid cross-CPU synchronization
- Pin threads to specific CPU cores
- Profile with `perf` or `flamegraph`

## Security Considerations

### Input Validation

- Validate all file paths
- Check permissions before operations
- Sanitize user input

### Resource Limits

- Implement queue depth limits
- Set memory usage bounds
- Handle disk space exhaustion gracefully

## Contributing

### Getting Help

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Questions and general discussion
- **Code Review**: All changes require review

### Code of Conduct

We follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

### License

This project is licensed under the MIT OR Apache-2.0 license.

## References

- [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
- [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
- [GitFlow Branching Model](https://nvie.com/posts/a-successful-git-branching-model/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [io_uring Documentation](https://kernel.dk/io_uring.pdf)
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Async Rust Best Practices](https://rust-lang.github.io/async-book/)
- [Criterion.rs Documentation](https://docs.rs/criterion/)

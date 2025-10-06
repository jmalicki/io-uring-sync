# Banned APIs Lint for io-uring-sync

A custom Dylint linter that enforces the `io_uring`-first policy for the `io-uring-sync` project by flagging direct usage of traditional POSIX APIs.

## Overview

This linter is designed to catch and flag usage of `libc::`, `nix::`, and `std::fs::` APIs to encourage the use of high-performance `io_uring` operations via `compio` and `compio-fs-extended`.

## Why This Linter Exists

The `io-uring-sync` project aims to be a high-performance file synchronization tool that leverages Linux's `io_uring` interface for maximum efficiency. Traditional POSIX APIs (`libc`, `nix`, `std::fs`) bypass the performance benefits of `io_uring` and should be avoided unless there's a strong technical justification.

## What It Catches

The linter detects and flags:

1. **Direct imports**: `use libc::*`, `use nix::*`, `use std::fs::*`
2. **Symbol usage**: Any reference to symbols from these crates, even when imported
3. **Method calls**: Calls to methods from these crates
4. **Function calls**: Direct function calls to these APIs

## How It Works

The linter uses Rust's type system and def-path resolution to track the origin of symbols, ensuring that even aliased or re-exported APIs are caught. It operates at the AST level to provide precise error reporting with helpful suggestions.

## Banned APIs

- **`libc::`**: Raw FFI bindings to C standard library functions
- **`nix::`**: Safe Rust bindings to Unix system calls
- **`std::fs::`**: Standard library filesystem operations

## Approved Alternatives

Instead of banned APIs, use:

- **`compio::fs`**: Async filesystem operations via `io_uring`
- **`compio-fs-extended`**: Extended `io_uring` operations (fadvise, fallocate, etc.)
- **`io_uring::opcode::*`**: Direct `io_uring` opcodes when available

## Usage

### Running the Linter

```bash
# Install cargo-dylint
cargo install cargo-dylint

# Run the linter on the project
cargo dylint ban_apis::BAN_LIBC_NIX_STDFS --workspace
```

### Exception Handling

When a banned API must be used (e.g., for operations not yet available in `io_uring`), add an explicit `#[allow(BAN_LIBC_NIX_STDFS)]` attribute with a comment explaining the technical justification:

```rust
#[allow(BAN_LIBC_NIX_STDFS)] // TODO: Replace with io_uring opcode when available
use std::fs::read_to_string;
```

## Examples

### ❌ Bad - Will be flagged

```rust
use libc::statx;
use std::fs::read_to_string;
use nix::fcntl::readlinkat;

fn bad_example() {
    let content = std::fs::read_to_string("file.txt").unwrap();
    let stat = unsafe { libc::statx(0, "path", 0, 0, &mut statx) };
}
```

### ✅ Good - Use io_uring alternatives

```rust
use compio::fs::File;
use compio_fs_extended::metadata::statx;
use compio_fs_extended::symlink::read_symlink_at_dirfd;

async fn good_example() {
    let content = File::open("file.txt").await?.read_to_string().await?;
    let stat = statx("path").await?;
}
```

## Technical Details

### Def-Path Resolution

The linter uses Rust's def-path resolution to track the origin of symbols, ensuring that even aliased or re-exported APIs are caught. This provides comprehensive coverage of banned API usage.

### AST-Level Analysis

The linter operates at the AST level to provide precise error reporting with helpful suggestions and proper IDE integration.

### Comprehensive Coverage

The linter catches all forms of banned API usage:
- Direct imports from banned crates
- Symbol references through any import path
- Method calls on types from banned crates
- Function calls to banned APIs

## Development

### Building the Linter

```bash
cd lints/ban-apis
cargo build
```

### Testing the Linter

```bash
cd lints/ban-apis
cargo test
```

### Adding New Banned APIs

To add new banned APIs, modify the `is_banned_path_str` function in `src/lib.rs`:

```rust
fn is_banned_path_str(path: &str) -> bool {
    path.starts_with("libc::") 
        || path.starts_with("nix::") 
        || path.starts_with("std::fs::")
        || path.starts_with("new_banned_crate::") // Add new banned APIs here
}
```

## Integration

### CI/CD Integration

Add the linter to your CI pipeline:

```yaml
- name: Run banned APIs linter
  run: cargo dylint ban_apis::BAN_LIBC_NIX_STDFS --workspace
```

### Pre-commit Hooks

Set up pre-commit hooks to run the linter automatically:

```bash
# Install pre-commit
pip install pre-commit

# Add to .pre-commit-config.yaml
- repo: local
  hooks:
    - id: banned-apis-lint
      name: Banned APIs Lint
      entry: cargo dylint ban_apis::BAN_LIBC_NIX_STDFS
      language: system
      files: \.rs$
```

## Contributing

When contributing to this linter:

1. **Documentation**: All functions should have comprehensive documentation
2. **Testing**: Add tests for new functionality
3. **Performance**: Consider the impact on compilation time
4. **Compatibility**: Ensure compatibility with different Rust versions

## License

This linter is part of the `io-uring-sync` project and is licensed under the MIT License.

## See Also

- [Dylint Documentation](https://github.com/trailofbits/dylint)
- [io-uring-sync Project](../README.md)
- [compio-fs-extended Documentation](../crates/compio-fs-extended/README.md)

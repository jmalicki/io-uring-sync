# compio-fs-extended

Extended filesystem operations for compio with support for copy_file_range, fadvise, symlinks, hardlinks, metadata operations, and extended attributes.

## Features

- **copy_file_range**: Efficient same-filesystem file copying using direct syscalls
- **fadvise**: File access pattern optimization using posix_fadvise with type-safe advice enum
- **fallocate**: File space preallocation and manipulation using io_uring
- **Symlink operations**: Create and read symbolic links using io_uring and secure *at variants
- **Hardlink operations**: Create and manage hard links using io_uring
- **Directory operations**: Enhanced directory creation and management with DirectoryFd for security
- **Metadata operations**: File permissions, timestamps, and ownership using std::fs fallbacks
- **Extended attributes (xattr)**: Support for extended attributes using io_uring opcodes
- **Device operations**: Special file creation (pipes, devices, sockets) with nix integration
- **Robust error handling**: Distinguishes between I/O errors and spawn failures, preserving kernel errno

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
compio-fs-extended = "0.1.0"
```

## Examples

### Copy File Range

```rust
use compio_fs_extended::ExtendedFile;
use compio::fs::File;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src_file = File::open("source.txt").await?;
    let dst_file = File::create("destination.txt").await?;
    
    let src_extended = ExtendedFile::new(src_file);
    let dst_extended = ExtendedFile::new(dst_file);
    
    // Use copy_file_range for efficient copying
    let bytes_copied = src_extended.copy_file_range(&dst_extended, 0, 0, 1024).await?;
    println!("Copied {} bytes", bytes_copied);
    
    Ok(())
}
```

### File Access Pattern Optimization

```rust
use compio_fs_extended::{ExtendedFile, Fadvise};
use compio::fs::File;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("large_file.txt").await?;
    let extended_file = ExtendedFile::new(file);
    
    // Optimize for sequential access
    extended_file.fadvise(libc::POSIX_FADV_SEQUENTIAL, 0, 0).await?;
    
    Ok(())
}
```

### Symlink Operations

```rust
use compio_fs_extended::{ExtendedFile, SymlinkOps};
use compio::fs::File;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("symlink.txt").await?;
    let extended_file = ExtendedFile::new(file);
    
    // Read symlink target
    let target = extended_file.read_symlink().await?;
    println!("Symlink points to: {:?}", target);
    
    Ok(())
}
```

## Architecture

This crate extends `compio::fs::File` with additional operations that are not available in the base compio-fs crate. It uses:

- **Direct syscalls** for most operations (copy_file_range, fadvise, symlinks, hardlinks)
- **io_uring opcodes** for extended attributes (IORING_OP_SETXATTR, IORING_OP_GETXATTR, IORING_OP_LISTXATTR)
- **compio runtime integration** for all async operations

## Requirements

- Linux kernel 5.6+ (for copy_file_range and xattr support)
- Rust 1.90+
- compio 0.16+

## License

MIT

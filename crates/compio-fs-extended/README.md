# compio-fs-extended

Extended filesystem operations for compio with support for copy_file_range, fadvise, symlinks, hardlinks, and extended attributes.

## Features

- **copy_file_range**: Efficient same-filesystem file copying using direct syscalls
- **fadvise**: File access pattern optimization using posix_fadvise
- **Symlink operations**: Create and read symbolic links
- **Hardlink operations**: Create and manage hard links
- **Directory operations**: Enhanced directory creation and management
- **Extended attributes (xattr)**: Support for extended attributes using io_uring opcodes

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

### Comprehensive Example: File Operations with Metadata

```rust
use compio_fs_extended::{ExtendedFile, XattrOps, Fadvise, FadviseAdvice};
use compio::fs::File;
use std::path::Path;

#[compio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = Path::new("example.txt");
    
    // Create a file with extended operations
    let file = File::create(file_path).await?;
    let extended_file = ExtendedFile::new(file);
    
    // Set extended attributes
    extended_file.set_xattr("user.author", b"compio-fs-extended").await?;
    extended_file.set_xattr("user.version", b"1.0.0").await?;
    
    // Optimize for sequential access
    extended_file.fadvise(FadviseAdvice::Sequential, 0, 0).await?;
    
    // Get extended attributes
    let author = extended_file.get_xattr("user.author").await?;
    println!("Author: {}", String::from_utf8_lossy(&author));
    
    // List all attributes
    let attrs = extended_file.list_xattr().await?;
    for attr in attrs {
        println!("Attribute: {}", attr);
    }
    
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

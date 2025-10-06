# compio-fs-extended

Extended filesystem operations for compio with comprehensive support for advanced filesystem features using io_uring and direct syscalls.

## Features

- **copy_file_range**: Efficient same-filesystem file copying using direct syscalls
- **fadvise**: File access pattern optimization with type-safe enum API
- **fallocate**: File space preallocation and hole punching
- **statx**: High-precision file metadata with nanosecond timestamps
- **Symlink operations**: Create, read, and manage symbolic links
- **Hardlink operations**: Create and manage hard links with duplicate detection
- **Directory operations**: Enhanced directory creation with DirectoryFd for efficient operations
- **Extended attributes (xattr)**: Full xattr support using io_uring opcodes
- **Device operations**: Special file creation (pipes, devices, sockets)
- **Metadata operations**: File permissions, timestamps, and ownership

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
compio-fs-extended = "0.1.0"
```

## Examples

### Copy File Range with Fadvise Optimization

```rust
use compio_fs_extended::{ExtendedFile, Fadvise, FadviseAdvice, Fallocate};
use compio::fs::File;

#[compio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src_file = File::open("source.txt").await?;
    let dst_file = File::create("destination.txt").await?;
    
    let src_extended = ExtendedFile::new(src_file);
    let dst_extended = ExtendedFile::new(dst_file);
    
    // Optimize for "one and done" copy
    src_extended.fadvise(FadviseAdvice::NoReuse, 0, 0).await?;
    
    // Preallocate destination space
    dst_extended.fallocate(0, 1024, 0).await?;
    
    // Use copy_file_range for efficient copying
    let bytes_copied = src_extended.copy_file_range(&dst_extended, 0, 0, 1024).await?;
    println!("Copied {} bytes", bytes_copied);
    
    Ok(())
}
```

### Directory Operations with DirectoryFd

```rust
use compio_fs_extended::{DirectoryFd};
use std::path::Path;

#[compio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open directory for efficient operations
    let dir_fd = DirectoryFd::open(Path::new("/tmp/project")).await?;
    
    // Create multiple directories efficiently
    dir_fd.create_directory("feature1", 0o755).await?;
    dir_fd.create_directory("feature2", 0o755).await?;
    dir_fd.create_directory("docs", 0o755).await?;
    
    // Get file descriptor for other operations
    let fd = dir_fd.fd();
    println!("Directory FD: {}", fd);
    
    Ok(())
}
```

### Extended Attributes (Xattr)

```rust
use compio_fs_extended::{ExtendedFile, XattrOps};
use compio::fs::File;

#[compio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create("metadata.txt").await?;
    let extended_file = ExtendedFile::new(file);
    
    // Set extended attributes
    extended_file.set_xattr("user.author", b"compio-fs-extended").await?;
    extended_file.set_xattr("user.version", b"1.0.0").await?;
    
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

### High-Precision File Metadata

```rust
use compio_fs_extended::{statx, StatxMetadata};
use std::path::Path;

#[compio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metadata = statx(Path::new("file.txt")).await?;
    
    println!("Size: {} bytes", metadata.size);
    println!("Accessed: {:?}", metadata.accessed);
    println!("Modified: {:?}", metadata.modified);
    println!("Mode: {:o}", metadata.mode);
    println!("Inode: {}", metadata.ino);
    
    Ok(())
}
```

### File Access Pattern Optimization

```rust
use compio_fs_extended::{ExtendedFile, Fadvise, FadviseAdvice};
use compio::fs::File;

#[compio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("large_file.txt").await?;
    let extended_file = ExtendedFile::new(file);
    
    // Optimize for different access patterns
    extended_file.fadvise(FadviseAdvice::Sequential, 0, 0).await?;
    // or
    extended_file.fadvise(FadviseAdvice::Random, 0, 0).await?;
    // or
    extended_file.fadvise(FadviseAdvice::DontNeed, 0, 0).await?;
    
    Ok(())
}
```

### Symlink and Hardlink Operations

```rust
use compio_fs_extended::{ExtendedFile, SymlinkOps, HardlinkOps};
use compio::fs::File;

#[compio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Symlink operations
    let symlink_file = File::open("symlink.txt").await?;
    let symlink_extended = ExtendedFile::new(symlink_file);
    let target = symlink_extended.read_symlink().await?;
    println!("Symlink points to: {:?}", target);
    
    // Hardlink operations
    let file = File::open("original.txt").await?;
    let extended_file = ExtendedFile::new(file);
    let link_count = extended_file.link_count().await?;
    println!("Link count: {}", link_count);
    
    Ok(())
}
```

## Architecture

This crate extends `compio::fs::File` with additional operations that are not available in the base compio-fs crate. It uses:

- **Direct syscalls** for most operations (copy_file_range, fadvise, symlinks, hardlinks)
- **io_uring opcodes** for extended attributes and advanced operations
- **compio runtime integration** with `spawn_blocking` for all async operations
- **Type-safe APIs** with proper error handling and lifetime management

## API Overview

### Core Types
- `ExtendedFile`: Wrapper around `compio::fs::File` with extended operations
- `DirectoryFd`: Efficient directory handle for relative operations
- `StatxMetadata`: High-precision file metadata
- `FadviseAdvice`: Type-safe file access pattern hints

### Operation Categories
- **File Operations**: copy_file_range, fadvise, fallocate, statx
- **Directory Operations**: DirectoryFd, recursive creation, size calculation
- **Link Operations**: symlinks, hardlinks, duplicate detection
- **Extended Attributes**: set/get/list/remove xattr operations
- **Device Operations**: pipes, character/block devices, sockets
- **Metadata Operations**: permissions, timestamps, ownership

## Performance Features

- **Efficient directory operations** using DirectoryFd and `*at` syscalls
- **Optimized file copying** with copy_file_range for same-filesystem transfers
- **Memory-mapped operations** with fadvise for access pattern optimization
- **Preallocation** with fallocate for write performance
- **High-precision timestamps** with statx for nanosecond accuracy

## Requirements

- **Linux kernel 5.6+** (for copy_file_range and xattr support)
- **Rust 1.90+**
- **compio 0.16+**
- **Filesystem support** for extended attributes (ext4, xfs, btrfs, etc.)

## Testing

The crate includes comprehensive tests:
- **27 unit tests** covering all functionality
- **Integration tests** with real-world scenarios
- **Performance tests** for timing validation
- **Error handling tests** for edge cases
- **Filesystem compatibility tests** for graceful degradation

## License

MIT
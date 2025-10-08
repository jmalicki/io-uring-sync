*Read this in other languages: [English](../../README.md) | [Pirate 🏴‍☠️](README.pirate.md)*

---

# compio-fs-extended (Pirate Edition)

Extended treasure vault operations fer compio with support fer copy_file_range, fadvise, treasure maps (symlinks), treasure links (hardlinks), and extended treasure markings.

## Features (What's in the Hold)

- **copy_file_range**: Efficient same-vault treasure plunderin' usin' direct syscalls
- **fadvise**: Treasure access pattern optimization usin' posix_fadvise
- **Treasure map operations (Symlink)**: Create and read symbolic treasure maps
- **Treasure link operations (Hardlink)**: Create and manage hard treasure links
- **Cargo hold operations (Directory)**: Enhanced cargo hold creation and management
- **Extended treasure markings (xattr)**: Complete support fer extended attributes usin' treasure descriptor operations

## Usage (How to Use This Booty)

Add this to yer `Cargo.toml`:

```toml
[dependencies]
compio-fs-extended = "0.1.0"
```

## Examples (Showin' Ye the Ropes)

### Plunder Treasure Range (Copy File Range)

```rust
use compio_fs_extended::ExtendedFile;
use compio::fs::File;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src_treasure = File::open("source.txt").await?;
    let dst_treasure = File::create("destination.txt").await?;
    
    let src_extended = ExtendedFile::new(src_treasure);
    let dst_extended = ExtendedFile::new(dst_treasure);
    
    // Use copy_file_range fer efficient plunderin'
    let doubloons_plundered = src_extended.copy_file_range(&dst_extended, 0, 0, 1024).await?;
    println!("Plundered {} doubloons", doubloons_plundered);
    
    Ok(())
}
```

### Treasure Access Pattern Optimization

```rust
use compio_fs_extended::{ExtendedFile, Fadvise};
use compio::fs::File;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let treasure = File::open("large_treasure.txt").await?;
    let extended_treasure = ExtendedFile::new(treasure);
    
    // Optimize fer sequential plunderin'
    extended_treasure.fadvise(libc::POSIX_FADV_SEQUENTIAL, 0, 0).await?;
    
    Ok(())
}
```

### Treasure Map Operations (Symlink)

```rust
use compio_fs_extended::{ExtendedFile, SymlinkOps};
use compio::fs::File;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let treasure_map = File::open("treasure_map.txt").await?;
    let extended_map = ExtendedFile::new(treasure_map);
    
    // Read treasure map target
    let target = extended_map.read_symlink().await?;
    println!("Treasure map points to: {:?}", target);
    
    Ok(())
}
```

### Comprehensive Example: Treasure Operations with Markings

```rust
use compio_fs_extended::{ExtendedFile, XattrOps, Fadvise, FadviseAdvice, OwnershipOps};
use compio::fs::File;
use std::path::Path;

#[compio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let treasure_path = Path::new("example.txt");
    
    // Create a treasure with extended operations
    let treasure = File::create(treasure_path).await?;
    let extended_treasure = ExtendedFile::new(treasure);
    
    // Set extended treasure markings
    extended_treasure.set_xattr("user.pirate", b"Blackbeard").await?;
    extended_treasure.set_xattr("user.ship", b"Queen Anne's Revenge").await?;
    
    // Optimize fer sequential plunderin'
    extended_treasure.fadvise(FadviseAdvice::Sequential, 0, 0).await?;
    
    // Set treasure ownership (requires captain privileges)
    extended_treasure.fchown(1000, 1000).await?;
    
    // Get extended treasure markings
    let pirate = extended_treasure.get_xattr("user.pirate").await?;
    println!("Pirate: {}", String::from_utf8_lossy(&pirate));
    
    // List all markings
    let markings = extended_treasure.list_xattr().await?;
    for marking in markings {
        println!("Treasure marking: {}", marking);
    }
    
    Ok(())
}
```

### Cargo Hold Operations with Metadata Preservation

```rust
use compio_fs_extended::{ExtendedFile, XattrOps, OwnershipOps};
use compio::fs::File;
use std::path::Path;

#[compio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hold_path = Path::new("cargo_hold");
    
    // Create cargo hold
    std::fs::create_dir(hold_path)?;
    
    // Open cargo hold fer metadata operations
    let hold = File::open(hold_path).await?;
    let extended_hold = ExtendedFile::new(hold);
    
    // Set cargo hold extended markings
    extended_hold.set_xattr("user.purpose", b"plunder storage").await?;
    extended_hold.set_xattr("user.created", b"2024-01-01").await?;
    
    // Set cargo hold ownership
    extended_hold.fchown(1000, 1000).await?;
    
    // List cargo hold markings
    let markings = extended_hold.list_xattr().await?;
    for marking in markings {
        println!("Cargo hold marking: {}", marking);
    }
    
    Ok(())
}
```

## Architecture (How the Ship be Built)

This crate extends `compio::fs::File` with additional operations that ain't available in the base compio-fs crate. It uses:

- **Direct syscalls** fer most operations (copy_file_range, fadvise, treasure maps, treasure links)
- **Treasure descriptor operations** fer extended markings (fgetxattr, fsetxattr, flistxattr)
- **compio runtime integration** fer all async operations (keeps the ship sailin' smooth)

## Requirements (What Ye Need Aboard)

- Linux kernel 5.6+ (fer copy_file_range and xattr support)
- Rust 1.90+
- compio 0.16+

## License

MIT

**Arrr! May yer treasure plunderin' be swift and yer holds be full! 🏴‍☠️**


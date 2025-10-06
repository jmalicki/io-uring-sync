//! Device file and special file operations
//!
//! This module provides operations for creating and managing special files
//! including device files, named pipes (FIFOs), and sockets using io_uring
//! opcodes where available, with spawn_blocking fallbacks for missing opcodes.
//!
//! # Special File Types
//!
//! - **Named Pipes (FIFOs)**: Inter-process communication
//! - **Character Devices**: Serial ports, terminals, etc.
//! - **Block Devices**: Hard drives, SSDs, etc.
//! - **Sockets**: Network and Unix domain sockets
//!
//! # Usage
//!
//! ```rust,no_run
//! use compio_fs_extended::device::{create_special_file_at_path, create_named_pipe_at_path};
//! use std::path::Path;
//!
//! # async fn example() -> compio_fs_extended::Result<()> {
//! // Create a named pipe
//! let pipe_path = Path::new("/tmp/my_pipe");
//! create_named_pipe_at_path(pipe_path, 0o644).await?;
//!
//! // Create a character device
//! let dev_path = Path::new("/tmp/my_device");
//! create_special_file_at_path(dev_path, 0o200000 | 0o644, 0x1234).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{ExtendedError, Result};
use nix::sys::stat;
use nix::unistd;
use std::path::Path;

/// Create a special file at the given path using async spawn
///
/// # Arguments
///
/// * `path` - Path where the special file should be created
/// * `mode` - File mode and type (e.g., S_IFIFO for named pipe, S_IFCHR for character device)
/// * `dev` - Device number (for device files, 0 for others)
///
/// # Returns
///
/// `Ok(())` if the special file was created successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The pathname already exists
/// - Permission is denied
/// - Invalid mode or device number
/// - The operation fails due to I/O errors
pub async fn create_special_file_at_path(path: &Path, mode: u32, dev: u64) -> Result<()> {
    let path = path.to_path_buf();

    compio::runtime::spawn(async move {
        stat::mknod(
            &path,
            stat::SFlag::from_bits_truncate(mode),
            stat::Mode::from_bits_truncate(mode & 0o777),
            dev as u64,
        )
        .map_err(|e| device_error(&format!("mknod failed: {}", e)))
    })
    .await
    .map_err(|e| device_error(&format!("spawn failed: {:?}", e)))?
}

/// Create a named pipe (FIFO) at the given path using async spawn
///
/// # Arguments
///
/// * `path` - Path where the named pipe should be created
/// * `mode` - File mode for the named pipe (e.g., 0o644)
///
/// # Returns
///
/// `Ok(())` if the named pipe was created successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The pathname already exists
/// - Permission is denied
/// - The operation fails due to I/O errors
pub async fn create_named_pipe_at_path(path: &Path, mode: u32) -> Result<()> {
    let path = path.to_path_buf();

    compio::runtime::spawn(async move {
        unistd::mkfifo(&path, stat::Mode::from_bits_truncate(mode & 0o777))
            .map_err(|e| device_error(&format!("mkfifo failed: {}", e)))
    })
    .await
    .map_err(|e| device_error(&format!("spawn failed: {:?}", e)))?
}

/// Create a character device at the given path
///
/// # Arguments
///
/// * `path` - Path where the character device should be created
/// * `mode` - File mode for the character device (e.g., 0o644)
/// * `major` - Major device number
/// * `minor` - Minor device number
///
/// # Returns
///
/// `Ok(())` if the character device was created successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The pathname already exists
/// - Permission is denied
/// - Invalid device numbers
/// - The operation fails due to I/O errors
pub async fn create_char_device_at_path(
    path: &Path,
    mode: u32,
    major: u32,
    minor: u32,
) -> Result<()> {
    let dev = ((major & 0xfff) << 8) | (minor & 0xff) | (((major >> 12) & 0xfffff) << 32);
    let device_mode = stat::SFlag::S_IFCHR.bits() | (mode & 0o777);

    create_special_file_at_path(path, device_mode, dev as u64).await
}

/// Create a block device at the given path
///
/// # Arguments
///
/// * `path` - Path where the block device should be created
/// * `mode` - File mode for the block device (e.g., 0o644)
/// * `major` - Major device number
/// * `minor` - Minor device number
///
/// # Returns
///
/// `Ok(())` if the block device was created successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The pathname already exists
/// - Permission is denied
/// - Invalid device numbers
/// - The operation fails due to I/O errors
pub async fn create_block_device_at_path(
    path: &Path,
    mode: u32,
    major: u32,
    minor: u32,
) -> Result<()> {
    let dev = ((major & 0xfff) << 8) | (minor & 0xff) | (((major >> 12) & 0xfffff) << 32);
    let device_mode = stat::SFlag::S_IFBLK.bits() | (mode & 0o777);

    create_special_file_at_path(path, device_mode, dev as u64).await
}

/// Create a Unix domain socket at the given path
///
/// # Arguments
///
/// * `path` - Path where the socket should be created
/// * `mode` - File mode for the socket (e.g., 0o644)
///
/// # Returns
///
/// `Ok(())` if the socket was created successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The pathname already exists
/// - Permission is denied
/// - The operation fails due to I/O errors
pub async fn create_socket_at_path(path: &Path, mode: u32) -> Result<()> {
    let socket_mode = stat::SFlag::S_IFSOCK.bits() | (mode & 0o777);

    create_special_file_at_path(path, socket_mode, 0).await
}

/// Error helper for device operations
fn device_error(msg: &str) -> ExtendedError {
    crate::error::device_error(msg)
}

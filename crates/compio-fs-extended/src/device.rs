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
use libc;
use std::ffi::CString;
use std::path::Path;

/// Create a special file at the given path using spawn_blocking
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
    let path_cstr = CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| device_error(&format!("Invalid path: {}", e)))?;

    // Use spawn_blocking since IORING_OP_MKNODAT is not available in current io-uring crate
    let path_cstr = path_cstr.clone();

    let result = compio::runtime::spawn_blocking(move || unsafe {
        libc::mknodat(
            libc::AT_FDCWD,
            path_cstr.as_ptr(),
            mode as libc::mode_t,
            dev as libc::dev_t,
        )
    })
    .await
    .map_err(|e| device_error(&format!("spawn_blocking failed: {:?}", e)))?;

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(device_error(&format!("mknodat failed: {}", errno)));
    }

    Ok(())
}

/// Create a named pipe (FIFO) at the given path using spawn_blocking
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
    let path_cstr = CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| device_error(&format!("Invalid path: {}", e)))?;

    // Use spawn_blocking since IORING_OP_MKFIFOAT is not available in current io-uring crate
    let path_cstr = path_cstr.clone();

    let result = compio::runtime::spawn_blocking(move || unsafe {
        libc::mkfifoat(libc::AT_FDCWD, path_cstr.as_ptr(), mode as libc::mode_t)
    })
    .await
    .map_err(|e| device_error(&format!("spawn_blocking failed: {:?}", e)))?;

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(device_error(&format!("mkfifoat failed: {}", errno)));
    }

    Ok(())
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
    let device_mode = libc::S_IFCHR | (mode & 0o777);

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
    let device_mode = libc::S_IFBLK | (mode & 0o777);

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
    let socket_mode = libc::S_IFSOCK | (mode & 0o777);

    create_special_file_at_path(path, socket_mode, 0).await
}

/// Error helper for device operations
fn device_error(msg: &str) -> ExtendedError {
    crate::error::device_error(msg)
}

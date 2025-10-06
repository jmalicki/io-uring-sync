//! statx operations for nanosecond precision timestamps
//!
//! This module provides statx operations using spawn_blocking for efficient
//! file metadata retrieval with nanosecond precision timestamps.
//!
//! # Operations
//!
//! - **statx**: Get file metadata with nanosecond precision timestamps
//! - **fstatx**: Get file metadata from file descriptor with nanosecond precision
//!
//! # Usage
//!
//! ```rust,no_run
//! use compio_fs_extended::statx::{statx, fstatx};
//! use std::path::Path;
//!
//! # async fn example() -> compio_fs_extended::Result<()> {
//! // Path-based statx
//! let metadata = statx(Path::new("file.txt")).await?;
//! println!("Accessed: {:?}", metadata.accessed);
//! println!("Modified: {:?}", metadata.modified);
//!
//! // File descriptor-based statx
//! let file = std::fs::File::open("file.txt")?;
//! let metadata = fstatx(file.as_raw_fd()).await?;
//! println!("Size: {}", metadata.size);
//! # Ok(())
//! # }
//! ```

use crate::error::{statx_error, Result};
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// File metadata with nanosecond precision timestamps
#[derive(Debug, Clone)]
pub struct StatxMetadata {
    /// File size in bytes
    pub size: u64,
    /// Access time with nanosecond precision
    pub accessed: SystemTime,
    /// Modification time with nanosecond precision
    pub modified: SystemTime,
    /// File permissions
    pub mode: u32,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Number of hard links
    pub nlink: u64,
    /// Device ID
    pub dev: u64,
    /// Inode number
    pub ino: u64,
}

/// Get file metadata using statx with nanosecond precision
///
/// # Arguments
///
/// * `path` - Path to the file
///
/// # Returns
///
/// Returns `Ok(StatxMetadata)` if metadata was retrieved successfully, or `Err(ExtendedError)` if failed.
///
/// # Errors
///
/// This function will return an error if:
/// - The file doesn't exist
/// - Permission is denied
/// - The path is invalid
/// - The statx operation fails
pub async fn statx(path: &Path) -> Result<StatxMetadata> {
    let path_cstr = CString::new(path.as_os_str().as_bytes())
        .map_err(|e| statx_error(&format!("Invalid path: {}", e)))?;

    // Use spawn_blocking for the statx system call
    let result = compio::runtime::spawn_blocking(move || {
        unsafe {
            let mut statx_buf = std::mem::zeroed::<libc::statx>();
            let result = libc::statx(
                libc::AT_FDCWD,
                path_cstr.as_ptr(),
                0,              // flags
                0x0000_07ffu32, // STATX_BASIC_STATS mask
                &mut statx_buf,
            );

            if result == 0 {
                Ok(statx_buf)
            } else {
                Err(std::io::Error::last_os_error())
            }
        }
    })
    .await
    .map_err(|e| statx_error(&format!("Failed to execute statx: {:?}", e)))?;

    let statx_buf = result.map_err(|e| statx_error(&format!("statx system call failed: {}", e)))?;

    // Convert statx result to our metadata structure
    let accessed = UNIX_EPOCH
        + std::time::Duration::new(
            statx_buf.stx_atime.tv_sec as u64,
            statx_buf.stx_atime.tv_nsec as u32,
        );
    let modified = UNIX_EPOCH
        + std::time::Duration::new(
            statx_buf.stx_mtime.tv_sec as u64,
            statx_buf.stx_mtime.tv_nsec as u32,
        );

    Ok(StatxMetadata {
        size: statx_buf.stx_size,
        accessed,
        modified,
        mode: statx_buf.stx_mode as u32,
        uid: statx_buf.stx_uid as u32,
        gid: statx_buf.stx_gid as u32,
        nlink: statx_buf.stx_nlink as u64,
        dev: ((statx_buf.stx_dev_major as u64) << 32) | (statx_buf.stx_dev_minor as u64),
        ino: statx_buf.stx_ino,
    })
}

/// Get file metadata using fstatx with nanosecond precision
///
/// # Arguments
///
/// * `fd` - File descriptor
///
/// # Returns
///
/// Returns `Ok(StatxMetadata)` if metadata was retrieved successfully, or `Err(ExtendedError)` if failed.
///
/// # Errors
///
/// This function will return an error if:
/// - Invalid file descriptor
/// - Permission is denied
/// - The fstatx operation fails
pub async fn fstatx(fd: i32) -> Result<StatxMetadata> {
    // Use spawn_blocking for the fstatx system call
    let result = compio::runtime::spawn_blocking(move || {
        unsafe {
            let mut statx_buf = std::mem::zeroed::<libc::statx>();
            let result = libc::statx(
                fd,
                std::ptr::null(),
                0,              // flags
                0x0000_07ffu32, // STATX_BASIC_STATS mask
                &mut statx_buf,
            );

            if result == 0 {
                Ok(statx_buf)
            } else {
                Err(std::io::Error::last_os_error())
            }
        }
    })
    .await
    .map_err(|e| statx_error(&format!("Failed to execute fstatx: {:?}", e)))?;

    let statx_buf =
        result.map_err(|e| statx_error(&format!("fstatx system call failed: {}", e)))?;

    // Convert statx result to our metadata structure
    let accessed = UNIX_EPOCH
        + std::time::Duration::new(
            statx_buf.stx_atime.tv_sec as u64,
            statx_buf.stx_atime.tv_nsec as u32,
        );
    let modified = UNIX_EPOCH
        + std::time::Duration::new(
            statx_buf.stx_mtime.tv_sec as u64,
            statx_buf.stx_mtime.tv_nsec as u32,
        );

    Ok(StatxMetadata {
        size: statx_buf.stx_size,
        accessed,
        modified,
        mode: statx_buf.stx_mode as u32,
        uid: statx_buf.stx_uid as u32,
        gid: statx_buf.stx_gid as u32,
        nlink: statx_buf.stx_nlink as u64,
        dev: ((statx_buf.stx_dev_major as u64) << 32) | (statx_buf.stx_dev_minor as u64),
        ino: statx_buf.stx_ino,
    })
}

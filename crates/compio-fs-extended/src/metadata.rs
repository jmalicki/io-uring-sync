//! File metadata operations using file descriptors
//!
//! This module provides file descriptor-based metadata operations for efficient
//! file attribute management without repeated path lookups. These operations
//! use spawn_blocking since the corresponding io_uring opcodes are not available.
//!
//! # Operations
//!
//! - **fchmodat**: Change file permissions using file descriptors
//! - **futimesat**: Change file timestamps using file descriptors
//! - **fchownat**: Change file ownership using file descriptors
//! - **fchmodat_with_dirfd**: Change file permissions using DirectoryFd (most efficient)
//! - **futimesat_with_dirfd**: Change file timestamps using DirectoryFd (most efficient)
//! - **fchownat_with_dirfd**: Change file ownership using DirectoryFd (most efficient)
//!
//! # Usage
//!
//! ```rust,no_run
//! use compio_fs_extended::metadata::{fchmodat, futimesat, fchownat, fchmodat_with_dirfd, futimesat_with_dirfd, fchownat_with_dirfd};
//! use compio_fs_extended::directory::DirectoryFd;
//! use std::path::Path;
//! use std::time::SystemTime;
//!
//! # async fn example() -> compio_fs_extended::Result<()> {
//! // Path-based operations (use AT_FDCWD)
//! fchmodat(Path::new("file.txt"), 0o644).await?;
//! let now = SystemTime::now();
//! futimesat(Path::new("file.txt"), now, now).await?;
//! fchownat(Path::new("file.txt"), 1000, 1000).await?;
//!
//! // DirectoryFd-based operations (most efficient)
//! let dir_fd = DirectoryFd::open(Path::new("/some/directory")).await?;
//! fchmodat_with_dirfd(dir_fd.fd(), "file.txt", 0o644).await?;
//! futimesat_with_dirfd(dir_fd.fd(), "file.txt", now, now).await?;
//! fchownat_with_dirfd(dir_fd.fd(), "file.txt", 1000, 1000).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{ExtendedError, Result};
use compio_runtime;
use libc;
use std::ffi::CString;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Change file permissions using file descriptor
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `mode` - New file permissions (e.g., 0o644)
///
/// # Returns
///
/// `Ok(())` if the permissions were changed successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The file doesn't exist
/// - Permission is denied
/// - Invalid mode value
/// - The operation fails due to I/O errors
pub async fn fchmodat(path: &Path, mode: u32) -> Result<()> {
    let path_cstr = CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| metadata_error(&format!("Invalid path: {}", e)))?;

    let path_cstr = path_cstr.clone();

    let result = compio_runtime::spawn_blocking(move || {
        unsafe {
            libc::fchmodat(
                libc::AT_FDCWD,
                path_cstr.as_ptr(),
                mode as libc::mode_t,
                0, // No flags
            )
        }
    })
    .await
    .map_err(|e| metadata_error(&format!("spawn_blocking failed: {:?}", e)))?;

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(metadata_error(&format!("fchmodat failed: {}", errno)));
    }

    Ok(())
}

/// Change file timestamps using file descriptor
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `accessed` - New access time
/// * `modified` - New modification time
///
/// # Returns
///
/// `Ok(())` if the timestamps were changed successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The file doesn't exist
/// - Permission is denied
/// - Invalid timestamp values
/// - The operation fails due to I/O errors
pub async fn futimesat(path: &Path, accessed: SystemTime, modified: SystemTime) -> Result<()> {
    let path_cstr = CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| metadata_error(&format!("Invalid path: {}", e)))?;

    // Convert SystemTime to libc::timespec
    let accessed_duration = accessed
        .duration_since(UNIX_EPOCH)
        .map_err(|e| metadata_error(&format!("Invalid access time: {}", e)))?;
    let modified_duration = modified
        .duration_since(UNIX_EPOCH)
        .map_err(|e| metadata_error(&format!("Invalid modification time: {}", e)))?;

    let accessed_ts = libc::timespec {
        tv_sec: accessed_duration.as_secs() as libc::time_t,
        tv_nsec: accessed_duration.subsec_nanos() as libc::c_long,
    };

    let modified_ts = libc::timespec {
        tv_sec: modified_duration.as_secs() as libc::time_t,
        tv_nsec: modified_duration.subsec_nanos() as libc::c_long,
    };

    let times = [accessed_ts, modified_ts];

    let path_cstr = path_cstr.clone();

    let result = compio_runtime::spawn_blocking(move || {
        unsafe {
            libc::utimensat(
                libc::AT_FDCWD,
                path_cstr.as_ptr(),
                times.as_ptr(),
                0, // No flags
            )
        }
    })
    .await
    .map_err(|e| metadata_error(&format!("spawn_blocking failed: {:?}", e)))?;

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(metadata_error(&format!("utimensat failed: {}", errno)));
    }

    Ok(())
}

/// Change file ownership using file descriptor
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `uid` - New user ID (use -1 to not change)
/// * `gid` - New group ID (use -1 to not change)
///
/// # Returns
///
/// `Ok(())` if the ownership was changed successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The file doesn't exist
/// - Permission is denied
/// - Invalid user/group IDs
/// - The operation fails due to I/O errors
pub async fn fchownat(path: &Path, uid: u32, gid: u32) -> Result<()> {
    let path_cstr = CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| metadata_error(&format!("Invalid path: {}", e)))?;

    let path_cstr = path_cstr.clone();
    let uid = uid as libc::uid_t;
    let gid = gid as libc::gid_t;

    let result = compio_runtime::spawn_blocking(move || {
        unsafe {
            libc::fchownat(
                libc::AT_FDCWD,
                path_cstr.as_ptr(),
                uid,
                gid,
                0, // No flags
            )
        }
    })
    .await
    .map_err(|e| metadata_error(&format!("spawn_blocking failed: {:?}", e)))?;

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(metadata_error(&format!("fchownat failed: {}", errno)));
    }

    Ok(())
}

/// Change file permissions using file descriptor (more efficient)
///
/// # Arguments
///
/// * `fd` - File descriptor
/// * `mode` - New file permissions (e.g., 0o644)
///
/// # Returns
///
/// `Ok(())` if the permissions were changed successfully
///
/// # Errors
///
/// This function will return an error if:
/// - Invalid file descriptor
/// - Permission is denied
/// - Invalid mode value
/// - The operation fails due to I/O errors
pub async fn fchmod(fd: i32, mode: u32) -> Result<()> {
    let result =
        compio_runtime::spawn_blocking(move || unsafe { libc::fchmod(fd, mode as libc::mode_t) })
            .await
            .map_err(|e| metadata_error(&format!("spawn_blocking failed: {:?}", e)))?;

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(metadata_error(&format!("fchmod failed: {}", errno)));
    }

    Ok(())
}

/// Change file timestamps using file descriptor (more efficient)
///
/// # Arguments
///
/// * `fd` - File descriptor
/// * `accessed` - New access time
/// * `modified` - New modification time
///
/// # Returns
///
/// `Ok(())` if the timestamps were changed successfully
///
/// # Errors
///
/// This function will return an error if:
/// - Invalid file descriptor
/// - Permission is denied
/// - Invalid timestamp values
/// - The operation fails due to I/O errors
pub async fn futimes(fd: i32, accessed: SystemTime, modified: SystemTime) -> Result<()> {
    // Convert SystemTime to libc::timespec
    let accessed_duration = accessed
        .duration_since(UNIX_EPOCH)
        .map_err(|e| metadata_error(&format!("Invalid access time: {}", e)))?;
    let modified_duration = modified
        .duration_since(UNIX_EPOCH)
        .map_err(|e| metadata_error(&format!("Invalid modification time: {}", e)))?;

    let accessed_ts = libc::timespec {
        tv_sec: accessed_duration.as_secs() as libc::time_t,
        tv_nsec: accessed_duration.subsec_nanos() as libc::c_long,
    };

    let modified_ts = libc::timespec {
        tv_sec: modified_duration.as_secs() as libc::time_t,
        tv_nsec: modified_duration.subsec_nanos() as libc::c_long,
    };

    let times = [accessed_ts, modified_ts];

    let result =
        compio_runtime::spawn_blocking(move || unsafe { libc::futimens(fd, times.as_ptr()) })
            .await
            .map_err(|e| metadata_error(&format!("spawn_blocking failed: {:?}", e)))?;

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(metadata_error(&format!("futimens failed: {}", errno)));
    }

    Ok(())
}

/// Change file ownership using file descriptor (more efficient)
///
/// # Arguments
///
/// * `fd` - File descriptor
/// * `uid` - New user ID (use -1 to not change)
/// * `gid` - New group ID (use -1 to not change)
///
/// # Returns
///
/// `Ok(())` if the ownership was changed successfully
///
/// # Errors
///
/// This function will return an error if:
/// - Invalid file descriptor
/// - Permission is denied
/// - Invalid user/group IDs
/// - The operation fails due to I/O errors
pub async fn fchown(fd: i32, uid: u32, gid: u32) -> Result<()> {
    let uid = uid as libc::uid_t;
    let gid = gid as libc::gid_t;

    let result = compio_runtime::spawn_blocking(move || unsafe { libc::fchown(fd, uid, gid) })
        .await
        .map_err(|e| metadata_error(&format!("spawn_blocking failed: {:?}", e)))?;

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(metadata_error(&format!("fchown failed: {}", errno)));
    }

    Ok(())
}

/// Change file permissions using DirectoryFd (most efficient)
///
/// # Arguments
///
/// * `dir_fd` - Directory file descriptor from DirectoryFd
/// * `pathname` - Relative path to the file
/// * `mode` - New file permissions (e.g., 0o644)
///
/// # Returns
///
/// `Ok(())` if the permissions were changed successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The file doesn't exist
/// - Permission is denied
/// - Invalid mode value
/// - The operation fails due to I/O errors
pub async fn fchmodat_with_dirfd(dir_fd: i32, pathname: &str, mode: u32) -> Result<()> {
    let pathname_cstr =
        CString::new(pathname).map_err(|e| metadata_error(&format!("Invalid pathname: {}", e)))?;

    let pathname_cstr = pathname_cstr.clone();

    let result = compio_runtime::spawn_blocking(move || {
        unsafe {
            libc::fchmodat(
                dir_fd,
                pathname_cstr.as_ptr(),
                mode as libc::mode_t,
                0, // No flags
            )
        }
    })
    .await
    .map_err(|e| metadata_error(&format!("spawn_blocking failed: {:?}", e)))?;

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(metadata_error(&format!("fchmodat failed: {}", errno)));
    }

    Ok(())
}

/// Change file timestamps using DirectoryFd (most efficient)
///
/// # Arguments
///
/// * `dir_fd` - Directory file descriptor from DirectoryFd
/// * `pathname` - Relative path to the file
/// * `accessed` - New access time
/// * `modified` - New modification time
///
/// # Returns
///
/// `Ok(())` if the timestamps were changed successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The file doesn't exist
/// - Permission is denied
/// - Invalid timestamp values
/// - The operation fails due to I/O errors
pub async fn futimesat_with_dirfd(
    dir_fd: i32,
    pathname: &str,
    accessed: SystemTime,
    modified: SystemTime,
) -> Result<()> {
    let pathname_cstr =
        CString::new(pathname).map_err(|e| metadata_error(&format!("Invalid pathname: {}", e)))?;

    // Convert SystemTime to libc::timespec
    let accessed_duration = accessed
        .duration_since(UNIX_EPOCH)
        .map_err(|e| metadata_error(&format!("Invalid access time: {}", e)))?;
    let modified_duration = modified
        .duration_since(UNIX_EPOCH)
        .map_err(|e| metadata_error(&format!("Invalid modification time: {}", e)))?;

    let accessed_ts = libc::timespec {
        tv_sec: accessed_duration.as_secs() as libc::time_t,
        tv_nsec: accessed_duration.subsec_nanos() as libc::c_long,
    };

    let modified_ts = libc::timespec {
        tv_sec: modified_duration.as_secs() as libc::time_t,
        tv_nsec: modified_duration.subsec_nanos() as libc::c_long,
    };

    let times = [accessed_ts, modified_ts];

    let pathname_cstr = pathname_cstr.clone();

    let result = compio_runtime::spawn_blocking(move || {
        unsafe {
            libc::utimensat(
                dir_fd,
                pathname_cstr.as_ptr(),
                times.as_ptr(),
                0, // No flags
            )
        }
    })
    .await
    .map_err(|e| metadata_error(&format!("spawn_blocking failed: {:?}", e)))?;

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(metadata_error(&format!("utimensat failed: {}", errno)));
    }

    Ok(())
}

/// Change file ownership using DirectoryFd (most efficient)
///
/// # Arguments
///
/// * `dir_fd` - Directory file descriptor from DirectoryFd
/// * `pathname` - Relative path to the file
/// * `uid` - New user ID (use -1 to not change)
/// * `gid` - New group ID (use -1 to not change)
///
/// # Returns
///
/// `Ok(())` if the ownership was changed successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The file doesn't exist
/// - Permission is denied
/// - Invalid user/group IDs
/// - The operation fails due to I/O errors
pub async fn fchownat_with_dirfd(dir_fd: i32, pathname: &str, uid: u32, gid: u32) -> Result<()> {
    let pathname_cstr =
        CString::new(pathname).map_err(|e| metadata_error(&format!("Invalid pathname: {}", e)))?;

    let pathname_cstr = pathname_cstr.clone();
    let uid = uid as libc::uid_t;
    let gid = gid as libc::gid_t;

    let result = compio_runtime::spawn_blocking(move || {
        unsafe {
            libc::fchownat(
                dir_fd,
                pathname_cstr.as_ptr(),
                uid,
                gid,
                0, // No flags
            )
        }
    })
    .await
    .map_err(|e| metadata_error(&format!("spawn_blocking failed: {:?}", e)))?;

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(metadata_error(&format!("fchownat failed: {}", errno)));
    }

    Ok(())
}

/// Error helper for metadata operations
fn metadata_error(msg: &str) -> ExtendedError {
    ExtendedError::Metadata(msg.to_string())
}

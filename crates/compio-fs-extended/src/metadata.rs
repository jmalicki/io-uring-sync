//! File metadata operations using file descriptors
//!
//! This module provides metadata operations using std::fs and filetime, with
//! file-descriptor and DirectoryFd variants built by resolving `/proc/self/fd`.
//! Calls run in a blocking closure scheduled via the compio runtime, since
//! native io_uring opcodes for chmod/chown/timestamps are not available.
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
use filetime::{set_file_times, FileTime};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Get the /proc/self/fd path for a file descriptor
fn proc_fd_path(fd: i32) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", fd))
}

/// Join a directory file descriptor path with a relative pathname
fn join_dirfd_path(dir_fd: i32, pathname: &str) -> Result<PathBuf> {
    let dir_path = std::fs::read_link(proc_fd_path(dir_fd))?;
    Ok(dir_path.join(pathname))
}

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
    let path = path.to_path_buf();
    let handle = compio::runtime::spawn(async move {
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(mode);
        std::fs::set_permissions(&path, perms)
    });

    match handle.await {
        Ok(inner) => inner.map_err(ExtendedError::from),
        Err(join_err) => Err(ExtendedError::SpawnJoin(format!(
            "spawn failed: {:?}",
            join_err
        ))),
    }
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
    let path = path.to_path_buf();
    let handle = compio::runtime::spawn(async move {
        let atime = FileTime::from(accessed);
        let mtime = FileTime::from(modified);
        set_file_times(&path, atime, mtime)
    });

    match handle.await {
        Ok(inner) => inner.map_err(ExtendedError::from),
        Err(join_err) => Err(ExtendedError::SpawnJoin(format!(
            "spawn failed: {:?}",
            join_err
        ))),
    }
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
pub async fn fchownat(_path: &Path, _uid: u32, _gid: u32) -> Result<()> {
    Err(metadata_error(
        "chown is not supported via std::fs; enable a libc-based path if required",
    ))
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
    let handle = compio::runtime::spawn(async move {
        let path = proc_fd_path(fd);
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(mode);
        std::fs::set_permissions(&path, perms)
    });

    match handle.await {
        Ok(inner) => inner.map_err(ExtendedError::from),
        Err(join_err) => Err(ExtendedError::SpawnJoin(format!(
            "spawn failed: {:?}",
            join_err
        ))),
    }
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
    let handle = compio::runtime::spawn(async move {
        let path = proc_fd_path(fd);
        let atime = FileTime::from(accessed);
        let mtime = FileTime::from(modified);
        set_file_times(&path, atime, mtime)
    });

    match handle.await {
        Ok(inner) => inner.map_err(ExtendedError::from),
        Err(join_err) => Err(ExtendedError::SpawnJoin(format!(
            "spawn failed: {:?}",
            join_err
        ))),
    }
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
pub async fn fchown(_fd: i32, _uid: u32, _gid: u32) -> Result<()> {
    Err(metadata_error(
        "chown is not supported via std::fs; enable a libc-based path if required",
    ))
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
    let full = join_dirfd_path(dir_fd, pathname)?;
    let handle = compio::runtime::spawn(async move {
        let mut perms = std::fs::metadata(&full)?.permissions();
        perms.set_mode(mode);
        std::fs::set_permissions(&full, perms)
    });

    match handle.await {
        Ok(inner) => inner.map_err(ExtendedError::from),
        Err(join_err) => Err(ExtendedError::SpawnJoin(format!(
            "spawn failed: {:?}",
            join_err
        ))),
    }
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
    let full = join_dirfd_path(dir_fd, pathname)?;
    let handle = compio::runtime::spawn(async move {
        let atime = FileTime::from(accessed);
        let mtime = FileTime::from(modified);
        set_file_times(&full, atime, mtime)
    });

    match handle.await {
        // preserve original std::io::Error from set_file_times
        Ok(inner) => inner.map_err(ExtendedError::from),
        // only non-inner failures (task panics/cancellations) become SpawnJoin
        Err(join_err) => Err(ExtendedError::SpawnJoin(format!(
            "spawn failed: {:?}",
            join_err
        ))),
    }
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
pub async fn fchownat_with_dirfd(
    _dir_fd: i32,
    _pathname: &str,
    _uid: u32,
    _gid: u32,
) -> Result<()> {
    Err(metadata_error(
        "chown is not supported via std::fs; enable a libc-based path if required",
    ))
}

/// Error helper for metadata operations
fn metadata_error(msg: &str) -> ExtendedError {
    crate::error::metadata_error(msg)
}

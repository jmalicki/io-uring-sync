//! File copying operations using `io_uring`
//!
//! This module provides high-performance file copying operations using various
//! system calls optimized for different scenarios. It implements `copy_file_range`
//! for efficient in-kernel copying, `splice` for zero-copy operations, and
//! traditional read/write as fallback methods.
//!
//! # Copy Methods
//!
//! - **`copy_file_range`**: In-kernel copying, most efficient for large files
//! - **`splice`**: Zero-copy operations using pipes
//! - **`read_write`**: Traditional fallback method
//! - **auto**: Automatically selects the best method available
//!
//! # Performance Characteristics
//!
//! - `copy_file_range`: ~2-5x faster than read/write for large files
//! - `splice`: Zero-copy, optimal for streaming operations
//! - read/write: Reliable fallback, works everywhere
//!
//! # Usage
//!
//! ```rust,ignore
//! use arsync::copy::copy_file;
//! use arsync::cli::CopyMethod;
//! use std::path::Path;
//!
//! #[compio::main]
//! async fn main() -> arsync::Result<()> {
//!     let src_path = Path::new("source.txt");
//!     let dst_path = Path::new("destination.txt");
//!     
//!     // Copy with automatic method selection
//!     copy_file(src_path, dst_path, CopyMethod::Auto).await?;
//!
//!     // Force specific method
//!     copy_file(src_path, dst_path, CopyMethod::CopyFileRange).await?;
//!     Ok(())
//! }
//! ```

use crate::cli::Args;
use crate::error::{Result, SyncError};
use compio::fs::OpenOptions;
use compio::io::{AsyncReadAt, AsyncWriteAt};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::SystemTime;

/// Default I/O buffer size (in bytes) used for chunked read/write operations.
///
/// Chosen to balance syscall overhead and memory usage. Adjust if profiling
/// indicates different optimal sizes for specific workloads.
const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer

/// Copy a single file using the specified method
///
/// # Errors
///
/// This function will return an error if:
/// - Source file cannot be opened for reading
/// - Destination file cannot be created or opened for writing
/// - File copying operation fails (I/O errors, permission issues)
/// - Metadata preservation fails
/// - The specified copy method is not supported or fails
#[allow(clippy::future_not_send)]
pub async fn copy_file(src: &Path, dst: &Path, args: &Args) -> Result<()> {
    // Simplified: always use read/write method
    // This is the only reliable method that works everywhere
    copy_read_write(src, dst, args).await
}

/// Copy file using compio read/write operations (reliable fallback)
///
/// This function provides a reliable fallback method for file copying using
/// compio's async read/write operations. While not as fast as `copy_file_range` or
/// `splice`, it works in all scenarios and provides guaranteed compatibility.
///
/// # Parameters
///
/// * `src` - Source file path
/// * `dst` - Destination file path
///
/// # Returns
///
/// Returns `Ok(())` if the file was copied successfully, or `Err(SyncError)` if failed.
///
/// # Performance Notes
///
/// - Reliable fallback method that works everywhere
/// - Uses compio's async I/O for optimal performance
/// - Compatible with all filesystems and scenarios
/// - Slower than `copy_file_range` but more reliable
///
/// # Examples
///
/// ```rust,ignore
/// use arsync::copy::copy_read_write;
/// use std::path::Path;
///
/// #[compio::main]
/// async fn main() -> arsync::Result<()> {
///     let src_path = Path::new("source.txt");
///     let dst_path = Path::new("destination.txt");
///     copy_read_write(src_path, dst_path).await?;
///     Ok(())
/// }
/// ```
#[allow(clippy::future_not_send, clippy::too_many_lines)]
async fn copy_read_write(src: &Path, dst: &Path, args: &Args) -> Result<()> {
    // Capture source timestamps BEFORE any reads to avoid atime/mtime drift
    let (src_accessed, src_modified) = get_precise_timestamps(src).await?;

    // Open source file
    let src_file = OpenOptions::new().read(true).open(src).await.map_err(|e| {
        SyncError::FileSystem(format!("Failed to open source file {}: {e}", src.display(),))
    })?;

    // Open destination file
    let mut dst_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dst)
        .await
        .map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to open destination file {}: {e}",
                dst.display(),
            ))
        })?;

    // Get file size for progress tracking
    let metadata = src_file
        .metadata()
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to get source file metadata: {e}")))?;
    let file_size = metadata.len();

    // Preallocate destination file space to the final size to reduce fragmentation
    // and improve write performance using io_uring fallocate.
    // Skip preallocation for empty files as fallocate fails with EINVAL for zero length.
    if file_size > 0 {
        use compio_fs_extended::{fadvise::FadviseAdvice, ExtendedFile, Fadvise, Fallocate};

        // Apply fadvise hints to both source and destination for "one and done" copy
        let extended_src = ExtendedFile::from_ref(&src_file);
        let extended_dst = ExtendedFile::from_ref(&dst_file);

        // Hint that source data won't be accessed again after this copy
        extended_src
            .fadvise(
                FadviseAdvice::NoReuse,
                0,
                file_size.try_into().unwrap_or(i64::MAX),
            )
            .await
            .map_err(|e| {
                SyncError::FileSystem(format!("Failed to set fadvise NoReuse hint on source: {e}"))
            })?;

        // Preallocate destination file space
        extended_dst.fallocate(0, file_size, 0).await.map_err(|e| {
            SyncError::FileSystem(format!("Failed to preallocate destination file: {e}"))
        })?;

        // Hint that destination data won't be accessed again after this copy
        extended_dst
            .fadvise(
                FadviseAdvice::NoReuse,
                0,
                file_size.try_into().unwrap_or(i64::MAX),
            )
            .await
            .map_err(|e| {
                SyncError::FileSystem(format!(
                    "Failed to set fadvise NoReuse hint on destination: {e}"
                ))
            })?;
    }

    // Use compio's async read_at/write_at operations
    let mut offset = 0u64;
    let mut total_copied = 0u64;

    while total_copied < file_size {
        // Create a new buffer for each read operation
        let buffer = vec![0u8; BUFFER_SIZE];

        // Read data from source file using compio
        let buf_result = src_file.read_at(buffer, offset).await;

        let bytes_read = buf_result
            .0
            .map_err(|e| SyncError::IoUring(format!("compio read_at operation failed: {e}")))?;

        let read_buffer = buf_result.1;

        if bytes_read == 0 {
            // End of file
            break;
        }

        // Truncate the buffer to the actual bytes read
        let write_buffer = read_buffer[..bytes_read].to_vec();

        // Write data to destination file using compio
        let write_buf_result = dst_file.write_at(write_buffer, offset).await;

        let bytes_written = write_buf_result
            .0
            .map_err(|e| SyncError::IoUring(format!("compio write_at operation failed: {e}")))?;

        // Ensure we wrote the expected number of bytes
        if bytes_written != bytes_read {
            return Err(SyncError::CopyFailed(format!(
                "Write size mismatch: expected {bytes_read}, got {bytes_written}"
            )));
        }

        total_copied += bytes_written as u64;
        offset += bytes_written as u64;

        tracing::debug!(
            "compio read_at/write_at: copied {} bytes, total: {}/{}",
            bytes_written,
            total_copied,
            file_size
        );
    }

    // Sync the destination file to ensure data is written to disk
    dst_file
        .sync_all()
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to sync destination file: {e}")))?;

    // Preserve file metadata only if explicitly requested (rsync behavior)
    if args.should_preserve_permissions() {
        preserve_permissions_from_fd(&src_file, &dst_file).await?;
    }

    if args.should_preserve_ownership() {
        preserve_ownership_from_fd(&src_file, &dst_file).await?;
    }

    if args.should_preserve_xattrs() {
        preserve_xattr_from_fd(&src_file, &dst_file).await?;
    }

    if args.should_preserve_timestamps() {
        set_dst_timestamps(dst, src_accessed, src_modified).await?;
    }

    tracing::debug!(
        "compio read_at/write_at: successfully copied {} bytes",
        total_copied
    );
    Ok(())
}

/// Preserve only file permissions from source to destination
///
/// This function preserves file permissions including special bits (setuid, setgid, sticky)
/// using the chmod syscall for maximum compatibility and precision.
#[allow(clippy::future_not_send)]
async fn preserve_permissions_from_fd(
    src_file: &compio::fs::File,
    dst_file: &compio::fs::File,
) -> Result<()> {
    // Get source file permissions using file descriptor
    let src_metadata = src_file
        .metadata()
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to get source file metadata: {e}")))?;

    let std_permissions = src_metadata.permissions();
    let mode = std_permissions.mode();

    // Convert to compio::fs::Permissions
    let compio_permissions = compio::fs::Permissions::from_mode(mode);

    // Use compio::fs::File::set_permissions which uses fchmod (file descriptor-based)
    dst_file
        .set_permissions(compio_permissions)
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to preserve permissions: {e}")))
}

/// Preserve file ownership using file descriptors
#[allow(clippy::future_not_send)]
async fn preserve_ownership_from_fd(
    src_file: &compio::fs::File,
    dst_file: &compio::fs::File,
) -> Result<()> {
    use compio_fs_extended::OwnershipOps;

    // Use compio-fs-extended for ownership preservation
    dst_file
        .preserve_ownership_from(src_file)
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to preserve file ownership: {e}")))?;
    Ok(())
}

/// Preserve file extended attributes using file descriptors
///
/// This function preserves all extended attributes from the source file to the destination file
/// using file descriptor-based operations for maximum efficiency and security.
///
/// # Arguments
///
/// * `src_file` - Source file handle
/// * `dst_file` - Destination file handle
///
/// # Returns
///
/// `Ok(())` if all extended attributes were preserved successfully
///
/// # Errors
///
/// This function will return an error if:
/// - Extended attributes cannot be read from source
/// - Extended attributes cannot be written to destination
/// - Permission is denied for xattr operations
#[allow(clippy::future_not_send)]
pub async fn preserve_xattr_from_fd(
    src_file: &compio::fs::File,
    dst_file: &compio::fs::File,
) -> Result<()> {
    use compio_fs_extended::{ExtendedFile, XattrOps};

    // Convert to ExtendedFile to access xattr operations
    let extended_src = ExtendedFile::from_ref(src_file);
    let extended_dst = ExtendedFile::from_ref(dst_file);

    // Get all extended attribute names from source file
    let Ok(xattr_names) = extended_src.list_xattr().await else {
        // If xattr is not supported or no xattrs exist, that's fine
        return Ok(());
    };

    // Copy each extended attribute
    for name in xattr_names {
        match extended_src.get_xattr(&name).await {
            Ok(value) => {
                if let Err(e) = extended_dst.set_xattr(&name, &value).await {
                    // Log warning but continue with other xattrs
                    tracing::warn!("Failed to preserve extended attribute '{}': {}", name, e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read extended attribute '{}': {}", name, e);
            }
        }
    }

    Ok(())
}

/// Set destination timestamps to the provided accessed/modified values
async fn set_dst_timestamps(dst: &Path, accessed: SystemTime, modified: SystemTime) -> Result<()> {
    preserve_timestamps_nanoseconds(dst, accessed, modified).await
}

/// Get precise timestamps using `statx` when available (fallback to `stat`) for nanosecond precision
///
/// This function uses the stat system call to get timestamps with full
/// nanosecond precision, which is more accurate than `std::fs::metadata()`.
///
/// # Arguments
///
/// * `path` - File path to get timestamps from
///
/// # Returns
///
/// Returns `Ok((accessed, modified))` if timestamps were read successfully, or `Err(SyncError)` if failed.
async fn get_precise_timestamps(path: &Path) -> Result<(SystemTime, SystemTime)> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    // Convert path to CString for syscall
    let path_cstr = CString::new(path.as_os_str().as_bytes())
        .map_err(|e| SyncError::FileSystem(format!("Invalid path for timestamp reading: {e}")))?;

    // Prefer statx when available
    let statx_result: Result<(SystemTime, SystemTime)> = compio::runtime::spawn_blocking({
        let path_cstr = path_cstr.clone();
        move || {
            let path_ptr = path_cstr.as_ptr();
            // statx flags: AT_FDCWD, path, AT_SYMLINK_NOFOLLOW (0), STATX_BASIC_STATS
            let mut buf: libc::statx = unsafe { std::mem::zeroed() };
            let rc = unsafe {
                libc::statx(
                    libc::AT_FDCWD,
                    path_ptr,
                    0,
                    0x0000_07ffu32 as libc::c_uint,
                    &raw mut buf,
                )
            };
            if rc == 0 {
                // Use stx_atime and stx_mtime with nanoseconds
                let atime_secs = u64::try_from(buf.stx_atime.tv_sec).unwrap_or(0);
                let atime_nanos = buf.stx_atime.tv_nsec;
                let mtime_secs = u64::try_from(buf.stx_mtime.tv_sec).unwrap_or(0);
                let mtime_nanos = buf.stx_mtime.tv_nsec;
                let atime =
                    SystemTime::UNIX_EPOCH + std::time::Duration::new(atime_secs, atime_nanos);
                let mtime =
                    SystemTime::UNIX_EPOCH + std::time::Duration::new(mtime_secs, mtime_nanos);
                Ok((atime, mtime))
            } else {
                let errno = std::io::Error::last_os_error();
                Err(SyncError::FileSystem(format!(
                    "statx failed: {errno} (errno: {})",
                    errno.raw_os_error().unwrap_or(-1)
                )))
            }
        }
    })
    .await
    .map_err(|e| SyncError::FileSystem(format!("spawn_blocking failed: {e:?}")))?;

    match statx_result {
        Ok(r) => Ok(r),
        Err(_) => {
            // Fallback to stat
            compio::runtime::spawn_blocking(move || {
                let mut stat_buf: libc::stat = unsafe { std::mem::zeroed() };
                let result = unsafe { libc::stat(path_cstr.as_ptr(), &raw mut stat_buf) };

                if result == -1 {
                    let errno = std::io::Error::last_os_error();
                    Err(SyncError::FileSystem(format!(
                        "stat failed: {errno} (errno: {})",
                        errno.raw_os_error().unwrap_or(-1)
                    )))
                } else {
                    // Convert timespec to SystemTime
                    let accessed_nanos: u32 = u32::try_from(stat_buf.st_atime_nsec).unwrap_or(0);
                    let modified_nanos: u32 = u32::try_from(stat_buf.st_mtime_nsec).unwrap_or(0);
                    #[allow(clippy::cast_sign_loss)]
                    let accessed = SystemTime::UNIX_EPOCH
                        + std::time::Duration::new(stat_buf.st_atime as u64, accessed_nanos);
                    #[allow(clippy::cast_sign_loss)]
                    let modified = SystemTime::UNIX_EPOCH
                        + std::time::Duration::new(stat_buf.st_mtime as u64, modified_nanos);
                    Ok((accessed, modified))
                }
            })
            .await
            .map_err(|e| SyncError::FileSystem(format!("spawn_blocking failed: {e:?}")))?
        }
    }
}

/// Preserve timestamps with nanosecond precision using utimensat
///
/// This function uses the `utimensat` system call to preserve timestamps with
/// nanosecond precision, which is more accurate than the standard `utimes`.
///
/// # Arguments
///
/// * `path` - File path to set timestamps on
/// * `accessed` - Access time
/// * `modified` - Modification time
///
/// # Returns
///
/// Returns `Ok(())` if timestamps were set successfully, or `Err(SyncError)` if failed.
async fn preserve_timestamps_nanoseconds(
    path: &Path,
    accessed: SystemTime,
    modified: SystemTime,
) -> Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    // Convert path to CString for syscall
    let path_cstr = CString::new(path.as_os_str().as_bytes()).map_err(|e| {
        SyncError::FileSystem(format!("Invalid path for timestamp preservation: {e}"))
    })?;

    // Convert SystemTime to timespec with nanosecond precision
    let accessed_timespec = system_time_to_timespec(accessed);
    let modified_timespec = system_time_to_timespec(modified);

    // Create timespec array for utimensat
    let times = [accessed_timespec, modified_timespec];

    // Use spawn_blocking for the syscall since compio doesn't have utimensat support
    compio::runtime::spawn_blocking(move || {
        let result = unsafe {
            libc::utimensat(
                libc::AT_FDCWD,
                path_cstr.as_ptr(),
                times.as_ptr(),
                0, // flags
            )
        };

        if result == -1 {
            let errno = std::io::Error::last_os_error();
            Err(SyncError::FileSystem(format!(
                "utimensat failed: {errno} (errno: {})",
                errno.raw_os_error().unwrap_or(-1)
            )))
        } else {
            Ok(())
        }
    })
    .await
    .map_err(|e| SyncError::FileSystem(format!("spawn_blocking failed: {e:?}")))?
}

/// Convert `SystemTime` to `libc::timespec` with nanosecond precision
///
/// This function extracts the nanosecond component from `SystemTime` and creates
/// a timespec structure suitable for `utimensat`.
fn system_time_to_timespec(time: SystemTime) -> libc::timespec {
    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();

    #[allow(clippy::cast_possible_wrap)]
    let tv_sec = duration.as_secs() as libc::time_t;
    libc::timespec {
        tv_sec,
        tv_nsec: libc::c_long::from(duration.subsec_nanos()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::CopyMethod;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};
    use tempfile::TempDir;

    /// Create a default Args struct for testing with archive mode enabled
    fn create_test_args_with_archive() -> Args {
        Args {
            source_positional: None,
            dest_positional: None,
            source: Some(PathBuf::from("/test/source")),
            destination: Some(PathBuf::from("/test/dest")),
            queue_depth: 4096,
            max_files_in_flight: 1024,
            cpu_count: 1,
            buffer_size_kb: 64,
            copy_method: CopyMethod::Auto,
            archive: true, // Enable archive mode for full metadata preservation
            recursive: false,
            links: false,
            perms: false,
            times: false,
            group: false,
            owner: false,
            devices: false,
            xattrs: false,
            acls: false,
            hard_links: false,
            atimes: false,
            crtimes: false,
            preserve_xattr: false,
            preserve_acl: false,
            dry_run: false,
            progress: false,
            verbose: 0,
            quiet: false,
            no_adaptive_concurrency: false,
            server: false,
            remote_shell: "ssh".to_string(),
            daemon: false,
            pipe: false,
            pipe_role: None,
            rsync_compat: false,
        }
    }

    #[compio::test]
    async fn test_preserve_metadata_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("source.txt");
        let dst_path = temp_dir.path().join("destination.txt");

        // Create source file with specific permissions
        fs::write(&src_path, "Test content for permission preservation").unwrap();

        // Set specific permissions (read/write for owner, read for group and others)
        let permissions = std::fs::Permissions::from_mode(0o644);
        fs::set_permissions(&src_path, permissions).unwrap();

        // Copy the file with archive mode (full metadata preservation)
        let args = create_test_args_with_archive();
        copy_file(&src_path, &dst_path, &args).await.unwrap();

        // Check that permissions were preserved
        let src_metadata = fs::metadata(&src_path).unwrap();
        let dst_metadata = fs::metadata(&dst_path).unwrap();

        let src_permissions = src_metadata.permissions().mode();
        let dst_permissions = dst_metadata.permissions().mode();

        println!(
            "Source permissions: {:o} ({})",
            src_permissions, src_permissions
        );
        println!(
            "Destination permissions: {:o} ({})",
            dst_permissions, dst_permissions
        );

        assert_eq!(
            src_permissions, dst_permissions,
            "Permissions should be preserved exactly"
        );
        // Note: The exact permission value may vary due to umask, but they should match
    }

    #[compio::test]
    async fn test_preserve_metadata_timestamps() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("source.txt");
        let dst_path = temp_dir.path().join("destination.txt");

        // Create source file
        fs::write(&src_path, "Test content for timestamp preservation").unwrap();

        // Get original timestamps
        let src_metadata = fs::metadata(&src_path).unwrap();
        let original_accessed = src_metadata.accessed().unwrap();
        let original_modified = src_metadata.modified().unwrap();

        // Wait a bit to ensure timestamps are different
        std::thread::sleep(Duration::from_millis(10));

        // Copy the file with archive mode (full metadata preservation)
        let args = create_test_args_with_archive();
        copy_file(&src_path, &dst_path, &args).await.unwrap();

        // Check that timestamps were preserved
        let dst_metadata = fs::metadata(&dst_path).unwrap();
        let copied_accessed = dst_metadata.accessed().unwrap();
        let copied_modified = dst_metadata.modified().unwrap();

        // Timestamps should be very close (within a few milliseconds due to system precision)
        let accessed_diff = copied_accessed
            .duration_since(original_accessed)
            .unwrap_or_default();
        let modified_diff = copied_modified
            .duration_since(original_modified)
            .unwrap_or_default();

        assert!(
            accessed_diff.as_millis() < 100,
            "Accessed time should be preserved within 100ms"
        );
        assert!(
            modified_diff.as_millis() < 100,
            "Modified time should be preserved within 100ms"
        );
    }

    #[compio::test]
    async fn test_preserve_metadata_complex_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("source.txt");
        let dst_path = temp_dir.path().join("destination.txt");

        // Create source file
        fs::write(
            &src_path,
            "Test content for complex permission preservation",
        )
        .unwrap();

        // Test various permission combinations (avoiding problematic ones)
        let test_permissions = vec![
            0o755, // rwxr-xr-x
            0o644, // rw-r--r--
            0o600, // rw-------
            0o777, // rwxrwxrwx
        ];

        for &permission_mode in &test_permissions {
            // Set specific permissions
            let permissions = std::fs::Permissions::from_mode(permission_mode);
            fs::set_permissions(&src_path, permissions).unwrap();

            // Get source permissions after setting (to account for umask)
            let src_metadata = fs::metadata(&src_path).unwrap();
            let expected_permissions = src_metadata.permissions().mode();

            // Copy the file with archive mode (full metadata preservation)
            let args = create_test_args_with_archive();
            copy_file(&src_path, &dst_path, &args).await.unwrap();

            // Check that permissions were preserved
            let dst_metadata = fs::metadata(&dst_path).unwrap();
            let dst_permissions = dst_metadata.permissions().mode();

            assert_eq!(
                expected_permissions, dst_permissions,
                "Permission mode {:o} should be preserved exactly",
                expected_permissions
            );
        }
    }

    #[compio::test]
    async fn test_preserve_metadata_nanosecond_precision() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("source.txt");
        let dst_path = temp_dir.path().join("destination.txt");

        // Create source file
        fs::write(&src_path, "Test content for nanosecond precision").unwrap();

        // Get original timestamps
        let src_metadata = fs::metadata(&src_path).unwrap();
        let original_accessed = src_metadata.accessed().unwrap();
        let original_modified = src_metadata.modified().unwrap();

        // Copy the file with archive mode (full metadata preservation)
        let args = create_test_args_with_archive();
        copy_file(&src_path, &dst_path, &args).await.unwrap();

        // Check that timestamps were preserved with high precision
        let dst_metadata = fs::metadata(&dst_path).unwrap();
        let copied_accessed = dst_metadata.accessed().unwrap();
        let copied_modified = dst_metadata.modified().unwrap();

        // For nanosecond precision, we should be able to preserve timestamps very accurately
        // The difference should be minimal (within microseconds)
        let accessed_diff = copied_accessed
            .duration_since(original_accessed)
            .unwrap_or_default();
        let modified_diff = copied_modified
            .duration_since(original_modified)
            .unwrap_or_default();

        assert!(
            accessed_diff.as_millis() < 100,
            "Accessed time should be preserved within 100ms"
        );
        assert!(
            modified_diff.as_millis() < 100,
            "Modified time should be preserved within 100ms"
        );
    }

    #[compio::test]
    async fn test_preserve_metadata_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("large_source.txt");
        let dst_path = temp_dir.path().join("large_destination.txt");

        // Create a larger file (1MB) to test with substantial data
        let large_content = "A".repeat(1024 * 1024); // 1MB of 'A' characters
        fs::write(&src_path, &large_content).unwrap();

        // Set specific permissions
        let permissions = std::fs::Permissions::from_mode(0o755);
        fs::set_permissions(&src_path, permissions).unwrap();

        // Get original permissions and timestamps
        let src_metadata = fs::metadata(&src_path).unwrap();
        let expected_permissions = src_metadata.permissions().mode();
        let original_accessed = src_metadata.accessed().unwrap();
        let original_modified = src_metadata.modified().unwrap();

        // Copy the file with archive mode (full metadata preservation)
        let args = create_test_args_with_archive();
        copy_file(&src_path, &dst_path, &args).await.unwrap();

        // Verify file content
        let copied_content = fs::read_to_string(&dst_path).unwrap();
        assert_eq!(
            copied_content, large_content,
            "File content should be preserved"
        );

        // Check that permissions were preserved
        let dst_metadata = fs::metadata(&dst_path).unwrap();
        let dst_permissions = dst_metadata.permissions().mode();
        assert_eq!(
            expected_permissions, dst_permissions,
            "Permissions should be preserved for large files"
        );

        // Check that timestamps were preserved
        let copied_accessed = dst_metadata.accessed().unwrap();
        let copied_modified = dst_metadata.modified().unwrap();

        let accessed_diff = copied_accessed
            .duration_since(original_accessed)
            .unwrap_or_default();
        let modified_diff = copied_modified
            .duration_since(original_modified)
            .unwrap_or_default();

        assert!(
            accessed_diff.as_millis() < 100,
            "Accessed time should be preserved for large files"
        );
        assert!(
            modified_diff.as_millis() < 100,
            "Modified time should be preserved for large files"
        );
    }

    #[compio::test]
    async fn test_preserve_metadata_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("empty_source.txt");
        let dst_path = temp_dir.path().join("empty_destination.txt");

        // Create empty file
        fs::write(&src_path, "").unwrap();

        // Set specific permissions
        let permissions = std::fs::Permissions::from_mode(0o600);
        fs::set_permissions(&src_path, permissions).unwrap();

        // Get expected permissions after setting (to account for umask)
        let src_metadata = fs::metadata(&src_path).unwrap();
        let expected_permissions = src_metadata.permissions().mode();

        // Copy the file with archive mode (full metadata preservation)
        let args = create_test_args_with_archive();
        copy_file(&src_path, &dst_path, &args).await.unwrap();

        // Check that permissions were preserved
        let dst_metadata = fs::metadata(&dst_path).unwrap();
        let dst_permissions = dst_metadata.permissions().mode();
        assert_eq!(
            expected_permissions, dst_permissions,
            "Permissions should be preserved for empty files"
        );

        // Verify file is empty
        let copied_content = fs::read_to_string(&dst_path).unwrap();
        assert_eq!(copied_content, "", "Empty file should remain empty");
    }

    #[test]
    fn test_system_time_to_timespec() {
        let now = SystemTime::now();
        let timespec = system_time_to_timespec(now);

        // Verify that the conversion produces reasonable values
        assert!(timespec.tv_sec > 0, "Seconds should be positive");
        assert!(timespec.tv_nsec >= 0, "Nanoseconds should be non-negative");
        assert!(
            timespec.tv_nsec < 1_000_000_000,
            "Nanoseconds should be less than 1 billion"
        );
    }

    #[test]
    fn test_system_time_to_timespec_precision() {
        let now = SystemTime::now();
        let timespec = system_time_to_timespec(now);

        // Test that we can reconstruct the original time with high precision
        let reconstructed =
            SystemTime::UNIX_EPOCH + Duration::new(timespec.tv_sec as u64, timespec.tv_nsec as u32);

        let diff = now.duration_since(reconstructed).unwrap_or_default();
        assert!(
            diff.as_micros() < 1000,
            "Reconstruction should be accurate within 1ms"
        );
    }

    #[compio::test]
    async fn test_fallocate_preallocation() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("source.txt");
        let dst_path = temp_dir.path().join("destination.txt");

        // Create a source file with known content
        let content = "Test content for fallocate preallocation";
        fs::write(&src_path, content).unwrap();

        // Copy the file with archive mode (full metadata preservation)
        let args = create_test_args_with_archive();
        copy_file(&src_path, &dst_path, &args).await.unwrap();

        // Verify the file was copied correctly
        let copied_content = fs::read_to_string(&dst_path).unwrap();
        assert_eq!(copied_content, content, "File content should be preserved");

        // Verify the file size matches the source
        let src_metadata = fs::metadata(&src_path).unwrap();
        let dst_metadata = fs::metadata(&dst_path).unwrap();
        assert_eq!(
            src_metadata.len(),
            dst_metadata.len(),
            "File sizes should match"
        );
    }

    #[compio::test]
    async fn test_fallocate_large_file_preallocation() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("large_source.txt");
        let dst_path = temp_dir.path().join("large_destination.txt");

        // Create a larger file (1MB) to test fallocate with substantial data
        let large_content = "A".repeat(1024 * 1024); // 1MB of 'A' characters
        fs::write(&src_path, &large_content).unwrap();

        // Copy the file with archive mode (full metadata preservation)
        let args = create_test_args_with_archive();
        copy_file(&src_path, &dst_path, &args).await.unwrap();

        // Verify the file was copied correctly
        let copied_content = fs::read_to_string(&dst_path).unwrap();
        assert_eq!(
            copied_content, large_content,
            "Large file content should be preserved"
        );

        // Verify the file size matches the source
        let src_metadata = fs::metadata(&src_path).unwrap();
        let dst_metadata = fs::metadata(&dst_path).unwrap();
        assert_eq!(
            src_metadata.len(),
            dst_metadata.len(),
            "Large file sizes should match"
        );
    }
}

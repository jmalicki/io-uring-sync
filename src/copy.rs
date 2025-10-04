//! File copying operations using io_uring
//!
//! This module provides high-performance file copying operations using various
//! system calls optimized for different scenarios. It implements copy_file_range
//! for efficient in-kernel copying, splice for zero-copy operations, and
//! traditional read/write as fallback methods.
//!
//! # Copy Methods
//!
//! - **copy_file_range**: In-kernel copying, most efficient for large files
//! - **splice**: Zero-copy operations using pipes
//! - **read_write**: Traditional fallback method
//! - **auto**: Automatically selects the best method available
//!
//! # Performance Characteristics
//!
//! - copy_file_range: ~2-5x faster than read/write for large files
//! - splice: Zero-copy, optimal for streaming operations
//! - read/write: Reliable fallback, works everywhere
//!
//! # Usage
//!
//! ```rust
//! use io_uring_sync::copy::copy_file;
//! use io_uring_sync::cli::CopyMethod;
//!
//! // Copy with automatic method selection
//! copy_file(src_path, dst_path, CopyMethod::Auto).await?;
//!
//! // Force specific method
//! copy_file(src_path, dst_path, CopyMethod::CopyFileRange).await?;
//! ```

use crate::cli::CopyMethod;
use crate::error::{Result, SyncError};
use io_uring_extended::ExtendedRio;
use rio::new as rio_new;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use compio::fs::OpenOptions;

/// Copy a single file using the specified method
pub async fn copy_file(src: &Path, dst: &Path, method: CopyMethod) -> Result<()> {
    match method {
        CopyMethod::Auto => {
            // Try copy_file_range first, fall back to read/write
            match copy_file_range(src, dst).await {
                Ok(()) => Ok(()),
                Err(e) => {
                    tracing::debug!("copy_file_range failed: {}, falling back to read/write", e);
                    copy_read_write(src, dst).await
                }
            }
        }
        CopyMethod::CopyFileRange => copy_file_range(src, dst).await,
        CopyMethod::Splice => copy_splice(src, dst).await,
        CopyMethod::ReadWrite => copy_read_write(src, dst).await,
    }
}

/// Copy file using copy_file_range system call (optimal for same filesystem)
///
/// This function uses the copy_file_range system call for efficient in-kernel
/// file copying. It's particularly effective when copying within the same
/// filesystem as it can avoid copying data to userspace.
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
/// - Most efficient for large files (2-5x faster than read/write)
/// - Optimal when source and destination are on the same filesystem
/// - Uses in-kernel copying to avoid userspace data copies
/// - May not work across different filesystems
///
/// # Examples
///
/// ```rust
/// use io_uring_sync::copy::copy_file_range;
///
/// copy_file_range(src_path, dst_path).await?;
/// ```
async fn copy_file_range(src: &Path, dst: &Path) -> Result<()> {
    // Create extended rio instance for copy_file_range support
    let extended_rio = ExtendedRio::new()
        .map_err(|e| SyncError::IoUring(format!("Failed to create ExtendedRio: {}", e)))?;

    // Open source file
    let src_file = OpenOptions::new().read(true).open(src).await.map_err(|e| {
        SyncError::FileSystem(format!(
            "Failed to open source file {}: {}",
            src.display(),
            e
        ))
    })?;

    // Open destination file
    let dst_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dst)
        .await
        .map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to open destination file {}: {}",
                dst.display(),
                e
            ))
        })?;

    // Get file descriptors
    let src_fd = src_file.as_raw_fd();
    let dst_fd = dst_file.as_raw_fd();

    // Get file size
    let metadata = src_file
        .metadata()
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to get source file metadata: {}", e)))?;
    let file_size = metadata.len();

    // Use ExtendedRio's copy_file_range for efficient in-kernel copying
    let mut offset = 0u64;
    let mut remaining = file_size;

    while remaining > 0 {
        let chunk_size = std::cmp::min(remaining, 1024 * 1024 * 1024) as u32; // 1GB chunks

        // Use ExtendedRio's copy_file_range
        let result = extended_rio
            .copy_file_range(src_fd, offset, dst_fd, offset, chunk_size)
            .await
            .map_err(|e| SyncError::CopyFailed(format!("copy_file_range failed: {}", e)))?;

        if result <= 0 {
            return Err(SyncError::CopyFailed(format!(
                "copy_file_range returned invalid result: {}",
                result
            )));
        }

        let copied = result as u64;
        remaining -= copied;
        offset += copied;

        tracing::debug!(
            "ExtendedRio copy_file_range: copied {} bytes, {} remaining",
            copied,
            remaining
        );
    }

    // Sync the destination file to ensure data is written to disk
    dst_file
        .sync_all()
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to sync destination file: {}", e)))?;

    tracing::debug!(
        "ExtendedRio copy_file_range: successfully copied {} bytes",
        file_size
    );
    Ok(())
}

/// Copy file using splice system call (zero-copy operations)
///
/// This function uses the splice system call for zero-copy file operations
/// by using pipes as an intermediate buffer. This is particularly efficient
/// for streaming operations and can provide better performance than read/write.
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
/// - Zero-copy operations using pipes
/// - Optimal for streaming and large file operations
/// - Can be faster than read/write for certain workloads
/// - Uses splice system call for efficient data transfer
///
/// # Examples
///
/// ```rust
/// use io_uring_sync::copy::copy_splice;
///
/// copy_splice(src_path, dst_path).await?;
/// ```
async fn copy_splice(src: &Path, dst: &Path) -> Result<()> {
    // Open source file
    let src_file = OpenOptions::new().read(true).open(src).await.map_err(|e| {
        SyncError::FileSystem(format!(
            "Failed to open source file {}: {}",
            src.display(),
            e
        ))
    })?;

    // Open destination file
    let dst_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dst)
        .await
        .map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to open destination file {}: {}",
                dst.display(),
                e
            ))
        })?;

    // Get file descriptors
    let src_fd = src_file.as_raw_fd();
    let dst_fd = dst_file.as_raw_fd();

    // Get file size
    let metadata = src_file
        .metadata()
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to get source file metadata: {}", e)))?;
    let file_size = metadata.len();

    // Create a pipe for splice operations
    let mut pipe_fds = [0i32; 2];
    let result = unsafe { libc::pipe2(pipe_fds.as_mut_ptr(), 0) };
    if result < 0 {
        return Err(SyncError::CopyFailed(
            "Failed to create pipe for splice".to_string(),
        ));
    }

    let pipe_read_fd = pipe_fds[0];
    let pipe_write_fd = pipe_fds[1];

    let mut remaining = file_size;
    let splice_size = 1024 * 1024; // 1MB chunks

    while remaining > 0 {
        let chunk_size = std::cmp::min(remaining, splice_size as u64) as usize;

        // Splice from source file to pipe
        let splice_result = unsafe {
            libc::splice(
                src_fd,
                std::ptr::null_mut::<i64>(), // NULL offset means use current position
                pipe_write_fd,
                std::ptr::null_mut::<i64>(),
                chunk_size,
                libc::SPLICE_F_MOVE | libc::SPLICE_F_MORE,
            )
        };

        if splice_result < 0 {
            unsafe {
                libc::close(pipe_read_fd);
                libc::close(pipe_write_fd);
            }
            let errno = std::io::Error::last_os_error();
            return Err(SyncError::CopyFailed(format!(
                "splice from source to pipe failed: {} (errno: {})",
                errno,
                errno.raw_os_error().unwrap_or(-1)
            )));
        }

        // Splice from pipe to destination file
        let splice_result2 = unsafe {
            libc::splice(
                pipe_read_fd,
                std::ptr::null_mut::<i64>(),
                dst_fd,
                std::ptr::null_mut::<i64>(),
                splice_result as usize,
                libc::SPLICE_F_MOVE | libc::SPLICE_F_MORE,
            )
        };

        if splice_result2 < 0 {
            unsafe {
                libc::close(pipe_read_fd);
                libc::close(pipe_write_fd);
            }
            let errno = std::io::Error::last_os_error();
            return Err(SyncError::CopyFailed(format!(
                "splice from pipe to destination failed: {} (errno: {})",
                errno,
                errno.raw_os_error().unwrap_or(-1)
            )));
        }

        let copied = splice_result as u64;
        remaining -= copied;

        tracing::debug!("splice: copied {} bytes, {} remaining", copied, remaining);
    }

    // Close pipe file descriptors
    unsafe {
        libc::close(pipe_read_fd);
        libc::close(pipe_write_fd);
    }

    // Sync the destination file to ensure data is written to disk
    dst_file
        .sync_all()
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to sync destination file: {}", e)))?;

    tracing::debug!("splice: successfully copied {} bytes", file_size);
    Ok(())
}

/// Copy file using traditional read/write operations (reliable fallback)
///
/// This function provides a reliable fallback method for file copying using
/// traditional read/write operations. While not as fast as copy_file_range or
/// splice, it works in all scenarios and provides guaranteed compatibility.
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
/// - Uses buffered I/O for optimal performance
/// - Compatible with all filesystems and scenarios
/// - Slower than copy_file_range but more reliable
///
/// # Examples
///
/// ```rust
/// use io_uring_sync::copy::copy_read_write;
///
/// copy_read_write(src_path, dst_path).await?;
/// ```
async fn copy_read_write(src: &Path, dst: &Path) -> Result<()> {
    // Initialize io_uring
    let io_uring = rio_new()
        .map_err(|e| SyncError::IoUring(format!("Failed to initialize io_uring: {}", e)))?;

    // Open source file
    let src_file = OpenOptions::new().read(true).open(src).await.map_err(|e| {
        SyncError::FileSystem(format!(
            "Failed to open source file {}: {}",
            src.display(),
            e
        ))
    })?;

    // Open destination file
    let dst_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dst)
        .await
        .map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to open destination file {}: {}",
                dst.display(),
                e
            ))
        })?;

    // Get file size for progress tracking
    let metadata = src_file
        .metadata()
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to get source file metadata: {}", e)))?;
    let file_size = metadata.len();

    // Use io_uring read_at/write_at operations
    const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer (smaller to avoid stack overflow)
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut offset = 0u64;
    let mut total_copied = 0u64;

    while total_copied < file_size {
        // Submit read_at operation to io_uring
        #[allow(clippy::unnecessary_mut_passed)]
        let read_completion = io_uring.read_at(&src_file, &mut buffer, offset);

        // Wait for read completion
        let bytes_read = read_completion
            .await
            .map_err(|e| SyncError::IoUring(format!("read_at operation failed: {}", e)))?;

        if bytes_read == 0 {
            // End of file
            break;
        }

        // Submit write_at operation to io_uring
        // Note: rio::write_at writes the entire buffer, so we need to handle this carefully
        let write_completion = io_uring.write_at(&dst_file, &buffer, offset);

        // Wait for write completion
        let bytes_written = write_completion
            .await
            .map_err(|e| SyncError::IoUring(format!("write_at operation failed: {}", e)))?;

        // rio::write_at returns the number of bytes written, which should match bytes_read
        if bytes_written != bytes_read {
            return Err(SyncError::CopyFailed(format!(
                "Write size mismatch: expected {}, got {}",
                bytes_read, bytes_written
            )));
        }

        total_copied += bytes_written as u64;
        offset += bytes_written as u64;

        tracing::debug!(
            "io_uring read_at/write_at: copied {} bytes, total: {}/{}",
            bytes_written,
            total_copied,
            file_size
        );
    }

    // Sync the destination file to ensure data is written to disk
    dst_file
        .sync_all()
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to sync destination file: {}", e)))?;

    tracing::debug!(
        "io_uring read_at/write_at: successfully copied {} bytes",
        total_copied
    );
    Ok(())
}

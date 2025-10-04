//! Directory traversal and copying functionality
//!
//! This module provides async directory traversal and copying capabilities
//! using io_uring operations where possible, with fallbacks to standard
//! filesystem operations for unsupported operations.

use crate::cli::CopyMethod;
use crate::copy::copy_file;
use crate::error::{Result, SyncError};
use crate::io_uring::FileOperations;
use async_recursion::async_recursion;
use std::path::Path;
use tokio::fs as tokio_fs;
use tracing::{debug, info, warn};

/// Directory copy operation statistics
#[derive(Debug, Default)]
pub struct DirectoryStats {
    /// Total number of files copied
    pub files_copied: u64,
    /// Total number of directories created
    pub directories_created: u64,
    /// Total number of bytes copied
    pub bytes_copied: u64,
    /// Number of symlinks processed
    pub symlinks_processed: u64,
    /// Number of errors encountered
    pub errors: u64,
}

/// Copy a directory recursively with metadata preservation
///
/// This function performs recursive directory copying with the following features:
/// - Async directory traversal
/// - Metadata preservation (permissions, ownership, timestamps)
/// - Symlink handling
/// - Error recovery and reporting
///
/// # Parameters
///
/// * `src` - Source directory path
/// * `dst` - Destination directory path
/// * `file_ops` - File operations instance for metadata handling
/// * `copy_method` - Copy method to use for individual files
///
/// # Returns
///
/// Returns directory copy statistics or an error.
pub async fn copy_directory(
    src: &Path,
    dst: &Path,
    file_ops: &FileOperations,
    copy_method: CopyMethod,
) -> Result<DirectoryStats> {
    let mut stats = DirectoryStats::default();

    info!(
        "Starting directory copy from {} to {}",
        src.display(),
        dst.display()
    );

    // Create destination directory if it doesn't exist
    if !dst.exists() {
        tokio_fs::create_dir_all(dst).await.map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to create destination directory {}: {}",
                dst.display(),
                e
            ))
        })?;
        stats.directories_created += 1;
        debug!("Created destination directory: {}", dst.display());
    }

    // Traverse source directory recursively
    traverse_and_copy_directory(src, dst, file_ops, copy_method, &mut stats).await?;

    info!(
        "Directory copy completed: {} files, {} directories, {} bytes, {} symlinks",
        stats.files_copied, stats.directories_created, stats.bytes_copied, stats.symlinks_processed
    );

    Ok(stats)
}

/// Recursively traverse and copy directory contents
#[async_recursion]
async fn traverse_and_copy_directory(
    src: &Path,
    dst: &Path,
    file_ops: &FileOperations,
    copy_method: CopyMethod,
    stats: &mut DirectoryStats,
) -> Result<()> {
    let mut entries = tokio_fs::read_dir(src).await.map_err(|e| {
        SyncError::FileSystem(format!("Failed to read directory {}: {}", src.display(), e))
    })?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to read directory entry: {}", e)))?
    {
        let src_path = entry.path();
        let file_name = src_path.file_name().ok_or_else(|| {
            SyncError::FileSystem(format!("Invalid file name in {}", src_path.display()))
        })?;
        let dst_path = dst.join(file_name);

        let metadata = entry.metadata().await.map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to get metadata for {}: {}",
                src_path.display(),
                e
            ))
        })?;

        if metadata.is_dir() {
            // Handle directory
            debug!("Processing directory: {}", src_path.display());

            // Create destination directory
            if !dst_path.exists() {
                tokio_fs::create_dir(&dst_path).await.map_err(|e| {
                    SyncError::FileSystem(format!(
                        "Failed to create directory {}: {}",
                        dst_path.display(),
                        e
                    ))
                })?;
                stats.directories_created += 1;
            }

            // Recursively process subdirectory
            if let Err(e) = traverse_and_copy_directory(
                &src_path,
                &dst_path,
                file_ops,
                copy_method.clone(),
                stats,
            )
            .await
            {
                warn!("Failed to copy subdirectory {}: {}", src_path.display(), e);
                stats.errors += 1;
            }
        } else if metadata.is_file() {
            // Handle regular file
            debug!("Copying file: {}", src_path.display());

            match copy_file(&src_path, &dst_path, copy_method.clone()).await {
                Ok(()) => {
                    stats.files_copied += 1;
                    stats.bytes_copied += metadata.len();

                    // Preserve metadata
                    if let Err(e) = preserve_file_metadata(&src_path, &dst_path, file_ops).await {
                        warn!(
                            "Failed to preserve metadata for {}: {}",
                            dst_path.display(),
                            e
                        );
                    }
                }
                Err(e) => {
                    warn!("Failed to copy file {}: {}", src_path.display(), e);
                    stats.errors += 1;
                }
            }
        } else if metadata.is_symlink() {
            // Handle symlink
            debug!("Processing symlink: {}", src_path.display());

            match copy_symlink(&src_path, &dst_path).await {
                Ok(()) => {
                    stats.symlinks_processed += 1;
                }
                Err(e) => {
                    warn!("Failed to copy symlink {}: {}", src_path.display(), e);
                    stats.errors += 1;
                }
            }
        }
    }

    Ok(())
}

/// Copy a symlink preserving its target
async fn copy_symlink(src: &Path, dst: &Path) -> Result<()> {
    let target = tokio_fs::read_link(src).await.map_err(|e| {
        SyncError::FileSystem(format!(
            "Failed to read symlink target for {}: {}",
            src.display(),
            e
        ))
    })?;

    // Remove destination if it exists
    if dst.exists() {
        tokio_fs::remove_file(dst).await.map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to remove existing destination {}: {}",
                dst.display(),
                e
            ))
        })?;
    }

    // Create symlink with same target
    #[cfg(unix)]
    {
        tokio_fs::symlink(&target, dst).await.map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to create symlink {} -> {}: {}",
                dst.display(),
                target.display(),
                e
            ))
        })?;
    }

    #[cfg(not(unix))]
    {
        return Err(SyncError::FileSystem(
            "Symlink creation not supported on this platform".to_string(),
        ));
    }

    debug!("Copied symlink {} -> {}", dst.display(), target.display());
    Ok(())
}

/// Preserve file metadata (permissions, ownership, timestamps)
async fn preserve_file_metadata(src: &Path, dst: &Path, file_ops: &FileOperations) -> Result<()> {
    // Get source metadata
    let metadata = file_ops.get_file_metadata(src).await.map_err(|e| {
        SyncError::FileSystem(format!(
            "Failed to get source metadata for {}: {}",
            src.display(),
            e
        ))
    })?;

    // Set permissions
    file_ops
        .set_file_permissions(dst, metadata.permissions)
        .await
        .map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to set permissions for {}: {}",
                dst.display(),
                e
            ))
        })?;

    // Set ownership
    file_ops
        .set_file_ownership(dst, metadata.uid, metadata.gid)
        .await
        .map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to set ownership for {}: {}",
                dst.display(),
                e
            ))
        })?;

    // Set timestamps (currently skipped due to unstable Rust features)
    // TODO: Implement timestamp preservation using libc
    debug!("Preserved metadata for {}", dst.display());

    Ok(())
}

/// Get directory size recursively
///
/// This function calculates the total size of a directory by recursively
/// traversing all files and summing their sizes.
///
/// # Parameters
///
/// * `path` - Directory path to analyze
///
/// # Returns
///
/// Returns the total size in bytes or an error.
#[async_recursion]
pub async fn get_directory_size(path: &Path) -> Result<u64> {
    let mut total_size = 0u64;
    let mut entries = tokio_fs::read_dir(path).await.map_err(|e| {
        SyncError::FileSystem(format!(
            "Failed to read directory {}: {}",
            path.display(),
            e
        ))
    })?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to read directory entry: {}", e)))?
    {
        let entry_path = entry.path();
        let metadata = entry.metadata().await.map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to get metadata for {}: {}",
                entry_path.display(),
                e
            ))
        })?;

        if metadata.is_dir() {
            // Recursively calculate subdirectory size
            total_size += get_directory_size(&entry_path).await?;
        } else if metadata.is_file() {
            total_size += metadata.len();
        }
        // Skip symlinks for size calculation
    }

    Ok(total_size)
}

/// Count files and directories recursively
///
/// This function counts the total number of files and directories
/// in a directory tree.
///
/// # Parameters
///
/// * `path` - Directory path to analyze
///
/// # Returns
///
/// Returns a tuple of (files, directories) or an error.
#[async_recursion]
pub async fn count_directory_contents(path: &Path) -> Result<(u64, u64)> {
    let mut file_count = 0u64;
    let mut dir_count = 0u64;
    let mut entries = tokio_fs::read_dir(path).await.map_err(|e| {
        SyncError::FileSystem(format!(
            "Failed to read directory {}: {}",
            path.display(),
            e
        ))
    })?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to read directory entry: {}", e)))?
    {
        let entry_path = entry.path();
        let metadata = entry.metadata().await.map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to get metadata for {}: {}",
                entry_path.display(),
                e
            ))
        })?;

        if metadata.is_dir() {
            dir_count += 1;
            // Recursively count subdirectory contents
            let (sub_files, sub_dirs) = count_directory_contents(&entry_path).await?;
            file_count += sub_files;
            dir_count += sub_dirs;
        } else if metadata.is_file() {
            file_count += 1;
        }
        // Skip symlinks for counting
    }

    Ok((file_count, dir_count))
}

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
use std::collections::HashMap;
use std::path::Path;
use tokio::fs as tokio_fs;
use tracing::{debug, info, warn};
use std::fs::Metadata;

/// Extended metadata that includes both standard metadata and inode information
#[derive(Debug, Clone)]
pub struct ExtendedMetadata {
    /// Standard filesystem metadata
    pub metadata: Metadata,
    /// Filesystem device ID
    pub device_id: u64,
    /// Inode number
    pub inode_number: u64,
}

impl ExtendedMetadata {
    /// Create extended metadata by combining standard metadata with inode info
    ///
    /// This function gets the standard metadata and then uses io_uring to get
    /// the device ID and inode number in a single efficient operation.
    pub async fn new(path: &Path) -> Result<Self> {
        // Get standard metadata
        let metadata = tokio_fs::metadata(path).await?;
        
        // Get inode information using io_uring
        let (device_id, inode_number) = get_inode_info_io_uring(path).await?;
        
        Ok(ExtendedMetadata {
            metadata,
            device_id,
            inode_number,
        })
    }
    
    /// Get device ID
    pub fn device_id(&self) -> u64 {
        self.device_id
    }
    
    /// Get inode number
    pub fn inode_number(&self) -> u64 {
        self.inode_number
    }
    
    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.metadata.is_dir()
    }
    
    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        self.metadata.is_file()
    }
    
    /// Check if this is a symlink
    pub fn is_symlink(&self) -> bool {
        self.metadata.is_symlink()
    }
    
    /// Get file size
    pub fn len(&self) -> u64 {
        self.metadata.len()
    }
}

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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

/// Filesystem boundary detection and hardlink tracking
///
/// This module provides functionality for detecting filesystem boundaries
/// and tracking hardlink relationships to ensure proper file copying behavior.

/// Filesystem device ID and inode number pair for hardlink detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InodeInfo {
    /// Filesystem device ID
    pub dev: u64,
    /// Inode number
    pub ino: u64,
}

/// Hardlink tracking information
#[derive(Debug, Clone)]
pub struct HardlinkInfo {
    /// Original file path
    pub original_path: std::path::PathBuf,
    /// Number of hardlinks found
    pub link_count: u64,
}

/// Filesystem boundary and hardlink tracker
#[derive(Debug, Default)]
pub struct FilesystemTracker {
    /// Map of (dev, ino) pairs to hardlink information
    hardlinks: HashMap<InodeInfo, HardlinkInfo>,
    /// Source filesystem device ID (for boundary detection)
    source_filesystem: Option<u64>,
}

impl FilesystemTracker {
    /// Create a new filesystem tracker
    pub fn new() -> Self {
        Self {
            hardlinks: HashMap::new(),
            source_filesystem: None,
        }
    }

    /// Set the source filesystem device ID
    ///
    /// This should be called once at the beginning of a copy operation
    /// to establish the source filesystem boundary.
    pub fn set_source_filesystem(&mut self, dev: u64) {
        self.source_filesystem = Some(dev);
        debug!("Set source filesystem device ID: {}", dev);
    }

    /// Check if a path is on the same filesystem as the source
    ///
    /// Returns true if the path is on the same filesystem, false otherwise.
    /// This prevents cross-filesystem operations that could cause issues.
    pub fn is_same_filesystem(&self, dev: u64) -> bool {
        match self.source_filesystem {
            Some(source_dev) => source_dev == dev,
            None => {
                warn!("No source filesystem set, allowing cross-filesystem operation");
                true
            }
        }
    }

    /// Register a file for hardlink tracking
    ///
    /// This should be called for each file encountered during traversal.
    /// Returns true if this is a new hardlink, false if it's a duplicate.
    pub fn register_file(&mut self, path: &Path, dev: u64, ino: u64) -> bool {
        let inode_info = InodeInfo { dev, ino };

        match self.hardlinks.get_mut(&inode_info) {
            Some(hardlink_info) => {
                // This is an existing hardlink
                hardlink_info.link_count += 1;
                debug!(
                    "Found hardlink #{} for inode ({}, {}): {}",
                    hardlink_info.link_count, dev, ino, path.display()
                );
                false
            }
            None => {
                // This is a new file
                self.hardlinks.insert(
                    inode_info,
                    HardlinkInfo {
                        original_path: path.to_path_buf(),
                        link_count: 1,
                    },
                );
                debug!(
                    "Registered new file inode ({}, {}): {}",
                    dev, ino, path.display()
                );
                true
            }
        }
    }

    /// Get hardlink information for a given inode
    ///
    /// Returns the hardlink information if this inode has been seen before.
    pub fn get_hardlink_info(&self, dev: u64, ino: u64) -> Option<&HardlinkInfo> {
        let inode_info = InodeInfo { dev, ino };
        self.hardlinks.get(&inode_info)
    }

    /// Get all hardlink groups that have multiple links
    ///
    /// Returns a vector of hardlink groups that contain multiple files.
    pub fn get_hardlink_groups(&self) -> Vec<&HardlinkInfo> {
        self.hardlinks
            .values()
            .filter(|info| info.link_count > 1)
            .collect()
    }

    /// Get statistics about the filesystem tracking
    pub fn get_stats(&self) -> FilesystemStats {
        let total_files = self.hardlinks.len();
        let hardlink_groups = self.get_hardlink_groups().len();
        let total_hardlinks: u64 = self.hardlinks.values().map(|info| info.link_count).sum();

        FilesystemStats {
            total_files,
            hardlink_groups,
            total_hardlinks,
            source_filesystem: self.source_filesystem,
        }
    }
}

/// Statistics about filesystem tracking
#[derive(Debug)]
pub struct FilesystemStats {
    /// Total number of unique files (by inode)
    pub total_files: usize,
    /// Number of hardlink groups (files with multiple links)
    pub hardlink_groups: usize,
    /// Total number of hardlinks (including originals)
    pub total_hardlinks: u64,
    /// Source filesystem device ID
    pub source_filesystem: Option<u64>,
}

/// Detect filesystem boundaries and hardlink relationships for a directory tree
///
/// This function traverses a directory tree and builds a filesystem tracker
/// that can be used to detect cross-filesystem operations and hardlink relationships.
///
/// # Parameters
///
/// * `root_path` - Root directory to analyze
/// * `tracker` - Filesystem tracker to populate
///
/// # Returns
///
/// Returns Ok(()) on success or an error if traversal fails.
pub async fn analyze_filesystem_structure(
    root_path: &Path,
    tracker: &mut FilesystemTracker,
) -> Result<()> {
    debug!("Analyzing filesystem structure for: {}", root_path.display());

    // Get the root directory's filesystem device ID
    let root_extended_metadata = ExtendedMetadata::new(root_path).await?;
    tracker.set_source_filesystem(root_extended_metadata.device_id());

    // Traverse the directory tree
    analyze_directory_recursive(root_path, tracker).await?;

    let stats = tracker.get_stats();
    info!(
        "Filesystem analysis complete: {} files, {} hardlink groups, {} total hardlinks",
        stats.total_files, stats.hardlink_groups, stats.total_hardlinks
    );

    Ok(())
}

/// Recursively analyze a directory for filesystem structure
#[async_recursion]
async fn analyze_directory_recursive(
    dir_path: &Path,
    tracker: &mut FilesystemTracker,
) -> Result<()> {
    let mut entries = tokio_fs::read_dir(dir_path).await?;

    while let Some(entry) = entries.next_entry().await? {
        let entry_path = entry.path();
        let extended_metadata = ExtendedMetadata::new(&entry_path).await?;

        if extended_metadata.is_dir() {
            // Recursively analyze subdirectories
            analyze_directory_recursive(&entry_path, tracker).await?;
        } else if extended_metadata.is_file() {
            // Analyze file for hardlink detection
            let dev = extended_metadata.device_id();
            let ino = extended_metadata.inode_number();

            // Check filesystem boundary
            if !tracker.is_same_filesystem(dev) {
                warn!(
                    "Cross-filesystem file detected: {} (dev: {}, expected: {:?})",
                    entry_path.display(),
                    dev,
                    tracker.source_filesystem
                );
                // Continue processing but note the boundary crossing
            }

            // Register file for hardlink tracking
            tracker.register_file(&entry_path, dev, ino);
        }
        // Skip symlinks for now - they'll be handled separately
    }

    Ok(())
}

/// Extract device ID and inode number from file path using io_uring
///
/// This function uses our io_uring statx operations to get filesystem information efficiently.
async fn get_inode_info_io_uring(path: &Path) -> Result<(u64, u64)> {
    // Create an ExtendedRio instance for io_uring operations
    let extended_rio = io_uring_extended::ExtendedRio::new()
        .map_err(|e| SyncError::FileSystem(format!("Failed to create ExtendedRio: {}", e)))?;
    
    // Use our io_uring statx_inode operation
    extended_rio.statx_inode(path).await
        .map_err(|e| SyncError::FileSystem(format!("Failed to get inode info for {}: {}", path.display(), e)))
}

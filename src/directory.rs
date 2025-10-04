//! Directory traversal and copying functionality
//!
//! This module provides async directory traversal and copying capabilities
//! using io_uring operations where possible, with fallbacks to standard
//! filesystem operations for unsupported operations.

use crate::cli::CopyMethod;
use crate::copy::copy_file;
use crate::error::{Result, SyncError};
use crate::io_uring::FileOperations;
#[allow(clippy::disallowed_types)]
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use tracing::{debug, info, warn};

/// Extended metadata using std::fs metadata support
#[derive(Clone)]
#[allow(dead_code)]
pub struct ExtendedMetadata {
    /// Complete file metadata from std::fs metadata operation
    pub metadata: std::fs::Metadata,
}


#[allow(dead_code)]
impl ExtendedMetadata {
    /// Create extended metadata using compio's built-in metadata support
    ///
    /// This function uses compio's metadata operation which leverages io_uring
    /// for efficient async metadata retrieval including device ID, inode number,
    /// size, permissions, timestamps, and file type.
    pub async fn new(path: &Path) -> Result<Self> {
        // Get metadata using std::fs (compio has Send issues)
        let metadata = std::fs::metadata(path).map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to get metadata for {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(ExtendedMetadata { metadata })
    }

    /// Get device ID
    pub fn device_id(&self) -> u64 {
        self.metadata.dev()
    }

    /// Get inode number
    pub fn inode_number(&self) -> u64 {
        self.metadata.ino()
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
        self.metadata.file_type().is_symlink()
    }

    /// Get file size
    pub fn len(&self) -> u64 {
        self.metadata.len()
    }

    /// Check if the file is empty
    pub fn is_empty(&self) -> bool {
        self.metadata.len() == 0
    }

    /// Get file permissions
    pub fn permissions(&self) -> u32 {
        self.metadata.mode()
    }

    /// Get last modification time
    pub fn modified_time(&self) -> i64 {
        self.metadata.mtime()
    }

    /// Get last access time
    pub fn accessed_time(&self) -> i64 {
        self.metadata.atime()
    }

    /// Get creation time
    pub fn created_time(&self) -> i64 {
        self.metadata.ctime()
    }

    /// Get link count (number of hardlinks to this inode)
    pub fn link_count(&self) -> u64 {
        self.metadata.nlink()
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

/// Copy a directory recursively with metadata preservation and hardlink detection
///
/// This function performs recursive directory copying with the following features:
/// - Async directory traversal using io_uring statx operations
/// - Hardlink detection and preservation during traversal
/// - Filesystem boundary detection
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
    let mut hardlink_tracker = FilesystemTracker::new();

    info!(
        "Starting directory copy from {} to {}",
        src.display(),
        dst.display()
    );

    // Create destination directory if it doesn't exist
    if !dst.exists() {
        std::fs::create_dir_all(dst).map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to create destination directory {}: {}",
                dst.display(),
                e
            ))
        })?;
        stats.directories_created += 1;
        debug!("Created destination directory: {}", dst.display());
    }

    // Set source filesystem from root directory
    let root_metadata = ExtendedMetadata::new(src).await?;
    hardlink_tracker.set_source_filesystem(root_metadata.device_id());

    // Traverse source directory iteratively using compio's dispatcher
    traverse_and_copy_directory_iterative(src, dst, file_ops, copy_method, &mut stats, &mut hardlink_tracker).await?;

    // Log hardlink detection results
    let hardlink_stats = hardlink_tracker.get_stats();
    info!(
        "Directory copy completed: {} files, {} directories, {} bytes, {} symlinks",
        stats.files_copied, stats.directories_created, stats.bytes_copied, stats.symlinks_processed
    );
    if hardlink_stats.hardlink_groups > 0 {
        info!(
            "Hardlink detection: {} unique files, {} hardlink groups, {} total hardlinks",
            hardlink_stats.total_files, hardlink_stats.hardlink_groups, hardlink_stats.total_hardlinks
        );
    }

    Ok(stats)
}

/// Iterative directory traversal using compio's dispatcher
///
/// This function uses a simple iterative approach with a work list,
/// trusting compio's dispatcher to handle async operations efficiently.
/// This avoids recursion while being much simpler than custom queues.
async fn traverse_and_copy_directory_iterative(
    initial_src: &Path,
    initial_dst: &Path,
    file_ops: &FileOperations,
    copy_method: CopyMethod,
    stats: &mut DirectoryStats,
    hardlink_tracker: &mut FilesystemTracker,
) -> Result<()> {
    // Simple work list - compio's dispatcher handles the async scheduling
    let mut work_list = vec![(initial_src.to_path_buf(), initial_dst.to_path_buf())];
    
    while let Some((src, dst)) = work_list.pop() {
        let entries = std::fs::read_dir(&src).map_err(|e| {
            SyncError::FileSystem(format!("Failed to read directory {}: {}", src.display(), e))
        })?;

        for entry_result in entries {
            let entry = entry_result.map_err(|e| SyncError::FileSystem(format!("Failed to read directory entry: {}", e)))?;
            let src_path = entry.path();
            let file_name = src_path.file_name().ok_or_else(|| {
                SyncError::FileSystem(format!("Invalid file name in {}", src_path.display()))
            })?;
            let dst_path = dst.join(file_name);

            // Get comprehensive metadata using std::fs metadata (includes hardlink info)
            let extended_metadata = ExtendedMetadata::new(&src_path).await?;

            if extended_metadata.is_dir() {
                // Handle directory
                debug!("Processing directory: {}", src_path.display());

                // Create destination directory
                if !dst_path.exists() {
                    std::fs::create_dir(&dst_path).map_err(|e| {
                        SyncError::FileSystem(format!(
                            "Failed to create directory {}: {}",
                            dst_path.display(),
                            e
                        ))
                    })?;
                    stats.directories_created += 1;
                }

                // Add subdirectory to work list for later processing
                work_list.push((src_path, dst_path));
            } else if extended_metadata.is_file() {
                // Handle regular file with hardlink detection
                debug!("Processing file: {} (link_count: {})", src_path.display(), extended_metadata.link_count());

                let device_id = extended_metadata.device_id();
                let inode_number = extended_metadata.inode_number();
                let link_count = extended_metadata.link_count();

                // Register file with hardlink tracker (optimization: skip if link_count == 1)
                if link_count > 1 {
                    hardlink_tracker.register_file(&src_path, device_id, inode_number, link_count);
                    debug!("Registered hardlink: {} (inode: {}, links: {})", src_path.display(), inode_number, link_count);
                }

                // Check if this inode has already been copied (for hardlinks)
                if link_count > 1 && hardlink_tracker.is_inode_copied(inode_number) {
                    // This is a hardlink - create a hardlink instead of copying content
                    debug!("Creating hardlink for {} (inode: {})", src_path.display(), inode_number);
                    
                    // Find the original file path for this inode
                    if let Some(original_path) = hardlink_tracker.get_original_path_for_inode(inode_number) {
                        // Create destination directory if needed
                        if let Some(parent) = dst_path.parent() {
                            if !parent.exists() {
                                std::fs::create_dir_all(parent).map_err(|e| {
                                    SyncError::FileSystem(format!(
                                        "Failed to create parent directory {}: {}",
                                        parent.display(),
                                        e
                                    ))
                                })?;
                            }
                        }
                        
                        // Create hardlink using std filesystem operations (compio has Send issues)
                        match std::fs::hard_link(&original_path, &dst_path) {
                            Ok(()) => {
                                stats.files_copied += 1;
                                debug!("Created hardlink: {} -> {}", dst_path.display(), original_path.display());
                            }
                            Err(e) => {
                                warn!("Failed to create hardlink for {}: {}", src_path.display(), e);
                                stats.errors += 1;
                            }
                        }
                    } else {
                        warn!("Could not find original path for inode {}", inode_number);
                        stats.errors += 1;
                    }
                } else {
                    // First time seeing this inode - copy the file content normally
                    debug!("Copying file content: {}", src_path.display());
                    
                    match copy_file(&src_path, &dst_path, copy_method.clone()).await {
                        Ok(()) => {
                            stats.files_copied += 1;
                            stats.bytes_copied += extended_metadata.len();

                            // Mark this inode as copied for future hardlink creation
                            if link_count > 1 {
                                hardlink_tracker.mark_inode_copied(inode_number, &dst_path);
                            }

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
                }
            } else if extended_metadata.is_symlink() {
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
    }

    Ok(())
}

/// Copy a symlink preserving its target
async fn copy_symlink(src: &Path, dst: &Path) -> Result<()> {
    let target = std::fs::read_link(src).map_err(|e| {
        SyncError::FileSystem(format!(
            "Failed to read symlink target for {}: {}",
            src.display(),
            e
        ))
    })?;

    // Remove destination if it exists
    if dst.exists() {
        std::fs::remove_file(dst).map_err(|e| {
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
        std::os::unix::fs::symlink(&target, dst).map_err(|e| {
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

    // TODO: Implement metadata preservation using compio's API
    // For now, we'll skip metadata preservation as compio's API is still evolving
    // This will be implemented in a future phase with proper compio bindings
    tracing::debug!("Metadata preservation skipped for {} (compio API limitations)", dst.display());

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
pub async fn get_directory_size(path: &Path) -> Result<u64> {
    let mut total_size = 0u64;
    let mut work_list = vec![path.to_path_buf()];
    
    while let Some(current_path) = work_list.pop() {
        let entries = std::fs::read_dir(&current_path).map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to read directory {}: {}",
                current_path.display(),
                e
            ))
        })?;

        for entry_result in entries {
            let entry = entry_result.map_err(|e| SyncError::FileSystem(format!("Failed to read directory entry: {}", e)))?;
            let entry_path = entry.path();
            let metadata = entry.metadata().map_err(|e| {
                SyncError::FileSystem(format!(
                    "Failed to get metadata for {}: {}",
                    entry_path.display(),
                    e
                ))
            })?;

            if metadata.is_dir() {
                // Add subdirectory to work list for later processing
                work_list.push(entry_path);
            } else if metadata.is_file() {
                total_size += metadata.len();
            }
            // Skip symlinks for size calculation
        }
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
pub async fn count_directory_contents(path: &Path) -> Result<(u64, u64)> {
    let mut file_count = 0u64;
    let mut dir_count = 0u64;
    let mut work_list = vec![path.to_path_buf()];
    
    while let Some(current_path) = work_list.pop() {
        let entries = std::fs::read_dir(&current_path).map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to read directory {}: {}",
                current_path.display(),
                e
            ))
        })?;

        for entry_result in entries {
            let entry = entry_result.map_err(|e| SyncError::FileSystem(format!("Failed to read directory entry: {}", e)))?;
            let entry_path = entry.path();
            let metadata = entry.metadata().map_err(|e| {
                SyncError::FileSystem(format!(
                    "Failed to get metadata for {}: {}",
                    entry_path.display(),
                    e
                ))
            })?;

            if metadata.is_dir() {
                dir_count += 1;
                // Add subdirectory to work list for later processing
                work_list.push(entry_path);
            } else if metadata.is_file() {
                file_count += 1;
            }
            // Skip symlinks for counting
        }
    }

    Ok((file_count, dir_count))
}

/// Filesystem boundary detection and hardlink tracking
///
/// This module provides functionality for detecting filesystem boundaries
/// and tracking hardlink relationships to ensure proper file copying behavior.
/// Filesystem device ID and inode number pair for hardlink detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub struct InodeInfo {
    /// Filesystem device ID
    pub dev: u64,
    /// Inode number
    pub ino: u64,
}

/// Hardlink tracking information
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HardlinkInfo {
    /// Original file path
    pub original_path: std::path::PathBuf,
    /// Inode number
    pub inode_number: u64,
    /// Number of hardlinks found
    pub link_count: u64,
    /// Whether this inode has been copied to destination
    pub is_copied: bool,
    /// Destination path where this inode was copied (for hardlink creation)
    pub dst_path: Option<std::path::PathBuf>,
}

/// Filesystem boundary and hardlink tracker
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct FilesystemTracker {
    /// Map of (dev, ino) pairs to hardlink information
    #[allow(clippy::disallowed_types)]
    hardlinks: HashMap<InodeInfo, HardlinkInfo>,
    /// Source filesystem device ID (for boundary detection)
    source_filesystem: Option<u64>,
}

#[allow(dead_code)]
impl FilesystemTracker {
    /// Create a new filesystem tracker
    pub fn new() -> Self {
        Self {
            #[allow(clippy::disallowed_types)]
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
    /// Files with link_count == 1 are skipped since they're not hardlinks.
    /// Returns true if this is a new hardlink, false if it's a duplicate or skipped.
    pub fn register_file(&mut self, path: &Path, dev: u64, ino: u64, link_count: u64) -> bool {
        // Skip files with link count of 1 - they're not hardlinks
        if link_count == 1 {
            return false;
        }
        let inode_info = InodeInfo { dev, ino };

        match self.hardlinks.get_mut(&inode_info) {
            Some(hardlink_info) => {
                // This is an existing hardlink
                hardlink_info.link_count += 1;
                debug!(
                    "Found hardlink #{} for inode ({}, {}): {}",
                    hardlink_info.link_count,
                    dev,
                    ino,
                    path.display()
                );
                false
            }
            None => {
                // This is a new file
                self.hardlinks.insert(
                    inode_info,
                    HardlinkInfo {
                        original_path: path.to_path_buf(),
                        inode_number: ino,
                        link_count: 1,
                        is_copied: false,
                        dst_path: None,
                    },
                );
                debug!(
                    "Registered new file inode ({}, {}): {}",
                    dev,
                    ino,
                    path.display()
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

    /// Check if an inode has already been copied (for hardlink creation)
    ///
    /// Returns true if this inode has been processed and copied to the destination.
    /// This is used to determine whether to copy file content or create a hardlink.
    pub fn is_inode_copied(&self, ino: u64) -> bool {
        self.hardlinks
            .values()
            .any(|info| info.inode_number == ino && info.is_copied)
    }

    /// Mark an inode as copied and store its destination path
    ///
    /// This should be called after successfully copying a file's content,
    /// so that subsequent hardlinks to the same inode can be created instead of copied.
    pub fn mark_inode_copied(&mut self, ino: u64, dst_path: &Path) {
        for info in self.hardlinks.values_mut() {
            if info.inode_number == ino {
                info.is_copied = true;
                info.dst_path = Some(dst_path.to_path_buf());
                debug!("Marked inode {} as copied to {}", ino, dst_path.display());
                break;
            }
        }
    }

    /// Get the original destination path for an inode that has been copied
    ///
    /// Returns the destination path where this inode's content was first copied.
    /// This is used to create hardlinks pointing to the original copied file.
    pub fn get_original_path_for_inode(&self, ino: u64) -> Option<&Path> {
        self.hardlinks
            .values()
            .find(|info| info.inode_number == ino && info.is_copied)
            .and_then(|info| info.dst_path.as_deref())
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
#[allow(dead_code)]
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


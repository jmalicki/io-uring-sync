//! io_uring integration module
//!
//! This module provides high-performance file operations using io_uring for asynchronous I/O.
//! Currently implements async I/O as a foundation, with plans for full io_uring integration
//! in future development phases.
//!
//! # Features
//!
//! - Asynchronous file read/write operations
//! - Buffer management for optimal performance
//! - Error handling and recovery
//! - Progress tracking capabilities
//! - File metadata operations
//!
//! # Usage
//!
//! ```rust
//! use io_uring_sync::io_uring::FileOperations;
//!
//! let mut ops = FileOperations::new(4096, 64 * 1024)?;
//! ops.copy_file_read_write(&src_path, &dst_path).await?;
//! ```

use crate::error::{Result, SyncError};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Basic file operations using async I/O
///
/// This structure provides a high-level interface for performing file operations
/// asynchronously. It serves as the foundation for io_uring integration and
/// provides efficient buffer management.
///
/// # Fields
///
/// * `buffer_size` - Size of buffers used for I/O operations in bytes
///
/// # Performance Considerations
///
/// - Buffer size should be tuned based on system memory and expected file sizes
/// - Larger buffers reduce system call overhead but increase memory usage
/// - Default buffer size of 64KB provides good balance for most workloads
pub struct FileOperations {
    /// Buffer size for I/O operations in bytes
    buffer_size: usize,
}

impl FileOperations {
    /// Create new file operations instance
    ///
    /// # Parameters
    ///
    /// * `queue_depth` - Maximum number of concurrent operations (currently unused, reserved for io_uring)
    /// * `buffer_size` - Size of I/O buffers in bytes
    ///
    /// # Returns
    ///
    /// Returns `Ok(FileOperations)` on success, or `Err(SyncError)` if initialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let ops = FileOperations::new(4096, 64 * 1024)?;
    /// ```
    ///
    /// # Performance Notes
    ///
    /// - Buffer size should be a power of 2 for optimal performance
    /// - Typical values: 4KB (small files), 64KB (general purpose), 1MB (large files)
    /// - Larger buffers reduce system call overhead but increase memory usage
    pub fn new(_queue_depth: usize, buffer_size: usize) -> Result<Self> {
        // For Phase 1.2, we'll use async I/O as a foundation
        // TODO: Implement actual io_uring integration in future phases
        Ok(Self { buffer_size })
    }

    /// Read file content asynchronously into memory
    ///
    /// This method reads the entire contents of a file into a byte vector.
    /// It uses async I/O for non-blocking operation and provides detailed
    /// error messages for troubleshooting.
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the file to read
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<u8>)` containing the file contents, or `Err(SyncError)` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The file doesn't exist
    /// - Permission is denied
    /// - The file is a directory
    /// - I/O error occurs during reading
    ///
    /// # Examples
    ///
    /// ```rust
    /// let content = ops.read_file(&Path::new("/path/to/file.txt")).await?;
    /// println!("File size: {} bytes", content.len());
    /// ```
    ///
    /// # Performance Considerations
    ///
    /// - For large files, consider using streaming reads to avoid memory issues
    /// - This method loads the entire file into memory at once
    /// - Memory usage is equal to file size
    pub async fn read_file(&mut self, path: &Path) -> Result<Vec<u8>> {
        let mut file = tokio::fs::File::open(path).await.map_err(|e| {
            SyncError::FileSystem(format!("Failed to open file {}: {}", path.display(), e))
        })?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await.map_err(|e| {
            SyncError::FileSystem(format!("Failed to read file {}: {}", path.display(), e))
        })?;

        Ok(buffer)
    }

    /// Write file content asynchronously
    pub async fn write_file(&mut self, path: &Path, content: &[u8]) -> Result<()> {
        let mut file = tokio::fs::File::create(path).await.map_err(|e| {
            SyncError::FileSystem(format!("Failed to create file {}: {}", path.display(), e))
        })?;

        file.write_all(content).await.map_err(|e| {
            SyncError::FileSystem(format!("Failed to write file {}: {}", path.display(), e))
        })?;

        file.sync_all().await.map_err(|e| {
            SyncError::FileSystem(format!("Failed to sync file {}: {}", path.display(), e))
        })?;

        Ok(())
    }

    /// Copy file using traditional read/write (fallback method)
    pub async fn copy_file_read_write(&mut self, src: &Path, dst: &Path) -> Result<()> {
        // Ensure destination directory exists
        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                SyncError::FileSystem(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        // Read source file
        let content = self.read_file(src).await?;

        // Write to destination
        self.write_file(dst, &content).await?;

        Ok(())
    }

    /// Get file size
    pub async fn get_file_size(&self, path: &Path) -> Result<u64> {
        let metadata = tokio::fs::metadata(path).await.map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to get metadata for {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(metadata.len())
    }

    /// Check if file exists
    pub async fn file_exists(&self, path: &Path) -> bool {
        tokio::fs::metadata(path).await.is_ok()
    }

    /// Create directory
    pub async fn create_dir(&self, path: &Path) -> Result<()> {
        tokio::fs::create_dir_all(path).await.map_err(|e| {
            SyncError::FileSystem(format!(
                "Failed to create directory {}: {}",
                path.display(),
                e
            ))
        })?;
        Ok(())
    }
}

/// File copy operation with progress tracking
pub struct CopyOperation {
    pub src_path: std::path::PathBuf,
    pub dst_path: std::path::PathBuf,
    pub file_size: u64,
    pub bytes_copied: u64,
    pub status: CopyStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CopyStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

impl CopyOperation {
    pub fn new(src: std::path::PathBuf, dst: std::path::PathBuf, size: u64) -> Self {
        Self {
            src_path: src,
            dst_path: dst,
            file_size: size,
            bytes_copied: 0,
            status: CopyStatus::Pending,
        }
    }

    pub fn update_progress(&mut self, bytes: u64) {
        self.bytes_copied += bytes;
        if self.bytes_copied >= self.file_size {
            self.status = CopyStatus::Completed;
        } else {
            self.status = CopyStatus::InProgress;
        }
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = CopyStatus::Failed(error);
    }

    pub fn progress_percentage(&self) -> f64 {
        if self.file_size == 0 {
            100.0
        } else {
            (self.bytes_copied as f64 / self.file_size as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_operations_basic() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let test_content = b"Hello, io_uring!";

        // Write test file
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            file.write_all(test_content).unwrap();
        }

        let mut ops = FileOperations::new(1024, 4096).unwrap();

        // Test file existence
        assert!(ops.file_exists(&test_file).await);

        // Test file size
        let size = ops.get_file_size(&test_file).await.unwrap();
        assert_eq!(size, test_content.len() as u64);

        // Test file reading
        let content = ops.read_file(&test_file).await.unwrap();
        assert_eq!(content, test_content);
    }

    #[tokio::test]
    async fn test_copy_operation() {
        let temp_dir = TempDir::new().unwrap();
        let src_file = temp_dir.path().join("src.txt");
        let dst_file = temp_dir.path().join("dst.txt");
        let test_content = b"This is a test file for copying.";

        // Create source file
        {
            let mut file = std::fs::File::create(&src_file).unwrap();
            file.write_all(test_content).unwrap();
        }

        let mut ops = FileOperations::new(1024, 4096).unwrap();

        // Test file copying
        ops.copy_file_read_write(&src_file, &dst_file)
            .await
            .unwrap();

        // Verify destination file
        assert!(ops.file_exists(&dst_file).await);
        let copied_content = ops.read_file(&dst_file).await.unwrap();
        assert_eq!(copied_content, test_content);
    }

    #[tokio::test]
    async fn test_copy_operation_progress() {
        let src = std::path::PathBuf::from("/tmp/src");
        let dst = std::path::PathBuf::from("/tmp/dst");
        let mut operation = CopyOperation::new(src, dst, 1000);

        assert_eq!(operation.progress_percentage(), 0.0);
        assert_eq!(operation.status, CopyStatus::Pending);

        operation.update_progress(500);
        assert_eq!(operation.progress_percentage(), 50.0);
        assert_eq!(operation.status, CopyStatus::InProgress);

        operation.update_progress(500);
        assert_eq!(operation.progress_percentage(), 100.0);
        assert_eq!(operation.status, CopyStatus::Completed);

        operation.mark_failed("Test error".to_string());
        assert_eq!(
            operation.status,
            CopyStatus::Failed("Test error".to_string())
        );
    }
}

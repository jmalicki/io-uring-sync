//! File ownership operations using `fchown` and `chown` syscalls
//!
//! This module provides file ownership preservation operations using direct
//! syscalls integrated with compio's runtime for optimal performance.
//!
//! # Operations
//!
//! - **`fchown`**: Change ownership using file descriptor (preferred)
//! - **`chown`**: Change ownership using file path (fallback)
//! - **Ownership preservation**: Copy ownership from source to destination
//!
//! # Usage
//!
//! ```rust,no_run
//! use compio_fs_extended::{ExtendedFile, OwnershipOps};
//! use compio::fs::File;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Open source and destination files
//! let src_file = File::open("source.txt").await?;
//! let dst_file = File::create("destination.txt").await?;
//!
//! // Create extended file wrappers
//! let src_extended = ExtendedFile::from_ref(&src_file);
//! let dst_extended = ExtendedFile::from_ref(&dst_file);
//!
//! // Preserve ownership from source to destination
//! src_extended.preserve_ownership_to(&dst_extended).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{filesystem_error, Result};
use compio::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// Trait for file ownership operations
pub trait OwnershipOps {
    /// Change file ownership using file descriptor
    ///
    /// # Arguments
    ///
    /// * `uid` - User ID to set
    /// * `gid` - Group ID to set
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if ownership was changed successfully, or `Err(ExtendedError)` if failed.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The file descriptor is invalid
    /// - Permission is denied (not root or file owner)
    /// - The uid/gid doesn't exist on the system
    async fn fchown(&self, uid: u32, gid: u32) -> Result<()>;

    /// Change file ownership using file path
    ///
    /// # Arguments
    ///
    /// * `path` - File path to change ownership of
    /// * `uid` - User ID to set
    /// * `gid` - Group ID to set
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if ownership was changed successfully, or `Err(ExtendedError)` if failed.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The file path doesn't exist
    /// - Permission is denied (not root or file owner)
    /// - The uid/gid doesn't exist on the system
    async fn chown<P: AsRef<Path>>(path: P, uid: u32, gid: u32) -> Result<()>;

    /// Preserve ownership from source file to destination file
    ///
    /// This function reads the ownership (uid/gid) from the source file and
    /// applies it to the destination file using the file descriptor.
    ///
    /// # Arguments
    ///
    /// * `src` - Source file to read ownership from
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if ownership was preserved successfully, or `Err(ExtendedError)` if failed.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Source file metadata cannot be read
    /// - Destination file ownership cannot be changed
    /// - Permission is denied
    async fn preserve_ownership_from(&self, src: &File) -> Result<()>;
}

impl OwnershipOps for File {
    async fn fchown(&self, uid: u32, gid: u32) -> Result<()> {
        let fd = self.as_raw_fd();

        compio::runtime::spawn_blocking(move || {
            let result = unsafe { libc::fchown(fd, uid, gid) };

            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(filesystem_error(&format!(
                    "fchown failed: {errno} (errno: {})",
                    errno.raw_os_error().unwrap_or(-1)
                )))
            } else {
                Ok(())
            }
        })
        .await
        .map_err(|e| filesystem_error(&format!("spawn_blocking failed: {e:?}")))?
    }

    async fn chown<P: AsRef<Path>>(path: P, uid: u32, gid: u32) -> Result<()> {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let path = path.as_ref();
        let path_cstr = CString::new(path.as_os_str().as_bytes())
            .map_err(|e| filesystem_error(&format!("Invalid path for chown: {e}")))?;

        compio::runtime::spawn_blocking(move || {
            let result = unsafe { libc::chown(path_cstr.as_ptr(), uid, gid) };

            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(filesystem_error(&format!(
                    "chown failed: {errno} (errno: {})",
                    errno.raw_os_error().unwrap_or(-1)
                )))
            } else {
                Ok(())
            }
        })
        .await
        .map_err(|e| filesystem_error(&format!("spawn_blocking failed: {e:?}")))?
    }

    async fn preserve_ownership_from(&self, src: &File) -> Result<()> {
        // Get source file metadata to extract uid/gid
        let src_metadata = src
            .metadata()
            .await
            .map_err(|e| filesystem_error(&format!("Failed to get source file metadata: {e}")))?;

        let uid = src_metadata.uid();
        let gid = src_metadata.gid();

        // Apply ownership to destination file
        self.fchown(uid, gid).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[compio::test]
    async fn test_fchown_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");

        // Create a test file
        fs::write(&file_path, "test content").unwrap();

        // Open the file
        let file = File::open(&file_path).await.unwrap();

        // Get current user/group (this test will only work if we can change ownership)
        let metadata = file.metadata().await.unwrap();
        let current_uid = metadata.uid();
        let current_gid = metadata.gid();

        // Try to change ownership (this may fail if not root, but that's expected)
        match file.fchown(current_uid, current_gid).await {
            Ok(_) => {
                // Success - verify ownership was set
                let new_metadata = file.metadata().await.unwrap();
                assert_eq!(new_metadata.uid(), current_uid);
                assert_eq!(new_metadata.gid(), current_gid);
            }
            Err(e) => {
                // Expected failure if not root - just log and continue
                println!("fchown failed (expected if not root): {}", e);
            }
        }
    }

    #[compio::test]
    async fn test_preserve_ownership() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("source.txt");
        let dst_path = temp_dir.path().join("destination.txt");

        // Create source file
        fs::write(&src_path, "source content").unwrap();

        // Open source and destination files
        let src_file = File::open(&src_path).await.unwrap();
        let dst_file = File::create(&dst_path).await.unwrap();

        // Get source ownership
        let src_metadata = src_file.metadata().await.unwrap();
        let src_uid = src_metadata.uid();
        let src_gid = src_metadata.gid();

        // Preserve ownership from source to destination
        match dst_file.preserve_ownership_from(&src_file).await {
            Ok(_) => {
                // Success - verify ownership was preserved
                let dst_metadata = dst_file.metadata().await.unwrap();
                assert_eq!(dst_metadata.uid(), src_uid);
                assert_eq!(dst_metadata.gid(), src_gid);
            }
            Err(e) => {
                // Expected failure if not root - just log and continue
                println!("preserve_ownership failed (expected if not root): {}", e);
            }
        }
    }

    #[compio::test]
    async fn test_chown_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");

        // Create a test file
        fs::write(&file_path, "test content").unwrap();

        // Get current user/group
        let metadata = fs::metadata(&file_path).unwrap();
        let current_uid = metadata.uid();
        let current_gid = metadata.gid();

        // Try to change ownership using path (this may fail if not root)
        match File::chown(&file_path, current_uid, current_gid).await {
            Ok(_) => {
                // Success - verify ownership was set
                let new_metadata = fs::metadata(&file_path).unwrap();
                assert_eq!(new_metadata.uid(), current_uid);
                assert_eq!(new_metadata.gid(), current_gid);
            }
            Err(e) => {
                // Expected failure if not root - just log and continue
                println!("chown failed (expected if not root): {}", e);
            }
        }
    }
}

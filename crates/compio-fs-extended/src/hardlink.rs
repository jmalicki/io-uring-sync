//! Hardlink operations for creating hard links

use crate::error::{hardlink_error, Result};
use compio::fs::File;
use std::path::Path;

/// Trait for hardlink operations
#[allow(async_fn_in_trait)]
pub trait HardlinkOps {
    /// Create a hard link to the file
    ///
    /// # Arguments
    ///
    /// * `target` - The target path for the hard link
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The target path already exists
    /// - The target is on a different filesystem
    /// - Permission is denied
    /// - The operation fails due to I/O errors
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use compio_fs_extended::{ExtendedFile, HardlinkOps};
    /// use compio::fs::File;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let file = File::open("original.txt").await?;
    /// let extended_file = ExtendedFile::new(file);
    ///
    /// extended_file.create_hardlink("hardlink.txt").await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn create_hardlink(&self, target: &Path) -> Result<()>;
}

/// Implementation of hardlink operations using direct syscalls
///
/// # Errors
///
/// This function will return an error if the hardlink creation fails
pub async fn create_hardlink_impl(_file: &File, _target: &Path) -> Result<()> {
    // Get the file path from the file descriptor
    // This is a simplified implementation - in practice, we'd need to track the path
    Err(hardlink_error(
        "create_hardlink not yet implemented - requires path tracking",
    ))
}

/// Create a hard link at the given path
///
/// # Arguments
///
/// * `original_path` - Path to the original file
/// * `link_path` - Path where the hard link will be created
///
/// # Returns
///
/// `Ok(())` if the hard link was created successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The link path already exists
/// - The original and link are on different filesystems
/// - Permission is denied
/// - The operation fails due to I/O errors
pub async fn create_hardlink_at_path(original_path: &Path, link_path: &Path) -> Result<()> {
    let original_cstr = std::ffi::CString::new(original_path.to_string_lossy().as_bytes())
        .map_err(|e| hardlink_error(&format!("Invalid original path: {}", e)))?;
    let link_cstr = std::ffi::CString::new(link_path.to_string_lossy().as_bytes())
        .map_err(|e| hardlink_error(&format!("Invalid link path: {}", e)))?;

    let result = unsafe { libc::link(original_cstr.as_ptr(), link_cstr.as_ptr()) };

    if result != 0 {
        let errno = std::io::Error::last_os_error();
        return Err(hardlink_error(&format!(
            "hardlink creation failed: {}",
            errno
        )));
    }

    Ok(())
}

/// Check if two paths point to the same file (same inode)
///
/// # Arguments
///
/// * `path1` - First path to compare
/// * `path2` - Second path to compare
///
/// # Returns
///
/// `true` if the paths point to the same file, `false` otherwise
#[must_use]
pub fn are_same_file(path1: &Path, path2: &Path) -> bool {
    match (std::fs::metadata(path1), std::fs::metadata(path2)) {
        (Ok(meta1), Ok(meta2)) => {
            use std::os::unix::fs::MetadataExt;
            meta1.ino() == meta2.ino() && meta1.dev() == meta2.dev()
        }
        _ => false,
    }
}

/// Get the number of hard links for a file
///
/// # Arguments
///
/// * `path` - Path to the file
///
/// # Returns
///
/// The number of hard links, or `None` if the operation fails
#[must_use]
pub fn get_link_count(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|meta| {
        use std::os::unix::fs::MetadataExt;
        meta.nlink()
    })
}

/// Check if a file has multiple hard links
///
/// # Arguments
///
/// * `path` - Path to the file
///
/// # Returns
///
/// `true` if the file has multiple hard links, `false` otherwise
#[must_use]
pub fn has_multiple_links(path: &Path) -> bool {
    get_link_count(path).is_some_and(|count| count > 1)
}

/// Find all hard links for a file
///
/// # Arguments
///
/// * `path` - Path to the file
///
/// # Returns
///
/// Vector of paths that are hard links to the same file
///
/// # Note
///
/// This is a simplified implementation that only checks the immediate directory.
/// A full implementation would need to traverse the filesystem.
///
/// # Errors
///
/// This function will return an error if the file metadata cannot be read
pub fn find_hard_links(path: &Path) -> Result<Vec<std::path::PathBuf>> {
    let original_meta = std::fs::metadata(path).map_err(|e| {
        hardlink_error(&format!(
            "Failed to get metadata for {}: {}",
            path.display(),
            e
        ))
    })?;

    use std::os::unix::fs::MetadataExt;
    let original_ino = original_meta.ino();
    let original_dev = original_meta.dev();

    let mut hard_links = Vec::new();

    if let Some(parent) = path.parent() {
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if let Ok(meta) = std::fs::metadata(&entry_path) {
                    if meta.ino() == original_ino && meta.dev() == original_dev {
                        hard_links.push(entry_path);
                    }
                }
            }
        }
    }

    Ok(hard_links)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_hardlink() {
        let temp_dir = TempDir::new().unwrap();
        let original_path = temp_dir.path().join("original.txt");
        let link_path = temp_dir.path().join("hardlink.txt");

        // Create original file
        fs::write(&original_path, "original content").unwrap();

        // Create hard link
        create_hardlink_at_path(&original_path, &link_path)
            .await
            .unwrap();

        // Verify both files exist and have the same content
        assert!(original_path.exists());
        assert!(link_path.exists());
        assert_eq!(
            fs::read(&original_path).unwrap(),
            fs::read(&link_path).unwrap()
        );

        // Verify they are the same file
        assert!(are_same_file(&original_path, &link_path));

        // Verify link count
        assert_eq!(get_link_count(&original_path), Some(2));
        assert_eq!(get_link_count(&link_path), Some(2));
    }

    #[tokio::test]
    async fn test_are_same_file() {
        let temp_dir = TempDir::new().unwrap();
        let original_path = temp_dir.path().join("original.txt");
        let link_path = temp_dir.path().join("hardlink.txt");
        let different_path = temp_dir.path().join("different.txt");

        // Create original file
        fs::write(&original_path, "content").unwrap();

        // Create hard link
        create_hardlink_at_path(&original_path, &link_path)
            .await
            .unwrap();

        // Create different file
        fs::write(&different_path, "different content").unwrap();

        // Test same file detection
        assert!(are_same_file(&original_path, &link_path));
        assert!(!are_same_file(&original_path, &different_path));
        assert!(!are_same_file(&link_path, &different_path));
    }

    #[tokio::test]
    async fn test_link_count() {
        let temp_dir = TempDir::new().unwrap();
        let original_path = temp_dir.path().join("original.txt");
        let link_path = temp_dir.path().join("hardlink.txt");

        // Create original file
        fs::write(&original_path, "content").unwrap();

        // Initially should have 1 link
        assert_eq!(get_link_count(&original_path), Some(1));
        assert!(!has_multiple_links(&original_path));

        // Create hard link
        create_hardlink_at_path(&original_path, &link_path)
            .await
            .unwrap();

        // Now should have 2 links
        assert_eq!(get_link_count(&original_path), Some(2));
        assert_eq!(get_link_count(&link_path), Some(2));
        assert!(has_multiple_links(&original_path));
        assert!(has_multiple_links(&link_path));
    }

    #[tokio::test]
    async fn test_find_hard_links() {
        let temp_dir = TempDir::new().unwrap();
        let original_path = temp_dir.path().join("original.txt");
        let link_path = temp_dir.path().join("hardlink.txt");

        // Create original file
        fs::write(&original_path, "content").unwrap();

        // Create hard link
        create_hardlink_at_path(&original_path, &link_path)
            .await
            .unwrap();

        // Find hard links
        let hard_links = find_hard_links(&original_path).unwrap();
        assert!(hard_links.len() >= 2); // At least original and link
        assert!(hard_links.contains(&original_path));
        assert!(hard_links.contains(&link_path));
    }
}

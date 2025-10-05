//! Directory operations for creating and managing directories

use crate::error::{directory_error, Result};
use compio::fs::File;
use std::path::Path;

/// Trait for directory operations
#[allow(async_fn_in_trait)]
pub trait DirectoryOps {
    /// Create a directory
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the directory will be created
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The directory already exists
    /// - The parent directory doesn't exist
    /// - Permission is denied
    /// - The operation fails due to I/O errors
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use compio_fs_extended::{ExtendedFile, DirectoryOps};
    /// use compio::fs::File;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let file = File::open("parent_dir").await?;
    /// let extended_file = ExtendedFile::new(file);
    ///
    /// extended_file.create_directory("new_dir").await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn create_directory(&self, path: &Path) -> Result<()>;

    /// Remove a directory
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the directory to remove
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The directory doesn't exist
    /// - The directory is not empty
    /// - Permission is denied
    /// - The operation fails due to I/O errors
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use compio_fs_extended::{ExtendedFile, DirectoryOps};
    /// use compio::fs::File;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let file = File::open("parent_dir").await?;
    /// let extended_file = ExtendedFile::new(file);
    ///
    /// extended_file.remove_directory("old_dir").await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn remove_directory(&self, path: &Path) -> Result<()>;
}

/// Implementation of directory operations using direct syscalls
///
/// # Errors
///
/// This function will return an error if the directory creation fails
pub async fn create_directory_impl(_file: &File, _path: &Path) -> Result<()> {
    // Get the file path from the file descriptor
    // This is a simplified implementation - in practice, we'd need to track the path
    Err(directory_error(
        "create_directory not yet implemented - requires path tracking",
    ))
}

/// Implementation of directory removal using direct syscalls
///
/// # Errors
///
/// This function will return an error if the directory removal fails
pub async fn remove_directory_impl(_file: &File, _path: &Path) -> Result<()> {
    // Get the file path from the file descriptor
    // This is a simplified implementation - in practice, we'd need to track the path
    Err(directory_error(
        "remove_directory not yet implemented - requires path tracking",
    ))
}

/// Create a directory at the given path
///
/// # Arguments
///
/// * `path` - Path where the directory will be created
///
/// # Returns
///
/// `Ok(())` if the directory was created successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The directory already exists
/// - The parent directory doesn't exist
/// - Permission is denied
/// - The operation fails due to I/O errors
pub async fn create_directory_at_path(path: &Path) -> Result<()> {
    let path_cstr = std::ffi::CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| directory_error(&format!("Invalid path: {}", e)))?;

    let result = unsafe {
        libc::mkdir(
            path_cstr.as_ptr(),
            0o755, // Default permissions
        )
    };

    if result != 0 {
        let errno = std::io::Error::last_os_error();
        return Err(directory_error(&format!("mkdir failed: {}", errno)));
    }

    Ok(())
}

/// Create a directory with specific permissions
///
/// # Arguments
///
/// * `path` - Path where the directory will be created
/// * `mode` - Permissions for the directory (octal)
///
/// # Returns
///
/// `Ok(())` if the directory was created successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The directory already exists
/// - The parent directory doesn't exist
/// - Permission is denied
/// - The operation fails due to I/O errors
pub async fn create_directory_with_mode(path: &Path, mode: u32) -> Result<()> {
    let path_cstr = std::ffi::CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| directory_error(&format!("Invalid path: {}", e)))?;

    let result = unsafe { libc::mkdir(path_cstr.as_ptr(), mode) };

    if result != 0 {
        let errno = std::io::Error::last_os_error();
        return Err(directory_error(&format!("mkdir failed: {}", errno)));
    }

    Ok(())
}

/// Remove a directory at the given path
///
/// # Arguments
///
/// * `path` - Path to the directory to remove
///
/// # Returns
///
/// `Ok(())` if the directory was removed successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The directory doesn't exist
/// - The directory is not empty
/// - Permission is denied
/// - The operation fails due to I/O errors
pub async fn remove_directory_at_path(path: &Path) -> Result<()> {
    let path_cstr = std::ffi::CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| directory_error(&format!("Invalid path: {}", e)))?;

    let result = unsafe { libc::rmdir(path_cstr.as_ptr()) };

    if result != 0 {
        let errno = std::io::Error::last_os_error();
        return Err(directory_error(&format!("rmdir failed: {}", errno)));
    }

    Ok(())
}

/// Create a directory and all parent directories if they don't exist
///
/// # Arguments
///
/// * `path` - Path where the directory will be created
///
/// # Returns
///
/// `Ok(())` if the directory was created successfully
///
/// # Errors
///
/// This function will return an error if:
/// - Permission is denied
/// - The operation fails due to I/O errors
pub async fn create_directory_recursive(path: &Path) -> Result<()> {
    if path.exists() {
        if path.is_dir() {
            return Ok(());
        } else {
            return Err(directory_error(&format!(
                "Path exists but is not a directory: {}",
                path.display()
            )));
        }
    }

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            Box::pin(create_directory_recursive(parent)).await?;
        }
    }

    create_directory_at_path(path).await
}

/// Check if a path is a directory
///
/// # Arguments
///
/// * `path` - Path to check
///
/// # Returns
///
/// `true` if the path is a directory, `false` otherwise
#[must_use]
pub fn is_directory(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.is_dir())
        .unwrap_or(false)
}

/// Check if a directory is empty
///
/// # Arguments
///
/// * `path` - Path to the directory
///
/// # Returns
///
/// `true` if the directory is empty, `false` otherwise
#[must_use]
pub fn is_directory_empty(path: &Path) -> bool {
    std::fs::read_dir(path)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
}

/// Get the size of a directory (sum of all file sizes)
///
/// # Arguments
///
/// * `path` - Path to the directory
///
/// # Returns
///
/// The total size in bytes, or `None` if the operation fails
#[must_use]
pub fn get_directory_size(path: &Path) -> Option<u64> {
    let mut total_size = 0u64;

    fn calculate_size(path: &Path, total: &mut u64) -> bool {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if let Ok(metadata) = std::fs::metadata(&entry_path) {
                    if metadata.is_dir() {
                        if !calculate_size(&entry_path, total) {
                            return false;
                        }
                    } else {
                        *total += metadata.len();
                    }
                }
            }
        }
        true
    }

    if calculate_size(path, &mut total_size) {
        Some(total_size)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("new_dir");

        // Create directory
        create_directory_at_path(&dir_path).await.unwrap();

        // Verify directory exists
        assert!(dir_path.exists());
        assert!(is_directory(&dir_path));
    }

    #[tokio::test]
    async fn test_create_directory_with_mode() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("new_dir");

        // Create directory with specific mode
        create_directory_with_mode(&dir_path, 0o700).await.unwrap();

        // Verify directory exists
        assert!(dir_path.exists());
        assert!(is_directory(&dir_path));
    }

    #[tokio::test]
    async fn test_remove_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("temp_dir");

        // Create directory
        create_directory_at_path(&dir_path).await.unwrap();
        assert!(dir_path.exists());

        // Remove directory
        remove_directory_at_path(&dir_path).await.unwrap();
        assert!(!dir_path.exists());
    }

    #[tokio::test]
    async fn test_create_directory_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("level1").join("level2").join("level3");

        // Create nested directory
        create_directory_recursive(&nested_path).await.unwrap();

        // Verify all levels exist
        assert!(nested_path.exists());
        assert!(nested_path.parent().unwrap().exists());
        assert!(nested_path.parent().unwrap().parent().unwrap().exists());
    }

    #[tokio::test]
    async fn test_is_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("test_dir");
        let file_path = temp_dir.path().join("test_file.txt");

        // Create directory and file
        create_directory_at_path(&dir_path).await.unwrap();
        fs::write(&file_path, "content").unwrap();

        // Test directory detection
        assert!(is_directory(&dir_path));
        assert!(!is_directory(&file_path));
        assert!(!is_directory(&temp_dir.path().join("nonexistent")));
    }

    #[tokio::test]
    async fn test_is_directory_empty() {
        let temp_dir = TempDir::new().unwrap();
        let empty_dir = temp_dir.path().join("empty_dir");
        let non_empty_dir = temp_dir.path().join("non_empty_dir");

        // Create empty directory
        create_directory_at_path(&empty_dir).await.unwrap();
        assert!(is_directory_empty(&empty_dir));

        // Create non-empty directory
        create_directory_at_path(&non_empty_dir).await.unwrap();
        fs::write(non_empty_dir.join("file.txt"), "content").unwrap();
        assert!(!is_directory_empty(&non_empty_dir));
    }

    #[tokio::test]
    async fn test_get_directory_size() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("test_dir");

        // Create directory with files
        create_directory_at_path(&dir_path).await.unwrap();
        fs::write(dir_path.join("file1.txt"), "content1").unwrap();
        fs::write(dir_path.join("file2.txt"), "content2").unwrap();

        // Get directory size
        let size = get_directory_size(&dir_path).unwrap();
        assert!(size > 0);
    }
}

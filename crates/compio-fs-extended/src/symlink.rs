//! Symlink operations for creating and reading symbolic links

use crate::error::{symlink_error, Result};
use compio::driver::OpCode;
use compio::fs::File;
use compio::runtime::submit;
use io_uring::{opcode, types};
use nix::fcntl;
use std::ffi::CString;
use std::path::Path;
use std::pin::Pin;

/// Custom symlink operation that implements compio's OpCode trait
pub struct SymlinkOp {
    target: CString,
    link_path: CString,
    dir_fd: Option<std::os::unix::io::RawFd>,
}

impl SymlinkOp {
    pub fn new_with_dirfd(
        dir_fd: &crate::directory::DirectoryFd,
        target: &str,
        link_name: &str,
    ) -> Result<Self> {
        let target_cstr =
            CString::new(target).map_err(|e| symlink_error(&format!("Invalid target: {}", e)))?;
        let link_path_cstr = CString::new(link_name)
            .map_err(|e| symlink_error(&format!("Invalid link name: {}", e)))?;

        Ok(Self {
            target: target_cstr,
            link_path: link_path_cstr,
            dir_fd: Some(dir_fd.as_raw_fd()),
        })
    }
}

impl OpCode for SymlinkOp {
    fn create_entry(self: Pin<&mut Self>) -> compio::driver::OpEntry {
        compio::driver::OpEntry::Submission(
            opcode::SymlinkAt::new(
                types::Fd(self.dir_fd.unwrap_or(libc::AT_FDCWD)),
                self.target.as_ptr(),
                self.link_path.as_ptr(),
            )
            .build(),
        )
    }
}

/// Trait for symlink operations
#[allow(async_fn_in_trait)]
pub trait SymlinkOps {
    /// Read the target of a symbolic link
    ///
    /// # Returns
    ///
    /// The target path of the symbolic link
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The file is not a symbolic link
    /// - The symbolic link is broken
    /// - The operation fails due to I/O errors
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use compio_fs_extended::{ExtendedFile, SymlinkOps};
    /// use compio::fs::File;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let file = File::open("symlink.txt").await?;
    /// let extended_file = ExtendedFile::new(file);
    ///
    /// let target = extended_file.read_symlink().await?;
    /// println!("Symlink points to: {:?}", target);
    /// # Ok(())
    /// # }
    /// ```
    async fn read_symlink(&self) -> Result<std::path::PathBuf>;

    /// Create a symbolic link
    ///
    /// # Arguments
    ///
    /// * `target` - The target path for the symbolic link
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The target path is invalid
    /// - The operation fails due to I/O errors
    /// - Permission is denied
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use compio_fs_extended::{ExtendedFile, SymlinkOps};
    /// use compio::fs::File;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let file = File::create("new_symlink.txt").await?;
    /// let extended_file = ExtendedFile::new(file);
    ///
    /// extended_file.create_symlink("target.txt").await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn create_symlink(&self, target: &Path) -> Result<()>;
}

/// Implementation of symlink operations using direct syscalls
///
/// # Errors
///
/// This function will return an error if the symlink read fails
pub async fn read_symlink_impl(_file: &File) -> Result<std::path::PathBuf> {
    // Get the file path from the file descriptor
    // This is a simplified implementation - in practice, we'd need to track the path
    Err(symlink_error(
        "read_symlink not yet implemented - requires path tracking",
    ))
}

/// Implementation of symlink creation using direct syscalls
///
/// # Errors
///
/// This function will return an error if the symlink creation fails
pub async fn create_symlink_impl(_file: &File, _target: &Path) -> Result<()> {
    // Get the file path from the file descriptor
    // This is a simplified implementation - in practice, we'd need to track the path
    Err(symlink_error(
        "create_symlink not yet implemented - requires path tracking",
    ))
}

/// Create a symbolic link at the given path
///
/// # Arguments
///
/// * `link_path` - Path where the symbolic link will be created
/// * `target` - Target path for the symbolic link
///
/// # Returns
///
/// `Ok(())` if the symbolic link was created successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The link path already exists
/// - The target path is invalid
/// - Permission is denied
/// - The operation fails due to I/O errors
// Note: io_uring symlink operations removed - using secure *at variants instead
///   Create a symbolic link using io_uring with DirectoryFd (secure)
///
/// # Arguments
///
/// * `dir_fd` - Directory file descriptor for secure operation
/// * `target` - Target path for the symbolic link
/// * `link_name` - Name of the symbolic link relative to the directory
///
/// # Returns
///
/// `Ok(())` if the symbolic link was created successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The link name is invalid
/// - Permission is denied
/// - The operation fails due to I/O errors
///
/// # Example
///
/// ```rust,no_run
/// use compio_fs_extended::{directory::DirectoryFd, symlink::create_symlink_at_dirfd};
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let dir_fd = DirectoryFd::open(Path::new("/some/directory")).await?;
/// create_symlink_at_dirfd(&dir_fd, "target_file", "my_link").await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_symlink_at_dirfd(
    dir_fd: &crate::directory::DirectoryFd,
    target: &str,
    link_name: &str,
) -> Result<()> {
    // Submit io_uring symlink operation using compio's runtime with DirectoryFd
    let result = submit(SymlinkOp::new_with_dirfd(dir_fd, target, link_name)?).await;

    // Convert the result to our error type
    match result.0 {
        Ok(_) => Ok(()),
        Err(e) => Err(symlink_error(&format!(
            "io_uring symlink operation failed: {}",
            e
        ))),
    }
}

// Note: Basic symlink operations are provided by std::fs or compio::fs
// This module focuses on io_uring operations and secure *at variants

// Note: Basic readlink operations are provided by std::fs or compio::fs
// This module focuses on io_uring operations and secure *at variants

/// Read the target of a symbolic link using DirectoryFd (secure)
///
/// # Arguments
///
/// * `dir_fd` - Directory file descriptor for secure operation
/// * `link_name` - Name of the symbolic link relative to the directory
///
/// # Returns
///
/// The target path of the symbolic link
///
/// # Errors
///
/// This function will return an error if:
/// - The path is not a symbolic link
/// - The symbolic link is broken
/// - The operation fails due to I/O errors
///
/// # Example
///
/// ```rust,no_run
/// use compio_fs_extended::{directory::DirectoryFd, symlink::read_symlink_at_dirfd};
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let dir_fd = DirectoryFd::open(Path::new("/some/directory")).await?;
/// let target = read_symlink_at_dirfd(&dir_fd, "my_link").await?;
/// # Ok(())
/// # }
/// ```
pub async fn read_symlink_at_dirfd(
    dir_fd: &crate::directory::DirectoryFd,
    link_name: &str,
) -> Result<std::path::PathBuf> {
    let link_name = link_name.to_string();
    let dir_fd_raw = dir_fd.as_raw_fd();

    let os_string = compio::runtime::spawn(async move {
        fcntl::readlinkat(Some(dir_fd_raw), std::path::Path::new(&link_name))
            .map_err(|e| symlink_error(&format!("readlinkat failed for '{}': {}", link_name, e)))
    })
    .await
    .map_err(|e| symlink_error(&format!("spawn failed: {:?}", e)))?;

    Ok(std::path::PathBuf::from(os_string?))
}

// Note: Basic symlink operations like is_symlink, is_broken_symlink are provided by std::fs
// This module focuses on io_uring operations and secure *at variants

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[compio::test]
    async fn test_secure_symlink_creation() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("target.txt");

        // Create target file
        fs::write(&target_path, "target content").unwrap();

        // Test secure symlink creation using DirectoryFd
        let dir_fd = crate::directory::DirectoryFd::open(temp_dir.path())
            .await
            .unwrap();

        // Use a unique link name to avoid conflicts
        let link_name = "unique_secure_link";
        let link_path = temp_dir.path().join(link_name);

        // Clean up any existing symlink first
        if link_path.exists() {
            fs::remove_file(&link_path).unwrap();
        }

        create_symlink_at_dirfd(&dir_fd, "target.txt", link_name)
            .await
            .unwrap();

        // Verify the symlink was created using std::fs
        assert!(link_path.is_symlink());

        // Read the symlink target using std::fs
        let target = std::fs::read_link(&link_path).unwrap();
        assert_eq!(target, std::path::PathBuf::from("target.txt"));
    }

    #[compio::test]
    async fn test_secure_symlink_operations() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("target.txt");

        // Create target file
        fs::write(&target_path, "target content").unwrap();

        // Test secure symlink creation using DirectoryFd
        let dir_fd = crate::directory::DirectoryFd::open(temp_dir.path())
            .await
            .unwrap();

        // Use a unique link name to avoid conflicts
        let link_name = "unique_secure_ops_link";
        let link_path = temp_dir.path().join(link_name);

        // Clean up any existing symlink first
        if link_path.exists() {
            fs::remove_file(&link_path).unwrap();
        }

        create_symlink_at_dirfd(&dir_fd, "target.txt", link_name)
            .await
            .unwrap();

        // Test secure symlink reading using DirectoryFd
        let target = read_symlink_at_dirfd(&dir_fd, link_name).await.unwrap();
        assert_eq!(target, std::path::PathBuf::from("target.txt"));
    }
}

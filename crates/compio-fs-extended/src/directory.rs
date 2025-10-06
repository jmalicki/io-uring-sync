//! Directory file descriptor for secure directory-based operations

use crate::error::{directory_error, Result};
use compio::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A directory file descriptor for secure directory-based operations
///
/// `DirectoryFd` provides a safe wrapper around a directory file descriptor,
/// enabling secure `*at` system calls that avoid TOCTOU (Time-of-Check-Time-of-Use)
/// race conditions. This is the recommended way to perform file operations
/// relative to a directory.
///
/// # Example
///
/// ```rust,no_run
/// use compio_fs_extended::directory::DirectoryFd;
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let dir_fd = DirectoryFd::open(Path::new("/some/directory")).await?;
/// // Use dir_fd for secure file operations
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct DirectoryFd {
    /// The underlying file descriptor
    file: Arc<File>,
    /// The path this directory represents (for debugging/error messages)
    path: PathBuf,
}

impl DirectoryFd {
    /// Open a directory and return a `DirectoryFd`
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the directory to open
    ///
    /// # Returns
    ///
    /// `Ok(DirectoryFd)` if the directory was opened successfully
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The path doesn't exist
    /// - The path is not a directory
    /// - Permission is denied
    /// - The operation fails due to I/O errors
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use compio_fs_extended::directory::DirectoryFd;
    /// use std::path::Path;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let dir_fd = DirectoryFd::open(Path::new("/tmp")).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)
            .await
            .map_err(|e| directory_error(&format!("Failed to open directory {:?}: {}", path, e)))?;

        Ok(Self {
            file: Arc::new(file),
            path: path.to_path_buf(),
        })
    }

    /// Get a reference to the underlying file descriptor
    ///
    /// This is used internally by `*at` operations to get the file descriptor
    /// for the directory.
    pub fn as_file(&self) -> &File {
        &self.file
    }

    /// Get the path this directory represents
    ///
    /// This is primarily used for error messages and debugging.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the raw file descriptor for use with system calls
    ///
    /// This is used internally by `*at` operations that need the raw fd.
    pub fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.file.as_raw_fd()
    }

    /// Create a directory relative to this DirectoryFd
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the directory to create (relative to this directory)
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
    /// - Permission is denied
    /// - The operation fails due to I/O errors
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use compio_fs_extended::directory::DirectoryFd;
    /// use std::path::Path;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let dir_fd = DirectoryFd::open(Path::new("/tmp")).await?;
    /// dir_fd.create_directory("new_dir", 0o755).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_directory(&self, name: &str, mode: u32) -> Result<()> {
        // TODO: Implement using io_uring MkdirAt opcode when available
        // For now, use nix with spawn for security
        let dir_fd = self.as_raw_fd();
        let name_owned = name.to_string();

        compio::runtime::spawn(async move {
            nix::sys::stat::mkdirat(
                Some(dir_fd),
                std::path::Path::new(&name_owned),
                nix::sys::stat::Mode::from_bits_truncate(mode),
            )
            .map_err(|e| directory_error(&format!("mkdirat failed for '{}': {}", name_owned, e)))
        })
        .await
        .map_err(|e| directory_error(&format!("spawn failed: {:?}", e)))?
    }
}

impl Clone for DirectoryFd {
    fn clone(&self) -> Self {
        Self {
            file: Arc::clone(&self.file),
            path: self.path.clone(),
        }
    }
}

// Note: Basic directory operations (create_dir, remove_dir, etc.) are provided by compio::fs
// This module only provides DirectoryFd for secure *at operations

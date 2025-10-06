//! Extended attributes (xattr) operations using io_uring opcodes

use crate::error::{xattr_error, Result};
use compio::fs::File;
use std::os::fd::AsRawFd;
use std::path::Path;

/// Trait for xattr operations
#[allow(async_fn_in_trait)]
pub trait XattrOps {
    /// Get an extended attribute value
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the extended attribute
    ///
    /// # Returns
    ///
    /// The value of the extended attribute
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The extended attribute doesn't exist
    /// - Permission is denied
    /// - The operation fails due to I/O errors
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use compio_fs_extended::{ExtendedFile, XattrOps};
    /// use compio::fs::File;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let file = File::open("file.txt").await?;
    /// let extended_file = ExtendedFile::new(file);
    ///
    /// let value = extended_file.get_xattr("user.custom").await?;
    /// println!("xattr value: {:?}", value);
    /// # Ok(())
    /// # }
    /// ```
    async fn get_xattr(&self, name: &str) -> Result<Vec<u8>>;

    /// Set an extended attribute value
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the extended attribute
    /// * `value` - Value to set
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Permission is denied
    /// - The operation fails due to I/O errors
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use compio_fs_extended::{ExtendedFile, XattrOps};
    /// use compio::fs::File;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let file = File::open("file.txt").await?;
    /// let extended_file = ExtendedFile::new(file);
    ///
    /// extended_file.set_xattr("user.custom", b"value").await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn set_xattr(&self, name: &str, value: &[u8]) -> Result<()>;

    /// List all extended attributes
    ///
    /// # Returns
    ///
    /// Vector of extended attribute names
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Permission is denied
    /// - The operation fails due to I/O errors
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use compio_fs_extended::{ExtendedFile, XattrOps};
    /// use compio::fs::File;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let file = File::open("file.txt").await?;
    /// let extended_file = ExtendedFile::new(file);
    ///
    /// let names = extended_file.list_xattr().await?;
    /// for name in names {
    ///     println!("xattr: {}", name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn list_xattr(&self) -> Result<Vec<String>>;
}

/// Implementation of xattr operations using io_uring opcodes
///
/// # Errors
///
/// This function will return an error if the xattr operation fails
pub async fn get_xattr_impl(file: &File, name: &str) -> Result<Vec<u8>> {
    let name_cstr = std::ffi::CString::new(name)
        .map_err(|e| xattr_error(&format!("Invalid xattr name: {}", e)))?;

    // Get file descriptor
    let fd = file.as_raw_fd();

    // First, get the size of the xattr value
    let name_cstr_clone = name_cstr.clone();
    let result = compio::runtime::spawn_blocking(move || unsafe {
        let size = libc::fgetxattr(fd, name_cstr_clone.as_ptr(), std::ptr::null_mut(), 0);
        if size < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(size as usize)
        }
    })
    .await
    .map_err(|e| xattr_error(&format!("Failed to get xattr size: {:?}", e)))?;

    let size = result.map_err(|e| xattr_error(&format!("Failed to get xattr size: {}", e)))?;

    if size == 0 {
        return Ok(Vec::new());
    }

    // Allocate buffer and get the actual value
    let result = compio::runtime::spawn_blocking(move || {
        let mut buffer = vec![0u8; size];
        let actual_size = unsafe {
            libc::fgetxattr(
                fd,
                name_cstr.as_ptr(),
                buffer.as_mut_ptr() as *mut libc::c_void,
                size,
            )
        };

        if actual_size < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            buffer.truncate(actual_size as usize);
            Ok(buffer)
        }
    })
    .await
    .map_err(|e| xattr_error(&format!("Failed to get xattr value: {:?}", e)))?;

    result.map_err(|e| xattr_error(&format!("Failed to get xattr value: {}", e)))
}

/// Implementation of xattr setting using io_uring opcodes
///
/// # Errors
///
/// This function will return an error if the xattr operation fails
pub async fn set_xattr_impl(file: &File, name: &str, value: &[u8]) -> Result<()> {
    let name_cstr = std::ffi::CString::new(name)
        .map_err(|e| xattr_error(&format!("Invalid xattr name: {}", e)))?;

    // Get file descriptor
    let fd = file.as_raw_fd();
    let value_vec = value.to_vec();

    let result = compio::runtime::spawn_blocking(move || {
        unsafe {
            let result = libc::fsetxattr(
                fd,
                name_cstr.as_ptr(),
                value_vec.as_ptr() as *const libc::c_void,
                value_vec.len(),
                0, // flags
            );

            if result < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    })
    .await
    .map_err(|e| xattr_error(&format!("Failed to set xattr: {:?}", e)))?;

    result.map_err(|e| xattr_error(&format!("Failed to set xattr: {}", e)))
}

/// Implementation of xattr listing using io_uring opcodes
///
/// # Errors
///
/// This function will return an error if the xattr operation fails
pub async fn list_xattr_impl(file: &File) -> Result<Vec<String>> {
    // Get file descriptor
    let fd = file.as_raw_fd();

    // First, get the size needed for the list
    let result = compio::runtime::spawn_blocking(move || unsafe {
        let size = libc::flistxattr(fd, std::ptr::null_mut(), 0);
        if size < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(size as usize)
        }
    })
    .await
    .map_err(|e| xattr_error(&format!("Failed to get xattr list size: {:?}", e)))?;

    let size = result.map_err(|e| xattr_error(&format!("Failed to get xattr list size: {}", e)))?;

    if size == 0 {
        return Ok(Vec::new());
    }

    // Allocate buffer and get the actual list
    let result = compio::runtime::spawn_blocking(move || {
        let mut buffer = vec![0u8; size];
        let actual_size =
            unsafe { libc::flistxattr(fd, buffer.as_mut_ptr() as *mut libc::c_char, size) };

        if actual_size < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            buffer.truncate(actual_size as usize);

            // Parse the null-separated list of attribute names
            let names: Vec<String> = buffer
                .split(|&b| b == 0)
                .filter(|s| !s.is_empty())
                .map(|s| String::from_utf8_lossy(s).to_string())
                .collect();

            Ok(names)
        }
    })
    .await
    .map_err(|e| xattr_error(&format!("Failed to list xattr: {:?}", e)))?;

    result.map_err(|e| xattr_error(&format!("Failed to list xattr: {}", e)))
}

/// Get an extended attribute value at the given path
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `name` - Name of the extended attribute
///
/// # Returns
///
/// The value of the extended attribute
///
/// # Errors
///
/// This function will return an error if:
/// - The extended attribute doesn't exist
/// - Permission is denied
/// - The operation fails due to I/O errors
pub async fn get_xattr_at_path(path: &Path, name: &str) -> Result<Vec<u8>> {
    let path_cstr = std::ffi::CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| xattr_error(&format!("Invalid path: {}", e)))?;
    let name_cstr =
        std::ffi::CString::new(name).map_err(|e| xattr_error(&format!("Invalid name: {}", e)))?;

    // Get the size first
    let size = unsafe {
        libc::getxattr(
            path_cstr.as_ptr(),
            name_cstr.as_ptr(),
            std::ptr::null_mut(),
            0,
        )
    };

    if size < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(xattr_error(&format!("getxattr failed: {}", errno)));
    }

    // Allocate buffer and get the value
    let mut buffer = vec![0u8; size as usize];
    let actual_size = unsafe {
        libc::getxattr(
            path_cstr.as_ptr(),
            name_cstr.as_ptr(),
            buffer.as_mut_ptr() as *mut libc::c_void,
            buffer.len(),
        )
    };

    if actual_size < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(xattr_error(&format!("getxattr failed: {}", errno)));
    }

    buffer.truncate(actual_size as usize);
    Ok(buffer)
}

/// Set an extended attribute value at the given path
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `name` - Name of the extended attribute
/// * `value` - Value to set
///
/// # Returns
///
/// `Ok(())` if the extended attribute was set successfully
///
/// # Errors
///
/// This function will return an error if:
/// - Permission is denied
/// - The operation fails due to I/O errors
pub async fn set_xattr_at_path(path: &Path, name: &str, value: &[u8]) -> Result<()> {
    let path_cstr = std::ffi::CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| xattr_error(&format!("Invalid path: {}", e)))?;
    let name_cstr =
        std::ffi::CString::new(name).map_err(|e| xattr_error(&format!("Invalid name: {}", e)))?;

    let result = unsafe {
        libc::setxattr(
            path_cstr.as_ptr(),
            name_cstr.as_ptr(),
            value.as_ptr() as *const libc::c_void,
            value.len(),
            0, // flags
        )
    };

    if result != 0 {
        let errno = std::io::Error::last_os_error();
        return Err(xattr_error(&format!("setxattr failed: {}", errno)));
    }

    Ok(())
}

/// List all extended attributes at the given path
///
/// # Arguments
///
/// * `path` - Path to the file
///
/// # Returns
///
/// Vector of extended attribute names
///
/// # Errors
///
/// This function will return an error if:
/// - Permission is denied
/// - The operation fails due to I/O errors
pub async fn list_xattr_at_path(path: &Path) -> Result<Vec<String>> {
    let path_cstr = std::ffi::CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| xattr_error(&format!("Invalid path: {}", e)))?;

    // Get the size first
    let size = unsafe { libc::listxattr(path_cstr.as_ptr(), std::ptr::null_mut(), 0) };

    if size < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(xattr_error(&format!("listxattr failed: {}", errno)));
    }

    if size == 0 {
        return Ok(Vec::new());
    }

    // Allocate buffer and get the list
    let mut buffer = vec![0u8; size as usize];
    let actual_size = unsafe {
        libc::listxattr(
            path_cstr.as_ptr(),
            buffer.as_mut_ptr() as *mut libc::c_char,
            buffer.len(),
        )
    };

    if actual_size < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(xattr_error(&format!("listxattr failed: {}", errno)));
    }

    // Parse the null-separated list
    let mut names = Vec::new();
    let mut start = 0;
    for (i, &byte) in buffer.iter().enumerate() {
        if byte == 0 {
            if start < i {
                if let Ok(name) = String::from_utf8(buffer[start..i].to_vec()) {
                    names.push(name);
                }
            }
            start = i + 1;
        }
    }

    Ok(names)
}

/// Remove an extended attribute
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `name` - Name of the extended attribute to remove
///
/// # Returns
///
/// `Ok(())` if the extended attribute was removed successfully
///
/// # Errors
///
/// This function will return an error if:
/// - The extended attribute doesn't exist
/// - Permission is denied
/// - The operation fails due to I/O errors
pub async fn remove_xattr_at_path(path: &Path, name: &str) -> Result<()> {
    let path_cstr = std::ffi::CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| xattr_error(&format!("Invalid path: {}", e)))?;
    let name_cstr =
        std::ffi::CString::new(name).map_err(|e| xattr_error(&format!("Invalid name: {}", e)))?;

    let result = unsafe { libc::removexattr(path_cstr.as_ptr(), name_cstr.as_ptr()) };

    if result != 0 {
        let errno = std::io::Error::last_os_error();
        return Err(xattr_error(&format!("removexattr failed: {}", errno)));
    }

    Ok(())
}

/// Check if extended attributes are supported on the filesystem
///
/// # Arguments
///
/// * `path` - Path to check
///
/// # Returns
///
/// `true` if extended attributes are supported, `false` otherwise
pub async fn is_xattr_supported(path: &Path) -> bool {
    // Try to set a test attribute
    let test_name = "user.compio_fs_extended_test";
    let test_value = b"test";

    match set_xattr_at_path(path, test_name, test_value).await {
        Ok(_) => {
            // Clean up the test attribute
            let _ = remove_xattr_at_path(path, test_name).await;
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[compio::test]
    async fn test_xattr_operations() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        fs::write(&file_path, "test content").unwrap();

        // Test xattr support
        if is_xattr_supported(&file_path).await {
            // Test set and get
            set_xattr_at_path(&file_path, "user.test", b"test_value")
                .await
                .unwrap();
            let value = get_xattr_at_path(&file_path, "user.test").await.unwrap();
            assert_eq!(value, b"test_value");

            // Test list
            let names = list_xattr_at_path(&file_path).await.unwrap();
            assert!(names.contains(&"user.test".to_string()));

            // Test remove
            remove_xattr_at_path(&file_path, "user.test").await.unwrap();
            let names_after = list_xattr_at_path(&file_path).await.unwrap();
            assert!(!names_after.contains(&"user.test".to_string()));
        } else {
            println!("Extended attributes not supported on this filesystem");
        }
    }

    #[compio::test]
    async fn test_xattr_support_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        fs::write(&file_path, "test content").unwrap();

        // Test support detection
        let supported = is_xattr_supported(&file_path).await;
        println!("xattr supported: {}", supported);
    }
}

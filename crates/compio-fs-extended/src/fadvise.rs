//! fadvise operations for file access pattern optimization

use crate::error::{fadvise_error, Result};
use compio::fs::File;
use std::os::unix::io::AsRawFd;

/// Trait for fadvise operations
#[allow(async_fn_in_trait)]
pub trait Fadvise {
    /// Provide advice about file access patterns to the kernel
    ///
    /// This allows the kernel to optimize caching and I/O behavior based on
    /// the expected access pattern.
    ///
    /// # Arguments
    ///
    /// * `advice` - The advice to give (see `FadviseAdvice` constants)
    /// * `offset` - File offset to start the advice
    /// * `len` - Length of the region to apply advice to
    ///
    /// # Returns
    ///
    /// `Ok(())` if the advice was successfully applied
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The file descriptor is invalid
    /// - The advice is not supported
    /// - The operation fails due to I/O errors
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use compio_fs_extended::{ExtendedFile, Fadvise};
    /// use compio::fs::File;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let file = File::open("large_file.txt").await?;
    /// let extended_file = ExtendedFile::new(file);
    ///
    /// // Advise sequential access for better performance
    /// extended_file.fadvise(libc::POSIX_FADV_SEQUENTIAL, 0, 0).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn fadvise(&self, advice: i32, offset: u64, len: u64) -> Result<()>;
}

/// fadvise advice constants
pub mod advice {
    /// Data will be accessed sequentially
    pub const SEQUENTIAL: i32 = libc::POSIX_FADV_SEQUENTIAL;
    /// Data will be accessed randomly
    pub const RANDOM: i32 = libc::POSIX_FADV_RANDOM;
    /// Data will not be accessed again soon
    pub const DONTNEED: i32 = libc::POSIX_FADV_DONTNEED;
    /// Data will be accessed again soon
    pub const WILLNEED: i32 = libc::POSIX_FADV_WILLNEED;
    /// Data will not be accessed again
    pub const NOREUSE: i32 = libc::POSIX_FADV_NOREUSE;
}

/// Implementation of fadvise using direct syscalls
///
/// # Errors
///
/// This function will return an error if the fadvise operation fails
pub async fn fadvise_impl(file: &File, advice: i32, offset: u64, len: u64) -> Result<()> {
    let fd = file.as_raw_fd();

    let result = unsafe { libc::posix_fadvise(fd, offset as i64, len as i64, advice) };

    if result != 0 {
        return Err(fadvise_error(&format!(
            "posix_fadvise failed with error code: {}",
            result
        )));
    }

    Ok(())
}

/// Optimize file for sequential access
///
/// # Arguments
///
/// * `file` - The file to optimize
/// * `offset` - File offset to start optimization
/// * `len` - Length of the region to optimize
///
/// # Returns
///
/// `Ok(())` if the optimization was successfully applied
///
/// # Errors
///
/// This function will return an error if the fadvise operation fails
pub async fn optimize_for_sequential_access(file: &File, offset: u64, len: u64) -> Result<()> {
    fadvise_impl(file, advice::SEQUENTIAL, offset, len).await
}

/// Optimize file for random access
///
/// # Arguments
///
/// * `file` - The file to optimize
/// * `offset` - File offset to start optimization
/// * `len` - Length of the region to optimize
///
/// # Returns
///
/// `Ok(())` if the optimization was successfully applied
///
/// # Errors
///
/// This function will return an error if the fadvise operation fails
pub async fn optimize_for_random_access(file: &File, offset: u64, len: u64) -> Result<()> {
    fadvise_impl(file, advice::RANDOM, offset, len).await
}

/// Hint that data will not be needed again soon
///
/// # Arguments
///
/// * `file` - The file to hint about
/// * `offset` - File offset to start hinting
/// * `len` - Length of the region to hint about
///
/// # Returns
///
/// `Ok(())` if the hint was successfully applied
///
/// # Errors
///
/// This function will return an error if the fadvise operation fails
pub async fn hint_dont_need(file: &File, offset: u64, len: u64) -> Result<()> {
    fadvise_impl(file, advice::DONTNEED, offset, len).await
}

/// Hint that data will be needed soon
///
/// # Arguments
///
/// * `file` - The file to hint about
/// * `offset` - File offset to start hinting
/// * `len` - Length of the region to hint about
///
/// # Returns
///
/// `Ok(())` if the hint was successfully applied
///
/// # Errors
///
/// This function will return an error if the fadvise operation fails
pub async fn hint_will_need(file: &File, offset: u64, len: u64) -> Result<()> {
    fadvise_impl(file, advice::WILLNEED, offset, len).await
}

/// Hint that data will not be reused
///
/// # Arguments
///
/// * `file` - The file to hint about
/// * `offset` - File offset to start hinting
/// * `len` - Length of the region to hint about
///
/// # Returns
///
/// `Ok(())` if the hint was successfully applied
///
/// # Errors
///
/// This function will return an error if the fadvise operation fails
pub async fn hint_no_reuse(file: &File, offset: u64, len: u64) -> Result<()> {
    fadvise_impl(file, advice::NOREUSE, offset, len).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use compio::fs::File;
    use std::fs::write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_fadvise_sequential() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        write(&file_path, "test data").unwrap();

        // Open file
        let file = File::open(&file_path).await.unwrap();

        // Test fadvise
        let result = fadvise_impl(&file, advice::SEQUENTIAL, 0, 0).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fadvise_random() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        write(&file_path, "test data").unwrap();

        // Open file
        let file = File::open(&file_path).await.unwrap();

        // Test fadvise
        let result = fadvise_impl(&file, advice::RANDOM, 0, 0).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_optimize_for_sequential_access() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        write(&file_path, "test data").unwrap();

        // Open file
        let file = File::open(&file_path).await.unwrap();

        // Test optimization
        let result = optimize_for_sequential_access(&file, 0, 0).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_optimize_for_random_access() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        write(&file_path, "test data").unwrap();

        // Open file
        let file = File::open(&file_path).await.unwrap();

        // Test optimization
        let result = optimize_for_random_access(&file, 0, 0).await;
        assert!(result.is_ok());
    }
}

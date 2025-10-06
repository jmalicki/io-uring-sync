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
    async fn fadvise(&self, advice: FadviseAdvice, offset: u64, len: u64) -> Result<()>;
}

/// fadvise advice enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadviseAdvice {
    /// Normal access pattern (no special optimization)
    Normal,
    /// Data will be accessed sequentially
    Sequential,
    /// Data will be accessed randomly
    Random,
    /// Data will not be accessed again soon
    DontNeed,
    /// Data will be accessed again soon
    WillNeed,
    /// Data will not be accessed again
    NoReuse,
}

impl FadviseAdvice {
    /// Convert to the underlying libc constant
    pub fn to_libc(self) -> i32 {
        match self {
            FadviseAdvice::Normal => 0,
            FadviseAdvice::Sequential => libc::POSIX_FADV_SEQUENTIAL,
            FadviseAdvice::Random => libc::POSIX_FADV_RANDOM,
            FadviseAdvice::DontNeed => libc::POSIX_FADV_DONTNEED,
            FadviseAdvice::WillNeed => libc::POSIX_FADV_WILLNEED,
            FadviseAdvice::NoReuse => libc::POSIX_FADV_NOREUSE,
        }
    }
}

/// Public fadvise function for direct use
///
/// # Arguments
///
/// * `file` - The file to apply advice to
/// * `advice` - The advice to apply
/// * `offset` - File offset to start the advice
/// * `len` - Length of the region to apply advice to
///
/// # Returns
///
/// `Ok(())` if the advice was successfully applied
///
/// # Errors
///
/// This function will return an error if the fadvise operation fails
pub async fn fadvise(file: &File, advice: FadviseAdvice, offset: u64, len: u64) -> Result<()> {
    fadvise_impl(file, advice, offset, len).await
}

/// Implementation of fadvise using direct syscalls
///
/// # Errors
///
/// This function will return an error if the fadvise operation fails
async fn fadvise_impl(file: &File, advice: FadviseAdvice, offset: u64, len: u64) -> Result<()> {
    let fd = file.as_raw_fd();
    let advice_value = advice.to_libc();

    let result = unsafe { libc::posix_fadvise(fd, offset as i64, len as i64, advice_value) };

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
    fadvise_impl(file, FadviseAdvice::Sequential, offset, len).await
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
    fadvise_impl(file, FadviseAdvice::Random, offset, len).await
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
    fadvise_impl(file, FadviseAdvice::DontNeed, offset, len).await
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
    fadvise_impl(file, FadviseAdvice::WillNeed, offset, len).await
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
    fadvise_impl(file, FadviseAdvice::NoReuse, offset, len).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use compio::fs::File;
    use std::fs::write;
    use tempfile::TempDir;

    #[compio::test]
    async fn test_fadvise_sequential() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        write(&file_path, "test data").unwrap();

        // Open file
        let file = File::open(&file_path).await.unwrap();

        // Test fadvise
        let result = fadvise_impl(&file, FadviseAdvice::Sequential, 0, 0).await;
        assert!(result.is_ok());
    }

    #[compio::test]
    async fn test_fadvise_random() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        write(&file_path, "test data").unwrap();

        // Open file
        let file = File::open(&file_path).await.unwrap();

        // Test fadvise
        let result = fadvise_impl(&file, FadviseAdvice::Random, 0, 0).await;
        assert!(result.is_ok());
    }

    #[compio::test]
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

    #[compio::test]
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

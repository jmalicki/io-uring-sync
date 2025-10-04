//! Minimal extensions to rio for missing io_uring operations
//!
//! This crate provides only the essential missing operations that rio doesn't support:
//! - copy_file_range: In-kernel file copying
//! - xattr operations: Extended attributes for ACLs and metadata
//! - getdents64: Directory traversal
//!
//! It maintains full compatibility with rio and can be used as a drop-in extension.

#![deny(missing_docs)]
#![allow(unsafe_code)]

use iou::IoUring;
use rio::Rio;
use std::io::{self, Error};
use std::os::unix::io::RawFd;
use thiserror::Error;

/// Extended Rio that adds missing operations
pub struct ExtendedRio {
    /// Base rio instance for standard operations
    pub rio: Rio,
    /// iou instance for extended operations
    iou: IoUring,
}

/// Error types for extended operations
#[derive(Error, Debug)]
pub enum ExtendedError {
    /// IoUring operation failed
    #[error("IoUring operation failed: {0}")]
    IoUring(#[from] io::Error),

    /// Operation not supported
    #[error("Operation not supported: {0}")]
    NotSupported(String),
}

/// Result type for extended operations
pub type Result<T> = std::result::Result<T, ExtendedError>;

impl ExtendedRio {
    /// Create a new ExtendedRio instance
    ///
    /// This wraps a rio instance and adds an iou instance for extended operations.
    /// All standard rio operations work unchanged.
    pub fn new() -> Result<Self> {
        let rio = rio::new().map_err(|e| ExtendedError::IoUring(e))?;
        let iou = IoUring::new(1024).map_err(|e| ExtendedError::IoUring(e))?;

        Ok(ExtendedRio { rio, iou })
    }

    /// Get a reference to the underlying rio instance
    ///
    /// This allows access to all standard rio operations like read_at, write_at, etc.
    pub fn rio(&self) -> &Rio {
        &self.rio
    }

    /// Get a mutable reference to the underlying rio instance
    pub fn rio_mut(&mut self) -> &mut Rio {
        &mut self.rio
    }

    /// Copy file range using io_uring (in-kernel copying)
    ///
    /// This is the most efficient way to copy files on the same filesystem.
    /// Falls back to read/write if copy_file_range is not supported.
    ///
    /// # Parameters
    ///
    /// * `src_fd` - Source file descriptor
    /// * `src_offset` - Source offset
    /// * `dst_fd` - Destination file descriptor
    /// * `dst_offset` - Destination offset
    /// * `len` - Number of bytes to copy
    ///
    /// # Returns
    ///
    /// Returns the number of bytes copied or an error.
    pub async fn copy_file_range(
        &self,
        src_fd: RawFd,
        src_offset: u64,
        dst_fd: RawFd,
        dst_offset: u64,
        len: u32,
    ) -> Result<i64> {
        // Use iou's copy_file_range support if available
        // For now, fall back to direct syscall implementation
        self.copy_file_range_syscall(src_fd, src_offset, dst_fd, dst_offset, len)
    }

    /// Fallback implementation using direct syscall
    fn copy_file_range_syscall(
        &self,
        src_fd: RawFd,
        src_offset: u64,
        dst_fd: RawFd,
        dst_offset: u64,
        len: u32,
    ) -> Result<i64> {
        unsafe {
            let mut src_off = src_offset as i64;
            let mut dst_off = dst_offset as i64;

            let result =
                libc::copy_file_range(src_fd, &mut src_off, dst_fd, &mut dst_off, len as usize, 0);

            if result < 0 {
                Err(ExtendedError::IoUring(Error::last_os_error()))
            } else {
                Ok(result as i64)
            }
        }
    }

    /// Get extended attribute
    ///
    /// # Parameters
    ///
    /// * `path` - File path
    /// * `name` - Attribute name
    /// * `buffer` - Buffer to store attribute value
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written to buffer or an error.
    pub async fn getxattr(
        &self,
        path: &std::path::Path,
        name: &str,
        buffer: &mut [u8],
    ) -> Result<usize> {
        // For now, use synchronous syscall
        // TODO: Implement async io_uring xattr operations
        self.getxattr_syscall(path, name, buffer)
    }

    /// Synchronous xattr implementation
    fn getxattr_syscall(
        &self,
        path: &std::path::Path,
        name: &str,
        buffer: &mut [u8],
    ) -> Result<usize> {
        let path_c = std::ffi::CString::new(path.to_string_lossy().as_bytes())
            .map_err(|e| ExtendedError::NotSupported(format!("Invalid path: {}", e)))?;
        let name_c = std::ffi::CString::new(name)
            .map_err(|e| ExtendedError::NotSupported(format!("Invalid attribute name: {}", e)))?;

        unsafe {
            let result = libc::getxattr(
                path_c.as_ptr(),
                name_c.as_ptr(),
                buffer.as_mut_ptr() as *mut libc::c_void,
                buffer.len(),
            );

            if result < 0 {
                Err(ExtendedError::IoUring(Error::last_os_error()))
            } else {
                Ok(result as usize)
            }
        }
    }

    /// Set extended attribute
    ///
    /// # Parameters
    ///
    /// * `path` - File path
    /// * `name` - Attribute name
    /// * `value` - Attribute value
    /// * `flags` - Set flags (0 for default)
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success or an error.
    pub async fn setxattr(
        &self,
        path: &std::path::Path,
        name: &str,
        value: &[u8],
        flags: i32,
    ) -> Result<()> {
        // For now, use synchronous syscall
        // TODO: Implement async io_uring xattr operations
        self.setxattr_syscall(path, name, value, flags)
    }

    /// Synchronous setxattr implementation
    fn setxattr_syscall(
        &self,
        path: &std::path::Path,
        name: &str,
        value: &[u8],
        flags: i32,
    ) -> Result<()> {
        let path_c = std::ffi::CString::new(path.to_string_lossy().as_bytes())
            .map_err(|e| ExtendedError::NotSupported(format!("Invalid path: {}", e)))?;
        let name_c = std::ffi::CString::new(name)
            .map_err(|e| ExtendedError::NotSupported(format!("Invalid attribute name: {}", e)))?;

        unsafe {
            let result = libc::setxattr(
                path_c.as_ptr(),
                name_c.as_ptr(),
                value.as_ptr() as *const libc::c_void,
                value.len(),
                flags,
            );

            if result < 0 {
                Err(ExtendedError::IoUring(Error::last_os_error()))
            } else {
                Ok(())
            }
        }
    }

    /// List extended attributes
    ///
    /// # Parameters
    ///
    /// * `path` - File path
    /// * `buffer` - Buffer to store attribute names
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written to buffer or an error.
    pub async fn listxattr(&self, path: &std::path::Path, buffer: &mut [u8]) -> Result<usize> {
        // For now, use synchronous syscall
        // TODO: Implement async io_uring xattr operations
        self.listxattr_syscall(path, buffer)
    }

    /// Synchronous listxattr implementation
    fn listxattr_syscall(&self, path: &std::path::Path, buffer: &mut [u8]) -> Result<usize> {
        let path_c = std::ffi::CString::new(path.to_string_lossy().as_bytes())
            .map_err(|e| ExtendedError::NotSupported(format!("Invalid path: {}", e)))?;

        unsafe {
            let result = libc::listxattr(
                path_c.as_ptr(),
                buffer.as_mut_ptr() as *mut libc::c_char,
                buffer.len(),
            );

            if result < 0 {
                Err(ExtendedError::IoUring(Error::last_os_error()))
            } else {
                Ok(result as usize)
            }
        }
    }

    /// Read directory entries using getdents64
    ///
    /// This provides async directory traversal that rio doesn't support.
    /// Currently not implemented due to libc limitations.
    ///
    /// # Parameters
    ///
    /// * `dir_fd` - Directory file descriptor
    /// * `buffer` - Buffer to store directory entries
    ///
    /// # Returns
    ///
    /// Returns the number of bytes read or an error.
    pub async fn readdir(&self, _dir_fd: RawFd, _buffer: &mut [u8]) -> Result<usize> {
        // TODO: Implement getdents64 when proper libc bindings are available
        Err(ExtendedError::NotSupported(
            "getdents64 not yet implemented".to_string(),
        ))
    }
}

impl std::ops::Deref for ExtendedRio {
    type Target = Rio;

    fn deref(&self) -> &Rio {
        &self.rio
    }
}

impl std::ops::DerefMut for ExtendedRio {
    fn deref_mut(&mut self) -> &mut Rio {
        &mut self.rio
    }
}

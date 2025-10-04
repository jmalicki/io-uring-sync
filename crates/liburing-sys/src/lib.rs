//! Low-level FFI bindings for liburing
//!
//! This crate provides safe Rust bindings for the liburing C library,
//! which provides a high-level interface to Linux's io_uring system.
//! This is more low-level than the `rio` crate and gives us direct
//! access to copy_file_range and other io_uring operations.
//!
//! # Features
//!
//! - Direct access to io_uring operations including copy_file_range
//! - Safe Rust wrappers around liburing C API
//! - Support for splice, read, write, and other io_uring operations
//! - Proper error handling and resource management
//!
//! # Usage
//!
//! ```rust
//! use liburing_sys::IoUring;
//!
//! let mut ring = IoUring::new(1024)?;
//! // Use ring for io_uring operations
//! ```

#![deny(missing_docs)]
#![deny(unsafe_code)]

use libc::{c_int, c_void, size_t};
use std::ffi::CString;
use std::io::{self, Error, ErrorKind};
use std::os::unix::io::RawFd;

mod bindings;

/// Result type for io_uring operations
pub type Result<T> = std::result::Result<T, io::Error>;

/// io_uring submission queue entry
#[repr(C)]
pub struct IoUringSqe {
    // This is opaque - we'll use bindings for actual access
    _private: [u8; 0],
}

/// io_uring completion queue entry
#[repr(C)]
pub struct IoUringCqe {
    // This is opaque - we'll use bindings for actual access
    _private: [u8; 0],
}

/// Main io_uring interface
pub struct IoUring {
    ring_fd: c_int,
    sq_entries: u32,
    cq_entries: u32,
}

impl IoUring {
    /// Create a new io_uring instance
    ///
    /// # Parameters
    ///
    /// * `entries` - Size of submission and completion queues
    ///
    /// # Returns
    ///
    /// Returns a new IoUring instance or an error if initialization fails.
    pub fn new(entries: u32) -> Result<Self> {
        let ring_fd = unsafe { bindings::io_uring_setup(entries, std::ptr::null_mut()) };
        
        if ring_fd < 0 {
            return Err(Error::last_os_error());
        }

        Ok(IoUring {
            ring_fd,
            sq_entries: entries,
            cq_entries: entries,
        })
    }

    /// Submit a copy_file_range operation
    ///
    /// # Parameters
    ///
    /// * `src_fd` - Source file descriptor
    /// * `src_offset` - Source offset
    /// * `dst_fd` - Destination file descriptor  
    /// * `dst_offset` - Destination offset
    /// * `len` - Number of bytes to copy
    /// * `flags` - Copy flags (0 for default)
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
        flags: u32,
    ) -> Result<i64> {
        // Get submission queue entry
        let sqe = unsafe { bindings::io_uring_get_sqe(self.ring_fd) };
        if sqe.is_null() {
            return Err(Error::new(ErrorKind::Other, "Failed to get submission queue entry"));
        }

        // Prepare copy_file_range operation
        unsafe {
            bindings::io_uring_prep_copy_file_range(
                sqe,
                src_fd,
                src_offset as i64,
                dst_fd,
                dst_offset as i64,
                len,
                flags,
            );
        }

        // Submit the operation
        let submitted = unsafe { bindings::io_uring_submit(self.ring_fd) };
        if submitted < 0 {
            return Err(Error::last_os_error());
        }

        // Wait for completion
        let mut cqe: *mut bindings::io_uring_cqe = std::ptr::null_mut();
        let ret = unsafe { bindings::io_uring_wait_cqe(self.ring_fd, &mut cqe) };
        
        if ret < 0 {
            return Err(Error::last_os_error());
        }

        let result = unsafe { (*cqe).res };
        unsafe { bindings::io_uring_cqe_seen(self.ring_fd, cqe) };

        if result < 0 {
            Err(Error::from_raw_os_error(-result))
        } else {
            Ok(result)
        }
    }

    /// Submit a splice operation
    ///
    /// # Parameters
    ///
    /// * `fd_in` - Input file descriptor
    /// * `off_in` - Input offset
    /// * `fd_out` - Output file descriptor
    /// * `off_out` - Output offset
    /// * `len` - Number of bytes to splice
    /// * `splice_flags` - Splice flags
    ///
    /// # Returns
    ///
    /// Returns the number of bytes spliced or an error.
    pub async fn splice(
        &self,
        fd_in: RawFd,
        off_in: u64,
        fd_out: RawFd,
        off_out: u64,
        len: u32,
        splice_flags: u32,
    ) -> Result<i64> {
        // Get submission queue entry
        let sqe = unsafe { bindings::io_uring_get_sqe(self.ring_fd) };
        if sqe.is_null() {
            return Err(Error::new(ErrorKind::Other, "Failed to get submission queue entry"));
        }

        // Prepare splice operation
        unsafe {
            bindings::io_uring_prep_splice(
                sqe,
                fd_in,
                off_in as i64,
                fd_out,
                off_out as i64,
                len,
                splice_flags,
            );
        }

        // Submit the operation
        let submitted = unsafe { bindings::io_uring_submit(self.ring_fd) };
        if submitted < 0 {
            return Err(Error::last_os_error());
        }

        // Wait for completion
        let mut cqe: *mut bindings::io_uring_cqe = std::ptr::null_mut();
        let ret = unsafe { bindings::io_uring_wait_cqe(self.ring_fd, &mut cqe) };
        
        if ret < 0 {
            return Err(Error::last_os_error());
        }

        let result = unsafe { (*cqe).res };
        unsafe { bindings::io_uring_cqe_seen(self.ring_fd, cqe) };

        if result < 0 {
            Err(Error::from_raw_os_error(-result))
        } else {
            Ok(result)
        }
    }
}

impl Drop for IoUring {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.ring_fd);
        }
    }
}

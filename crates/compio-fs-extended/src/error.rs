//! Error types for compio-fs-extended operations

use thiserror::Error;

/// Result type for compio-fs-extended operations
pub type Result<T> = std::result::Result<T, ExtendedError>;

/// Extended error types for filesystem operations
#[derive(Error, Debug)]
pub enum ExtendedError {
    /// Standard I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// copy_file_range specific error
    #[error("copy_file_range failed: {0}")]
    CopyFileRange(String),

    /// fadvise specific error
    #[error("fadvise failed: {0}")]
    Fadvise(String),

    /// fallocate specific error
    #[error("fallocate failed: {0}")]
    Fallocate(String),

    /// Symlink operation error
    #[error("symlink operation failed: {0}")]
    Symlink(String),

    /// Hardlink operation error
    #[error("hardlink operation failed: {0}")]
    Hardlink(String),

    /// Directory operation error
    #[error("directory operation failed: {0}")]
    Directory(String),

    /// Extended attributes error
    #[error("xattr operation failed: {0}")]
    Xattr(String),

    /// Filesystem detection error
    #[error("filesystem detection failed: {0}")]
    FilesystemDetection(String),

    /// Operation not supported
    #[error("operation not supported: {0}")]
    NotSupported(String),

    /// Invalid parameters
    #[error("invalid parameters: {0}")]
    InvalidParameters(String),

    /// System call error
    #[error("system call failed: {0}")]
    SystemCall(String),
}

impl ExtendedError {
    /// Check if error is due to operation not being supported
    pub fn is_not_supported(&self) -> bool {
        matches!(self, ExtendedError::NotSupported(_))
    }

    /// Check if error is due to invalid parameters
    pub fn is_invalid_parameters(&self) -> bool {
        matches!(self, ExtendedError::InvalidParameters(_))
    }

    /// Check if error is due to system call failure
    pub fn is_system_call_error(&self) -> bool {
        matches!(self, ExtendedError::SystemCall(_))
    }
}

/// Helper trait for converting system call results to ExtendedError
pub trait SyscallResult<T> {
    /// Convert system call result to ExtendedError
    fn into_extended_error(self, operation: &str) -> Result<T>;
}

impl<T> SyscallResult<T> for std::result::Result<T, std::io::Error> {
    fn into_extended_error(self, operation: &str) -> Result<T> {
        self.map_err(|e| ExtendedError::SystemCall(format!("{}: {}", operation, e)))
    }
}

/// Helper for creating copy_file_range specific errors
pub fn copy_file_range_error(msg: &str) -> ExtendedError {
    ExtendedError::CopyFileRange(msg.to_string())
}

/// Helper for creating fadvise specific errors
pub fn fadvise_error(msg: &str) -> ExtendedError {
    ExtendedError::Fadvise(msg.to_string())
}

/// Helper for creating fallocate specific errors
pub fn fallocate_error(msg: &str) -> ExtendedError {
    ExtendedError::Fallocate(msg.to_string())
}

/// Helper for creating symlink specific errors
pub fn symlink_error(msg: &str) -> ExtendedError {
    ExtendedError::Symlink(msg.to_string())
}

/// Helper for creating hardlink specific errors
pub fn hardlink_error(msg: &str) -> ExtendedError {
    ExtendedError::Hardlink(msg.to_string())
}

/// Helper for creating directory specific errors
pub fn directory_error(msg: &str) -> ExtendedError {
    ExtendedError::Directory(msg.to_string())
}

/// Helper for creating xattr specific errors
pub fn xattr_error(msg: &str) -> ExtendedError {
    ExtendedError::Xattr(msg.to_string())
}

/// Helper for creating filesystem detection errors
pub fn filesystem_detection_error(msg: &str) -> ExtendedError {
    ExtendedError::FilesystemDetection(msg.to_string())
}

/// Helper for creating not supported errors
pub fn not_supported_error(msg: &str) -> ExtendedError {
    ExtendedError::NotSupported(msg.to_string())
}

/// Helper for creating invalid parameters errors
pub fn invalid_parameters_error(msg: &str) -> ExtendedError {
    ExtendedError::InvalidParameters(msg.to_string())
}

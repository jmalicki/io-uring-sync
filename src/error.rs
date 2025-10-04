//! Error handling and types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("io_uring error: {0}")]
    IoUring(String),

    #[error("Copy operation failed: {0}")]
    CopyFailed(String),

    #[error("Metadata operation failed: {0}")]
    MetadataFailed(String),

    #[error("Directory traversal failed: {0}")]
    DirectoryTraversal(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, SyncError>;

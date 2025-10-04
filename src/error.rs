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
    #[allow(dead_code)]
    MetadataFailed(String),

    #[error("Directory traversal failed: {0}")]
    #[allow(dead_code)]
    DirectoryTraversal(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Permission denied: {0}")]
    #[allow(dead_code)]
    PermissionDenied(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Internal error: {0}")]
    #[allow(dead_code)]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, SyncError>;

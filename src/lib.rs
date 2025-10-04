//! io-uring-sync: High-performance file copying using io_uring
//!
//! This library provides efficient file copying capabilities using Linux's io_uring
//! interface for asynchronous I/O operations, similar to rsync but optimized for
//! single-machine operations with parallelism and metadata preservation.

pub mod cli;
pub mod copy;
pub mod directory;
pub mod error;
pub mod io_uring;
pub mod progress;
pub mod sync;

// Re-export commonly used types
pub use directory::{analyze_filesystem_structure, ExtendedMetadata, FilesystemTracker};
pub use error::{Result, SyncError};
pub use progress::ProgressTracker;

//! io-uring-sync: High-performance file copying using `io_uring`
//!
//! This library provides efficient file copying capabilities using Linux's `io_uring`
//! interface for asynchronous I/O operations, similar to rsync but optimized for
//! single-machine operations with parallelism and metadata preservation.
//!
//! ## Key Features
//!
//! - **Integrated Hardlink Detection**: Smart hardlink detection during traversal
//!   - Content is copied only once per unique inode
//!   - Subsequent files with the same inode become hardlinks
//!   - Uses `io_uring statx` for efficient metadata analysis
//!
//! - **Single-Pass Operation**: Efficient traversal that discovers and copies in one pass
//!   - No separate analysis phase required
//!   - Progress tracking shows both discovery and completion progress
//!   - Filesystem boundary detection integrated into traversal
//!
//! - **`io_uring` Throughout**: All operations use `io_uring` for maximum performance
//!   - `statx` for metadata discovery
//!   - `copy_file_range` for file copying
//!   - `linkat` for hardlink creation
//!   - `symlinkat`/`readlinkat` for symlink handling
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use io_uring_sync::directory::copy_directory;
//! use io_uring_sync::io_uring::FileOperations;
//! use io_uring_sync::cli::CopyMethod;
//!
//! #[compio::main]
//! async fn main() -> io_uring_sync::Result<()> {
//!     let file_ops = FileOperations::new(4096, 64 * 1024)?;
//!     let stats = copy_directory(
//!         &std::path::Path::new("/source"),
//!         &std::path::Path::new("/destination"),
//!         &file_ops,
//!         CopyMethod::Auto,
//!     ).await?;
//!     
//!     println!("Copied {} files, {} directories, {} bytes",
//!              stats.files_copied, stats.directories_created, stats.bytes_copied);
//!     Ok(())
//! }
//! ```

pub mod cli;
pub mod copy;
pub mod directory;
pub mod error;
pub mod integration;
pub mod io_uring;
pub mod progress;
pub mod sync;

// Re-export commonly used types
pub use directory::FilesystemTracker;
pub use error::{Result, SyncError};
pub use progress::ProgressTracker;

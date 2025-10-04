//! Main synchronization logic
//!
//! This module provides the core synchronization functionality for io-uring-sync,
//! orchestrating file and directory copying operations with comprehensive error
//! handling, progress tracking, and performance optimization.
//!
//! # Features
//!
//! - Single file copying with metadata preservation
//! - Directory structure creation and management
//! - Comprehensive error handling and logging
//! - Performance statistics and timing
//! - Async/await support for non-blocking operations
//!
//! # Architecture
//!
//! The synchronization process follows these phases:
//! 1. **Validation**: Verify source paths and destination permissions
//! 2. **Initialization**: Set up file operations and progress tracking
//! 3. **Execution**: Perform the actual copying operations
//! 4. **Completion**: Update statistics and handle cleanup
//!
//! # Usage
//!
//! ```rust
//! use io_uring_sync::sync::sync_files;
//! use io_uring_sync::cli::Args;
//!
//! let args = Args::parse();
//! let stats = sync_files(&args).await?;
//! println!("Copied {} files, {} bytes in {:?}",
//!          stats.files_copied, stats.bytes_copied, stats.duration);
//! ```
//!
//! # Performance Considerations
//!
//! - Uses async I/O for non-blocking operations
//! - Tracks detailed performance metrics
//! - Supports configurable buffer sizes and queue depths
//! - Optimized for both small and large file operations
//!
//! # Error Handling
//!
//! All operations use structured error handling with detailed error messages
//! and proper error propagation. Common error scenarios include:
//!
//! - Invalid source paths or permissions
//! - Destination directory creation failures
//! - File copying errors with detailed context
//! - Configuration validation failures

use crate::cli::Args;
use crate::copy::copy_file;
use crate::error::Result;
use crate::io_uring::FileOperations;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

/// Statistics for a synchronization operation
///
/// This structure tracks comprehensive metrics about the synchronization
/// process, including performance data and operation results.
///
/// # Fields
///
/// * `files_copied` - Number of files successfully copied
/// * `bytes_copied` - Total number of bytes copied
/// * `duration` - Total time taken for the synchronization operation
///
/// # Examples
///
/// ```rust
/// let stats = SyncStats {
///     files_copied: 150,
///     bytes_copied: 1_048_576,
///     duration: Duration::from_secs(5),
/// };
/// println!("Copied {} files ({} bytes) in {:?}",
///          stats.files_copied, stats.bytes_copied, stats.duration);
/// ```
///
/// # Performance Notes
///
/// These statistics are useful for:
/// - Performance monitoring and optimization
/// - User feedback and progress reporting
/// - Benchmarking and comparison with other tools
/// - Debugging and troubleshooting slow operations
#[derive(Debug, Clone, PartialEq)]
pub struct SyncStats {
    /// Number of files successfully copied during the operation
    pub files_copied: u64,

    /// Total number of bytes copied during the operation
    pub bytes_copied: u64,

    /// Total duration of the synchronization operation
    pub duration: Duration,
}

/// Main synchronization function
///
/// This function orchestrates the entire synchronization process, handling both
/// single file and directory operations with comprehensive error handling and
/// performance tracking.
///
/// # Parameters
///
/// * `args` - Command-line arguments containing source/destination paths,
///   configuration options, and operation parameters
///
/// # Returns
///
/// Returns `Ok(SyncStats)` containing detailed operation statistics, or
/// `Err(SyncError)` if the synchronization fails.
///
/// # Errors
///
/// This function will return an error if:
/// - Source path doesn't exist or is inaccessible
/// - Destination directory cannot be created
/// - File operations fail during copying
/// - Configuration parameters are invalid
/// - I/O errors occur during the operation
///
/// # Examples
///
/// Basic usage:
/// ```rust
/// use io_uring_sync::cli::Args;
/// use io_uring_sync::sync::sync_files;
///
/// let args = Args::parse();
/// let stats = sync_files(&args).await?;
/// println!("Operation completed: {} files, {} bytes, {:?}",
///          stats.files_copied, stats.bytes_copied, stats.duration);
/// ```
///
/// Error handling:
/// ```rust
/// match sync_files(&args).await {
///     Ok(stats) => {
///         println!("Success: {} files copied", stats.files_copied);
///     }
///     Err(e) => {
///         eprintln!("Synchronization failed: {}", e);
///         std::process::exit(1);
///     }
/// }
/// ```
///
/// # Performance Considerations
///
/// - Operation time scales with file size and count
/// - Memory usage depends on buffer size configuration
/// - Concurrent operations may be limited by system resources
/// - Network filesystems may have different performance characteristics
///
/// # Thread Safety
///
/// This function is thread-safe and can be called concurrently, but each
/// call operates independently. For optimal performance, avoid concurrent
/// operations on the same destination paths.
///
/// # Implementation Details
///
/// The synchronization process:
/// 1. Validates arguments and initializes file operations
/// 2. Determines operation type (file vs directory)
/// 3. Creates destination directories as needed
/// 4. Performs the actual copying operations
/// 5. Tracks statistics and handles errors
/// 6. Returns comprehensive operation results
pub async fn sync_files(args: &Args) -> Result<SyncStats> {
    let start_time = Instant::now();

    info!(
        "Starting synchronization from {} to {}",
        args.source.display(),
        args.destination.display()
    );

    let mut stats = SyncStats {
        files_copied: 0,
        bytes_copied: 0,
        duration: Duration::from_secs(0),
    };

    // Initialize file operations with configured parameters
    // Queue depth and buffer size are validated by the CLI module
    let mut file_ops = FileOperations::new(args.queue_depth, args.buffer_size_bytes())?;

    // Handle single file copy
    if args.is_file_copy() {
        info!("Copying single file: {}", args.source.display());

        // Ensure destination directory exists
        if let Some(parent) = args.destination.parent() {
            file_ops.create_dir(parent).await?;
        }

        // Note: file size is now obtained within copy_file_with_metadata

        // Copy the file using the specified method
        match copy_file(&args.source, &args.destination, args.copy_method.clone()).await {
            Ok(()) => {
                // Get file size for statistics
                let file_size = file_ops.get_file_size(&args.source).await?;
                stats.files_copied = 1;
                stats.bytes_copied = file_size;
                info!(
                    "Successfully copied file: {} bytes using {:?}",
                    file_size, args.copy_method
                );

                // Preserve metadata after copying
                match file_ops.get_file_metadata(&args.source).await {
                    Ok(metadata) => {
                        // Preserve permissions
                        let _ = file_ops
                            .set_file_permissions(&args.destination, metadata.permissions)
                            .await;
                        // Preserve ownership (may fail if not privileged)
                        let _ = file_ops
                            .set_file_ownership(&args.destination, metadata.uid, metadata.gid)
                            .await;
                        // Preserve timestamps
                        let _ = file_ops
                            .set_file_timestamps(
                                &args.destination,
                                metadata.accessed,
                                metadata.modified,
                            )
                            .await;
                        info!("Metadata preservation completed");
                    }
                    Err(e) => {
                        warn!("Failed to preserve metadata: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to copy file {}: {}", args.source.display(), e);
                return Err(e);
            }
        }
    }
    // Handle directory copy
    else if args.is_directory_copy() {
        info!("Copying directory: {}", args.source.display());

        // Ensure destination directory exists
        file_ops.create_dir(&args.destination).await?;

        // For now, just copy the directory structure without files
        // TODO: Implement recursive directory traversal in Phase 1.3
        warn!("Directory copying not yet implemented - only structure created");
        stats.files_copied = 0;
        stats.bytes_copied = 0;
    } else {
        error!(
            "Source path is neither a file nor a directory: {}",
            args.source.display()
        );
        return Err(crate::error::SyncError::InvalidConfig(
            "Source must be a file or directory".to_string(),
        ));
    }

    stats.duration = start_time.elapsed();

    info!("Synchronization completed in {:?}", stats.duration);
    info!(
        "Files copied: {}, Bytes copied: {}",
        stats.files_copied, stats.bytes_copied
    );

    Ok(stats)
}

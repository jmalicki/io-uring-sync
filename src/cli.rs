//! Command-line interface definitions

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// High-performance bulk file copying utility using `io_uring`
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Source directory or file
    #[arg(short, long)]
    pub source: PathBuf,

    /// Destination directory or file
    #[arg(short, long)]
    pub destination: PathBuf,

    /// Queue depth for `io_uring` operations
    #[arg(long, default_value = "4096")]
    pub queue_depth: usize,

    /// Maximum files in flight per CPU core
    #[arg(long, default_value = "1024")]
    pub max_files_in_flight: usize,

    /// Number of CPU cores to use (0 = auto-detect)
    #[arg(long, default_value = "0")]
    pub cpu_count: usize,

    /// Buffer size in KB (0 = auto-detect)
    #[arg(long, default_value = "0")]
    pub buffer_size_kb: usize,

    /// Copy method to use
    #[arg(long, default_value = "auto")]
    pub copy_method: CopyMethod,

    /// Preserve extended attributes
    #[arg(long)]
    pub preserve_xattr: bool,

    /// Preserve POSIX ACLs
    #[arg(long)]
    pub preserve_acl: bool,

    /// Show what would be copied without actually copying
    #[arg(long)]
    pub dry_run: bool,

    /// Show progress information
    #[arg(long)]
    pub progress: bool,

    /// Verbose output (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Quiet mode (suppress all output except errors)
    #[arg(short, long)]
    pub quiet: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CopyMethod {
    /// Automatically choose the best method
    Auto,
    /// Use `copy_file_range` for same-filesystem copies
    CopyFileRange,
    /// Use splice for zero-copy operations
    Splice,
    /// Use traditional read/write operations
    ReadWrite,
}

impl Args {
    /// Validate command-line arguments
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Source path does not exist
    /// - Source path is not a file or directory
    /// - Queue depth is outside valid bounds (1024-65536)
    /// - Max files in flight is outside valid bounds (1-10000)
    /// - Buffer size is too large (>1GB)
    /// - No CPU cores are available
    /// - Both --quiet and --verbose options are used
    pub fn validate(&self) -> Result<()> {
        // Check if source exists
        if !self.source.exists() {
            anyhow::bail!("Source path does not exist: {}", self.source.display());
        }

        // Check if source is readable
        if !self.source.is_dir() && !self.source.is_file() {
            anyhow::bail!(
                "Source path must be a file or directory: {}",
                self.source.display()
            );
        }

        // Check queue depth bounds
        if self.queue_depth < 1024 || self.queue_depth > 65_536 {
            anyhow::bail!(
                "Queue depth must be between 1024 and 65536, got: {}",
                self.queue_depth
            );
        }

        // Check max files in flight bounds
        if self.max_files_in_flight < 1 || self.max_files_in_flight > 10_000 {
            anyhow::bail!(
                "Max files in flight must be between 1 and 10000, got: {}",
                self.max_files_in_flight
            );
        }

        // Validate buffer size
        if self.buffer_size_kb > 1024 * 1024 {
            anyhow::bail!(
                "Buffer size too large (max 1GB): {} KB",
                self.buffer_size_kb
            );
        }

        // Check CPU count bounds
        let effective_cpu_count = self.effective_cpu_count();
        if effective_cpu_count == 0 {
            anyhow::bail!("No CPU cores available");
        }

        // Validate conflicting options
        if self.quiet && self.verbose > 0 {
            anyhow::bail!("Cannot use both --quiet and --verbose options");
        }

        Ok(())
    }

    /// Get the actual CPU count to use
    #[must_use]
    pub fn effective_cpu_count(&self) -> usize {
        if self.cpu_count == 0 {
            num_cpus::get()
        } else {
            self.cpu_count
        }
    }

    /// Get the actual buffer size in bytes
    #[allow(dead_code)]
    #[must_use]
    pub const fn effective_buffer_size(&self) -> usize {
        if self.buffer_size_kb == 0 {
            // Default to 64KB for now
            64 * 1024
        } else {
            self.buffer_size_kb * 1024
        }
    }

    /// Check if the source is a directory
    #[must_use]
    pub fn is_directory_copy(&self) -> bool {
        self.source.is_dir()
    }

    /// Check if the source is a single file
    #[must_use]
    pub fn is_file_copy(&self) -> bool {
        self.source.is_file()
    }

    /// Get buffer size in bytes
    #[must_use]
    pub const fn buffer_size_bytes(&self) -> usize {
        self.buffer_size_kb * 1024
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::error::SyncError;
    use compio::fs::File;
    use tempfile::TempDir;

    async fn create_temp_file() -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()
            .map_err(|e| SyncError::FileSystem(format!("Failed to create temp directory: {e}")))?;
        let file_path = temp_dir.path().join("test_file.txt");
        File::create(&file_path)
            .await
            .map_err(|e| SyncError::FileSystem(format!("Failed to create test file: {e}")))?;
        Ok((temp_dir, file_path))
    }

    async fn create_temp_dir() -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()
            .map_err(|e| SyncError::FileSystem(format!("Failed to create temp directory: {e}")))?;
        let sub_dir = temp_dir.path().join("test_dir");
        compio::fs::create_dir(&sub_dir)
            .await
            .map_err(|e| SyncError::FileSystem(format!("Failed to create test directory: {e}")))?;
        Ok((temp_dir, sub_dir))
    }

    #[compio::test]
    async fn test_validate_with_existing_file() {
        let (temp_dir, file_path) = create_temp_file().await.unwrap();
        let args = Args {
            source: file_path,
            destination: temp_dir.path().join("dest"),
            copy_method: CopyMethod::Auto,
            queue_depth: 4096,
            cpu_count: 2,
            buffer_size_kb: 1024,
            max_files_in_flight: 100,
            preserve_xattr: true,
            preserve_acl: false,
            dry_run: false,
            progress: false,
            verbose: 0,
            quiet: false,
        };

        assert!(args.validate().is_ok());
    }

    #[compio::test]
    async fn test_validate_with_existing_directory() {
        let (temp_dir, dir_path) = create_temp_dir().await.unwrap();
        let args = Args {
            source: dir_path,
            destination: temp_dir.path().join("dest"),
            copy_method: CopyMethod::Auto,
            queue_depth: 4096,
            cpu_count: 2,
            buffer_size_kb: 1024,
            max_files_in_flight: 100,
            preserve_xattr: true,
            preserve_acl: false,
            dry_run: false,
            progress: false,
            verbose: 0,
            quiet: false,
        };

        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_with_nonexistent_source() {
        let args = Args {
            source: PathBuf::from("/nonexistent/path"),
            destination: PathBuf::from("/tmp/dest"),
            copy_method: CopyMethod::Auto,
            queue_depth: 4096,
            cpu_count: 2,
            buffer_size_kb: 1024,
            max_files_in_flight: 100,
            preserve_xattr: true,
            preserve_acl: false,
            dry_run: false,
            progress: false,
            verbose: 0,
            quiet: false,
        };

        assert!(args.validate().is_err());
    }
}

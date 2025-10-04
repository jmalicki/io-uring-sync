//! Command-line interface definitions

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

/// High-performance bulk file copying utility using io_uring
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Source directory or file
    #[arg(short, long)]
    pub source: PathBuf,

    /// Destination directory or file
    #[arg(short, long)]
    pub destination: PathBuf,

    /// Queue depth for io_uring operations
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
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CopyMethod {
    /// Automatically choose the best method
    Auto,
    /// Use copy_file_range for same-filesystem copies
    CopyFileRange,
    /// Use splice for zero-copy operations
    Splice,
    /// Use traditional read/write operations
    ReadWrite,
}

impl Args {
    /// Validate command-line arguments
    pub fn validate(&self) -> Result<()> {
        // Check if source exists
        if !self.source.exists() {
            anyhow::bail!("Source path does not exist: {}", self.source.display());
        }

        // Check queue depth bounds
        if self.queue_depth < 1024 || self.queue_depth > 65536 {
            anyhow::bail!(
                "Queue depth must be between 1024 and 65536, got: {}",
                self.queue_depth
            );
        }

        // Check max files in flight bounds
        if self.max_files_in_flight < 1 || self.max_files_in_flight > 10000 {
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

        Ok(())
    }

    /// Get the actual CPU count to use
    pub fn effective_cpu_count(&self) -> usize {
        if self.cpu_count == 0 {
            num_cpus::get()
        } else {
            self.cpu_count
        }
    }

    /// Get the actual buffer size in bytes
    pub fn effective_buffer_size(&self) -> usize {
        if self.buffer_size_kb == 0 {
            // Auto-detect based on system memory and filesystem
            // Default to 64KB for now
            64 * 1024
        } else {
            self.buffer_size_kb * 1024
        }
    }
}

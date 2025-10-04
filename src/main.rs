//! io-uring-sync: High-performance bulk file copying utility
//!
//! This utility provides rsync-like functionality optimized for single-machine operations
//! using io_uring for maximum performance and parallelism.

use anyhow::Result;
use clap::Parser;
use tracing::{info, Level};

mod cli;
mod copy;
mod error;
mod progress;
mod sync;

use cli::Args;
use error::SyncError;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let args = Args::parse();
    
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(match args.verbose {
            0 => Level::WARN,
            1 => Level::INFO,
            2 => Level::DEBUG,
            _ => Level::TRACE,
        })
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("Starting io-uring-sync v{}", env!("CARGO_PKG_VERSION"));
    info!("Source: {}", args.source.display());
    info!("Destination: {}", args.destination.display());
    
    // Validate arguments
    args.validate()?;
    
    // Perform the sync operation
    let result = sync::sync_files(&args).await;
    
    match result {
        Ok(stats) => {
            info!("Sync completed successfully");
            info!("Files copied: {}", stats.files_copied);
            info!("Bytes copied: {}", stats.bytes_copied);
            info!("Duration: {:?}", stats.duration);
            Ok(())
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

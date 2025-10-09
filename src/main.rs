//! arsync: High-performance bulk file copying utility
//!
//! This utility provides rsync-like functionality optimized for single-machine operations
//! using `io_uring` for maximum performance and parallelism.

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{info, warn, Level};

mod adaptive_concurrency;
mod cli;
mod copy;
mod directory;
mod error;
mod io_uring;
mod progress;
mod sync;

// Remote sync protocol (feature-gated for now)
#[cfg(feature = "remote-sync")]
mod protocol;

use cli::{Args, Location};

#[compio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logging based on verbosity and quiet mode
    if args.quiet {
        // In quiet mode, only log errors
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(Level::ERROR)
            .with_target(false)
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
    } else {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(match args.verbose {
                0 => Level::WARN,
                1 => Level::INFO,
                2 => Level::DEBUG,
                _ => Level::TRACE,
            })
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
    }

    // Get source and destination
    let source = args.get_source().context("Failed to parse source")?;
    let destination = args
        .get_destination()
        .context("Failed to parse destination")?;

    // Log startup information (unless in quiet mode)
    if !args.quiet {
        info!("Starting arsync v{}", env!("CARGO_PKG_VERSION"));
        match &source {
            Location::Local(path) => info!("Source: {} (local)", path.display()),
            Location::Remote { user, host, path } => {
                if let Some(u) = user {
                    info!("Source: {}@{}:{} (remote)", u, host, path.display());
                } else {
                    info!("Source: {}:{} (remote)", host, path.display());
                }
            }
        }
        match &destination {
            Location::Local(path) => info!("Destination: {} (local)", path.display()),
            Location::Remote { user, host, path } => {
                if let Some(u) = user {
                    info!("Destination: {}@{}:{} (remote)", u, host, path.display());
                } else {
                    info!("Destination: {}:{} (remote)", host, path.display());
                }
            }
        }
        info!("Copy method: {:?}", args.copy_method);
        info!("Queue depth: {}", args.queue_depth);
        info!("CPU count: {}", args.effective_cpu_count());
        info!("Buffer size: {} KB", args.buffer_size_kb);
        info!("Max files in flight: {}", args.max_files_in_flight);
    }

    // Validate arguments
    args.validate().context("Invalid arguments")?;

    // Route to appropriate mode
    let result = if source.is_remote() || destination.is_remote() {
        // Remote sync mode
        #[cfg(feature = "remote-sync")]
        {
            protocol::remote_sync(&args, &source, &destination).await
        }
        #[cfg(not(feature = "remote-sync"))]
        {
            warn!("Remote sync support not compiled in");
            warn!("To enable remote sync, compile with: cargo build --features remote-sync");
            anyhow::bail!("Remote sync not supported in this build")
        }
    } else {
        // Local sync mode
        sync::sync_files(&args).await
    };

    match result {
        Ok(stats) => {
            info!("Sync completed successfully");
            info!("Files copied: {}", stats.files_copied);
            info!("Bytes copied: {}", stats.bytes_copied);
            info!("Duration: {:?}", stats.duration);
            Ok(())
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

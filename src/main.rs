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

#[compio::main(unwind_safe)]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logging based on verbosity and quiet mode
    // Note: In pipe mode, logs go to stderr (FD 2) so they don't interfere with protocol (FD 0/1)
    if args.pipe || args.quiet {
        // In quiet or pipe mode, only log errors (to stderr)
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(Level::ERROR)
            .with_target(false)
            .with_writer(std::io::stderr) // Explicit stderr
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
    let result = if args.pipe {
        // ============================================================
        // PIPE MODE (TESTING ONLY)
        // ============================================================
        // Uses rsync wire protocol over stdin/stdout
        // FOR PROTOCOL TESTING, NOT PRODUCTION USE
        // Local copies should use io_uring direct operations!
        // ============================================================
        #[cfg(feature = "remote-sync")]
        {
            match args.pipe_role {
                Some(cli::PipeRole::Sender) => {
                    info!("Pipe mode: sender");
                    protocol::pipe_sender(&args, &source).await
                }
                Some(cli::PipeRole::Receiver) => {
                    info!("Pipe mode: receiver");
                    protocol::pipe_receiver(&args, &destination).await
                }
                None => {
                    anyhow::bail!("--pipe requires --pipe-role (sender or receiver)")
                }
            }
        }
        #[cfg(not(feature = "remote-sync"))]
        {
            anyhow::bail!(
                "--pipe requires remote-sync feature (compile with --features remote-sync)"
            )
        }
    } else if source.is_remote() || destination.is_remote() {
        // Remote sync mode
        #[cfg(feature = "remote-sync")]
        {
            protocol::remote_sync(&args, &source, &destination).await
        }
        #[cfg(not(feature = "remote-sync"))]
        {
            warn!("Remote sync support not compiled in");
            warn!("To enable remote sync, compile with: cargo build --features remote-sync");
            Err(anyhow::anyhow!("Remote sync not supported in this build"))
        }
    } else {
        // Local sync mode
        sync::sync_files(&args).await.map_err(Into::into)
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

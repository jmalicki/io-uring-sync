//! arsync: High-performance bulk file copying utility
//!
//! This utility provides rsync-like functionality optimized for single-machine operations
//! using `io_uring` for maximum performance and parallelism.

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{info, Level};

mod adaptive_concurrency;
mod cli;
mod copy;
mod directory;
mod error;
mod i18n;
mod io_uring;
mod progress;
mod sync;

use cli::Args;
use i18n::{set_language, Language, TranslationKey};

#[compio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Set language based on --pirate flag
    if args.pirate {
        set_language(Language::Pirate);
    }

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

    // Log startup information (unless in quiet mode)
    if !args.quiet {
        info!(
            "{}: arsync v{}",
            TranslationKey::InfoStartingCopy.get(),
            env!("CARGO_PKG_VERSION")
        );
        info!(
            "{}: {}",
            TranslationKey::HelpSource.get(),
            args.source.display()
        );
        info!(
            "{}: {}",
            TranslationKey::HelpDestination.get(),
            args.destination.display()
        );
        info!("Copy method: {:?}", args.copy_method);
        info!("Queue depth: {}", args.queue_depth);
        info!("CPU count: {}", args.effective_cpu_count());
        info!("Buffer size: {} KB", args.buffer_size_kb);
        info!("Max files in flight: {}", args.max_files_in_flight);
    }

    // Validate arguments
    args.validate().context("Invalid arguments")?;

    // Perform the sync operation
    let result = sync::sync_files(&args).await;

    match result {
        Ok(stats) => {
            info!("{}", TranslationKey::StatusComplete.get());
            info!(
                "{} {}: {}",
                TranslationKey::ProgressFiles.get(),
                TranslationKey::ProgressCompleted.get(),
                stats.files_copied
            );
            info!(
                "{} {}: {}",
                TranslationKey::ProgressBytes.get(),
                TranslationKey::ProgressCompleted.get(),
                stats.bytes_copied
            );
            info!("Duration: {:?}", stats.duration);
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {e}", TranslationKey::StatusFailed.get());
            std::process::exit(1);
        }
    }
}

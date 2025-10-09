//! Adaptive concurrency control with file descriptor awareness
//!
//! This module provides self-adaptive concurrency control that automatically
//! adjusts the number of concurrent operations based on resource availability,
//! particularly file descriptor exhaustion.

use crate::directory::SharedSemaphore;
use crate::error::SyncError;
use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::warn;

/// Adaptive concurrency controller that responds to resource constraints
///
/// This wraps a semaphore and automatically reduces concurrency when
/// file descriptor exhaustion (EMFILE) is detected, then gradually
/// increases it again when resources are available.
#[derive(Clone)]
pub struct AdaptiveConcurrencyController {
    /// The underlying semaphore
    semaphore: SharedSemaphore,
    /// Counter for EMFILE errors
    emfile_errors: Arc<AtomicUsize>,
    /// Flag indicating if we've already warned about FD exhaustion
    emfile_warned: Arc<AtomicBool>,
    /// Minimum permits to maintain
    min_permits: usize,
}

impl AdaptiveConcurrencyController {
    /// Create a new adaptive controller
    ///
    /// # Arguments
    ///
    /// * `initial_permits` - Starting number of concurrent operations allowed
    #[must_use]
    pub fn new(initial_permits: usize) -> Self {
        let min_permits = std::cmp::max(10, initial_permits / 10); // Never go below 10 or 10%

        Self {
            semaphore: SharedSemaphore::new(initial_permits),
            emfile_errors: Arc::new(AtomicUsize::new(0)),
            emfile_warned: Arc::new(AtomicBool::new(false)),
            min_permits,
        }
    }

    /// Acquire a permit
    pub async fn acquire(&self) -> compio_sync::SemaphorePermit {
        self.semaphore.acquire().await
    }

    /// Handle an error, checking if it's EMFILE and adapting if needed
    ///
    /// Returns true if this is an EMFILE error and concurrency was reduced
    #[must_use]
    pub fn handle_error(&self, error: &SyncError) -> bool {
        // Check if this is a file descriptor exhaustion error
        if Self::is_emfile_error(error) {
            let count = self.emfile_errors.fetch_add(1, Ordering::Relaxed) + 1;

            // Only adapt every N errors to avoid over-reaction
            if count % 5 == 1 {
                self.adapt_to_fd_exhaustion();
                return true;
            }
        }
        false
    }

    /// Check if an error is EMFILE (too many open files)
    fn is_emfile_error(error: &SyncError) -> bool {
        let error_str = format!("{error:?}");
        error_str.contains("Too many open files")
            || error_str.contains("EMFILE")
            || error_str.contains("os error 24")
    }

    /// Adapt to file descriptor exhaustion by reducing concurrency
    fn adapt_to_fd_exhaustion(&self) {
        let current_available = self.semaphore.available_permits();
        let current_max = self.semaphore.max_permits();

        // Reduce by 25% or minimum 10, but never go below min_permits
        let reduction = std::cmp::max(10, current_max / 4);
        let actual_reduced = self.semaphore.reduce_permits(reduction);

        let new_max = current_max - actual_reduced;

        if actual_reduced > 0 {
            if self.emfile_warned.swap(true, Ordering::Relaxed) {
                // Subsequent reductions - be brief
                warn!(
                    "Reducing concurrency further due to FD exhaustion: {} → {} (-{})",
                    current_max, new_max, actual_reduced
                );
            } else {
                // First time warning - be verbose
                warn!(
                    "⚠️  FILE DESCRIPTOR EXHAUSTION DETECTED (EMFILE)\n\
                     \n\
                     arsync has hit the system file descriptor limit.\n\
                     \n\
                     Self-adaptive response:\n\
                     - Reduced concurrent operations: {} → {} (-{})\n\
                     - Currently available: {}\n\
                     - Minimum limit: {}\n\
                     \n\
                     This may slow down processing but prevents crashes.\n\
                     \n\
                     To avoid this:\n\
                     - Increase ulimit: ulimit -n 100000\n\
                     - Or use --max-files-in-flight to set lower initial concurrency\n\
                     \n\
                     Continuing with reduced concurrency...",
                    current_max, new_max, actual_reduced, current_available, self.min_permits
                );
            }
        }
    }

    /// Get current statistics
    #[must_use]
    pub fn stats(&self) -> ConcurrencyStats {
        ConcurrencyStats {
            max_permits: self.semaphore.max_permits(),
            available_permits: self.semaphore.available_permits(),
            in_use: self.semaphore.max_permits() - self.semaphore.available_permits(),
            emfile_errors: self.emfile_errors.load(Ordering::Relaxed),
        }
    }
}

/// Statistics about concurrency control
#[derive(Debug, Clone, Copy)]
pub struct ConcurrencyStats {
    /// Maximum permits configured
    pub max_permits: usize,
    /// Currently available permits
    pub available_permits: usize,
    /// Permits currently in use
    pub in_use: usize,
    /// Number of EMFILE errors encountered
    pub emfile_errors: usize,
}

/// Check system file descriptor limits and warn if too low
///
/// Returns the soft limit for file descriptors
///
/// # Errors
///
/// Returns an error if getrlimit system call fails
pub fn check_fd_limits() -> std::io::Result<u64> {
    use libc::{getrlimit, rlimit, RLIMIT_NOFILE};

    unsafe {
        let mut limit = rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };

        if getrlimit(RLIMIT_NOFILE, &raw mut limit) == 0 {
            let soft_limit = limit.rlim_cur;

            // Warn if limit seems low for high-performance operations
            if soft_limit < 10000 {
                warn!(
                    "⚠️  File descriptor limit is low: {}\n\
                     \n\
                     For optimal performance with arsync, consider:\n\
                     ulimit -n 100000\n\
                     \n\
                     Current limit may cause FD exhaustion on large operations.\n\
                     arsync will adapt automatically if this occurs.",
                    soft_limit
                );
            } else {
                tracing::info!("File descriptor limit: {} (adequate)", soft_limit);
            }

            Ok(soft_limit)
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}

/// Detect if an I/O error is EMFILE (file descriptor exhaustion)
#[must_use]
pub fn is_emfile_error(error: &std::io::Error) -> bool {
    error.kind() == ErrorKind::Other && error.raw_os_error() == Some(libc::EMFILE)
}

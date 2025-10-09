pub mod rsync_compat;

// Re-export commonly used functions
pub use rsync_compat::{compare_directories, rsync_available, run_arsync, run_rsync};

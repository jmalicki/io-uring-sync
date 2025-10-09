//! Command-line interface definitions

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// High-performance bulk file copying utility using `io_uring`
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)]
pub struct Args {
    /// Source directory or file (positional or --source)
    ///
    /// Supports rsync-style syntax:
    ///   - Local path: `/path/to/source`
    ///   - Remote path: `user@host:/path/to/source`
    ///   - Remote path: `host:/path/to/source`
    #[arg(value_name = "SOURCE")]
    pub source_positional: Option<String>,

    /// Destination directory or file (positional or --destination)
    ///
    /// Supports rsync-style syntax:
    ///   - Local path: `/path/to/dest`
    ///   - Remote path: `user@host:/path/to/dest`
    ///   - Remote path: `host:/path/to/dest`
    #[arg(value_name = "DEST")]
    pub dest_positional: Option<String>,

    /// Source directory or file (alternative to positional arg)
    #[arg(short, long, conflicts_with = "source_positional")]
    pub source: Option<PathBuf>,

    /// Destination directory or file (alternative to positional arg)
    #[arg(short, long, conflicts_with = "dest_positional")]
    pub destination: Option<PathBuf>,

    /// Queue depth for `io_uring` operations
    #[arg(long, default_value = "4096")]
    pub queue_depth: usize,

    /// Maximum total files in flight (across all CPU cores)
    ///
    /// Controls memory usage and system load by limiting the total number of
    /// files being copied simultaneously. Higher values increase throughput
    /// but consume more memory and file descriptors.
    ///
    /// Default: 1024
    /// High-performance (`NVMe`, 32GB+ RAM): 2048-4096
    /// Conservative (`HDD`, limited RAM): 256-512
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

    // ========== rsync-compatible flags ==========
    /// Archive mode; same as -rlptgoD (recursive, links, perms, times, group, owner, devices)
    #[arg(short = 'a', long)]
    pub archive: bool,

    /// Recurse into directories
    #[arg(short = 'r', long)]
    pub recursive: bool,

    /// Copy symlinks as symlinks
    #[arg(short = 'l', long)]
    pub links: bool,

    /// Preserve permissions
    #[arg(short = 'p', long)]
    pub perms: bool,

    /// Preserve modification times
    #[arg(short = 't', long)]
    pub times: bool,

    /// Preserve group
    #[arg(short = 'g', long)]
    pub group: bool,

    /// Preserve owner (super-user only)
    #[arg(short = 'o', long)]
    pub owner: bool,

    /// Preserve device files (super-user only) and special files
    #[arg(short = 'D', long)]
    pub devices: bool,

    /// Preserve extended attributes
    #[arg(short = 'X', long)]
    pub xattrs: bool,

    /// Preserve ACLs (implies --perms)
    #[arg(short = 'A', long)]
    pub acls: bool,

    /// Preserve hard links
    #[arg(short = 'H', long)]
    pub hard_links: bool,

    /// Preserve access (use) times
    #[arg(short = 'U', long)]
    pub atimes: bool,

    /// Preserve creation times (when supported)
    #[arg(long)]
    pub crtimes: bool,

    // ========== Deprecated flags (for backwards compatibility) ==========
    /// Preserve extended attributes (deprecated: use -X/--xattrs)
    #[arg(long, hide = true)]
    pub preserve_xattr: bool,

    /// Preserve POSIX ACLs (deprecated: use -A/--acls)
    #[arg(long, hide = true)]
    pub preserve_acl: bool,

    // ========== Other flags ==========
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

    // ========== Concurrency control flags ==========
    /// Disable adaptive concurrency control (fail fast on resource exhaustion)
    ///
    /// By default, arsync automatically reduces concurrency when hitting resource
    /// limits like "Too many open files" (EMFILE). This flag disables that behavior
    /// and causes arsync to exit immediately on such errors instead.
    ///
    /// Use this if you want strict resource limit enforcement or in CI/CD environments
    /// where you want to catch configuration issues early.
    #[arg(long)]
    pub no_adaptive_concurrency: bool,

    // ========== Remote sync options ==========
    /// Run in server mode (for remote sync)
    #[arg(long, hide = true)]
    pub server: bool,

    /// Remote shell to use (default: ssh)
    #[arg(short = 'e', long = "rsh", default_value = "ssh")]
    pub remote_shell: String,

    /// Daemon mode (rsyncd compatibility)
    #[arg(long, hide = true)]
    pub daemon: bool,

    // ========== Pipe mode (testing only) ==========
    /// Pipe mode: communicate via stdin/stdout (for protocol testing)
    ///
    /// This mode is for testing the rsync wire protocol without SSH.
    /// NOT for normal use - local copies use `io_uring` direct operations!
    #[arg(long, hide = true)]
    pub pipe: bool,

    /// Pipe role: sender or receiver
    #[arg(long, requires = "pipe", value_enum)]
    pub pipe_role: Option<PipeRole>,
}

/// Role in pipe mode
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum PipeRole {
    /// Sender: read files and send via protocol
    Sender,
    /// Receiver: receive via protocol and write files
    Receiver,
}

/// Parsed location (local or remote)
#[derive(Debug, Clone)]
pub enum Location {
    Local(PathBuf),
    Remote {
        user: Option<String>,
        host: String,
        path: PathBuf,
    },
}

impl Location {
    /// Parse rsync-style path: `[user@]host:path` or `/local/path`
    ///
    /// # Errors
    ///
    /// Returns an error if the path string is invalid (reserved for future validation)
    #[allow(clippy::unnecessary_wraps)] // May add validation in future
    pub fn parse(s: &str) -> Result<Self> {
        // Check for remote syntax: [user@]host:path
        if let Some(colon_pos) = s.find(':') {
            // Could be remote or Windows path (C:\...)
            // Windows paths have letter:\ pattern
            if colon_pos == 1 && s.chars().nth(0).is_some_and(|c| c.is_ascii_alphabetic()) {
                // Likely Windows path
                return Ok(Self::Local(PathBuf::from(s)));
            }

            let host_part = &s[..colon_pos];
            let path_part = &s[colon_pos + 1..];

            // Parse user@host or just host
            let (user, host) = host_part.find('@').map_or_else(
                || (None, host_part.to_string()),
                |at_pos| {
                    (
                        Some(host_part[..at_pos].to_string()),
                        host_part[at_pos + 1..].to_string(),
                    )
                },
            );

            Ok(Self::Remote {
                user,
                host,
                path: PathBuf::from(path_part),
            })
        } else {
            // Local path
            Ok(Self::Local(PathBuf::from(s)))
        }
    }

    /// Get the path component
    #[must_use]
    pub const fn path(&self) -> &PathBuf {
        match self {
            Self::Local(path) | Self::Remote { path, .. } => path,
        }
    }

    /// Check if this is a remote location
    #[must_use]
    pub const fn is_remote(&self) -> bool {
        matches!(self, Self::Remote { .. })
    }

    /// Check if this is a local location
    #[must_use]
    #[allow(dead_code)] // Will be used by protocol module
    pub const fn is_local(&self) -> bool {
        matches!(self, Self::Local(_))
    }
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
    /// Get the source location (from positional or flag)
    ///
    /// # Errors
    ///
    /// Returns an error if source is not specified or fails to parse
    pub fn get_source(&self) -> Result<Location> {
        if let Some(ref src) = self.source_positional {
            Location::parse(src)
        } else if let Some(ref src) = self.source {
            Ok(Location::Local(src.clone()))
        } else {
            anyhow::bail!("Source must be specified (positional or --source)")
        }
    }

    /// Get the destination location (from positional or flag)
    ///
    /// # Errors
    ///
    /// Returns an error if destination is not specified or fails to parse
    pub fn get_destination(&self) -> Result<Location> {
        if let Some(ref dest) = self.dest_positional {
            Location::parse(dest)
        } else if let Some(ref dest) = self.destination {
            Ok(Location::Local(dest.clone()))
        } else {
            anyhow::bail!("Destination must be specified (positional or --destination)")
        }
    }

    /// Validate command-line arguments
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Source/destination not specified
    /// - Source path does not exist (for local paths)
    /// - Source path is not a file or directory (for local paths)
    /// - Queue depth is outside valid bounds (1024-65536)
    /// - Max files in flight is outside valid bounds (1-10000)
    /// - Buffer size is too large (>1GB)
    /// - No CPU cores are available
    /// - Both --quiet and --verbose options are used
    pub fn validate(&self) -> Result<()> {
        // Pipe mode: skip path validation (paths from stdin/stdout)
        if self.pipe {
            if self.pipe_role.is_none() {
                anyhow::bail!("--pipe requires --pipe-role (sender or receiver)");
            }
            return self.validate_common();
        }

        // Get source and destination
        let source = self.get_source()?;
        let destination = self.get_destination()?;

        // Validate remote sync options
        if source.is_remote() || destination.is_remote() {
            // Remote sync mode
            if source.is_remote() && destination.is_remote() {
                anyhow::bail!("Cannot sync from remote to remote (yet)");
            }
            // Remote sync validation passes - will be validated when connecting
            return self.validate_common();
        }

        // Local sync mode - validate source exists
        let source_path = source.path();
        if !source_path.exists() {
            anyhow::bail!("Source path does not exist: {}", source_path.display());
        }

        // Check if source is readable
        if !source_path.is_dir() && !source_path.is_file() {
            anyhow::bail!(
                "Source path must be a file or directory: {}",
                source_path.display()
            );
        }

        self.validate_common()
    }

    fn validate_common(&self) -> Result<()> {
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

    /// Check if the source is a directory (for local sources)
    #[must_use]
    #[allow(dead_code)] // Will be used by tests
    pub fn is_directory_copy(&self) -> bool {
        if let Ok(Location::Local(path)) = self.get_source() {
            return path.is_dir();
        }
        false
    }

    /// Check if the source is a single file (for local sources)
    #[must_use]
    #[allow(dead_code)] // Will be used by tests
    pub fn is_file_copy(&self) -> bool {
        if let Ok(Location::Local(path)) = self.get_source() {
            return path.is_file();
        }
        false
    }

    /// Get buffer size in bytes
    #[must_use]
    pub const fn buffer_size_bytes(&self) -> usize {
        self.buffer_size_kb * 1024
    }

    // ========== rsync-compatible helper methods ==========

    /// Check if permissions should be preserved
    #[must_use]
    pub const fn should_preserve_permissions(&self) -> bool {
        self.perms || self.archive || self.acls
    }

    /// Check if ownership (user and/or group) should be preserved
    #[must_use]
    pub const fn should_preserve_ownership(&self) -> bool {
        self.owner || self.group || self.archive
    }

    /// Check if user ownership should be preserved
    #[allow(dead_code)]
    #[must_use]
    pub const fn should_preserve_owner(&self) -> bool {
        self.owner || self.archive
    }

    /// Check if group ownership should be preserved
    #[allow(dead_code)]
    #[must_use]
    pub const fn should_preserve_group(&self) -> bool {
        self.group || self.archive
    }

    /// Check if timestamps should be preserved
    #[must_use]
    pub const fn should_preserve_timestamps(&self) -> bool {
        self.times || self.archive
    }

    /// Check if access times should be preserved
    #[allow(dead_code)]
    #[must_use]
    pub const fn should_preserve_atimes(&self) -> bool {
        self.atimes
    }

    /// Check if creation times should be preserved
    #[allow(dead_code)]
    #[must_use]
    pub const fn should_preserve_crtimes(&self) -> bool {
        self.crtimes
    }

    /// Check if extended attributes should be preserved
    #[must_use]
    pub const fn should_preserve_xattrs(&self) -> bool {
        self.xattrs || self.preserve_xattr
    }

    /// Check if ACLs should be preserved
    #[allow(dead_code)]
    #[must_use]
    pub const fn should_preserve_acls(&self) -> bool {
        self.acls || self.preserve_acl
    }

    /// Check if symlinks should be copied as symlinks
    #[allow(dead_code)]
    #[must_use]
    pub const fn should_preserve_links(&self) -> bool {
        self.links || self.archive
    }

    /// Check if hard links should be preserved
    #[allow(dead_code)]
    #[must_use]
    pub const fn should_preserve_hard_links(&self) -> bool {
        self.hard_links
    }

    /// Check if device files should be preserved
    #[allow(dead_code)]
    #[must_use]
    pub const fn should_preserve_devices(&self) -> bool {
        self.devices || self.archive
    }

    /// Check if recursive copying should be performed
    #[allow(dead_code)]
    #[must_use]
    pub const fn should_recurse(&self) -> bool {
        self.recursive || self.archive
    }
}

impl Args {
    /// Create a test Args instance with default values (for testing)
    #[cfg(test)]
    pub fn test_default(source: PathBuf, destination: PathBuf) -> Self {
        Self {
            source_positional: None,
            dest_positional: None,
            source: Some(source),
            destination: Some(destination),
            queue_depth: 4096,
            max_files_in_flight: 1024,
            cpu_count: 1,
            buffer_size_kb: 64,
            copy_method: CopyMethod::Auto,
            archive: false,
            recursive: false,
            links: false,
            perms: false,
            times: false,
            group: false,
            owner: false,
            devices: false,
            xattrs: false,
            acls: false,
            hard_links: false,
            atimes: false,
            crtimes: false,
            preserve_xattr: false,
            preserve_acl: false,
            dry_run: false,
            progress: false,
            verbose: 0,
            quiet: false,
            no_adaptive_concurrency: false,
            server: false,
            remote_shell: "ssh".to_string(),
            daemon: false,
            pipe: false,
            pipe_role: None,
        }
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
        let mut args = Args::test_default(file_path.clone(), temp_dir.path().join("dest"));
        args = Args {
            source_positional: None,
            dest_positional: None,
            source: Some(file_path),
            destination: Some(temp_dir.path().join("dest")),
            copy_method: CopyMethod::Auto,
            queue_depth: 4096,
            cpu_count: 2,
            buffer_size_kb: 1024,
            max_files_in_flight: 100,
            archive: false,
            recursive: false,
            links: false,
            perms: false,
            times: false,
            group: false,
            owner: false,
            devices: false,
            xattrs: true,
            acls: false,
            hard_links: false,
            atimes: false,
            crtimes: false,
            preserve_xattr: false,
            preserve_acl: false,
            dry_run: false,
            progress: false,
            verbose: 0,
            quiet: false,
            no_adaptive_concurrency: false,
            server: false,
            remote_shell: "ssh".to_string(),
            daemon: false,
            pipe: false,
            pipe_role: None,
            ..args
        };

        assert!(args.validate().is_ok());
    }

    #[compio::test]
    async fn test_validate_with_existing_directory() {
        let (temp_dir, dir_path) = create_temp_dir().await.unwrap();
        let mut args = Args::test_default(dir_path.clone(), temp_dir.path().join("dest"));
        args = Args {
            source_positional: None,
            dest_positional: None,
            source: Some(dir_path),
            destination: Some(temp_dir.path().join("dest")),
            copy_method: CopyMethod::Auto,
            queue_depth: 4096,
            cpu_count: 2,
            buffer_size_kb: 1024,
            max_files_in_flight: 100,
            archive: false,
            recursive: false,
            links: false,
            perms: false,
            times: false,
            group: false,
            owner: false,
            devices: false,
            xattrs: true,
            acls: false,
            hard_links: false,
            atimes: false,
            crtimes: false,
            preserve_xattr: false,
            preserve_acl: false,
            dry_run: false,
            progress: false,
            verbose: 0,
            quiet: false,
            no_adaptive_concurrency: false,
            server: false,
            remote_shell: "ssh".to_string(),
            daemon: false,
            pipe: false,
            pipe_role: None,
            ..args
        };

        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_with_nonexistent_source() {
        let mut args = Args::test_default(PathBuf::from("/nonexistent"), PathBuf::from("/dest"));
        args = Args {
            source_positional: None,
            dest_positional: None,
            source: Some(PathBuf::from("/nonexistent/path")),
            destination: Some(PathBuf::from("/tmp/dest")),
            copy_method: CopyMethod::Auto,
            queue_depth: 4096,
            cpu_count: 2,
            buffer_size_kb: 1024,
            max_files_in_flight: 100,
            archive: false,
            recursive: false,
            links: false,
            perms: false,
            times: false,
            group: false,
            owner: false,
            devices: false,
            xattrs: true,
            acls: false,
            hard_links: false,
            atimes: false,
            crtimes: false,
            preserve_xattr: false,
            preserve_acl: false,
            dry_run: false,
            progress: false,
            verbose: 0,
            quiet: false,
            no_adaptive_concurrency: false,
            server: false,
            remote_shell: "ssh".to_string(),
            daemon: false,
            pipe: false,
            pipe_role: None,
            ..args
        };

        assert!(args.validate().is_err());
    }

    #[test]
    fn test_parse_local_path() {
        let loc = Location::parse("/home/user/file.txt").unwrap();
        assert!(loc.is_local());
        assert_eq!(loc.path(), &PathBuf::from("/home/user/file.txt"));
    }

    #[test]
    fn test_parse_remote_path_with_user() {
        let loc = Location::parse("user@host:/path/to/file").unwrap();
        assert!(loc.is_remote());
        if let Location::Remote { user, host, path } = loc {
            assert_eq!(user, Some("user".to_string()));
            assert_eq!(host, "host");
            assert_eq!(path, PathBuf::from("/path/to/file"));
        } else {
            panic!("Expected Remote location");
        }
    }

    #[test]
    fn test_parse_remote_path_without_user() {
        let loc = Location::parse("host:/path/to/file").unwrap();
        assert!(loc.is_remote());
        if let Location::Remote { user, host, path } = loc {
            assert_eq!(user, None);
            assert_eq!(host, "host");
            assert_eq!(path, PathBuf::from("/path/to/file"));
        } else {
            panic!("Expected Remote location");
        }
    }

    #[test]
    fn test_positional_args() {
        let mut args = Args::test_default(PathBuf::from("/dev/null"), PathBuf::from("/dev/null"));
        args = Args {
            source_positional: Some("/src".to_string()),
            dest_positional: Some("/dest".to_string()),
            source: None,
            destination: None,
            copy_method: CopyMethod::Auto,
            queue_depth: 4096,
            cpu_count: 2,
            buffer_size_kb: 1024,
            max_files_in_flight: 100,
            archive: false,
            recursive: false,
            links: false,
            perms: false,
            times: false,
            group: false,
            owner: false,
            devices: false,
            xattrs: false,
            acls: false,
            hard_links: false,
            atimes: false,
            crtimes: false,
            preserve_xattr: false,
            preserve_acl: false,
            dry_run: false,
            progress: false,
            verbose: 0,
            quiet: false,
            no_adaptive_concurrency: false,
            server: false,
            remote_shell: "ssh".to_string(),
            daemon: false,
            pipe: false,
            pipe_role: None,
            ..args
        };

        let source = args.get_source().unwrap();
        let dest = args.get_destination().unwrap();

        assert!(source.is_local());
        assert!(dest.is_local());
        assert_eq!(source.path(), &PathBuf::from("/src"));
        assert_eq!(dest.path(), &PathBuf::from("/dest"));
    }
}

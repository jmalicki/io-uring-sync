//! Common test helpers for integration tests

use arsync::cli::{Args, CopyMethod};
use std::path::PathBuf;

/// Create a default Args for testing
pub fn create_test_args(source: PathBuf, destination: PathBuf) -> Args {
    Args {
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
        rsync_compat: false,
    }
}

/// Create Args with archive mode for testing
pub fn create_test_args_archive(source: PathBuf, destination: PathBuf) -> Args {
    let mut args = create_test_args(source, destination);
    args.archive = true;
    args
}

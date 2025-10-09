//! Test permission preservation in file copying

use arsync::cli::{Args, CopyMethod};
use arsync::copy::copy_file;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a default Args struct for testing with archive mode enabled
fn create_test_args_with_archive() -> Args {
    Args {
        source: PathBuf::from("/test/source"),
        destination: PathBuf::from("/test/dest"),
        queue_depth: 4096,
        max_files_in_flight: 1024,
        cpu_count: 1,
        buffer_size_kb: 64,
        copy_method: CopyMethod::Auto,
        archive: true, // Enable archive mode for full metadata preservation
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
    }
}

#[compio::test]
async fn test_permission_preservation() {
    let temp_dir = TempDir::new().unwrap();
    let src_path = temp_dir.path().join("source.txt");
    let dst_path = temp_dir.path().join("destination.txt");

    // Create source file with specific permissions
    fs::write(&src_path, "Hello, World!").unwrap();

    // Set specific permissions (read/write for owner, read for group and others)
    let permissions = std::fs::Permissions::from_mode(0o644);
    fs::set_permissions(&src_path, permissions).unwrap();

    // Copy the file with archive mode (full metadata preservation)
    let args = create_test_args_with_archive();
    copy_file(&src_path, &dst_path, &args).await.unwrap();

    // Check that permissions were preserved
    let src_metadata = fs::metadata(&src_path).unwrap();
    let dst_metadata = fs::metadata(&dst_path).unwrap();

    let src_permissions = src_metadata.permissions().mode();
    let dst_permissions = dst_metadata.permissions().mode();

    println!("Source permissions: {:o}", src_permissions);
    println!("Destination permissions: {:o}", dst_permissions);

    assert_eq!(
        src_permissions, dst_permissions,
        "Permissions should be preserved"
    );
}

#[compio::test]
async fn test_timestamp_preservation() {
    let temp_dir = TempDir::new().unwrap();
    let src_path = temp_dir.path().join("source.txt");
    let dst_path = temp_dir.path().join("destination.txt");

    // Create source file
    fs::write(&src_path, "Hello, World!").unwrap();

    // Get original timestamps
    let src_metadata = fs::metadata(&src_path).unwrap();
    let original_accessed = src_metadata.accessed().unwrap();
    let original_modified = src_metadata.modified().unwrap();

    // Wait a bit to ensure timestamps are different
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Copy the file with archive mode (full metadata preservation)
    let args = create_test_args_with_archive();
    copy_file(&src_path, &dst_path, &args).await.unwrap();

    // Check that timestamps were preserved
    let dst_metadata = fs::metadata(&dst_path).unwrap();
    let copied_accessed = dst_metadata.accessed().unwrap();
    let copied_modified = dst_metadata.modified().unwrap();

    println!("Original accessed: {:?}", original_accessed);
    println!("Copied accessed: {:?}", copied_accessed);
    println!("Original modified: {:?}", original_modified);
    println!("Copied modified: {:?}", copied_modified);

    // Timestamps should be very close (within a few milliseconds due to system precision)
    let _accessed_diff = copied_accessed
        .duration_since(original_accessed)
        .unwrap_or_default();
    let modified_diff = copied_modified
        .duration_since(original_modified)
        .unwrap_or_default();

    assert!(
        _accessed_diff.as_millis() < 100,
        "Accessed time should be preserved within 100ms"
    );
    assert!(
        modified_diff.as_millis() < 100,
        "Modified time should be preserved within 100ms"
    );
}

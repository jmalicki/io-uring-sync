//! Integration tests for hardlink detection and copying during directory traversal


use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

/// Test that hardlinks are properly detected and copied during directory traversal

async fn test_integrated_hardlink_copying() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_path = temp_dir.path().join("src");
    let dst_path = temp_dir.path().join("dst");

    // Create hardlinks
    let hardlink1 = src_path.join("hardlink1.txt");
    let hardlink2 = src_path.join("hardlink2.txt");
    std::fs::hard_link(&original_file, &hardlink1).expect("Failed to create hardlink1");
    std::fs::hard_link(&original_file, &hardlink2).expect("Failed to create hardlink2");

    let subdir_file = subdir.join("subdir_file.txt");
    let mut file = File::create(&subdir_file).expect("Failed to create subdir file");
    writeln!(file, "File in subdirectory.").expect("Failed to write to subdir file");
    drop(file);

    // Copy directory with integrated hardlink detection
    let stats = copy_directory(&src_path, &dst_path, &file_ops, CopyMethod::Auto)
        .await
        .expect("Failed to copy directory");

async fn test_filesystem_boundary_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_path = temp_dir.path().join("src");
    let dst_path = temp_dir.path().join("dst");

    // Create a test file
    let test_file = src_path.join("test.txt");
    let mut file = File::create(&test_file).expect("Failed to create test file");
    writeln!(file, "Test content").expect("Failed to write to test file");
    drop(file);

    // Copy directory - should detect filesystem boundaries
    let stats = copy_directory(&src_path, &dst_path, &file_ops, CopyMethod::Auto)
        .await
        .expect("Failed to copy directory");

async fn test_symlink_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_path = temp_dir.path().join("src");
    let dst_path = temp_dir.path().join("dst");

    // Create a target file
    let target_file = src_path.join("target.txt");
    let mut file = File::create(&target_file).expect("Failed to create target file");
    writeln!(file, "Target content").expect("Failed to write to target file");
    drop(file);

    // Copy directory with symlink
    let stats = copy_directory(&src_path, &dst_path, &file_ops, CopyMethod::Auto)
        .await
        .expect("Failed to copy directory");

    // Test hardlink registration
    let test_path = std::path::Path::new("/test/file.txt");
    let registered = tracker.register_file(test_path, 12345, 100, 2);
    assert!(registered, "Should register new file with link_count > 1");

    // Test statistics
    let stats = tracker.get_stats();
    assert_eq!(stats.total_files, 1, "Should have 1 unique file");
    assert_eq!(stats.hardlink_groups, 1, "Should have 1 hardlink group");
    assert_eq!(stats.total_hardlinks, 2, "Should have 2 total hardlinks");
}

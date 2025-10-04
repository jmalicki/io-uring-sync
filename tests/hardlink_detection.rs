//! Integration tests for hardlink detection functionality
//!
//! These tests verify that our hardlink detection system works correctly
//! by creating actual hardlinks and testing the detection logic.

use io_uring_sync::directory::{FilesystemTracker, analyze_filesystem_structure};
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[tokio::test]
async fn test_hardlink_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create a test file
    let original_file = temp_path.join("original.txt");
    let mut file = File::create(&original_file).expect("Failed to create original file");
    writeln!(file, "Hello, world!").expect("Failed to write to original file");
    drop(file);

    // Create a hardlink
    let hardlink_file = temp_path.join("hardlink.txt");
    std::fs::hard_link(&original_file, &hardlink_file)
        .expect("Failed to create hardlink");

    // Create a regular file (not a hardlink)
    let regular_file = temp_path.join("regular.txt");
    let mut file = File::create(&regular_file).expect("Failed to create regular file");
    writeln!(file, "Different content").expect("Failed to write to regular file");
    drop(file);

    // Test hardlink detection
    let mut tracker = FilesystemTracker::new();
    analyze_filesystem_structure(temp_path, &mut tracker)
        .await
        .expect("Failed to analyze filesystem structure");

    let stats = tracker.get_stats();
    
    // We should have 2 unique files (original + regular, hardlink is same inode)
    assert_eq!(stats.total_files, 2, "Should have 2 unique files");
    
    // We should have 1 hardlink group (original + hardlink)
    assert_eq!(stats.hardlink_groups, 1, "Should have 1 hardlink group");
    
    // We should have 3 total hardlinks (1 original + 1 hardlink + 1 regular)
    assert_eq!(stats.total_hardlinks, 3, "Should have 3 total hardlinks");

    // Test individual hardlink detection
    let hardlink_groups = tracker.get_hardlink_groups();
    assert_eq!(hardlink_groups.len(), 1, "Should have 1 hardlink group");
    
    let hardlink_info = &hardlink_groups[0];
    assert_eq!(hardlink_info.link_count, 2, "Hardlink group should have 2 links");
    
    // The original path should be one of our files
    let original_path = &hardlink_info.original_path;
    assert!(
        original_path == &original_file || original_path == &hardlink_file,
        "Original path should be one of our hardlinked files"
    );
}

#[tokio::test]
async fn test_filesystem_boundary_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create a test file
    let test_file = temp_path.join("test.txt");
    let mut file = File::create(&test_file).expect("Failed to create test file");
    writeln!(file, "Test content").expect("Failed to write to test file");
    drop(file);

    // Test filesystem boundary detection
    let mut tracker = FilesystemTracker::new();
    analyze_filesystem_structure(temp_path, &mut tracker)
        .await
        .expect("Failed to analyze filesystem structure");

    let stats = tracker.get_stats();
    
    // Should have a valid source filesystem
    assert!(stats.source_filesystem.is_some(), "Should have a source filesystem");
    
    let source_dev = stats.source_filesystem.unwrap();
    assert!(source_dev > 0, "Source filesystem device ID should be positive");
    
    // All files should be on the same filesystem
    assert_eq!(stats.total_files, 1, "Should have 1 file");
    assert_eq!(stats.total_hardlinks, 1, "Should have 1 hardlink (the file itself)");
}

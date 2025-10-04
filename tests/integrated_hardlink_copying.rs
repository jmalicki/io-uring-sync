//! Integration tests for hardlink detection and copying during directory traversal

use io_uring_sync::cli::CopyMethod;
use io_uring_sync::directory::{copy_directory, FilesystemTracker};
use io_uring_sync::io_uring::FileOperations;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

/// Test that hardlinks are properly detected and copied during directory traversal
#[tokio::test]
async fn test_integrated_hardlink_copying() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_path = temp_dir.path().join("src");
    let dst_path = temp_dir.path().join("dst");

    // Create source directory structure with hardlinks
    std::fs::create_dir(&src_path).expect("Failed to create source directory");

    // Create original file
    let original_file = src_path.join("original.txt");
    let mut file = File::create(&original_file).expect("Failed to create original file");
    writeln!(
        file,
        "This is the original content that will be hardlinked."
    )
    .expect("Failed to write to original file");
    drop(file);

    // Create hardlinks
    let hardlink1 = src_path.join("hardlink1.txt");
    let hardlink2 = src_path.join("hardlink2.txt");
    std::fs::hard_link(&original_file, &hardlink1).expect("Failed to create hardlink1");
    std::fs::hard_link(&original_file, &hardlink2).expect("Failed to create hardlink2");

    // Create a regular file (not a hardlink)
    let regular_file = src_path.join("regular.txt");
    let mut file = File::create(&regular_file).expect("Failed to create regular file");
    writeln!(file, "This is a regular file, not a hardlink.")
        .expect("Failed to write to regular file");
    drop(file);

    // Create subdirectory with more files
    let subdir = src_path.join("subdir");
    std::fs::create_dir(&subdir).expect("Failed to create subdirectory");

    let subdir_file = subdir.join("subdir_file.txt");
    let mut file = File::create(&subdir_file).expect("Failed to create subdir file");
    writeln!(file, "File in subdirectory.").expect("Failed to write to subdir file");
    drop(file);

    // Create file operations instance
    let file_ops = FileOperations::new(4096, 64 * 1024).expect("Failed to create FileOperations");

    // Copy directory with integrated hardlink detection
    let stats = copy_directory(&src_path, &dst_path, &file_ops, CopyMethod::Auto)
        .await
        .expect("Failed to copy directory");

    // Verify basic copy statistics
    assert!(
        stats.files_copied >= 4,
        "Should have copied at least 4 files"
    );
    assert!(
        stats.directories_created >= 1,
        "Should have created at least 1 directory"
    );
    assert!(stats.errors == 0, "Should have no errors");

    // Verify that all files exist in destination
    assert!(
        dst_path.join("original.txt").exists(),
        "Original file should exist"
    );
    assert!(
        dst_path.join("hardlink1.txt").exists(),
        "Hardlink1 should exist"
    );
    assert!(
        dst_path.join("hardlink2.txt").exists(),
        "Hardlink2 should exist"
    );
    assert!(
        dst_path.join("regular.txt").exists(),
        "Regular file should exist"
    );
    assert!(
        dst_path.join("subdir").exists(),
        "Subdirectory should exist"
    );
    assert!(
        dst_path.join("subdir").join("subdir_file.txt").exists(),
        "Subdir file should exist"
    );

    // Verify that hardlinks have the same content
    let original_content =
        std::fs::read_to_string(dst_path.join("original.txt")).expect("Failed to read original");
    let hardlink1_content =
        std::fs::read_to_string(dst_path.join("hardlink1.txt")).expect("Failed to read hardlink1");
    let hardlink2_content =
        std::fs::read_to_string(dst_path.join("hardlink2.txt")).expect("Failed to read hardlink2");

    assert_eq!(
        original_content, hardlink1_content,
        "Hardlink1 should have same content as original"
    );
    assert_eq!(
        original_content, hardlink2_content,
        "Hardlink2 should have same content as original"
    );
    assert_eq!(
        hardlink1_content, hardlink2_content,
        "Hardlinks should have same content"
    );

    // Verify that regular file has different content
    let regular_content =
        std::fs::read_to_string(dst_path.join("regular.txt")).expect("Failed to read regular");
    assert_ne!(
        original_content, regular_content,
        "Regular file should have different content"
    );
}

/// Test that filesystem boundary detection works during traversal
#[tokio::test]
async fn test_filesystem_boundary_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_path = temp_dir.path().join("src");
    let dst_path = temp_dir.path().join("dst");

    // Create source directory
    std::fs::create_dir(&src_path).expect("Failed to create source directory");

    // Create a test file
    let test_file = src_path.join("test.txt");
    let mut file = File::create(&test_file).expect("Failed to create test file");
    writeln!(file, "Test content").expect("Failed to write to test file");
    drop(file);

    // Create file operations instance
    let file_ops = FileOperations::new(4096, 64 * 1024).expect("Failed to create FileOperations");

    // Copy directory - should detect filesystem boundaries
    let stats = copy_directory(&src_path, &dst_path, &file_ops, CopyMethod::Auto)
        .await
        .expect("Failed to copy directory");

    // Should complete successfully
    assert!(
        stats.errors == 0,
        "Should have no filesystem boundary errors"
    );
    assert!(
        stats.files_copied >= 1,
        "Should have copied at least 1 file"
    );

    // Verify file was copied
    assert!(
        dst_path.join("test.txt").exists(),
        "Test file should exist in destination"
    );
}

/// Test that symlinks are handled during integrated traversal
#[tokio::test]
async fn test_symlink_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_path = temp_dir.path().join("src");
    let dst_path = temp_dir.path().join("dst");

    // Create source directory
    std::fs::create_dir(&src_path).expect("Failed to create source directory");

    // Create a target file
    let target_file = src_path.join("target.txt");
    let mut file = File::create(&target_file).expect("Failed to create target file");
    writeln!(file, "Target content").expect("Failed to write to target file");
    drop(file);

    // Create a symlink
    let symlink_file = src_path.join("symlink.txt");
    std::os::unix::fs::symlink(&target_file, &symlink_file).expect("Failed to create symlink");

    // Create file operations instance
    let file_ops = FileOperations::new(4096, 64 * 1024).expect("Failed to create FileOperations");

    // Copy directory with symlink
    let stats = copy_directory(&src_path, &dst_path, &file_ops, CopyMethod::Auto)
        .await
        .expect("Failed to copy directory");

    // Should complete successfully
    assert!(stats.errors == 0, "Should have no symlink errors");
    assert!(
        stats.symlinks_processed >= 1,
        "Should have processed at least 1 symlink"
    );

    // Verify symlink was copied
    assert!(
        dst_path.join("symlink.txt").is_symlink(),
        "Symlink should exist in destination"
    );
    assert!(
        dst_path.join("target.txt").exists(),
        "Target file should exist in destination"
    );
}

/// Test FilesystemTracker functionality
#[tokio::test]
async fn test_filesystem_tracker_functionality() {
    let mut tracker = FilesystemTracker::new();

    // Test initial state - use get_stats to check filesystem
    let initial_stats = tracker.get_stats();
    assert!(
        initial_stats.source_filesystem.is_none(),
        "Source filesystem should be None initially"
    );

    // Set source filesystem
    tracker.set_source_filesystem(12345);
    let updated_stats = tracker.get_stats();
    assert_eq!(
        updated_stats.source_filesystem,
        Some(12345),
        "Source filesystem should be set"
    );

    // Test filesystem boundary detection
    assert!(
        tracker.is_same_filesystem(12345),
        "Same filesystem should return true"
    );
    assert!(
        !tracker.is_same_filesystem(54321),
        "Different filesystem should return false"
    );

    // Test hardlink registration
    let test_path = std::path::Path::new("/test/file.txt");
    let registered = tracker.register_file(test_path, 12345, 100, 2);
    assert!(registered, "Should register new file with link_count > 1");

    // Test duplicate registration (same inode)
    let duplicate_registered = tracker.register_file(test_path, 12345, 100, 2);
    assert!(!duplicate_registered, "Should not register duplicate inode");

    // Test file with link_count = 1 (should be skipped)
    let skipped = tracker.register_file(test_path, 12345, 101, 1);
    assert!(!skipped, "Should skip file with link_count = 1");

    // Test inode tracking
    assert!(
        !tracker.is_inode_copied(100),
        "Inode should not be marked as copied initially"
    );

    // Mark inode as copied
    tracker.mark_inode_copied(100, std::path::Path::new("/dst/file.txt"));
    assert!(
        tracker.is_inode_copied(100),
        "Inode should be marked as copied"
    );

    // Test getting original path
    let original_path = tracker.get_original_path_for_inode(100);
    assert!(
        original_path.is_some(),
        "Should be able to get original path"
    );
    assert_eq!(
        original_path.unwrap(),
        std::path::Path::new("/dst/file.txt")
    );

    // Test statistics
    let stats = tracker.get_stats();
    assert_eq!(stats.total_files, 1, "Should have 1 unique file");
    assert_eq!(stats.hardlink_groups, 1, "Should have 1 hardlink group");
    assert_eq!(stats.total_hardlinks, 2, "Should have 2 total hardlinks");
}

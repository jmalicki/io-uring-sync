//! Basic integration tests for io-uring-extended operations
//!
//! These tests verify that our extended io_uring operations work correctly
//! and can be used as expected.

use io_uring_extended::ExtendedRio;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[tokio::test]
async fn test_symlink_operations() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create a test file
    let test_file = temp_path.join("test_file.txt");
    let mut file = File::create(&test_file).expect("Failed to create test file");
    writeln!(file, "Hello, world!").expect("Failed to write to test file");
    drop(file);

    // Create ExtendedRio instance
    let extended_rio = ExtendedRio::new().expect("Failed to create ExtendedRio");

    // Test symlink creation
    let symlink_path = temp_path.join("test_symlink");
    extended_rio
        .symlinkat(&test_file, &symlink_path)
        .await
        .expect("Failed to create symlink");

    // Verify symlink was created
    assert!(symlink_path.exists(), "Symlink should exist");
    assert!(symlink_path.is_symlink(), "Path should be a symlink");

    // Test symlink reading
    let mut buffer = vec![0u8; 1024];
    let bytes_read = extended_rio
        .readlinkat(&symlink_path, &mut buffer)
        .await
        .expect("Failed to read symlink");

    let target_path = String::from_utf8_lossy(&buffer[..bytes_read]);
    assert_eq!(
        target_path,
        test_file.to_string_lossy(),
        "Symlink target should match original file path"
    );
}

#[tokio::test]
async fn test_statx_inode_operations() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create a test file
    let test_file = temp_path.join("test_file.txt");
    let mut file = File::create(&test_file).expect("Failed to create test file");
    writeln!(file, "Hello, world!").expect("Failed to write to test file");
    drop(file);

    // Create ExtendedRio instance
    let extended_rio = ExtendedRio::new().expect("Failed to create ExtendedRio");

    // Test statx_inode operation
    let (dev, ino) = extended_rio
        .statx_inode(&test_file)
        .await
        .expect("Failed to get inode information");

    // Verify we got valid device and inode numbers
    // The statx_inode function returns actual filesystem values
    assert!(dev > 0, "Device ID should be a positive number");
    assert!(ino > 0, "Inode number should be a positive number");
}

#[tokio::test]
async fn test_hardlink_operations() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create a test file
    let original_file = temp_path.join("original.txt");
    let mut file = File::create(&original_file).expect("Failed to create original file");
    writeln!(file, "Hello, world!").expect("Failed to write to original file");
    drop(file);

    // Create ExtendedRio instance
    let extended_rio = ExtendedRio::new().expect("Failed to create ExtendedRio");

    // Test hardlink creation
    let hardlink_path = temp_path.join("hardlink.txt");
    extended_rio
        .linkat(&original_file, &hardlink_path)
        .await
        .expect("Failed to create hardlink");

    // Verify hardlink was created
    assert!(hardlink_path.exists(), "Hardlink should exist");

    // Verify both files have the same content
    let original_content = std::fs::read_to_string(&original_file).expect("Failed to read original file");
    let hardlink_content = std::fs::read_to_string(&hardlink_path).expect("Failed to read hardlink");
    assert_eq!(original_content, hardlink_content, "Hardlink should have same content as original");
}

#[tokio::test]
async fn test_extended_rio_basic_functionality() {
    // Test that we can create an ExtendedRio instance
    let extended_rio = ExtendedRio::new().expect("Failed to create ExtendedRio");
    
    // Test that we can access the underlying rio instance
    let _rio_ref = extended_rio.rio();
    
    // This test verifies that our ExtendedRio struct works correctly
    // and can be instantiated without errors
    // If we get here, the creation succeeded
}

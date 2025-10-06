//! Tests for directory extended attributes (xattr) preservation

use compio::fs;
use compio_fs_extended::{ExtendedFile, XattrOps};
use io_uring_sync::directory::preserve_directory_xattr;
use tempfile::TempDir;

/// Test basic directory extended attributes preservation
#[compio::test]
async fn test_directory_xattr_preservation() {
    let temp_dir = TempDir::new().unwrap();
    let src_path = temp_dir.path().join("source_dir");
    let dst_path = temp_dir.path().join("destination_dir");

    // Create source directory
    fs::create_dir(&src_path).await.unwrap();

    // Set extended attributes on source directory
    let src_dir = fs::File::open(&src_path).await.unwrap();
    let extended_src = ExtendedFile::from_ref(&src_dir);

    // Set some test xattrs
    extended_src
        .set_xattr("user.test", b"test_value")
        .await
        .unwrap();
    extended_src
        .set_xattr("user.description", b"Test directory for xattr preservation")
        .await
        .unwrap();

    // Create destination directory
    fs::create_dir(&dst_path).await.unwrap();

    // Test xattr preservation
    preserve_directory_xattr(&src_path, &dst_path)
        .await
        .unwrap();

    // Verify xattrs were preserved
    let dst_dir = fs::File::open(&dst_path).await.unwrap();
    let extended_dst = ExtendedFile::from_ref(&dst_dir);
    let test_value = extended_dst.get_xattr("user.test").await.unwrap();
    let description_value = extended_dst.get_xattr("user.description").await.unwrap();

    assert_eq!(test_value, b"test_value");
    assert_eq!(description_value, b"Test directory for xattr preservation");
}

/// Test directory xattr preservation with no xattrs
#[compio::test]
async fn test_directory_xattr_preservation_no_xattrs() {
    let temp_dir = TempDir::new().unwrap();
    let src_path = temp_dir.path().join("source_dir");
    let dst_path = temp_dir.path().join("destination_dir");

    // Create source directory (no xattrs)
    fs::create_dir(&src_path).await.unwrap();

    // Create destination directory
    fs::create_dir(&dst_path).await.unwrap();

    // Test xattr preservation (should not fail)
    preserve_directory_xattr(&src_path, &dst_path)
        .await
        .unwrap();

    // Verify no xattrs were set
    let dst_dir = fs::File::open(&dst_path).await.unwrap();
    let extended_dst = ExtendedFile::from_ref(&dst_dir);
    let xattr_list = extended_dst.list_xattr().await.unwrap();
    assert!(xattr_list.is_empty());
}

/// Test directory xattr preservation with multiple xattrs
#[compio::test]
async fn test_directory_xattr_preservation_multiple() {
    let temp_dir = TempDir::new().unwrap();
    let src_path = temp_dir.path().join("source_dir");
    let dst_path = temp_dir.path().join("destination_dir");

    // Create source directory
    fs::create_dir(&src_path).await.unwrap();

    // Set multiple extended attributes on source directory
    let src_dir = fs::File::open(&src_path).await.unwrap();
    let extended_src = ExtendedFile::from_ref(&src_dir);

    let xattrs = vec![
        ("user.test1", b"value1".as_slice()),
        ("user.test2", b"value2".as_slice()),
        ("user.test3", b"value3".as_slice()),
        ("user.description", b"Multiple xattrs test".as_slice()),
    ];

    for (name, value) in &xattrs {
        extended_src.set_xattr(name, value).await.unwrap();
    }

    // Create destination directory
    fs::create_dir(&dst_path).await.unwrap();

    // Test xattr preservation
    preserve_directory_xattr(&src_path, &dst_path)
        .await
        .unwrap();

    // Verify all xattrs were preserved
    let dst_dir = fs::File::open(&dst_path).await.unwrap();
    let extended_dst = ExtendedFile::from_ref(&dst_dir);
    for (name, expected_value) in &xattrs {
        let actual_value = extended_dst.get_xattr(name).await.unwrap();
        assert_eq!(actual_value, *expected_value);
    }
}

/// Test directory xattr preservation with binary data
#[compio::test]
async fn test_directory_xattr_preservation_binary_data() {
    let temp_dir = TempDir::new().unwrap();
    let src_path = temp_dir.path().join("source_dir");
    let dst_path = temp_dir.path().join("destination_dir");

    // Create source directory
    fs::create_dir(&src_path).await.unwrap();

    // Set binary extended attribute on source directory
    let src_dir = fs::File::open(&src_path).await.unwrap();
    let extended_src = ExtendedFile::from_ref(&src_dir);

    let binary_data = vec![0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD, 0xFC];
    extended_src
        .set_xattr("user.binary", &binary_data)
        .await
        .unwrap();

    // Create destination directory
    fs::create_dir(&dst_path).await.unwrap();

    // Test xattr preservation
    preserve_directory_xattr(&src_path, &dst_path)
        .await
        .unwrap();

    // Verify binary xattr was preserved
    let dst_dir = fs::File::open(&dst_path).await.unwrap();
    let extended_dst = ExtendedFile::from_ref(&dst_dir);
    let preserved_data = extended_dst.get_xattr("user.binary").await.unwrap();
    assert_eq!(preserved_data, binary_data);
}

/// Test directory xattr preservation error handling
#[compio::test]
async fn test_directory_xattr_preservation_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let src_path = temp_dir.path().join("source_dir");
    let dst_path = temp_dir.path().join("destination_dir");

    // Create source directory
    fs::create_dir(&src_path).await.unwrap();

    // Set extended attribute on source directory
    let src_dir = fs::File::open(&src_path).await.unwrap();
    let extended_src = ExtendedFile::from_ref(&src_dir);
    extended_src
        .set_xattr("user.test", b"test_value")
        .await
        .unwrap();

    // Create destination directory
    fs::create_dir(&dst_path).await.unwrap();

    // Test xattr preservation (should not fail even if some xattrs can't be set)
    let result = preserve_directory_xattr(&src_path, &dst_path).await;

    // Should succeed (warnings are logged but don't fail the operation)
    assert!(result.is_ok());
}

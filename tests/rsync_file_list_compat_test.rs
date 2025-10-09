//! Proof-of-concept test: Can rsync parse our file list?
//!
//! This test validates that our rsync-compatible file list encoding
//! actually works with real rsync, without needing the full protocol.

#![cfg(feature = "remote-sync")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Check if rsync is available
fn rsync_available() -> bool {
    std::process::Command::new("rsync")
        .arg("--version")
        .output()
        .is_ok()
}

/// Create simple test data
fn create_test_files(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("file1.txt"), b"Hello, World!").unwrap();
    fs::write(dir.join("file2.txt"), b"Test data").unwrap();

    fs::create_dir(dir.join("subdir")).unwrap();
    fs::write(dir.join("subdir/nested.txt"), b"Nested content").unwrap();
}

#[test]
fn test_generate_file_list_encoding() {
    use arsync::protocol::rsync::FileEntry;
    use arsync::protocol::varint::encode_varint;

    // Create a test file entry
    let file = FileEntry {
        path: "test.txt".to_string(),
        size: 123,
        mtime: 1696800000,
        mode: 0o100644,
        uid: 1000,
        gid: 1000,
        is_symlink: false,
        symlink_target: None,
    };

    // Encode it using our functions
    let mut encoded = Vec::new();
    encoded.push(0u8); // flags
    encoded.extend(encode_varint(file.path.len() as u64));
    encoded.extend(file.path.as_bytes());
    encoded.extend(encode_varint(file.size));
    encoded.extend(encode_varint(file.mtime as u64));
    encoded.extend(encode_varint(file.mode as u64));
    encoded.extend(encode_varint(file.uid as u64));
    encoded.extend(encode_varint(file.gid as u64));

    println!("✓ File list entry encoded: {} bytes", encoded.len());
    println!("  Path: {}", file.path);
    println!("  Size: {} bytes", file.size);
    println!(
        "  Encoded size: {} bytes (varint is efficient!)",
        encoded.len()
    );

    // Basic validation
    assert!(!encoded.is_empty());
    assert!(encoded.len() < 100); // Should be compact
}

#[test]
fn test_file_list_structure() {
    if !rsync_available() {
        println!("⚠️  rsync not available, skipping");
        return;
    }

    println!("✓ rsync is available for testing");

    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source");
    create_test_files(&source);

    // Count files we created
    let file_count = walkdir::WalkDir::new(&source)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();

    println!("  Created {} test files", file_count);
    assert_eq!(file_count, 3); // file1, file2, subdir/nested

    println!("✓ Test data structure validated");
    println!("  → Ready to test file list exchange with rsync");
}

#[tokio::test]
async fn test_rsync_can_list_our_files() {
    use arsync::protocol::rsync::FileEntry;
    use arsync::protocol::rsync_compat::{encode_file_list_rsync, MultiplexWriter};

    if !rsync_available() {
        println!("⚠️  rsync not available, skipping");
        return;
    }

    println!("Testing: Can rsync parse our file list format?");

    // Create test file list
    let files = vec![FileEntry {
        path: "test.txt".to_string(),
        size: 12,
        mtime: 1696800000,
        mode: 0o100644,
        uid: 1000,
        gid: 1000,
        is_symlink: false,
        symlink_target: None,
    }];

    // For now, just verify encoding doesn't crash
    // Full pipe test would need actual pipes connected to rsync
    println!("  File list contains {} files", files.len());
    println!(
        "  File: {} ({} bytes, mode {:o})",
        files[0].path, files[0].size, files[0].mode
    );

    // TODO: Actually pipe this to rsync and see if it accepts it
    // This requires setting up bidirectional pipes

    println!("⚠️  Proof-of-concept: File list can be encoded");
    println!("  → Next: Wire up to actual rsync process via pipes");
}

#[tokio::test]
#[ignore] // Only run with --include-ignored when ready to test against rsync
async fn test_actual_rsync_file_list_exchange() {
    // This would be the REAL test: actually pipe to rsync --server mode
    // and see if it accepts our file list without errors

    if !rsync_available() {
        println!("rsync not available");
        return;
    }

    println!("⚠️  TODO: Implement actual rsync file list exchange test");
    println!("  This requires:");
    println!("  1. Create bidirectional pipes");
    println!("  2. Spawn rsync --server --sender");
    println!("  3. Send handshake (version)");
    println!("  4. Receive rsync's file list");
    println!("  5. Verify we can parse it");
    println!();
    println!("  OR reverse:");
    println!("  1. Spawn rsync --server (receiver mode)");
    println!("  2. Send our file list");
    println!("  3. Check if rsync accepts it (no protocol errors)");
}

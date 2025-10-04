//! Test data generation utilities

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::TempDir;

/// Create test directory structure with various file types
pub fn create_test_directory() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path();

    // Create directory structure
    fs::create_dir_all(test_dir.join("subdir1")).unwrap();
    fs::create_dir_all(test_dir.join("subdir2")).unwrap();
    fs::create_dir_all(test_dir.join("subdir1/nested")).unwrap();

    // Create files of various sizes
    create_test_file(test_dir.join("small_file.txt"), 1024);
    create_test_file(test_dir.join("medium_file.bin"), 1024 * 1024);
    create_test_file(test_dir.join("large_file.bin"), 10 * 1024 * 1024);
    create_test_file(test_dir.join("subdir1/file.txt"), 512);
    create_test_file(test_dir.join("subdir1/nested/deep.txt"), 256);
    create_test_file(test_dir.join("subdir2/another.bin"), 2048);

    temp_dir
}

/// Create a test file with specified size
fn create_test_file(path: std::path::PathBuf, size: usize) {
    let data = vec![0u8; size];
    fs::write(path, data).unwrap();
}

/// Create test files with extended attributes
pub fn create_test_directory_with_xattrs() -> TempDir {
    let temp_dir = create_test_directory();
    let test_dir = temp_dir.path();

    // Add extended attributes to some files
    let file_path = test_dir.join("small_file.txt");

    // Set a user attribute
    xattr::set(&file_path, "user.test_attr", "test_value".as_bytes()).unwrap();

    // Set a system attribute (if supported)
    if cfg!(target_os = "linux") {
        xattr::set(
            &file_path,
            "user.comment",
            "Test file with xattrs".as_bytes(),
        )
        .unwrap();
    }

    temp_dir
}

/// Verify that two directories have identical content
pub fn verify_directory_identical(src: &Path, dst: &Path) -> Result<(), String> {
    let src_entries = collect_directory_entries(src)?;
    let dst_entries = collect_directory_entries(dst)?;

    if src_entries.len() != dst_entries.len() {
        return Err(format!(
            "Directory entry count mismatch: {} vs {}",
            src_entries.len(),
            dst_entries.len()
        ));
    }

    for (src_entry, dst_entry) in src_entries.iter().zip(dst_entries.iter()) {
        if src_entry != dst_entry {
            return Err(format!(
                "Entry mismatch: {:?} vs {:?}",
                src_entry, dst_entry
            ));
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
struct DirectoryEntry {
    path: String,
    size: u64,
    permissions: u32,
    modified: std::time::SystemTime,
}

fn collect_directory_entries(dir: &Path) -> Result<Vec<DirectoryEntry>, String> {
    let mut entries = Vec::new();

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.map_err(|e| format!("WalkDir error: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            let metadata =
                fs::metadata(path).map_err(|e| format!("Metadata error for {:?}: {}", path, e))?;

            entries.push(DirectoryEntry {
                path: path
                    .strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
                size: metadata.len(),
                permissions: metadata.permissions().mode(),
                modified: metadata
                    .modified()
                    .map_err(|e| format!("Modified time error: {}", e))?,
            });
        }
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

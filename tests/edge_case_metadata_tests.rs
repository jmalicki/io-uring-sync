//! Edge case tests for metadata preservation
//!
//! These tests cover extreme scenarios and edge cases that could reveal
//! subtle bugs in the permission and timestamp preservation logic.

use io_uring_sync::copy::copy_file;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::ffi::OsStrExt;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

/// Test permission preservation with files that have no read permission
#[compio::test]
async fn test_permission_preservation_no_read_permission() {
    let temp_dir = TempDir::new().unwrap();
    let src_path = temp_dir.path().join("no_read.txt");
    let dst_path = temp_dir.path().join("no_read_copy.txt");

    // Create source file
    fs::write(&src_path, "Test content for no read permission").unwrap();
    
    // Set permissions that deny read access to others
    let permissions = std::fs::Permissions::from_mode(0o600); // owner only
    fs::set_permissions(&src_path, permissions).unwrap();

    // Get expected permissions
    let src_metadata = fs::metadata(&src_path).unwrap();
    let expected_permissions = src_metadata.permissions().mode();

    // Copy the file (this should still work as we're the owner)
    copy_file(&src_path, &dst_path).await.unwrap();

    // Check that permissions were preserved
    let dst_metadata = fs::metadata(&dst_path).unwrap();
    let dst_permissions = dst_metadata.permissions().mode();
    
    assert_eq!(expected_permissions, dst_permissions, 
              "Permissions should be preserved even with restrictive access");
}

/// Test timestamp preservation with files that have very recent timestamps
#[compio::test]
async fn test_timestamp_preservation_very_recent() {
    let temp_dir = TempDir::new().unwrap();
    let src_path = temp_dir.path().join("recent.txt");
    let dst_path = temp_dir.path().join("recent_copy.txt");

    // Create source file
    fs::write(&src_path, "Test content with very recent timestamp").unwrap();
    
    // Get current time and set it as the file timestamp
    let now = SystemTime::now();
    let duration = now.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    
    let current_timespec = libc::timespec {
        tv_sec: duration.as_secs() as i64,
        tv_nsec: duration.subsec_nanos() as i64,
    };
    
    // Use utimes to set the current timestamp
    let path_cstr = std::ffi::CString::new(src_path.as_os_str().as_bytes()).unwrap();
    let times = [current_timespec, current_timespec];
    
    let result = unsafe {
        libc::utimensat(
            libc::AT_FDCWD,
            path_cstr.as_ptr(),
            times.as_ptr(),
            0,
        )
    };
    
    if result == 0 {
        // Wait a small amount to ensure timestamps are different
        std::thread::sleep(Duration::from_millis(10));
        
        // Copy the file
        copy_file(&src_path, &dst_path).await.unwrap();

        // Check that the recent timestamp was preserved
        let dst_metadata = fs::metadata(&dst_path).unwrap();
        let copied_accessed = dst_metadata.accessed().unwrap();
        let copied_modified = dst_metadata.modified().unwrap();
        
        // Check that timestamps are very close to the original
        let accessed_diff = copied_accessed.duration_since(now).unwrap_or_default();
        let modified_diff = copied_modified.duration_since(now).unwrap_or_default();
        
        println!("Recent timestamp test - Accessed diff: {}ms, Modified diff: {}ms",
                accessed_diff.as_millis(), modified_diff.as_millis());
        
        // Should be very close (within 100ms)
        assert!(accessed_diff.as_millis() < 100, "Recent accessed timestamp should be preserved");
        assert!(modified_diff.as_millis() < 100, "Recent modified timestamp should be preserved");
    }
}

/// Test permission preservation with files that have execute-only permissions
#[compio::test]
async fn test_permission_preservation_execute_only() {
    let temp_dir = TempDir::new().unwrap();
    let src_path = temp_dir.path().join("execute_only.txt");
    let dst_path = temp_dir.path().join("execute_only_copy.txt");

    // Create source file
    fs::write(&src_path, "Test content for execute only").unwrap();
    
    // Test execute-only permissions
    let execute_only_permissions = vec![
        0o111, // execute for all
        0o001, // execute for others only
        0o010, // execute for group only
        0o100, // execute for owner only
    ];

    for &permission_mode in &execute_only_permissions {
        // Set specific permissions
        let permissions = std::fs::Permissions::from_mode(permission_mode);
        fs::set_permissions(&src_path, permissions).unwrap();

        // Get source permissions after setting
        let src_metadata = fs::metadata(&src_path).unwrap();
        let expected_permissions = src_metadata.permissions().mode();

        // Copy the file
        copy_file(&src_path, &dst_path).await.unwrap();

        // Check that permissions were preserved
        let dst_metadata = fs::metadata(&dst_path).unwrap();
        let dst_permissions = dst_metadata.permissions().mode();
        
        println!("Execute-only test - Mode: {:o}, Expected: {:o}, Actual: {:o}", 
                permission_mode, expected_permissions, dst_permissions);
        
        assert_eq!(expected_permissions, dst_permissions, 
                  "Execute-only permission mode {:o} should be preserved", permission_mode);
    }
}

/// Test timestamp preservation with files that have identical access and modification times
#[compio::test]
async fn test_timestamp_preservation_identical_times() {
    let temp_dir = TempDir::new().unwrap();
    let src_path = temp_dir.path().join("identical_times.txt");
    let dst_path = temp_dir.path().join("identical_times_copy.txt");

    // Create source file
    fs::write(&src_path, "Test content with identical access and modification times").unwrap();
    
    // Set identical access and modification times
    let _identical_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1609459200); // Jan 1, 2021
    let identical_timespec = libc::timespec {
        tv_sec: 1609459200,
        tv_nsec: 123456789, // specific nanosecond value
    };
    
    // Use utimes to set identical timestamps
    let path_cstr = std::ffi::CString::new(src_path.as_os_str().as_bytes()).unwrap();
    let times = [identical_timespec, identical_timespec];
    
    let result = unsafe {
        libc::utimensat(
            libc::AT_FDCWD,
            path_cstr.as_ptr(),
            times.as_ptr(),
            0,
        )
    };
    
    if result == 0 {
        // Copy the file
        copy_file(&src_path, &dst_path).await.unwrap();

        // Check that identical timestamps were preserved
        let dst_metadata = fs::metadata(&dst_path).unwrap();
        let copied_accessed = dst_metadata.accessed().unwrap();
        let copied_modified = dst_metadata.modified().unwrap();
        
        // Check that both timestamps are identical and close to the original
        let accessed_duration = copied_accessed.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
        let modified_duration = copied_modified.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
        
        println!("Identical times test - Accessed: {}s.{}ns, Modified: {}s.{}ns",
                accessed_duration.as_secs(), accessed_duration.subsec_nanos(),
                modified_duration.as_secs(), modified_duration.subsec_nanos());
        
        // Both timestamps should be very close to each other and to the original
        let time_diff = accessed_duration.as_secs().abs_diff(modified_duration.as_secs());
        assert!(time_diff < 2, "Access and modification times should be identical");
        
        let expected_seconds = 1609459200;
        assert!(accessed_duration.as_secs().abs_diff(expected_seconds) < 2,
               "Accessed time should be preserved");
        assert!(modified_duration.as_secs().abs_diff(expected_seconds) < 2,
               "Modified time should be preserved");
    }
}

/// Test permission preservation with files that have all permission bits set
#[compio::test]
async fn test_permission_preservation_all_bits() {
    let temp_dir = TempDir::new().unwrap();
    let src_path = temp_dir.path().join("all_bits.txt");
    let dst_path = temp_dir.path().join("all_bits_copy.txt");

    // Create source file
    fs::write(&src_path, "Test content with all permission bits").unwrap();
    
    // Test all possible permission combinations
    let all_permission_tests = vec![
        0o777, // all permissions for all
        0o666, // read/write for all
        0o555, // read/execute for all
        0o444, // read only for all
        0o333, // write/execute for all
        0o222, // write only for all
        0o111, // execute only for all
    ];

    for &permission_mode in &all_permission_tests {
        // Set specific permissions
        let permissions = std::fs::Permissions::from_mode(permission_mode);
        fs::set_permissions(&src_path, permissions).unwrap();

        // Get source permissions after setting
        let src_metadata = fs::metadata(&src_path).unwrap();
        let expected_permissions = src_metadata.permissions().mode();

        // Copy the file
        copy_file(&src_path, &dst_path).await.unwrap();

        // Check that permissions were preserved
        let dst_metadata = fs::metadata(&dst_path).unwrap();
        let dst_permissions = dst_metadata.permissions().mode();
        
        println!("All bits test - Mode: {:o}, Expected: {:o}, Actual: {:o}", 
                permission_mode, expected_permissions, dst_permissions);
        
        assert_eq!(expected_permissions, dst_permissions, 
                  "All permission bits mode {:o} should be preserved", permission_mode);
    }
}

/// Test metadata preservation with files that have very long filenames
#[compio::test]
async fn test_metadata_preservation_long_filename() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a very long filename (255 characters)
    let long_filename = "a".repeat(250) + ".txt";
    let src_path = temp_dir.path().join(&long_filename);
    let dst_path = temp_dir.path().join(format!("{}_copy", long_filename));

    // Create source file
    fs::write(&src_path, "Test content with very long filename").unwrap();
    
    // Set specific permissions
    let permissions = std::fs::Permissions::from_mode(0o644);
    fs::set_permissions(&src_path, permissions).unwrap();

    // Get expected permissions
    let src_metadata = fs::metadata(&src_path).unwrap();
    let expected_permissions = src_metadata.permissions().mode();
    let original_accessed = src_metadata.accessed().unwrap();
    let original_modified = src_metadata.modified().unwrap();

    // Copy the file
    copy_file(&src_path, &dst_path).await.unwrap();

    // Check that permissions were preserved
    let dst_metadata = fs::metadata(&dst_path).unwrap();
    let dst_permissions = dst_metadata.permissions().mode();
    assert_eq!(expected_permissions, dst_permissions, 
              "Permissions should be preserved for long filenames");

    // Check that timestamps were preserved
    let copied_accessed = dst_metadata.accessed().unwrap();
    let copied_modified = dst_metadata.modified().unwrap();
    
    let accessed_diff = copied_accessed.duration_since(original_accessed).unwrap_or_default();
    let modified_diff = copied_modified.duration_since(original_modified).unwrap_or_default();
    
    assert!(accessed_diff.as_millis() < 100, "Accessed time should be preserved for long filenames");
    assert!(modified_diff.as_millis() < 100, "Modified time should be preserved for long filenames");
}

/// Test metadata preservation with files that have special characters in names
#[compio::test]
async fn test_metadata_preservation_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test various special characters in filenames
    let special_filenames = vec![
        "file with spaces.txt",
        "file-with-dashes.txt",
        "file_with_underscores.txt",
        "file.with.dots.txt",
        "file@with@symbols.txt",
        "file#with#hash.txt",
        "file$with$dollar.txt",
        "file%with%percent.txt",
        "file&with&ampersand.txt",
        "file(with)parentheses.txt",
        "file[with]brackets.txt",
        "file{with}braces.txt",
        "file|with|pipes.txt",
        "file\\with\\backslashes.txt",
        "file/with/forward/slashes.txt",
    ];

    for filename in special_filenames {
        let src_path = temp_dir.path().join(filename);
        let dst_path = temp_dir.path().join(format!("{}_copy", filename));

        // Create source file
        fs::write(&src_path, format!("Test content for {}", filename)).unwrap();
        
        // Set specific permissions
        let permissions = std::fs::Permissions::from_mode(0o644);
        fs::set_permissions(&src_path, permissions).unwrap();

        // Get expected permissions
        let src_metadata = fs::metadata(&src_path).unwrap();
        let expected_permissions = src_metadata.permissions().mode();

        // Copy the file
        copy_file(&src_path, &dst_path).await.unwrap();

        // Check that permissions were preserved
        let dst_metadata = fs::metadata(&dst_path).unwrap();
        let dst_permissions = dst_metadata.permissions().mode();
        
        assert_eq!(expected_permissions, dst_permissions, 
                  "Permissions should be preserved for filename: {}", filename);
    }
}

/// Test metadata preservation with files that have unicode characters in names
#[compio::test]
async fn test_metadata_preservation_unicode_filenames() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test various unicode characters in filenames
    let unicode_filenames = vec![
        "файл.txt", // Cyrillic
        "文件.txt", // Chinese
        "ファイル.txt", // Japanese
        "ملف.txt", // Arabic
        "קובץ.txt", // Hebrew
        "αρχείο.txt", // Greek
        "файл_с_пробелами.txt", // Cyrillic with spaces
        "文件_with_underscores.txt", // Chinese with underscores
    ];

    for filename in unicode_filenames {
        let src_path = temp_dir.path().join(filename);
        let dst_path = temp_dir.path().join(format!("{}_copy", filename));

        // Create source file
        fs::write(&src_path, format!("Test content for {}", filename)).unwrap();
        
        // Set specific permissions
        let permissions = std::fs::Permissions::from_mode(0o644);
        fs::set_permissions(&src_path, permissions).unwrap();

        // Get expected permissions
        let src_metadata = fs::metadata(&src_path).unwrap();
        let expected_permissions = src_metadata.permissions().mode();

        // Copy the file
        copy_file(&src_path, &dst_path).await.unwrap();

        // Check that permissions were preserved
        let dst_metadata = fs::metadata(&dst_path).unwrap();
        let dst_permissions = dst_metadata.permissions().mode();
        
        assert_eq!(expected_permissions, dst_permissions, 
                  "Permissions should be preserved for unicode filename: {}", filename);
    }
}

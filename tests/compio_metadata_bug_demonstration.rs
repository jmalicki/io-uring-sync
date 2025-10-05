#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Comprehensive demonstration of compio::fs::metadata timestamp corruption bug
//!
//! This test suite provides irrefutable evidence that compio::fs::metadata
//! has a critical bug that corrupts file modification timestamps.

use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::time::SystemTime;
use tempfile::TempDir;

/// Test case demonstrating the compio::fs::metadata bug
#[compio::test]
async fn test_compio_metadata_bug_basic() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("bug_demo.txt");

    // Create test file
    fs::write(&test_file, "compio metadata bug demonstration").unwrap();

    // Set a specific timestamp: January 1, 2021, 12:34:56.789123456
    let target_secs = 1609459200; // Jan 1, 2021 00:00:00
    let target_nanos = 123456789; // 123456789 nanoseconds

    let path_cstr = CString::new(test_file.as_os_str().as_bytes()).unwrap();
    let timespec = libc::timespec {
        tv_sec: target_secs,
        tv_nsec: target_nanos,
    };
    let times = [timespec, timespec];
    let result = unsafe { libc::utimensat(libc::AT_FDCWD, path_cstr.as_ptr(), times.as_ptr(), 0) };

    assert_eq!(result, 0, "Failed to set timestamp");

    // Test compio::fs::metadata (BROKEN)
    let compio_metadata = compio::fs::metadata(&test_file).await.unwrap();
    let compio_accessed = compio_metadata.accessed().unwrap();
    let compio_modified = compio_metadata.modified().unwrap();

    // Test libc::stat (WORKS)
    let path_cstr = CString::new(test_file.as_os_str().as_bytes()).unwrap();
    let mut stat_buf: libc::stat = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::stat(path_cstr.as_ptr(), &mut stat_buf) };
    assert_eq!(result, 0, "libc::stat failed");

    let libc_accessed = SystemTime::UNIX_EPOCH
        + std::time::Duration::new(stat_buf.st_atime as u64, stat_buf.st_atime_nsec as u32);
    let libc_modified = SystemTime::UNIX_EPOCH
        + std::time::Duration::new(stat_buf.st_mtime as u64, stat_buf.st_mtime_nsec as u32);

    // Analyze the results
    let compio_accessed_duration = compio_accessed
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let compio_modified_duration = compio_modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let libc_accessed_duration = libc_accessed
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let libc_modified_duration = libc_modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    println!("=== COMPIO::FS::METADATA BUG DEMONSTRATION ===");
    println!("Target timestamp: {}s.{}ns", target_secs, target_nanos);
    println!();
    println!("compio::fs::metadata results:");
    println!(
        "  Accessed: {}s.{}ns",
        compio_accessed_duration.as_secs(),
        compio_accessed_duration.subsec_nanos()
    );
    println!(
        "  Modified: {}s.{}ns",
        compio_modified_duration.as_secs(),
        compio_modified_duration.subsec_nanos()
    );
    println!();
    println!("libc::stat results:");
    println!(
        "  Accessed: {}s.{}ns",
        libc_accessed_duration.as_secs(),
        libc_accessed_duration.subsec_nanos()
    );
    println!(
        "  Modified: {}s.{}ns",
        libc_modified_duration.as_secs(),
        libc_modified_duration.subsec_nanos()
    );

    // The smoking gun: compio returns 0 for modified time
    assert_eq!(
        compio_modified_duration.as_secs(),
        0,
        "compio::fs::metadata returns 0 seconds for modified time - this is the bug!"
    );
    assert_eq!(
        compio_modified_duration.subsec_nanos(),
        0,
        "compio::fs::metadata returns 0 nanoseconds for modified time - complete data loss!"
    );

    // libc::stat works correctly
    assert_eq!(
        libc_modified_duration.as_secs(),
        target_secs as u64,
        "libc::stat correctly returns the target seconds"
    );
    assert_eq!(
        libc_modified_duration.subsec_nanos(),
        target_nanos as u32,
        "libc::stat correctly returns the target nanoseconds"
    );

    println!();
    println!("🚨 BUG CONFIRMED: compio::fs::metadata corrupts modified timestamps!");
    println!("✅ libc::stat works correctly with full precision");
}

/// Test multiple timestamp scenarios to prove the bug is systematic
#[compio::test]
async fn test_compio_metadata_bug_multiple_scenarios() {
    let temp_dir = TempDir::new().unwrap();

    // Test various timestamp scenarios
    let test_cases = vec![
        (946684800, 0, "Y2K epoch"),
        (946684800, 123456789, "Y2K with nanoseconds"),
        (1609459200, 0, "2021 New Year"),
        (1609459200, 999999999, "2021 with max nanoseconds"),
        (0, 0, "Unix epoch"),
        (0, 1, "Unix epoch + 1ns"),
    ];

    for (target_secs, target_nanos, description) in test_cases {
        let test_file = temp_dir
            .path()
            .join(format!("test_{}_{}.txt", target_secs, target_nanos));
        fs::write(&test_file, format!("Test file for {}", description)).unwrap();

        // Set timestamp
        let path_cstr = CString::new(test_file.as_os_str().as_bytes()).unwrap();
        let timespec = libc::timespec {
            tv_sec: target_secs,
            tv_nsec: target_nanos,
        };
        let times = [timespec, timespec];
        let result =
            unsafe { libc::utimensat(libc::AT_FDCWD, path_cstr.as_ptr(), times.as_ptr(), 0) };
        assert_eq!(result, 0, "Failed to set timestamp for {}", description);

        // Test compio::fs::metadata
        let compio_metadata = compio::fs::metadata(&test_file).await.unwrap();
        let compio_modified = compio_metadata.modified().unwrap();
        let compio_modified_duration = compio_modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();

        // Test libc::stat
        let path_cstr = CString::new(test_file.as_os_str().as_bytes()).unwrap();
        let mut stat_buf: libc::stat = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::stat(path_cstr.as_ptr(), &mut stat_buf) };
        assert_eq!(result, 0, "libc::stat failed for {}", description);

        let libc_modified = SystemTime::UNIX_EPOCH
            + std::time::Duration::new(stat_buf.st_mtime as u64, stat_buf.st_mtime_nsec as u32);
        let libc_modified_duration = libc_modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();

        println!("=== {} ===", description);
        println!("Target: {}s.{}ns", target_secs, target_nanos);
        println!(
            "compio modified: {}s.{}ns",
            compio_modified_duration.as_secs(),
            compio_modified_duration.subsec_nanos()
        );
        println!(
            "libc modified: {}s.{}ns",
            libc_modified_duration.as_secs(),
            libc_modified_duration.subsec_nanos()
        );

        // Verify the bug is consistent
        assert_eq!(
            compio_modified_duration.as_secs(),
            0,
            "compio::fs::metadata bug: modified time is 0 for {}",
            description
        );
        assert_eq!(
            libc_modified_duration.as_secs(),
            target_secs as u64,
            "libc::stat works correctly for {}",
            description
        );

        println!("✅ Bug confirmed for {}", description);
    }

    println!("\n🚨 SYSTEMATIC BUG: compio::fs::metadata corrupts ALL modified timestamps!");
}

/// Test the impact on real-world scenarios
#[compio::test]
async fn test_compio_metadata_bug_real_world_impact() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("important_file.txt");

    // Simulate a file that was modified recently
    fs::write(&test_file, "Important file content").unwrap();

    // Set a recent timestamp (1 hour ago)
    let one_hour_ago = SystemTime::now() - std::time::Duration::from_secs(3600);
    let one_hour_ago_duration = one_hour_ago.duration_since(SystemTime::UNIX_EPOCH).unwrap();

    let path_cstr = CString::new(test_file.as_os_str().as_bytes()).unwrap();
    let timespec = libc::timespec {
        tv_sec: one_hour_ago_duration.as_secs() as i64,
        tv_nsec: one_hour_ago_duration.subsec_nanos() as i64,
    };
    let times = [timespec, timespec];
    let result = unsafe { libc::utimensat(libc::AT_FDCWD, path_cstr.as_ptr(), times.as_ptr(), 0) };
    assert_eq!(result, 0, "Failed to set timestamp");

    // Test compio::fs::metadata (BROKEN)
    let compio_metadata = compio::fs::metadata(&test_file).await.unwrap();
    let compio_modified = compio_metadata.modified().unwrap();

    // Test libc::stat (WORKS)
    let path_cstr = CString::new(test_file.as_os_str().as_bytes()).unwrap();
    let mut stat_buf: libc::stat = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::stat(path_cstr.as_ptr(), &mut stat_buf) };
    assert_eq!(result, 0, "libc::stat failed");

    let libc_modified = SystemTime::UNIX_EPOCH
        + std::time::Duration::new(stat_buf.st_mtime as u64, stat_buf.st_mtime_nsec as u32);

    println!("=== REAL-WORLD IMPACT DEMONSTRATION ===");
    println!("File was modified 1 hour ago");
    println!("compio::fs::metadata says: {:?}", compio_modified);
    println!("libc::stat says: {:?}", libc_modified);

    // Demonstrate the impact
    let now = SystemTime::now();
    let compio_age = now.duration_since(compio_modified).unwrap();
    let libc_age = now.duration_since(libc_modified).unwrap();

    println!("compio thinks file is {} seconds old", compio_age.as_secs());
    println!("libc thinks file is {} seconds old", libc_age.as_secs());

    // The bug makes the file appear to be from 1970 (epoch)
    assert!(
        compio_age.as_secs() > 50 * 365 * 24 * 3600,
        "compio::fs::metadata makes file appear to be from 1970!"
    );
    assert!(
        libc_age.as_secs() < 2 * 3600,
        "libc::stat correctly identifies recent file"
    );

    println!("🚨 IMPACT: compio::fs::metadata makes recent files appear 50+ years old!");
    println!("✅ libc::stat correctly identifies file age");
}

/// Test precision loss analysis
#[compio::test]
async fn test_compio_metadata_bug_precision_analysis() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("precision_test.txt");
    fs::write(&test_file, "Precision test").unwrap();

    // Test with maximum nanosecond precision
    let target_secs = 1609459200;
    let target_nanos = 999999999; // Maximum nanoseconds

    let path_cstr = CString::new(test_file.as_os_str().as_bytes()).unwrap();
    let timespec = libc::timespec {
        tv_sec: target_secs,
        tv_nsec: target_nanos,
    };
    let times = [timespec, timespec];
    let result = unsafe { libc::utimensat(libc::AT_FDCWD, path_cstr.as_ptr(), times.as_ptr(), 0) };
    assert_eq!(result, 0, "Failed to set timestamp");

    // Test compio::fs::metadata
    let compio_metadata = compio::fs::metadata(&test_file).await.unwrap();
    let compio_modified = compio_metadata.modified().unwrap();
    let compio_modified_duration = compio_modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    // Test libc::stat
    let path_cstr = CString::new(test_file.as_os_str().as_bytes()).unwrap();
    let mut stat_buf: libc::stat = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::stat(path_cstr.as_ptr(), &mut stat_buf) };
    assert_eq!(result, 0, "libc::stat failed");

    let libc_modified = SystemTime::UNIX_EPOCH
        + std::time::Duration::new(stat_buf.st_mtime as u64, stat_buf.st_mtime_nsec as u32);
    let libc_modified_duration = libc_modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    println!("=== PRECISION ANALYSIS ===");
    println!(
        "Target: {}s.{}ns (maximum precision)",
        target_secs, target_nanos
    );
    println!(
        "compio precision: {}s.{}ns",
        compio_modified_duration.as_secs(),
        compio_modified_duration.subsec_nanos()
    );
    println!(
        "libc precision: {}s.{}ns",
        libc_modified_duration.as_secs(),
        libc_modified_duration.subsec_nanos()
    );

    // Calculate precision loss
    let compio_nanos = compio_modified_duration.subsec_nanos();
    let libc_nanos = libc_modified_duration.subsec_nanos();
    let expected_nanos = target_nanos;

    let compio_loss = (compio_nanos as i32 - expected_nanos as i32).abs();
    let libc_loss = (libc_nanos as i32 - expected_nanos as i32).abs();

    println!("Expected nanoseconds: {}", expected_nanos);
    println!("compio precision loss: {}ns", compio_loss);
    println!("libc precision loss: {}ns", libc_loss);
    println!(
        "compio data loss: {:.1}%",
        (compio_loss as f64 / expected_nanos as f64) * 100.0
    );
    println!(
        "libc data loss: {:.1}%",
        (libc_loss as f64 / expected_nanos as f64) * 100.0
    );

    // Verify the precision loss
    assert_eq!(
        compio_nanos, 0,
        "compio::fs::metadata loses ALL nanosecond precision"
    );
    assert_eq!(
        libc_nanos, expected_nanos as u32,
        "libc::stat preserves full precision"
    );

    println!("🚨 PRECISION LOSS: compio::fs::metadata loses 100% of nanosecond precision!");
    println!("✅ libc::stat preserves full nanosecond precision!");
}

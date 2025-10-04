//! Integration tests for io-uring-sync

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_help_output() {
    let mut cmd = Command::cargo_bin("io-uring-sync").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("High-performance bulk file copying utility"));
}

#[test]
fn test_version_output() {
    let mut cmd = Command::cargo_bin("io-uring-sync").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("io-uring-sync"));
}

#[test]
fn test_missing_source() {
    let temp_dir = TempDir::new().unwrap();
    
    let mut cmd = Command::cargo_bin("io-uring-sync").unwrap();
    cmd.args([
        "--source",
        "/nonexistent/path",
        "--destination",
        temp_dir.path().to_str().unwrap(),
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("Source path does not exist"));
}

#[test]
fn test_invalid_queue_depth() {
    let temp_dir = TempDir::new().unwrap();
    
    let mut cmd = Command::cargo_bin("io-uring-sync").unwrap();
    cmd.args([
        "--source",
        temp_dir.path().to_str().unwrap(),
        "--destination",
        temp_dir.path().to_str().unwrap(),
        "--queue-depth",
        "100", // Too small
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("Queue depth must be between 1024 and 65536"));
}

#[test]
fn test_dry_run() {
    let temp_dir = TempDir::new().unwrap();
    
    let mut cmd = Command::cargo_bin("io-uring-sync").unwrap();
    cmd.args([
        "--source",
        temp_dir.path().to_str().unwrap(),
        "--destination",
        temp_dir.path().to_str().unwrap(),
        "--dry-run",
    ])
    .assert()
    .success();
}

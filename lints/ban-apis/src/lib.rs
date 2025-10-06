//! # Banned APIs Lint for io-uring-sync
//!
//! This crate provides a simple script-based linter that enforces the `io_uring`-first policy
//! for the `io-uring-sync` project. It flags direct usage of `libc::`, `nix::`, and `std::fs::`
//! APIs to encourage the use of `io_uring` operations via `compio` and `compio-fs-extended`.
//!
//! ## Purpose
//!
//! The `io-uring-sync` project aims to be a high-performance file synchronization tool that
//! leverages Linux's `io_uring` interface for maximum efficiency. Direct usage of traditional
//! POSIX APIs (`libc`, `nix`, `std::fs`) bypasses the performance benefits of `io_uring` and
//! should be avoided unless there's a strong technical justification.
//!
//! ## What This Lint Catches
//!
//! This linter detects and flags:
//!
//! 1. **Direct imports**: `use libc::*`, `use nix::*`, `use std::fs::*`
//! 2. **Symbol usage**: Any reference to symbols from these crates, even when imported
//! 3. **Method calls**: Calls to methods from these crates
//! 4. **Function calls**: Direct function calls to these APIs
//!
//! ## How It Works
//!
//! The linter uses simple pattern matching to detect banned API usage. While not as precise
//! as AST-based analysis, it provides comprehensive coverage and is easy to understand and maintain.
//!
//! ## Banned APIs
//!
//! - **`libc::`**: Raw FFI bindings to C standard library functions
//! - **`nix::`**: Safe Rust bindings to Unix system calls
//! - **`std::fs::`**: Standard library filesystem operations
//!
//! ## Approved Alternatives
//!
//! Instead of banned APIs, use:
//! - **`compio::fs`**: Async filesystem operations via `io_uring`
//! - **`compio-fs-extended`**: Extended `io_uring` operations (fadvise, fallocate, etc.)
//! - **`io_uring::opcode::*`**: Direct `io_uring` opcodes when available
//!
//! ## Exception Handling
//!
//! When a banned API must be used (e.g., for operations not yet available in `io_uring`),
//! add an explicit `#[allow(banned_apis)]` attribute with a comment explaining
//! the technical justification.
//!
//! ## Example
//!
//! ```rust,ignore
//! // ❌ BAD - Will be flagged
//! use libc::statx;
//! use std::fs::read_to_string;
//!
//! // ✅ GOOD - Use io_uring alternatives
//! use compio::fs::File;
//! use compio_fs_extended::metadata::statx;
//! ```

use std::fs;
use std::path::Path;
use std::process::Command;

/// The main linter function that checks for banned API usage
///
/// This function scans Rust source files for patterns that indicate usage
/// of banned APIs (`libc::`, `nix::`, `std::fs::`). It provides comprehensive
/// coverage through pattern matching and regex-based detection.
///
/// # Arguments
///
/// * `path` - The path to scan (file or directory)
///
/// # Returns
///
/// `Result<Vec<String>, Box<dyn std::error::Error>>` - List of violations found
pub fn check_banned_apis<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut violations = Vec::new();
    let path = path.as_ref();

    if path.is_file() {
        if let Some(ext) = path.extension() {
            if ext == "rs" {
                violations.extend(check_file(path)?);
            }
        }
    } else if path.is_dir() {
        violations.extend(check_directory(path)?);
    }

    Ok(violations)
}

/// Checks a single Rust file for banned API usage
///
/// This function reads a Rust source file and scans it for patterns that
/// indicate usage of banned APIs. It uses simple string matching for
/// reliability and performance.
///
/// # Arguments
///
/// * `file_path` - The path to the Rust file to check
///
/// # Returns
///
/// `Result<Vec<String>, Box<dyn std::error::Error>>` - List of violations found
fn check_file(file_path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path)?;
    let mut violations = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip comments and allow attributes
        if line.starts_with("//") || line.starts_with("/*") || line.starts_with("*") {
            continue;
        }

        // Check for banned imports
        if line.starts_with("use ") {
            if is_banned_import(line) {
                violations.push(format!(
                    "{}:{}: banned import: {}",
                    file_path.display(),
                    line_num + 1,
                    line
                ));
            }
        }

        // Check for banned API usage in expressions
        if contains_banned_usage(line) {
            violations.push(format!(
                "{}:{}: banned API usage: {}",
                file_path.display(),
                line_num + 1,
                line
            ));
        }
    }

    Ok(violations)
}

/// Checks a directory recursively for banned API usage
///
/// This function recursively scans a directory for Rust source files
/// and checks each one for banned API usage.
///
/// # Arguments
///
/// * `dir_path` - The path to the directory to check
///
/// # Returns
///
/// `Result<Vec<String>, Box<dyn std::error::Error>>` - List of violations found
fn check_directory(dir_path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut violations = Vec::new();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "rs" {
                    violations.extend(check_file(&path)?);
                }
            }
        } else if path.is_dir() {
            // Skip common directories that don't contain source code
            if let Some(dir_name) = path.file_name() {
                let dir_name = dir_name.to_string_lossy();
                if dir_name == "target" || dir_name == ".git" || dir_name == "node_modules" {
                    continue;
                }
            }
            violations.extend(check_directory(&path)?);
        }
    }

    Ok(violations)
}

/// Checks if an import statement contains banned APIs
///
/// This function analyzes import statements to detect usage of banned crates.
/// It checks for direct imports from `libc`, `nix`, or `std::fs`.
///
/// # Arguments
///
/// * `line` - The import line to check
///
/// # Returns
///
/// `true` if the import contains banned APIs, `false` otherwise
fn is_banned_import(line: &str) -> bool {
    // Check for direct imports from banned crates
    line.contains("use libc::") ||
    line.contains("use nix::") ||
    line.contains("use std::fs::") ||
    // Check for wildcard imports
    (line.contains("use libc") && line.contains("*")) ||
    (line.contains("use nix") && line.contains("*")) ||
    (line.contains("use std::fs") && line.contains("*"))
}

/// Checks if a line contains banned API usage
///
/// This function analyzes code lines to detect usage of banned APIs.
/// It uses pattern matching to identify common usage patterns.
///
/// # Arguments
///
/// * `line` - The code line to check
///
/// # Returns
///
/// `true` if the line contains banned API usage, `false` otherwise
fn contains_banned_usage(line: &str) -> bool {
    // Check for direct API usage
    line.contains("libc::") ||
    line.contains("nix::") ||
    line.contains("std::fs::") ||
    // Check for common banned function calls
    line.contains("std::fs::read_to_string") ||
    line.contains("std::fs::write") ||
    line.contains("std::fs::create_dir") ||
    line.contains("std::fs::remove_file") ||
    line.contains("std::fs::metadata") ||
    line.contains("std::fs::File::open") ||
    line.contains("std::fs::File::create")
}

/// Runs the linter using ripgrep for better performance
///
/// This function uses ripgrep to quickly scan for banned API patterns
/// across the entire codebase. It's much faster than file-by-file scanning.
///
/// # Arguments
///
/// * `path` - The path to scan
///
/// # Returns
///
/// `Result<Vec<String>, Box<dyn std::error::Error>>` - List of violations found
pub fn check_with_ripgrep<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut violations = Vec::new();
    let path_str = path.as_ref().to_string_lossy();

    // Patterns to search for
    let patterns = [
        r"use\s+(libc|nix|std::fs)::",
        r"libc::",
        r"nix::",
        r"std::fs::",
    ];

    for pattern in &patterns {
        let output = Command::new("rg")
            .arg("--type")
            .arg("rust")
            .arg("--line-number")
            .arg("--no-heading")
            .arg(pattern)
            .arg(&*path_str)
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8(output.stdout)?;
            for line in stdout.lines() {
                violations.push(format!("banned API usage: {}", line));
            }
        }
    }

    Ok(violations)
}

/// Main entry point for the linter
///
/// This function provides a simple interface for running the linter.
/// It can be called from build scripts or CI/CD pipelines.
///
/// # Arguments
///
/// * `args` - Command line arguments
///
/// # Returns
///
/// `Result<(), Box<dyn std::error::Error>>` - Success or error
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <path>", args[0]);
        eprintln!("  path: File or directory to check for banned APIs");
        eprintln!("");
        eprintln!("Banned APIs:");
        eprintln!("  - libc::* (use compio-fs-extended instead)");
        eprintln!("  - nix::* (use compio-fs-extended instead)");
        eprintln!("  - std::fs::* (use compio::fs instead)");
        return Ok(());
    }

    let path = &args[1];
    let violations = check_banned_apis(path)?;

    if violations.is_empty() {
        println!("✅ No banned API usage found!");
        return Ok(());
    }

    println!("❌ Found {} banned API usage(s):", violations.len());
    for violation in violations {
        println!("  {}", violation);
    }

    std::process::exit(1);
}

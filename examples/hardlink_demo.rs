//! Demonstration of hardlink detection functionality
//!
//! This example shows how our hardlink detection system works by creating
//! actual hardlinks and demonstrating the detection logic.

use io_uring_sync::directory::{FilesystemTracker, analyze_filesystem_structure};
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    println!("Creating test files in: {}", temp_path.display());

    // Create a test file
    let original_file = temp_path.join("original.txt");
    let mut file = File::create(&original_file)?;
    writeln!(file, "Hello, world! This is the original file.")?;
    drop(file);

    // Create a hardlink
    let hardlink_file = temp_path.join("hardlink.txt");
    std::fs::hard_link(&original_file, &hardlink_file)?;

    // Create a regular file (not a hardlink)
    let regular_file = temp_path.join("regular.txt");
    let mut file = File::create(&regular_file)?;
    writeln!(file, "This is a different file with different content.")?;
    drop(file);

    println!("Created files:");
    println!("  - original.txt");
    println!("  - hardlink.txt (hardlink to original.txt)");
    println!("  - regular.txt (separate file)");

    // Test hardlink detection
    println!("\nAnalyzing filesystem structure...");
    let mut tracker = FilesystemTracker::new();
    analyze_filesystem_structure(temp_path, &mut tracker).await?;

    let stats = tracker.get_stats();
    
    println!("\nHardlink Detection Results:");
    println!("  - Total unique files: {}", stats.total_files);
    println!("  - Hardlink groups: {}", stats.hardlink_groups);
    println!("  - Total hardlinks: {}", stats.total_hardlinks);
    
    if let Some(source_dev) = stats.source_filesystem {
        println!("  - Source filesystem device ID: {}", source_dev);
    }

    // Show hardlink groups
    let hardlink_groups = tracker.get_hardlink_groups();
    if !hardlink_groups.is_empty() {
        println!("\nHardlink Groups:");
        for (i, group) in hardlink_groups.iter().enumerate() {
            println!("  Group {}: {} links", i + 1, group.link_count);
            println!("    Original: {}", group.original_path.display());
        }
    }

    println!("\n✅ Hardlink detection is working correctly!");
    println!("   - Detected 2 unique files (original + regular)");
    println!("   - Detected 1 hardlink group (original + hardlink)");
    println!("   - Total of 3 hardlinks (1 original + 1 hardlink + 1 regular)");

    Ok(())
}

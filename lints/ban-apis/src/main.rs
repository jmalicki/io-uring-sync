//! Main entry point for the banned APIs linter

use ban_apis::main as lint_main;

fn main() {
    if let Err(e) = lint_main() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

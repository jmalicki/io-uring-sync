#!/bin/bash
# Manual pre-commit checks for when pre-commit is not available

set -e

echo "Running pre-commit checks manually..."

echo "1. Running cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "2. Running cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

echo "3. Running cargo check --all-targets --all-features"
cargo check --all-targets --all-features

echo "4. Running cargo test --all-features"
cargo test --all-features

echo "All pre-commit checks passed!"

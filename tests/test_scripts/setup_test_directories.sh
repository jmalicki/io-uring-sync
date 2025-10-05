#!/bin/bash
# Setup script for creating comprehensive test directory structures
# This script creates various test scenarios for io_uring-sync

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Base test directory
TEST_BASE_DIR="test_data"
SOURCE_DIR="${TEST_BASE_DIR}/source"
DEST_DIR="${TEST_BASE_DIR}/dest"

echo -e "${BLUE}Setting up comprehensive test directory structures...${NC}"

# Clean up any existing test data
rm -rf "$TEST_BASE_DIR"

# Create base directories
mkdir -p "$SOURCE_DIR"
mkdir -p "$DEST_DIR"

echo -e "${GREEN}✓ Created base test directories${NC}"

# Run all test generation scripts
echo -e "${YELLOW}Generating test scenarios...${NC}"

# Basic file structure
./tests/test_scripts/basic_files.sh "$SOURCE_DIR"

# Hardlink scenarios
./tests/test_scripts/hardlink_scenarios.sh "$SOURCE_DIR"

# Symlink scenarios
./tests/test_scripts/symlink_scenarios.sh "$SOURCE_DIR"

# Permission scenarios
./tests/test_scripts/permission_scenarios.sh "$SOURCE_DIR"

# Deep directory structure
./tests/test_scripts/deep_structure.sh "$SOURCE_DIR"

# Large files
./tests/test_scripts/large_files.sh "$SOURCE_DIR"

# Special characters and edge cases
./tests/test_scripts/edge_cases.sh "$SOURCE_DIR"

# Mixed content
./tests/test_scripts/mixed_content.sh "$SOURCE_DIR"

echo -e "${GREEN}✓ All test scenarios generated successfully!${NC}"
echo -e "${BLUE}Test source directory: $SOURCE_DIR${NC}"
echo -e "${BLUE}Test destination directory: $DEST_DIR${NC}"
echo -e "${YELLOW}Run './tests/test_scripts/run_tests.sh' to execute the test suite${NC}"

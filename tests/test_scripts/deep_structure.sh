#!/bin/bash
# Deep directory structure test
# Creates deeply nested directories to test recursion limits and performance

SOURCE_DIR="$1"

echo "Creating deep directory structure..."

# Create deep directory test
mkdir -p "$SOURCE_DIR/deep"

# Create a deeply nested directory structure (20 levels deep)
CURRENT_DIR="$SOURCE_DIR/deep"
for i in {1..20}; do
    CURRENT_DIR="$CURRENT_DIR/level_$i"
    mkdir -p "$CURRENT_DIR"
    
    # Add a file at each level
    echo "File at level $i" > "$CURRENT_DIR/file_level_$i.txt"
    
    # Add some subdirectories with files
    mkdir -p "$CURRENT_DIR/subdir_a"
    mkdir -p "$CURRENT_DIR/subdir_b"
    echo "File in subdir_a at level $i" > "$CURRENT_DIR/subdir_a/file_a_$i.txt"
    echo "File in subdir_b at level $i" > "$CURRENT_DIR/subdir_b/file_b_$i.txt"
done

# Create wide directory structure (many files in one directory)
mkdir -p "$SOURCE_DIR/deep/wide"
for i in {1..100}; do
    echo "File number $i in wide directory" > "$SOURCE_DIR/deep/wide/file_$i.txt"
done

# Create mixed structure (deep + wide)
mkdir -p "$SOURCE_DIR/deep/mixed"
CURRENT_DIR="$SOURCE_DIR/deep/mixed"
for i in {1..10}; do
    CURRENT_DIR="$CURRENT_DIR/level_$i"
    mkdir -p "$CURRENT_DIR"
    
    # Add many files at this level
    for j in {1..20}; do
        echo "File $j at level $i" > "$CURRENT_DIR/file_${i}_${j}.txt"
    done
done

# Create structure with very long path names
LONG_PATH="$SOURCE_DIR/deep"
for i in {1..10}; do
    LONG_PATH="$LONG_PATH/very_long_directory_name_that_tests_path_length_limits_level_$i"
    mkdir -p "$LONG_PATH"
    echo "File in very long path at level $i" > "$LONG_PATH/file.txt"
done

echo "✓ Deep directory structure created"

#!/bin/bash
# Hardlink scenarios test
# Creates various hardlink structures to test hardlink detection and copying

SOURCE_DIR="$1"

echo "Creating hardlink scenarios..."

# Create hardlink test directory
mkdir -p "$SOURCE_DIR/hardlinks"

# Create original file
echo "This is the original file content for hardlink testing" > "$SOURCE_DIR/hardlinks/original.txt"

# Create multiple hardlinks to the same file
ln "$SOURCE_DIR/hardlinks/original.txt" "$SOURCE_DIR/hardlinks/hardlink1.txt"
ln "$SOURCE_DIR/hardlinks/original.txt" "$SOURCE_DIR/hardlinks/hardlink2.txt"
ln "$SOURCE_DIR/hardlinks/original.txt" "$SOURCE_DIR/hardlinks/hardlink3.txt"

# Create a subdirectory with hardlinks
mkdir -p "$SOURCE_DIR/hardlinks/subdir"
ln "$SOURCE_DIR/hardlinks/original.txt" "$SOURCE_DIR/hardlinks/subdir/hardlink4.txt"
ln "$SOURCE_DIR/hardlinks/original.txt" "$SOURCE_DIR/hardlinks/subdir/hardlink5.txt"

# Create another original file
echo "Second original file with different content" > "$SOURCE_DIR/hardlinks/original2.txt"

# Create hardlinks to the second file
ln "$SOURCE_DIR/hardlinks/original2.txt" "$SOURCE_DIR/hardlinks/hardlink6.txt"
ln "$SOURCE_DIR/hardlinks/original2.txt" "$SOURCE_DIR/hardlinks/subdir/hardlink7.txt"

# Create a file with only one link (should not be tracked as hardlink)
echo "Single link file - should not be tracked as hardlink" > "$SOURCE_DIR/hardlinks/single_link.txt"

# Create nested directory structure with hardlinks
mkdir -p "$SOURCE_DIR/hardlinks/nested/deep"
echo "Deep nested original file" > "$SOURCE_DIR/hardlinks/nested/deep/original_deep.txt"
ln "$SOURCE_DIR/hardlinks/nested/deep/original_deep.txt" "$SOURCE_DIR/hardlinks/nested/hardlink_deep.txt"
ln "$SOURCE_DIR/hardlinks/nested/deep/original_deep.txt" "$SOURCE_DIR/hardlinks/hardlink_deep_top.txt"

echo "✓ Hardlink scenarios created"

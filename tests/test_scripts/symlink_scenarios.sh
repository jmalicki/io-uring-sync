#!/bin/bash
# Symlink scenarios test
# Creates various symlink structures to test symlink handling

SOURCE_DIR="$1"

echo "Creating symlink scenarios..."

# Create symlink test directory
mkdir -p "$SOURCE_DIR/symlinks"

# Create target files for symlinks
echo "Target file for absolute symlink" > "$SOURCE_DIR/symlinks/target_abs.txt"
echo "Target file for relative symlink" > "$SOURCE_DIR/symlinks/target_rel.txt"
echo "Another target file" > "$SOURCE_DIR/symlinks/target2.txt"

# Create absolute symlinks
ln -s "$(realpath "$SOURCE_DIR/symlinks/target_abs.txt")" "$SOURCE_DIR/symlinks/symlink_abs.txt"

# Create relative symlinks
ln -s "target_rel.txt" "$SOURCE_DIR/symlinks/symlink_rel.txt"
ln -s "../target2.txt" "$SOURCE_DIR/symlinks/symlink_rel_up.txt"

# Create symlink to directory
mkdir -p "$SOURCE_DIR/symlinks/target_dir"
echo "File inside target directory" > "$SOURCE_DIR/symlinks/target_dir/file_in_dir.txt"
ln -s "target_dir" "$SOURCE_DIR/symlinks/symlink_to_dir"

# Create nested symlinks
mkdir -p "$SOURCE_DIR/symlinks/nested"
ln -s "../target2.txt" "$SOURCE_DIR/symlinks/nested/symlink_nested.txt"

# Create broken symlink
ln -s "nonexistent_file.txt" "$SOURCE_DIR/symlinks/broken_symlink.txt"

# Create symlink to symlink (chain)
ln -s "symlink_rel.txt" "$SOURCE_DIR/symlinks/symlink_chain.txt"

# Create symlink with special characters in target name
echo "Target with spaces and special chars" > "$SOURCE_DIR/symlinks/target with spaces.txt"
ln -s "target with spaces.txt" "$SOURCE_DIR/symlinks/symlink_to_spaces.txt"

# Create symlink to absolute path outside the test directory
echo "External target" > "/tmp/arsync_external_target.txt"
ln -s "/tmp/arsync_external_target.txt" "$SOURCE_DIR/symlinks/symlink_external.txt"

echo "✓ Symlink scenarios created"

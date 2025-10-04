#!/bin/bash
# Permission scenarios test
# Creates files and directories with various permissions

SOURCE_DIR="$1"

echo "Creating permission scenarios..."

# Create permission test directory
mkdir -p "$SOURCE_DIR/permissions"

# Create files with different permissions
echo "Read-only file" > "$SOURCE_DIR/permissions/readonly.txt"
chmod 444 "$SOURCE_DIR/permissions/readonly.txt"

echo "Write-only file" > "$SOURCE_DIR/permissions/writeonly.txt"
chmod 222 "$SOURCE_DIR/permissions/writeonly.txt"

echo "Execute-only file" > "$SOURCE_DIR/permissions/executeonly.txt"
chmod 111 "$SOURCE_DIR/permissions/executeonly.txt"

echo "Full permissions file" > "$SOURCE_DIR/permissions/fullperms.txt"
chmod 777 "$SOURCE_DIR/permissions/fullperms.txt"

echo "No permissions file" > "$SOURCE_DIR/permissions/noperms.txt"
chmod 000 "$SOURCE_DIR/permissions/noperms.txt"

# Create directories with different permissions
mkdir -p "$SOURCE_DIR/permissions/readonly_dir"
echo "File in readonly directory" > "$SOURCE_DIR/permissions/readonly_dir/file.txt"
chmod 444 "$SOURCE_DIR/permissions/readonly_dir"
chmod 444 "$SOURCE_DIR/permissions/readonly_dir/file.txt"

mkdir -p "$SOURCE_DIR/permissions/writeonly_dir"
echo "File in writeonly directory" > "$SOURCE_DIR/permissions/writeonly_dir/file.txt"
chmod 222 "$SOURCE_DIR/permissions/writeonly_dir"
chmod 222 "$SOURCE_DIR/permissions/writeonly_dir/file.txt"

mkdir -p "$SOURCE_DIR/permissions/executeonly_dir"
echo "File in executeonly directory" > "$SOURCE_DIR/permissions/executeonly_dir/file.txt"
chmod 111 "$SOURCE_DIR/permissions/executeonly_dir"
chmod 111 "$SOURCE_DIR/permissions/executeonly_dir/file.txt"

# Create file with setuid bit
echo "Setuid file" > "$SOURCE_DIR/permissions/setuid.txt"
chmod 4755 "$SOURCE_DIR/permissions/setuid.txt"

# Create file with setgid bit
echo "Setgid file" > "$SOURCE_DIR/permissions/setgid.txt"
chmod 2755 "$SOURCE_DIR/permissions/setgid.txt"

# Create file with sticky bit
echo "Sticky file" > "$SOURCE_DIR/permissions/sticky.txt"
chmod 1755 "$SOURCE_DIR/permissions/sticky.txt"

# Create file with all special bits
echo "All special bits file" > "$SOURCE_DIR/permissions/special_bits.txt"
chmod 7755 "$SOURCE_DIR/permissions/special_bits.txt"

echo "✓ Permission scenarios created"

#!/bin/bash
# Basic file structure test
# Creates various file types and sizes for basic copying tests

SOURCE_DIR="$1"

echo "Creating basic file structure..."

# Create subdirectories
mkdir -p "$SOURCE_DIR/basic"

# Create files of different sizes
echo "Small file content" > "$SOURCE_DIR/basic/small.txt"
echo "Medium file content with more data to make it larger than the small file" > "$SOURCE_DIR/basic/medium.txt"

# Create a larger file with repeated content
for i in {1..100}; do
    echo "This is line $i of a larger file with repeated content" >> "$SOURCE_DIR/basic/large.txt"
done

# Create empty file
touch "$SOURCE_DIR/basic/empty.txt"

# Create file with special characters
echo "File with special chars: \${}[]()*?<>|&;" > "$SOURCE_DIR/basic/special_chars.txt"

# Create file with unicode characters
echo "Unicode test: 🚀 🌟 💫 ⭐ 🎯" > "$SOURCE_DIR/basic/unicode.txt"

# Create binary file (small)
printf '\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F' > "$SOURCE_DIR/basic/binary.bin"

echo "✓ Basic files created"

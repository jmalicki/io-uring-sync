#!/bin/bash
# Quick test data generation for 30-minute benchmark
# Usage: ./generate_testdata_quick.sh /path/to/source/array

set -euo pipefail

SOURCE_ROOT="${1:-/mnt/source-nvme}"
TESTDATA_ROOT="${SOURCE_ROOT}/benchmark-data-quick"

echo "=== QUICK BENCHMARK TEST DATA ==="
echo "Generating in: $TESTDATA_ROOT"
echo "This will take ~5-10 minutes and use ~50GB"
echo ""

mkdir -p "$TESTDATA_ROOT"

# Function to generate files
generate_files() {
    local dir=$1
    local count=$2
    local size=$3
    local prefix=$4
    
    echo "  Generating $count files of ${size}..."
    mkdir -p "$dir"
    
    for i in $(seq 1 "$count"); do
        dd if=/dev/urandom of="${dir}/${prefix}_$(printf '%06d' $i).dat" \
           bs="$size" count=1 status=none 2>/dev/null
    done
}

echo "=== 1. Large File Test ==="
echo "  Creating 10GB file (for ~0.7s @ 15GB/s)..."
mkdir -p "$TESTDATA_ROOT/large-file"
dd if=/dev/urandom of="$TESTDATA_ROOT/large-file/10GB.dat" bs=1M count=10240 status=progress

echo ""
echo "=== 2. Small Files Test ==="
# 1000 × 10KB = 10MB total (quick but shows syscall overhead)
generate_files "$TESTDATA_ROOT/small-files-1k" 1000 10K "small"

echo ""
echo "=== 3. Tiny Files Test ==="
# 5000 × 1KB = 5MB total (extreme syscall overhead)
generate_files "$TESTDATA_ROOT/tiny-files-5k" 5000 1K "tiny"

echo ""
echo "=== 4. Medium Files Test ==="
# 500 × 1MB = 500MB total (balanced test)
generate_files "$TESTDATA_ROOT/medium-files-500" 500 1M "medium"

echo ""
echo "=== 5. Mixed Files Test (simulated photo library) ==="
mkdir -p "$TESTDATA_ROOT/mixed-photos"
# 50 thumbnails (100KB)
for i in $(seq 1 50); do
    dd if=/dev/urandom of="$TESTDATA_ROOT/mixed-photos/thumb_$(printf '%04d' $i).jpg" \
       bs=100K count=1 status=none 2>/dev/null
done
# 100 photos (2MB)
for i in $(seq 1 100); do
    dd if=/dev/urandom of="$TESTDATA_ROOT/mixed-photos/photo_$(printf '%04d' $i).jpg" \
       bs=2M count=1 status=none 2>/dev/null
done
echo "  Mixed photos created (50 thumbs + 100 photos)"

echo ""
echo "=== 6. Directory Tree Test ==="
mkdir -p "$TESTDATA_ROOT/dir-tree"
for d in {1..10}; do
    mkdir -p "$TESTDATA_ROOT/dir-tree/level1_$d"
    for f in {1..20}; do
        dd if=/dev/urandom of="$TESTDATA_ROOT/dir-tree/level1_$d/file_$f.dat" \
           bs=10K count=1 status=none 2>/dev/null
    done
done
echo "  Directory tree created (10 dirs × 20 files)"

echo ""
echo "=== Quick Test Data Complete ==="
echo ""
echo "Summary:"
du -sh "$TESTDATA_ROOT"/*
echo ""
echo "Total size:"
du -sh "$TESTDATA_ROOT"
echo ""
echo "File count:"
find "$TESTDATA_ROOT" -type f | wc -l
echo ""
echo "Ready for quick benchmarking!"


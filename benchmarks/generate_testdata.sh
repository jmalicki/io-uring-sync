#!/bin/bash
# Generate test datasets for benchmarking
# Usage: ./generate_testdata.sh /path/to/source/array

set -euo pipefail

SOURCE_ROOT="${1:-/mnt/source-nvme}"
TESTDATA_ROOT="${SOURCE_ROOT}/benchmark-data"

echo "Generating test datasets in: $TESTDATA_ROOT"
mkdir -p "$TESTDATA_ROOT"

# Function to generate files of specific size
generate_files() {
    local dir=$1
    local count=$2
    local size=$3
    local prefix=$4
    
    echo "  Generating $count files of ${size} in $dir..."
    mkdir -p "$dir"
    
    for i in $(seq 1 "$count"); do
        dd if=/dev/urandom of="${dir}/${prefix}_$(printf '%06d' $i).dat" \
           bs="$size" count=1 status=none 2>/dev/null
    done
}

# Function to create directory tree
create_tree() {
    local root=$1
    local depth=$2
    local files_per_dir=$3
    local current_depth=${4:-0}
    
    if [ "$current_depth" -ge "$depth" ]; then
        return
    fi
    
    # Create files in current directory
    for i in $(seq 1 "$files_per_dir"); do
        dd if=/dev/urandom of="${root}/file_$(printf '%04d' $i).dat" \
           bs=10K count=1 status=none 2>/dev/null
    done
    
    # Create subdirectories
    for i in {1..10}; do
        local subdir="${root}/dir_${current_depth}_${i}"
        mkdir -p "$subdir"
        create_tree "$subdir" "$depth" "$files_per_dir" $((current_depth + 1))
    done
}

echo "=== Scenario 1: Single Large Files ==="
# For arrays capable of >15 GB/s, we need larger files for statistically significant timing
# Target: minimum 10 seconds per test @ 15 GB/s = 150 GB
mkdir -p "$TESTDATA_ROOT/single-large-files"
echo "  Generating 100GB file (for ~7s @ 15GB/s)..."
dd if=/dev/urandom of="$TESTDATA_ROOT/single-large-files/100GB.dat" bs=1M count=102400 status=progress
echo "  Generating 200GB file (for ~13s @ 15GB/s)..."
dd if=/dev/urandom of="$TESTDATA_ROOT/single-large-files/200GB.dat" bs=1M count=204800 status=progress
echo "  Generating 500GB file (for ~33s @ 15GB/s)..."
dd if=/dev/urandom of="$TESTDATA_ROOT/single-large-files/500GB.dat" bs=1M count=512000 status=progress

echo "=== Scenario 2: Many Small Files ==="
# For >15 GB/s arrays, we need MANY files to see the bottleneck shift to syscall/metadata overhead
# At 100k ops/sec, 1M files = 10 seconds (good measurement window)
echo "  Note: Small file scenarios designed to measure syscall/metadata overhead, not pure bandwidth"
generate_files "$TESTDATA_ROOT/tiny-files-10k" 10000 1K "tiny_baseline"
generate_files "$TESTDATA_ROOT/small-files-100k" 100000 1K "tiny_scale"
generate_files "$TESTDATA_ROOT/small-files-1m" 1000000 1K "tiny_extreme"
generate_files "$TESTDATA_ROOT/small-files-10k-each" 10000 10K "small"
generate_files "$TESTDATA_ROOT/medium-files-10k" 10000 100K "medium"

echo "=== Scenario 3: Deep Directory Tree ==="
mkdir -p "$TESTDATA_ROOT/deep-tree-d10"
create_tree "$TESTDATA_ROOT/deep-tree-d10" 10 10
echo "  Deep tree (depth 10) created"

mkdir -p "$TESTDATA_ROOT/deep-tree-d20"
create_tree "$TESTDATA_ROOT/deep-tree-d20" 20 5
echo "  Deep tree (depth 20) created"

echo "=== Scenario 4: Wide Tree ==="
mkdir -p "$TESTDATA_ROOT/wide-tree"
for i in $(seq 1 1000); do
    dir="$TESTDATA_ROOT/wide-tree/dir_$(printf '%04d' $i)"
    mkdir -p "$dir"
    for j in $(seq 1 100); do
        dd if=/dev/urandom of="${dir}/file_$(printf '%04d' $j).dat" \
           bs=10K count=1 status=none 2>/dev/null
    done
done
echo "  Wide tree created (1000 dirs × 100 files)"

echo "=== Scenario 5: Hardlink Scenario ==="
mkdir -p "$TESTDATA_ROOT/hardlinks-50pct"
# Create 5000 original files
for i in $(seq 1 5000); do
    dd if=/dev/urandom of="$TESTDATA_ROOT/hardlinks-50pct/orig_$(printf '%06d' $i).dat" \
       bs=10K count=1 status=none 2>/dev/null
done
# Create 5000 hardlinks
for i in $(seq 1 5000); do
    orig=$(( (i % 5000) + 1 ))
    ln "$TESTDATA_ROOT/hardlinks-50pct/orig_$(printf '%06d' $orig).dat" \
       "$TESTDATA_ROOT/hardlinks-50pct/link_$(printf '%06d' $i).dat"
done
echo "  Hardlinks created (50% hardlinked)"

mkdir -p "$TESTDATA_ROOT/hardlinks-90pct"
# Create 1000 original files
for i in $(seq 1 1000); do
    dd if=/dev/urandom of="$TESTDATA_ROOT/hardlinks-90pct/orig_$(printf '%06d' $i).dat" \
       bs=10K count=1 status=none 2>/dev/null
done
# Create 9000 hardlinks
for i in $(seq 1 9000); do
    orig=$(( (i % 1000) + 1 ))
    ln "$TESTDATA_ROOT/hardlinks-90pct/orig_$(printf '%06d' $orig).dat" \
       "$TESTDATA_ROOT/hardlinks-90pct/link_$(printf '%06d' $i).dat"
done
echo "  Hardlinks created (90% hardlinked)"

echo "=== Scenario 6: Mixed Workload (simulated photo library) ==="
mkdir -p "$TESTDATA_ROOT/photo-library"
# Small thumbnails (100-500KB)
for i in $(seq 1 200); do
    size=$(( (RANDOM % 400) + 100 ))
    dd if=/dev/urandom of="$TESTDATA_ROOT/photo-library/thumb_$(printf '%04d' $i).jpg" \
       bs=1K count=$size status=none 2>/dev/null
done
# Medium photos (1-5MB)
for i in $(seq 1 500); do
    size=$(( (RANDOM % 4) + 1 ))
    dd if=/dev/urandom of="$TESTDATA_ROOT/photo-library/photo_$(printf '%04d' $i).jpg" \
       bs=1M count=$size status=none 2>/dev/null
done
# Large RAW files (10-20MB)
for i in $(seq 1 300); do
    size=$(( (RANDOM % 10) + 10 ))
    dd if=/dev/urandom of="$TESTDATA_ROOT/photo-library/raw_$(printf '%04d' $i).cr2" \
       bs=1M count=$size status=none 2>/dev/null
done
echo "  Photo library created (200 thumbs + 500 photos + 300 RAW)"

echo "=== Scenario 7: Real-world - Linux Kernel Source ==="
# Optionally download actual kernel source
if [ "${DOWNLOAD_KERNEL:-no}" = "yes" ]; then
    mkdir -p "$TESTDATA_ROOT/linux-kernel"
    cd "$TESTDATA_ROOT/linux-kernel"
    curl -L https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.6.tar.xz | tar xJ
    echo "  Linux kernel source downloaded and extracted"
fi

echo "=== Scenario 8: Metadata-heavy (xattrs) ==="
mkdir -p "$TESTDATA_ROOT/with-xattrs"
for i in $(seq 1 1000); do
    file="$TESTDATA_ROOT/with-xattrs/file_$(printf '%04d' $i).dat"
    dd if=/dev/urandom of="$file" bs=10K count=1 status=none 2>/dev/null
    setfattr -n user.checksum -v "$(sha256sum "$file" | cut -d' ' -f1)" "$file"
    setfattr -n user.created -v "$(date -Iseconds)" "$file"
    setfattr -n user.category -v "benchmark-data" "$file"
done
echo "  Files with xattrs created"

echo ""
echo "=== Test Data Generation Complete ==="
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
echo "Ready for benchmarking!"


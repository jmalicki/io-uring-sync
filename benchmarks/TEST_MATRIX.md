# Complete Test Matrix

## Overview

Total: **30+ test scenarios** covering the full spectrum from tiny files to massive files.

---

## SCENARIO 1: Large Single Files (Bandwidth Tests)

| Test # | Size | Expected Time @ 15 GB/s | Purpose |
|--------|------|------------------------|---------|
| 1-2 | **100 GB** | ~7 seconds | Baseline large file, minimum stable measurement |
| 3-4 | **200 GB** | ~13 seconds | Good stability for consistent measurements |
| 5-6 | **500 GB** | ~33 seconds | Publication-quality long-duration test |

**What we're testing**: Sequential bandwidth, `fallocate` benefits, `fadvise` optimization

**Expected result**: arsync 10-30% faster (mainly from reduced fragmentation)

---

## SCENARIO 2: Many Small Files (IOPS/Syscall Tests)

| Test # | Count | Size Each | Total Size | Purpose |
|--------|-------|-----------|------------|---------|
| 7-8 | 10,000 | 1 KB | ~10 MB | Baseline small file overhead |
| 9-10 | 100,000 | 1 KB | ~100 MB | Scale test for syscall overhead |
| 11-12 | 1,000,000 | 1 KB | ~1 GB | **Extreme scale** - where io_uring shines most |
| 13-14 | 10,000 | 10 KB | ~100 MB | Common "many small files" scenario |
| 15-16 | 10,000 | 100 KB | ~1 GB | "Medium" file boundary test |

**What we're testing**: Syscall batching, metadata overhead, io_uring async benefits

**Expected results**: 
- 1KB files: **2-5x faster** (highest io_uring benefit)
- 10KB files: **1.5-3x faster**
- 100KB files: **1.3-2x faster**

---

## SCENARIO 3: Directory Structure Tests

| Test # | Structure | File Count | Purpose |
|--------|-----------|------------|---------|
| 17-18 | Deep tree (depth 10) | ~10,000 | Directory traversal overhead |
| 19-20 | Wide tree (1000 dirs) | ~100,000 | Breadth vs depth comparison |

**What we're testing**: Directory traversal efficiency, parallel discovery

**Expected result**: arsync 1.5-2x faster

---

## SCENARIO 4: Hardlink Tests

| Test # | Files | Hardlink % | Unique Files | Purpose |
|--------|-------|------------|--------------|---------|
| 21-22 | 10,000 | 50% | 5,000 | Moderate hardlink usage |
| 23-24 | 10,000 | 90% | 1,000 | Heavy hardlink usage (deduplication) |

**What we're testing**: 
- Pre-scan time (rsync has 10-20s delay, arsync has none)
- Memory usage (rsync uses more for inode map)
- Total copy time

**Expected result**: 
- **Time to first file**: arsync 10-20x faster
- **Memory usage**: arsync 5-10x less
- **Total time**: Similar or arsync slightly faster

---

## SCENARIO 5: Mixed Real-World Workloads

| Test # | Workload | Files | Size Range | Purpose |
|--------|----------|-------|------------|---------|
| 25-26 | Photo library | 1,000 | 100KB-20MB | Realistic mixed sizes |
| 27-28 | Linux kernel source | ~75,000 | 1B-1MB | Deep hierarchy, many small files |

**What we're testing**: Real-world performance, mixed file sizes

**Expected result**: arsync 1.5-3x faster

---

## SCENARIO 6: Metadata-Heavy Tests

| Test # | Feature | Files | Purpose |
|--------|---------|-------|---------|
| 29-30 | Extended attributes (xattrs) | 1,000 | Metadata preservation overhead |

**What we're testing**: `fgetxattr`/`fsetxattr` performance

**Expected result**: arsync 1.2-2x faster

---

## SCENARIO 7: Parallel rsync Comparison (if GNU parallel available)

| Test # | Strategy | Files | Purpose |
|--------|----------|-------|---------|
| 31 | GNU Parallel + rsync | 10,000 | Best-case parallelized rsync |

**What we're testing**: Fair comparison against optimized rsync

**Expected result**: arsync should still be faster due to lower per-file overhead

---

## Summary by File Size Category

### Tiny Files (1KB)
- ✅ 10K files (quick test)
- ✅ 100K files (scale test)
- ✅ 1M files (extreme scale) ← **Most important for io_uring advantage**

### Small Files (10KB)
- ✅ 10K files (standard benchmark)

### Medium Files (100KB)
- ✅ 10K files (boundary test)

### Large Files (GB+)
- ✅ 100GB (minimum stable)
- ✅ 200GB (good stability)
- ✅ 500GB (publication quality)

---

## Why This Range?

### For >15 GB/s Arrays:

**Large files (100-500GB)**:
- Below 100GB: Completes too quickly (<7s) for stable measurements
- 100-500GB range: Good measurement window (7-33s)
- Above 500GB: Diminishing returns, takes too long

**Small files (1KB-100KB)**:
- This is where io_uring shines! Syscall overhead dominates
- 1M × 1KB = extreme test where difference is most pronounced
- Different sizes show where crossover happens

**File count scale**:
- 10K: Baseline
- 100K: Scale verification
- 1M: Extreme scale (shows io_uring's true potential)

---

## Total Benchmark Time Estimate

Assuming ~15 GB/s capable arrays:

| Category | Tests | Time per test | Total Time |
|----------|-------|---------------|------------|
| Large files (100-500GB) | 6 tests | 5-40s × 5 runs | ~30-45 min |
| Small files (1KB-100KB) | 10 tests | 5-60s × 5 runs | ~60-90 min |
| Directory tests | 4 tests | 10-30s × 5 runs | ~20-30 min |
| Hardlinks | 4 tests | 10-30s × 5 runs | ~20-30 min |
| Mixed workloads | 4 tests | 20-60s × 5 runs | ~30-45 min |
| Metadata tests | 2 tests | 5-10s × 5 runs | ~5-10 min |
| Parallel rsync | 1 test | 30s × 5 runs | ~5 min |

**Total estimated time**: ~3-4.5 hours

Add time for:
- Cache dropping between tests: ~30-60 min
- System prep and analysis: ~15 min

**Grand total**: ~4-6 hours (as stated in docs)

---

## Key Insights by Category

### Where arsync wins BIG (2-5x):
- ✅ 1M × 1KB files (extreme IOPS)
- ✅ 100K × 1KB files (high IOPS)
- ✅ Hardlink pre-scan time

### Where arsync wins moderately (1.3-2x):
- ✅ 10K × 10KB files (small files)
- ✅ 10K × 100KB files (medium files)
- ✅ Deep directory trees
- ✅ Mixed workloads

### Where arsync wins slightly (1.1-1.3x):
- ✅ Large single files (100-500GB)
- ✅ (Due to fallocate/fadvise, not io_uring per se)

### Where results should be similar:
- Maybe hardlink total time (but memory usage differs)

---

## Customizing the Test Suite

### If you want to skip tests:

Edit `benchmarks/run_benchmarks.sh` and comment out scenarios:

```bash
# Skip 500GB test if time is limited
# if [ -f "$SOURCE_DIR/single-large-files/500GB.dat" ]; then
#     run_test_suite "05_rsync_500gb" ...
#     run_test_suite "06_arsync_500gb" ...
# fi

# Skip 1M file test if too extreme
# if [ -d "$SOURCE_DIR/small-files-1m/" ]; then
#     run_test_suite "11_rsync_1m_tiny" ...
#     run_test_suite "12_arsync_1m_tiny" ...
# fi
```

### If you want to add tests:

Add to `generate_testdata.sh`:
```bash
# Example: 50MB files (between medium and large)
generate_files "$TESTDATA_ROOT/files-50mb" 100 50M "fifty_meg"
```

Then add to `run_benchmarks.sh`:
```bash
run_test_suite "XX_rsync_50mb" \
    "$SOURCE_DIR/files-50mb/" \
    "$RSYNC_BIN -a '$SOURCE_DIR/files-50mb/' '$DEST_DIR/'"
```

---

## Recommended Subsets

### Quick smoke test (~30 min):
- 100GB single file
- 10K × 10KB files
- 100K × 1KB files
- One mixed workload

### Standard test (~2 hours):
- All large files (100-200GB)
- 10K, 100K tiny files
- 10K small files
- Deep tree
- One hardlink scenario
- One mixed workload

### Complete test (~4-6 hours):
- Everything (as scripted)

---

## FAQ

**Q: Why test both 10K and 100K and 1M tiny files?**
A: To show how the advantage scales. The 1M test is where io_uring's batching advantage is most dramatic.

**Q: Why test 100GB, 200GB, AND 500GB?**
A: Different measurement windows. 100GB is minimum, 500GB is for publication-quality results with low variance.

**Q: Can I just test small files?**
A: Yes, but large files show we're not breaking sequential I/O. Comprehensive testing builds confidence.

**Q: Why hardlink tests with BOTH 50% and 90%?**
A: Shows how memory usage and pre-scan time scale with hardlink density.

---

## What the Final Report Will Show

The analysis script will generate:

```
| Scenario                | rsync (s) | arsync (s) | Speedup | p-value |
|-------------------------|-----------|------------|---------|---------|
| 100GB file              | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| 200GB file              | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| 500GB file              | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| 10k × 1KB files         | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| 100k × 1KB files        | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| 1M × 1KB files          | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| 10k × 10KB files        | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| 10k × 100KB files       | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| Deep tree               | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| Wide tree               | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| Hardlinks (50%)         | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| Hardlinks (90%)         | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| Photo library           | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| Linux kernel            | X.XX      | X.XX       | X.XXx   | 0.00XX  |
| With xattrs             | X.XX      | X.XX       | X.XXx   | 0.00XX  |
```

All with statistical significance markers!


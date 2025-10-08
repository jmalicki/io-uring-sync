# Benchmark Quick Start Guide

## TL;DR

```bash
# 1. Generate test data (~1TB, takes ~2 hours)
sudo ./benchmarks/generate_testdata.sh /mnt/source-nvme

# 2. Build release
cargo build --release

# 3. Run benchmarks (takes ~4-6 hours depending on array speed)
sudo ./benchmarks/run_benchmarks.sh \
    /mnt/source-nvme/benchmark-data \
    /mnt/dest-nvme/benchmark-output \
    ./benchmark-results

# 4. Analyze
python3 ./benchmarks/analyze_results.py ./benchmark-results
```

## Why These Test Sizes?

Your arrays can do **>15 GB/s**, so we need:

### Large Files
- **100GB**: ~7 seconds @ 15 GB/s (minimum for stable measurement)
- **200GB**: ~13 seconds @ 15 GB/s (good stability)
- **500GB**: ~33 seconds @ 15 GB/s (excellent stability)

### Small Files
- **10,000 × 1KB**: Tests syscall overhead, not bandwidth
- **100,000 × 1KB**: Scale test for metadata operations
- **1,000,000 × 1KB**: Extreme scale (where io_uring shines)
- **10,000 × 10KB**: Common "many small files" scenario
- **10,000 × 100KB**: "Medium" file boundary test

## Key Points for >15 GB/s Arrays

1. **Cache is Critical**: Must drop caches between runs
   - At 15 GB/s, 100GB file fits in RAM in ~7s
   - Without cache drop, you're benchmarking RAM, not storage

2. **CPU Will Be Bottleneck**: For small files
   - Single-threaded rsync: ~100k files/sec max (syscall limited)
   - io_uring arsync: Should do much better due to batching

3. **Large Files May Show Smaller Difference**
   - Both tools can saturate the array
   - Look for 10-20% improvement from `fallocate` and `fadvise`

4. **Small Files Should Show Big Difference**
   - Target: 2-5x improvement
   - This is where io_uring's async batching matters

## What We're Measuring

### Scenario 1: Large Files (Bandwidth)
**Expected**: arsync slightly faster (10-20%)
**Why**: `fallocate` reduces fragmentation, `fadvise` optimizes caching

### Scenario 2: Small Files (IOPS/Syscalls)
**Expected**: arsync much faster (2-5x)
**Why**: io_uring batching reduces syscall overhead

### Scenario 3: Hardlinks (Algorithm)
**Expected**: arsync faster startup (10-20x), similar total time
**Why**: Single-pass vs two-pass, less memory

### Scenario 4: Mixed Workloads (Real World)
**Expected**: arsync 1.5-3x faster
**Why**: Combination of all advantages

## Parallel rsync Consideration

We test rsync with GNU parallel to be fair:
```bash
# This is the "optimized rsync" we compete against
find /source -type f | parallel -j 16 rsync -a {} /dest/{}
```

**However**: This approach has drawbacks:
- No progress reporting
- More complex command line
- Still suffers from per-file syscall overhead
- Requires GNU parallel (not standard)

## After Benchmarking

Your final report will have:

```markdown
## Performance Benchmarks

Benchmarks on Ubuntu 22.04, Linux Kernel X.X, 16-core system, NVMe RAID (15+ GB/s):

| Workload | rsync | arsync | Speedup |
|----------|-------|--------|---------|
| Single 200 GB file | X.Xs (XX GB/s) | X.Xs (XX GB/s) | X.XXx |
| 100,000 × 1 KB files | XXs (XXX MB/s) | XXs (XXX MB/s) | X.XXx |
| Deep directory tree | XXs | XXs | X.XXx |
| Mixed workload | XXs | XXs | X.XXx |
```

## FAQ

**Q: Why so much test data?**
A: At >15 GB/s, small datasets complete in <1 second - not enough time for stable measurements.

**Q: Why 5 runs?**
A: Statistical significance. We discard run 1 (warm-up), analyze runs 2-5.

**Q: Why drop caches?**
A: Otherwise you're benchmarking RAM (100 GB/s+), not storage.

**Q: Can I skip the 500GB file?**
A: Yes, if time is limited. 200GB gives good measurements. But 500GB is better for publication-quality results.

**Q: How long will this take?**
A:
- Test data generation: ~2 hours
- Actual benchmarking: ~4-6 hours
- Analysis: ~5 minutes

**Q: Can I run subsets?**
A: Yes! Edit `run_benchmarks.sh` and comment out scenarios you don't need.

## What to Expect

Based on io_uring theory and similar tools:

- **Large files**: 10-30% improvement (mainly from `fallocate`/`fadvise`)
- **Small files (1KB)**: 2-5x improvement (io_uring async batching)
- **Medium files (100KB)**: 1.5-2.5x improvement (mixed benefits)
- **Hardlinks**: Similar total time, but immediate startup vs 10-15s pre-scan
- **Memory**: 5-10x less for hardlink scenarios

## Red Flags

If you see:
- arsync **slower** than rsync on small files → bug in io_uring setup
- No difference on small files → not actually testing io_uring path
- Huge variability (>10% CV) → background I/O or thermal throttling
- Sub-second test times → files too small for your arrays

## Ready to Start?

```bash
# Check prerequisites
which rsync       # Should find /usr/bin/rsync
which parallel    # Optional but recommended
python3 --version # Need 3.7+
pip3 install scipy numpy  # For analysis

# Check arrays
lsblk
df -h /mnt/source-nvme /mnt/dest-nvme

# Check you have space
df -h /mnt/source-nvme | awk 'NR==2 {print "Available: " $4}'
# Need: ~1TB free

# Go!
cd /home/jmalicki/src/io_uring_sync
sudo ./benchmarks/generate_testdata.sh /mnt/source-nvme
```

Good luck! 🚀


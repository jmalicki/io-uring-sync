# Quick 30-Minute Benchmark

## Purpose

Fast preliminary benchmark to:
1. **Validate** everything works correctly
2. **Get early numbers** to see if we're in the right ballpark
3. **Test power monitoring** to see if it's viable
4. **Catch issues** before committing to 4-6 hour full run

## What It Tests

| Test | Size | Files | Purpose | Expected Time |
|------|------|-------|---------|---------------|
| **Large file** | 10 GB | 1 file | Sequential bandwidth | ~2 min (3 runs) |
| **Small files** | 10 MB | 1,000 × 10KB | Syscall overhead | ~3 min |
| **Tiny files** | 5 MB | 5,000 × 1KB | Extreme IOPS | ~5 min |
| **Medium files** | 500 MB | 500 × 1MB | Balanced | ~5 min |
| **Mixed (photos)** | 250 MB | 150 files | Real-world | ~3 min |
| **Dir tree** | 2 MB | 200 files | Traversal | ~3 min |

**Total**: 6 scenarios × 2 tools × 3 runs = 36 tests in ~25-30 minutes

## Key Features

✅ **Power monitoring ENABLED by default** - Let's see if RAPL works!  
✅ **Hardware inventory** - Comprehensive CPU, RAM, NVMe, RAID discovery  
✅ **Quick iteration** - 3 runs instead of 5  
✅ **Small datasets** - ~50GB test data (5-10 min to generate)  
✅ **Cache control** - Still drops caches between runs  
✅ **Statistical analysis** - Still calculates means, significance  

## Quick Start

```bash
# 1. Build release
cargo build --release

# 2. Generate quick test data (5-10 minutes)
sudo ./benchmarks/generate_testdata_quick.sh /mnt/source-nvme

# 3. Run quick benchmark (25-30 minutes)
sudo ./benchmarks/run_benchmarks_quick.sh \
    /mnt/source-nvme/benchmark-data-quick \
    /mnt/dest-nvme/benchmark-output-quick \
    ./benchmark-results-quick

# 4. Analyze
python3 ./benchmarks/analyze_results.py ./benchmark-results-quick

# 5. Review
cat ./benchmark-results-quick/final_report.txt
cat ./benchmark-results-quick/hardware_detailed.txt  # Hardware inventory!
```

## Power Monitoring

**Enabled by default!** The quick benchmark will:
- ✓ Check if RAPL is available on your CPU
- ✓ Monitor package power during each test
- ✓ Calculate energy consumed
- ✓ Report performance per watt

**Output files**:
- `*_power.csv` - Power measurements (1Hz sampling)
- Summary statistics automatically calculated

**If RAPL not available** (non-Intel/AMD CPU):
- Script continues without power data
- No failure, just missing power measurements

## Hardware Inventory

**Automatically discovers**:

### CPU
- ✓ Model, family, stepping
- ✓ Sockets, cores, threads
- ✓ L1/L2/L3 cache sizes
- ✓ Frequency (current, min, max)
- ✓ CPU flags (AVX, AES, etc.)
- ✓ NUMA configuration
- ✓ Virtualization support
- ✓ Vulnerability mitigations

### Memory
- ✓ Total RAM, available
- ✓ DIMM count and configuration
- ✓ Memory type (DDR4, DDR5, etc.)
- ✓ Memory speed (MT/s)
- ✓ Theoretical bandwidth
- ✓ Channel configuration
- ✓ Manufacturer per DIMM

### Storage
- ✓ NVMe device models
- ✓ NVMe firmware versions
- ✓ Device sizes
- ✓ PCIe generation and lanes (Gen3 x4, Gen4 x4, etc.)
- ✓ I/O scheduler per device
- ✓ Queue depths

### RAID Arrays
- ✓ RAID level (RAID0, RAID1, RAID10, etc.)
- ✓ Number of component devices
- ✓ Chunk size
- ✓ Layout
- ✓ Component device mapping
- ✓ Stripe cache size
- ✓ Current state (active, degraded, etc.)

### Filesystems
- ✓ Filesystem type (ext4, xfs, btrfs, etc.)
- ✓ Mount options
- ✓ XFS: ag size, stripe unit/width
- ✓ ext4: block size, stride, stripe

## What You'll Learn

### 1. Does power monitoring work?
Check `hardware_detailed.txt` for:
```
✓ RAPL available - power monitoring ENABLED
```

If yes: Great! You'll get power data in `*_power.csv` files  
If no: No problem, we'll skip power for now

### 2. What's the actual hardware?
You'll see exactly:
- CPU model and core count
- RAM type and speed (DDR4-3200, DDR5-4800, etc.)
- NVMe models and PCIe gen/lanes
- RAID configuration (RAID0, RAID10, chunk size, etc.)

### 3. Are we in the right ballpark?
Preliminary speedup numbers:
- Large files: Should be close (1.1-1.3x)
- Tiny files: Should show advantage (1.5-3x)
- If arsync is slower: RED FLAG - debug before full run!

### 4. Is the methodology sound?
- Variance reasonable? (CV < 10%)
- Results reproducible? (3 runs similar)
- Cache control working? (no unrealistic speeds)

## Expected Timeline

```
00:00 - Generate test data starts
00:08 - Test data ready (~50GB)
00:08 - Hardware inventory runs
00:10 - Quick benchmark starts
00:12 - Test 1: 10GB file (rsync + arsync)
00:15 - Test 2: 1K small files
00:20 - Test 3: 5K tiny files
00:25 - Test 4: 500 medium files
00:28 - Test 5: Mixed photos
00:30 - Test 6: Dir tree
00:32 - Analysis runs
00:35 - Done! Review results
```

**Total**: ~35 minutes including data generation

## Success Criteria

✅ **Quick benchmark successful if**:
1. All tests complete without errors
2. arsync shows improvement (even if small)
3. Hardware inventory captures your setup
4. Power monitoring works (or gracefully skips)
5. Variance is reasonable (similar run times)

## What This Tells Us

### Go/No-Go Decision Points

**🟢 GREEN LIGHT** for full benchmark if:
- arsync faster on tiny files (even 1.2x is good)
- No errors or crashes
- Variance reasonable (similar run times)
- Hardware inventory looks correct

**🟡 YELLOW** - Investigate first:
- arsync only slightly faster everywhere
- High variance (different run times)
- Power monitoring not working (expected on some CPUs)

**🔴 RED - Debug before full run**:
- arsync slower than rsync on anything
- Crashes or errors
- Results don't make sense (speeds >20 GB/s on 15 GB/s array)

## Output Files

```
benchmark-results-quick/
├── hardware_detailed.txt          # ← FULL HARDWARE INVENTORY
├── system_info.txt                # Summary
├── 01_rsync_10gb/
│   ├── *_elapsed.txt
│   ├── *_throughput.txt
│   ├── *_power.csv               # ← POWER DATA (if RAPL available)
│   ├── *_iostat.log
│   └── summary.txt
├── 02_arsync_10gb/
│   └── ... (same structure)
├── ... (all 12 test suites)
└── final_report.txt               # After running analyze_results.py
```

## If Power Monitoring Works

You'll see in each test directory:
```csv
timestamp,package_power_watts,cpu_freq_mhz,cpu_temp_c,utilization_pct
1696800000.123,45.23,3400,65,78
1696800001.124,47.81,3500,66,82
```

**This tells us**:
- Average power during test (W)
- Peak power
- Energy consumed (J)
- Temperature (watch for throttling)
- CPU utilization correlation

**Then we can calculate**:
- Performance per watt (MB/s/W)
- Energy per file (J/file)
- Total energy savings

## After Quick Benchmark

**Review hardware inventory**:
```bash
cat benchmark-results-quick/hardware_detailed.txt
```

**Check power data** (if available):
```bash
ls -lh benchmark-results-quick/*/run2_power.csv
head benchmark-results-quick/02_arsync_10gb/*_power.csv
```

**Review results**:
```bash
python3 ./benchmarks/analyze_results.py ./benchmark-results-quick
cat benchmark-results-quick/final_report.txt
```

**Decision**: Proceed with full benchmark or debug issues?

---

## Ready to Run!

```bash
# Complete quick benchmark in 35 minutes:
cd /home/jmalicki/src/io_uring_sync
cargo build --release
sudo ./benchmarks/generate_testdata_quick.sh /mnt/source-nvme
sudo ./benchmarks/run_benchmarks_quick.sh

# Then review
python3 ./benchmarks/analyze_results.py ./benchmark-results-quick-*
cat benchmark-results-quick-*/hardware_detailed.txt
```

**Let's see what the hardware can really do!** 🚀


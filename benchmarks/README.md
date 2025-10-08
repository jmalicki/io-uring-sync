# Benchmarking Suite for High-Performance NVMe RAID

This directory contains scripts for rigorous benchmarking of arsync vs rsync on high-performance NVMe RAID arrays (>15 GB/s capable).

## Quick Start

```bash
# 1. Generate test data (requires ~1TB free space on source array)
sudo ./generate_testdata.sh /mnt/source-nvme

# 2. Build arsync in release mode
cd .. && cargo build --release && cd benchmarks

# 3. Run benchmarks (requires root for cache dropping)
sudo ./run_benchmarks.sh \
    /mnt/source-nvme/benchmark-data \
    /mnt/dest-nvme/benchmark-output \
    ./results-$(date +%Y%m%d)

# 4. Analyze results
python3 ./analyze_results.py ./results-$(date +%Y%m%d)
```

## Requirements

### Hardware
- Two NVMe RAID arrays (or separate fast storage)
- Source array: Contains test data (read-only during tests)
- Destination array: Wiped between tests
- Recommended: >15 GB/s capable storage
- Recommended: 16+ cores for parallel tests

### Software
- Linux kernel 5.15+ (for io_uring)
- Root access (for dropping caches)
- Python 3.7+ with scipy, numpy
- GNU parallel (optional, for parallel rsync tests)
- rsync 3.0+

## Test Scenarios

### 1. Single Large Files (100GB, 200GB, 500GB)
Tests sustained sequential throughput on large files.
**Goal**: Verify maximum bandwidth utilization

### 2. Many Small Files (10K-1M files, 1KB-100KB each)
Tests syscall overhead and metadata performance.
**Goal**: Show io_uring's advantage in high IOPS scenarios

### 3. Deep Directory Trees
Tests directory traversal overhead.
**Goal**: Verify efficient filesystem navigation

### 4. Hardlink Scenarios (50%, 90% hardlinked)
Tests hardlink detection and memory usage.
**Goal**: Show single-pass vs two-pass advantages

### 5. Mixed Workloads (photos, kernel source)
Tests realistic usage patterns.
**Goal**: Demonstrate real-world performance

### 6. Metadata-Heavy (xattrs, ACLs)
Tests extended attribute handling.
**Goal**: Verify metadata preservation performance

## Methodology

### Statistical Rigor
- **5 runs** per test (first run discarded as warm-up)
- **Cache dropping** between all runs
- **Statistical analysis**: mean, median, std dev, Cohen's d, t-test
- **Confidence intervals**: 95% CI reported
- **Outlier detection**: Automated

### Environmental Controls
- CPU governor set to "performance"
- Background services disabled
- RAID in idle (no rebuild)
- I/O quiesce period between tests
- Full cache drop (`echo 3 > /proc/sys/vm/drop_caches`)

### Measurements
- Wall clock time (primary metric)
- CPU time (user + system)
- Maximum RSS (memory usage)
- I/O statistics (via iostat)
- Throughput (GB/s)
- Files per second

## Output Structure

```
results-YYYYMMDD/
├── system_info.txt                      # System configuration
├── 01_rsync_100gb/                      # Test suite directory
│   ├── 01_rsync_100gb_run1_elapsed.txt  # Raw measurements
│   ├── 01_rsync_100gb_run1_throughput.txt
│   ├── 01_rsync_100gb_run1_time.log     # /usr/bin/time output
│   ├── 01_rsync_100gb_run1_iostat.log   # I/O statistics
│   ├── ... (runs 2-5)
│   └── summary.txt                       # Statistical summary
├── 02_arsync_100gb/
│   └── ... (same structure)
├── ... (all test suites)
├── final_report.txt                      # Human-readable report
└── results.json                          # Machine-readable data
```

## Results Interpretation

### Speedup Calculation
- For elapsed time: `speedup = rsync_time / arsync_time`
- For throughput: `speedup = arsync_throughput / rsync_throughput`

### Statistical Significance
- **p < 0.05**: Statistically significant difference
- **Cohen's d**:
  - < 0.2: Small effect
  - 0.2-0.8: Medium effect
  - \> 0.8: Large effect

### Coefficient of Variation (CV)
- Measures run-to-run consistency
- CV < 0.05: Excellent consistency
- CV < 0.10: Good consistency
- CV > 0.10: High variability (investigate)

## Common Issues

### Test Times Out
- Arrays may be slower than expected
- Increase timeout in `run_benchmarks.sh`
- Check RAID health: `cat /proc/mdstat`

### High Variability
- Other I/O activity present
- Check: `iostat -x 1`
- Disable background services
- Check CPU thermal throttling

### rsync Faster Than Expected
- May be hitting cache
- Verify cache drop: `free -h` before test
- Check source is actually on NVMe

### Destination Not Cleared
- Manual cleanup: `rm -rf /dest/array/benchmark-output/*`
- Verify with: `ls -la /dest/array/benchmark-output/`

## Parallel rsync Testing

The suite automatically tests rsync with GNU parallel:

```bash
# This is tested automatically if parallel is installed
find /source -type f | parallel -j 16 rsync -a {} /dest/{}
```

This represents the **best-case** parallel rsync performance for fair comparison.

## Customization

### Adjust Test Parameters

Edit `run_benchmarks.sh`:
- `NUM_RUNS`: Number of iterations (default: 5)
- `CPUS`: Number of cores for parallel tests
- Add/remove test scenarios

### Generate Different Test Data

Edit `generate_testdata.sh`:
- File sizes
- File counts
- Directory depths
- Hardlink ratios

### Analysis Options

Edit `analyze_results.py`:
- Statistical methods
- Report format
- Graph generation (TODO)

## Next Steps After Benchmarking

1. **Review Results**:
   ```bash
   cat results-YYYYMMDD/final_report.txt
   ```

2. **Update README.md**:
   - Copy numbers from "README.md TEMPLATE" section
   - Add "Benchmarked on: <system specs>"
   - Include kernel version, array details

3. **Archive Results**:
   ```bash
   tar czf benchmark-results-YYYYMMDD.tar.gz results-YYYYMMDD/
   ```

4. **Share/Publish**:
   - Include in GitHub release
   - Link from main README
   - Consider publishing methodology paper

## References

- [Rigorous Benchmarking in Reasonable Time](https://www.cse.unsw.edu.au/~gernot/benchmarking-crimes.html)
- [SPEC CPU Methodology](https://www.spec.org/cpu2017/Docs/overview.html)
- [Linux perf tools](http://www.brendangregg.com/linuxperf.html)
- [Systems Performance](http://www.brendangregg.com/systems-performance-2nd-edition-book.html)

## Contact

For questions about methodology or results interpretation, open an issue on GitHub.


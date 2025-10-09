# Test Parameters

## Benchmark Configuration

**Date**: Wed Oct  8 17:27:38 PDT 2025
**Source directory**: /mnt/newhome/benchmark-data-quick
**Destination directory**: /mnt/newhome/benchmark-output-quick
**Results directory**: /home/jmalicki/src/io_uring_sync/benchmark-results-quick-20251008_172738

## Test Methodology

### Environmental Controls

- **Cache management**: Dropped between all runs (`echo 3 > /proc/sys/vm/drop_caches`)
- **CPU governor**: Set to 'performance' mode
- **Background services**: [Should be disabled]
- **I/O quiesce**: 2-3 second wait between tests

### Statistical Approach

- **Runs per test**: 5  # Run each test 5 times, discard first (warm-up)
- **Warm-up**: First run discarded
- **Analysis**: Mean, median, std dev, p-values, Cohen's d
- **Significance threshold**: p < 0.05

### Tools Tested

- **rsync**: rsync  version 3.4.1  protocol version 32
- **arsync**: version TBD

### Monitoring

- **I/O statistics**: iostat (1 second intervals)
- **Time measurement**: /usr/bin/time -v (detailed metrics)
- **Power monitoring**: DISABLED

## Test Scenarios

See individual test directories (01_*, 02_*, etc.) for specific scenarios.

Each scenario tests:
1. rsync (baseline)
2. arsync (comparison)

## Data Validation

Before analyzing results, verify:
- [ ] All tests completed without errors
- [ ] File counts match between rsync and arsync outputs
- [ ] No thermal throttling occurred (check temps in logs)
- [ ] Variance is reasonable (CV < 10%)
- [ ] No background I/O interference

## Interpreting Results

### Expected Performance Characteristics

Based on hardware:
- **RAID0**: Should see near-linear scaling with device count
- **RAID1**: Read bandwidth ~1x (single device), write ~0.5x
- **RAID10**: Read ~2x, write ~1x (for 4-drive RAID10)

### arsync vs rsync

Expected improvements:
- **Large files**: 10-30% (from fallocate/fadvise)
- **Small files**: 1.5-5x (from io_uring batching)
- **Hardlinks**: 10-20x faster startup

## Notes

All raw data is preserved in this directory for:
- Reproducibility
- Independent verification
- Future analysis
- Publication reference

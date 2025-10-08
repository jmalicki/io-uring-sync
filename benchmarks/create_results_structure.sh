#!/bin/bash
# Creates well-documented results directory structure
# Called automatically by run_benchmarks*.sh

RESULTS_DIR="$1"
SOURCE_DIR="$2"
DEST_DIR="$3"

mkdir -p "$RESULTS_DIR"
cd "$RESULTS_DIR"

# Create README for this benchmark run
cat > README.md << 'EOFREADME'
# Benchmark Results

**Date**: $(date)
**Benchmark Type**: $(basename "$0")

## Directory Structure

```
.
├── README.md                    # This file - overview
├── HARDWARE.md                  # Complete hardware specifications  
├── TEST_PARAMETERS.md           # Test configuration and parameters
├── hardware_detailed.txt        # Raw hardware inventory output
├── system_info.txt              # System summary
├── final_report.txt             # Statistical analysis results (generated after tests)
├── results.json                 # Machine-readable results (generated after tests)
├── 01_rsync_*/                  # Individual test results
│   ├── *_run1_elapsed.txt       # Raw timing data
│   ├── *_run1_throughput.txt    # Throughput calculation
│   ├── *_run1_power.csv         # Power measurements (if available)
│   ├── *_run1_iostat.log        # I/O statistics
│   ├── *_run1_time.log          # /usr/bin/time output
│   └── summary.txt              # Test summary statistics
└── ... (all test scenarios)
```

## How to Read These Results

### 1. Start with Hardware Specs
```bash
cat HARDWARE.md              # Human-readable hardware summary
cat hardware_detailed.txt    # Complete technical details
```

### 2. Review Test Parameters
```bash
cat TEST_PARAMETERS.md       # What was tested and how
```

### 3. View Performance Results
```bash
cat final_report.txt         # After running analyze_results.py
```

### 4. Examine Individual Tests
```bash
# For a specific test
cat 01_rsync_10gb/summary.txt
cat 02_arsync_10gb/summary.txt

# Compare power usage (if available)
cat 01_rsync_10gb/*_power.csv
cat 02_arsync_10gb/*_power.csv
```

## Verification

All raw data is included for reproducibility:
- Timing: `*_elapsed.txt` files
- Throughput: `*_throughput.txt` files  
- System metrics: `*_time.log`, `*_iostat.log`
- Power: `*_power.csv` (if RAPL available)

## Analysis

Run analysis on these results:
```bash
python3 ../benchmarks/analyze_results.py .
```

This generates:
- `final_report.txt` - Statistical summary
- `results.json` - Machine-readable data
EOFREADME

# Create HARDWARE.md template (will be filled by hardware_inventory.sh)
cat > HARDWARE.md << 'EOFHW'
# Hardware Configuration

This benchmark was run on the following hardware:

## Quick Summary

**CPU**: [Extracted from hardware inventory]
**RAM**: [Extracted from hardware inventory]  
**Storage**: [Extracted from hardware inventory]
**OS**: [Extracted from system info]

See `hardware_detailed.txt` for complete technical specifications.

## Storage Arrays

### Source Array
- **Device**: $SOURCE_DIR
- **Backing devices**: [From hardware inventory]
- **RAID level**: [From hardware inventory]
- **Expected bandwidth**: [TBD from specs]

### Destination Array  
- **Device**: $DEST_DIR
- **Backing devices**: [From hardware inventory]
- **RAID level**: [From hardware inventory]
- **Expected bandwidth**: [TBD from specs]

## Why This Hardware Configuration Matters

The test results should be interpreted in context of:
- RAID level affects theoretical bandwidth
- Number of NVMe devices affects parallelism
- PCIe generation/lanes affect maximum speed
- CPU core count affects parallel processing
- RAM speed affects buffer operations

See `TEST_PARAMETERS.md` for how we configured the tests for this hardware.
EOFHW

# Create TEST_PARAMETERS.md
cat > TEST_PARAMETERS.md << EOFPARAMS
# Test Parameters

## Benchmark Configuration

**Date**: $(date)
**Source directory**: $SOURCE_DIR
**Destination directory**: $DEST_DIR
**Results directory**: $RESULTS_DIR

## Test Methodology

### Environmental Controls

- **Cache management**: Dropped between all runs (\`echo 3 > /proc/sys/vm/drop_caches\`)
- **CPU governor**: Set to 'performance' mode
- **Background services**: [Should be disabled]
- **I/O quiesce**: 2-3 second wait between tests

### Statistical Approach

- **Runs per test**: $(grep "NUM_RUNS=" ../benchmarks/run_benchmarks*.sh | head -1 | cut -d= -f2 | tr -d '"')
- **Warm-up**: First run discarded
- **Analysis**: Mean, median, std dev, p-values, Cohen's d
- **Significance threshold**: p < 0.05

### Tools Tested

- **rsync**: $(rsync --version | head -1)
- **arsync**: $(./target/release/arsync --version 2>/dev/null || echo "version TBD")

### Monitoring

- **I/O statistics**: iostat (1 second intervals)
- **Time measurement**: /usr/bin/time -v (detailed metrics)
- **Power monitoring**: $([ "$ENABLE_POWER_MONITORING" = "yes" ] && echo "ENABLED (RAPL)" || echo "DISABLED")

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
EOFPARAMS

echo "✓ Results directory structure created: $RESULTS_DIR"
echo "  - README.md: Overview"
echo "  - HARDWARE.md: Hardware specs (template)"
echo "  - TEST_PARAMETERS.md: Test configuration"


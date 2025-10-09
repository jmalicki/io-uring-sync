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

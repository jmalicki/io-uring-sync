# Power Consumption Measurement

## Yes, It's Doable! And Quite Easy on Linux

Power measurement can provide interesting insights into io_uring efficiency beyond just performance.

---

## Why Measure Power?

### Efficiency Benefits of io_uring

io_uring's async architecture should theoretically be more power-efficient:
- **Fewer context switches** → Less CPU wake-ups
- **Batch processing** → More time in low-power states
- **Better cache locality** → Less memory traffic
- **Reduced syscall overhead** → Less kernel time

### Real-World Impact

- **Data centers**: Power = significant operational cost
- **Laptops**: Battery life improvements
- **Sustainability**: Carbon footprint reduction
- **Marketing**: "Not just faster, but greener"

---

## Linux Power Measurement Tools

### 1. RAPL (Running Average Power Limit) - **Best Option**

**What it is**: Intel/AMD hardware interface for measuring CPU package power

**Access**:
```bash
# Check if RAPL is available
ls /sys/class/powercap/intel-rapl/

# Read package power (microjoules)
cat /sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj
```

**How to use**:
- Sample energy counter before/after test
- Calculate: Power (W) = Energy_delta (J) / Time_delta (s)
- Requires root on some systems

**Pros**: 
- ✅ Hardware-accurate
- ✅ Low overhead
- ✅ Built into Intel CPUs (Sandy Bridge+) and AMD (Zen+)

**Cons**:
- ❌ CPU package only (doesn't include storage, RAM)
- ❌ Requires modern CPU

### 2. `turbostat` - Intel Tool

```bash
# Monitor power consumption
sudo turbostat --interval 1
```

**Output includes**:
- Package power (Watts)
- Core power
- CPU frequency
- C-state residency

**Pros**:
- ✅ Easy to use
- ✅ Rich information

**Cons**:
- ❌ Intel only
- ❌ Text output (harder to parse)

### 3. `powertop` - Generic Tool

```bash
# Interactive monitoring
sudo powertop

# CSV output mode
sudo powertop --csv=power.csv --time=60
```

**Pros**:
- ✅ Works on most systems
- ✅ Includes peripheral power estimates

**Cons**:
- ❌ Estimates, not measurements
- ❌ Higher overhead

### 4. `perf` Energy Counters

```bash
# Measure energy during command
sudo perf stat -e power/energy-pkg/ \
    -e power/energy-cores/ \
    -e power/energy-ram/ \
    ./your_command
```

**Pros**:
- ✅ Integrates with existing perf infrastructure
- ✅ Per-process attribution

**Cons**:
- ❌ Requires recent kernel
- ❌ RAPL-based (same limitations)

---

## Our Implementation: `power_monitoring.sh`

We've created a lightweight power monitoring script:

### Usage

```bash
# Enable power monitoring in benchmarks
export ENABLE_POWER_MONITORING=yes
sudo ./benchmarks/run_benchmarks.sh ...

# Or standalone
sudo ./benchmarks/power_monitoring.sh power_output.csv <pid>
```

### What It Measures

- **Package power** (via RAPL) - CPU + integrated GPU + memory controller
- **CPU frequency** - To correlate with power
- **CPU temperature** - To detect thermal throttling
- **CPU utilization** - To normalize power per utilization

### Output Format (CSV)

```
timestamp,package_power_watts,cpu_freq_mhz,cpu_temp_c,utilization_pct
1696800000.123,45.23,3400,65,78
1696800001.124,47.81,3500,66,82
...
```

### Automatic Summary

The script calculates:
- **Mean power**: Average watts during test
- **Energy consumed**: Total joules (and Wh)
- **Peak power**: Maximum observed

---

## Integration with Benchmark Suite

### Enabled by Environment Variable

```bash
# Enable power monitoring for all tests
export ENABLE_POWER_MONITORING=yes
sudo ./benchmarks/run_benchmarks.sh /src /dest /results
```

### Per-Test Power Data

Each test gets:
- `test_name_runN_power.csv` - Raw power measurements
- Integrated into analysis script

### Analysis Enhancement

Update `analyze_results.py` to include:

```python
def analyze_power(test_dir):
    """Analyze power consumption from power.csv files."""
    powers = []
    energies = []
    
    for f in glob.glob(f"{test_dir}/*_power.csv"):
        df = pd.read_csv(f)
        powers.append(df['package_power_watts'].mean())
        # Energy = power × time
        energies.append(df['package_power_watts'].sum())
    
    return {
        'mean_power': np.mean(powers),
        'total_energy': np.mean(energies),  # Joules
    }
```

---

## Expected Results: arsync vs rsync

### Hypothesis: arsync Should Be More Power-Efficient

**For small files (1M × 1KB)**:
- rsync: ~100K syscalls/sec = continuous CPU wake-ups
- arsync: Batched operations = more idle time
- **Expected**: arsync uses **10-30% less power** per file copied

**For large files (200GB)**:
- Both saturate I/O → similar power
- Slight advantage to arsync (less syscall overhead)
- **Expected**: arsync uses **5-15% less power** total

**Metric to report**: 
- **Power (W)**: Average during test
- **Energy (J)**: Total consumed
- **Energy per file (J/file)**: Efficiency metric
- **Energy per GB (J/GB)**: For large files

---

## Interesting Metrics to Calculate

### 1. Performance per Watt

```
Efficiency = Throughput (MB/s) / Power (W)
          = MB/s/W

Higher is better
```

Example:
- rsync: 500 MB/s @ 50W = 10 MB/s/W
- arsync: 1000 MB/s @ 55W = 18.2 MB/s/W
- **arsync is 82% more efficient!**

### 2. Energy to Completion

```
Total Energy = Mean Power (W) × Time (s)
             = Joules

Lower is better
```

Example (100K files):
- rsync: 60s @ 50W = 3000 J
- arsync: 20s @ 55W = 1100 J
- **arsync uses 63% less energy!**

### 3. Energy per File

```
Energy per file = Total Energy (J) / File Count

Lower is better
```

Great for small file tests!

---

## Limitations & Caveats

### What RAPL Measures
✅ **Includes**: CPU cores, L3 cache, memory controller, integrated GPU
❌ **Excludes**: Storage devices, discrete GPU, DRAM (on some CPUs), chipset

### Why This Is Still Useful
- **CPU is the bottleneck** for small file operations
- Syscall overhead (what we're optimizing) happens on CPU
- Storage power is relatively constant (sequential I/O)

### What We're Missing
- NVMe SSD power consumption
  - Modern NVMe: ~2-8W active, ~0.5W idle
  - Would need external power meter or drive telemetry
- System-wide power
  - Would need wall power meter (Kill-A-Watt, etc.)

---

## Optional: Whole-System Power Measurement

### For Complete Picture (Advanced)

**Option 1: Wall Power Meter**
- Device: Kill-A-Watt, Watts Up Pro
- Measures: Entire system power from wall
- Accuracy: ±1-2%
- Cost: $20-100

**Option 2: PDU with Power Monitoring**
- Enterprise solution
- SNMP-queryable power data
- Cost: $200-500

**Option 3: Smart Plug with API**
- TP-Link Kasa, Shelly Plug
- REST API for power data
- Cost: $15-30

**Implementation**:
```bash
# Query smart plug during test
while true; do
    curl -s http://plug-ip/power | jq .watts >> power.log
    sleep 1
done
```

---

## Should We Enable This?

### Pros
✅ **Interesting data**: Power efficiency is compelling
✅ **Easy to implement**: RAPL is built-in
✅ **Low overhead**: ~1% CPU for monitoring
✅ **Differentiator**: Most benchmarks don't include power
✅ **Sustainability angle**: "Greener" file copying

### Cons
❌ **Not primary focus**: Performance is main goal
❌ **Platform-specific**: RAPL on Intel/AMD only
❌ **Incomplete picture**: Missing storage/system power
❌ **Added complexity**: More data to analyze

### Recommendation

**Start with optional**:
```bash
# Default: disabled
./run_benchmarks.sh ...

# Enable if interested
ENABLE_POWER_MONITORING=yes ./run_benchmarks.sh ...
```

**If results are interesting**: Highlight in README
- "Not just 2x faster, but uses 60% less energy"
- Great for sustainability-focused users
- Valuable for laptop/portable use cases

**If results are boring**: Keep data but don't emphasize

---

## Example Results Section

```markdown
## Performance & Efficiency

### 100,000 × 1KB Files

| Tool | Time | Throughput | Power | Energy | Efficiency |
|------|------|------------|-------|--------|------------|
| rsync | 60s | 500 MB/s | 50W | 3000 J | 10 MB/s/W |
| arsync | 20s | 1500 MB/s | 55W | 1100 J | 27 MB/s/W |

**arsync is**:
- ⚡ 3x faster
- 🔋 63% less energy consumed
- ♻️ 2.7x more efficient (performance per watt)
```

---

## References

- [Intel RAPL](https://www.intel.com/content/www/us/en/developer/articles/technical/software-security-guidance/advisory-guidance/running-average-power-limit-energy-reporting.html)
- [Linux RAPL interface](https://www.kernel.org/doc/html/latest/power/powercap/powercap.html)
- [perf energy counters](https://perf.wiki.kernel.org/index.php/Main_Page)
- [Green Computing](https://en.wikipedia.org/wiki/Green_computing)

---

## Quick Start

```bash
# 1. Check if RAPL is available
ls /sys/class/powercap/intel-rapl/

# 2. Test power monitoring script
sudo ./benchmarks/power_monitoring.sh test_power.csv

# 3. Run benchmarks with power monitoring
ENABLE_POWER_MONITORING=yes sudo ./benchmarks/run_benchmarks.sh ...

# 4. View power summary
cat results/test_name_run2_power.csv
```

**Bottom line**: Power measurement is easy to add and could provide compelling differentiation! 🔋


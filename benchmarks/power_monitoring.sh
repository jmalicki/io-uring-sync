#!/bin/bash
# Power consumption monitoring for benchmarks
# Measures system power during benchmark runs
#
# Usage: ./power_monitoring.sh <output_file> <pid_to_monitor>

set -euo pipefail

OUTPUT_FILE="${1:-power_measurements.csv}"
MONITOR_PID="${2:-}"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "WARNING: Not running as root. Some power measurements may be unavailable."
fi

echo "timestamp,package_power_watts,cpu_freq_mhz,cpu_temp_c,utilization_pct" > "$OUTPUT_FILE"

# Function to read RAPL (Running Average Power Limit) - Intel/AMD
read_rapl_power() {
    local power=0
    
    # Intel RAPL interface
    if [ -d /sys/class/powercap/intel-rapl ]; then
        for rapl in /sys/class/powercap/intel-rapl/intel-rapl:*/energy_uj; do
            if [ -f "$rapl" ]; then
                local uj=$(cat "$rapl")
                # Convert microjoules to watts (need to sample over time)
                power=$((power + uj))
            fi
        done
    fi
    
    echo "$power"
}

# Function to read CPU frequency
read_cpu_freq() {
    if [ -f /proc/cpuinfo ]; then
        grep "cpu MHz" /proc/cpuinfo | head -1 | awk '{print $4}' || echo "0"
    else
        echo "0"
    fi
}

# Function to read CPU temperature
read_cpu_temp() {
    if [ -d /sys/class/thermal ]; then
        local temp_file=$(find /sys/class/thermal -name "temp" | head -1)
        if [ -f "$temp_file" ]; then
            local temp=$(cat "$temp_file")
            # Convert millidegrees to degrees
            echo "scale=1; $temp / 1000" | bc
        else
            echo "0"
        fi
    else
        echo "0"
    fi
}

# Function to read CPU utilization
read_cpu_util() {
    if command -v mpstat &> /dev/null; then
        mpstat 1 1 | tail -1 | awk '{print 100 - $NF}' || echo "0"
    else
        # Fallback: use top
        top -bn1 | grep "Cpu(s)" | awk '{print $2}' | sed 's/%us,//' || echo "0"
    fi
}

# Monitoring loop
echo "Starting power monitoring..."
echo "Output: $OUTPUT_FILE"
[ -n "$MONITOR_PID" ] && echo "Monitoring PID: $MONITOR_PID"

# Initial RAPL reading for delta calculation
last_energy=$(read_rapl_power)
last_time=$(date +%s.%N)

while true; do
    # Check if PID still exists (if monitoring specific process)
    if [ -n "$MONITOR_PID" ]; then
        if ! kill -0 "$MONITOR_PID" 2>/dev/null; then
            echo "Monitored process $MONITOR_PID has terminated"
            break
        fi
    fi
    
    sleep 1
    
    # Read current values
    current_time=$(date +%s.%N)
    current_energy=$(read_rapl_power)
    
    # Calculate power (energy delta / time delta)
    time_delta=$(echo "$current_time - $last_time" | bc)
    energy_delta=$(echo "$current_energy - $last_energy" | bc)
    
    # Convert microjoules to watts
    if [ "$time_delta" != "0" ]; then
        power_watts=$(echo "scale=2; ($energy_delta / $time_delta) / 1000000" | bc)
    else
        power_watts="0"
    fi
    
    # Read other metrics
    cpu_freq=$(read_cpu_freq)
    cpu_temp=$(read_cpu_temp)
    cpu_util=$(read_cpu_util)
    
    # Log to CSV
    timestamp=$(date +%s.%N)
    echo "$timestamp,$power_watts,$cpu_freq,$cpu_temp,$cpu_util" >> "$OUTPUT_FILE"
    
    # Update last values
    last_energy=$current_energy
    last_time=$current_time
done

echo "Power monitoring complete"
echo "Results saved to: $OUTPUT_FILE"

# Calculate summary statistics
if command -v python3 &> /dev/null; then
    python3 << 'EOF'
import csv
import sys

try:
    with open(sys.argv[1]) as f:
        reader = csv.DictReader(f)
        powers = [float(row['package_power_watts']) for row in reader if float(row['package_power_watts']) > 0]
    
    if powers:
        print(f"\nPower Statistics:")
        print(f"  Mean: {sum(powers)/len(powers):.2f} W")
        print(f"  Min:  {min(powers):.2f} W")
        print(f"  Max:  {max(powers):.2f} W")
        
        # Calculate energy consumed (Watt-seconds)
        energy_joules = sum(powers)  # Each sample is ~1 second
        print(f"  Total energy: {energy_joules:.0f} J ({energy_joules/3600:.3f} Wh)")
except Exception as e:
    print(f"Could not calculate statistics: {e}")
EOF
fi


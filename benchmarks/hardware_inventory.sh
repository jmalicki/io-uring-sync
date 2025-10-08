#!/bin/bash
# Comprehensive hardware inventory for benchmark documentation
# Discovers actual NVMe devices backing RAID arrays and their configuration
#
# Usage: sudo ./hardware_inventory.sh [output_file]

set -euo pipefail

OUTPUT_FILE="${1:-hardware_inventory.txt}"

exec > >(tee "$OUTPUT_FILE")

echo "========================================="
echo "HARDWARE INVENTORY FOR BENCHMARKING"
echo "========================================="
echo "Generated: $(date)"
echo "Hostname: $(hostname)"
echo ""

# System info
echo "=== SYSTEM INFORMATION ==="
echo "Kernel: $(uname -r)"
echo "Distribution: $(cat /etc/os-release | grep PRETTY_NAME | cut -d'"' -f2)"
echo "Architecture: $(uname -m)"
echo ""

# CPU details
echo "=== CPU INFORMATION ==="
echo "Full CPU details:"
lscpu
echo ""

echo "CPU Summary:"
echo "  Model: $(lscpu | grep "Model name" | cut -d: -f2 | xargs)"
echo "  Sockets: $(lscpu | grep "Socket(s)" | cut -d: -f2 | xargs)"
echo "  Cores per socket: $(lscpu | grep "Core(s) per socket" | cut -d: -f2 | xargs)"
echo "  Threads per core: $(lscpu | grep "Thread(s) per core" | cut -d: -f2 | xargs)"
echo "  Total logical CPUs: $(nproc)"
echo "  Architecture: $(lscpu | grep "Architecture" | cut -d: -f2 | xargs)"
echo "  CPU family: $(lscpu | grep "CPU family" | cut -d: -f2 | xargs || echo 'n/a')"
echo "  Model: $(lscpu | grep "^Model:" | cut -d: -f2 | xargs || echo 'n/a')"
echo "  Stepping: $(lscpu | grep "Stepping" | cut -d: -f2 | xargs || echo 'n/a')"
echo ""

echo "Cache Information:"
echo "  L1d cache: $(lscpu | grep "L1d cache" | cut -d: -f2 | xargs)"
echo "  L1i cache: $(lscpu | grep "L1i cache" | cut -d: -f2 | xargs)"
echo "  L2 cache: $(lscpu | grep "L2 cache" | cut -d: -f2 | xargs)"
echo "  L3 cache: $(lscpu | grep "L3 cache" | cut -d: -f2 | xargs)"
echo ""

echo "CPU Frequency:"
echo "  Current: $(lscpu | grep "CPU MHz" | cut -d: -f2 | xargs) MHz"
echo "  Min: $(lscpu | grep "CPU min MHz" | cut -d: -f2 | xargs || echo 'n/a') MHz"
echo "  Max: $(lscpu | grep "CPU max MHz" | cut -d: -f2 | xargs || echo 'n/a') MHz"
echo ""

echo "CPU Flags (key features):"
grep flags /proc/cpuinfo | head -1 | grep -o -E "sse|avx|aes|sha|pcid|pti|flush_l1d|rdrand" | sort -u | tr '\n' ' '
echo ""
echo ""

echo "Virtualization:"
echo "  VT-x/AMD-V: $(lscpu | grep "Virtualization" | cut -d: -f2 | xargs || echo 'Not available')"
echo ""

echo "CPU Vulnerabilities Mitigations:"
lscpu | grep -E "Vulnerability" | head -5 | sed 's/^/  /'
echo ""

echo "NUMA Configuration:"
if [ -d /sys/devices/system/node/node0 ]; then
    for node in /sys/devices/system/node/node*; do
        node_num=$(basename "$node" | sed 's/node//')
        if [ -f "$node/cpulist" ]; then
            cpus=$(cat "$node/cpulist")
            echo "  Node $node_num CPUs: $cpus"
        fi
    done
else
    echo "  Single NUMA node (UMA system)"
fi
echo ""

# Memory
echo "=== MEMORY INFORMATION ==="
echo "Memory Summary:"
free -h
echo ""

echo "Total RAM: $(free -h | grep Mem | awk '{print $2}')"
echo "Available: $(free -h | grep Mem | awk '{print $7}')"
echo ""

if command -v dmidecode &> /dev/null; then
    echo "Detailed Memory Information (from dmidecode):"
    
    # Count DIMMs
    dimm_count=$(dmidecode -t memory 2>/dev/null | grep -c "Size:.*MB\|Size:.*GB" || echo 0)
    echo "  DIMMs installed: $dimm_count"
    echo ""
    
    # Get details for each DIMM
    dmidecode -t memory 2>/dev/null | awk '
        /Memory Device$/,/^$/ {
            if ($1 == "Size:" && $2 != "No") size = $2 " " $3
            if ($1 == "Type:" && $2 != "Unknown") type = $2
            if ($1 == "Speed:" && $2 != "Unknown") speed = $2 " " $3
            if ($1 == "Manufacturer:") manuf = $2
            if ($1 == "Locator:") {
                locator = $2
                if (size != "" && type != "") {
                    printf "  %s: %s %s @ %s (%s)\n", locator, size, type, speed, manuf
                    size = ""; type = ""; speed = ""; manuf = ""
                }
            }
        }
    ' | grep -v "No Module" | head -20
    echo ""
    
    # Memory channel information
    echo "  Channel configuration:"
    dmidecode -t memory 2>/dev/null | grep -E "Locator:|Bank Locator:" | paste - - | head -8
    echo ""
else
    echo "dmidecode not available (install dmidecode for detailed memory info)"
    echo ""
fi

# Memory bandwidth (if available)
echo "Memory Bandwidth:"
if command -v dmidecode &> /dev/null; then
    # Calculate theoretical bandwidth
    mem_type=$(dmidecode -t memory 2>/dev/null | grep "Type:" | grep -v "Unknown\|Error" | head -1 | awk '{print $2}')
    mem_speed=$(dmidecode -t memory 2>/dev/null | grep "Speed:" | grep -v "Unknown" | head -1 | awk '{print $2}')
    mem_width=$(dmidecode -t memory 2>/dev/null | grep "Total Width:" | head -1 | awk '{print $3}')
    
    if [ -n "$mem_type" ] && [ -n "$mem_speed" ]; then
        echo "  Type: $mem_type"
        echo "  Speed: ${mem_speed} MT/s"
        echo "  Data width: ${mem_width:-64} bits"
        
        # Calculate theoretical bandwidth (rough estimate)
        # DDR = Double Data Rate, so MT/s * 8 bytes * channels
        if [ -n "$mem_speed" ]; then
            # Assume dual-channel (common), 64-bit width
            bw=$(echo "scale=1; $mem_speed * 8 * 2 / 1000" | bc 2>/dev/null || echo "n/a")
            echo "  Theoretical peak (dual-channel): ~${bw} GB/s"
        fi
    fi
else
    echo "  dmidecode not available"
fi
echo ""

# Memory timings if available
if [ -d /sys/devices/system/edac/mc ]; then
    echo "Memory Controller (EDAC):"
    for mc in /sys/devices/system/edac/mc/mc*; do
        if [ -d "$mc" ]; then
            mc_name=$(basename "$mc")
            size=$(cat "$mc/size_mb" 2>/dev/null || echo "unknown")
            echo "  $mc_name: ${size} MB"
        fi
    done
    echo ""
fi
echo ""

# Storage overview
echo "=== STORAGE OVERVIEW ==="
lsblk -o NAME,TYPE,SIZE,ROTA,DISC-GRAN,DISC-MAX,MODEL,TRAN
echo ""

# Detailed NVMe information
echo "=== NVME DEVICE DETAILS ==="
for nvme_dev in /dev/nvme*n1; do
    if [ -e "$nvme_dev" ]; then
        echo "--- Device: $nvme_dev ---"
        nvme list | grep "$(basename $nvme_dev)" || echo "nvme-cli not installed"
        
        # NVMe identify controller
        if command -v nvme &> /dev/null; then
            nvme id-ctrl "$nvme_dev" | grep -E "^mn |^sn |^fr " || true
        fi
        
        # Block device info
        echo "  Size: $(blockdev --getsize64 $nvme_dev | awk '{print $1/1024/1024/1024 " GB"}')"
        echo "  Sector size: $(blockdev --getss $nvme_dev) bytes"
        echo "  I/O scheduler: $(cat /sys/block/$(basename $nvme_dev)/queue/scheduler)"
        echo "  Queue depth: $(cat /sys/block/$(basename $nvme_dev)/queue/nr_requests)"
        echo "  Rotational: $(cat /sys/block/$(basename $nvme_dev)/queue/rotational)"
        echo "  Read-ahead: $(blockdev --getra $nvme_dev) KB"
        echo "  Max sectors: $(cat /sys/block/$(basename $nvme_dev)/queue/max_sectors_kb) KB"
        
        # PCIe info
        if [ -d "/sys/block/$(basename $nvme_dev)/device" ]; then
            local pci_addr=$(readlink /sys/block/$(basename $nvme_dev)/device | grep -o '[0-9a-f]\{4\}:[0-9a-f]\{2\}:[0-9a-f]\{2\}\.[0-9]' || echo "unknown")
            if [ "$pci_addr" != "unknown" ]; then
                echo "  PCIe Address: $pci_addr"
                lspci -s "$pci_addr" -vv 2>/dev/null | grep -E "LnkCap:|LnkSta:|NVMExp" | head -5 || true
            fi
        fi
        
        echo ""
    fi
done

# RAID array information
echo "=== RAID ARRAY CONFIGURATION ==="
if [ -f /proc/mdstat ]; then
    cat /proc/mdstat
    echo ""
    
    # For each MD device, get detailed info
    for md in /dev/md*; do
        if [ -b "$md" ]; then
            md_name=$(basename "$md")
            echo "--- RAID Device: $md ---"
            
            # RAID level and configuration
            if [ -f "/sys/block/$md_name/md/level" ]; then
                echo "  RAID Level: $(cat /sys/block/$md_name/md/level)"
            fi
            
            if [ -f "/sys/block/$md_name/md/raid_disks" ]; then
                echo "  Number of devices: $(cat /sys/block/$md_name/md/raid_disks)"
            fi
            
            if [ -f "/sys/block/$md_name/md/chunk_size" ]; then
                chunk_kb=$(cat /sys/block/$md_name/md/chunk_size)
                echo "  Chunk size: $((chunk_kb / 1024)) KB"
            fi
            
            if [ -f "/sys/block/$md_name/md/layout" ]; then
                echo "  Layout: $(cat /sys/block/$md_name/md/layout)"
            fi
            
            # Component devices
            echo "  Component devices:"
            for dev in /sys/block/$md_name/md/dev-*; do
                if [ -d "$dev" ]; then
                    dev_name=$(basename "$dev" | sed 's/dev-//')
                    block_dev=$(cat "$dev/block/dev" 2>/dev/null || echo "unknown")
                    state=$(cat "$dev/state" 2>/dev/null || echo "unknown")
                    echo "    - $dev_name: $block_dev (state: $state)"
                fi
            done
            
            # I/O scheduler for RAID
            echo "  I/O Scheduler: $(cat /sys/block/$md_name/queue/scheduler 2>/dev/null || echo 'n/a')"
            echo "  Queue depth: $(cat /sys/block/$md_name/queue/nr_requests 2>/dev/null || echo 'n/a')"
            echo "  Stripe cache size: $(cat /sys/block/$md_name/md/stripe_cache_size 2>/dev/null || echo 'n/a') pages"
            
            # Performance settings
            if [ -f "/sys/block/$md_name/md/sync_speed_min" ]; then
                echo "  Sync speed min: $(cat /sys/block/$md_name/md/sync_speed_min) KB/s"
            fi
            if [ -f "/sys/block/$md_name/md/sync_speed_max" ]; then
                echo "  Sync speed max: $(cat /sys/block/$md_name/md/sync_speed_max) KB/s"
            fi
            
            # Use mdadm for more details
            if command -v mdadm &> /dev/null; then
                echo ""
                echo "  Detailed mdadm info:"
                mdadm --detail "$md" 2>/dev/null | grep -E "Level|Devices|Chunk Size|State|Active Devices" | sed 's/^/    /'
            fi
            
            echo ""
        fi
    done
else
    echo "No MD RAID arrays found (not using software RAID)"
    echo ""
fi

# LVM information (in case using LVM instead of MD RAID)
echo "=== LVM CONFIGURATION ==="
if command -v pvs &> /dev/null; then
    echo "Physical Volumes:"
    pvs
    echo ""
    echo "Volume Groups:"
    vgs
    echo ""
    echo "Logical Volumes:"
    lvs
    echo ""
    
    # Detail for each LV
    for lv in $(lvs --noheadings -o lv_path); do
        echo "--- Logical Volume: $lv ---"
        lvdisplay "$lv" | grep -E "LV Name|VG Name|LV Size|Segments|Stripes|Stripe size"
        echo ""
    done
else
    echo "LVM not in use"
    echo ""
fi

# Filesystem information
echo "=== FILESYSTEM INFORMATION ==="
df -h -t ext4 -t xfs -t btrfs -t f2fs
echo ""

# Mount options
echo "=== MOUNT OPTIONS ==="
mount | grep -E "nvme|md|mapper" || echo "No NVMe/RAID mounts found"
echo ""

# For each mount, show detailed options
for mount_point in /mnt/*; do
    if mountpoint -q "$mount_point" 2>/dev/null; then
        echo "Mount: $mount_point"
        mount | grep "$mount_point" | sed 's/^/  /'
        
        # Filesystem type and features
        fs_type=$(stat -f -c %T "$mount_point")
        echo "  Filesystem: $fs_type"
        
        # XFS specific
        if [ "$fs_type" = "xfs" ]; then
            xfs_info "$mount_point" 2>/dev/null | grep -E "agcount|agsize|sunit|swidth|isize" | sed 's/^/    /' || true
        fi
        
        # ext4 specific
        if [ "$fs_type" = "ext4" ]; then
            tune2fs -l "$(df "$mount_point" | tail -1 | awk '{print $1}')" 2>/dev/null | grep -E "Block size|Stride|Stripe" | sed 's/^/    /' || true
        fi
        
        echo ""
    fi
done

# PCIe topology
echo "=== PCIe TOPOLOGY ==="
echo "NVMe Controllers:"
lspci | grep -i nvme
echo ""

# Detailed PCIe info for NVMe controllers
for pci_addr in $(lspci | grep -i nvme | cut -d' ' -f1); do
    echo "--- PCIe Device: $pci_addr ---"
    lspci -s "$pci_addr" -vv 2>/dev/null | grep -E "LnkCap:|LnkSta:|Width|Speed|MaxPayload" || true
    echo ""
done

# NUMA topology (important for multi-socket systems)
echo "=== NUMA TOPOLOGY ==="
if command -v numactl &> /dev/null; then
    numactl --hardware
else
    echo "numactl not installed"
fi
echo ""

# I/O Schedulers summary
echo "=== I/O SCHEDULER SUMMARY ==="
for dev in /sys/block/*/queue/scheduler; do
    block_dev=$(echo "$dev" | cut -d'/' -f4)
    if [[ "$block_dev" =~ (nvme|md|dm) ]]; then
        printf "%-15s: %s\n" "$block_dev" "$(cat $dev)"
    fi
done
echo ""

# Queue depths summary
echo "=== QUEUE DEPTH SUMMARY ==="
for dev in /sys/block/*/queue/nr_requests; do
    block_dev=$(echo "$dev" | cut -d'/' -f4)
    if [[ "$block_dev" =~ (nvme|md|dm) ]]; then
        printf "%-15s: %s\n" "$block_dev" "$(cat $dev)"
    fi
done
echo ""

# Performance governor check
echo "=== CPU GOVERNOR ==="
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo "CPU frequency scaling not available"
echo ""

# Transparent Huge Pages
echo "=== TRANSPARENT HUGE PAGES ==="
cat /sys/kernel/mm/transparent_hugepage/enabled 2>/dev/null || echo "THP info not available"
echo ""

# Swappiness
echo "=== SWAPPINESS ==="
cat /proc/sys/vm/swappiness
echo ""

# Summary for quick reference
echo "========================================="
echo "QUICK REFERENCE SUMMARY"
echo "========================================="
echo ""
echo "CPU: $(grep "model name" /proc/cpuinfo | head -1 | cut -d: -f2 | xargs)"
echo "Cores: $(nproc)"
echo "RAM: $(free -h | grep Mem | awk '{print $2}')"
echo ""

# RAID arrays
if [ -f /proc/mdstat ]; then
    echo "RAID Arrays:"
    grep "^md" /proc/mdstat | while read line; do
        md_dev=$(echo "$line" | awk '{print $1}')
        md_level=$(cat /sys/block/$md_dev/md/level 2>/dev/null || echo "unknown")
        md_disks=$(cat /sys/block/$md_dev/md/raid_disks 2>/dev/null || echo "?")
        md_chunk=$(cat /sys/block/$md_dev/md/chunk_size 2>/dev/null)
        md_chunk_kb=$((md_chunk / 1024))
        echo "  /dev/$md_dev: $md_level with $md_disks devices, ${md_chunk_kb}KB chunks"
    done
    echo ""
fi

# NVMe devices
echo "NVMe Devices:"
for nvme in /dev/nvme*n1; do
    if [ -e "$nvme" ]; then
        model=$(cat /sys/block/$(basename $nvme)/device/model 2>/dev/null | xargs || echo "unknown")
        size=$(blockdev --getsize64 $nvme | awk '{printf "%.0f GB", $1/1024/1024/1024}')
        echo "  $(basename $nvme): $model ($size)"
    fi
done
echo ""

echo "========================================="
echo "Inventory complete: $OUTPUT_FILE"
echo "========================================="


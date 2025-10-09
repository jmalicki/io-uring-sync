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

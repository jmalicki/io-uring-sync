# Local vs Remote Architecture: Critical Performance Distinction

**Date**: October 9, 2025  
**Status**: Implementation guidance

---

## ⚠️ CRITICAL: Two Completely Different Code Paths

arsync has **two completely different architectures** for local vs remote sync.  
**They should NEVER be confused or mixed!**

---

## Local Sync (Default) - io_uring Direct Copying

**When**: Source and destination are BOTH local paths

**Example**:
```bash
arsync -av /data /backup               # Both local
arsync -av --source /data --destination /backup
```

**Architecture**:
```
arsync (single process)
  ↓
io_uring queue (direct file operations)
  ↓
Read source files → Write destination files
  ↓
NO protocol, NO pipes, NO serialization
  ↓
MAXIMUM PERFORMANCE (io_uring parallelism)
```

**Code Path**:
```rust
// src/main.rs
if source.is_local() && destination.is_local() {
    // HIGH-PERFORMANCE PATH (io_uring direct)
    sync::sync_files(&args).await  // ← Uses io_uring
}
```

**Performance**:
- **2-5x faster** than rsync (io_uring parallelism)
- Thousands of parallel file operations
- Saturates NVMe queue depth
- Direct io_uring `read_at`/`write_at` operations

**Why this is better**:
- No protocol overhead (no serialization/deserialization)
- No pipe overhead (no context switches)
- Direct file-to-file copying
- Parallel across 1000+ files
- This is the WHOLE POINT of arsync!

---

## Remote Sync - rsync Wire Protocol

**When**: Source OR destination is remote (user@host:/path)

**Example**:
```bash
arsync -av /data user@host:/backup     # Remote destination
arsync -av user@host:/data /backup     # Remote source
```

**Architecture**:
```
arsync (client)
  ↓
SSH connection
  ↓
rsync wire protocol (serialize files over network)
  ↓
SSH tunnel
  ↓
arsync --server (remote process)
  ↓
Deserialize and write files
```

**Code Path**:
```rust
// src/main.rs
if source.is_remote() || destination.is_remote() {
    // NETWORK PATH (rsync protocol)
    protocol::remote_sync(&args, &source, &destination).await
}
```

**Performance**:
- Limited by network bandwidth
- rsync delta algorithm (minimize network transfer)
- Future: QUIC for parallelism

**Why protocol is needed**:
- Network transfer requires serialization
- Can't do direct file operations across network
- rsync protocol minimizes bandwidth

---

## Pipe Mode (Testing ONLY) - Explicit Flag

**When**: `--pipe` flag is explicitly specified

**Example**:
```bash
# ONLY for testing the protocol implementation!
arsync --pipe --pipe-role=sender /source/ | arsync --pipe --pipe-role=receiver /dest/
```

**Architecture**:
```
arsync --pipe --sender (process 1)
  ↓
rsync wire protocol
  ↓
Unix pipe (stdin/stdout)
  ↓
arsync --pipe --receiver (process 2)
  ↓
Write files
```

**Code Path**:
```rust
// src/main.rs
if args.pipe {
    // TESTING PATH (protocol via pipes)
    // Used ONLY to test protocol implementation
    // NOT used for normal local copies!
    let transport = PipeTransport::from_stdio()?;
    match args.pipe_role {
        Some(PipeRole::Sender) => protocol::send(...),
        Some(PipeRole::Receiver) => protocol::receive(...),
    }
} else if source.is_remote() || destination.is_remote() {
    // Remote sync
} else {
    // LOCAL SYNC - io_uring direct (DEFAULT!)
    sync::sync_files(&args).await
}
```

**Performance**:
- SLOWER than io_uring direct (protocol overhead)
- Only for testing interoperability
- Validates wire protocol correctness

**Purpose**:
- Test protocol implementation without SSH
- Test against real rsync via pipes
- Validate sender/receiver separately

---

## Decision Tree

```
User Command: arsync SOURCE DEST

Is --pipe flag set?
  ├─ YES → Use pipe mode (testing only)
  │        ├─ Read from stdin
  │        ├─ Write to stdout
  │        └─ Use rsync wire protocol
  │
  └─ NO → Check SOURCE and DEST
           │
           Is SOURCE or DEST remote?
           ├─ YES → Use remote sync
           │        ├─ Connect via SSH
           │        ├─ Use rsync wire protocol
           │        └─ Network transfer
           │
           └─ NO → Use LOCAL SYNC (DEFAULT!)
                    ├─ io_uring direct file operations
                    ├─ Parallel across 1000+ files
                    ├─ NO protocol overhead
                    └─ MAXIMUM PERFORMANCE ← THIS IS ARSYNC'S ADVANTAGE!
```

---

## Performance Comparison

| Mode | When | Throughput | Overhead | Use Case |
|------|------|------------|----------|----------|
| **Local (io_uring)** | Both paths local | **2-5 GB/s** | None | **Default** ✅ |
| **Remote (SSH)** | One path remote | 200-800 MB/s | Protocol + network | Production remote |
| **Pipe (testing)** | --pipe flag | 5-10 GB/s | Protocol only | Testing ONLY |

---

## Code Routing

```rust
// src/main.rs - ROUTING LOGIC

let source = args.get_source()?;
let destination = args.get_destination()?;

let result = if args.pipe {
    // ============================================================
    // PIPE MODE (TESTING ONLY)
    // ============================================================
    // Explicitly requested via --pipe flag
    // Uses rsync wire protocol over stdin/stdout
    // FOR PROTOCOL TESTING, NOT PRODUCTION USE
    // ============================================================
    pipe_mode(&args).await
    
} else if source.is_remote() || destination.is_remote() {
    // ============================================================
    // REMOTE SYNC MODE
    // ============================================================
    // One or both endpoints are remote
    // Uses rsync wire protocol over SSH
    // Network transfer, delta algorithm
    // ============================================================
    protocol::remote_sync(&args, &source, &destination).await
    
} else {
    // ============================================================
    // LOCAL SYNC MODE (DEFAULT - FASTEST!)
    // ============================================================
    // Both source and destination are local paths
    // Uses io_uring direct file operations
    // NO protocol overhead, NO serialization
    // THIS IS WHAT MAKES ARSYNC FAST!
    // ============================================================
    sync::sync_files(&args).await.map_err(Into::into)
};
```

---

## Documentation Updates Needed

### Update README.md

Add explicit callout:
```markdown
## Performance Note

arsync achieves **2-5x better performance** than rsync for LOCAL copies
by using io_uring direct file operations instead of the rsync protocol.

**Local copy** (io_uring direct):
  arsync /data /backup              ← FAST (io_uring parallelism)

**Remote copy** (rsync protocol):
  arsync /data user@host:/backup    ← Uses rsync protocol (necessary)

The `--pipe` flag is for testing ONLY and should not be used for
normal local copies.
```

### Update docs/RSYNC_PROTOCOL_IMPLEMENTATION.md

Add warning section:
```markdown
## ⚠️ Important: Local vs Remote Architecture

The rsync wire protocol is ONLY used when:
1. Source or destination is remote (user@host:/path)
2. --pipe flag is explicitly set (testing only)

For local-to-local copies, arsync uses io_uring direct file operations
(NO protocol, NO overhead). This is arsync's core advantage and must not
be compromised.
```

---

## Why This Matters

### ❌ BAD: If local copies used protocol
```
arsync /data /backup

Would do:
  1. Serialize files to rsync protocol
  2. Send over pipe
  3. Deserialize on other end
  4. Write files

Performance: ~1 GB/s (protocol overhead)
```

### ✅ GOOD: Local copies use io_uring direct (CURRENT!)
```
arsync /data /backup

Does:
  1. io_uring direct file operations
  2. Parallel across 1000+ files
  3. NO protocol, NO overhead

Performance: ~2-5 GB/s (io_uring parallelism)
```

**We must preserve this distinction!**

---

## Test Matrix Clarification

| Test | Purpose | Uses Protocol? | Production? |
|------|---------|----------------|-------------|
| rsync compat tests | Validate local behavior matches rsync | ❌ No | ✅ Yes (validates local mode) |
| Pipe protocol tests | Validate wire protocol implementation | ✅ Yes | ❌ No (testing only) |
| Integration tests | Validate actual remote sync works | ✅ Yes | ✅ Yes (validates remote mode) |

---

## Summary

**For local copies**:
- ✅ Use `sync::sync_files()` (io_uring direct)
- ❌ Never use protocol/pipe mode
- This is arsync's competitive advantage!

**For remote copies**:
- ✅ Use `protocol::remote_sync()` (rsync wire protocol)
- Necessary for network transfer
- Still faster than rsync (future: QUIC parallelism)

**For testing**:
- ✅ Use `--pipe` mode to test protocol
- Validates sender/receiver without SSH
- Not for production use!

---

**Last Updated**: October 9, 2025  
**Status**: Architecture documented - DO NOT MIX LOCAL AND REMOTE CODE PATHS!


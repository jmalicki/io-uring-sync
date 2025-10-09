# rsync Wire Protocol Implementation Plan

**Status**: Foundation Complete - Implementation In Progress  
**Date**: October 9, 2025  
**Stacked On**: research/remote-sync-protocol

---

## Overview

This document describes the implementation plan for rsync wire protocol compatibility in arsync. This enables arsync to work as a drop-in replacement for rsync in remote synchronization scenarios.

---

## Implemented (This PR)

### ✅ CLI Changes - rsync-style Positional Arguments

**Before** (arsync-specific):
```bash
arsync --source /data --destination /backup
```

**Now** (rsync-compatible):
```bash
arsync /data /backup                    # Local to local
arsync /data user@host:/backup          # Local to remote (push)
arsync user@host:/data /backup          # Remote to local (pull)
arsync -av /data host:/backup           # With flags (just like rsync!)
```

**Backward Compatibility**:
```bash
# Old style still works
arsync --source /data --destination /backup
```

### ✅ Location Parsing

Supports rsync-style syntax:
- `user@host:/path` - Remote with user
- `host:/path` - Remote (uses current user)
- `/local/path` - Local path
- Windows paths handled correctly (`C:\path`)

### ✅ Feature Flags

**New Cargo features**:
- `remote-sync` - Enables remote sync via SSH (uses tokio for process management)
- `quic` - Enables QUIC transport (requires `remote-sync`)

**Compilation**:
```bash
# Default: Local sync only
cargo build --release

# With remote sync
cargo build --release --features remote-sync

# With QUIC support
cargo build --release --features quic
```

### ✅ Module Structure

```
src/protocol/
├── mod.rs              # Entry point, routing logic
├── ssh.rs              # SSH connection management
├── rsync.rs            # rsync wire protocol (stubs)
└── quic.rs             # QUIC transport (stubs, feature-gated)
```

### ✅ Routing Logic

```rust
if source.is_remote() || destination.is_remote() {
    #[cfg(feature = "remote-sync")]
    protocol::remote_sync(...)  // Use protocol module
    
    #[cfg(not(feature = "remote-sync"))]
    // Error with helpful message
} else {
    sync::sync_files(...)  // Local sync (existing code)
}
```

---

## To Be Implemented (Next Steps)

### 🚧 Phase 1: rsync Protocol Handshake (Week 1-2)

**Goals**:
- [ ] Implement protocol version negotiation
- [ ] Capability exchange
- [ ] Checksum algorithm selection

**Files to Implement**:
- `src/protocol/rsync.rs::handshake()`

**Protocol Flow**:
```
Client → Server: Protocol version (31)
Server → Client: Protocol version (31)
Client → Server: Seed (random number)
Server → Client: Seed (random number)
// Now ready for file list exchange
```

### 🚧 Phase 2: File List Exchange (Week 3-4)

**Goals**:
- [ ] Generate file list from local directory
- [ ] Encode file list in rsync wire format
- [ ] Send/receive file lists

**Files to Implement**:
- `src/protocol/rsync.rs::generate_file_list()`
- `src/protocol/rsync.rs::send_file_list()`
- `src/protocol/rsync.rs::receive_file_list()`

**Wire Format** (to implement):
```
For each file:
  - varint: flags (file type, metadata present, etc.)
  - varint: path length
  - bytes: path (UTF-8)
  - varint: file size
  - varint: mtime
  - varint: mode
  - varint: uid (if preserving ownership)
  - varint: gid (if preserving ownership)
```

### 🚧 Phase 3: Block Checksums (Week 5-6)

**Goals**:
- [ ] Implement rolling checksum (Adler-32 variant)
- [ ] Implement strong checksum (MD5/SHA-256)
- [ ] Generate and transmit block checksums

**Files to Implement**:
- `src/protocol/rsync.rs::generate_block_checksums()`
- `src/protocol/checksums.rs` (new file)

**Algorithm**:
```rust
// Rolling checksum (can be updated incrementally)
fn rolling_checksum(data: &[u8]) -> u32;
fn roll_checksum(old: u32, old_byte: u8, new_byte: u8, window_size: usize) -> u32;

// Strong checksum (cryptographic verification)
fn strong_checksum(data: &[u8]) -> [u8; 16];  // MD5 or SHA-256
```

### 🚧 Phase 4: Delta Generation (Week 7-8)

**Goals**:
- [ ] Implement rsync rolling checksum matching algorithm
- [ ] Generate delta instructions
- [ ] Encode delta in wire format

**Files to Implement**:
- `src/protocol/rsync.rs::generate_delta()`
- `src/protocol/delta.rs` (new file)

**rsync Algorithm**:
```
1. Build hash table of receiver's block checksums
2. Scan sender file with rolling window
3. For each position:
   - Compute rolling checksum
   - If checksum matches table entry:
     - Compute strong checksum
     - If strong checksum also matches:
       → Emit "copy block N" instruction
   - Else:
     → Accumulate literal data
4. Emit literal data when threshold reached
```

### 🚧 Phase 5: Delta Application (Week 9-10)

**Goals**:
- [ ] Apply delta instructions to reconstruct file
- [ ] Handle both copy and literal instructions
- [ ] Verify final checksum

**Files to Implement**:
- `src/protocol/rsync.rs::apply_delta()`
- `src/protocol/reconstruct.rs` (new file)

### 🚧 Phase 6: Metadata Preservation (Week 11-12)

**Goals**:
- [ ] Transmit metadata (permissions, timestamps, owner/group)
- [ ] Apply metadata to synchronized files
- [ ] Handle extended attributes and ACLs over wire protocol

**Files to Implement**:
- `src/protocol/metadata.rs` (new file)

### 🚧 Phase 7: Integration Testing (Week 13-14)

**Goals**:
- [ ] Test against real rsync servers (rsync 2.x, 3.x)
- [ ] Test against arsync servers (self-sync)
- [ ] Compatibility test suite

**New Tests**:
- `tests/rsync_wire_protocol_tests.rs`
- `tests/remote_sync_integration_tests.rs`

---

## Future Enhancements

### Merkle Tree Protocol Extension (After Phase 7)

Once basic rsync compatibility is working, implement merkle tree extensions:

**Protocol Negotiation**:
```
Client → Server: Capabilities[rsync-v31, merkle-tree-v1]
Server → Client: Capabilities[rsync-v31, merkle-tree-v1]
→ Use merkle tree protocol
```

**Files to Add**:
- `src/protocol/merkle.rs`
- `src/protocol/merkle/tree.rs`
- `src/protocol/merkle/sync.rs`

### QUIC Transport (After Merkle Trees)

Implement SSH-QUIC hybrid protocol as designed in research docs:

**Files to Add**:
- `src/protocol/quic.rs` (expand stub)
- `src/protocol/quic/handshake.rs`
- `src/protocol/quic/streams.rs`

---

## Testing Strategy

### Unit Tests

**Per Phase**:
- Phase 1: Handshake parsing and encoding
- Phase 2: File list encoding/decoding
- Phase 3: Checksum algorithms (rolling + strong)
- Phase 4: Delta generation algorithm
- Phase 5: Delta application and reconstruction
- Phase 6: Metadata encoding/decoding

### Integration Tests

**Against Real rsync**:
```bash
# Start rsync server
rsync --daemon --config=test_rsyncd.conf

# Test arsync client → rsync server
arsync -av /source localhost::module/dest

# Test rsync client → arsync server
arsync --daemon &
rsync -av /source localhost::module/dest
```

### Compatibility Matrix

| Client | Server | Expected Result |
|--------|--------|-----------------|
| arsync | rsync 2.x | ✅ Works (basic protocol) |
| arsync | rsync 3.x | ✅ Works (modern features) |
| arsync | arsync | ✅ Works (can use extensions) |
| rsync | arsync | ✅ Works (server mode) |

---

## Dependencies Added

```toml
[dependencies]
# Remote sync
tokio = { version = "1.0", features = ["process", "io-util", "rt"], optional = true }
quinn = { version = "0.11", optional = true }
rustls = { version = "0.23", optional = true }
whoami = "1.0"  # For default username

[features]
remote-sync = ["tokio"]
quic = ["remote-sync", "quinn", "rustls"]
```

---

## Example Usage (After Implementation)

```bash
# Push to remote (arsync or rsync server)
arsync -av /local/data user@server:/backup/

# Pull from remote
arsync -av user@server:/data /local/backup/

# With progress
arsync -av --progress /data server:/backup/

# Dry run
arsync -av --dry-run /data server:/backup/

# Over non-default SSH port
arsync -e "ssh -p 2222" -av /data server:/backup/

# With QUIC (if compiled with --features quic)
arsync -av /data server:/backup/
# (automatically negotiates QUIC if both sides support it)
```

---

## Performance Expectations

### rsync Protocol Over SSH

**Baseline** (single SSH connection):
- Same as traditional rsync
- Sequential file transfer
- Limited by SSH encryption overhead

### With Future Enhancements

**Merkle Trees**:
- 50-90% bandwidth reduction for sparse changes
- O(log n) verification vs O(n)

**QUIC Transport**:
- 10-20x improvement on high-latency networks
- 1000+ parallel file transfers
- Independent stream congestion control

---

## References

- [rsync Algorithm Technical Report](https://rsync.samba.org/tech_report/)
- [rsync Source Code](https://github.com/RsyncProject/rsync)
- [librsync Library](https://librsync.github.io/)
- [Research: REMOTE_SYNC_RESEARCH.md](research/REMOTE_SYNC_RESEARCH.md)
- [Research: SSH_QUIC_HYBRID_PROTOCOL.md](research/SSH_QUIC_HYBRID_PROTOCOL.md)

---

**Document Version**: 1.0  
**Last Updated**: October 9, 2025  
**Status**: Foundation complete, implementation in progress


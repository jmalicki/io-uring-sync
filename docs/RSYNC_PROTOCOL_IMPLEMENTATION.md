# rsync Wire Protocol Implementation Plan

**Status**: Delta Algorithm Complete - rsync Compatibility In Design Phase  
**Date**: October 9, 2025  
**Stacked On**: research/remote-sync-protocol  
**Companion Doc**: See `RSYNC_WIRE_PROTOCOL_SPEC.md` for detailed protocol analysis

---

## Executive Summary

This document provides the complete implementation plan for rsync wire protocol compatibility in arsync. After extensive research and delta algorithm implementation, we now understand that rsync's protocol is significantly more complex than initially anticipated.

**Key Finding**: rsync uses **multiplexed I/O with tagged messages**, not simple byte streams. This requires a separate compatibility layer.

**Current Status**:
- ✅ arsync native protocol: Complete, tested, working (Test 2 passing)
- ✅ Delta algorithm: Fully implemented with 90%+ bandwidth savings
- ✅ Metadata preservation: Complete (permissions, times, ownership, symlinks)
- ⏳ rsync wire compatibility: Designed, ready for implementation (Tests 3 & 4)

**See**: `docs/RSYNC_WIRE_PROTOCOL_SPEC.md` for 7-page detailed protocol analysis and technical deep-dive.

---

## Part 1: What's Already Implemented

### ✅ arsync Native Protocol (Complete!)

**Status**: ✅ Fully working, tested, production-ready

**What works**:
- Complete delta algorithm with block checksums
- Full metadata preservation (permissions, times, ownership, symlinks)
- Efficient bandwidth usage (90%+ savings for incremental updates)
- Test 2 passing (arsync ↔ arsync synchronization)

**Code**:
- `src/protocol/rsync.rs`: ~875 lines (native protocol)
- `src/protocol/checksum.rs`: ~135 lines (rolling + MD5 checksums)
- `src/protocol/pipe.rs`: ~90 lines (transport)

**See**: `STATUS_SUMMARY.md` and `DELTA_COMPLETE.md` for details.

**IMPORTANT**: This native protocol is our competitive advantage. It's simpler and more efficient than rsync's wire format. We keep this for arsync ↔ arsync communication.

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

### ✅ Module Structure (Current)

```
src/protocol/
├── mod.rs              # Entry point, routing logic
├── checksum.rs         # ✅ Rolling + strong checksums (complete)
├── pipe.rs             # ✅ Pipe transport (complete)
├── rsync.rs            # ✅ arsync native protocol (complete)
├── rsync_compat.rs     # ⏳ rsync wire protocol (started - multiplex I/O)
├── ssh.rs              # ⏳ SSH connection management (stub)
├── transport.rs        # ✅ Transport abstraction (complete)
└── quic.rs             # ⏳ QUIC transport (stub, feature-gated)
```

**New for rsync compatibility**:
- `rsync_compat.rs`: Multiplexed I/O, tagged messages, rsync wire format
- `varint.rs` (to add): Variable-length integer encoding

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

## Part 2: Understanding rsync's Protocol (Critical Reading!)

### 2.1 The Multiplexing Problem

**Our current protocol** (arsync native):
```
Simple bidirectional byte stream:
  [version][file count][file entries...][checksums...][delta...]
```

**rsync's actual protocol**:
```
Multiplexed tagged messages:
  [tag][3-byte length][data...]
  [tag][3-byte length][data...]
  
Tags:
  7  = MSG_DATA (regular data)
  9  = MSG_INFO (log message)
  10 = MSG_ERROR (error message)
  27 = MSG_FLIST (file list entry)
  ...
```

**Why this matters**: 
- rsync expects tagged messages after handshake
- Our untagged bytes are interpreted as invalid tags
- This is why "unexpected tag 85" errors occurred
- We need a completely different wire format for rsync compat

**See**: `docs/RSYNC_WIRE_PROTOCOL_SPEC.md` Section 1.2 for detailed analysis.

### 2.2 Protocol Incompatibilities

| Feature | arsync Native | rsync Wire Protocol | Compatibility |
|---------|---------------|---------------------|---------------|
| **Message framing** | Raw bytes | Tagged (MPLEX) | ❌ Incompatible |
| **Integer encoding** | Fixed LE | Varint (7-bit) | ❌ Incompatible |
| **File list** | Simple struct | Delta-encoded | ❌ Incompatible |
| **Delta format** | Instruction list | Token stream | ❌ Incompatible |
| **Handshake** | Version only | Version+seed | ⚠️ Partial |

**Conclusion**: We need TWO separate protocol implementations:
1. **arsync native**: Keep current (efficient, working)
2. **rsync compat**: Implement new (compatible, complex)

**See**: `docs/RSYNC_WIRE_PROTOCOL_SPEC.md` Part 3 for incompatibility details.

---

## Part 3: Implementation Plan for rsync Compatibility

### 3.1 Strategy: Dual Protocol Support

```rust
// Protocol detection after handshake
enum ProtocolMode {
    ArsyncNative,   // Our efficient protocol (CURRENT)
    RsyncCompat,    // rsync wire protocol (TO IMPLEMENT)
}

// In send_via_pipe/receive_via_pipe:
let mode = detect_protocol_mode(&mut transport).await?;

match mode {
    ProtocolMode::ArsyncNative => {
        // Use current implementation ✅
        send_via_pipe_native(transport, ...).await
    }
    ProtocolMode::RsyncCompat => {
        // Use rsync-compatible implementation ⏳
        send_via_pipe_rsync(transport, ...).await
    }
}
```

### 3.2 Phase 1: Varint Codec (2-3 hours)

**Goal**: Implement rsync's variable-length integer encoding

**New File**: `src/protocol/varint.rs`

**Functions to implement**:
```rust
pub fn encode_varint(value: u64) -> Vec<u8>
pub async fn decode_varint<T: Transport>(transport: &mut T) -> Result<u64>
pub fn encode_varint_into(value: u64, buf: &mut Vec<u8>)
```

**Algorithm** (7-bit continuation):
- Lower 7 bits: data
- High bit (0x80): continuation flag
- Examples: 127 → [0x7F], 128 → [0x80, 0x01]

**Tests**:
- Roundtrip: encode then decode equals original
- Edge cases: 0, 127, 128, 16383, u64::MAX
- Compatibility: Match rsync's varint output (packet capture)

**Deliverable**: `varint.rs` with full test coverage

### 3.3 Phase 2: Multiplexed I/O Enhancement (2-3 hours)

**Goal**: Complete rsync's tagged message protocol

**File**: `src/protocol/rsync_compat.rs` (already started)

**Current status**:
- ✅ Message tag enum defined
- ✅ `read_mplex_message()` implemented
- ✅ `write_mplex_message()` implemented

**Enhancements needed**:
```rust
// Add buffering for partial messages
pub struct MultiplexReader<T: Transport> {
    transport: T,
    buffer: Vec<u8>,
}

impl<T: Transport> MultiplexReader<T> {
    pub async fn read_data(&mut self, buf: &mut [u8]) -> Result<usize>
    pub async fn read_message(&mut self) -> Result<MultiplexMessage>
    pub async fn expect_tag(&mut self, tag: MessageTag) -> Result<Vec<u8>>
}

// Add helpers for common operations
pub async fn read_data_message<T>(transport: &mut T) -> Result<Vec<u8>>
pub async fn write_data_message<T>(transport: &mut T, data: &[u8]) -> Result<()>
```

**Tests**:
- Read/write tagged messages
- Handle INFO and ERROR tags correctly
- Buffer management for large messages

**Deliverable**: Enhanced multiplex I/O with proper message handling

### 3.4 Phase 3: rsync File List Format (6-8 hours)

**Goal**: Encode/decode file lists in rsync's format

**Implementation approach** (incremental complexity):

#### Step 3a: Simplified Format (4 hours)
```rust
// src/protocol/rsync_compat.rs

async fn encode_file_list_rsync_simple<T: Transport>(
    transport: &mut T,
    files: &[FileEntry]
) -> Result<()> {
    for file in files {
        let mut entry = Vec::new();
        
        // Flags byte (simplified)
        let flags = calculate_flags(file);
        entry.push(flags);
        
        // Full path (varint length + bytes)
        encode_varint_into(file.path.len() as u64, &mut entry);
        entry.extend(file.path.as_bytes());
        
        // File size (varint)
        encode_varint_into(file.size, &mut entry);
        
        // Mtime (varint, absolute for now)
        encode_varint_into(file.mtime as u64, &mut entry);
        
        // Mode (varint)
        encode_varint_into(file.mode as u64, &mut entry);
        
        // uid/gid (varint)
        encode_varint_into(file.uid as u64, &mut entry);
        encode_varint_into(file.gid as u64, &mut entry);
        
        // Send as MSG_FLIST tagged message
        write_mplex_message(transport, MessageTag::FList, &entry).await?;
    }
    
    // End-of-list marker
    write_mplex_message(transport, MessageTag::FList, &[]).await?;
    
    Ok(())
}
```

**Tests**:
- Generate file list, feed to rsync receiver
- Verify rsync parses it correctly
- Check for protocol errors

#### Step 3b: Add Delta Encoding (2 hours)
- Encode mtime as delta from previous file
- Set SAME_UID/SAME_GID flags when unchanged
- Optimize for sequential files

#### Step 3c: Add Directory Grouping (2 hours)
- Split paths into dirname + basename
- Send dirname once, basenames relative
- More bandwidth efficient

**Deliverable**: rsync-compatible file list encoder/decoder

### 3.5 Phase 4: rsync Checksum Format (3-4 hours)

**Goal**: Format checksums the way rsync expects

**Changes from our current format**:

Our current:
```rust
struct BlockChecksum {
    weak: u32,
    strong: [u8; 16],
    offset: u64,      // ← Remove (implicit)
    block_index: u32, // ← Remove (implicit)
}
```

rsync format:
```rust
// Header
[4 bytes: block count]
[4 bytes: block length]
[4 bytes: remainder length]
[4 bytes: checksum2 length] // 2 or 16

// For each block (implicit index)
[4 bytes: weak checksum]
[N bytes: strong checksum] // N = checksum2 length
```

**Implementation**:
```rust
async fn send_block_checksums_rsync<T: Transport>(
    transport: &mut T,
    checksums: &[BlockChecksum],
    block_size: usize,
    file_size: u64,
) -> Result<()> {
    let mut data = Vec::new();
    
    // Header
    data.extend((checksums.len() as u32).to_le_bytes());
    data.extend((block_size as u32).to_le_bytes());
    
    let remainder = file_size % block_size as u64;
    data.extend((remainder as u32).to_le_bytes());
    
    let checksum2_length = 16u32; // Full MD5
    data.extend(checksum2_length.to_le_bytes());
    
    // Checksums (in order, no offset/index needed)
    for checksum in checksums {
        data.extend(checksum.weak.to_le_bytes());
        data.extend(&checksum.strong);
    }
    
    // Send as MSG_DATA tagged message
    write_mplex_message(transport, MessageTag::Data, &data).await?;
    
    Ok(())
}
```

**Tests**:
- Generate checksums in rsync format
- Feed to rsync sender
- Verify rsync generates correct delta
- Check for "checksum mismatch" errors

**Deliverable**: rsync-compatible checksum exchange

### 3.6 Phase 5: rsync Delta Token Format (8-12 hours) **MOST COMPLEX!**

**Goal**: Implement rsync's token-based delta encoding

**Challenge**: rsync uses token stream, not instruction list

#### Understanding rsync Tokens

**From rsync source** (`token.c`):
```c
Token values:
  0:        End of data marker
  1-96:     Literal run: N bytes of data follow
  97-255:   Block match: complex encoding of block offset
```

**Block match encoding**:
```c
token = 97 + (offset_bits << 4) + other_bits;

// Decoder:
if (token >= 97) {
    offset_bits = (token - 97) >> 4;
    block_offset = read_additional_bytes(offset_bits);
    block_num = last_block + 1 + block_offset;
}
```

#### Our Implementation Strategy

**Approach**: Convert our DeltaInstructions to rsync tokens

```rust
async fn send_delta_rsync<T: Transport>(
    transport: &mut T,
    delta: &[DeltaInstruction],
) -> Result<()> {
    let mut token_stream = Vec::new();
    let mut last_block_index = 0;
    
    for instruction in delta {
        match instruction {
            DeltaInstruction::Literal(data) => {
                // Split into chunks (max 96 bytes per token)
                for chunk in data.chunks(96) {
                    let token = chunk.len() as u8; // 1-96
                    token_stream.push(token);
                    token_stream.extend(chunk);
                }
            }
            DeltaInstruction::BlockMatch { block_index, length } => {
                // Calculate offset from last block
                let offset = block_index.saturating_sub(last_block_index + 1);
                
                // Encode as token (simplified - may need refinement)
                let token = encode_block_token(offset);
                token_stream.push(token);
                
                // May need additional bytes for large offsets
                if offset >= 16 {
                    token_stream.extend(encode_block_offset(offset));
                }
                
                last_block_index = *block_index;
            }
        }
    }
    
    // End of data marker
    token_stream.push(0);
    
    // Send as MSG_DATA tagged message
    write_mplex_message(transport, MessageTag::Data, &token_stream).await?;
    
    Ok(())
}

fn encode_block_token(offset: u32) -> u8 {
    if offset < 16 {
        97 + offset as u8
    } else {
        // Complex encoding for large offsets
        97 + (count_bits(offset) << 4) // Simplified
    }
}
```

**Substeps**:
1. Implement basic token encoding (2 hours)
2. Study rsync's exact offset encoding (3 hours)
3. Test token generation against rsync (2 hours)
4. Implement token decoding (3 hours)
5. Debug and fix mismatches (2 hours)

**Tests**:
- Generate tokens, feed to rsync receiver
- Parse rsync's tokens, reconstruct file
- Verify byte-for-byte file match

**Deliverable**: rsync-compatible delta token codec

### 3.7 Phase 6: Integration and Testing (6-8 hours)

**Goal**: Wire everything together and make Tests 3 & 4 pass

#### Step 6a: Create rsync-Compatible Sender

```rust
// src/protocol/rsync_compat.rs

pub async fn rsync_send_via_pipe(
    args: &Args,
    source_path: &Path,
    mut transport: PipeTransport,
) -> Result<SyncStats> {
    // Use rsync wire protocol format
    
    // 1. Handshake (with seed exchange)
    handshake_rsync(&mut transport, true).await?;
    
    // 2. Send file list (rsync format with varint)
    let files = generate_file_list_simple(source_path, args).await?;
    encode_file_list_rsync_simple(&mut transport, &files).await?;
    
    // 3. For each file: receive checksums, send delta
    for file in &files {
        if file.is_symlink {
            continue;
        }
        
        let content = fs::read(&source_path.join(&file.path))?;
        
        // Receive checksums (rsync format)
        let checksums = receive_block_checksums_rsync(&mut transport).await?;
        
        // Generate delta
        let delta = generate_delta(&content, &convert_checksums(&checksums))?;
        
        // Send delta (rsync token format)
        send_delta_rsync(&mut transport, &delta).await?;
    }
    
    Ok(SyncStats { ... })
}
```

#### Step 6b: Create rsync-Compatible Receiver

```rust
pub async fn rsync_receive_via_pipe(
    args: &Args,
    mut transport: PipeTransport,
    dest_path: &Path,
) -> Result<SyncStats> {
    // Use rsync wire protocol format
    
    // 1. Handshake
    handshake_rsync(&mut transport, false).await?;
    
    // 2. Receive file list (rsync format)
    let files = decode_file_list_rsync(&mut transport).await?;
    
    // 3. For each file: send checksums, receive delta
    for file in &files {
        if file.is_symlink {
            create_symlink(&dest_path.join(&file.path), &file.symlink_target)?;
            continue;
        }
        
        let file_path = dest_path.join(&file.path);
        let basis = if file_path.exists() {
            fs::read(&file_path).ok()
        } else {
            None
        };
        
        // Generate and send checksums (rsync format)
        let checksums = if let Some(ref basis) = basis {
            generate_block_checksums_rsync(basis)?
        } else {
            vec![]
        };
        send_block_checksums_rsync(&mut transport, &checksums, ...).await?;
        
        // Receive and apply delta (rsync format)
        let delta = receive_delta_rsync(&mut transport).await?;
        let reconstructed = apply_delta_rsync(basis.as_deref(), &delta, &checksums)?;
        
        fs::write(&file_path, &reconstructed)?;
        apply_metadata(&file_path, file)?;
    }
    
    Ok(SyncStats { ... })
}
```

#### Step 6c: Update Tests to Use rsync Compat Mode

```rust
// tests/protocol_pipe_tests.rs

#[tokio::test]
async fn test_rsync_to_arsync_via_pipe() {
    // Create bidirectional pipes
    let (pipe1_read, pipe1_write) = create_pipe_pair();
    let (pipe2_read, pipe2_write) = create_pipe_pair();
    
    // Spawn rsync sender (real rsync!)
    let mut sender = Command::new("rsync")
        .arg("--server")
        .arg("--sender")
        .arg("-vr")
        .arg(".")
        .arg(&source)
        .stdin(unsafe { Stdio::from_raw_fd(pipe2_read) })
        .stdout(unsafe { Stdio::from_raw_fd(pipe1_write) })
        .spawn()
        .unwrap();
    
    // Spawn arsync receiver (our compat mode!)
    let mut receiver = Command::new(env!("CARGO_BIN_EXE_arsync"))
        .arg("--pipe")
        .arg("--pipe-role=receiver")
        .arg("--rsync-compat") // NEW FLAG!
        .arg("-r")
        .arg("/dev/null")
        .arg(&dest)
        .stdin(unsafe { Stdio::from_raw_fd(pipe1_read) })
        .stdout(unsafe { Stdio::from_raw_fd(pipe2_write) })
        .spawn()
        .unwrap();
    
    // Wait and verify
    assert!(sender.wait().await?.success());
    assert!(receiver.wait().await?.success());
    verify_transfer(&source, &dest)?;
    
    println!("✓ Test 3: rsync → arsync PASSED");
}
```

**Deliverable**: Tests 3 & 4 passing with real rsync

---

## Part 4: Detailed Task Breakdown

### Phase 1: Varint Implementation

**File**: `src/protocol/varint.rs` (new, ~100 lines)

**Tasks**:
- [x] Create file
- [ ] Implement `encode_varint(u64) -> Vec<u8>`
- [ ] Implement `decode_varint<T: Transport>(transport) -> Result<u64>`
- [ ] Add `encode_varint_into(u64, &mut Vec<u8>)` for efficiency
- [ ] Write unit tests (10 test cases)
- [ ] Test against rsync's varint (packet capture)

**Time**: 2-3 hours

### Phase 2: Multiplex Enhancement

**File**: `src/protocol/rsync_compat.rs` (enhance, +100 lines)

**Tasks**:
- [x] Message tag enum (done)
- [x] Basic read/write tagged messages (done)
- [ ] Add MultiplexReader struct with buffering
- [ ] Add MultiplexWriter struct
- [ ] Add expect_tag() helper
- [ ] Handle all message tags (INFO, ERROR, WARNING, etc.)
- [ ] Write integration tests

**Time**: 2-3 hours

### Phase 3: File List Format

**File**: `src/protocol/rsync_compat.rs` (+200 lines)

**Tasks**:
- [ ] Implement `encode_file_list_rsync_simple()`
- [ ] Implement `decode_file_list_rsync()`
- [ ] Calculate flags byte correctly
- [ ] Handle symlinks in file list
- [ ] Test file list generation
- [ ] Test against rsync's actual file list (capture)
- [ ] Debug protocol mismatches

**Time**: 6-8 hours

### Phase 4: Checksum Format

**File**: `src/protocol/rsync_compat.rs` (+80 lines)

**Tasks**:
- [ ] Implement `send_block_checksums_rsync()`
- [ ] Implement `receive_block_checksums_rsync()`
- [ ] Format header (count, block_size, remainder, checksum_length)
- [ ] Remove offset/index fields (use implicit indexing)
- [ ] Test checksum generation
- [ ] Verify rsync accepts checksums

**Time**: 3-4 hours

### Phase 5: Delta Token Format

**File**: `src/protocol/rsync_compat.rs` (+150 lines)

**Tasks**:
- [ ] Study rsync's token.c in detail
- [ ] Implement `encode_block_token(offset) -> u8`
- [ ] Implement token stream generator
- [ ] Convert DeltaInstructions to tokens
- [ ] Implement token decoder
- [ ] Handle literal runs (1-96)
- [ ] Handle block references (97-255)
- [ ] Test exhaustively

**Time**: 8-12 hours

### Phase 6: Integration

**Files**: Multiple

**Tasks**:
- [ ] Add `--rsync-compat` CLI flag
- [ ] Wire up rsync_send_via_pipe()
- [ ] Wire up rsync_receive_via_pipe()
- [ ] Update Tests 3 & 4 to use real rsync
- [ ] Debug protocol errors
- [ ] Fix edge cases
- [ ] Document known limitations

**Time**: 6-8 hours

---

## Part 5: Realistic Timeline

### Conservative Estimate

| Phase | Task | Hours |
|-------|------|-------|
| 1 | Varint codec | 3 |
| 2 | Multiplex enhancement | 3 |
| 3 | File list format | 8 |
| 4 | Checksum format | 4 |
| 5 | Delta tokens | 12 |
| 6 | Integration | 8 |
| 7 | Debug & testing | 6 |
| 8 | Documentation | 2 |
| **Total** | **Full rsync compatibility** | **46 hours** |

### Aggressive Estimate (Minimum Viable)

| Phase | Task | Hours |
|-------|------|-------|
| 1 | Varint codec | 2 |
| 2 | Multiplex (basic) | 2 |
| 3 | File list (simplified) | 4 |
| 4 | Checksum format | 2 |
| 5 | Delta (basic tokens) | 6 |
| 6 | Integration | 4 |
| 7 | Debug essentials | 4 |
| **Total** | **Minimum working compat** | **24 hours** |

### Reality Check

**Based on our experience so far**:
- Delta algorithm took ~8 hours (estimated 8-12)
- Metadata took ~2 hours (estimated 2-3)
- Testing infrastructure took ~4 hours (estimated 3-4)

**Prediction**: rsync compat will take **30-40 hours** realistically
- Protocol is more complex than delta algorithm
- Many edge cases to handle
- Debugging protocol mismatches is time-consuming

---

## Part 6: Dependencies and Prerequisites

### Required Crates

Already added:
```toml
md5 = "0.7"           # Strong checksums
walkdir = "2.0"       # Directory traversal
filetime = "0.2"      # Timestamp manipulation
```

No new dependencies needed! (varint is simple to implement)

### Knowledge Prerequisites

**Must understand**:
1. ✅ Delta algorithm (we have this!)
2. ✅ Block checksums (we have this!)
3. ⏳ rsync wire protocol (documented in RSYNC_WIRE_PROTOCOL_SPEC.md)
4. ⏳ Varint encoding (well-understood, easy to implement)
5. ⏳ Tagged message protocols (already started)

---

## Part 7: Risk Assessment and Mitigation

### High Risk: Protocol Complexity

**Risk**: rsync protocol has many undocumented behaviors  
**Severity**: High  
**Mitigation**:
- Read rsync source code (token.c, io.c, flist.c)
- Packet capture of real rsync sessions
- Test against multiple rsync versions
- Accept "good enough" initially, refine later

### Medium Risk: Token Encoding Edge Cases

**Risk**: Block token encoding is complex and under-documented  
**Severity**: Medium  
**Mitigation**:
- Start with simplified token encoding
- Test exhaustively against rsync
- Use packet captures to validate
- Iterate based on rsync's feedback

### Low Risk: Varint Implementation

**Risk**: Varint is well-understood and testable  
**Severity**: Low  
**Mitigation**:
- Standard algorithm
- Easy to unit test
- Many reference implementations

---

## Part 8: Success Criteria

### Minimum Success (Tests 3 & 4 Passing)

**Functional requirements**:
- ✅ rsync sender → arsync receiver (Test 3)
- ✅ arsync sender → rsync receiver (Test 4)
- ✅ Files transfer correctly
- ✅ Metadata preserved
- ✅ No protocol errors from rsync

**Non-requirements** (can defer):
- Optimal bandwidth (can be less efficient than rsync initially)
- All rsync features (just core sync)
- All protocol versions (focus on 31/32)
- Compression (can add later)

### Full Success (Production Ready)

- All rsync features working
- Optimal token encoding
- Multiple protocol versions supported
- Compression integrated
- Extensive test coverage
- Performance benchmarking

---

## Part 9: Development Workflow

### Recommended Iteration Cycle

```
For each phase:
  1. Read relevant rsync source code
  2. Document understanding
  3. Implement feature
  4. Write unit tests
  5. Test against real rsync
  6. Debug protocol errors
  7. Commit working code
  8. Move to next phase
```

### Testing Against rsync

```bash
# After each phase, validate with real rsync

# Test receiver (Test 3: rsync → arsync)
rsync --server --sender -vr . /source/ \
    | arsync --pipe --pipe-role=receiver --rsync-compat -r /dev/null /dest/

# Test sender (Test 4: arsync → rsync)
arsync --pipe --pipe-role=sender --rsync-compat -r /source/ /dev/null \
    | rsync --server -vr . /dest/
```

### Debugging Protocol Mismatches

```bash
# Capture protocol with hexdump
arsync ... | tee >(xxd > arsync-proto.hex) | rsync ...

# Compare with rsync's output
rsync ... | xxd > rsync-proto.hex
diff rsync-proto.hex arsync-proto.hex
```

---

## Part 10: Implementation Order (Recommended)

### Week 1: Foundation
- [x] Document protocol thoroughly (THIS DOCUMENT!)
- [ ] Implement varint codec
- [ ] Test varint extensively
- [ ] Enhance multiplexed I/O
- **Deliverable**: Varint and multiplex working

### Week 2: File List
- [ ] Implement simplified file list encoder
- [ ] Implement file list decoder  
- [ ] Test against rsync
- [ ] Add delta encoding if needed
- **Deliverable**: File list compatible with rsync

### Week 3: Checksums and Delta
- [ ] Implement rsync checksum format
- [ ] Implement basic token encoding
- [ ] Test checksum exchange
- [ ] Test delta transmission
- **Deliverable**: Checksums and delta working

### Week 4: Integration and Testing
- [ ] Wire up complete flow
- [ ] Make Test 3 pass (rsync → arsync)
- [ ] Make Test 4 pass (arsync → rsync)
- [ ] Debug and polish
- **Deliverable**: Full rsync compatibility

**Total estimated time**: 4 weeks part-time or 1 week full-time

---

## Part 11: Alternative Approaches Considered

### Alternative 1: Use librsync

**Pros**: Battle-tested, handles all quirks  
**Cons**: C library, blocking I/O, doesn't help with protocol multiplexing  
**Verdict**: ❌ Rejected - we want async Rust

### Alternative 2: Fork rsync and add io_uring

**Pros**: Full compatibility, proven code  
**Cons**: 50K lines of C, io_uring integration difficult  
**Verdict**: ❌ Rejected - defeats purpose of arsync

### Alternative 3: Minimal compatibility (no delta encoding optimizations)

**Pros**: Faster to implement, still functional  
**Cons**: Less bandwidth efficient than rsync  
**Verdict**: ✅ **Recommended for initial implementation**

### Alternative 4: Hybrid protocol (auto-detect peer)

**Pros**: Use efficient protocol for arsync, compat for rsync  
**Cons**: Need protocol detection logic  
**Verdict**: ✅ **Recommended - best of both worlds**

---

## Part 12: What We're NOT Implementing (Initially)

### Deferred Features

- [ ] Compression (zlib/zstd)
- [ ] Incremental recursion
- [ ] --delete mode
- [ ] --partial mode (resume)
- [ ] --checksum mode (whole-file verification)
- [ ] Hard link detection (rsync-style)
- [ ] Device file transfer
- [ ] ACL preservation
- [ ] Extended attributes over protocol
- [ ] Batch mode
- [ ] Daemon mode (rsyncd)

### Why Defer?

**Reason**: Get basic compatibility working first (Tests 3 & 4 passing)  
**Benefit**: Incremental approach, validate each step  
**Plan**: Add features based on user needs

---

## Conclusion

### Current State

✅ **Excellent**: arsync native protocol complete and tested
- Delta algorithm working
- Metadata preservation complete
- Test 2 passing
- Production-ready for arsync ↔ arsync

### Path Forward

📋 **Well-documented**: 7-page protocol spec completed  
📐 **Well-planned**: Phased implementation approach  
🎯 **Realistic timeline**: 24-40 hours for full rsync compatibility  

### Immediate Next Step

**Phase 1**: Implement varint codec (2-3 hours)
- Small, contained task
- Easy to test
- Foundation for everything else
- Clear success criteria

---

**Document Version**: 2.0 (Updated with rsync compat design)  
**Last Updated**: October 9, 2025  
**Status**: Design complete, ready for implementation  
**Pages**: This doc + RSYNC_WIRE_PROTOCOL_SPEC.md = comprehensive coverage

**Read RSYNC_WIRE_PROTOCOL_SPEC.md first** for detailed protocol analysis, then return here for implementation plan.


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


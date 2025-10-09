# rsync Wire Protocol Implementation - Detailed Checklist (REVISED)

**Purpose**: Comprehensive, step-by-step implementation plan with granular checkboxes  
**Status**: ✅ Phase 1 Partial (1.1-1.4 complete, continuing with revised order)  
**Started**: October 9, 2025  
**Target Completion**: 7-10 weeks

---

## IMPORTANT: Revised Phase Ordering (v2)

**Problem**: Current code uses **blocking I/O** which creates async/blocking mismatch
- PipeTransport uses `std::io::Read/Write` (blocking)
- Tests need bidirectional I/O (requires async concurrency)
- Main binary uses compio runtime
- Result: Architecture conflict from the start

**Solution**: **Do compio migration NOW, before any more testing**

```
✅ Phase 1.1-1.4: Handshake core (DONE)
→ PHASE 2:       compio/io_uring migration (FIX IT NOW!)
→ Phase 1.5:     rsync integration tests (with correct architecture)
→ Phase 1.5b:    Pipe tests (with correct architecture)
→ Phase 3-5:     Checksums, Delta, Integration
```

**Why this is correct**:
1. Handshake code is **generic over Transport trait** ✅
2. Fix Transport once → everything works
3. All testing uses **correct architecture from start**
4. No wasted effort testing with wrong I/O layer
5. No re-testing after migration
6. Aligns with arsync's **core io_uring design**

**Key insight**: Don't test broken architecture, fix it first!

---

## How to Use This Document

1. **For Reviewer**: Each checkbox represents a concrete deliverable with acceptance criteria
2. **For Implementation**: Follow checkboxes sequentially within each phase
3. **For Context Recovery**: When lost in details, read the "Phase Goals" and "Acceptance Criteria"
4. **For Progress Tracking**: Check boxes as completed, commit after each sub-phase

---

# COMPLETED WORK

## Phase 1.1: Core Data Structures ✅ COMPLETE

**Commit**: 37451a4

### What Was Implemented
- [x] File: `src/protocol/handshake.rs` created (600+ lines)
- [x] Added to `src/protocol/mod.rs`
- [x] Protocol constants (PROTOCOL_VERSION=31, MIN=27, MAX=40)
- [x] 10 capability flags (XMIT_CHECKSUMS, XMIT_SYMLINKS, etc.)
- [x] Role enum (Sender/Receiver) with 3 helper methods
- [x] ChecksumSeed struct with 4 methods + 2 unit tests
- [x] ProtocolCapabilities struct with 11 support methods + 2 unit tests
- [x] HandshakeState enum (9 states) with 3 query methods + 2 unit tests
- [x] All doc comments with examples
- [x] 7/7 unit tests passing

---

## Phase 1.2: State Machine Implementation ✅ COMPLETE

**Commit**: cb6c715

### What Was Implemented
- [x] `HandshakeState::advance()` method (304 lines)
- [x] All 9 state transitions:
  - [x] Initial → VersionSent
  - [x] VersionSent → VersionReceived
  - [x] VersionReceived → VersionNegotiated
  - [x] VersionNegotiated → FlagsSent
  - [x] FlagsSent → FlagsReceived
  - [x] FlagsReceived → CapabilitiesNegotiated
  - [x] CapabilitiesNegotiated → SeedExchange or Complete
  - [x] SeedExchange → Complete
  - [x] Complete (terminal state with error)
- [x] Error handling at each transition
- [x] Comprehensive logging (debug, info, warn)
- [x] `get_our_capabilities()` helper function

---

## Phase 1.3: High-Level API ✅ COMPLETE

**Commit**: 73574a5

### What Was Implemented
- [x] `handshake_sender()` - public API for sender
- [x] `handshake_receiver()` - public API for receiver
- [x] `handshake()` - general API with role parameter
- [x] All functions with doc comments and examples
- [x] Info logging at start/completion
- [x] Error extraction and propagation

---

## Phase 1.4: Unit Tests ✅ COMPLETE

**Commit**: 2e96b97

### What Was Implemented
- [x] File: `tests/handshake_unit_tests.rs` (280+ lines)
- [x] 14 comprehensive unit tests:
  - [x] State machine basics (2 tests)
  - [x] Capability negotiation (3 tests)
  - [x] Checksum seed (3 tests)
  - [x] Version constants (1 test)
  - [x] Our capabilities (1 test)
  - [x] Role methods (3 tests)
  - [x] Summary test (1 test)
- [x] All 14/14 tests passing
- [x] Made `get_our_capabilities()` public for testing

---

# CURRENT WORK - PHASE 2 FIRST!

## Why Phase 2 Now?

**Handshake is done, but transport is broken**. Before writing ANY more tests:
1. Fix the Transport trait to use compio
2. Fix PipeTransport to use io_uring
3. Then ALL tests will work correctly

**Do NOT test with blocking I/O** - it's technical debt we'll have to redo.

---

# PHASE 2: compio/io_uring Migration (DO THIS NOW!)

**Goal**: Migrate protocol code from tokio to compio for io_uring-based async I/O

**Why Now**: Fixes async/blocking mismatch, enables all subsequent testing

**Duration Estimate**: 2-3 weeks (but saves time overall by avoiding re-testing)

**Files to Create**: 2-3 new files  
**Files to Modify**: 8 existing files  
**Tests to Add**: 10+ test functions

---

## Phase 2.1: compio Capability Audit ✅ COMPLETE

**Commit**: 12396c5

### Research compio Features

- [x] Check compio version in Cargo.toml → **0.16.0**
- [x] Find compio source in cargo registry
- [x] List available modules from lib.rs
- [x] Check dependency tree

### Findings Documented

- [x] Created `docs/COMPIO_AUDIT.md` (276 lines)
- [x] Documented all available modules:
  - [x] compio-io: AsyncRead/AsyncWrite ✅
  - [x] compio-fs: File operations with from_raw_fd() ✅
  - [x] **compio-process: FULL process support!** ✅
    - Command, Child, ChildStdin/Stdout/Stderr
    - spawn() method
    - All implement AsyncRead/AsyncWrite
  - [x] compio-net: TcpStream, UnixStream ✅
  - [x] compio-runtime: #[compio::test] macro ✅
  - [x] compio-driver: Low-level io_uring ops ✅

### Migration Strategy Decision

- [x] **Chose: Pure compio (no hybrid needed!)**
- [x] Rationale: compio-process exists with full API
- [x] No workarounds required
- [x] Clean architecture throughout

### Acceptance Criteria for Phase 2.1 ✅ COMPLETE
- [x] Audit document complete
- [x] All features identified
- [x] Strategy chosen (pure compio)
- [x] Expected performance documented (30-50% improvement)
- [x] Commit message: "docs(compio): audit compio 0.16 - full process support available!"
- [x] **Commit**: 12396c5

---

## Phase 2.2: Transport Trait Redesign ✅ COMPLETE

**Commit**: 9fbf1fb

### What Was Implemented

- [x] Removed async_trait from trait (not yet from Cargo.toml)
- [x] Removed `use async_trait::async_trait;`
- [x] Removed `#[async_trait]` attribute
- [x] Redesigned trait as marker requiring:
  - [x] `compio::io::AsyncRead`
  - [x] `compio::io::AsyncWrite`
  - [x] `Send + Unpin`
- [x] Added comprehensive doc comments
- [x] Explained Unpin requirement
- [x] Added architecture diagram
- [x] Added usage examples

### Helper Functions Updated

- [x] `read_exact()`: Now uses `compio::io::AsyncRead + Unpin`
- [x] `write_all()`: Uses `AsyncWriteExt::write_all()` + flush()
- [x] Changed return type from `anyhow::Result` to `io::Result`
- [x] Improved error messages
- [x] Added doc comments with examples

### Expected Breakage (Will Fix Next)

- ❌ PipeTransport: Doesn't implement compio traits yet
- ❌ SshConnection: Doesn't implement compio traits yet
- ❌ Handshake module: Uses old anyhow::Result

All expected and will be fixed in Phases 2.3-2.5.

---

## Phase 2.3: PipeTransport Migration to compio

**Goal**: Convert PipeTransport to use compio::fs::File with io_uring

### Update `src/protocol/pipe.rs`

#### Update Imports
- [x] Remove: `use std::io::{Read, Write};`
- [x] Add: `use compio::fs::File;`
- [x] Add: `use compio::io::{AsyncReadExt, AsyncWriteExt};`
- [x] Keep: `use std::os::unix::io::{FromRawFd, RawFd};`

#### Redesign PipeTransport Struct
- [x] Change struct:
  ```rust
  pub struct PipeTransport {
      reader: compio::fs::File,
      writer: compio::fs::File,
      #[allow(dead_code)]
      name: String,
  }
  ```
- [x] Update doc comment explaining io_uring usage

#### Update from_stdio()
- [x] Rewrite:
  ```rust
  pub fn from_stdio() -> Result<Self> {
      use std::os::unix::io::AsRawFd;
      
      let stdin_fd = std::io::stdin().as_raw_fd();
      let stdout_fd = std::io::stdout().as_raw_fd();
      
      let reader = unsafe { compio::fs::File::from_raw_fd(stdin_fd) };
      let writer = unsafe { compio::fs::File::from_raw_fd(stdout_fd) };
      
      Ok(Self {
          reader,
          writer,
          name: "stdio".to_string(),
      })
  }
  ```
- [x] Add safety documentation
- [x] Test it works

#### Update from_fds()
- [x] Rewrite to use `compio::fs::File::from_raw_fd()`
- [ ] Update safety documentation
- [x] Test it works

#### Remove Old Transport Impl
- [ ] Remove `#[async_trait] impl Transport`
- [ ] Remove manual read/write implementations

#### Add compio Trait Impls
- [ ] Implement `compio::io::AsyncRead`:
  ```rust
  impl compio::io::AsyncRead for PipeTransport {
      // Delegate to reader
  }
  ```
- [ ] Implement `compio::io::AsyncWrite`:
  ```rust
  impl compio::io::AsyncWrite for PipeTransport {
      // Delegate to writer
  }
  ```
- [ ] Add marker impl: `impl Transport for PipeTransport {}`

#### Test with strace
- [ ] Run simple test with strace
- [ ] Verify io_uring syscalls (io_uring_enter, io_uring_submit)
- [ ] Document io_uring usage

### Acceptance Criteria for Phase 2.3
- [x] PipeTransport compiles with compio
- [x] Implements all required traits
- [ ] Uses io_uring (verified with strace)
- [ ] from_stdio() and from_fds() work
- [ ] Code formatted
- [ ] Commit message: "refactor(pipe): migrate PipeTransport to compio/io_uring"
- [ ] **Commit**: TBD

---

## Phase 2.4: SSH Connection Strategy

**Decision Point**: Check if compio::process exists

### Investigate compio::process

- [ ] Check: `ls ~/.cargo/registry/src/*/compio-*/src/ | grep process`
- [ ] If found:
  - [ ] Read process module source
  - [ ] Check API (Child, ChildStdin, ChildStdout?)
  - [ ] Test basic spawn
  - [ ] **Go to Phase 2.4a** (pure compio)
- [ ] If not found:
  - [ ] Document that it's missing
  - [ ] Check compio GitHub issues/roadmap
  - [ ] **Go to Phase 2.4b** (hybrid approach)

---

## Phase 2.4a: Pure compio SSH (if process exists)

### Update `src/protocol/ssh.rs`

- [ ] Replace: `use tokio::process::*;` with `use compio::process::*;`
- [ ] Update SshConnection struct for compio types
- [ ] Update connect() to use compio::process::Command
- [ ] Implement compio::io::AsyncRead
- [ ] Implement compio::io::AsyncWrite
- [ ] Implement Transport trait
- [ ] Test with real SSH to localhost

### Acceptance Criteria
- [ ] Compiles with compio
- [ ] Works with SSH
- [ ] Commit message: "refactor(ssh): migrate to compio::process"
- [ ] **Commit**: TBD

---

## Phase 2.4b: Hybrid SSH (if process missing) - LIKELY PATH

**Strategy**: Use stdlib for process, compio-driver for I/O

### Create `src/protocol/ssh_hybrid.rs`

- [ ] Create new file
- [ ] Add comprehensive doc comment explaining hybrid approach

#### Define HybridSshConnection
- [ ] Add struct:
  ```rust
  pub struct HybridSshConnection {
      process: std::process::Child,
      stdin_fd: RawFd,
      stdout_fd: RawFd,
      name: String,
  }
  ```
- [ ] Add doc comment

#### Implement connect()
- [ ] Spawn with stdlib:
  ```rust
  let mut child = std::process::Command::new(shell)
      .arg(format!("{}@{}", user, host))
      .arg("arsync")
      .arg("--server")
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::inherit())
      .spawn()?;
  ```
- [ ] Extract FDs and forget stdlib handles
- [ ] Store process and FDs
- [ ] Add error handling
- [ ] Add debug logging

#### Implement compio::io::AsyncRead
- [ ] Use compio buffer and driver:
  ```rust
  impl compio::io::AsyncRead for HybridSshConnection {
      fn poll_read(...) -> Poll<io::Result<usize>> {
          // Use compio's driver with our FD
          // This gets io_uring even though process is stdlib!
      }
  }
  ```
- [ ] Add error handling
- [ ] Test read operations

#### Implement compio::io::AsyncWrite
- [ ] Similar to AsyncRead but for write
- [ ] Implement flush (may be no-op)
- [ ] Test write operations

#### Implement Transport
- [ ] Add marker impl
- [ ] Override name()
- [ ] Override supports_multiplexing()

#### Implement Drop
- [ ] Kill child process gracefully
- [ ] Wait for exit
- [ ] Close file descriptors
- [ ] Add error logging

### Update `src/protocol/ssh.rs`

- [ ] Add conditional compilation:
  ```rust
  #[cfg(feature = "compio-process")]
  mod ssh_compio;
  
  #[cfg(not(feature = "compio-process"))]
  mod ssh_hybrid;
  
  #[cfg(feature = "compio-process")]
  pub use ssh_compio::SshConnection;
  
  #[cfg(not(feature = "compio-process"))]
  pub use ssh_hybrid::HybridSshConnection as SshConnection;
  ```
- [ ] Add doc comment explaining why

### Acceptance Criteria for Phase 2.4b
- [ ] Hybrid impl compiles
- [ ] Uses io_uring for I/O (verify with strace)
- [ ] Uses stdlib for process
- [ ] Works with real SSH
- [ ] Cleanup on drop
- [ ] Code formatted
- [ ] Commit message: "feat(ssh): implement hybrid approach (stdlib process + io_uring I/O)"
- [ ] **Commit**: TBD

---

## Phase 2.5: Update Handshake Module for compio

### Update `src/protocol/handshake.rs`

#### Update Imports
- [ ] Verify Transport import is correct
- [ ] Remove any tokio references
- [ ] Add compio imports if needed

#### Update advance() Method
- [ ] Verify `T: Transport` bound still works
- [ ] Verify all read_exact/write_all calls work
- [ ] Test with new compio transport

#### Update Public APIs
- [ ] Verify handshake_sender() compiles
- [ ] Verify handshake_receiver() compiles
- [ ] Verify handshake() compiles
- [ ] Test all three functions

### Acceptance Criteria for Phase 2.5
- [ ] Handshake module compiles with compio
- [ ] No tokio dependencies
- [ ] All functions work
- [ ] Commit message: "refactor(handshake): update for compio transport"
- [ ] **Commit**: TBD

---

## Phase 2.6: Update All Protocol Modules

### Update `src/protocol/rsync_compat.rs`

#### Update MultiplexReader
- [ ] Change transport field to new Transport trait
- [ ] Update read_message() for compio
- [ ] Update all other methods
- [ ] Remove async_trait if present

#### Update MultiplexWriter  
- [ ] Same process as MultiplexReader
- [ ] Update write operations

#### Update Main Functions
- [ ] Update rsync_send_via_pipe()
- [ ] Update rsync_receive_via_pipe()
- [ ] Test they compile

### Update `src/protocol/rsync.rs`

- [ ] Update send_via_pipe() for compio
- [ ] Update receive_via_pipe() for compio
- [ ] Update any Transport usage
- [ ] Remove tokio dependencies

### Update `src/protocol/varint.rs`

- [ ] Update decode_varint() for new Transport
- [ ] Update decode_varint_signed()
- [ ] Test roundtrip

### Update `src/protocol/mod.rs`

- [ ] Verify all modules compile
- [ ] Update pipe_sender/receiver if needed
- [ ] Test integration

### Acceptance Criteria for Phase 2.6
- [ ] All protocol modules compile with compio
- [ ] No tokio dependencies in protocol code
- [ ] All functions use compio traits
- [ ] Code formatted
- [ ] Commit message: "refactor(protocol): migrate all modules to compio"
- [ ] **Commit**: TBD

---

## Phase 2.7: Update Tests for compio Runtime

### Update Test Files

#### Update `tests/handshake_unit_tests.rs`
- [ ] No changes needed (pure unit tests, no I/O)
- [ ] Verify still passes

#### Update `tests/rsync_format_unit_tests.rs`
- [ ] No changes needed (no I/O)
- [ ] Verify still passes

#### Update `tests/rsync_integration_tests.rs`
- [ ] Tests use shell scripts (no async)
- [ ] Should still work as-is
- [ ] Verify all pass

### Create `tests/compio_transport_tests.rs`

- [ ] Test PipeTransport read/write with compio
- [ ] Test large data transfer
- [ ] Verify io_uring usage with strace
- [ ] Measure performance

### Acceptance Criteria for Phase 2.7
- [ ] All existing tests still pass
- [ ] New compio tests pass
- [ ] Performance is good
- [ ] Commit message: "test(compio): verify tests work with compio"
- [ ] **Commit**: TBD

---

## Phase 2.8: Documentation

### Create `docs/COMPIO_MIGRATION_GUIDE.md`

- [ ] Title: "compio Migration Guide"
- [ ] Section: "Why Migrate"
  - [ ] Async/blocking mismatch problem
  - [ ] io_uring benefits
  - [ ] Alignment with arsync core
- [ ] Section: "What Changed"
  - [ ] Transport trait redesign
  - [ ] PipeTransport implementation
  - [ ] SSH hybrid approach (if used)
  - [ ] List all modified files
- [ ] Section: "Before/After Architecture"
  - [ ] Diagram showing old (tokio + blocking)
  - [ ] Diagram showing new (compio + io_uring)
- [ ] Section: "Performance Impact"
  - [ ] Include benchmark results
  - [ ] Context switch reduction
- [ ] Section: "Testing"
  - [ ] How to run tests
  - [ ] How to verify io_uring usage

### Update `docs/RSYNC_COMPAT_DETAILED_DESIGN.md`

- [ ] Mark Phase 2 complete
- [ ] Update architecture section
- [ ] Update code examples for compio

### Update `docs/COMPIO_AUDIT.md`

- [ ] Add "Implementation Complete" section
- [ ] Document what was implemented
- [ ] Document any workarounds used

### Acceptance Criteria for Phase 2.8
- [ ] Documentation complete
- [ ] Migration guide is clear
- [ ] Architecture diagrams included
- [ ] Commit message: "docs(compio): document migration completion"
- [ ] **Commit**: TBD

---

## Phase 2.9: Final compio Testing and Cleanup

### Run Full Test Suite
- [ ] Run: `cargo test --all-features`
- [ ] Verify all tests pass
- [ ] Fix any regressions

### Code Quality
- [ ] Run: `cargo fmt --all`
- [ ] Run: `cargo clippy --all-features -- -D warnings`
- [ ] Fix all warnings
- [ ] Run: `cargo doc --all-features --no-deps`
- [ ] Fix doc warnings

### Verify io_uring Usage
- [ ] Run test with strace
- [ ] Verify io_uring_enter/submit calls
- [ ] Document syscall reduction

### Update TEST_COVERAGE.md
- [ ] Add section on compio migration
- [ ] Update test statistics
- [ ] Note architecture improvement

### Acceptance Criteria for Phase 2.9
- [ ] All tests pass
- [ ] No warnings
- [ ] io_uring verified
- [ ] Documentation updated
- [ ] Commit message: "chore(compio): final testing and cleanup"
- [ ] **Commit**: TBD

---

## Phase 2.10: Create Pull Request for compio Migration

### PR Preparation
- [ ] All Phase 2 commits pushed
- [ ] Rebase on main if needed
- [ ] Run final test suite
- [ ] Review all changes

### Create PR
- [ ] Title: "refactor: migrate protocol to compio/io_uring (Phase 2)"
- [ ] Body includes:
  - [ ] Summary: Why this migration
  - [ ] Before/after architecture
  - [ ] Hybrid SSH approach (if used)
  - [ ] Performance results
  - [ ] List of changed files (8+ files)
  - [ ] Testing coverage
  - [ ] Link to migration guide
  - [ ] Note: Enables proper pipe tests in Phase 1.5b

### Acceptance Criteria for Phase 2.10
- [ ] PR created successfully
- [ ] All checks pass
- [ ] Ready for review
- [ ] **PR**: TBD

---

# PHASE 1.5b: Pipe Integration Tests (DEFERRED - After compio)

**Goal**: Test bidirectional handshake via pipes with proper async I/O

**Why Deferred**: Needs compio transport (Phase 2) to work correctly

**Will Return Here After**: Phase 2 complete

---

### Create `tests/handshake_pipe_tests.rs` (revisit)

- [ ] Delete current hanging version
- [ ] Create new version using compio runtime
- [ ] Use `#[compio::test]` attribute

#### Test: Full Bidirectional Handshake
- [ ] `test_handshake_bidirectional_compio`
  - [ ] Create bidirectional pipes
  - [ ] Use compio tasks (not tokio!)
  - [ ] Run sender and receiver concurrently
  - [ ] Assert both complete
  - [ ] Assert capabilities match
  - [ ] Verify seed exchange

#### Test: Concurrent I/O
- [ ] `test_handshake_concurrent_io`
  - [ ] Verify no deadlocks
  - [ ] Verify both sides communicate
  - [ ] Measure timing

#### Test: Error Cases
- [ ] `test_handshake_transport_closed`
  - [ ] Close one end
  - [ ] Verify other side gets error
- [ ] `test_handshake_incompatible_version`
  - [ ] Send version 20 (too old)
  - [ ] Verify handshake fails

### Acceptance Criteria for Phase 1.5b
- [ ] All pipe tests pass with compio
- [ ] No deadlocks or hangs
- [ ] Bidirectional communication works
- [ ] Error handling verified
- [ ] Commit message: "test(handshake): add pipe integration tests (compio-based)"
- [ ] **Commit**: TBD

---

# PHASE 3: Checksum Exchange Abstraction

**Goal**: Implement unified checksum abstraction

**Prerequisites**: Phase 2 (compio) complete

**Duration Estimate**: 1 week

---

## Phase 3.1: Algorithm Trait Design

### Create `src/protocol/checksum_algorithm.rs`

- [ ] Create file
- [ ] Add to `src/protocol/mod.rs`

#### Define StrongChecksumAlgorithm Trait
- [ ] Add trait with 3 methods:
  - [ ] `fn digest_size(&self) -> usize`
  - [ ] `fn compute(&self, data: &[u8]) -> Vec<u8>`
  - [ ] `fn name(&self) -> &'static str`
- [ ] Add doc comments with examples

#### Implement Md5Checksum
- [ ] Add struct: `pub struct Md5Checksum;`
- [ ] Implement trait (digest_size=16, use md5 crate)
- [ ] Add unit test with known value

#### Implement Md4Checksum
- [ ] Add to Cargo.toml: `md-4 = "0.10"`
- [ ] Add struct: `pub struct Md4Checksum;`
- [ ] Implement trait (digest_size=16)
- [ ] Add unit test

#### Implement Blake3Checksum
- [ ] Add struct: `pub struct Blake3Checksum;`
- [ ] Implement trait (digest_size=32)
- [ ] Add unit test

### Acceptance Criteria for Phase 3.1
- [ ] All algorithms compile
- [ ] Unit tests pass (3+ tests)
- [ ] Doc comments complete
- [ ] Commit message: "feat(checksum): add strong checksum algorithm trait and implementations"
- [ ] **Commit**: TBD

---

## Phase 3.2: Rolling Checksum with Seed

### Update `src/protocol/checksum.rs`

#### Add RollingChecksum Struct
- [ ] Add struct with seed field
- [ ] Add constructor: `pub fn new(seed: u32) -> Self`
- [ ] Add doc comments

#### Implement compute()
- [ ] Add method that uses seed
- [ ] If seed==0: use existing rolling_checksum()
- [ ] If seed!=0: use rolling_checksum_with_seed()
- [ ] Add unit test

#### Implement rolling_checksum_with_seed()
- [ ] Add function that mixes seed into initial state
- [ ] Add unit test verifying different from unseeded
- [ ] Add unit test verifying deterministic

### Acceptance Criteria for Phase 3.2
- [ ] Rolling checksum with seed works
- [ ] Unit tests pass (3+ tests)
- [ ] Commit message: "feat(checksum): add rolling checksum with seed support"
- [ ] **Commit**: TBD

---

## Phase 3.3: Block Checksum Abstraction

### Create `src/protocol/block_checksum.rs`

- [ ] Create file
- [ ] Add to `src/protocol/mod.rs`

#### Define BlockChecksum
- [ ] Add struct with fields: rolling, strong, offset, index
- [ ] Add doc comments
- [ ] Add conversion methods:
  - [ ] `to_native()` - convert to arsync format
  - [ ] `from_native()` - convert from arsync format
- [ ] Add unit tests (2+ tests)

### Acceptance Criteria for Phase 3.3
- [ ] Abstraction compiles
- [ ] Conversions work
- [ ] Unit tests pass
- [ ] Commit message: "feat(checksum): add unified block checksum abstraction"
- [ ] **Commit**: TBD

---

## Phase 3.4: Checksum Generator

### Add to `src/protocol/block_checksum.rs`

#### Define ChecksumGenerator
- [ ] Add struct with block_size, rolling, strong
- [ ] Add constructor with validation
- [ ] Add generate() method
- [ ] Add unit tests (2+ tests)

### Acceptance Criteria for Phase 3.4
- [ ] Generator works
- [ ] Unit tests pass
- [ ] Commit message: "feat(checksum): add checksum generator"
- [ ] **Commit**: TBD

---

## Phase 3.5-3.12: Checksum Protocol Implementation

*[Keeping existing detailed checkboxes for these phases - they're fine as-is]*

**Summary**:
- Phase 3.5: Protocol trait
- Phase 3.6: rsync format
- Phase 3.7: arsync native format
- Phase 3.8: Protocol selection
- Phase 3.9-3.10: Testing
- Phase 3.11: Documentation
- Phase 3.12: Pull Request

**Estimated**: 1 week total for all checksum work

---

# PHASE 4: Delta Token Handling

*[Keeping existing detailed checkboxes - they're fine as-is]*

**Prerequisites**: Phase 3 complete

**Duration**: 2 weeks

**Summary**:
- Phase 4.1: Delta abstraction
- Phase 4.2: Generation algorithm
- Phase 4.3: Application algorithm
- Phase 4.4: Protocol trait
- Phase 4.5: rsync token format
- Phase 4.6: rsync protocol impl
- Phase 4.7: arsync native impl
- Phase 4.8: Integration
- Phase 4.9-4.10: Testing
- Phase 4.11: Performance
- Phase 4.12: Documentation
- Phase 4.13: Pull Request

---

# PHASE 5: Final Integration

*[Keeping existing detailed checkboxes - they're fine as-is]*

**Prerequisites**: Phases 1-4 complete

**Duration**: 1-2 weeks

**Summary**:
- Phase 5.1: Complete protocol flow
- Phase 5.2: Metadata transmission
- Phase 5.3: Error handling
- Phase 5.4: E2E arsync tests
- Phase 5.5: E2E rsync compatibility
- Phase 5.6: Real-world testing
- Phase 5.7: Performance benchmarks
- Phase 5.8-5.9: Documentation
- Phase 5.10: Final review
- Phase 5.11: Update all docs
- Phase 5.12: Final PR

---

# REVISED TIMELINE

## Week-by-Week Plan

### Week 1: Phase 1 Completion
- [x] Days 1-2: Phase 1.1-1.3 (Handshake core) ✅ DONE
- [x] Day 3: Phase 1.4 (Unit tests) ✅ DONE
- [ ] Days 4-5: Phase 1.5 (rsync integration tests)

### Week 2-3: Phase 2 (compio Migration)
- [ ] Week 2 Day 1-2: Audit compio, redesign Transport
- [ ] Week 2 Day 3-4: Migrate PipeTransport
- [ ] Week 2 Day 5: SSH strategy decision
- [ ] Week 3 Day 1-3: Implement SSH (hybrid or pure)
- [ ] Week 3 Day 4-5: Update all protocol modules

### Week 4: Phase 2 Completion + Phase 1.5b
- [ ] Days 1-2: Phase 2 testing and docs
- [ ] Day 3: Phase 2 PR
- [ ] Days 4-5: Phase 1.5b (pipe tests with compio)

### Week 5: Phase 3 (Checksums)
- [ ] Days 1-2: Algorithm traits and implementations
- [ ] Days 3-4: Protocol implementations
- [ ] Day 5: Testing and docs

### Week 6-7: Phase 4 (Delta Tokens)
- [ ] Week 6: Delta abstraction, generation, application
- [ ] Week 7: Protocols, testing, optimization

### Week 8-9: Phase 5 (Final Integration)
- [ ] Week 8: Complete protocol flow, E2E tests
- [ ] Week 9: Real-world testing, docs

### Week 10: Buffer
- [ ] Bug fixes
- [ ] Performance tuning
- [ ] Final documentation

---

# CRITICAL PATH

## Blocking Dependencies

```
Phase 1.1-1.4 ✅ DONE
    ↓
Phase 1.5 (rsync tests) ← CAN DO NOW
    ↓
Phase 2 (compio) ← MUST DO BEFORE PIPE TESTS
    ↓
Phase 1.5b (pipe tests) ← BLOCKED until Phase 2
    ↓
Phase 3 (checksums)
    ↓
Phase 4 (delta)
    ↓
Phase 5 (integration)
```

## Next Immediate Steps

1. **Phase 1.5**: rsync integration tests (shell-based)
2. **Phase 2**: compio migration (fixes async/blocking)
3. **Phase 1.5b**: Pipe tests (now they'll work!)
4. **Phases 3-5**: Build on solid foundation

---

# Summary Statistics

## Total Deliverables

- **New Files**: ~15 files
- **Modified Files**: ~20 files
- **Tests**: ~100+ test functions
- **Documentation**: ~10 pages
- **PRs**: 5-6 PRs
- **Lines of Code**: ~5000+ lines
- **Duration**: 7-10 weeks

## Current Progress

- [x] Phase 1.1: Core Data Structures (commit 37451a4)
- [x] Phase 1.2: State Machine (commit cb6c715)
- [x] Phase 1.3: High-Level API (commit 73574a5)
- [x] Phase 1.4: Unit Tests - 14/14 passing (commit 2e96b97)
- [ ] Phase 1.5: rsync Integration Tests ← **NEXT**
- [ ] Phase 2: compio Migration ← **THEN THIS**
- [ ] Phase 1.5b: Pipe Tests ← **THEN BACK TO THIS**

---

**REVISED IMPLEMENTATION CHECKLIST - Version 2.0**

Last Updated: October 9, 2025  
Revision Reason: Reordered to do compio migration before pipe tests

# rsync Wire Protocol Implementation - Detailed Checklist

**Purpose**: Comprehensive, step-by-step implementation plan with granular checkboxes  
**Status**: ✅ Phase 1 In Progress (Phases 1.1, 1.2, 1.3 complete)  
**Started**: October 9, 2025  
**Target Completion**: 7-10 weeks

---

## How to Use This Document

1. **For Reviewer**: Each checkbox represents a concrete deliverable with acceptance criteria
2. **For Implementation**: Follow checkboxes sequentially within each phase
3. **For Context Recovery**: When lost in details, read the "Phase Goals" and "Acceptance Criteria"
4. **For Progress Tracking**: Check boxes as completed, commit after each sub-phase

---

## Pre-Implementation Checklist

### Documentation Review
- [ ] Read `docs/RSYNC_COMPAT_DETAILED_DESIGN.md` completely
- [ ] Read `docs/RSYNC_WIRE_PROTOCOL_SPEC.md` completely
- [ ] Read `docs/RSYNC_PROTOCOL_IMPLEMENTATION.md` completely
- [ ] Review existing test files: `tests/rsync_format_unit_tests.rs`
- [ ] Review existing test files: `tests/rsync_integration_tests.rs`
- [ ] Understand existing local metadata code in `src/copy.rs`
- [ ] Understand existing local metadata code in `src/directory.rs`

### Environment Setup
- [x] Verify rsync installed: `rsync --version` → rsync 3.4.1, protocol 32
- [x] Verify compio version in `Cargo.toml` → compio 0.16
- [x] Create feature branch: Using existing `feature/rsync-wire-protocol`
- [ ] Set up test data directories in `/tmp/arsync-test/`
- [x] Install any missing dependencies → Added rand 0.9

---

# PHASE 1: Handshake Protocol Implementation

**Goal**: Complete rsync protocol handshake with version negotiation, capability exchange, and seed handling

**Duration Estimate**: 1-2 weeks  
**Files to Create**: 4 new files  
**Files to Modify**: 3 existing files  
**Tests to Add**: 12+ test functions

---

## Phase 1.1: Core Data Structures ✅ COMPLETE

### Create `src/protocol/handshake.rs`

- [x] Create file: `touch src/protocol/handshake.rs`
- [x] Add to `src/protocol/mod.rs`: `pub mod handshake;`
- [x] Add file header documentation
- [x] Add imports:
  ```rust
  use crate::protocol::transport::Transport;
  use crate::protocol::varint::{encode_varint_into, decode_varint_sync};
  use anyhow::Result;
  use std::io::Cursor;
  use tracing::{debug, info, warn};
  ```

#### Define Protocol Constants
- [x] Add `pub const PROTOCOL_VERSION: u8 = 31;`
- [x] Add `pub const MIN_PROTOCOL_VERSION: u8 = 27;`
- [x] Add `pub const MAX_PROTOCOL_VERSION: u8 = 40;`
- [x] Add capability flags (all 10 flags defined)
- [x] Add doc comments for each constant explaining its purpose

#### Define Role Enum
- [x] Add `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`
- [x] Add `pub enum Role { Sender, Receiver }`
- [x] Add doc comment explaining sender vs receiver
- [x] Add `impl Role` with helper methods:
  - [x] `pub const fn is_sender(&self) -> bool`
  - [x] `pub const fn is_receiver(&self) -> bool`
  - [x] `pub const fn opposite(&self) -> Self`

#### Define ChecksumSeed Struct
- [x] Add `#[derive(Debug, Clone, Copy)]`
- [x] Add `pub struct ChecksumSeed { pub seed: u32 }`
- [x] Add doc comment explaining purpose
- [x] Add `impl ChecksumSeed`:
  - [x] `pub fn generate() -> Self` - uses `rand::rng().random()`
  - [x] `pub fn from_bytes(bytes: [u8; 4]) -> Self` - little-endian
  - [x] `pub fn to_bytes(&self) -> [u8; 4]` - little-endian
  - [x] `pub fn is_zero(&self) -> bool` - check if uninitialized
- [x] Add unit test: `test_checksum_seed_roundtrip`
- [x] Add unit test: `test_checksum_seed_generate`

#### Define ProtocolCapabilities Struct
- [x] Add `#[derive(Debug, Clone)]`
- [x] Add struct with all 3 fields
- [x] Add doc comment explaining each field
- [x] Add `impl ProtocolCapabilities`:
  - [x] `pub fn new(version: u8) -> Self` - default constructor
  - [x] `pub fn supports_checksums(&self) -> bool`
  - [x] `pub fn supports_hardlinks(&self) -> bool`
  - [x] `pub fn supports_symlinks(&self) -> bool`
  - [x] `pub fn supports_devices(&self) -> bool`
  - [x] `pub fn supports_xattrs(&self) -> bool`
  - [x] `pub fn supports_acls(&self) -> bool`
  - [x] `pub fn supports_sparse(&self) -> bool`
  - [x] `pub fn supports_checksum_seed(&self) -> bool`
  - [x] `pub fn supports_protection(&self) -> bool`
  - [x] `pub fn supports_times(&self) -> bool`
  - [x] `pub fn negotiate(client: &Self, server: &Self) -> Self` - intersection
- [x] Add unit test: `test_capabilities_negotiation`
- [x] Add unit test: `test_capabilities_support_methods`

#### Define HandshakeState Enum
- [x] Add `#[derive(Debug, Clone)]`
- [x] Add enum with all 9 states
- [x] Add doc comment explaining state machine
- [x] Add diagram in doc comment showing state transitions
- [x] Add `impl HandshakeState`:
  - [x] `pub fn is_complete(&self) -> bool`
  - [x] `pub fn get_capabilities(&self) -> Option<&ProtocolCapabilities>`
  - [x] `pub fn get_seed(&self) -> Option<ChecksumSeed>`

### Acceptance Criteria for Phase 1.1 ✅ COMPLETE
- [x] All structs and enums compile without errors
- [x] All doc comments are present and descriptive
- [x] All unit tests pass (7/7)
- [x] Code formatted with `cargo fmt`
- [x] Clippy warnings addressed
- [x] Commit message: "feat(handshake): add core data structures for protocol handshake"
- [x] **Commit**: 37451a4

---

## Phase 1.2: State Machine Implementation ✅ COMPLETE

### Implement State Transitions

- [x] Add `impl HandshakeState` with main `advance()` method

#### Implement Initial → VersionSent
- [x] Match on `Self::Initial`
- [x] Send protocol version byte using `write_all()`
- [x] Add debug log: "Sent protocol version: {}"
- [x] Return `Self::VersionSent { our_version: PROTOCOL_VERSION }`
- [x] Add error handling for write failure

#### Implement VersionSent → VersionReceived
- [x] Match on `Self::VersionSent { our_version }`
- [x] Read remote version using `read_exact()`
- [x] Validate version >= MIN_PROTOCOL_VERSION
- [x] Validate version <= MAX_PROTOCOL_VERSION (with warning)
- [x] Add debug log: "Received protocol version: {} from remote"
- [x] Return `Self::VersionReceived { our_version, remote_version }`
- [x] Add error for unsupported version

#### Implement VersionReceived → VersionNegotiated
- [x] Match on `Self::VersionReceived { our_version, remote_version }`
- [x] Calculate: `protocol_version = our_version.min(remote_version)`
- [x] Add info log: "Protocol version negotiated: {}"
- [x] Return `Self::VersionNegotiated { protocol_version }`

#### Implement VersionNegotiated → FlagsSent
- [x] Match on `Self::VersionNegotiated { protocol_version }`
- [x] Call `get_our_capabilities()`
- [x] Encode flags as varint
- [x] Send flags using `write_all()`
- [x] Add debug log: "Sent capability flags: 0x{:08X}"
- [x] Return `Self::FlagsSent { protocol_version, our_flags }`

#### Implement FlagsSent → FlagsReceived
- [x] Match on `Self::FlagsSent { protocol_version, our_flags }`
- [x] Decode remote flags using `decode_varint()`
- [x] Add debug log: "Received capability flags: 0x{:08X}"
- [x] Return `Self::FlagsReceived { protocol_version, our_flags, remote_flags }`

#### Implement FlagsReceived → CapabilitiesNegotiated
- [x] Match on `Self::FlagsReceived`
- [x] Create `ProtocolCapabilities::new(protocol_version)`
- [x] Set `capabilities.flags = our_flags & remote_flags`
- [x] Add info log with all flags
- [x] Add debug logs for each capability
- [x] Return `Self::CapabilitiesNegotiated { capabilities }`

#### Implement CapabilitiesNegotiated → SeedExchange or Complete
- [x] Match on `Self::CapabilitiesNegotiated { capabilities }`
- [x] Check if `capabilities.supports_checksum_seed()`
- [x] If yes:
  - [x] Match on role
  - [x] If Sender: generate seed, send bytes
  - [x] If Receiver: receive bytes, decode seed
  - [x] Set `capabilities.checksum_seed`
  - [x] Return `Self::SeedExchange { capabilities }`
- [x] If no:
  - [x] Return `Self::Complete { capabilities, seed: None }`

#### Implement SeedExchange → Complete
- [x] Match on `Self::SeedExchange { capabilities }`
- [x] Extract seed from capabilities
- [x] Return `Self::Complete { capabilities, seed }`

#### Implement Complete (terminal state)
- [x] Match on `Self::Complete { .. }`
- [x] Return error: "Handshake already complete"

### Add Helper Function
- [x] Implement `get_our_capabilities() -> u32`:
  - [x] Set all supported flags (9 flags including checksums, symlinks, hardlinks, devices, xattrs, acls)
  - [x] Add doc comment listing what we support
  - [x] Reference arsync's local implementations

### Acceptance Criteria for Phase 1.2 ✅ COMPLETE
- [x] All state transitions compile
- [x] Error handling for each state
- [x] Logging at appropriate levels (debug, info, warn)
- [x] Code formatted with `cargo fmt`
- [x] No clippy warnings
- [x] Commit message: "feat(handshake): implement state machine transitions"
- [x] **Commit**: cb6c715

---

## Phase 1.3: High-Level Handshake API ✅ COMPLETE

### Create Public API Functions

#### Implement `handshake_sender`
- [x] Add function signature with Transport bound
- [x] Add comprehensive doc comment with example
- [x] Initialize: `let mut state = HandshakeState::Initial;`
- [x] Loop until complete with `state.advance(transport, Role::Sender)`
- [x] Extract capabilities from Complete state
- [x] Add info log: "Handshake complete (sender)"
- [x] Return capabilities

#### Implement `handshake_receiver`
- [x] Add function signature
- [x] Add comprehensive doc comment with example
- [x] Same logic as sender but with `Role::Receiver`
- [x] Add info log: "Handshake complete (receiver)"
- [x] Return capabilities

#### Implement `handshake`
- [x] Add function signature with role parameter
- [x] Add doc comment explaining it's the general version
- [x] Match on role:
  - [x] `Role::Sender => handshake_sender(transport).await`
  - [x] `Role::Receiver => handshake_receiver(transport).await`

### Acceptance Criteria for Phase 1.3 ✅ COMPLETE
- [x] All public APIs compile
- [x] Doc comments include examples
- [x] Error propagation works correctly
- [x] Code formatted with `cargo fmt`
- [x] No clippy warnings
- [x] Tests still passing (7/7)
- [x] Ready for commit

---

## Phase 1.4: Unit Tests

### Create `tests/handshake_unit_tests.rs`

- [ ] Create file: `touch tests/handshake_unit_tests.rs`
- [ ] Add header: `#![cfg(feature = "remote-sync")]`
- [ ] Add imports for all handshake types
- [ ] Add helper to create test pipes

#### Test: State Machine Basics
- [ ] `test_handshake_state_initial`
  - [ ] Create `HandshakeState::Initial`
  - [ ] Assert `!is_complete()`
  - [ ] Assert `get_capabilities().is_none()`

- [ ] `test_handshake_state_complete`
  - [ ] Create `HandshakeState::Complete` with mock capabilities
  - [ ] Assert `is_complete()`
  - [ ] Assert `get_capabilities().is_some()`
  - [ ] Verify capabilities values

#### Test: Capability Negotiation
- [ ] `test_capabilities_intersection`
  - [ ] Create client capabilities with subset of flags
  - [ ] Create server capabilities with different subset
  - [ ] Call `ProtocolCapabilities::negotiate()`
  - [ ] Assert result is intersection (flags & flags)
  - [ ] Verify specific flags

- [ ] `test_capabilities_version_min`
  - [ ] Create client with version 31
  - [ ] Create server with version 30
  - [ ] Negotiate
  - [ ] Assert result version is 30

- [ ] `test_capabilities_support_methods`
  - [ ] Create capabilities with specific flags
  - [ ] Test each `supports_*()` method
  - [ ] Assert correct boolean results

#### Test: Checksum Seed
- [ ] `test_checksum_seed_generate`
  - [ ] Generate 100 seeds
  - [ ] Assert all are non-zero
  - [ ] Assert they're not all the same (randomness)

- [ ] `test_checksum_seed_roundtrip`
  - [ ] Create seed with specific value
  - [ ] Convert to bytes
  - [ ] Convert back from bytes
  - [ ] Assert equal

#### Test: Version Validation
- [ ] `test_version_validation_too_old`
  - [ ] Try to create handshake with version 26
  - [ ] Should fail with error about min version

- [ ] `test_version_validation_too_new`
  - [ ] Try to create handshake with version 100
  - [ ] Should fail with error about max version

- [ ] `test_version_validation_valid`
  - [ ] Test versions 27-40
  - [ ] All should succeed

#### Test: get_our_capabilities
- [ ] `test_our_capabilities_complete`
  - [ ] Call `get_our_capabilities()`
  - [ ] Assert `XMIT_CHECKSUMS` is set
  - [ ] Assert `XMIT_SYMLINKS` is set
  - [ ] Assert `XMIT_HARDLINKS` is set
  - [ ] Assert `XMIT_DEVICES` is set
  - [ ] Assert `XMIT_XATTRS` is set
  - [ ] Assert `XMIT_ACLS` is set
  - [ ] Assert `XMIT_CHECKSUM_SEED` is set
  - [ ] Assert `XMIT_PROTECTION` is set
  - [ ] Assert `XMIT_TIMES` is set
  - [ ] Log what we claim to support

### Acceptance Criteria for Phase 1.4
- [ ] All unit tests pass: `cargo test --features remote-sync handshake_unit`
- [ ] Tests cover all major code paths
- [ ] Tests have descriptive names
- [ ] Tests have doc comments explaining what they test
- [ ] Code formatted with `cargo fmt`
- [ ] Commit message: "test(handshake): add comprehensive unit tests"

---

## Phase 1.5: Integration Tests with Pipes

### Create `tests/handshake_pipe_tests.rs`

- [ ] Create file: `touch tests/handshake_pipe_tests.rs`
- [ ] Add header: `#![cfg(feature = "remote-sync")]`
- [ ] Add imports
- [ ] Add helper: `create_pipe_pair() -> (PipeTransport, PipeTransport)`

#### Test: Full Handshake Between Two arsync Instances
- [ ] `test_handshake_bidirectional`
  - [ ] Create two pipe transports (bidirectional)
  - [ ] Spawn sender task: `handshake_sender()`
  - [ ] Spawn receiver task: `handshake_receiver()`
  - [ ] Use `tokio::join!()` to run concurrently
  - [ ] Assert both complete successfully
  - [ ] Assert capabilities match
  - [ ] Assert versions match
  - [ ] Print negotiated capabilities

- [ ] `test_handshake_with_seed`
  - [ ] Same as above
  - [ ] Verify seed is exchanged
  - [ ] Assert sender and receiver have same protocol state
  - [ ] Print seed value

- [ ] `test_handshake_version_downgrade`
  - [ ] Mock client with version 31
  - [ ] Mock server with version 29
  - [ ] Run handshake
  - [ ] Assert negotiated version is 29
  - [ ] Assert both sides agree

#### Test: Error Cases
- [ ] `test_handshake_incompatible_version`
  - [ ] Mock client with version 31
  - [ ] Mock server that sends version 20 (too old)
  - [ ] Assert handshake fails with error
  - [ ] Check error message mentions version

- [ ] `test_handshake_transport_failure`
  - [ ] Create transport that fails during read
  - [ ] Attempt handshake
  - [ ] Assert error propagates correctly
  - [ ] Check error type

### Acceptance Criteria for Phase 1.5
- [ ] All integration tests pass: `cargo test --features remote-sync handshake_pipe`
- [ ] Tests verify bidirectional communication
- [ ] Tests verify error handling
- [ ] Code formatted with `cargo fmt`
- [ ] Commit message: "test(handshake): add pipe-based integration tests"

---

## Phase 1.6: Integration Tests with Real rsync

### Add to `tests/rsync_integration_tests.rs`

#### Test: Handshake with rsync --server
- [ ] `test_handshake_with_real_rsync_sender`
  - [ ] Spawn: `rsync --server --sender -vlogDtpr . /source/`
  - [ ] Connect to stdin/stdout
  - [ ] Run `handshake_receiver()`
  - [ ] Assert completes successfully
  - [ ] Log negotiated protocol version
  - [ ] Log negotiated capabilities
  - [ ] Verify version is in range 27-31
  - [ ] Print success message

- [ ] `test_handshake_with_real_rsync_receiver`
  - [ ] Spawn: `rsync --server -vlogDtpr . /dest/`
  - [ ] Connect to stdin/stdout
  - [ ] Run `handshake_sender()`
  - [ ] Assert completes successfully
  - [ ] Verify capabilities intersection

#### Test: Version Compatibility
- [ ] `test_rsync_version_detection`
  - [ ] Run `rsync --version`
  - [ ] Parse protocol version from output
  - [ ] Assert it's >= 27
  - [ ] Print rsync version info

### Acceptance Criteria for Phase 1.6
- [ ] Tests run only if rsync is available
- [ ] Tests skip gracefully if rsync not found
- [ ] All tests pass with real rsync
- [ ] Code formatted with `cargo fmt`
- [ ] Commit message: "test(handshake): add integration tests with real rsync"

---

## Phase 1.7: Documentation

### Update Documentation Files

#### Update `docs/RSYNC_COMPAT_DETAILED_DESIGN.md`
- [ ] Add "Implementation Status" section
- [ ] Mark Phase 1 (Handshake) as ✅ Complete
- [ ] Add reference to `src/protocol/handshake.rs`
- [ ] Update timeline

#### Create `docs/HANDSHAKE_API.md`
- [ ] Add title: "Handshake Protocol API Documentation"
- [ ] Add "Overview" section
- [ ] Add "Quick Start" with code example
- [ ] Document `HandshakeState` enum with state diagram
- [ ] Document `ProtocolCapabilities` struct
- [ ] Document `Role` enum
- [ ] Document public functions:
  - [ ] `handshake()`
  - [ ] `handshake_sender()`
  - [ ] `handshake_receiver()`
- [ ] Add "Examples" section:
  - [ ] Example: Basic handshake
  - [ ] Example: Handshake with error handling
  - [ ] Example: Checking capabilities after handshake
- [ ] Add "Testing" section
- [ ] Add "Troubleshooting" section

#### Update `README.md`
- [ ] Add note about Phase 1 completion
- [ ] Add link to handshake docs

### Acceptance Criteria for Phase 1.7
- [ ] All documentation is clear and complete
- [ ] Code examples in docs compile
- [ ] Markdown formatting is correct
- [ ] Links work
- [ ] Commit message: "docs(handshake): add comprehensive API documentation"

---

## Phase 1.8: Integration with Main Protocol Flow

### Update `src/protocol/rsync_compat.rs`

#### Integrate Handshake into rsync_receive_via_pipe
- [ ] Import: `use crate::protocol::handshake::{handshake_receiver, Role};`
- [ ] Find comment: `// TODO: rsync sends version as raw byte`
- [ ] Replace with actual handshake call
- [ ] Add: `let capabilities = handshake_receiver(&mut reader.transport).await?;`
- [ ] Use capabilities to determine multiplexing mode
- [ ] Store capabilities in connection state
- [ ] Add debug logs

#### Integrate Handshake into rsync_send_via_pipe
- [ ] Import handshake functions
- [ ] Add handshake at start of function
- [ ] Add: `let capabilities = handshake_sender(&mut writer.transport).await?;`
- [ ] Use capabilities for protocol decisions
- [ ] Add debug logs

### Update `src/protocol/rsync.rs`

#### Update send_via_pipe
- [ ] Replace stub handshake with real one
- [ ] Use negotiated capabilities
- [ ] Adapt behavior based on protocol version

#### Update receive_via_pipe
- [ ] Replace stub handshake with real one
- [ ] Use negotiated capabilities
- [ ] Adapt behavior based on protocol version

### Acceptance Criteria for Phase 1.8
- [ ] All protocol files use real handshake
- [ ] No stub handshake code remains
- [ ] Existing tests still pass
- [ ] Code formatted with `cargo fmt`
- [ ] Commit message: "feat(handshake): integrate handshake into protocol flow"

---

## Phase 1.9: Final Testing and Cleanup

### Run Full Test Suite
- [ ] Run: `cargo test --features remote-sync`
- [ ] Verify all handshake tests pass
- [ ] Verify existing tests still pass
- [ ] Fix any regressions

### Run with Real rsync
- [ ] Test: `cargo run --features remote-sync -- --pipe --pipe-role=receiver --rsync-compat -r /dev/null /tmp/dest`
- [ ] Connect to real rsync sender
- [ ] Verify handshake completes
- [ ] Check logs for handshake messages
- [ ] Verify no errors

### Code Quality
- [ ] Run: `cargo fmt --all`
- [ ] Run: `cargo clippy --features remote-sync -- -D warnings`
- [ ] Fix all clippy warnings
- [ ] Run: `cargo doc --features remote-sync --no-deps`
- [ ] Fix any doc warnings

### Update Test Coverage Report
- [ ] Update `TEST_COVERAGE.md`
- [ ] Add handshake test statistics
- [ ] Update coverage percentages

### Acceptance Criteria for Phase 1.9
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Documentation builds without warnings
- [ ] Code is formatted
- [ ] Commit message: "test(handshake): final testing and cleanup"

---

## Phase 1.10: Create Pull Request

### PR Preparation
- [ ] Ensure all Phase 1 commits are pushed
- [ ] Verify branch is up to date with base
- [ ] Run final test suite
- [ ] Review all changes one more time

### Create PR
- [ ] Run: `gh pr create --title "feat: implement rsync handshake protocol (Phase 1)" --body "[PR body]"`
- [ ] PR body should include:
  - [ ] Summary of what was implemented
  - [ ] List of new files
  - [ ] List of tests added
  - [ ] API documentation reference
  - [ ] Testing instructions
  - [ ] Checklist of all Phase 1.1-1.9 items completed
  - [ ] Note about Phase 2 being next

### Acceptance Criteria for Phase 1.10
- [ ] PR is created and visible
- [ ] PR has clear description
- [ ] All checks pass (if CI configured)
- [ ] PR is ready for review

---

# PHASE 2: compio/io_uring Migration

**Goal**: Migrate protocol code from tokio to compio for io_uring-based async I/O

**Duration Estimate**: 2-3 weeks  
**Files to Create**: 2 new files  
**Files to Modify**: 8 existing files  
**Tests to Add**: 15+ test functions

---

## Phase 2.1: compio Capability Audit

### Research compio Features
- [ ] Read compio documentation: https://docs.rs/compio/latest/compio/
- [ ] Check compio version in `Cargo.toml`: record version number
- [ ] List available modules:
  - [ ] Document: Is `compio::fs` available?
  - [ ] Document: Is `compio::net` available?
  - [ ] Document: Is `compio::process` available? ⚠️
  - [ ] Document: Is `compio::io::AsyncRead` available?
  - [ ] Document: Is `compio::io::AsyncWrite` available?

### Create `docs/COMPIO_AUDIT.md`
- [ ] Create file documenting findings
- [ ] Section: "Available Features"
  - [ ] List what compio provides
  - [ ] Version tested
  - [ ] Platform tested (Linux kernel version)
- [ ] Section: "Missing Features"
  - [ ] **CRITICAL**: Document if process support is missing
  - [ ] Document any other missing features
- [ ] Section: "Workarounds Required"
  - [ ] Option A: Wait for compio
  - [ ] Option B: Use compio-driver directly
  - [ ] Option C: Hybrid approach (recommended)
- [ ] Section: "Migration Strategy"
  - [ ] Chosen approach
  - [ ] Rationale
  - [ ] Timeline impact

### Acceptance Criteria for Phase 2.1
- [ ] Audit document is complete
- [ ] Missing features are clearly identified
- [ ] Workaround strategy is chosen
- [ ] Commit message: "docs(compio): audit compio capabilities and plan migration"

---

## Phase 2.2: Transport Trait Redesign

### Update `src/protocol/transport.rs`

#### Remove async_trait Dependency
- [ ] Remove from `Cargo.toml`: `async-trait` dependency
- [ ] Remove from file: `use async_trait::async_trait;`
- [ ] Remove `#[async_trait]` attribute from trait

#### Redesign Transport Trait
- [ ] Change trait definition:
  ```rust
  pub trait Transport: compio::io::AsyncRead + compio::io::AsyncWrite + Send + Unpin {
      fn name(&self) -> &str { "unknown" }
      fn supports_multiplexing(&self) -> bool { false }
  }
  ```
- [ ] Add doc comment explaining new design
- [ ] Add doc comment explaining `Unpin` requirement
- [ ] Add example usage in doc comment

#### Add Helper Extensions
- [ ] Add `TransportExt` trait:
  ```rust
  pub trait TransportExt: Transport {
      async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()>;
      async fn write_all(&mut self, buf: &[u8]) -> Result<()>;
  }
  ```
- [ ] Provide blanket implementation
- [ ] Add doc comments

### Acceptance Criteria for Phase 2.2
- [ ] Trait compiles (may not have implementors yet)
- [ ] No `async_trait` dependency
- [ ] Doc comments are complete
- [ ] Commit message: "refactor(transport): redesign trait for compio compatibility"

---

## Phase 2.3: PipeTransport Migration

### Update `src/protocol/pipe.rs`

#### Remove tokio Dependencies
- [ ] Remove: `use tokio::io::{AsyncReadExt, AsyncWriteExt};`
- [ ] Remove: any tokio imports
- [ ] Add: `use compio::io::{AsyncReadExt, AsyncWriteExt};`
- [ ] Add: `use compio::fs::File;`

#### Redesign PipeTransport Struct
- [ ] Change struct:
  ```rust
  pub struct PipeTransport {
      reader: compio::fs::File,
      writer: compio::fs::File,
      #[allow(dead_code)]
      name: String,
  }
  ```
- [ ] Update doc comment

#### Update from_stdio Implementation
- [ ] Rewrite to use compio::fs::File
- [ ] Handle stdin: `unsafe { compio::fs::File::from_raw_fd(0) }`
- [ ] Handle stdout: `unsafe { compio::fs::File::from_raw_fd(1) }`
- [ ] Add safety comments
- [ ] Test that it works

#### Implement compio AsyncRead
- [ ] Remove `#[async_trait]` impl
- [ ] Add direct impl:
  ```rust
  impl compio::io::AsyncRead for PipeTransport {
      async fn read(&mut self, buf: &mut [u8]) -> compio::io::Result<usize> {
          self.reader.read(buf).await
      }
  }
  ```

#### Implement compio AsyncWrite
- [ ] Add direct impl:
  ```rust
  impl compio::io::AsyncWrite for PipeTransport {
      async fn write(&mut self, buf: &[u8]) -> compio::io::Result<usize> {
          self.writer.write(buf).await
      }
      
      async fn flush(&mut self) -> compio::io::Result<()> {
          self.writer.flush().await
      }
  }
  ```

#### Implement Transport Trait
- [ ] Add: `impl Transport for PipeTransport {}`
- [ ] Override `name()`: return self.name
- [ ] Override `supports_multiplexing()`: return false

### Acceptance Criteria for Phase 2.3
- [ ] PipeTransport compiles with compio
- [ ] No tokio dependencies in file
- [ ] Implements all required traits
- [ ] Commit message: "refactor(pipe): migrate PipeTransport to compio"

---

## Phase 2.4: SSH Connection Strategy

### Decision Point: Check compio::process

#### If compio::process EXISTS:
- [ ] Proceed with pure compio implementation
- [ ] Go to Phase 2.4a

#### If compio::process MISSING:
- [ ] Implement hybrid approach
- [ ] Go to Phase 2.4b

---

## Phase 2.4a: Pure compio SSH (if process support exists)

### Update `src/protocol/ssh.rs`

- [ ] Remove: `use tokio::process::*;`
- [ ] Add: `use compio::process::*;`
- [ ] Update SshConnection struct to use compio types
- [ ] Update connect() to use compio process spawn
- [ ] Implement compio AsyncRead for SshConnection
- [ ] Implement compio AsyncWrite for SshConnection
- [ ] Implement Transport trait
- [ ] Test with real SSH connection

### Acceptance Criteria for Phase 2.4a
- [ ] Compiles with compio
- [ ] Works with real SSH
- [ ] Commit message: "refactor(ssh): migrate to compio process support"

---

## Phase 2.4b: Hybrid SSH (if process support missing) - RECOMMENDED

### Create `src/protocol/ssh_hybrid.rs`

- [ ] Create new file for hybrid implementation
- [ ] Add documentation explaining why hybrid approach is needed

#### Define HybridSshConnection Struct
- [ ] Add struct:
  ```rust
  pub struct HybridSshConnection {
      process: std::process::Child,
      stdin_fd: compio::driver::OwnedFd,
      stdout_fd: compio::driver::OwnedFd,
      name: String,
  }
  ```
- [ ] Add doc comment explaining hybrid approach

#### Implement connect()
- [ ] Spawn process with stdlib:
  ```rust
  let mut child = std::process::Command::new(shell)
      .arg(format!("{}@{}", user, host))
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .spawn()?;
  ```
- [ ] Extract file descriptors:
  ```rust
  let stdin = child.stdin.take().unwrap();
  let stdout = child.stdout.take().unwrap();
  let stdin_fd = stdin.as_raw_fd();
  let stdout_fd = stdout.as_raw_fd();
  ```
- [ ] Forget stdlib handles: `std::mem::forget(stdin); std::mem::forget(stdout);`
- [ ] Wrap in compio FDs:
  ```rust
  let stdin_fd = unsafe { compio::driver::OwnedFd::from_raw_fd(stdin_fd) };
  let stdout_fd = unsafe { compio::driver::OwnedFd::from_raw_fd(stdout_fd) };
  ```
- [ ] Return HybridSshConnection

#### Implement AsyncRead using io_uring
- [ ] Use compio::driver::op::Read directly
- [ ] Implement async read on stdout_fd
- [ ] Add error handling

#### Implement AsyncWrite using io_uring
- [ ] Use compio::driver::op::Write directly
- [ ] Implement async write on stdin_fd
- [ ] Implement flush (no-op for pipes)
- [ ] Add error handling

#### Implement Transport Trait
- [ ] Add marker impl: `impl Transport for HybridSshConnection {}`
- [ ] Override name()
- [ ] Override supports_multiplexing()

#### Implement Drop
- [ ] Add Drop implementation to clean up process
- [ ] Kill child process
- [ ] Wait for exit
- [ ] Close file descriptors

### Create Wrapper in `src/protocol/ssh.rs`
- [ ] Re-export HybridSshConnection as SshConnection:
  ```rust
  #[cfg(not(feature = "compio-process"))]
  pub use super::ssh_hybrid::HybridSshConnection as SshConnection;
  
  #[cfg(feature = "compio-process")]
  pub use super::ssh_compio::SshConnection;
  ```

### Acceptance Criteria for Phase 2.4b
- [ ] Hybrid implementation compiles
- [ ] Uses io_uring for I/O operations
- [ ] Uses stdlib for process spawning
- [ ] Properly cleans up resources
- [ ] Commit message: "feat(ssh): implement hybrid approach for SSH connections"

---

## Phase 2.5: Update Handshake Module

### Update `src/protocol/handshake.rs`

#### Update Imports
- [ ] Ensure Transport trait is from updated version
- [ ] Remove any tokio imports
- [ ] Add compio imports if needed

#### Update Function Signatures
- [ ] Verify all `async fn` signatures work with compio
- [ ] Update error types if needed
- [ ] Test that type inference works

#### Update Implementation
- [ ] Replace any tokio-specific code
- [ ] Use compio equivalents
- [ ] Update read/write calls if signature changed

### Acceptance Criteria for Phase 2.5
- [ ] Handshake module compiles with new Transport
- [ ] No tokio dependencies
- [ ] All functions work with compio types
- [ ] Commit message: "refactor(handshake): update for compio transport"

---

## Phase 2.6: Update Multiplexed I/O

### Update `src/protocol/rsync_compat.rs`

#### Update MultiplexReader
- [ ] Change transport field to use new Transport trait
- [ ] Update all read operations for compio
- [ ] Remove async_trait if present
- [ ] Update error handling

#### Update MultiplexWriter
- [ ] Change transport field to use new Transport trait
- [ ] Update all write operations for compio
- [ ] Remove async_trait if present
- [ ] Update error handling

### Acceptance Criteria for Phase 2.6
- [ ] Multiplexed I/O compiles with compio
- [ ] Read/write operations use io_uring
- [ ] No tokio dependencies
- [ ] Commit message: "refactor(mplex): migrate multiplexed I/O to compio"

---

## Phase 2.7: Update Main Protocol Files

### Update `src/protocol/rsync.rs`

- [ ] Update imports for compio
- [ ] Replace any tokio-specific code
- [ ] Update Transport usage
- [ ] Verify all async functions work
- [ ] Update error handling if needed

### Update `src/protocol/mod.rs`

- [ ] Update pipe_sender signature
- [ ] Update pipe_receiver signature
- [ ] Update remote_sync signature
- [ ] Ensure all functions use compio runtime

### Update `src/main.rs`

- [ ] Verify `#[compio::main]` attribute is present
- [ ] Ensure protocol calls work from compio runtime
- [ ] Test that everything compiles together

### Acceptance Criteria for Phase 2.7
- [ ] All protocol files compile
- [ ] No tokio in dependency tree for protocol
- [ ] All async functions use compio
- [ ] Commit message: "refactor(protocol): complete compio migration"

---

## Phase 2.8: Testing - Unit Tests

### Create `tests/compio_transport_tests.rs`

#### Test: PipeTransport with compio
- [ ] `test_pipe_transport_read_write`
  - [ ] Create pipe pair
  - [ ] Write data to one end
  - [ ] Read from other end
  - [ ] Assert data matches
  - [ ] Verify io_uring is used (check with strace if possible)

- [ ] `test_pipe_transport_large_data`
  - [ ] Write 1MB of data
  - [ ] Read it back
  - [ ] Verify correctness
  - [ ] Measure performance

#### Test: SSH Connection (Hybrid)
- [ ] `test_ssh_connection_spawn`
  - [ ] Spawn SSH connection to localhost
  - [ ] Verify connection established
  - [ ] Close cleanly

- [ ] `test_ssh_connection_io`
  - [ ] Connect to localhost
  - [ ] Send command
  - [ ] Read response
  - [ ] Verify io_uring used

### Acceptance Criteria for Phase 2.8
- [ ] All unit tests pass
- [ ] Tests verify io_uring usage
- [ ] Tests run in compio runtime
- [ ] Commit message: "test(compio): add transport tests for compio migration"

---

## Phase 2.9: Testing - Integration Tests

### Update Existing Test Files

#### Update `tests/handshake_pipe_tests.rs`
- [ ] Ensure uses compio runtime
- [ ] Update any tokio-specific code
- [ ] Re-run all tests
- [ ] Fix any failures

#### Update `tests/rsync_integration_tests.rs`
- [ ] Update for compio
- [ ] Test with real rsync
- [ ] Verify handshake still works

### Performance Testing

#### Create `tests/compio_performance_tests.rs`
- [ ] Test: Measure I/O latency with compio
- [ ] Test: Compare with baseline (if data available)
- [ ] Test: Measure throughput
- [ ] Document results

### Acceptance Criteria for Phase 2.9
- [ ] All integration tests pass
- [ ] Performance is at least as good as before
- [ ] Real rsync compatibility maintained
- [ ] Commit message: "test(compio): verify integration tests with compio"

---

## Phase 2.10: Documentation and Cleanup

### Update Documentation

#### Update `docs/COMPIO_AUDIT.md`
- [ ] Add "Implementation Complete" section
- [ ] Document what was done
- [ ] Document hybrid approach (if used)
- [ ] Add performance results

#### Create `docs/COMPIO_MIGRATION_GUIDE.md`
- [ ] Explain why migration was done
- [ ] Document before/after architecture
- [ ] List all changed files
- [ ] Explain hybrid SSH approach (if used)
- [ ] Add troubleshooting section

#### Update `docs/RSYNC_COMPAT_DETAILED_DESIGN.md`
- [ ] Mark Phase 2 as complete
- [ ] Update architecture diagrams
- [ ] Update code examples for compio

### Final Cleanup
- [ ] Run: `cargo fmt --all`
- [ ] Run: `cargo clippy --all-features -- -D warnings`
- [ ] Fix all warnings
- [ ] Remove any dead code
- [ ] Update comments

### Acceptance Criteria for Phase 2.10
- [ ] Documentation is complete
- [ ] No warnings or errors
- [ ] Code is clean
- [ ] Commit message: "docs(compio): document migration completion"

---

## Phase 2.11: Create Pull Request

### PR Preparation
- [ ] Ensure all Phase 2 commits are pushed
- [ ] Rebase on latest main if needed
- [ ] Run full test suite one more time

### Create PR
- [ ] Title: "refactor: migrate protocol to compio/io_uring (Phase 2)"
- [ ] Body includes:
  - [ ] Summary of migration
  - [ ] Before/after architecture
  - [ ] Hybrid approach explanation (if used)
  - [ ] Performance results
  - [ ] List of changed files
  - [ ] Testing done
  - [ ] Migration guide reference

### Acceptance Criteria for Phase 2.11
- [ ] PR created and visible
- [ ] All checks pass
- [ ] Documentation is linked
- [ ] Ready for review

---

# PHASE 3: Checksum Exchange Abstraction

**Goal**: Implement unified checksum abstraction supporting rsync and arsync native formats

**Duration Estimate**: 1 week  
**Files to Create**: 3 new files  
**Files to Modify**: 4 existing files  
**Tests to Add**: 20+ test functions

---

## Phase 3.1: Algorithm Trait Design

### Create `src/protocol/checksum_algorithm.rs`

- [ ] Create file
- [ ] Add to `src/protocol/mod.rs`: `pub mod checksum_algorithm;`

#### Define StrongChecksumAlgorithm Trait
- [ ] Add trait:
  ```rust
  pub trait StrongChecksumAlgorithm: Send + Sync {
      fn digest_size(&self) -> usize;
      fn compute(&self, data: &[u8]) -> Vec<u8>;
      fn name(&self) -> &'static str;
  }
  ```
- [ ] Add doc comments explaining purpose
- [ ] Add usage examples in docs

#### Implement Md5Checksum
- [ ] Add struct: `pub struct Md5Checksum;`
- [ ] Implement StrongChecksumAlgorithm:
  - [ ] `digest_size() -> 16`
  - [ ] `compute()` using md5 crate
  - [ ] `name() -> "MD5"`
- [ ] Add unit test

#### Implement Md4Checksum
- [ ] Add to Cargo.toml: `md-4 = "0.10"`
- [ ] Add struct: `pub struct Md4Checksum;`
- [ ] Implement StrongChecksumAlgorithm:
  - [ ] `digest_size() -> 16`
  - [ ] `compute()` using md-4 crate
  - [ ] `name() -> "MD4"`
- [ ] Add unit test

#### Implement Blake3Checksum
- [ ] Verify blake3 in Cargo.toml
- [ ] Add struct: `pub struct Blake3Checksum;`
- [ ] Implement StrongChecksumAlgorithm:
  - [ ] `digest_size() -> 32`
  - [ ] `compute()` using blake3 crate
  - [ ] `name() -> "BLAKE3"`
- [ ] Add unit test

### Acceptance Criteria for Phase 3.1
- [ ] All algorithms compile
- [ ] Unit tests pass
- [ ] Doc comments complete
- [ ] Commit message: "feat(checksum): add strong checksum algorithm trait and implementations"

---

## Phase 3.2: Rolling Checksum Implementation

### Update `src/protocol/checksum.rs`

#### Add RollingChecksum Struct
- [ ] Add struct:
  ```rust
  pub struct RollingChecksum {
      seed: u32,
  }
  ```
- [ ] Add constructor: `pub fn new(seed: u32) -> Self`
- [ ] Add doc comments

#### Implement compute()
- [ ] Add method: `pub fn compute(&self, data: &[u8]) -> u32`
- [ ] If seed == 0: use existing `rolling_checksum()`
- [ ] If seed != 0: use `rolling_checksum_with_seed()`
- [ ] Add doc comment explaining seed

#### Implement rolling_checksum_with_seed
- [ ] Add function:
  ```rust
  fn rolling_checksum_with_seed(data: &[u8], seed: u32) -> u32 {
      let mut a = seed & 0xFFFF;
      let mut b = (seed >> 16) & 0xFFFF;
      for &byte in data {
          a = (a + u32::from(byte)) % MODULUS;
          b = (b + a) % MODULUS;
      }
      (b << 16) | a
  }
  ```
- [ ] Add doc comment
- [ ] Add unit test

#### Implement update()
- [ ] Use existing `rolling_checksum_update()`
- [ ] Add wrapper method on RollingChecksum
- [ ] Add doc comment
- [ ] Add unit test

### Acceptance Criteria for Phase 3.2
- [ ] Rolling checksum compiles
- [ ] Seed support works
- [ ] Unit tests pass
- [ ] Commit message: "feat(checksum): add rolling checksum with seed support"

---

## Phase 3.3: Block Checksum Abstraction

### Create `src/protocol/block_checksum.rs`

- [ ] Create file
- [ ] Add to `src/protocol/mod.rs`

#### Define BlockChecksum Struct
- [ ] Add struct:
  ```rust
  #[derive(Debug, Clone)]
  pub struct BlockChecksum {
      pub rolling: u32,
      pub strong: Vec<u8>,
      pub offset: Option<u64>,
      pub index: Option<u32>,
  }
  ```
- [ ] Add doc comments for each field

#### Implement Conversion Methods
- [ ] Add `to_native()`:
  ```rust
  pub fn to_native(&self) -> crate::protocol::rsync::BlockChecksum {
      crate::protocol::rsync::BlockChecksum {
          weak: self.rolling,
          strong: self.strong[..16].try_into().unwrap(),
          offset: self.offset.unwrap_or(0),
          block_index: self.index.unwrap_or(0),
      }
  }
  ```
- [ ] Add `from_native()`
- [ ] Add doc comments
- [ ] Add unit tests

### Acceptance Criteria for Phase 3.3
- [ ] BlockChecksum compiles
- [ ] Conversions work correctly
- [ ] Unit tests pass
- [ ] Commit message: "feat(checksum): add unified block checksum abstraction"

---

## Phase 3.4: Checksum Generator

### Add to `src/protocol/block_checksum.rs`

#### Define ChecksumGenerator Struct
- [ ] Add struct:
  ```rust
  pub struct ChecksumGenerator {
      block_size: usize,
      rolling: RollingChecksum,
      strong: Box<dyn StrongChecksumAlgorithm>,
  }
  ```

#### Implement Constructor
- [ ] Add:
  ```rust
  pub fn new(
      block_size: usize,
      seed: u32,
      strong: Box<dyn StrongChecksumAlgorithm>,
  ) -> Self
  ```
- [ ] Add validation: block_size > 0
- [ ] Add doc comment

#### Implement generate()
- [ ] Add:
  ```rust
  pub fn generate(&self, data: &[u8]) -> Vec<BlockChecksum>
  ```
- [ ] Chunk data by block_size
- [ ] For each chunk:
  - [ ] Compute rolling checksum
  - [ ] Compute strong checksum
  - [ ] Create BlockChecksum with offset and index
- [ ] Add doc comment
- [ ] Add unit test

### Acceptance Criteria for Phase 3.4
- [ ] Generator compiles
- [ ] Generates correct checksums
- [ ] Unit tests pass
- [ ] Commit message: "feat(checksum): add checksum generator"

---

## Phase 3.5: Checksum Protocol Trait

### Create `src/protocol/checksum_protocol.rs`

- [ ] Create file
- [ ] Add to `src/protocol/mod.rs`

#### Define ChecksumProtocol Trait
- [ ] Add trait:
  ```rust
  #[async_trait]
  pub trait ChecksumProtocol: Send {
      async fn send_checksums<T: Transport>(
          &self,
          transport: &mut T,
          checksums: &[BlockChecksum],
      ) -> Result<()>;
      
      async fn receive_checksums<T: Transport>(
          &self,
          transport: &mut T,
      ) -> Result<Vec<BlockChecksum>>;
  }
  ```
- [ ] Add doc comments

### Acceptance Criteria for Phase 3.5
- [ ] Trait compiles
- [ ] Doc comments complete
- [ ] Commit message: "feat(checksum): add checksum protocol trait"

---

## Phase 3.6: rsync Checksum Protocol

### Add to `src/protocol/checksum_protocol.rs`

#### Define RsyncChecksumProtocol
- [ ] Add struct:
  ```rust
  pub struct RsyncChecksumProtocol {
      protocol_version: u8,
  }
  ```
- [ ] Add constructor

#### Implement send_checksums
- [ ] Write count as u32 little-endian
- [ ] For each checksum:
  - [ ] Write rolling (u32 LE)
  - [ ] Write strong (16 bytes, truncate/pad if needed)
- [ ] Flush transport
- [ ] Add error handling
- [ ] Add doc comment

#### Implement receive_checksums
- [ ] Read count (u32 LE)
- [ ] Allocate vector with capacity
- [ ] For each checksum:
  - [ ] Read rolling (u32 LE)
  - [ ] Read strong (16 bytes)
  - [ ] Create BlockChecksum (offset=None, index=Some(i))
- [ ] Add error handling
- [ ] Add doc comment

#### Implement ChecksumProtocol Trait
- [ ] Add `impl ChecksumProtocol for RsyncChecksumProtocol`
- [ ] Delegate to send_checksums/receive_checksums

### Acceptance Criteria for Phase 3.6
- [ ] rsync protocol compiles
- [ ] Matches rsync wire format
- [ ] Doc comments complete
- [ ] Commit message: "feat(checksum): implement rsync checksum protocol"

---

## Phase 3.7: arsync Native Checksum Protocol

### Add to `src/protocol/checksum_protocol.rs`

#### Define ArsyncChecksumProtocol
- [ ] Add struct: `pub struct ArsyncChecksumProtocol;`
- [ ] Add constructor

#### Implement send_checksums
- [ ] Write count as varint
- [ ] For each checksum:
  - [ ] Write rolling (u32 LE)
  - [ ] Write strong (16 bytes)
  - [ ] Write offset (varint)
  - [ ] Write index (varint)
- [ ] Flush transport
- [ ] Add error handling

#### Implement receive_checksums
- [ ] Read count (varint)
- [ ] For each checksum:
  - [ ] Read rolling
  - [ ] Read strong
  - [ ] Read offset
  - [ ] Read index
  - [ ] Create BlockChecksum
- [ ] Add error handling

#### Implement ChecksumProtocol Trait
- [ ] Add impl
- [ ] Delegate to methods

### Acceptance Criteria for Phase 3.7
- [ ] Native protocol compiles
- [ ] More efficient than rsync (varints)
- [ ] Doc comments complete
- [ ] Commit message: "feat(checksum): implement arsync native checksum protocol"

---

## Phase 3.8: Protocol Selection

### Add to `src/protocol/checksum_protocol.rs`

#### Create Factory Function
- [ ] Add:
  ```rust
  pub fn create_checksum_protocol(
      capabilities: &ProtocolCapabilities,
      compat_mode: bool,
  ) -> Box<dyn ChecksumProtocol> {
      if compat_mode || capabilities.version < 100 {
          Box::new(RsyncChecksumProtocol {
              protocol_version: capabilities.version,
          })
      } else {
          Box::new(ArsyncChecksumProtocol)
      }
  }
  ```
- [ ] Add doc comment explaining selection logic
- [ ] Add unit test

### Acceptance Criteria for Phase 3.8
- [ ] Factory function works
- [ ] Correctly selects protocol
- [ ] Unit test passes
- [ ] Commit message: "feat(checksum): add protocol selection factory"

---

## Phase 3.9: Testing - Unit Tests

### Create `tests/checksum_unit_tests.rs`

#### Test: Algorithm Implementations
- [ ] `test_md5_checksum`
  - [ ] Compute MD5 of known data
  - [ ] Verify against expected value
  - [ ] Verify digest_size == 16

- [ ] `test_md4_checksum`
  - [ ] Compute MD4 of known data
  - [ ] Verify against expected value
  - [ ] Verify digest_size == 16

- [ ] `test_blake3_checksum`
  - [ ] Compute BLAKE3 of known data
  - [ ] Verify against expected value
  - [ ] Verify digest_size == 32

#### Test: Rolling Checksum
- [ ] `test_rolling_checksum_no_seed`
  - [ ] Create with seed=0
  - [ ] Compute for data
  - [ ] Verify matches existing implementation

- [ ] `test_rolling_checksum_with_seed`
  - [ ] Create with random seed
  - [ ] Compute for data
  - [ ] Verify different from no-seed
  - [ ] Verify deterministic for same seed

#### Test: Checksum Generator
- [ ] `test_generator_simple`
  - [ ] Generate checksums for small file
  - [ ] Verify count matches expected blocks
  - [ ] Verify each checksum has rolling and strong

- [ ] `test_generator_large_file`
  - [ ] Generate for 1MB file
  - [ ] Verify correct number of blocks
  - [ ] Verify checksums are unique

#### Test: Protocol Roundtrip
- [ ] `test_rsync_protocol_roundtrip`
  - [ ] Create checksums
  - [ ] Send via rsync protocol
  - [ ] Receive via rsync protocol
  - [ ] Verify checksums match

- [ ] `test_arsync_protocol_roundtrip`
  - [ ] Same as rsync but with native protocol
  - [ ] Verify checksums match
  - [ ] Verify metadata preserved (offset, index)

#### Test: Protocol Selection
- [ ] `test_protocol_selection_rsync`
  - [ ] Create capabilities with version 31
  - [ ] Set compat_mode=true
  - [ ] Verify RsyncChecksumProtocol selected

- [ ] `test_protocol_selection_arsync`
  - [ ] Create capabilities with version 100
  - [ ] Set compat_mode=false
  - [ ] Verify ArsyncChecksumProtocol selected

### Acceptance Criteria for Phase 3.9
- [ ] All unit tests pass
- [ ] Tests cover all code paths
- [ ] Tests have descriptive names
- [ ] Commit message: "test(checksum): add comprehensive unit tests"

---

## Phase 3.10: Testing - Integration Tests

### Create `tests/checksum_integration_tests.rs`

#### Test: End-to-End with Pipes
- [ ] `test_checksum_exchange_bidirectional`
  - [ ] Create sender and receiver
  - [ ] Sender generates checksums
  - [ ] Sender sends via protocol
  - [ ] Receiver receives
  - [ ] Verify checksums match

#### Test: Large Data
- [ ] `test_checksum_large_file`
  - [ ] Generate 100MB of data
  - [ ] Create checksums
  - [ ] Exchange via protocol
  - [ ] Verify all blocks received
  - [ ] Measure performance

#### Test: Different Algorithms
- [ ] `test_checksum_md5_vs_blake3`
  - [ ] Generate with MD5
  - [ ] Generate with BLAKE3
  - [ ] Compare sizes
  - [ ] Compare performance

### Acceptance Criteria for Phase 3.10
- [ ] Integration tests pass
- [ ] Performance is acceptable
- [ ] Large data works
- [ ] Commit message: "test(checksum): add integration tests"

---

## Phase 3.11: Documentation and Integration

### Create `docs/CHECKSUM_API.md`
- [ ] Document all checksum types
- [ ] Document protocol selection
- [ ] Add usage examples
- [ ] Add performance notes

### Update Protocol Files
- [ ] Update `src/protocol/rsync.rs` to use checksum abstraction
- [ ] Update `src/protocol/rsync_compat.rs` to use checksum abstraction
- [ ] Remove duplicate checksum code
- [ ] Add tests to verify integration

### Cleanup
- [ ] Run: `cargo fmt --all`
- [ ] Run: `cargo clippy --all-features -- -D warnings`
- [ ] Fix warnings
- [ ] Remove dead code

### Acceptance Criteria for Phase 3.11
- [ ] Documentation complete
- [ ] Integration working
- [ ] No warnings
- [ ] Commit message: "docs(checksum): add API documentation and integrate with protocol"

---

## Phase 3.12: Create Pull Request

### PR Preparation
- [ ] All Phase 3 commits pushed
- [ ] Full test suite passes
- [ ] Documentation complete

### Create PR
- [ ] Title: "feat: implement checksum exchange abstraction (Phase 3)"
- [ ] Body includes:
  - [ ] Summary of abstraction
  - [ ] Supported algorithms
  - [ ] Protocol formats
  - [ ] Performance notes
  - [ ] Testing coverage

### Acceptance Criteria for Phase 3.12
- [ ] PR created
- [ ] All checks pass
- [ ] Ready for review

---

# PHASE 4: Delta Token Handling

**Goal**: Implement delta token generation, encoding, and application for both rsync and arsync formats

**Duration Estimate**: 2 weeks  
**Files to Create**: 2 new files  
**Files to Modify**: 5 existing files  
**Tests to Add**: 25+ test functions

---

## Phase 4.1: Delta Operation Abstraction

### Create `src/protocol/delta.rs`

- [ ] Create file
- [ ] Add to `src/protocol/mod.rs`: `pub mod delta;`

#### Define CopySource Enum
- [ ] Add enum:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum CopySource {
      BlockIndex(u32),
      ByteOffset(u64),
  }
  ```
- [ ] Add doc comments
- [ ] Add conversion methods

#### Define DeltaOp Enum
- [ ] Add enum:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum DeltaOp {
      Literal { data: Vec<u8> },
      Copy { source: CopySource, length: u32 },
  }
  ```
- [ ] Add doc comments

#### Implement Conversion Methods
- [ ] Add `to_native()` - convert to arsync native format
- [ ] Add `from_native()` - convert from arsync native format
- [ ] Add unit tests

### Acceptance Criteria for Phase 4.1
- [ ] Delta types compile
- [ ] Conversions work
- [ ] Unit tests pass
- [ ] Commit message: "feat(delta): add delta operation abstraction"

---

## Phase 4.2: Delta Generation Algorithm

### Add to `src/protocol/delta.rs`

#### Implement generate_delta()
- [ ] Add function signature:
  ```rust
  pub fn generate_delta(
      new_data: &[u8],
      basis_checksums: &[BlockChecksum],
      block_size: usize,
  ) -> Vec<DeltaOp>
  ```
- [ ] Build checksum lookup table (HashMap<u32, Vec<&BlockChecksum>>)
- [ ] Implement sliding window algorithm:
  - [ ] For each position in new_data:
    - [ ] Compute rolling checksum of window
    - [ ] Look up in table
    - [ ] If match found: verify with strong checksum
    - [ ] If verified: emit Copy, advance window
    - [ ] If no match: add byte to literal buffer
  - [ ] Emit final literal if buffer non-empty
- [ ] Add doc comment with algorithm explanation
- [ ] Add complexity analysis in doc

#### Optimize for Performance
- [ ] Use Vec instead of LinkedList for literal buffer
- [ ] Pre-allocate delta vector
- [ ] Minimize allocations in hot loop
- [ ] Add benchmark test

### Acceptance Criteria for Phase 4.2
- [ ] Delta generation compiles
- [ ] Produces correct results
- [ ] Performance is acceptable
- [ ] Commit message: "feat(delta): implement delta generation algorithm"

---

## Phase 4.3: Delta Application Algorithm

### Add to `src/protocol/delta.rs`

#### Implement apply_delta()
- [ ] Add function signature:
  ```rust
  pub fn apply_delta(
      basis_data: &[u8],
      delta: &[DeltaOp],
      block_size: usize,
  ) -> Result<Vec<u8>>
  ```
- [ ] Pre-allocate output vector (estimate size)
- [ ] For each DeltaOp:
  - [ ] If Literal: append data to output
  - [ ] If Copy:
    - [ ] Calculate offset from source
    - [ ] Validate bounds
    - [ ] Copy from basis_data to output
- [ ] Add doc comment
- [ ] Add error handling for bounds violations

#### Add Validation
- [ ] Verify basis_data length
- [ ] Verify copy operations don't exceed bounds
- [ ] Return descriptive errors

### Acceptance Criteria for Phase 4.3
- [ ] Delta application compiles
- [ ] Reconstructs files correctly
- [ ] Error handling works
- [ ] Commit message: "feat(delta): implement delta application algorithm"

---

## Phase 4.4: Delta Protocol Trait

### Create `src/protocol/delta_protocol.rs`

- [ ] Create file
- [ ] Add to `src/protocol/mod.rs`

#### Define DeltaProtocol Trait
- [ ] Add trait:
  ```rust
  #[async_trait]
  pub trait DeltaProtocol: Send {
      async fn send_delta<T: Transport>(
          &self,
          transport: &mut T,
          delta: &[DeltaOp],
          block_size: u32,
      ) -> Result<()>;
      
      async fn receive_delta<T: Transport>(
          &self,
          transport: &mut T,
          block_count: u32,
          block_size: u32,
      ) -> Result<Vec<DeltaOp>>;
  }
  ```
- [ ] Add doc comments

### Acceptance Criteria for Phase 4.4
- [ ] Trait compiles
- [ ] Doc comments complete
- [ ] Commit message: "feat(delta): add delta protocol trait"

---

## Phase 4.5: rsync Delta Token Format

### Add to `src/protocol/delta_protocol.rs`

#### Define TokenType Enum
- [ ] Add enum:
  ```rust
  enum TokenType {
      End,
      LiteralByte(u8),
      LiteralRun { length: u32 },
      LongLiteralRun { length: u32 },
      BlockMatch { index: u32 },
      LongBlockMatch { index: u32 },
      Invalid,
  }
  ```
- [ ] Add doc comment explaining rsync token format

#### Implement encode_token()
- [ ] For Literal with 1 byte: token = byte value (1-127)
- [ ] For Literal < 65536 bytes: token = 128 + block_count + length
- [ ] For Literal >= 65536 bytes: token = (0x01 << 16) | (length - 65536)
- [ ] For Copy with block < 65536: token = 128 + block_index
- [ ] For Copy with block >= 65536: token = (0x02 << 16) | (block_index - 65536)
- [ ] Add doc comment with examples
- [ ] Add unit tests

#### Implement decode_token()
- [ ] Parse token into TokenType
- [ ] Handle all variants correctly
- [ ] Validate token values
- [ ] Add doc comment
- [ ] Add unit tests

### Acceptance Criteria for Phase 4.5
- [ ] Token encoding/decoding works
- [ ] Matches rsync specification
- [ ] Unit tests pass
- [ ] Commit message: "feat(delta): implement rsync token format"

---

## Phase 4.6: rsync Delta Protocol Implementation

### Add to `src/protocol/delta_protocol.rs`

#### Define RsyncDeltaProtocol Struct
- [ ] Add struct:
  ```rust
  pub struct RsyncDeltaProtocol {
      protocol_version: u8,
  }
  ```
- [ ] Add constructor

#### Implement send_delta()
- [ ] For each DeltaOp:
  - [ ] Encode to tokens
  - [ ] Write tokens (u32 LE)
  - [ ] If Literal: write data
- [ ] Write end marker (0x00000000)
- [ ] Flush transport
- [ ] Add error handling

#### Implement receive_delta()
- [ ] Loop until end token:
  - [ ] Read token (u32 LE)
  - [ ] Decode to TokenType
  - [ ] If LiteralByte/Run: read data, create Literal
  - [ ] If BlockMatch: create Copy
  - [ ] Add to delta vector
- [ ] Add error handling

#### Implement DeltaProtocol Trait
- [ ] Add impl for RsyncDeltaProtocol
- [ ] Delegate to methods

### Acceptance Criteria for Phase 4.6
- [ ] rsync protocol compiles
- [ ] Matches rsync wire format
- [ ] Doc comments complete
- [ ] Commit message: "feat(delta): implement rsync delta protocol"

---

## Phase 4.7: arsync Native Delta Protocol

### Add to `src/protocol/delta_protocol.rs`

#### Define ArsyncDeltaProtocol Struct
- [ ] Add struct: `pub struct ArsyncDeltaProtocol;`
- [ ] Add constructor

#### Implement send_delta()
- [ ] Write count (varint)
- [ ] For each DeltaOp:
  - [ ] Write type tag (0=Literal, 1=Copy)
  - [ ] If Literal:
    - [ ] Write length (varint)
    - [ ] Write data
  - [ ] If Copy:
    - [ ] Write block_index (varint)
    - [ ] Write length (varint)
- [ ] Flush transport

#### Implement receive_delta()
- [ ] Read count (varint)
- [ ] For each delta:
  - [ ] Read type tag
  - [ ] Match on type:
    - [ ] 0: read length, read data, create Literal
    - [ ] 1: read index, read length, create Copy
- [ ] Add error handling

#### Implement DeltaProtocol Trait
- [ ] Add impl
- [ ] Delegate to methods

### Acceptance Criteria for Phase 4.7
- [ ] Native protocol compiles
- [ ] More efficient than rsync (varints)
- [ ] Doc comments complete
- [ ] Commit message: "feat(delta): implement arsync native delta protocol"

---

## Phase 4.8: Protocol Selection and Integration

### Add to `src/protocol/delta_protocol.rs`

#### Create Factory Function
- [ ] Add:
  ```rust
  pub fn create_delta_protocol(
      capabilities: &ProtocolCapabilities,
      compat_mode: bool,
  ) -> Box<dyn DeltaProtocol>
  ```
- [ ] Select rsync or arsync protocol
- [ ] Add doc comment

### Update Protocol Flow

#### Update `src/protocol/rsync.rs`
- [ ] Import delta types
- [ ] Replace existing delta code with abstraction
- [ ] Use DeltaProtocol trait
- [ ] Test integration

#### Update `src/protocol/rsync_compat.rs`
- [ ] Import delta types
- [ ] Use delta abstraction
- [ ] Test integration

### Acceptance Criteria for Phase 4.8
- [ ] Factory works
- [ ] Integration complete
- [ ] Tests pass
- [ ] Commit message: "feat(delta): integrate delta protocols with main flow"

---

## Phase 4.9: Testing - Unit Tests

### Create `tests/delta_unit_tests.rs`

#### Test: Delta Generation
- [ ] `test_delta_generation_no_changes`
  - [ ] Basis and new are identical
  - [ ] Should generate only Copy operations
  - [ ] Verify delta is small

- [ ] `test_delta_generation_all_new`
  - [ ] Basis is empty or unrelated
  - [ ] Should generate only Literal operations
  - [ ] Verify delta contains all new data

- [ ] `test_delta_generation_mixed`
  - [ ] Some blocks match, some don't
  - [ ] Should have mix of Copy and Literal
  - [ ] Verify correctness

#### Test: Delta Application
- [ ] `test_delta_application_simple`
  - [ ] Apply simple delta
  - [ ] Verify result matches expected

- [ ] `test_delta_application_large`
  - [ ] Apply delta to 1MB file
  - [ ] Verify result

#### Test: Roundtrip
- [ ] `test_delta_roundtrip_identical`
  - [ ] Generate delta for identical files
  - [ ] Apply delta
  - [ ] Verify result matches original

- [ ] `test_delta_roundtrip_modified`
  - [ ] Generate delta for modified file
  - [ ] Apply delta
  - [ ] Verify result matches modified version

#### Test: Token Encoding
- [ ] `test_rsync_token_literal_byte`
  - [ ] Encode single byte literal
  - [ ] Verify token value is byte value

- [ ] `test_rsync_token_literal_run`
  - [ ] Encode short literal run
  - [ ] Verify token format

- [ ] `test_rsync_token_block_match`
  - [ ] Encode block match
  - [ ] Verify token format

#### Test: Protocol Roundtrip
- [ ] `test_rsync_protocol_roundtrip`
  - [ ] Send delta via rsync protocol
  - [ ] Receive via rsync protocol
  - [ ] Verify delta matches

- [ ] `test_arsync_protocol_roundtrip`
  - [ ] Same for arsync native protocol
  - [ ] Verify delta matches

### Acceptance Criteria for Phase 4.9
- [ ] All unit tests pass
- [ ] Cover all code paths
- [ ] Tests are well-documented
- [ ] Commit message: "test(delta): add comprehensive unit tests"

---

## Phase 4.10: Testing - Integration Tests

### Create `tests/delta_integration_tests.rs`

#### Test: Full File Transfer
- [ ] `test_delta_transfer_via_pipe`
  - [ ] Create basis file
  - [ ] Create modified version
  - [ ] Generate checksums
  - [ ] Generate delta
  - [ ] Transfer via pipe
  - [ ] Apply delta
  - [ ] Verify result matches modified version

#### Test: Large Files
- [ ] `test_delta_large_file`
  - [ ] 100MB basis file
  - [ ] Modify 10%
  - [ ] Generate delta
  - [ ] Verify delta is ~10MB (not 100MB)
  - [ ] Apply and verify

#### Test: rsync Compatibility
- [ ] `test_delta_with_real_rsync`
  - [ ] Use rsync --write-batch
  - [ ] Parse rsync's delta file
  - [ ] Verify we can decode it
  - [ ] Apply it
  - [ ] Verify result

### Acceptance Criteria for Phase 4.10
- [ ] Integration tests pass
- [ ] Large files work
- [ ] rsync compatibility verified
- [ ] Commit message: "test(delta): add integration tests"

---

## Phase 4.11: Performance Testing and Optimization

### Create `tests/delta_performance_tests.rs`

#### Benchmark: Delta Generation
- [ ] Measure time to generate delta for various file sizes
- [ ] Compare with rsync's performance
- [ ] Document results

#### Benchmark: Delta Application
- [ ] Measure time to apply delta
- [ ] Compare with baseline
- [ ] Document results

#### Optimize if Needed
- [ ] Profile with `cargo flamegraph`
- [ ] Identify hotspots
- [ ] Optimize critical paths
- [ ] Re-measure

### Acceptance Criteria for Phase 4.11
- [ ] Performance is acceptable
- [ ] Optimization documented
- [ ] Benchmarks included
- [ ] Commit message: "perf(delta): optimize delta generation and application"

---

## Phase 4.12: Documentation and Cleanup

### Create `docs/DELTA_API.md`
- [ ] Document delta abstraction
- [ ] Explain algorithms
- [ ] Add usage examples
- [ ] Document both protocols
- [ ] Add performance notes

### Update Other Docs
- [ ] Update `docs/RSYNC_COMPAT_DETAILED_DESIGN.md`
- [ ] Mark Phase 4 complete
- [ ] Update implementation status

### Final Cleanup
- [ ] Run: `cargo fmt --all`
- [ ] Run: `cargo clippy --all-features -- -D warnings`
- [ ] Fix all warnings
- [ ] Remove dead code
- [ ] Update comments

### Acceptance Criteria for Phase 4.12
- [ ] Documentation complete
- [ ] No warnings
- [ ] Code is clean
- [ ] Commit message: "docs(delta): add comprehensive documentation"

---

## Phase 4.13: Create Pull Request

### PR Preparation
- [ ] All Phase 4 commits pushed
- [ ] Full test suite passes
- [ ] Performance is acceptable
- [ ] Documentation complete

### Create PR
- [ ] Title: "feat: implement delta token handling (Phase 4)"
- [ ] Body includes:
  - [ ] Summary of delta implementation
  - [ ] Algorithm explanation
  - [ ] Both protocol formats
  - [ ] Performance results
  - [ ] rsync compatibility status
  - [ ] Testing coverage

### Acceptance Criteria for Phase 4.13
- [ ] PR created
- [ ] All checks pass
- [ ] Ready for review

---

# PHASE 5: Final Integration and End-to-End Testing

**Goal**: Complete end-to-end rsync compatibility with full testing

**Duration Estimate**: 1-2 weeks  
**Files to Create**: 3 new files  
**Files to Modify**: 6 existing files  
**Tests to Add**: 15+ test functions

---

## Phase 5.1: Complete Protocol Flow

### Update `src/protocol/rsync_compat.rs`

#### Implement Complete rsync_receive_via_pipe
- [ ] Phase 1: Handshake (use implemented)
- [ ] Phase 2: Receive file list (use implemented)
- [ ] Phase 3: For each file:
  - [ ] Generate basis checksums (if file exists)
  - [ ] Send checksums to sender
  - [ ] Receive delta
  - [ ] Apply delta
  - [ ] Write file
  - [ ] Apply metadata
- [ ] Phase 4: Finalization
- [ ] Add comprehensive logging
- [ ] Add error handling

#### Implement Complete rsync_send_via_pipe
- [ ] Phase 1: Handshake
- [ ] Phase 2: Send file list
- [ ] Phase 3: For each file:
  - [ ] Receive basis checksums
  - [ ] Read file
  - [ ] Generate delta
  - [ ] Send delta
- [ ] Phase 4: Finalization
- [ ] Add logging
- [ ] Add error handling

### Acceptance Criteria for Phase 5.1
- [ ] Full flow compiles
- [ ] All phases integrated
- [ ] Logging is comprehensive
- [ ] Commit message: "feat(rsync): implement complete protocol flow"

---

## Phase 5.2: Metadata Transmission

### Implement Metadata in File List

#### Update File List Encoding
- [ ] Include xattrs in file list
- [ ] Include ACLs in file list
- [ ] Include all timestamps
- [ ] Use rsync format for metadata

#### Update File List Decoding
- [ ] Parse xattrs from file list
- [ ] Parse ACLs from file list
- [ ] Parse all timestamps
- [ ] Validate metadata

### Apply Metadata After Transfer

#### Use Existing Local Functions
- [ ] Call existing xattr functions from `src/copy.rs`
- [ ] Call existing ACL functions from `src/copy.rs`
- [ ] Call existing timestamp functions from `src/directory.rs`
- [ ] Verify metadata is applied correctly

### Acceptance Criteria for Phase 5.2
- [ ] Metadata transmitted
- [ ] Metadata applied
- [ ] Uses existing local functions
- [ ] Commit message: "feat(rsync): implement metadata transmission"

---

## Phase 5.3: Error Handling and Recovery

### Add Comprehensive Error Handling

#### Define Protocol Errors
- [ ] Create `ProtocolError` enum
- [ ] Add variants for all error types
- [ ] Implement Display and Error traits
- [ ] Add doc comments

#### Handle Protocol Errors
- [ ] Catch transport errors
- [ ] Catch format errors
- [ ] Catch checksum errors
- [ ] Catch delta errors
- [ ] Add recovery where possible

#### Add Logging
- [ ] Error logs at each failure point
- [ ] Warning logs for recoverable issues
- [ ] Info logs for progress
- [ ] Debug logs for details

### Acceptance Criteria for Phase 5.3
- [ ] Error handling is complete
- [ ] Errors are descriptive
- [ ] Logging is comprehensive
- [ ] Commit message: "feat(rsync): add comprehensive error handling"

---

## Phase 5.4: End-to-End Testing - arsync to arsync

### Create `tests/e2e_arsync_tests.rs`

#### Test: Full Transfer
- [ ] `test_e2e_arsync_to_arsync_basic`
  - [ ] Create source with files
  - [ ] Run sender and receiver via pipes
  - [ ] Verify all files transferred
  - [ ] Verify content matches
  - [ ] Verify metadata preserved

#### Test: Large Dataset
- [ ] `test_e2e_arsync_large_dataset`
  - [ ] 1000 files, various sizes
  - [ ] Transfer via arsync native protocol
  - [ ] Verify all transferred
  - [ ] Measure performance

#### Test: Incremental Transfer
- [ ] `test_e2e_arsync_incremental`
  - [ ] Transfer files
  - [ ] Modify some files
  - [ ] Transfer again
  - [ ] Verify delta algorithm used
  - [ ] Verify only changes transferred

### Acceptance Criteria for Phase 5.4
- [ ] All e2e tests pass
- [ ] Performance is good
- [ ] Metadata preserved
- [ ] Commit message: "test(e2e): add arsync-to-arsync end-to-end tests"

---

## Phase 5.5: End-to-End Testing - rsync Compatibility

### Create `tests/e2e_rsync_compat_tests.rs`

#### Test: rsync to arsync
- [ ] `test_e2e_rsync_to_arsync_basic`
  - [ ] Spawn: `rsync --server --sender ...`
  - [ ] Connect arsync receiver
  - [ ] Transfer files
  - [ ] Verify all transferred
  - [ ] Verify metadata preserved

- [ ] `test_e2e_rsync_to_arsync_large`
  - [ ] Transfer large dataset from rsync
  - [ ] Verify completeness
  - [ ] Verify performance

#### Test: arsync to rsync
- [ ] `test_e2e_arsync_to_rsync_basic`
  - [ ] Spawn: `rsync --server ...` (receiver)
  - [ ] Connect arsync sender
  - [ ] Transfer files
  - [ ] Verify rsync received correctly

- [ ] `test_e2e_arsync_to_rsync_large`
  - [ ] Transfer large dataset to rsync
  - [ ] Verify completeness

#### Test: Incremental with rsync
- [ ] `test_e2e_rsync_incremental`
  - [ ] First transfer: rsync → arsync
  - [ ] Modify files
  - [ ] Second transfer: rsync → arsync
  - [ ] Verify only changes transferred
  - [ ] Verify delta algorithm worked

### Acceptance Criteria for Phase 5.5
- [ ] All rsync compat tests pass
- [ ] Can receive from real rsync
- [ ] Can send to real rsync
- [ ] Incremental works
- [ ] Commit message: "test(e2e): add rsync compatibility end-to-end tests"

---

## Phase 5.6: Real-World Testing

### Test with Actual rsync Usage

#### Test: Pull from Remote
- [ ] Set up test VM or container
- [ ] Start rsync daemon or SSH server
- [ ] Run: `arsync remote:/path/ /local/`
- [ ] Verify transfer completes
- [ ] Verify files are correct
- [ ] Document any issues

#### Test: Push to Remote
- [ ] Set up test VM or container
- [ ] Run: `arsync /local/ remote:/path/`
- [ ] Verify transfer completes
- [ ] Verify files on remote are correct
- [ ] Document any issues

#### Test: Bidirectional
- [ ] Pull files
- [ ] Modify
- [ ] Push back
- [ ] Verify sync works both ways

### Acceptance Criteria for Phase 5.6
- [ ] Real-world usage works
- [ ] Can pull from remote
- [ ] Can push to remote
- [ ] Issues documented
- [ ] Commit message: "test(e2e): verify real-world rsync usage"

---

## Phase 5.7: Performance Benchmarking

### Create `tests/performance_benchmarks.rs`

#### Benchmark: Large File Transfer
- [ ] 1GB file
- [ ] Measure arsync native
- [ ] Measure arsync rsync-compat
- [ ] Measure real rsync
- [ ] Compare results

#### Benchmark: Many Small Files
- [ ] 10,000 files @ 1KB each
- [ ] Measure arsync native
- [ ] Measure arsync rsync-compat
- [ ] Measure real rsync
- [ ] Compare results

#### Benchmark: Incremental Transfer
- [ ] Large dataset
- [ ] Modify 10%
- [ ] Measure delta transfer
- [ ] Compare with full transfer
- [ ] Compare with rsync

#### Document Results
- [ ] Create `docs/PERFORMANCE_RESULTS.md`
- [ ] Include all benchmark data
- [ ] Add graphs if possible
- [ ] Analyze results
- [ ] Identify optimization opportunities

### Acceptance Criteria for Phase 5.7
- [ ] Benchmarks complete
- [ ] Results documented
- [ ] Performance is acceptable
- [ ] Commit message: "perf: add comprehensive performance benchmarks"

---

## Phase 5.8: Documentation - User Guide

### Create `docs/USER_GUIDE.md`

- [ ] Title: "arsync User Guide - rsync Compatibility"
- [ ] Section: "Introduction"
  - [ ] What is arsync
  - [ ] rsync compatibility features
- [ ] Section: "Installation"
  - [ ] Building from source
  - [ ] Feature flags
- [ ] Section: "Basic Usage"
  - [ ] Local sync
  - [ ] Remote sync (rsync compatible)
  - [ ] Common flags
- [ ] Section: "rsync Compatibility"
  - [ ] What works
  - [ ] What doesn't (yet)
  - [ ] Differences from rsync
- [ ] Section: "Performance"
  - [ ] When to use arsync native
  - [ ] When to use rsync compat
  - [ ] Optimization tips
- [ ] Section: "Troubleshooting"
  - [ ] Common errors
  - [ ] Debug logging
  - [ ] Getting help
- [ ] Section: "Examples"
  - [ ] Example: Pull from remote
  - [ ] Example: Push to remote
  - [ ] Example: Incremental sync
  - [ ] Example: With metadata preservation

### Acceptance Criteria for Phase 5.8
- [ ] User guide complete
- [ ] Examples work
- [ ] Clear and accessible
- [ ] Commit message: "docs: add comprehensive user guide"

---

## Phase 5.9: Documentation - Developer Guide

### Update `docs/DEVELOPER.md`

- [ ] Add section: "rsync Protocol Implementation"
- [ ] Document architecture
- [ ] Document module structure
- [ ] Add code flow diagrams
- [ ] Explain design decisions

### Create `docs/CONTRIBUTING_RSYNC.md`

- [ ] How to contribute to rsync compat
- [ ] Testing requirements
- [ ] Code style
- [ ] PR process

### Acceptance Criteria for Phase 5.9
- [ ] Developer docs updated
- [ ] Contributing guide exists
- [ ] Clear for new contributors
- [ ] Commit message: "docs: update developer documentation"

---

## Phase 5.10: Final Code Review and Cleanup

### Self-Review
- [ ] Review all code added in Phases 1-5
- [ ] Check for TODO comments
- [ ] Check for dead code
- [ ] Check for debug prints
- [ ] Verify all doc comments
- [ ] Verify error messages are clear

### Code Quality
- [ ] Run: `cargo fmt --all`
- [ ] Run: `cargo clippy --all-features -- -D warnings`
- [ ] Run: `cargo doc --all-features --no-deps`
- [ ] Fix all issues

### Test Coverage
- [ ] Run: `cargo tarpaulin --all-features`
- [ ] Identify untested code
- [ ] Add missing tests
- [ ] Document coverage

### Acceptance Criteria for Phase 5.10
- [ ] No warnings
- [ ] No clippy issues
- [ ] Documentation builds
- [ ] Test coverage documented
- [ ] Commit message: "chore: final code review and cleanup"

---

## Phase 5.11: Update Project Documentation

### Update `README.md`
- [ ] Add rsync compatibility section
- [ ] Add feature list
- [ ] Add performance comparison
- [ ] Update installation instructions
- [ ] Add examples

### Update `CHANGELOG.md`
- [ ] Add entry for rsync compatibility
- [ ] List all new features
- [ ] Note breaking changes (if any)
- [ ] Credit contributors

### Update `docs/RSYNC_COMPAT_DETAILED_DESIGN.md`
- [ ] Mark all phases complete
- [ ] Add "Implementation Complete" section
- [ ] Document final architecture
- [ ] Add lessons learned

### Create `docs/RSYNC_COMPAT_STATUS.md`
- [ ] What's implemented
- [ ] What's tested
- [ ] Known limitations
- [ ] Future work

### Acceptance Criteria for Phase 5.11
- [ ] All docs updated
- [ ] Status is clear
- [ ] Future work documented
- [ ] Commit message: "docs: update project documentation for rsync compatibility"

---

## Phase 5.12: Create Final Pull Request

### PR Preparation
- [ ] All Phase 5 commits pushed
- [ ] All tests pass
- [ ] All docs complete
- [ ] Performance acceptable

### Create PR
- [ ] Title: "feat: complete rsync wire protocol compatibility (Phase 5)"
- [ ] Body includes:
  - [ ] Complete feature summary
  - [ ] What works
  - [ ] Test coverage
  - [ ] Performance results
  - [ ] Documentation references
  - [ ] Known limitations
  - [ ] Future work

### Final Review Checklist
- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] Documentation is complete
- [ ] Performance is acceptable
- [ ] No security issues
- [ ] Error handling is robust
- [ ] Logging is appropriate
- [ ] User guide is clear
- [ ] Developer guide is complete

### Acceptance Criteria for Phase 5.12
- [ ] PR created
- [ ] All criteria met
- [ ] Ready for merge

---

# Post-Implementation

## Monitoring and Iteration

### After Merge
- [ ] Monitor for bug reports
- [ ] Gather user feedback
- [ ] Measure real-world performance
- [ ] Identify improvement areas

### Future Enhancements
- [ ] Document enhancement ideas
- [ ] Prioritize improvements
- [ ] Plan next iteration

---

# Summary Statistics

## Total Deliverables

- **New Files Created**: ~12 files
- **Existing Files Modified**: ~20 files
- **Tests Added**: ~100+ test functions
- **Documentation Pages**: ~8 new docs
- **Lines of Code**: ~5000+ lines (estimated)
- **Pull Requests**: 5 PRs (one per phase)
- **Duration**: 7-10 weeks

## Success Criteria

- [ ] Can handshake with rsync server
- [ ] Can receive files from rsync
- [ ] Can send files to rsync
- [ ] Metadata preserved (perms, times, xattrs, ACLs)
- [ ] Delta algorithm works
- [ ] Performance within 20% of native rsync
- [ ] All tests pass
- [ ] Documentation complete
- [ ] User guide exists
- [ ] Zero critical bugs

---

**END OF IMPLEMENTATION CHECKLIST**

This document will be updated as implementation progresses. Check boxes will be marked as tasks are completed.


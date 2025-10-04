# Implementation Plan

This document outlines the detailed implementation plan for io-uring-sync, including phases, deliverables, acceptance criteria, and testing requirements.

## Table of Contents

- [Architecture Decisions](#architecture-decisions)
  - [Recommended Technology Stack](#recommended-technology-stack)
  - [Key Design Decisions](#key-design-decisions)
    - [Channel Blocking vs Async Compatibility](#channel-blocking-vs-async-compatibility)
    - [Direct liburing Bindings Feasibility](#direct-liburing-bindings-feasibility)
    - [io_uring xattr Support](#io_uring-xattr-support)
- [Development Workflow Requirements](#development-workflow-requirements)
  - [Phase Completion Checklist](#phase-completion-checklist)
  - [Stacked PR Workflow](#stacked-pr-workflow)
- [Implementation Phases](#implementation-phases)
  - [Phase 1: Foundation and Basic Copying (Weeks 1-3)](#phase-1-foundation-and-basic-copying-weeks-1-3)
    - [1.1 Project Setup and Infrastructure (Week 1)](#11-project-setup-and-infrastructure-week-1)
    - [1.2 Basic io_uring Integration (Week 2)](#12-basic-io_uring-integration-week-2)
    - [1.3 Metadata Preservation (Week 3)](#13-metadata-preservation-week-3)
  - [Phase 2: Optimization and Parallelism (Weeks 4-6)](#phase-2-optimization-and-parallelism-weeks-4-6)
    - [2.1 copy_file_range Implementation (Week 4)](#21-copy_file_range-implementation-week-4)
    - [2.2 Directory Traversal (Week 5)](#22-directory-traversal-week-5)
    - [2.3 Per-CPU Queue Architecture (Week 6)](#23-per-cpu-queue-architecture-week-6)
  - [Phase 3: Advanced Features (Weeks 7-9)](#phase-3-advanced-features-weeks-7-9)
    - [3.1 Extended Attributes Support (Week 7)](#31-extended-attributes-support-week-7)
    - [3.2 Advanced Error Recovery (Week 8)](#32-advanced-error-recovery-week-8)
    - [3.3 Performance Optimization (Week 9)](#33-performance-optimization-week-9)
  - [Phase 4: Production Readiness (Weeks 10-12)](#phase-4-production-readiness-weeks-10-12)
    - [4.1 Comprehensive Testing Suite (Week 10)](#41-comprehensive-testing-suite-week-10)
    - [4.2 Documentation and User Experience (Week 11)](#42-documentation-and-user-experience-week-11)
    - [4.3 Release Preparation (Week 12)](#43-release-preparation-week-12)
- [Success Metrics](#success-metrics)
  - [Performance Targets](#performance-targets)
  - [Quality Targets](#quality-targets)
  - [Reliability Targets](#reliability-targets)
- [Risk Mitigation](#risk-mitigation)
  - [Technical Risks](#technical-risks)
  - [Project Risks](#project-risks)
- [References](#references)

## Architecture Decisions

### Recommended Technology Stack

Based on comprehensive research and analysis, the optimal approach is:

1. **Base Library**: Use `rio` as the foundation for io_uring operations
2. **Extended Operations**: Create `io-uring-extended` subcrate with:
   - `copy_file_range` support
   - `getdents64` directory traversal
   - Complete xattr operations suite
   - Additional missing operations
3. **Async Coordination**: Use `tokio::sync::mpsc` for all inter-task communication
4. **Per-CPU Architecture**: One io_uring instance per CPU core
5. **Modular Design**: Release extended operations as standalone crate

### Key Design Decisions

#### Channel Blocking vs Async Compatibility
**Answer**: Channel blocking is **NOT** async-friendly and should be avoided.

**The Problem**:
- Blocking channels (`crossbeam::channel`) can monopolize async threads
- Prevents the async runtime from scheduling other tasks efficiently
- Violates Rust's cooperative scheduling model

**The Solution**:
- ✅ Use `tokio::sync::mpsc` for async-friendly communication
- ✅ Operations like `sender.send().await` and `receiver.recv().await` are async-compatible
- ✅ Automatic backpressure through async channel semantics
- ❌ Avoid `crossbeam::channel` blocking operations in async contexts

#### Direct liburing Bindings Feasibility
**Answer**: ✅ **Highly Feasible** and recommended approach.

**Why It Works**:
- liburing is mature and well-designed for Rust integration
- Existing examples (`rio`, `tokio-uring`) prove the concept
- liburing's API is amenable to safe Rust abstractions

**Implementation Strategy**:
- Create `io-uring-extended` subcrate that extends `rio`
- Follow rio's design patterns for safety and ergonomics
- Release as standalone crate for community use
- Incremental development - add operations as needed

**Benefits**:
- **Modularity**: Independent crate that can be reused
- **Compatibility**: Works alongside existing libraries
- **Community Value**: Fills gaps in Rust io_uring ecosystem
- **Performance**: Direct access to all io_uring capabilities

#### io_uring xattr Support
**Answer**: ✅ **Fully Supported** through direct liburing bindings.

**Kernel Support**:
- io_uring supports xattr operations since kernel 5.6
- Available operations: `IORING_OP_SETXATTR`, `IORING_OP_GETXATTR`, `IORING_OP_LISTXATTR`
- liburing provides complete xattr wrappers

**Implementation Approach**:
- Direct liburing integration for xattr operations
- Safe Rust abstractions over unsafe syscalls
- Native async/await support through io_uring completion system
- No thread pool overhead - true async xattr operations

**Key Benefits**:
- **Complete Metadata Preservation**: ACLs, SELinux contexts, custom attributes
- **Optimal Performance**: No synchronous fallbacks needed
- **Consistent Architecture**: All operations use same io_uring infrastructure

## Development Workflow Requirements

### Phase Completion Checklist
After completing each phase, the following steps are **mandatory** before proceeding:

1. **Code Formatting**: Run `cargo fmt` to ensure all code is properly formatted
2. **Unit Tests**: All public functions must have corresponding unit tests
3. **System Tests**: Simple end-to-end system tests must be implemented and passing
4. **Test Execution**: All unit tests must pass (`cargo test`)
5. **Code Quality**: All clippy warnings must be resolved (`cargo clippy`)
6. **Commit**: Create a descriptive commit message following conventional commits
7. **Pull Request**: Create a PR targeting the previous branch
8. **New Branch**: Create a new branch for the next phase (stacked PR workflow)

### Stacked PR Workflow
- **Phase 1 PR**: Targets `main` branch
- **Phase 2 PR**: Targets `phase-1` branch  
- **Phase 3 PR**: Targets `phase-2` branch
- **Phase 4 PR**: Targets `phase-3` branch
- **Final PR**: Merge `phase-4` into `main` (after all phases reviewed)

This ensures incremental review and allows for easy rollback if needed.

## Implementation Phases

### Phase 1: Foundation and Basic Copying (Weeks 1-3)

#### 1.1 Project Setup and Infrastructure (Week 1)
**Deliverables:**
- Complete project structure with CI/CD pipeline
- Basic CLI interface with argument parsing
- Error handling framework
- Unit test framework setup
- Documentation structure

**Acceptance Criteria:**
- [x] All CI checks pass (formatting, linting, tests)
- [x] CLI shows help and version information
- [x] Basic argument validation works
- [x] Project builds and runs without errors
- [x] Code coverage reporting set up
- [x] Pre-commit hooks configured

**Testing Requirements:**
- Unit tests for CLI argument parsing
- Integration tests for help/version commands
- CI pipeline tests on multiple Rust versions

**Phase Completion Workflow:**
- [x] Run `cargo fmt` to format all code
- [x] Ensure all public functions have unit tests
- [x] Implement basic system tests (help/version commands)
- [x] Run `cargo test` - all tests must pass
- [x] Run `cargo clippy` - resolve all warnings
- [x] Commit with message: `feat(phase-1): complete project setup and infrastructure`
- [x] Create PR targeting `main` branch
- [x] Create new branch `phase-1` for next phase

#### 1.2 Basic io_uring Integration (Week 2)
**Deliverables:**
- rio integration for basic io_uring operations
- Simple file read/write operations
- Basic error handling and recovery
- Progress tracking framework

**Acceptance Criteria:**
- [x] Can open files using io_uring
- [x] Can read and write files asynchronously
- [x] Basic error handling works
- [x] Progress reporting functional
- [x] Memory usage is reasonable

**Testing Requirements:**
- Unit tests for file operations
- Integration tests for basic file copying
- Performance benchmarks for read/write operations
- Memory leak detection

**Phase Completion Workflow:**
- [x] Run `cargo fmt` to format all code
- [x] Ensure all io_uring functions have unit tests
- [x] Implement system tests (basic file copy operations)
- [x] Run `cargo test` - all tests must pass
- [x] Run `cargo clippy` - resolve all warnings
- [x] Commit with message: `feat(phase-1): implement basic io_uring integration`
- [x] Create PR targeting `main` branch
- [x] Continue on `phase-1` branch for next deliverable

#### 1.3 Metadata Preservation (Week 3)
**Deliverables:**
- File ownership preservation (chown operations)
- Permission preservation (chmod operations)
- Timestamp preservation
- Directory creation with proper permissions

**Acceptance Criteria:**
- [x] File ownership is preserved after copy
- [x] File permissions are preserved after copy
- [x] Modification timestamps are preserved
- [x] Directory permissions are correct
- [x] Handles permission errors gracefully

**Testing Requirements:**
- Unit tests for metadata operations
- Integration tests with various permission scenarios
- Verification tests comparing source and destination metadata

**Phase Completion Workflow:**
- [x] Run `cargo fmt` to format all code
- [x] Ensure all metadata functions have unit tests
- [x] Implement system tests (metadata preservation verification)
- [x] Run `cargo test` - all tests must pass
- [x] Run `cargo clippy` - resolve all warnings
- [x] Commit with message: `feat(phase-1): implement metadata preservation`
- [x] Create PR targeting `main` branch (final Phase 1 PR)
- [x] Create new branch `phase-2` for Phase 2 work

### Phase 2: Optimization and Parallelism (Weeks 4-6)

#### 2.1 copy_file_range Implementation (Week 4)
**Deliverables:**
- Manual copy_file_range implementation using liburing
- Automatic detection of same-filesystem operations
- Fallback to read/write for cross-filesystem
- Performance optimization for large files

**Acceptance Criteria:**
- [x] copy_file_range works for same-filesystem copies
- [x] Automatic fallback to read/write for different filesystems
- [x] Performance improvement over read/write for same-filesystem
- [x] Handles partial copy failures correctly
- [x] Zero-copy operations work as expected

**Testing Requirements:**
- Unit tests for copy_file_range operations
- Performance benchmarks comparing methods
- Cross-filesystem fallback tests
- Large file copy tests (files > RAM size)

**Phase Completion Workflow:**
- [x] Run `cargo fmt` to format all code
- [x] Ensure all copy_file_range functions have unit tests
- [x] Implement system tests (copy_file_range vs read/write comparison)
- [x] Run `cargo test` - all tests must pass
- [x] Run `cargo clippy` - resolve all warnings
- [x] Commit with message: `feat(phase-2): implement copy_file_range support`
- [x] Create PR targeting `phase-1` branch
- [x] Continue on `phase-2` branch for next deliverable

#### 2.2 Directory Traversal (Week 5)
**Deliverables:**
- Hybrid directory traversal (std::fs + io_uring)
- Parallel directory scanning
- File discovery and queuing system
- Directory structure preservation

**Acceptance Criteria:**
- [x] Can traverse large directory trees efficiently
- [x] Maintains directory structure in destination
- [ ] Handles symbolic links correctly (basic handling implemented, io_uring operations pending)
- [x] Processes directories in parallel
- [x] Memory usage scales with directory size

**Testing Requirements:**
- Unit tests for directory traversal
- Integration tests with complex directory structures
- Performance tests with large directory trees
- Memory usage tests for deep nesting

**Phase Completion Workflow:**
- [x] Run `cargo fmt` to format all code
- [x] Ensure all directory traversal functions have unit tests
- [x] Implement system tests (complex directory structure copying)
- [x] Run `cargo test` - all tests must pass
- [x] Run `cargo clippy` - resolve all warnings
- [x] Commit with message: `feat(phase-2): implement directory traversal`
- [x] Create PR targeting `phase-1` branch
- [x] Continue on `phase-2` branch for next deliverable

#### 2.3 Symlink and Filesystem Operations (Week 6)
**Deliverables:**
- Complete io_uring symlink operations (IORING_OP_SYMLINKAT, IORING_OP_READLINK)
- Filesystem boundary detection using statx (st_dev comparison)
- Hardlink detection and creation using io_uring (IORING_OP_STATX, IORING_OP_LINKAT)
- Cross-filesystem traversal prevention

**Acceptance Criteria:**
- [x] io_uring symlink creation and reading operations work correctly
- [x] Filesystem boundary detection prevents cross-filesystem traversal
- [x] Hardlink detection identifies existing hardlinks by (st_dev, st_ino) pairs
- [x] Hardlink creation preserves hardlink relationships
- [x] Symlink handling preserves symlink targets and metadata

**Testing Requirements:**
- Unit tests for symlink operations
- Integration tests for filesystem boundary detection
- Hardlink detection and creation tests
- Cross-filesystem traversal prevention tests

**Phase Completion Workflow:**
- [x] Run `cargo fmt` to format all code
- [x] Ensure all symlink and filesystem functions have unit tests
- [x] Implement system tests (symlink and hardlink operations verification)
- [x] Run `cargo test` - all tests must pass
- [x] Run `cargo clippy` - resolve all warnings
- [x] Commit with message: `feat(phase-2-3): implement symlink and filesystem operations`
- [x] Create PR targeting `phase-1` branch (final Phase 2 PR)
- [x] Create new branch `phase-3` for Phase 3 work

### Phase 3: Advanced Features and Performance (Weeks 7-9)

#### 3.1 Extended Attributes and Advanced Metadata (Week 7)
**Deliverables:**
- Complete io_uring xattr implementation using IORING_OP_GETXATTR, IORING_OP_SETXATTR, IORING_OP_LISTXATTR
- POSIX ACL preservation and copying
- SELinux context preservation using security.selinux xattr
- Custom user-defined extended attributes support
- Fallback mechanisms for filesystems without xattr support

**Acceptance Criteria:**
- [ ] All extended attributes are preserved using io_uring operations
- [ ] POSIX ACLs are copied correctly with proper permission preservation
- [ ] SELinux contexts are preserved on SELinux-enabled systems
- [ ] User-defined attributes work across different filesystem types
- [ ] Graceful fallback when xattr operations are not supported
- [ ] Performance is optimal with io_uring vs synchronous xattr calls

**Testing Requirements:**
- Unit tests for xattr operations
- Integration tests with various xattr types
- ACL preservation verification tests
- SELinux context tests (on SELinux systems)

**Phase Completion Workflow:**
- [ ] Run `cargo fmt` to format all code
- [ ] Ensure all xattr functions have unit tests
- [ ] Implement system tests (xattr preservation verification)
- [ ] Run `cargo test` - all tests must pass
- [ ] Run `cargo clippy` - resolve all warnings
- [ ] Commit with message: `feat(phase-3): implement extended attributes support`
- [ ] Create PR targeting `phase-2` branch
- [ ] Continue on `phase-3` branch for next deliverable

#### 3.2 Per-CPU Queue Architecture and Parallelism (Week 8)
**Deliverables:**
- Per-CPU io_uring instances with thread pinning
- CPU affinity management and work distribution
- Configurable queue depths and load balancing
- Performance scaling with CPU core count
- Memory-efficient per-CPU buffer management

**Acceptance Criteria:**
- [ ] Each CPU core has its own dedicated io_uring instance
- [ ] Threads are pinned to specific CPU cores for optimal performance
- [ ] Work is distributed evenly across all available CPUs
- [ ] Queue depths are configurable and bounded to prevent memory exhaustion
- [ ] Linear performance scaling with CPU count up to 32 cores
- [ ] Memory usage remains reasonable with per-CPU architecture

**Testing Requirements:**
- Unit tests for per-CPU queue management
- Performance tests with different CPU counts (1, 2, 4, 8, 16, 32 cores)
- Load balancing verification tests
- Memory usage tests for per-CPU architecture
- Scalability benchmarks comparing single vs multi-CPU performance

**Phase Completion Workflow:**
- [ ] Run `cargo fmt` to format all code
- [ ] Ensure all per-CPU queue functions have unit tests
- [ ] Implement system tests (multi-CPU performance verification)
- [ ] Run `cargo test` - all tests must pass
- [ ] Run `cargo clippy` - resolve all warnings
- [ ] Commit with message: `feat(phase-3): implement per-CPU queue architecture`
- [ ] Create PR targeting `phase-2` branch
- [ ] Continue on `phase-3` branch for next deliverable

#### 3.3 Direct I/O and Performance Optimization (Week 9)
**Deliverables:**
- O_DIRECT support with aligned buffer management
- File preallocation using fallocate for optimal performance
- Buffer pooling and memory reuse strategies
- Memory-mapped file support for very large files
- Advanced batching strategies to reduce system call overhead
- Performance profiling and optimization tools

**Acceptance Criteria:**
- [ ] O_DIRECT operations work correctly with aligned buffers
- [ ] File preallocation improves copy performance for large files
- [ ] Buffer pooling reduces memory allocations and improves throughput
- [ ] Memory-mapped files provide optimal performance for very large files
- [ ] Batching strategies reduce system call overhead
- [ ] Performance meets or exceeds targets: >500 MB/s for same-filesystem copies
- [ ] Memory usage remains under 100MB base + 1MB per 1000 files

**Testing Requirements:**
- Performance benchmarks for all optimizations
- Memory usage tests for buffer pooling
- Large file performance tests
- System resource utilization tests

### Phase 4: Production Readiness and Release (Weeks 10-12)

#### 4.1 Comprehensive Testing and Quality Assurance (Week 10)
**Deliverables:**
- Complete end-to-end test suite covering all features
- Property-based testing for edge cases and data integrity
- Performance regression testing with automated benchmarks
- Cross-platform compatibility testing on target systems
- Chaos engineering tests for error recovery
- Security testing and vulnerability assessment

**Acceptance Criteria:**
- [ ] All test categories pass consistently across all environments
- [ ] Property-based tests successfully find edge cases and data corruption scenarios
- [ ] Performance benchmarks establish reliable baselines for regression detection
- [ ] Cross-platform tests pass on all target Linux distributions
- [ ] Test coverage exceeds 90% for all critical code paths
- [ ] Chaos engineering tests verify error recovery mechanisms
- [ ] Security audit passes with no critical vulnerabilities

**Testing Requirements:**
- Complete test suite execution
- Property-based test validation
- Performance baseline establishment
- Cross-platform compatibility verification

#### 4.2 Documentation, User Experience, and Community (Week 11)
**Deliverables:**
- Complete user documentation with installation and usage guides
- Comprehensive API documentation with code examples
- Performance tuning guide for different use cases
- Troubleshooting guide for common issues
- Community contribution guidelines and development setup
- Release notes and changelog management

**Acceptance Criteria:**
- [ ] User documentation is complete, accurate, and easy to follow
- [ ] All public APIs are documented with comprehensive examples
- [ ] Performance tuning guide helps users optimize for their specific use cases
- [ ] Troubleshooting guide covers all common issues and edge cases
- [ ] Documentation builds without warnings and is accessible
- [ ] Community guidelines are clear and encourage contributions
- [ ] Release process is documented and reproducible

**Testing Requirements:**
- Documentation accuracy verification
- Example code validation
- User experience testing
- Documentation build tests

#### 4.3 Release Preparation and Distribution (Week 12)
**Deliverables:**
- Final performance optimization and benchmark validation
- Complete security audit and vulnerability assessment
- Release packaging for multiple Linux distributions
- Automated release pipeline and distribution setup
- Community announcement and documentation
- Long-term maintenance and support planning

**Acceptance Criteria:**
- [ ] Performance meets or exceeds all established targets
- [ ] Security audit passes with no critical or high-severity issues
- [ ] Release packages work correctly on all target platforms
- [ ] Community documentation is complete and ready for users
- [ ] Release process is fully automated and documented
- [ ] Long-term maintenance plan is established
- [ ] Project is ready for community adoption and contributions

**Testing Requirements:**
- Final performance validation
- Security testing
- Release package verification
- Community readiness assessment

## Success Metrics

### Performance Targets
- **Throughput**: >500 MB/s for same-filesystem copies on SSD
- **Latency**: <1ms per operation for small files
- **Scalability**: Linear scaling with CPU cores up to 32 cores
- **Memory**: <100MB base memory usage + 1MB per 1000 files

### Quality Targets
- **Test Coverage**: >90% code coverage
- **Error Handling**: Graceful handling of all error conditions
- **Documentation**: 100% public API documentation
- **Compatibility**: Support for Linux kernel 5.6+ and Rust 1.70+

### Reliability Targets
- **Data Integrity**: 100% file integrity verification
- **Metadata Preservation**: Complete metadata preservation
- **Error Recovery**: Recovery from all transient failures
- **Stability**: No memory leaks or crashes under normal operation

## Risk Mitigation

### Technical Risks
- **io_uring Library Gaps**: Mitigated by direct liburing implementation
- **Cross-platform Compatibility**: Mitigated by comprehensive testing matrix
- **Performance Regression**: Mitigated by automated performance testing

### Project Risks
- **Scope Creep**: Mitigated by strict phase boundaries and acceptance criteria
- **Timeline Delays**: Mitigated by parallel development and incremental delivery
- **Quality Issues**: Mitigated by comprehensive testing and code review process

## References

### Implementation Resources
- [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
- [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
- [GitFlow Branching Model](https://nvie.com/posts/a-successful-git-branching-model/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [io_uring Documentation](https://kernel.dk/io_uring.pdf)
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Async Rust Best Practices](https://rust-lang.github.io/async-book/)
- [Criterion.rs Documentation](https://docs.rs/criterion/)

This implementation plan provides a clear roadmap for delivering a production-ready io_uring-sync utility with comprehensive testing, documentation, and performance optimization.

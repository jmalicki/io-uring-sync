# io_uring Bulk Copying Utility Research

## Executive Summary

This document outlines the research findings for developing a highly efficient bulk copying utility in Rust using io_uring for asynchronous I/O operations, similar to rsync but optimized for single-machine operations with parallelism and metadata preservation.

## 1. io_uring Overview and Capabilities

### Core io_uring Features
- **Introduction**: Linux kernel interface introduced in version 5.1 (2019)
- **Purpose**: Provides efficient asynchronous I/O operations using ring buffers
- **Performance**: Reduces system call overhead by batching operations
- **Architecture**: Uses submission and completion queues for asynchronous operation management

### High-Level Primitives Support

#### copy_file_range Support
- **Kernel Version**: Available since kernel 5.3, with io_uring support since 5.6
- **Functionality**: Enables in-kernel file copying without transferring data to user space
- **Benefits**: Significantly reduces context switches and memory copies
- **Performance**: Can achieve near-zero-copy file operations for same-filesystem copies
- **Reference**: [Linux man pages - copy_file_range](https://man7.org/linux/man-pages/man2/copy_file_range.2.html)

#### sendfile Support
- **Availability**: Supported in io_uring for efficient file-to-socket transfers
- **Use Case**: Primarily for network operations, but can be adapted for local file copying
- **Limitation**: Not directly applicable for local file-to-file operations

#### Zero-Copy Operations
- **splice() and tee()**: Available through io_uring for zero-copy operations
- **Use Case**: Efficient for piping data between file descriptors
- **Performance**: Eliminates user-space data copying entirely

### Metadata Operations
- **Extended Attributes (xattr)**: io_uring supports asynchronous xattr operations
- **File Permissions**: Standard stat/chmod operations are supported
- **ACLs**: Can be handled through xattr operations for POSIX ACLs
- **Ownership**: Supported through chown operations

## 2. Rust Libraries for io_uring Integration

### Core io_uring Libraries

#### rio
- **Repository**: [github.com/spacejam/rio](https://github.com/spacejam/rio)
- **Type**: Pure Rust implementation
- **Features**: 
  - Thread and async-friendly
  - Misuse-resistant design
  - Automatic submission queue management
  - Zero-copy system call support
- **Advantages**: No external C dependencies, leverages Rust's type system for safety

#### tokio-uring
- **Repository**: [github.com/tokio-rs/io-uring](https://github.com/tokio-rs/io-uring)
- **Type**: Tokio runtime integration
- **Features**:
  - Seamless integration with existing Tokio ecosystem
  - Async/await support
  - File I/O operations
- **Reference**: [Tokio-uring exploration article](https://developerlife.com/2024/05/25/tokio-uring-exploration-rust/)

#### uring-fs
- **Repository**: [lib.rs/crates/uring-fs](https://lib.rs/crates/uring-fs)
- **Type**: File system operations library
- **Features**:
  - Truly asynchronous file operations
  - Compatible with any async runtime
  - Supports open, stat, read, write operations
- **Advantages**: No thread pool required, pure async implementation

### High-Performance Runtimes

#### Glommio
- **Repository**: [docs.rs/glommio](https://docs.rs/glommio)
- **Type**: Thread-per-core async framework
- **Features**:
  - Safe Rust interface for io_uring
  - Thread-local I/O operations
  - CPU pinning support
  - Timers, file I/O, and networking abstractions
- **Use Case**: High-performance applications requiring maximum efficiency

#### Monoio
- **Repository**: [github.com/bytedance/monoio](https://github.com/bytedance/monoio)
- **Type**: Thread-per-core runtime
- **Features**:
  - Pure async I/O interface
  - Compatible with Tokio interface
  - High-performance networking solutions
  - Driver detection and switching
- **Reference**: [Monoio introduction](https://www.cloudwego.io/blog/2023/04/17/introducing-monoio-a-high-performance-rust-runtime-based-on-io-uring/)

#### libuio
- **Repository**: [docs.rs/libuio](https://docs.rs/libuio)
- **Type**: Async framework for networking
- **Features**:
  - Fully featured async framework
  - TCP listeners and streams
  - File operation modules

## 3. Implementation Considerations

### File Metadata Handling

#### Ownership and Permissions
- **Standard Operations**: Use stat, chown, chmod operations through io_uring
- **Preservation**: Maintain original file ownership and permissions during copy
- **Error Handling**: Graceful fallback for permission errors

#### Extended Attributes and ACLs
- **POSIX ACLs**: Handle through xattr operations
- **SELinux Contexts**: Preserve security contexts via extended attributes
- **Custom Attributes**: Support application-specific extended attributes

### Parallelism Strategies

#### Thread-Per-Core Model
- **Advantages**: Reduces context switching, improves cache efficiency
- **Implementation**: Use Glommio or Monoio for thread-per-core architecture
- **CPU Pinning**: Pin threads to specific CPU cores for optimal performance

#### Async Task Management
- **Concurrency**: Use Rust's async/await for concurrent file operations
- **Batching**: Submit multiple io_uring operations in batches
- **Backpressure**: Implement proper flow control for large file operations

### Performance Optimization

#### Buffer Management
- **Zero-Copy**: Leverage copy_file_range for same-filesystem operations
- **Buffer Pooling**: Reuse buffers to reduce allocation overhead
- **Size Tuning**: Optimize buffer sizes based on filesystem characteristics

#### Operation Batching
- **Submission Queues**: Batch multiple operations before submission
- **Completion Handling**: Efficiently process completion events
- **Error Recovery**: Handle partial failures gracefully

## 4. Technical Requirements

### Kernel Version Requirements
- **Minimum**: Linux kernel 5.1 (basic io_uring support)
- **Recommended**: Linux kernel 5.6+ (copy_file_range support in io_uring)
- **Optimal**: Linux kernel 5.8+ (full feature set)

### Dependencies
- **Primary**: Rust async runtime (Tokio, Glommio, or Monoio)
- **io_uring**: rio, tokio-uring, or uring-fs
- **File Operations**: Standard library + xattr support
- **Parallelism**: Rayon or native async concurrency

### Performance Targets
- **Throughput**: Aim for near-line-speed copying on fast storage
- **Latency**: Minimize per-operation overhead
- **Scalability**: Linear scaling with available CPU cores
- **Memory**: Efficient memory usage with minimal copying

## 5. Recommended Architecture

### Core Components
1. **File Scanner**: Async directory traversal with metadata collection
2. **Copy Engine**: io_uring-based file copying with parallelism
3. **Metadata Manager**: Preservation of ownership, permissions, ACLs
4. **Progress Tracker**: Real-time progress reporting
5. **Error Handler**: Comprehensive error recovery and reporting

### Technology Stack
- **Runtime**: Glommio for thread-per-core performance
- **io_uring**: rio for pure Rust implementation
- **Async**: Native async/await with proper task scheduling
- **Parallelism**: CPU-aware task distribution

## 6. Complete io_uring Operations Inventory

### Required Operations for rsync-like Functionality

#### File Copying Operations (Priority: HIGH)
1. **copy_file_range** - In-kernel file copying (kernel 5.6+, io_uring support 5.6+)
   - **Use Case**: Primary file copying mechanism for same-filesystem operations
   - **Performance**: Near-zero-copy, minimal context switches
   - **Status**: ✅ Supported in modern kernels

2. **splice** - Zero-copy data transfer between file descriptors
   - **Use Case**: Efficient copying when copy_file_range unavailable
   - **Performance**: Zero-copy operation
   - **Status**: ✅ Available in io_uring

3. **sendfile** - File-to-socket transfers
   - **Use Case**: Network operations, less relevant for local copying
   - **Performance**: Zero-copy for compatible operations
   - **Status**: ✅ Available in io_uring

4. **read/write** - Traditional file I/O operations
   - **Use Case**: Fallback for unsupported operations or cross-filesystem copies
   - **Performance**: Standard user-space copying
   - **Status**: ✅ Core io_uring operations

#### Directory Operations (Priority: HIGH)
5. **getdents64** - Directory entry reading
   - **Use Case**: Async directory traversal
   - **Performance**: Non-blocking directory listing
   - **Status**: ⚠️ Limited support in current Rust libraries

6. **openat/close** - File/directory opening/closing
   - **Use Case**: File descriptor management
   - **Performance**: Async file operations
   - **Status**: ✅ Widely supported

7. **mkdirat** - Directory creation
   - **Use Case**: Creating target directory structure
   - **Performance**: Async directory creation
   - **Status**: ✅ Available in io_uring

#### Metadata Operations (Priority: HIGH)
8. **statx** - Extended file statistics
   - **Use Case**: Comprehensive metadata retrieval
   - **Performance**: Async metadata collection
   - **Status**: ✅ Supported in io_uring

9. **fchmod** - File permission modification
   - **Use Case**: Preserving file permissions
   - **Performance**: Async permission updates
   - **Status**: ✅ Available in io_uring

10. **fchown** - File ownership modification
    - **Use Case**: Preserving file ownership
    - **Performance**: Async ownership updates
    - **Status**: ✅ Available in io_uring

11. **setxattr/getxattr** - Extended attributes management
    - **Use Case**: ACL preservation, SELinux contexts
    - **Performance**: Async xattr operations
    - **Status**: ✅ Available in io_uring

12. **listxattr** - Extended attributes listing
    - **Use Case**: Discovering all extended attributes
    - **Performance**: Async xattr enumeration
    - **Status**: ✅ Available in io_uring

#### Utility Operations (Priority: MEDIUM)
13. **unlinkat** - File/directory removal
    - **Use Case**: Cleanup operations, overwrite handling
    - **Performance**: Async file removal
    - **Status**: ✅ Available in io_uring

14. **renameat2** - File/directory renaming
    - **Use Case**: Atomic file moves, temporary file handling
    - **Performance**: Async rename operations
    - **Status**: ✅ Available in io_uring

15. **sync_file_range** - File synchronization
    - **Use Case**: Ensuring data integrity, explicit flushing
    - **Performance**: Async sync operations
    - **Status**: ✅ Available in io_uring

## 7. Library Support Analysis

### rio Library Support Assessment
- **Repository**: [github.com/spacejam/rio](https://github.com/spacejam/rio)
- **copy_file_range**: ❌ Not directly supported (manual implementation required)
- **splice/sendfile**: ❌ Not directly supported
- **getdents64**: ❌ Not supported (major limitation)
- **statx**: ✅ Supported
- **fchmod/fchown**: ✅ Supported
- **xattr operations**: ❌ Not supported
- **openat/close**: ✅ Supported
- **read/write**: ✅ Core functionality
- **Overall Assessment**: ⚠️ Limited - requires significant manual workarounds

### tokio-uring Library Support Assessment
- **Repository**: [github.com/tokio-rs/io-uring](https://github.com/tokio-rs/io-uring)
- **copy_file_range**: ❌ Not directly supported
- **splice/sendfile**: ❌ Not directly supported
- **getdents64**: ❌ Not supported
- **statx**: ⚠️ Limited support
- **fchmod/fchown**: ❌ Not supported
- **xattr operations**: ❌ Not supported
- **openat/close**: ✅ Supported
- **read/write**: ✅ Core functionality
- **Overall Assessment**: ⚠️ Very Limited - primarily file I/O focused

### Glommio Library Support Assessment
- **Repository**: [docs.rs/glommio](https://docs.rs/glommio)
- **copy_file_range**: ❌ Not directly supported
- **splice/sendfile**: ❌ Not directly supported
- **getdents64**: ❌ Not supported
- **statx**: ✅ Supported
- **fchmod/fchown**: ❌ Not supported
- **xattr operations**: ❌ Not supported
- **openat/close**: ✅ Supported
- **read/write**: ✅ Core functionality
- **Overall Assessment**: ⚠️ Limited - good for basic file I/O only

### Recommended Approach
Given the limited support in existing libraries, a **hybrid approach** is recommended:

1. **Use rio** as the base io_uring interface
2. **Implement custom wrappers** for unsupported operations:
   - Manual copy_file_range implementation
   - Custom xattr handling
   - Fallback directory traversal using std::fs
3. **Consider liburing bindings** for operations not supported by rio

## 8. Scalability and Queueing Architecture

### Queue Depth Limits and Recommendations

#### Submission Queue Depth Guidelines
- **Default**: 4096 operations (reasonable balance of memory usage and concurrency)
- **Minimum**: 1024 operations (for basic functionality)
- **Maximum**: 65536 operations (system resource dependent)
- **Command-line override**: `--queue-depth <N>` parameter

#### Completion Queue Depth Guidelines
- **Default**: Match submission queue depth
- **Memory impact**: ~8KB per 1024 entries (64-bit systems)
- **Recommendation**: Always match submission queue depth

#### Per-CPU Queue Architecture

##### Thread-Per-Core Model
```rust
// Conceptual architecture
struct PerCPUQueue {
    uring: Rio,                    // One io_uring instance per CPU
    submission_queue: Vec<Op>,     // CPU-local submission queue
    completion_handler: Handler,   // CPU-local completion processing
    cpu_id: usize,                 // CPU affinity
}
```

##### Benefits of Per-CPU Queues
- **Reduced contention**: No cross-CPU synchronization
- **Better cache locality**: CPU-local data structures
- **Linear scalability**: Performance scales with CPU count
- **Simplified error handling**: CPU-local error recovery

##### Implementation Strategy
1. **CPU Detection**: Use `num_cpus` crate for CPU count detection
2. **Thread Pinning**: Use `libc::sched_setaffinity` for CPU pinning
3. **Work Distribution**: Round-robin or work-stealing for load balancing
4. **Global Coordination**: Minimal shared state for progress tracking

### File-in-Flight Limits

#### Recommended Limits
- **Default**: 1024 files per CPU core
- **Command-line override**: `--max-files-in-flight <N>`
- **Memory calculation**: ~1KB per file metadata + buffer space
- **Total memory**: ~1MB per 1024 files per CPU

#### Queueing Strategy
```rust
// Async-friendly bounded channel for file operation coordination
let (file_sender, file_receiver) = tokio::sync::mpsc::channel(max_files_per_cpu);

// Per-CPU work queues with async coordination
let cpu_queues: Vec<PerCPUQueue> = (0..num_cpus)
    .map(|cpu_id| PerCPUQueue::new(cpu_id))
    .collect();
```

#### Async-Friendly Backpressure Handling
- **Non-blocking channels**: Use `tokio::sync::mpsc` instead of `crossbeam::channel`
- **Async coordination**: Channels that work seamlessly with async/await
- **Graceful degradation**: Reduce concurrency under memory pressure
- **Progress reporting**: Real-time queue depth monitoring

**Channel Blocking vs Async Compatibility**:
- ❌ **Avoid**: `crossbeam::channel` blocking operations in async contexts
- ✅ **Use**: `tokio::sync::mpsc` for async-friendly communication
- ✅ **Pattern**: `sender.send().await` and `receiver.recv().await` are async-friendly
- ⚠️ **Note**: Blocking channels can monopolize async threads and hurt performance

## 9. Critical Questions and Implementation Challenges

### Directory Traversal Limitations
**Question**: How do we handle directory traversal when getdents64 is not supported in current Rust io_uring libraries?

**Current Status**: 
- ⚠️ **Major Gap**: None of the major Rust io_uring libraries support getdents64
- **Impact**: Cannot perform truly async directory traversal
- **Workarounds**:
  1. **Hybrid approach**: Use std::fs for directory scanning, io_uring for file operations
  2. **Manual implementation**: Direct liburing bindings for getdents64
  3. **Batch processing**: Pre-scan directories synchronously, then process files async

**Recommendation**: Implement hybrid approach initially, with plans for manual getdents64 support

### Direct liburing Bindings Feasibility
**Question**: Is creating direct liburing bindings feasible and how can they integrate with existing libraries?

**Feasibility Assessment**: ✅ **Highly Feasible**
- **Existing Examples**: `rio` and `tokio-uring` demonstrate successful liburing integration
- **liburing Maturity**: Well-established C library with stable API
- **Rust Integration**: liburing's design is amenable to safe Rust abstractions

**Implementation Strategy**:
```rust
// Proposed subcrate: io-uring-extended
// Structure similar to rio but with extended operations

pub struct ExtendedRio {
    inner: rio::Rio,  // Base rio instance
    // Additional state for extended operations
}

impl ExtendedRio {
    // copy_file_range support
    pub async fn copy_file_range(
        &self,
        src_fd: i32,
        dst_fd: i32,
        src_offset: u64,
        dst_offset: u64,
        len: u64,
    ) -> Result<usize> {
        // Manual syscall implementation using liburing
        unsafe {
            let mut src_off = src_offset;
            let mut dst_off = dst_offset;
            libc::copy_file_range(
                src_fd,
                &mut src_off,
                dst_fd,
                &mut dst_off,
                len as usize,
                0,
            )
        }
    }

    // getdents64 support
    pub async fn readdir(&self, dir_fd: i32, buffer: &mut [u8]) -> Result<usize> {
        // Direct liburing getdents64 implementation
    }
}
```

**Subcrate Design Benefits**:
- **Modularity**: Can be released independently as `io-uring-extended`
- **Compatibility**: Works alongside existing rio/tokio-uring
- **Incremental Adoption**: Add operations as needed
- **Community Contribution**: Others can contribute missing operations

**Integration with rio Style**:
- **Safe Abstractions**: Encapsulate unsafe operations in safe APIs
- **Error Handling**: Consistent error types and handling
- **Async/Await**: Native async support with proper `.await` points
- **Documentation**: Comprehensive docs with examples

### copy_file_range Implementation Gap
**Question**: How do we implement copy_file_range when it's not supported by existing libraries?

**Current Status**:
- ❌ **Critical Gap**: No Rust library provides copy_file_range support
- **Impact**: Cannot achieve optimal zero-copy performance for same-filesystem operations
- **Workarounds**:
  1. **Manual syscall**: Direct system call implementation using nix or libc
  2. **Fallback to read/write**: Use standard io_uring read/write operations
  3. **splice operations**: Use splice as alternative zero-copy mechanism

**Implementation Strategy**:
```rust
// Conceptual implementation
async fn copy_file_range_async(
    src_fd: i32,
    dst_fd: i32,
    src_offset: u64,
    dst_offset: u64,
    len: u64,
) -> Result<usize> {
    // Manual syscall implementation
    unsafe {
        libc::copy_file_range(
            src_fd,
            &mut src_offset as *mut _,
            dst_fd,
            &mut dst_offset as *mut _,
            len as usize,
            0,
        )
    }
}
```

### Extended Attributes (xattr) Support
**Question**: How do we handle ACLs and extended attributes when xattr operations aren't supported?

**Current Status**:
- ❌ **Major Gap**: No Rust io_uring library supports xattr operations
- **Impact**: Cannot preserve ACLs, SELinux contexts, or custom attributes

**io_uring xattr Support Investigation**:
- **Kernel Support**: io_uring does support xattr operations since kernel 5.6
- **Available Operations**: `IORING_OP_SETXATTR`, `IORING_OP_GETXATTR`, `IORING_OP_LISTXATTR`
- **liburing Support**: ✅ **Available** - liburing provides xattr wrappers
- **Rust Integration**: ❌ **Missing** - No Rust library exposes these operations

**Direct liburing xattr Implementation**:
```rust
// Proposed xattr support in io-uring-extended crate
impl ExtendedRio {
    // Get extended attribute
    pub async fn getxattr(
        &self,
        path: &Path,
        name: &str,
        buffer: &mut [u8],
    ) -> Result<usize> {
        let path_c = CString::new(path.as_os_str().as_bytes())?;
        let name_c = CString::new(name)?;
        
        // Submit io_uring xattr operation
        let submission = io_uring_sqe {
            opcode: IORING_OP_GETXATTR as u8,
            flags: 0,
            ioprio: 0,
            fd: -1, // Use path instead of fd
            off: 0,
            addr: path_c.as_ptr() as u64,
            len: buffer.len() as u32,
            opcode_flags: 0,
            user_data: self.next_user_data(),
            buf_index: 0,
            personality: 0,
            splice_fd_in: 0,
            __pad2: [0; 2],
        };
        
        self.submit_and_wait(submission).await
    }

    // Set extended attribute
    pub async fn setxattr(
        &self,
        path: &Path,
        name: &str,
        value: &[u8],
        flags: i32,
    ) -> Result<()> {
        // Similar implementation for setxattr
    }

    // List extended attributes
    pub async fn listxattr(
        &self,
        path: &Path,
        buffer: &mut [u8],
    ) -> Result<usize> {
        // Implementation for listxattr
    }
}
```

**Implementation Strategy**:
1. **Direct liburing Integration**: Use liburing's xattr wrappers
2. **Safe Rust Abstractions**: Encapsulate unsafe operations in safe APIs
3. **Async Coordination**: Integrate with io_uring completion system
4. **Error Handling**: Proper error propagation and handling

**Benefits of Direct xattr Support**:
- ✅ **True Async**: Native io_uring async xattr operations
- ✅ **Performance**: No thread pool overhead for xattr operations
- ✅ **Consistency**: All operations use same io_uring infrastructure
- ✅ **Completeness**: Full metadata preservation capabilities

### Queue Depth and Memory Management
**Question**: What are the practical limits for queue depth and how do we prevent memory exhaustion?

**Critical Considerations**:
- **Memory per operation**: ~1KB metadata + buffer space per file
- **Queue overflow**: io_uring submission queue can overflow, causing operation loss
- **Backpressure**: Need mechanisms to prevent overwhelming the system

**Recommended Limits**:
- **Conservative**: 1024 operations per CPU core
- **Aggressive**: 4096 operations per CPU core
- **Maximum**: 65536 operations (system-dependent)

### Thread Safety and Per-CPU Architecture
**Question**: How do we implement thread-safe per-CPU queues without cross-CPU contention?

**Architecture Decisions**:
1. **One io_uring instance per CPU**: Eliminates cross-CPU synchronization
2. **CPU-local work queues**: Each CPU manages its own file queue
3. **Global coordination**: Minimal shared state for progress tracking
4. **Work stealing**: Optional load balancing between CPUs

### Error Handling and Recovery
**Question**: How do we handle partial failures and ensure data integrity?

**Challenges**:
- **Partial file copies**: What happens if copy_file_range fails mid-operation?
- **Metadata failures**: How to handle permission/ownership errors?
- **Queue overflow**: Recovery from submission queue overflow
- **Cross-filesystem operations**: Fallback strategies for different filesystems

### Performance Optimization Questions
**Question**: What are the optimal buffer sizes and batching strategies?

**Research Needed**:
- **Buffer size tuning**: Optimal sizes for different storage types (SSD vs HDD)
- **Batch size optimization**: How many operations to submit per batch
- **Memory mapping**: Should we use mmap for large files?
- **Direct I/O**: When to use O_DIRECT for maximum performance

### Command-Line Interface Design
**Question**: What configuration options should be exposed to users?

**Proposed Options**:
```bash
io_uring_sync [OPTIONS] <SOURCE> <DESTINATION>

Options:
  --queue-depth <N>           # Submission queue depth (default: 4096)
  --max-files-in-flight <N>   # Max concurrent files per CPU (default: 1024)
  --cpu-count <N>             # Number of CPUs to use (default: auto-detect)
  --buffer-size <N>           # Buffer size in KB (default: auto-detect)
  --copy-method <METHOD>      # copy_file_range, splice, read/write
  --preserve-xattr            # Preserve extended attributes
  --preserve-acl              # Preserve POSIX ACLs
  --dry-run                   # Show what would be copied
  --progress                  # Show progress information
  --verbose                   # Verbose output
```

## 10. Summary: Addressing Key Implementation Questions

### Channel Blocking vs Async Compatibility
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

### Direct liburing Bindings Feasibility
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

### io_uring xattr Support
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

### Recommended Architecture
Based on this analysis, the optimal approach is:

1. **Base Library**: Use `rio` as the foundation
2. **Extended Operations**: Create `io-uring-extended` subcrate with:
   - `copy_file_range` support
   - `getdents64` directory traversal
   - Complete xattr operations suite
   - Additional missing operations
3. **Async Coordination**: Use `tokio::sync::mpsc` for all inter-task communication
4. **Per-CPU Architecture**: One io_uring instance per CPU core
5. **Modular Design**: Release extended operations as standalone crate

This approach provides the best balance of performance, safety, and maintainability while filling the critical gaps in the current Rust io_uring ecosystem.

## 11. Comprehensive Testing Strategy

### High-Level End-to-End Testing

#### Data Integrity Verification
**Challenge**: How do we ensure files are copied without corruption and all metadata is preserved?

**Testing Approach**:
1. **Checksum Verification**: Use cryptographic hashes (SHA-256) to verify file integrity
2. **Metadata Comparison**: Compare ownership, permissions, timestamps, and extended attributes
3. **Directory Structure Verification**: Ensure complete directory tree replication
4. **Stress Testing**: Large files, many files, deep directory structures

**Implementation Strategy**:
```rust
// Test data generation
pub struct TestDataGenerator {
    // Create files with known checksums
    pub fn create_known_files(&self, dir: &Path) -> HashMap<PathBuf, String>;
    
    // Create files with extended attributes
    pub fn create_files_with_xattrs(&self, dir: &Path) -> Vec<PathBuf>;
    
    // Create stress test scenarios
    pub fn create_stress_test(&self, dir: &Path) -> TestScenario;
}

// Verification utilities
pub struct VerificationSuite {
    pub fn verify_file_integrity(&self, src: &Path, dst: &Path) -> Result<()>;
    pub fn verify_metadata(&self, src: &Path, dst: &Path) -> Result<()>;
    pub fn verify_xattrs(&self, src: &Path, dst: &Path) -> Result<()>;
    pub fn verify_directory_structure(&self, src: &Path, dst: &Path) -> Result<()>;
}
```

#### Test Scenarios

##### Basic Functionality Tests
- **Single File Copy**: Small, medium, large files
- **Directory Copy**: Shallow and deep directory structures
- **Metadata Preservation**: Ownership, permissions, timestamps
- **Extended Attributes**: User attributes, system attributes, ACLs

##### Edge Cases and Error Conditions
- **Permission Errors**: Read-only files, protected directories
- **Disk Space**: Insufficient space scenarios
- **Concurrent Access**: Files being modified during copy
- **Symbolic Links**: Hard links, soft links, broken links
- **Special Files**: Devices, sockets, named pipes

##### Performance and Stress Tests
- **Large Files**: Files larger than available RAM
- **Many Small Files**: Thousands of small files
- **Mixed Workloads**: Various file sizes and types
- **Memory Pressure**: Limited available memory
- **CPU Contention**: Multiple concurrent operations

##### Cross-Filesystem Testing
- **Same Filesystem**: Optimal copy_file_range performance
- **Different Filesystems**: Fallback to read/write operations
- **Network Filesystems**: NFS, CIFS, etc.
- **Special Filesystems**: tmpfs, ramfs, etc.

#### Automated Test Suite

##### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_copy_file_range_same_filesystem() {
        // Test copy_file_range on same filesystem
    }
    
    #[test]
    fn test_copy_file_range_cross_filesystem() {
        // Test fallback behavior
    }
    
    #[test]
    fn test_xattr_preservation() {
        // Test extended attribute preservation
    }
}
```

##### Integration Tests
```rust
#[test]
fn test_end_to_end_copy() {
    let temp_dir = create_test_directory();
    let src = temp_dir.path().join("source");
    let dst = temp_dir.path().join("destination");
    
    // Run the copy operation
    let result = sync_files(&args).await;
    assert!(result.is_ok());
    
    // Verify integrity
    let verification = VerificationSuite::new();
    verification.verify_file_integrity(&src, &dst).unwrap();
    verification.verify_metadata(&src, &dst).unwrap();
    verification.verify_xattrs(&src, &dst).unwrap();
}
```

##### Property-Based Tests
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_copy_any_file_size(size in 0..10_000_000usize) {
        // Test copying files of random sizes
        let data = vec![0u8; size];
        // ... test implementation
    }
    
    #[test]
    fn test_copy_with_random_xattrs(
        xattr_count in 0..10usize,
        xattr_size in 0..1000usize
    ) {
        // Test copying files with random extended attributes
        // ... test implementation
    }
}
```

##### Benchmark Tests
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_copy_methods(c: &mut Criterion) {
    let mut group = c.benchmark_group("copy_methods");
    
    group.bench_function("copy_file_range", |b| {
        b.iter(|| copy_file_range(black_box(&src), black_box(&dst)))
    });
    
    group.bench_function("read_write", |b| {
        b.iter(|| copy_read_write(black_box(&src), black_box(&dst)))
    });
    
    group.finish();
}
```

#### Continuous Integration Testing

##### Test Matrix
- **Operating Systems**: Ubuntu 20.04+, CentOS 8+, Debian 11+
- **Kernel Versions**: 5.6+ (minimum), 5.8+ (recommended), latest
- **Architectures**: x86_64, aarch64
- **Filesystems**: ext4, xfs, btrfs, zfs
- **Memory Configurations**: 1GB, 4GB, 16GB+

##### Automated Test Execution
```yaml
# .github/workflows/test-matrix.yml
strategy:
  matrix:
    os: [ubuntu-20.04, ubuntu-22.04, centos-8]
    kernel: [5.6, 5.8, 5.15]
    filesystem: [ext4, xfs, btrfs]
    memory: [1gb, 4gb, 16gb]
```

#### Test Data Management

##### Synthetic Test Data
- **Known Patterns**: Files with predictable content for integrity checking
- **Random Data**: Files with random content for stress testing
- **Real-world Data**: Sample datasets from common use cases
- **Edge Cases**: Empty files, very large files, files with unusual names

##### Test Environment Setup
```rust
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub source_dir: PathBuf,
    pub dest_dir: PathBuf,
    pub test_files: Vec<TestFile>,
}

impl TestEnvironment {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let dest_dir = temp_dir.path().join("destination");
        
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&dest_dir).unwrap();
        
        Self {
            temp_dir,
            source_dir,
            dest_dir,
            test_files: Vec::new(),
        }
    }
    
    pub fn create_test_scenario(&mut self, scenario: TestScenario) {
        match scenario {
            TestScenario::BasicFiles => self.create_basic_files(),
            TestScenario::LargeFiles => self.create_large_files(),
            TestScenario::ManyFiles => self.create_many_files(),
            TestScenario::WithXattrs => self.create_files_with_xattrs(),
        }
    }
}
```

#### Performance Regression Testing

##### Baseline Performance Metrics
- **Throughput**: MB/s for various file sizes
- **Latency**: Time per operation
- **Memory Usage**: Peak and average memory consumption
- **CPU Utilization**: Efficiency across different workloads

##### Automated Performance Testing
```rust
pub struct PerformanceBenchmark {
    pub throughput: f64,      // MB/s
    pub latency: Duration,    // Average operation time
    pub memory_usage: usize,  // Peak memory in bytes
    pub cpu_efficiency: f64,  // Operations per CPU second
}

impl PerformanceBenchmark {
    pub fn run_benchmark(&self, test_data: &TestData) -> BenchmarkResults {
        // Run comprehensive performance tests
    }
    
    pub fn compare_with_baseline(&self, baseline: &BenchmarkResults) -> RegressionReport {
        // Compare against previous results
    }
}
```

#### Error Injection Testing

##### Simulated Failure Conditions
- **IO Errors**: Disk full, permission denied, network failures
- **Memory Pressure**: Limited available memory
- **Interruptions**: Process termination, signal handling
- **Concurrent Modifications**: Files changed during copy

##### Chaos Engineering
```rust
pub struct ChaosEngine {
    pub fn inject_io_errors(&self, probability: f64);
    pub fn limit_memory(&self, max_memory: usize);
    pub fn simulate_disk_full(&self);
    pub fn random_delays(&self, max_delay: Duration);
}
```

This comprehensive testing strategy ensures that io-uring-sync is reliable, performant, and handles all edge cases correctly while preserving data integrity and metadata.

## 12. Detailed Implementation Plan

### Phase 1: Foundation and Basic Copying (Weeks 1-3)

#### 1.1 Project Setup and Infrastructure (Week 1)
**Deliverables:**
- Complete project structure with CI/CD pipeline
- Basic CLI interface with argument parsing
- Error handling framework
- Unit test framework setup
- Documentation structure

**Acceptance Criteria:**
- ✅ All CI checks pass (formatting, linting, tests)
- ✅ CLI shows help and version information
- ✅ Basic argument validation works
- ✅ Project builds and runs without errors
- ✅ Code coverage reporting set up
- ✅ Pre-commit hooks configured

**Testing Requirements:**
- Unit tests for CLI argument parsing
- Integration tests for help/version commands
- CI pipeline tests on multiple Rust versions

#### 1.2 Basic io_uring Integration (Week 2)
**Deliverables:**
- rio integration for basic io_uring operations
- Simple file read/write operations
- Basic error handling and recovery
- Progress tracking framework

**Acceptance Criteria:**
- ✅ Can open files using io_uring
- ✅ Can read and write files asynchronously
- ✅ Basic error handling works
- ✅ Progress reporting functional
- ✅ Memory usage is reasonable

**Testing Requirements:**
- Unit tests for file operations
- Integration tests for basic file copying
- Performance benchmarks for read/write operations
- Memory leak detection

#### 1.3 Metadata Preservation (Week 3)
**Deliverables:**
- File ownership preservation (chown operations)
- Permission preservation (chmod operations)
- Timestamp preservation
- Directory creation with proper permissions

**Acceptance Criteria:**
- ✅ File ownership is preserved after copy
- ✅ File permissions are preserved after copy
- ✅ Modification timestamps are preserved
- ✅ Directory permissions are correct
- ✅ Handles permission errors gracefully

**Testing Requirements:**
- Unit tests for metadata operations
- Integration tests with various permission scenarios
- Verification tests comparing source and destination metadata

### Phase 2: Optimization and Parallelism (Weeks 4-6)

#### 2.1 copy_file_range Implementation (Week 4)
**Deliverables:**
- Manual copy_file_range implementation using liburing
- Automatic detection of same-filesystem operations
- Fallback to read/write for cross-filesystem
- Performance optimization for large files

**Acceptance Criteria:**
- ✅ copy_file_range works for same-filesystem copies
- ✅ Automatic fallback to read/write for different filesystems
- ✅ Performance improvement over read/write for same-filesystem
- ✅ Handles partial copy failures correctly
- ✅ Zero-copy operations work as expected

**Testing Requirements:**
- Unit tests for copy_file_range operations
- Performance benchmarks comparing methods
- Cross-filesystem fallback tests
- Large file copy tests (files > RAM size)

#### 2.2 Directory Traversal (Week 5)
**Deliverables:**
- Hybrid directory traversal (std::fs + io_uring)
- Parallel directory scanning
- File discovery and queuing system
- Directory structure preservation

**Acceptance Criteria:**
- ✅ Can traverse large directory trees efficiently
- ✅ Maintains directory structure in destination
- ✅ Handles symbolic links correctly
- ✅ Processes directories in parallel
- ✅ Memory usage scales with directory size

**Testing Requirements:**
- Unit tests for directory traversal
- Integration tests with complex directory structures
- Performance tests with large directory trees
- Memory usage tests for deep nesting

#### 2.3 Per-CPU Queue Architecture (Week 6)
**Deliverables:**
- Per-CPU io_uring instances
- Thread pinning and CPU affinity
- Work distribution and load balancing
- Queue depth management

**Acceptance Criteria:**
- ✅ Each CPU core has its own io_uring instance
- ✅ Threads are pinned to specific CPU cores
- ✅ Work is distributed evenly across CPUs
- ✅ Queue depths are configurable and bounded
- ✅ Linear performance scaling with CPU count

**Testing Requirements:**
- Unit tests for queue management
- Performance tests with different CPU counts
- Load balancing verification tests
- Memory usage tests for per-CPU architecture

### Phase 3: Advanced Features (Weeks 7-9)

#### 3.1 Extended Attributes Support (Week 7)
**Deliverables:**
- Direct liburing xattr implementation
- Complete xattr operations suite (getxattr, setxattr, listxattr)
- ACL preservation support
- SELinux context preservation

**Acceptance Criteria:**
- ✅ All extended attributes are preserved
- ✅ POSIX ACLs are copied correctly
- ✅ SELinux contexts are preserved
- ✅ User-defined attributes work
- ✅ Handles missing xattr support gracefully

**Testing Requirements:**
- Unit tests for xattr operations
- Integration tests with various xattr types
- ACL preservation verification tests
- SELinux context tests (on SELinux systems)

#### 3.2 Advanced Error Recovery (Week 8)
**Deliverables:**
- Comprehensive error handling and recovery
- Retry logic for transient failures
- Partial failure recovery
- Detailed error reporting and logging

**Acceptance Criteria:**
- ✅ Handles disk full errors gracefully
- ✅ Retries transient failures automatically
- ✅ Recovers from partial copy failures
- ✅ Provides detailed error messages
- ✅ Logs errors with sufficient context

**Testing Requirements:**
- Unit tests for error conditions
- Integration tests with simulated failures
- Chaos engineering tests
- Error message verification tests

#### 3.3 Performance Optimization (Week 9)
**Deliverables:**
- Buffer pooling and reuse
- Memory-mapped file support for large files
- Direct I/O optimization
- Advanced batching strategies

**Acceptance Criteria:**
- ✅ Buffer pooling reduces memory allocations
- ✅ Memory-mapped files improve large file performance
- ✅ Direct I/O provides optimal performance when appropriate
- ✅ Batching reduces system call overhead
- ✅ Performance scales with system resources

**Testing Requirements:**
- Performance benchmarks for all optimizations
- Memory usage tests for buffer pooling
- Large file performance tests
- System resource utilization tests

### Phase 4: Production Readiness (Weeks 10-12)

#### 4.1 Comprehensive Testing Suite (Week 10)
**Deliverables:**
- Complete end-to-end test suite
- Property-based testing for edge cases
- Performance regression testing
- Cross-platform compatibility testing

**Acceptance Criteria:**
- ✅ All test categories pass consistently
- ✅ Property-based tests find edge cases
- ✅ Performance benchmarks establish baselines
- ✅ Cross-platform tests pass on target systems
- ✅ Test coverage exceeds 90%

**Testing Requirements:**
- Complete test suite execution
- Property-based test validation
- Performance baseline establishment
- Cross-platform compatibility verification

#### 4.2 Documentation and User Experience (Week 11)
**Deliverables:**
- Complete user documentation
- API documentation with examples
- Performance tuning guide
- Troubleshooting guide

**Acceptance Criteria:**
- ✅ User documentation is complete and accurate
- ✅ All public APIs are documented with examples
- ✅ Performance tuning guide helps users optimize
- ✅ Troubleshooting guide covers common issues
- ✅ Documentation builds without warnings

**Testing Requirements:**
- Documentation accuracy verification
- Example code validation
- User experience testing
- Documentation build tests

#### 4.3 Release Preparation (Week 12)
**Deliverables:**
- Final performance optimization
- Security audit and hardening
- Release packaging and distribution
- Community preparation

**Acceptance Criteria:**
- ✅ Performance meets or exceeds targets
- ✅ Security audit passes with no critical issues
- ✅ Release packages work on target platforms
- ✅ Community documentation is ready
- ✅ Release process is documented and tested

**Testing Requirements:**
- Final performance validation
- Security testing
- Release package verification
- Community readiness assessment

### Success Metrics

#### Performance Targets
- **Throughput**: >500 MB/s for same-filesystem copies on SSD
- **Latency**: <1ms per operation for small files
- **Scalability**: Linear scaling with CPU cores up to 32 cores
- **Memory**: <100MB base memory usage + 1MB per 1000 files

#### Quality Targets
- **Test Coverage**: >90% code coverage
- **Error Handling**: Graceful handling of all error conditions
- **Documentation**: 100% public API documentation
- **Compatibility**: Support for Linux kernel 5.6+ and Rust 1.70+

#### Reliability Targets
- **Data Integrity**: 100% file integrity verification
- **Metadata Preservation**: Complete metadata preservation
- **Error Recovery**: Recovery from all transient failures
- **Stability**: No memory leaks or crashes under normal operation

### Risk Mitigation

#### Technical Risks
- **io_uring Library Gaps**: Mitigated by direct liburing implementation
- **Cross-platform Compatibility**: Mitigated by comprehensive testing matrix
- **Performance Regression**: Mitigated by automated performance testing

#### Project Risks
- **Scope Creep**: Mitigated by strict phase boundaries and acceptance criteria
- **Timeline Delays**: Mitigated by parallel development and incremental delivery
- **Quality Issues**: Mitigated by comprehensive testing and code review process

This detailed implementation plan provides a clear roadmap for delivering a production-ready io_uring-sync utility with comprehensive testing, documentation, and performance optimization.

## References

### Official Documentation
- [io_uring Official Repository](https://github.com/axboe/liburing)
- [Linux man pages - copy_file_range](https://man7.org/linux/man-pages/man2/copy_file_range.2.html)
- [Linux man pages - io_uring](https://man7.org/linux/man-pages/man7/io_uring.7.html)

### Rust Libraries
- [rio - Pure Rust io_uring](https://github.com/spacejam/rio)
- [tokio-uring - Tokio integration](https://github.com/tokio-rs/io-uring)
- [Glommio - Thread-per-core framework](https://docs.rs/glommio)
- [Monoio - High-performance runtime](https://github.com/bytedance/monoio)
- [uring-fs - Async file operations](https://lib.rs/crates/uring-fs)

### Research and Articles
- [Tokio-uring exploration with Rust](https://developerlife.com/2024/05/25/tokio-uring-exploration-rust/)
- [Monoio introduction and architecture](https://www.cloudwego.io/blog/2023/04/17/introducing-monoio-a-high-performance-rust-runtime-based-on-io-uring/)

### Performance Resources
- [io_uring Performance Analysis](https://kernel.dk/io_uring.pdf)
- [Zero-Copy Operations in Linux](https://lwn.net/Articles/667550/)
- [copy_file_range Performance Benefits](https://lwn.net/Articles/720675/)

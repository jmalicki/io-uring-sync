# Semaphore-Based Concurrency Control Design

## Problem Statement

### The Unbounded Queue Problem

arsync's current breadth-first directory traversal creates an **unbounded queue** of pending file operations:

```
Directory Tree:
/data/
  ├── dir1/ (1000 files)
  ├── dir2/ (1000 files)
  ├── dir3/ (1000 files)
  └── dir4/ (1000 files)

Current Behavior (BFS):
1. Discover all 4 directories
2. Queue all 4,000 files for processing
3. Start copying files
4. Memory usage: ~400 MB for queued operations
5. Context switching overhead from too much concurrency
```

**Problems:**
1. **Memory Pressure**: Queuing thousands of operations consumes significant memory
2. **Poor Cache Locality**: Files from different directories interleaved, hurting CPU cache
3. **Excessive Context Switching**: Too many concurrent operations overwhelm the scheduler
4. **Resource Exhaustion**: Can hit file descriptor limits or kernel queue limits
5. **Unpredictable Performance**: No control over concurrency level

### The Solution: Bounded Concurrency with Semaphores

**Goal**: Limit the number of files being processed concurrently to a configurable maximum.

**Mechanism**: Use a semaphore that:
- Has a fixed number of permits (e.g., 1024)
- Each file operation acquires a permit before starting
- Releases the permit when the file is fully copied
- Directory traversal blocks when all permits are in use
- Automatically resumes when permits become available

**Benefits:**
1. **Bounded Memory**: Queue size limited by semaphore permits
2. **Better Cache Locality**: Fewer concurrent files means better CPU cache utilization
3. **Predictable Performance**: Configurable concurrency matches system capabilities
4. **Resource Protection**: Prevents file descriptor exhaustion
5. **Backpressure**: Discovery pauses when processing is saturated

## Design

### Semaphore Requirements

The semaphore must be:

1. **Async-Compatible**: Works with compio's async runtime (no blocking operations)
2. **Fair**: FIFO ordering to prevent starvation
3. **Cloneable**: Can be shared across async tasks via `Arc`
4. **Efficient**: Minimal overhead (lock-free when possible)
5. **Cancellation-Safe**: Handles task cancellation without leaking permits

### Inspiration: Tokio's Semaphore

[Tokio's Semaphore](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html) provides a good reference implementation:

```rust
// Tokio pattern (for reference):
let semaphore = Arc::new(Semaphore::new(1024));

let permit = semaphore.acquire().await?;  // Waits if no permits available
// Do work...
drop(permit);  // Automatically releases on drop (RAII)
```

**Key features we need:**
- `new(permits)` - Create semaphore with initial permits
- `acquire() -> impl Future<Output = SemaphorePermit>` - Acquire permit (async wait)
- `SemaphorePermit` - RAII guard that releases on drop
- `try_acquire()` - Non-blocking acquire attempt
- `available_permits()` - Query available permits (for debugging/metrics)

### Compio-Compatible Implementation

Since compio doesn't provide a built-in semaphore, we'll implement one using:

**Option 1: std::sync primitives with async wrapper**
```rust
use std::sync::{Arc, Mutex, Condvar};
use compio::runtime::spawn;

struct Semaphore {
    state: Arc<(Mutex<SemaphoreState>, Condvar)>,
}

struct SemaphoreState {
    available: usize,
    waiters: VecDeque<Waker>,
}
```

**Option 2: Atomic-based lock-free semaphore**
```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct Semaphore {
    permits: Arc<AtomicUsize>,
    waiters: Arc<Mutex<VecDeque<Waker>>>,
}
```

**Recommended: Option 2** (lock-free fast path, mutex only for waiters)

### API Design

```rust
/// A compio-compatible async semaphore for bounding concurrency
pub struct Semaphore {
    /// Available permits (atomic for lock-free fast path)
    permits: Arc<AtomicUsize>,
    /// Waiters queue for when permits are exhausted
    waiters: Arc<Mutex<VecDeque<Waker>>>,
}

impl Semaphore {
    /// Create a new semaphore with the given number of permits
    pub fn new(permits: usize) -> Self;
    
    /// Acquire a permit, waiting asynchronously if none available
    pub async fn acquire(&self) -> SemaphorePermit<'_>;
    
    /// Try to acquire a permit without waiting
    pub fn try_acquire(&self) -> Option<SemaphorePermit<'_>>;
    
    /// Get the number of available permits
    pub fn available_permits(&self) -> usize;
}

/// RAII guard that releases the semaphore permit on drop
pub struct SemaphorePermit<'a> {
    semaphore: &'a Semaphore,
}

impl Drop for SemaphorePermit<'_> {
    fn drop(&mut self) {
        // Release permit and wake one waiter
        self.semaphore.release();
    }
}
```

### Integration Points

#### 1. CLI Configuration

Add flag to `src/cli.rs`:

```rust
#[derive(Parser, Debug)]
pub struct Args {
    // ... existing fields ...
    
    /// Maximum number of files to process concurrently
    /// 
    /// Controls memory usage and system load. Higher values increase
    /// throughput but consume more memory and file descriptors.
    /// 
    /// Default: 1024 (good balance for most systems)
    /// High-performance: 2048-4096 (NVMe, lots of RAM)
    /// Conservative: 256-512 (spinning disks, limited RAM)
    #[arg(long, default_value = "1024")]
    pub max_files_in_flight: usize,
}
```

#### 2. Directory Traversal Integration

Modify `src/directory.rs` to use the semaphore:

```rust
async fn traverse_and_copy_directory_iterative(
    // ... existing parameters ...
    semaphore: Arc<Semaphore>,  // NEW: Shared semaphore
) -> Result<()> {
    // ... existing setup ...
    
    // Process each directory entry
    for entry_result in entries {
        // ... get entry ...
        
        // Acquire semaphore permit BEFORE dispatching
        let permit = semaphore.acquire().await;
        
        // Dispatch the operation (permit moved into closure)
        let receiver = dispatcher.dispatch(move || async move {
            // Permit held during entire operation
            let result = process_file_or_directory(...).await;
            drop(permit);  // Released when operation completes
            result
        })?;
        
        futures.push(receiver);
    }
    
    // Wait for all operations
    futures::future::try_join_all(futures).await?;
}
```

#### 3. File Operation Integration

Ensure file copying holds the permit:

```rust
async fn process_file(
    src: PathBuf,
    dst: PathBuf,
    _permit: SemaphorePermit<'_>,  // Held until function completes
) -> Result<()> {
    // Copy file...
    copy_file(&src, &dst, args).await?;
    
    // Permit automatically released when _permit is dropped
    Ok(())
}
```

### Behavior and Guarantees

#### Backpressure Flow

```
State: 1024 permits available
┌─────────────────────────────────────────────────────────┐
│ Discover dir1/ with 500 files                          │
│ → Acquire 500 permits (524 remaining)                  │
│ → Start copying 500 files                              │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ Discover dir2/ with 800 files                          │
│ → Acquire 524 permits (0 remaining)                    │
│ → Start copying 524 more files (1024 total in-flight)  │
│ → Need 276 more permits... WAIT                        │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ File from dir1/ completes                              │
│ → Release 1 permit (1 available)                       │
│ → Wake 1 waiting acquire() call                        │
│ → Start copying 1 file from dir2/                      │
└─────────────────────────────────────────────────────────┘

Result: Never more than 1024 files in-flight
```

#### Discovery Pausing

**Without semaphore (current):**
```
Timeline:
0s:  Discover 10,000 files → Queue all 10,000 → Start processing
1s:  Queue: 9,500 pending, 500 in-flight
2s:  Queue: 9,000 pending, 500 in-flight (memory wasted on queue)
...
20s: Queue: 0 pending, 500 in-flight (finally!)
```

**With semaphore:**
```
Timeline:
0s:  Discover 1,024 files → Queue 1,024 → Start processing
0s:  Try to discover more → Semaphore full → PAUSE discovery
1s:  50 files complete → 50 permits freed → Discover 50 more files
2s:  100 files complete → 100 permits freed → Discover 100 more files
...
10s: Steady state: ~1,024 in-flight, discovery keeps pace with completion
```

**Benefits:**
- Lower memory usage (only queue what can be processed)
- Better cache locality (process nearby files together)
- Smoother resource usage (no spike then plateau)

### Performance Tuning

#### Choosing max-files-in-flight

**Factors to consider:**

1. **Storage Type:**
   - **NVMe SSD**: 2048-4096 (high queue depth, parallel flash channels)
   - **SATA SSD**: 1024-2048 (moderate parallelism)
   - **HDD**: 256-512 (seek time dominates, limited benefit from high concurrency)
   - **Network storage**: 512-1024 (depends on latency and bandwidth)

2. **File Size:**
   - **Many small files**: Higher limit (2048+) for maximum throughput
   - **Large files**: Lower limit (512-1024) to avoid memory exhaustion
   - **Mixed sizes**: Default 1024 works well

3. **System Resources:**
   - **RAM**: Each in-flight file needs buffer space (64-128 KB)
     - 1024 files × 128 KB = 128 MB buffer memory
     - 4096 files × 128 KB = 512 MB buffer memory
   - **File Descriptors**: Each file holds 2 FDs (source + destination)
     - 1024 files = 2048 FDs (check `ulimit -n`)
   - **CPU Cores**: More cores can handle more concurrency
     - 8 cores: 512-1024 files
     - 16 cores: 1024-2048 files
     - 32+ cores: 2048-4096 files

4. **io_uring Queue Depth:**
   - Semaphore limit should be ≤ `queue_depth / 2`
   - Queue depth 8192 → max 4096 files
   - Queue depth 4096 → max 2048 files

#### Recommended Configurations

**Conservative (default):**
```bash
arsync -a --source /data --destination /backup \
  --max-files-in-flight 1024 \
  --queue-depth 4096
```

**High-throughput (NVMe, 16+ cores, 32GB+ RAM):**
```bash
arsync -a --source /data --destination /backup \
  --max-files-in-flight 4096 \
  --queue-depth 16384 \
  --cpu-count 16
```

**Low-resource (HDD, limited RAM):**
```bash
arsync -a --source /data --destination /backup \
  --max-files-in-flight 256 \
  --queue-depth 1024 \
  --cpu-count 4
```

### Implementation Plan

#### Phase 1: Semaphore Implementation

Create `src/semaphore.rs`:

```rust
//! Async semaphore for bounding concurrency in compio
//!
//! Provides a semaphore primitive compatible with compio's async runtime
//! to limit the number of concurrent file operations.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

/// A compio-compatible async semaphore
///
/// Limits concurrent operations by issuing a fixed number of permits.
/// Operations must acquire a permit before proceeding and release it
/// when complete (via RAII `SemaphorePermit` guard).
#[derive(Clone)]
pub struct Semaphore {
    inner: Arc<SemaphoreInner>,
}

struct SemaphoreInner {
    /// Available permits (atomic for lock-free fast path)
    permits: AtomicUsize,
    /// Waiters queue (mutex-protected, only accessed when no permits)
    waiters: Mutex<VecDeque<Waker>>,
}

impl Semaphore {
    /// Create a new semaphore with the given number of permits
    pub fn new(permits: usize) -> Self {
        Self {
            inner: Arc::new(SemaphoreInner {
                permits: AtomicUsize::new(permits),
                waiters: Mutex::new(VecDeque::new()),
            }),
        }
    }
    
    /// Acquire a permit, waiting asynchronously if none available
    pub async fn acquire(&self) -> SemaphorePermit<'_> {
        AcquireFuture { semaphore: self }.await
    }
    
    /// Try to acquire a permit without waiting
    pub fn try_acquire(&self) -> Option<SemaphorePermit<'_>> {
        // Fast path: atomic decrement if permits available
        let mut current = self.inner.permits.load(Ordering::Acquire);
        loop {
            if current == 0 {
                return None;  // No permits available
            }
            
            match self.inner.permits.compare_exchange_weak(
                current,
                current - 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(SemaphorePermit { semaphore: self }),
                Err(actual) => current = actual,  // Retry with updated value
            }
        }
    }
    
    /// Get the number of available permits
    pub fn available_permits(&self) -> usize {
        self.inner.permits.load(Ordering::Acquire)
    }
    
    /// Release a permit (called by SemaphorePermit::drop)
    fn release(&self) {
        // Increment available permits
        self.inner.permits.fetch_add(1, Ordering::Release);
        
        // Wake one waiter if any
        if let Ok(mut waiters) = self.inner.waiters.lock() {
            if let Some(waker) = waiters.pop_front() {
                waker.wake();
            }
        }
    }
    
    /// Add a waiter to the queue (called by AcquireFuture)
    fn add_waiter(&self, waker: Waker) {
        if let Ok(mut waiters) = self.inner.waiters.lock() {
            waiters.push_back(waker);
        }
    }
}

/// RAII guard that releases the semaphore permit on drop
pub struct SemaphorePermit<'a> {
    semaphore: &'a Semaphore,
}

impl Drop for SemaphorePermit<'_> {
    fn drop(&mut self) {
        self.semaphore.release();
    }
}

/// Future that resolves when a semaphore permit is acquired
struct AcquireFuture<'a> {
    semaphore: &'a Semaphore,
}

impl<'a> Future for AcquireFuture<'a> {
    type Output = SemaphorePermit<'a>;
    
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Try fast path first
        if let Some(permit) = self.semaphore.try_acquire() {
            return Poll::Ready(permit);
        }
        
        // No permits available - register waker and wait
        self.semaphore.add_waiter(cx.waker().clone());
        
        // Try again in case permit became available while registering
        if let Some(permit) = self.semaphore.try_acquire() {
            return Poll::Ready(permit);
        }
        
        Poll::Pending
    }
}
```

#### Phase 2: CLI Integration

Update `src/cli.rs`:

```rust
#[derive(Parser, Debug, Clone)]
pub struct Args {
    // ... existing fields ...
    
    /// Maximum number of files to process concurrently
    /// 
    /// Controls memory usage and system load by limiting how many files
    /// are being copied simultaneously. When this limit is reached, directory
    /// discovery pauses until in-flight operations complete.
    /// 
    /// Tuning guidelines:
    /// - Default: 1024 (balanced for most systems)
    /// - NVMe + 16+ cores + 32GB RAM: 2048-4096
    /// - SATA SSD + 8 cores + 16GB RAM: 1024-2048
    /// - HDD or limited RAM: 256-512
    /// 
    /// Memory usage: ~128 KB per file × max-files-in-flight
    /// File descriptors: 2 FDs per file × max-files-in-flight
    #[arg(long, default_value = "1024", value_name = "COUNT")]
    pub max_files_in_flight: usize,
}
```

#### Phase 3: Directory Traversal Integration

Update `src/directory.rs`:

```rust
use crate::semaphore::{Semaphore, SemaphorePermit};

async fn traverse_and_copy_directory_iterative(
    // ... existing parameters ...
    args: &Args,
) -> Result<()> {
    // Create semaphore with configured limit
    let semaphore = Arc::new(Semaphore::new(args.max_files_in_flight));
    
    // ... existing setup ...
    
    // Process the directory with semaphore
    let result = process_directory_entry_with_compio(
        dispatcher,
        initial_src,
        initial_dst,
        file_ops_static,
        _copy_method,
        shared_stats.clone(),
        shared_hardlink_tracker.clone(),
        args_static,
        semaphore,  // NEW: Pass semaphore
    )
    .await;
    
    result
}

async fn process_directory_entry_with_compio(
    dispatcher: &'static Dispatcher,
    src_path: PathBuf,
    dst_path: PathBuf,
    // ... existing parameters ...
    semaphore: Arc<Semaphore>,  // NEW: Semaphore for bounding
) -> Result<()> {
    let extended_metadata = ExtendedMetadata::new(&src_path).await?;
    
    if extended_metadata.is_dir() {
        // Process directory entries
        let entries = std::fs::read_dir(&src_path)?;
        
        let mut futures = Vec::new();
        
        for entry_result in entries {
            let entry = entry_result?;
            let child_src_path = entry.path();
            let child_dst_path = dst_path.join(entry.file_name());
            
            // Clone semaphore for child task
            let semaphore = semaphore.clone();
            
            // Dispatch child processing
            let receiver = dispatcher.dispatch(move || async move {
                // Acquire permit FIRST (blocks if max in-flight reached)
                let permit = semaphore.acquire().await;
                
                // Process the entry (permit held throughout)
                let result = process_directory_entry_with_compio(
                    dispatcher,
                    child_src_path,
                    child_dst_path,
                    file_ops,
                    copy_method,
                    stats,
                    hardlink_tracker,
                    args,
                    semaphore,
                ).await;
                
                // Permit released here (automatic via Drop)
                drop(permit);
                
                result
            })?;
            
            futures.push(receiver);
        }
        
        // Wait for all children
        futures::future::try_join_all(futures).await?;
    } else {
        // File processing - permit already acquired by caller
        copy_file(&src_path, &dst_path, args).await?;
    }
    
    Ok(())
}
```

### Alternative Design: Permit Acquisition Strategy

**Option A: Acquire per file (recommended)**
- Acquire permit before starting file copy
- Release when copy completes
- Simple, straightforward backpressure

**Option B: Acquire per directory entry**
- Acquire permit before processing any entry (file or directory)
- Directories release immediately after creating
- Files hold until copy completes
- More complex but better granularity

**Option C: Two-level semaphore**
- One semaphore for total in-flight operations
- Separate semaphore for file I/O operations
- Maximum flexibility but added complexity

**Recommendation: Start with Option A** (simplest, most predictable)

### Testing Strategy

#### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[compio::test]
    async fn test_semaphore_basic() {
        let sem = Semaphore::new(2);
        
        let permit1 = sem.acquire().await;
        let permit2 = sem.acquire().await;
        assert_eq!(sem.available_permits(), 0);
        
        drop(permit1);
        assert_eq!(sem.available_permits(), 1);
        
        drop(permit2);
        assert_eq!(sem.available_permits(), 2);
    }
    
    #[compio::test]
    async fn test_semaphore_blocking() {
        let sem = Arc::new(Semaphore::new(1));
        
        let permit1 = sem.acquire().await;
        
        // Spawn task that tries to acquire (should block)
        let sem2 = sem.clone();
        let handle = compio::runtime::spawn(async move {
            sem2.acquire().await;
            42
        });
        
        // Give spawned task time to block
        compio::runtime::yield_now().await;
        
        // Release permit
        drop(permit1);
        
        // Spawned task should now complete
        let result = handle.await;
        assert_eq!(result, 42);
    }
    
    #[compio::test]
    async fn test_try_acquire() {
        let sem = Semaphore::new(1);
        
        let permit = sem.try_acquire();
        assert!(permit.is_some());
        
        let permit2 = sem.try_acquire();
        assert!(permit2.is_none());
        
        drop(permit);
        
        let permit3 = sem.try_acquire();
        assert!(permit3.is_some());
    }
}
```

#### Integration Tests

```rust
#[compio::test]
async fn test_bounded_directory_copy() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source");
    let dest = temp.path().join("dest");
    
    // Create 100 files
    fs::create_dir(&source).unwrap();
    for i in 0..100 {
        fs::write(source.join(format!("file{}.txt", i)), b"test").unwrap();
    }
    
    let args = Args {
        source: source.clone(),
        destination: dest.clone(),
        max_files_in_flight: 10,  // Low limit for testing
        ..Default::default()
    };
    
    // Copy with bounded concurrency
    let result = sync_directories(&args).await;
    assert!(result.is_ok());
    
    // Verify all files copied
    for i in 0..100 {
        assert!(dest.join(format!("file{}.txt", i)).exists());
    }
}
```

### Monitoring and Observability

Add metrics to track semaphore usage:

```rust
/// Statistics for semaphore usage
pub struct SemaphoreStats {
    /// Maximum permits (configured limit)
    pub max_permits: usize,
    /// Current available permits
    pub available_permits: usize,
    /// Current in-flight operations
    pub in_flight: usize,
    /// Total operations completed
    pub completed: usize,
    /// Total wait time (nanoseconds)
    pub total_wait_time_ns: u64,
}

impl Semaphore {
    /// Get current usage statistics
    pub fn stats(&self) -> SemaphoreStats {
        let available = self.available_permits();
        SemaphoreStats {
            max_permits: self.max_permits,
            available_permits: available,
            in_flight: self.max_permits - available,
            // ... other stats ...
        }
    }
}
```

Progress reporting can show:

```
Discovered: 1,523 files | In-flight: 1,024/1,024 | Completed: 847 files
[==============>              ] 55% | 1.5 GB/s | ETA: 0:00:03
                              ↑ Semaphore at capacity (backpressure active)
```

### Edge Cases and Considerations

#### 1. **Deadlock Prevention**

Ensure operations don't hold permits while waiting for other permits:

```rust
// ❌ BAD: Potential deadlock
let permit1 = sem.acquire().await;
let permit2 = sem.acquire().await;  // Deadlock if semaphore has 1 permit

// ✓ GOOD: Single permit per operation
let permit = sem.acquire().await;
do_work().await;
drop(permit);
```

#### 2. **Permit Leaks**

`SemaphorePermit` uses RAII (Drop trait) to prevent leaks:

```rust
// Permit released even on error
let permit = semaphore.acquire().await;
copy_file(&src, &dst).await?;  // Even if this errors, permit is released
// Permit dropped here automatically
```

#### 3. **Fairness**

Use `VecDeque` for FIFO ordering of waiters to prevent starvation:

```rust
// First waiter added gets woken first
waiters.push_back(waker);  // Add to back
waiters.pop_front();       // Take from front (FIFO)
```

#### 4. **Cancellation Safety**

If a task is cancelled mid-`acquire()`:
- Waker is dropped
- Permit is not acquired
- No leak occurs (waker removal is best-effort)

### Comparison with Other Approaches

#### vs. Channel-Based Backpressure

```rust
// Channel approach:
let (tx, rx) = channel(1024);
// Send work items
tx.send(work).await;  // Blocks when channel full
```

**Pros of semaphore approach:**
- More flexible (not tied to work item types)
- Lower overhead (atomic fast path)
- Clearer semantics (permits = concurrency)

#### vs. Manual Counting

```rust
// Manual approach:
let in_flight = Arc::new(AtomicUsize::new(0));
while in_flight.load(Ordering::Acquire) >= MAX {
    yield_now().await;  // Busy-wait (wasteful)
}
in_flight.fetch_add(1, Ordering::Release);
```

**Pros of semaphore approach:**
- No busy-waiting (efficient)
- Automatic wakeup (no polling)
- RAII cleanup (no manual decrement)

### Future Enhancements

1. **Adaptive Limits**: Automatically adjust based on system load
2. **Per-CPU Semaphores**: Separate limits per CPU core
3. **Priority Permits**: Different limits for different file types
4. **Metrics Collection**: Detailed histograms of wait times
5. **Dynamic Tuning**: Adjust limits based on observed performance

### References

**Tokio Semaphore:**
- [Implementation](https://github.com/tokio-rs/tokio/blob/master/tokio/src/sync/semaphore.rs)
- [Documentation](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html)

**Async Synchronization Primitives:**
- [Async Book: Synchronization](https://rust-lang.github.io/async-book/04_pinning/01_chapter.html)
- [crossbeam](https://docs.rs/crossbeam/latest/crossbeam/) - Lock-free data structures

**Backpressure Patterns:**
- [Async Streams with Backpressure](https://tokio.rs/tokio/topics/streams)
- [Buffering and Backpressure](https://without.boats/blog/poll-drop/)

## Summary

The semaphore-based concurrency control provides:

✅ **Bounded Memory**: Queue size limited by permit count
✅ **Predictable Performance**: Configurable concurrency level
✅ **Resource Protection**: Prevents FD exhaustion, kernel queue overflow
✅ **Better Locality**: Fewer concurrent files = better cache utilization
✅ **Backpressure**: Discovery pauses when processing saturated
✅ **Simple Integration**: Drop-in addition to existing code
✅ **Observable**: Can report in-flight count in progress display

This is a critical improvement for production use, especially when copying large directory trees with thousands of files.


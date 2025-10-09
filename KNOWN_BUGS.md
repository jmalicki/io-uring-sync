# Known Bugs

## CRITICAL: Deadlock on "Too many open files" Error

**Severity**: CRITICAL - Causes hang/deadlock  
**Status**: DISCOVERED during benchmarking  
**Date**: 2025-10-08

### Symptoms

When arsync encounters "Too many open files" (EMFILE) error:
1. Logs WARN messages about failing to copy files
2. **Does NOT exit gracefully**
3. **HANGS/DEADLOCKS** instead of failing

### Reproduction

```bash
# With default settings, copying 1000+ small files causes:
./target/release/arsync -a --source /path/to/1000-files/ --destination /dest/

# Output shows:
WARN Failed to copy file ... Too many open files (os error 24)
# Then hangs forever (deadlock)
```

### Expected Behavior

When hitting EMFILE, arsync should:
1. ✅ Log the error (currently does this)
2. ❌ **Clean up open file descriptors**
3. ❌ **Exit with non-zero status**
4. ❌ **Never hang/deadlock**

### Actual Behavior

- ❌ Hangs indefinitely
- ❌ Does not exit
- ❌ Does not clean up resources
- ❌ Requires kill -9

### Workaround

Add `--max-files-in-flight 100` to limit concurrent operations:
```bash
arsync -a --max-files-in-flight 100 --source ... --destination ...
```

### Root Cause (Hypothesis)

Likely in `src/directory.rs`:
- Semaphore permits may not be released properly when file open fails
- Error handling path doesn't clean up pending operations
- Tasks may be waiting on semaphore that never gets released

### Priority

**MUST FIX BEFORE 1.0 RELEASE**

This is a critical reliability bug:
- Violates "fail fast" principle
- Makes debugging difficult (looks hung, not failed)
- Could cause issues in production use
- Undermines trust in the tool

### Investigation Needed

1. Check semaphore release in error paths
2. Verify all `acquire()` have matching releases
3. Add timeout protection in compio runtime
4. Ensure file descriptor cleanup on error
5. Add test that deliberately triggers EMFILE

### Testing

Added to benchmark scripts:
- Timeout protection (5 min for tests that should complete in <2 min)
- `--max-files-in-flight 100` workaround
- Clear error message if timeout occurs

### References

- Issue discovered during quick benchmark run
- Logs show "Too many open files" followed by hang
- Process still running but not making progress
- Requires `pkill -9 arsync` to kill

---

## Fix Strategy

### Short-term (for benchmarking)
✅ Add `--max-files-in-flight 100` to all tests  
✅ Add timeout protection to detect hangs  
✅ Document issue clearly

### Long-term (before 1.0)
- [ ] Fix semaphore release on error paths
- [ ] Add proper cleanup in error handling
- [ ] Add test that triggers EMFILE deliberately
- [ ] Ensure graceful failure, never hang
- [ ] Consider backpressure when hitting limits


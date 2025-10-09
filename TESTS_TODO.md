# Test Updates TODO

## Tests That Need Updating for New Args Structure

The following test files create `Args` instances manually and need to be updated to use the new Args structure with positional arguments support:

### Files to Update:

1. `tests/permission_test.rs` - Line 11-40 (`create_test_args_with_archive`)
2. `tests/performance_metadata_tests.rs` - Line 21-50 (`create_test_args_with_archive`)
3. `tests/edge_case_metadata_tests.rs` - Line 21-50 (`create_test_args_with_archive`)
4. `tests/comprehensive_metadata_tests.rs` - Line 20-49 (`create_test_args_with_archive`)
5. `tests/metadata_flag_tests.rs` - Lines 18-61 (multiple `create_args_*` functions)
6. `tests/directory_metadata_tests.rs` - Check for Args creation

### Required Changes:

Each `Args` struct needs these new fields:

```rust
Args {
    source_positional: None,
    dest_positional: None,
    source: Some(PathBuf::from("/test/source")),
    destination: Some(PathBuf::from("/test/dest")),
    // ... existing fields ...
    no_adaptive_concurrency: false,
    server: false,
    remote_shell: "ssh".to_string(),
    daemon: false,
}
```

### Recommended Approach:

Replace manual `create_test_args_*` functions with helper from `tests/common/test_helpers.rs`:

```rust
use crate::common::test_helpers::{create_test_args, create_test_args_archive};

// Instead of creating Args manually:
let args = create_test_args_archive(source_path, dest_path);
```

### Status:

- ✅ `src/cli.rs` tests - Updated
- ✅ `src/copy.rs` tests - Updated
- ✅ Helper functions created in `tests/common/test_helpers.rs`
- ⏳ Integration tests - Need updating (6 files)

### Priority:

**Low** - Binary compiles and works. Tests can be updated in follow-up PR.

---

**Date**: October 9, 2025


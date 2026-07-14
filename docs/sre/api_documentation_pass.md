# API Documentation Pass

**Document:** Public API Documentation Requirements  
**Version:** 1.0.0  
**Status:** Active  
**Last Updated:** 2026-07-12  

---

## Overview

All public functions in core crates MUST have comprehensive documentation including:
- Description of what the function does
- `# Errors` section documenting all error conditions
- `# Panics` section (or explicit "This function never panics")
- `# Examples` section with compilable code examples

---

## Priority Crates

| Crate | Public Functions | Documented | Gap |
|-------|-----------------|------------|-----|
| ferro-common | ~50 | ~35 | ~15 |
| ferro-core | ~120 | ~80 | ~40 |
| ferro-auth | ~80 | ~55 | ~25 |
| ferro-dav | ~60 | ~40 | ~20 |
| ferro-crypto | ~30 | ~20 | ~10 |
| ferro-circuit-breaker | ~15 | ~10 | ~5 |
| ferro-health | ~10 | ~8 | ~2 |
| ferro-rate-limiter | ~15 | ~10 | ~5 |
| **Total** | **~380** | **~258** | **~122** |

---

## Documentation Template

```rust
/// Brief description of what the function does.
///
/// Longer description if needed, explaining the purpose,
/// behavior, and any important details.
///
/// # Arguments
///
/// * `param1` - Description of param1
/// * `param2` - Description of param2
///
/// # Returns
///
/// Returns `Ok(value)` on success, or `Err(error)` on failure.
///
/// # Errors
///
/// This function returns `Err(Error::NotFound)` if the resource
/// does not exist.
///
/// This function returns `Err(Error::Unauthorized)` if the
/// provided credentials are invalid.
///
/// This function returns `Err(Error::Conflict)` if there is
/// a version conflict.
///
/// # Panics
///
/// This function never panics.
///
/// # Examples
///
/// ```
/// use ferro_core::storage::StorageEngine;
///
/// let engine = StorageEngine::new_memory();
/// let result = engine.get("path/to/file").await;
/// assert!(result.is_ok());
/// ```
pub async fn get(&self, path: &str) -> Result<Option<FileData>, Error> {
    // ...
}
```

---

## Documentation Standards

### Error Documentation

Every function returning `Result` MUST document:
1. All possible error variants that can be returned
2. The conditions under which each error occurs
3. Any error recovery strategies

```rust
/// # Errors
///
/// * `Error::NotFound` - File does not exist at the given path
/// * `Error::PermissionDenied` - User lacks read permission
/// * `Error::StorageBackend` - Underlying storage system failure
/// * `Error::PathTraversal` - Path contains ".." or other traversal sequences
```

### Panic Documentation

Every function MUST document panic behavior:
```rust
/// # Panics
///
/// Panics if `path` is empty. Use `is_empty()` check before calling.
```

Or:
```rust
/// # Panics
///
/// This function never panics.
```

### Example Documentation

Every public function SHOULD have at least one compilable example:
```rust
/// # Examples
///
/// ```
/// let hash = ferro_crypto::hash::sha256(b"hello world");
/// assert_eq!(hash.len(), 32);
/// ```
```

---

## Implementation Steps

### Step 1: Enable Documentation Warnings

Add to each priority crate's lib.rs:
```rust
#![warn(missing_docs)]
#![warn(missing_doc_code_examples)]
```

### Step 2: Fix Missing Doc Comments

For each crate, systematically add documentation to all public items:
1. Structs and enums
2. Methods and functions
3. Trait implementations
4. Module-level documentation

### Step 3: Add Error Documentation

For all functions returning Result:
1. Add `# Errors` section
2. Document each possible error variant
3. Explain when each error occurs

### Step 4: Add Panic Documentation

For all public functions:
1. Add `# Panics` section
2. Document any panic conditions
3. Or explicitly state "This function never panics"

### Step 5: Add Examples

For core public APIs:
1. Add `# Examples` section
2. Write compilable code examples
3. Verify examples compile with `cargo test --doc`

### Step 6: Verify Documentation

```bash
# Build documentation
cargo doc --workspace --no-deps --open

# Test documentation examples
cargo test --doc --workspace

# Check for missing docs warnings
cargo clippy --workspace -- -W missing-docs
```

---

## Tracking

| Crate | Status | Last Updated | Notes |
|-------|--------|--------------|-------|
| ferro-common | In Progress | - | Priority: high |
| ferro-core | In Progress | - | Priority: high |
| ferro-auth | In Progress | - | Priority: high |
| ferro-dav | Not Started | - | Priority: high |
| ferro-crypto | Not Started | - | Priority: medium |
| ferro-circuit-breaker | Not Started | - | Priority: medium |
| ferro-health | Not Started | - | Priority: low |
| ferro-rate-limiter | Not Started | - | Priority: low |

---

## Quality Gates

| Gate | Threshold | Blocking |
|------|-----------|----------|
| missing_docs warnings | 0 | Advisory |
| missing_doc_code_examples | 0 | Advisory |
| cargo test --doc | Pass | Blocking |
| Doc coverage (rustdoc) | > 90% | Advisory |

---

## Timeline

| Week | Focus | Deliverable |
|------|-------|-------------|
| Week 1 | ferro-common, ferro-crypto | Documentation complete |
| Week 2 | ferro-auth, ferro-circuit-breaker | Documentation complete |
| Week 3 | ferro-core (partial) | 50% documented |
| Week 4 | ferro-core (complete), ferro-dav | Documentation complete |
| Week 5 | ferro-health, ferro-rate-limiter, review | All crates documented |
| Week 6 | Examples compilation, CI integration | CI gate active |

# Ferro Migration Guide

Lessons learned from migrating ferro's dependencies (crawlkit, clawdius, suture).

## Key Patterns

### Import Path Updates

Always update ALL import sites, not just the first match:

```rust
// Before
use suture::prelude::*;

// After
use new_crate::prelude::*;
```

Search for every occurrence:
```bash
rg -l "old_crate" --type rust
```

### Type Wrapper Mismatches

Ferro uses wrapper types extensively. When changing crates, verify signatures:

```rust
// Old crate expected
fn process(input: Zeroizing<String>) -> Result<()>

// New crate expects
fn process(input: &str) -> Result<()>

// Adapter needed:
fn process_wrapper(input: Zeroizing<String>) -> Result<()> {
    new_crate::process(input.as_ref())
}
```

### Module Re-exports

After adding new modules to `src/lib.rs`, re-export them for downstream:

```rust
pub mod new_module;
pub use new_module::PublicType;
```

### Feature Flags

Check if old crate features map 1:1 to new crate features:

```toml
# Old
suture = { version = "0.1", features = ["full"] }

# New - feature names may differ
new_crate = { version = "1.0", features = ["std"] }
```

## Verification Order

1. `cargo check --workspace` — catch compilation errors
2. `cargo clippy --workspace -- -D warnings` — catch lint issues
3. `cargo test --workspace` — catch behavioral regressions
4. `cargo fmt --all -- --check` — catch formatting drift

## Common Ferro-Specific Issues

- **Zeroizing wrapper**: Many ferro functions use `Zeroizing<String>` for secrets. New crates may expect plain `String`.
- **Arc requirements**: Shared state often requires `Arc<T>`. Verify new crate accepts this.
- **Error types**: Different crates define different error enums. Map between them.

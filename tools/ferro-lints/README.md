# Ferro Custom Clippy Lints

Custom lint rules for enforcing Ferro-specific code quality invariants.

## Lints

### FERRO_SAFETY_COMMENT
Warns if `unsafe` blocks in non-test code lack a preceding `// SAFETY:` doc comment.

```rust
// BAD
let ptr = unsafe { raw_ptr.add(offset) };

// GOOD
// SAFETY: raw_ptr is valid for offset within allocated bounds
let ptr = unsafe { raw_ptr.add(offset) };
```

### FERRO_NO_UNWRAP_CRITICAL
Warns if `unwrap()` or `expect()` is used in critical path functions (server, auth, crypto crates).

```rust
// BAD
let config = load_config().unwrap();

// GOOD
let config = load_config().map_err(|e| Error::ConfigLoad(e))?;
```

### FERRO_SECRET_NO_DEBUG
Warns if types with fields named `*password*`, `*secret*`, `*token*`, or `*key*` derive `Debug`.

```rust
// BAD
#[derive(Debug)]
struct UserConfig {
    password: String,
}

// GOOD
struct UserConfig {
    password: String,
}
// Implement Debug manually with redaction
impl fmt::Debug for UserConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserConfig")
            .field("password", &"[REDACTED]")
            .finish()
    }
}
```

## Usage

```bash
# Build the lints (requires nightly toolchain)
cd tools/ferro-lints
cargo +nightly build

# The lint library will be generated as a dynamic library
# Copy it to the appropriate location for dylint to find it
```

## Requirements

- Rust nightly toolchain (required for `#![feature(rustc_private)]`)
- `rustc-dev` component installed: `rustup component add rustc-dev --toolchain nightly`

## CI Integration

Add to `.github/workflows/quality.yml`:
```yaml
  custom-lints:
    name: Custom Lints
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - name: Install rustc-dev component
        run: rustup component add rustc-dev
      - name: Build custom lints
        run: cargo build -p ferro-lints
      - name: Run custom lints
        run: cargo dylint ferro-lints -- --workspace
```

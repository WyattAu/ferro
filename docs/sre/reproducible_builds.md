# Reproducible Builds

**Document:** Reproducible Build Configuration  
**Version:** 1.0.0  
**Status:** Active  
**Last Updated:** 2026-07-12  

---

## Overview

Reproducible builds ensure that given the same source code, toolchain, and build configuration, the output binary is identical (bit-for-bit). This provides supply chain integrity and allows third parties to verify that distributed binaries were built from the published source.

---

## Configuration

### Build Settings

| Setting | Value | Purpose |
|---------|-------|---------|
| `opt-level` | `z` | Size optimization (consistent across builds) |
| `lto` | `true` | Link-time optimization (deterministic) |
| `codegen-units` | `1` | Single codegen unit (eliminates ordering variance) |
| `strip` | `true` | Strip debug symbols (consistent output) |
| `panic` | `abort` | No unwinding (eliminates personality tables) |

### Environment Variables

| Variable | Value | Purpose |
|----------|-------|---------|
| `CARGO_INCREMENTAL` | `0` | Disable incremental compilation |
| `SOURCE_DATE_EPOCH` | Build timestamp | Deterministic timestamps in metadata |

### Toolchain Pinning

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.95"
components = ["clippy", "rustfmt"]
targets = ["x86_64-unknown-linux-gnu"]
```

---

## Verification Process

### Step 1: Build from source

```bash
# Clone repository
git clone https://github.com/WyattAu/ferro.git
cd ferro

# Build release binary
CARGO_INCREMENTAL=0 cargo build --release -p ferro-server

# Record checksum
sha256sum target/release/ferro-server
```

### Step 2: Compare with distributed binary

```bash
# Download released binary
wget https://github.com/WyattAu/ferro/releases/download/v3.1.0/ferro-server

# Compare checksums
sha256sum ferro-server
# Should match Step 1 output
```

### Step 3: Verify with cargo-auditable

```bash
# Install cargo-auditable
cargo install cargo-auditable

# Build with embedded SBOM
cargo auditable build --release -p ferro-server

# Verify embedded dependencies
cargo auditable identify target/release/ferro-server
```

---

## Binary SBOM Embedding

`cargo-auditable` embeds a compressed SBOM directly into the binary, allowing runtime verification of all dependencies:

```bash
# Build with embedded SBOM
cargo auditable build --release -p ferro-server

# Extract SBOM from binary
cargo auditable identify target/release/ferro-server

# Audit embedded dependencies
cargo auditable audit target/release/ferro-server
```

---

## CI Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/reproducible-build.yml
name: Reproducible Build Verification

on:
  push:
    tags: ['v*']

jobs:
  verify-reproducible:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.95
      - name: Build binary
        run: |
          CARGO_INCREMENTAL=0 cargo build --release -p ferro-server
          sha256sum target/release/ferro-server > checksum-build.txt
      - name: Download release binary
        run: |
          wget -q https://github.com/${{ github.repository }}/releases/download/${{ github.ref_name }}/ferro-server-x86_64-unknown-linux-gnu
          sha256sum ferro-server-x86_64-unknown-linux-gnu > checksum-release.txt
      - name: Compare checksums
        run: |
          diff <(cut -d' ' -f1 checksum-build.txt) <(cut -d' ' -f1 checksum-release.txt)
```

---

## Known Limitations

| Factor | Impact | Mitigation |
|--------|--------|------------|
| Timestamps in metadata | Binary metadata may include build time | Use `SOURCE_DATE_EPOCH=0` for strict reproducibility |
| Compiler host differences | Cross-compilation may produce different output | Build on same target architecture |
| Feature flags | Different features produce different binaries | Document exact feature flags per release |
| System libraries | Linked system libraries may differ | Use bundled dependencies where possible |

---

## References

- [Reproducible Builds](https://reproducible-builds.org/)
- [cargo-auditable](https://github.com/rust-secure-code/cargo-auditable)
- [SLSA Framework](https://slsa.dev/)
- [in-toto](https://in-toto.io/)

# Plan: Add Continuous Fuzzing Targets with cargo-fuzz

## Context

The Ferro workspace already has a `fuzz/` directory with 8 existing fuzz targets covering proppatch, lock requests, XML escape, WASM magic, calcardav, config, path normalization, and API auth. The user wants to add 4 new targets focused on critical parsers: XML (WebDAV), JSON, vCard, and iCalendar.

## Current State

- `fuzz/` directory exists with `Cargo.toml` and 8 targets in `fuzz/fuzz_targets/`
- Existing `fuzz/Cargo.toml` already has `libfuzzer-sys`, `ferro-dav`, `serde_json`, and `quick-xml` dependencies
- The `fuzz_calcardav.rs` target already partially covers XML and iCalendar parsing but is combined with other logic
- New targets will provide focused fuzzing for each parser independently

## Files to Create/Modify

### 1. `fuzz/fuzz_targets/fuzz_xml.rs` (NEW)

Focused XML parser fuzzing using the actual `ferro_dav::xml_ext` functions:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use ferro_dav::xml_ext::{
    parse_calendar_query_time_range, parse_addressbook_query_filter,
    parse_multiget_hrefs, parse_sync_collection,
};

fuzz_target!(|data: &[u8]| {
    let _ = parse_calendar_query_time_range(data);
    let _ = parse_addressbook_query_filter(data);
    let _ = parse_multiget_hrefs(data);
    let _ = parse_sync_collection(data);
});
```

### 2. `fuzz/fuzz_targets/fuzz_json.rs` (NEW)

JSON deserialization fuzzing:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<Value>(s);
    }
});
```

### 3. `fuzz/fuzz_targets/fuzz_vcard.rs` (NEW)

vCard parser fuzzing using `ferro_dav::vcard::parse_vcard`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use ferro_dav::vcard::parse_vcard;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_vcard(s);
    }
});
```

### 4. `fuzz/fuzz_targets/fuzz_ical.rs` (NEW)

iCalendar parser fuzzing using `ferro_dav::ical::parse_ical`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use ferro_dav::ical::parse_ical;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_ical(s);
    }
});
```

### 5. `fuzz/Cargo.toml` (MODIFY)

Add 4 new `[[bin]]` entries for the new targets. Append after existing entries:

```toml
[[bin]]
name = "fuzz_xml"
path = "fuzz_targets/fuzz_xml.rs"
test = false
doc = false

[[bin]]
name = "fuzz_json"
path = "fuzz_targets/fuzz_json.rs"
test = false
doc = false

[[bin]]
name = "fuzz_vcard"
path = "fuzz_targets/fuzz_vcard.rs"
test = false
doc = false

[[bin]]
name = "fuzz_ical"
path = "fuzz_targets/fuzz_ical.rs"
test = false
doc = false
```

### 6. `scripts/run_fuzz.sh` (NEW)

Script to run all fuzz targets for 60 seconds each:

```bash
#!/bin/bash
set -euo pipefail

echo "Running fuzz targets for 60 seconds each..."

for target in fuzz_xml fuzz_json fuzz_vcard fuzz_ical; do
    echo "Fuzzing $target..."
    cargo fuzz run $target -- -max_total_time=60 || true
done

echo "Fuzzing complete."
```

## Verification

1. `cargo fuzz list` should show all 12 targets (8 existing + 4 new)
2. `cargo fuzz build` should compile all targets successfully
3. `scripts/run_fuzz.sh` should run each target for 60 seconds without panics

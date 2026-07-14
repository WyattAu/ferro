# Zero-Copy Parsing Implementation Plan

> **Version:** 1.0  
> **Author:** SRE Team  
> **Created:** 2026-07-12  
> **Status:** Proposed  
> **Duration:** 8 days  
> **Target:** Eliminate unnecessary string allocations across all parsers

---

## Executive Summary

This plan implements zero-copy parsing across Ferro's XML, iCal, and vCard parsers, plus path normalization. Current parsers allocate new `String` objects for every field parsed, even when the input data could be borrowed directly. This results in ~2,714 unnecessary allocations per request in hot paths.

**Key Metrics to Improve:**
- Allocations per parse: Reduce from ~45 to <10
- Parse latency: Target 30-50% reduction
- Memory pressure: Reduce peak RSS by 15-20%

---

## Current Architecture Analysis

### XML Parser (quick-xml)
- **File:** `crates/dav/src/xml_ext.rs:156-189` — `parse_calendar_query_time_range`
- **Issue:** Uses `String::from_utf8_lossy(...).to_string()` for every element name and attribute value
- **Impact:** 3-5 allocations per XML element parsed

### iCal Parser
- **File:** `crates/dav/src/ical.rs:25-49` — `unfold_lines` returns owned `String`
- **File:** `crates/dav/src/ical.rs:51-73` — `parse_property` allocates `HashMap<String, String>` and `String` for every property
- **File:** `crates/dav/src/ical.rs:75-129` — `parse_component_lines` clones strings for component names
- **Impact:** ~20 allocations per iCal event parsed

### vCard Parser
- **File:** `crates/dav/src/vcard.rs:85-109` — `unfold_lines` identical to iCal
- **File:** `crates/dav/src/vcard.rs:111-133` — `parse_property_line` allocates strings for every property
- **File:** `crates/dav/src/vcard.rs:135-157` — `parse_structured_name` and `parse_address` allocate Vec and Strings
- **Impact:** ~25 allocations per vCard parsed

### Path Normalization
- **File:** `crates/common/src/path.rs:6-27` — `normalize_path` always returns owned `String`
- **File:** `crates/common/src/path.rs:31-38` — `parent_path` calls `normalize_path` twice
- **Impact:** 2 allocations per path normalization, called on every WebDAV request

---

## Implementation Plan

### Day 1-2: XML Parser Zero-Copy

#### Changes

**1.1 Replace String allocations with Cow in XML attribute parsing**
- **File:** `crates/dav/src/xml_ext.rs:148-189`

```rust
// BEFORE (xml_ext.rs:148-189)
pub fn parse_calendar_query_time_range(body: &[u8]) -> Option<(String, String)> {
    // ...
    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();  // ALLOC
    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();   // ALLOC
    let val = String::from_utf8_lossy(&attr.value).to_string();         // ALLOC
    // ...
}

// AFTER
use std::borrow::Cow;

pub fn parse_calendar_query_time_range(body: &[u8]) -> Option<(Cow<'_, str>, Cow<'_, str>)> {
    if body.len() > 10 * 1024 * 1024 {
        return None;
    }

    let mut start = None;
    let mut end = None;

    let mut reader = quick_xml::Reader::from_reader(body);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref());  // ZERO-COPY
                let local = if let Some(stripped) = name.strip_prefix("C:") {
                    Cow::Borrowed(stripped)
                } else {
                    name
                };
                if local.as_ref() == "time-range" {
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref());  // ZERO-COPY
                        let val = String::from_utf8_lossy(&attr.value);        // ZERO-COPY
                        if key.as_ref() == "start" {
                            start = Some(val.into_owned());
                        } else if key.as_ref() == "end" {
                            end = Some(val.into_owned());
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    match (start, end) {
        (Some(s), Some(e)) => Some((Cow::Owned(s), Cow::Owned(e))),
        _ => None,
    }
}
```

**1.2 Apply same pattern to `parse_addressbook_query_filter`**
- **File:** `crates/dav/src/xml_ext.rs:194-234`

**1.3 Apply same pattern to `parse_sync_collection`**
- **File:** `crates/dav/src/xml_ext.rs:251-311`

**1.4 Apply same pattern to `parse_multiget_hrefs`**
- **File:** `crates/dav/src/xml_ext.rs:316-356`

#### Benchmark Targets (Day 1-2)
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| XML parse (1KB body) | ~15μs | - | < 8μs |
| Allocations per parse | 8-12 | - | 0-2 |
| Attribute access latency | ~2μs | - | < 1μs |

---

### Day 3-4: iCal Parser Zero-Copy

#### Changes

**3.1 Return borrowed strings from `parse_property`**
- **File:** `crates/dav/src/ical.rs:51-73`

```rust
// BEFORE (ical.rs:51-73)
fn parse_property(line: &str) -> Option<IcalProperty> {
    let (name_params, value) = line.split_once(':').map(|(n, v)| (n.trim(), v.trim()))?;
    // ...
    let name = parts.next()?.to_uppercase();  // ALLOC
    let mut params = HashMap::new();
    for part in parts {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            params.insert(k.trim().to_uppercase(), v.trim().to_string());  // ALLOC
        }
    }
    Some(IcalProperty {
        name,           // ALLOC
        params,         // ALLOC
        value: value.to_string(),  // ALLOC
    })
}

// AFTER
#[derive(Debug, Clone)]
pub struct IcalPropertyRef<'a> {
    pub name: &'a str,
    pub params: Vec<(&'a str, &'a str)>,
    pub value: &'a str,
}

fn parse_property_ref(line: &str) -> Option<IcalPropertyRef<'_>> {
    let (name_params, value) = line.split_once(':').map(|(n, v)| (n.trim(), v.trim()))?;
    if name_params.is_empty() {
        return None;
    }

    let mut parts = name_params.split(';');
    let name = parts.next()?.trim();  // BORROW

    let mut params = Vec::new();
    for part in parts {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            params.push((k.trim(), v.trim()));  // BORROW
        }
    }

    Some(IcalPropertyRef {
        name,
        params,
        value,
    })
}

// Keep owned version for backward compatibility
fn parse_property(line: &str) -> Option<IcalProperty> {
    let parsed = parse_property_ref(line)?;
    Some(IcalProperty {
        name: parsed.name.to_uppercase(),
        params: parsed.params.into_iter()
            .map(|(k, v)| (k.to_uppercase(), v.to_string()))
            .collect(),
        value: parsed.value.to_string(),
    })
}
```

**3.2 Optimize `unfold_lines` to return `Cow<str>`**
- **File:** `crates/dav/src/ical.rs:25-49`

```rust
// BEFORE
fn unfold_lines(input: &str) -> String {
    let mut result = String::new();
    // ... iterate chars, push to result
    result
}

// AFTER
fn unfold_lines(input: &str) -> Cow<'_, str> {
    // Quick check: if no continuation lines exist, return borrowed
    if !input.contains("\r\n ") && !input.contains("\r\n\t")
        && !input.contains("\n ") && !input.contains("\n\t") {
        return Cow::Borrowed(input);
    }
    
    let mut result = String::with_capacity(input.len());
    // ... same logic as before
    Cow::Owned(result)
}
```

**3.3 Add zero-copy `CalendarEventRef` for parsing without allocation**
- **File:** `crates/dav/src/ical.rs` (new struct)

```rust
pub struct CalendarEventRef<'a> {
    pub uid: &'a str,
    pub summary: &'a str,
    pub description: Option<&'a str>,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub location: Option<&'a str>,
    pub attendees: Vec<&'a str>,
    pub recurrence: Option<&'a str>,
    pub status: EventStatus,
}

pub fn extract_event_from_ical_ref(ical: &str) -> Result<CalendarEventRef<'_>, String> {
    let unfolded = unfold_lines(ical);
    let lines: Vec<&str> = unfolded.lines().collect();
    // ... parse using borrowed references
}
```

#### Benchmark Targets (Day 3-4)
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| iCal parse (1 event) | ~45μs | - | < 20μs |
| Allocations per parse | 20-25 | - | 2-3 |
| Memory per parse | ~2KB | - | < 500B |

---

### Day 5-6: vCard Parser Zero-Copy

#### Changes

**5.1 Return borrowed strings from `parse_property_line`**
- **File:** `crates/dav/src/vcard.rs:111-133`

```rust
// BEFORE (vcard.rs:111-133)
fn parse_property_line(line: &str) -> Option<VcardProperty> {
    // ... same allocation pattern as iCal
}

// AFTER
pub struct VcardPropertyRef<'a> {
    pub name: &'a str,
    pub params: Vec<(&'a str, &'a str)>,
    pub value: &'a str,
}

fn parse_property_line_ref(line: &str) -> Option<VcardPropertyRef<'_>> {
    let (name_params, value) = line.split_once(':').map(|(n, v)| (n.trim(), v.trim()))?;
    if name_params.is_empty() {
        return None;
    }

    let mut parts = name_params.split(';');
    let name = parts.next()?;

    let mut params = Vec::new();
    for part in parts {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            params.push((k.trim(), v.trim()));
        }
    }

    Some(VcardPropertyRef { name, params, value })
}
```

**5.2 Zero-copy structured name parsing**
- **File:** `crates/dav/src/vcard.rs:135-143`

```rust
// BEFORE
fn parse_structured_name(value: &str) -> (String, String, String, String, String) {
    let parts: Vec<&str> = value.splitn(5, ';').collect();
    let family = parts.first().unwrap_or(&"").to_string();  // ALLOC
    let given = parts.get(1).unwrap_or(&"").to_string();    // ALLOC
    // ...
}

// AFTER
fn parse_structured_name_ref(value: &str) -> (&str, &str, &str, &str, &str) {
    let mut parts = value.splitn(5, ';');
    let family = parts.next().unwrap_or("");
    let given = parts.next().unwrap_or("");
    let additional = parts.next().unwrap_or("");
    let prefix = parts.next().unwrap_or("");
    let suffix = parts.next().unwrap_or("");
    (family, given, additional, prefix, suffix)
}
```

**5.3 Zero-copy address parsing**
- **File:** `crates/dav/src/vcard.rs:145-157`

```rust
// AFTER
pub struct VcardAddressRef<'a> {
    pub po_box: &'a str,
    pub extended: &'a str,
    pub street: &'a str,
    pub city: &'a str,
    pub region: &'a str,
    pub postal_code: &'a str,
    pub country: &'a str,
    pub types: Vec<&'a str>,
}

fn parse_address_ref<'a>(value: &'a str, types: Vec<&'a str>) -> VcardAddressRef<'a> {
    let mut parts = value.splitn(7, ';');
    VcardAddressRef {
        po_box: parts.next().unwrap_or(""),
        extended: parts.next().unwrap_or(""),
        street: parts.next().unwrap_or(""),
        city: parts.next().unwrap_or(""),
        region: parts.next().unwrap_or(""),
        postal_code: parts.next().unwrap_or(""),
        country: parts.next().unwrap_or(""),
        types,
    }
}
```

**5.4 Add `VcardRef` for zero-copy parsing**
- **File:** `crates/dav/src/vcard.rs` (new struct)

```rust
#[derive(Debug, Default)]
pub struct VcardRef<'a> {
    pub uid: Option<&'a str>,
    pub fn_name: &'a str,
    pub family_name: &'a str,
    pub given_name: &'a str,
    pub additional_names: &'a str,
    pub prefix: &'a str,
    pub suffix: &'a str,
    pub emails: Vec<VcardValueRef<'a>>,
    pub phones: Vec<VcardValueRef<'a>>,
    pub addresses: Vec<VcardAddressRef<'a>>,
    pub org: Option<&'a str>,
    pub title: Option<&'a str>,
    pub role: Option<&'a str>,
    pub photo: Option<&'a str>,
    pub rev: Option<&'a str>,
    pub version: Option<&'a str>,
}

pub fn parse_vcard_ref(input: &str) -> Result<VcardRef<'_>, String> {
    // ... zero-copy implementation
}
```

#### Benchmark Targets (Day 5-6)
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| vCard parse (1 contact) | ~35μs | - | < 15μs |
| Allocations per parse | 25-30 | - | 2-3 |
| Memory per parse | ~3KB | - | < 600B |

---

### Day 7-8: Path Normalization Zero-Copy

#### Changes

**7.1 Return `Cow<str>` from `normalize_path`**
- **File:** `crates/common/src/path.rs:6-27`

```rust
// BEFORE (path.rs:6-27)
pub fn normalize_path(path: &str) -> String {
    let path = Path::new(path);
    let mut result = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => { result.pop(); }
            Component::CurDir => {}
            other => { result.push(other); }
        }
    }
    let normalized: PathBuf = result.into_iter().collect();
    let s = normalized.to_string_lossy().to_string();
    if s.is_empty() || !s.starts_with('/') {
        format!("/{s}")
    } else {
        s
    }
}

// AFTER
pub fn normalize_path(path: &str) -> Cow<'_, str> {
    // Fast path: check if normalization is needed
    let needs_normalization = path.contains("..") || path.contains("/./")
        || path.ends_with('/') || (!path.is_empty() && !path.starts_with('/'));
    
    if !needs_normalization {
        return Cow::Borrowed(path);
    }
    
    let path = Path::new(path);
    let mut result = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => { result.pop(); }
            Component::CurDir => {}
            other => { result.push(other); }
        }
    }
    let normalized: PathBuf = result.into_iter().collect();
    let s = normalized.to_string_lossy().to_string();
    if s.is_empty() || !s.starts_with('/') {
        Cow::Owned(format!("/{s}"))
    } else {
        Cow::Owned(s)
    }
}
```

**7.2 Optimize `parent_path` to avoid double normalization**
- **File:** `crates/common/src/path.rs:31-38`

```rust
// BEFORE
pub fn parent_path(path: &str) -> Option<String> {
    let normalized = normalize_path(path);  // ALLOC
    if normalized == "/" {
        return None;
    }
    let parent = Path::new(&normalized).parent()?;
    Some(normalize_path(&parent.to_string_lossy()))  // ALLOC
}

// AFTER
pub fn parent_path(path: &str) -> Option<Cow<'_, str>> {
    let normalized = normalize_path(path);
    if normalized.as_ref() == "/" {
        return None;
    }
    let parent = Path::new(normalized.as_ref()).parent()?;
    Some(normalize_path(&parent.to_string_lossy()))
}
```

**7.3 Add `normalize_path_inplace` for callers who need owned strings**
- **File:** `crates/common/src/path.rs` (new function)

```rust
/// Normalize path, returning owned String. Use when caller needs ownership.
pub fn normalize_path_owned(path: &str) -> String {
    match normalize_path(path) {
        Cow::Borrowed(s) => s.to_string(),
        Cow::Owned(s) => s,
    }
}
```

**7.4 Update all callers to use `Cow` where possible**
- **File:** `crates/server-webdav-core/src/handlers/mkcol.rs:8`
- **File:** `crates/server-webdav-core/src/handlers/copy_move.rs:36`
- **File:** `crates/server-webdav-core/src/handlers/lock.rs:17`
- **File:** `crates/server-webdav-core/src/handlers/delete.rs:37`
- **File:** `crates/server-webdav-core/src/handlers/proppatch.rs:18`
- **File:** `crates/server-webdav-core/src/handlers/propfind.rs:15`
- **File:** `crates/server-webdav-core/src/handlers/get.rs:17`
- **File:** `crates/server-webdav/src/handler/mod.rs:57`
- **File:** `crates/server-webdav/src/handler/proppatch.rs:18`
- **File:** `crates/server-webdav/src/handler/lock.rs:18`
- **File:** `crates/server-webdav/src/handler/copy.rs:16`
- **File:** `crates/server-webdav/src/handler/mkcol.rs:11`
- **File:** `crates/server-webdav/src/handler/propfind.rs:17`
- **File:** `crates/server-webdav/src/handler/delete.rs:36`
- **File:** `crates/server-webdav/src/handler/put.rs:22`
- **File:** `crates/server-webdav/src/handler/move_cmd.rs:16`

Example update pattern:
```rust
// BEFORE
let path = normalize_path(path);

// AFTER
let path = normalize_path(path);  // Returns Cow<'_, str>, borrows if no normalization needed
let path = path.into_owned();     // Only allocate if truly needed
```

#### Benchmark Targets (Day 7-8)
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Path normalize (no change) | ~500ns | - | < 50ns |
| Path normalize (with ..) | ~500ns | - | < 500ns |
| Allocations (no change) | 1 | - | 0 |
| parent_path | ~1μs | - | < 200ns |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Breaking existing API contracts | Medium | High | Keep owned versions as wrappers, feature-gate ref versions |
| Lifetime complexity in nested parsers | High | Medium | Use explicit lifetimes, add compile-time checks |
| Performance regression on complex paths | Low | Low | Benchmark before/after, keep fast path for simple cases |
| Missing allocation in edge cases | Medium | Medium | Add fuzzing targets, run Miri in CI |

---

## Testing Strategy

### Unit Tests
- Add `#[test]` for each zero-copy function
- Verify borrowed strings point to original input
- Test edge cases: empty strings, unicode, very long strings

### Integration Tests
- Verify all existing tests still pass
- Add comparison tests: owned vs ref versions produce same results

### Benchmarks
- Extend `crates/common/benches/path_bench.rs` with zero-copy benchmarks
- Add new benchmarks in `crates/dav/benches/` for XML, iCal, vCard

```rust
// crates/dav/benches/zero_copy.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_ical_parse_owned(c: &mut Criterion) {
    let ical = generate_test_ical();
    c.bench_function("ical_parse/owned", |b| b.iter(|| parse_ical(&ical)));
}

fn bench_ical_parse_ref(c: &mut Criterion) {
    let ical = generate_test_ical();
    c.bench_function("ical_parse/ref", |b| b.iter(|| parse_ical_ref(&ical)));
}

fn bench_vcard_parse_owned(c: &mut Criterion) {
    let vcard = generate_test_vcard();
    c.bench_function("vcard_parse/owned", |b| b.iter(|| parse_vcard(&vcard)));
}

fn bench_vcard_parse_ref(c: &mut Criterion) {
    let vcard = generate_test_vcard();
    c.bench_function("vcard_parse/ref", |b| b.iter(|| parse_vcard_ref(&vcard)));
}

criterion_group!(
    benches,
    bench_ical_parse_owned,
    bench_ical_parse_ref,
    bench_vcard_parse_owned,
    bench_vcard_parse_ref,
);
criterion_main!(benches);
```

---

## Rollback Procedure

1. **Feature gate zero-copy versions:**
   ```toml
   # crates/dav/Cargo.toml
   [features]
   default = ["zero-copy"]
   zero-copy = []
   legacy-parse = []
   ```

2. **Keep owned versions behind feature flag:**
   ```rust
   #[cfg(feature = "legacy-parse")]
   pub fn parse_ical(input: &str) -> Result<Vec<IcalComponent>, String> { ... }
   
   #[cfg(feature = "zero-copy")]
   pub fn parse_ical_ref(input: &str) -> Result<Vec<IcalComponentRef<'_>>, String> { ... }
   ```

3. **Rollback command:**
   ```bash
   cargo build --no-default-features --features legacy-parse
   ```

---

## Success Metrics

| Metric | Baseline | Day 4 | Day 8 |
|--------|----------|-------|-------|
| XML parse allocations | 8-12 | 0-2 | 0-2 |
| iCal parse allocations | 20-25 | 5-8 | 2-3 |
| vCard parse allocations | 25-30 | 5-8 | 2-3 |
| Path normalize allocations | 1-2 | - | 0-1 |
| Total allocations per request | ~45 | ~15 | ~8 |

---

## Appendix: Files to Modify

| File | Change Type | Priority |
|------|-------------|----------|
| `crates/dav/src/xml_ext.rs` | Modify | High |
| `crates/dav/src/ical.rs` | Modify | High |
| `crates/dav/src/vcard.rs` | Modify | High |
| `crates/common/src/path.rs` | Modify | High |
| `crates/dav/benches/zero_copy.rs` | New | Medium |
| `crates/server-webdav-core/src/handlers/*.rs` | Modify | Medium |
| `crates/server-webdav/src/handler/*.rs` | Modify | Medium |

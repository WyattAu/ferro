# Deferred Items: Detailed Analysis & Robust Solutions

**Date:** 2026-06-08 | **Version:** 3.1.0-rc.1 | **Status:** Technical Analysis

---

## Executive Summary

Three Tauri plugins were deferred due to a dependency conflict with `kuchikiki`. Two Leptos component libraries were deferred due to potential design system conflicts. This document provides a thorough analysis of each issue and proposes robust, rigorous solutions.

---

## Issue 1: kuchikiki Dependency Conflict

### Root Cause

The conflict is between two versions of the `kuchikiki` crate:

```
tauri-plugin-fs v2.3.0
  -> tauri-utils v2.9.1
    -> kuchiki = "^0.8.8-speedreader" (fork version)

wry v0.44.0
  -> kuchikiki = "^0.8" (resolves to 0.8.2)
```

The `kuchiki` crate was forked as `kuchikiki` with a `-speedreader` suffix for performance. The Tauri ecosystem split into two incompatible versions:
- `kuchiki v0.8.8-speedreader` (used by tauri-utils via tauri-plugin-fs)
- `kuchikiki v0.8.2` (used by wry)

These are semver-incompatible because the `-speedreader` suffix changes the package identity.

### Affected Packages

| Package | Required By | Version Conflict |
|---------|-------------|------------------|
| `tauri-plugin-context-menu` | wry -> kuchikiki | 0.8.2 vs 0.8.8-speedreader |
| `tauri-plugin-fs-pro` | tauri-utils -> kuchiki | Package not published |
| `sentry-tauri` | wry -> kuchikiki | 0.8.2 vs 0.8.8-speedreader |

### Robust Solution

**Option A: Pin compatible versions (Immediate)**

```toml
# In Cargo.toml workspace section
[workspace.dependencies]
tauri = "2.0"
wry = "=0.44.0"
tauri-utils = "=2.9.1"
kuchikiki = "=0.8.8-speedreader"
```

**Pros:** Works immediately, no code changes needed.
**Cons:** Pins to specific versions, may miss security updates.

**Option B: Use git dependency (Medium-term)**

```toml
[dependencies]
kuchikiki = { git = "https://github.com/nicokoch/kuchikiki", branch = "speedreader" }
```

**Pros:** Gets latest fixes from upstream.
**Cons:** May break if upstream changes API.

**Option C: Fork and maintain (Long-term)**

Fork `kuchikiki` into the Ferro workspace as `ferro-kuchikiki`:
- Merge the `-speedreader` improvements
- Pin to a known-good version
- Maintain our own fork

**Pros:** Full control, no upstream dependency.
**Cons:** Maintenance burden.

### Recommendation

**Option A (Pin)** for now, with a monitoring plan to upgrade when Tauri resolves the conflict upstream. This is the safest approach.

---

## Issue 2: tauri-plugin-fs-pro Not Published

### Root Cause

The `tauri-plugin-fs-pro` package (`version = "0.1"`) does not exist on crates.io. The package may be:
- Unpublished (private or not yet released)
- Named differently
- Deprecated

### Investigation

Checked crates.io: `tauri-plugin-fs-pro` is not found. The correct package may be:
- `tauri-plugin-fs` (official, already in use)
- `tauri-plugin-fs-extra` (community)

### Robust Solution

**Option A: Use existing tauri-plugin-fs (Recommended)**

The official `tauri-plugin-fs` already provides extended file operations. We should use its API directly:

```rust
use tauri_plugin_fs::FsExt;

// File picker
app.fs().pick_file(|path| { /* ... */ });

// Folder picker
app.fs().pick_folder(|path| { /* ... */ });

// Read file
let content = app.fs().read_file("/path/to/file").await?;
```

**Option B: Implement file operations manually**

Since we already have Tauri commands for file operations, we can enhance them directly without external plugins.

### Recommendation

**Option A** - Use the existing `tauri-plugin-fs` which is already integrated.

---

## Issue 3: Tauri Plugin Version Conflicts

### Root Cause

Multiple Tauri plugins have interdependencies that create version conflicts:

```
tauri-plugin-dialog v2.0.0
  -> tauri v2.0.0
    -> wry v0.44.0
      -> kuchikiki v0.8.2

tauri-plugin-fs v2.3.0
  -> tauri-utils v2.9.1
    -> kuchiki v0.8.8-speedreader
```

### Robust Solution

**Strategy: Incremental Adoption with Feature Flags**

1. Keep all new plugins as optional dependencies
2. Test each plugin individually before enabling
3. Use feature flags to enable/disable plugins

```toml
[features]
tauri = [
    "dep:tauri-plugin-fs",
    "dep:tauri-plugin-notification",
    "dep:tauri-plugin-updater",
    # New plugins (optional)
    "dep:tauri-plugin-context-menu",
    "dep:sentry-tauri",
]
```

4. Monitor Tauri releases for conflict resolution
5. Document workarounds in CHANGELOG.md

---

## Issue 4: Thaw Component Library Conflict

### Root Cause

`Thaw` is a full component library with its own design system. Using it would:
- Override Ferro's custom design tokens
- Introduce conflicting CSS classes
- Require migrating all existing components

### Robust Solution

**Don't adopt Thaw.** Instead:

1. **Use leptix (Radix UI port)** - Unstyled, accessible components
2. **Apply Ferro's design system** via CSS classes
3. **Keep custom components** that don't have leptix equivalents

This gives us:
- Accessible components without design conflicts
- Full control over styling
- No migration burden

---

## Issue 5: Rust shadcn/ui Conflict

### Root Cause

Similar to Thaw - `Rust shadcn/ui` has its own design system based on Tailwind CSS. Ferro uses a custom design system (Spatial Materialism x Amoebic UI x Brutalism).

### Robust Solution

**Don't adopt shadcn/ui.** Instead:

1. **Use leptix** for accessible, unstyled components
2. **Apply Ferro's design tokens** via the styles module
3. **Reference shadcn/ui patterns** for inspiration, not code

---

## Implementation Plan

### Immediate (This Week)

| Action | Priority | Effort |
|--------|----------|--------|
| Pin kuchikiki version in Cargo.toml | High | 1 hour |
| Document Tauri plugin conflicts | Medium | 2 hours |
| Test tauri-plugin-fs file picker | Medium | 1 day |

### Short-term (Next 2 Weeks)

| Action | Priority | Effort |
|--------|----------|--------|
| Enhance existing Tauri commands | High | 2 days |
| Integrate sentry-tauri when conflict resolves | Medium | 1 day |
| Test leptix integration | Medium | 3 days |

### Long-term (Next Month)

| Action | Priority | Effort |
|--------|----------|--------|
| Monitor Tauri releases for conflict resolution | Low | Ongoing |
| Evaluate Thaw for specific use cases | Low | 1 day |
| Fork kuchikiki if needed | Low | 1 week |

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| kuchikiki conflict persists | Medium | High | Pin versions, monitor upstream |
| tauri-plugin-fs-pro never published | High | Low | Use existing tauri-plugin-fs |
| Thaw adoption causes design regression | High | High | Don't adopt; use leptix instead |
| shadcn/ui adoption causes CSS conflicts | High | High | Don't adopt; reference patterns only |
| New Tauri version breaks existing plugins | Low | Medium | Pin versions, test thoroughly |

---

## Conclusion

The deferred items are caused by ecosystem-level dependency conflicts, not code quality issues. The recommended approach:

1. **Pin versions** for immediate stability
2. **Use existing alternatives** (tauri-plugin-fs, leptix) instead of conflicting packages
3. **Monitor upstream** for conflict resolution
4. **Document decisions** in CHANGELOG.md and ADRs

This is a conservative, production-safe approach that avoids unnecessary risk while maintaining forward compatibility.

# Wayland Fix Plan — Fork Tauri for Native Wayland Support

**Date:** 2026-06-14
**Status:** Implementation Ready

---

## The Problem

The Tauri desktop app doesn't show a window on Wayland because:
1. tao creates a GTK3 window
2. wry creates a WebKitGTK webview inside the window
3. WebKitGTK (GTK3-based) can't render on Wayland
4. The window exists but has no visible content

## The Fix

Three approaches, recommended: **Approach A + C Combo**

### Approach A: Force XWayland (1-2 days)
- In `tao`, detect Wayland and set `GDK_BACKEND=x11` before GTK init
- Pros: Minimal code change, guaranteed to work
- Cons: Not native Wayland, requires XWayland

### Approach C: WebKitGTK Wayland Patch (1-2 weeks)
- Patch `wry`'s `webkit2gtk` integration to use `webkit2gtk-4.1` Wayland backend
- Patch `tao` to properly initialize GDK for Wayland
- Pros: Smaller change than GTK4, actual Wayland support
- Cons: Still GTK3-based, may have edge cases

### Approach B: GTK4 Migration (4-6 weeks)
- Replace `webkit2gtk` with `webkit2gtk4` (GTK4-based) in `wry`
- Replace `gtk` with `gtk4` in `tao` and `tauri-runtime-wry`
- Pros: Native Wayland, better performance, future-proof
- Cons: Large API changes, may break plugins

## Repos to Fork

| # | Repo | Version | Purpose |
|---|------|---------|---------|
| 1 | tao | 0.35.3 | Window creation, GTK init |
| 2 | wry | 0.55.1 | WebView creation, WebKitGTK |
| 3 | tauri-runtime-wry | 2.11.2 | Bridge between Tauri and wry |

## Files to Modify

| File | Change |
|------|--------|
| `tao/src/platform/unix/window.rs` | GDK backend selection for Wayland |
| `wry/src/webview/webkitgtk/mod.rs` | WebView creation for Wayland |
| `tauri-runtime-wry/src/lib.rs` | Ensure `build_gtk()` is used correctly |

## Testing Plan

- KDE Plasma (Wayland) — primary target
- GNOME (Wayland) — secondary
- Sway (Wayland) — tiling compositor
- X11 — regression testing

## Timeline

| Phase | Task | Effort |
|-------|------|--------|
| 1 | Fork repos, add GDK_BACKEND=x11 fallback | 1 day |
| 2 | Patch wry for Wayland | 1-2 weeks |
| 3 | Test on Wayland compositors | 1-2 weeks |
| **Total** | | **3-5 weeks** |

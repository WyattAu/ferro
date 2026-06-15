# Tauri Wayland Fix Plan: SUPERSEDED

**Status:** SUPERSEDED by wayland-support-analysis.md (2026-06-15)

---

## Summary

The original fix plan (2026-06-14) proposed creating GTK4 window integration for Wayland
support. This is no longer needed. Upstream Tauri v2.11.2 with wry 0.55.1 works natively
on Wayland without any modifications.

## What Was Proposed (No Longer Needed)

1. ~~Fork wry with GTK4 window support~~ - NOT NEEDED
2. ~~Fork tauri-runtime-wry to use GTK4 window~~ - NOT NEEDED
3. ~~Create webkit2gtk-4.1-sys crate~~ - NOT NEEDED
4. ~~Integrate GTK4 and GTK3 event loops~~ - NOT NEEDED

## Why It Was Superseded

Testing with WAYLAND_DEBUG protocol logging proved that:
- Upstream Tauri creates xdg_toplevel surfaces correctly on Wayland
- WebKitGTK 2.52.4 renders natively on Wayland
- The original "no window" observation was due to testing errors
  (wrong feature flags, xdotool limitation)

## Lesson Learned

Always verify window visibility through Wayland protocol logging (WAYLAND_DEBUG=1)
rather than relying on X11 tools (xdotool) when testing on Wayland.

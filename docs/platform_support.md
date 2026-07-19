# Platform Support Matrix

**Date:** 2026-07-16 | **Version:** 3.0.0

---

## Executive Summary

Ferro supports **6 platform categories** with **12 deployment targets**:

| Category | Platforms | Status |
|----------|-----------|--------|
| Server binaries | Linux (x86_64, musl, aarch64), macOS (Intel, ARM) | Production |
| Docker | linux/amd64, linux/arm64 | Production |
| Desktop app | Windows, macOS, Linux (deb/rpm/AppImage) | Production |
| Mobile | Android, iOS | Beta |
| Web | WASM (any modern browser), PWA | Production |
| FUSE/NFS mount | Linux (FUSE3, NFS v3/v4, SMB) | Production |

---

## Detailed Platform Support

### 1. Server Binaries

| Target | OS | Arch | Format | CI | Status |
|--------|-----|------|--------|-----|--------|
| x86_64-unknown-linux-gnu | Linux | x86_64 | ELF binary | Yes | Production |
| x86_64-unknown-linux-musl | Linux | x86_64 | Static ELF | Yes | Production |
| aarch64-unknown-linux-gnu | Linux | ARM64 | ELF binary | Yes | Production |
| x86_64-apple-darwin | macOS | Intel | Mach-O | Yes | Production |
| aarch64-apple-darwin | macOS | ARM64 | Mach-O | Yes | Production |

**Features available:** s3, gcs, azure, pg, redis, ldap (all tested in CI)

### 2. Docker

| Platform | Format | CI | Status |
|----------|--------|-----|--------|
| linux/amd64 | OCI image | Yes | Production |
| linux/arm64 | OCI image | Yes | Production |

**Base:** Scratch (minimal image)
**Features:** s3, gcs, azure (default)

### 3. Desktop App (Tauri v2)

| Platform | Format | CI | Status |
|----------|--------|-----|--------|
| Windows x86_64 | NSIS installer | Yes | Production |
| macOS Universal | DMG | Yes | Production |
| Linux x86_64 | deb, rpm, AppImage | Yes | Production |
| Linux aarch64 | deb, rpm | No | Beta |

**Features:** Full sync, mount, shell integration, auto-updater (desktop only)

### 4. Mobile App (Tauri v2)

| Platform | Format | CI | Status |
|----------|--------|-----|--------|
| Android aarch64 | APK | Yes | Beta |
| iOS aarch64 | IPA | No (requires Xcode) | Beta |

**Features:** Offline pinning, push notifications, background sync, connectivity monitoring

### 5. Web Frontend (Leptos WASM)

| Platform | Format | Status |
|----------|--------|--------|
| Desktop browsers | WASM + JS | Production |
| Mobile browsers | WASM + JS | Production |
| PWA | Manifest + SW | Production |

**Features:** 14 themes, responsive design, touch support, offline fallback, service worker

### 6. FUSE/NFS Mount

| Platform | Format | Status |
|----------|--------|--------|
| Linux FUSE3 | FUSE mount | Production |
| Linux NFS v3/v4 | NFS mount | Production |
| Linux SMB | SMB mount | Production |

**Features:** Read/write mount, background sync, offline cache

---

## Platform-Specific Notes

### Windows
- NSIS installer for desktop app
- Shell context menu integration
- Windows autostart support
- NTFS-compatible file handling

### macOS
- Universal binary (Intel + ARM)
- DMG installer
- Native macOS menu integration
- Spotlight indexing support

### Linux
- deb, rpm, AppImage packages
- FUSE3 mount support
- NFS v3/v4 and SMB mount support
- systemd service file available

### Android
- Tauri v2 mobile with Android SDK
- Push notifications (FCM)
- Background sync with WiFi/charging constraints
- Offline file pinning

### iOS
- Tauri v2 mobile with Xcode
- Push notifications (APNS)
- Biometric authentication
- Files Provider integration

### Web
- Leptos WASM frontend
- PWA with service worker
- Responsive design (mobile, tablet, desktop)
- 14 themes with CSS custom properties
- Touch gesture support
- Keyboard navigation (WCAG 2.1 AA)

---

## Testing Coverage

| Platform | CI Build | Integration Tests | E2E Tests | Status |
|----------|----------|-------------------|-----------|--------|
| Linux x86_64 | Yes | Yes | Yes (Playwright) | Verified |
| Linux ARM64 | Yes | Yes | No | CI only |
| Linux musl | Yes | Yes | No | CI only |
| macOS Intel | Yes | No | No | CI only |
| macOS ARM64 | Yes | No | No | CI only |
| Windows | Yes | No | No | CI only |
| Android | Yes | No | No | CI only |
| iOS | No | No | No | Requires Xcode |
| Web (WASM) | Yes | Yes | Yes (Playwright) | Verified |
| Docker | Yes | No | No | CI only |
| FUSE/NFS | No | No | No | Runtime only |

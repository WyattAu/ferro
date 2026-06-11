# Ferro Mobile (iOS / Android)

Build instructions for the Ferro mobile apps using Tauri v2.

## Prerequisites

### Common

- Rust toolchain: `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android` (Android) and/or `rustup target add aarch64-apple-ios x86_64-apple-ios` (iOS)
- `cargo install tauri-cli` (or use `cargo tauri` via the project's toolchain)

### iOS

- macOS with Xcode 15+
- iOS Simulator SDK (for local testing)
- Apple Developer account for device deployment / App Store distribution

### Android

- Android Studio or standalone Android SDK
- Android SDK Platform 34 (or matching `compileSdkVersion`)
- Android Build Tools
- NDK (side-by-side) — Tauri v2 auto-detects the NDK path
- Set `ANDROID_HOME` environment variable (e.g. `~/Android/Sdk`)
- Java JDK 17+

## Initialisation

These commands generate the native Xcode / Android Studio projects under `gen/mobile/`.

```bash
# iOS — run once
cargo tauri ios init

# Android — run once
cargo tauri android init
```

`init` is idempotent — re-running it won't overwrite existing changes unless you pass `--ci`.

## Development

```bash
# Run on iOS Simulator
cargo tauri ios dev

# Run on connected Android device / emulator
cargo tauri android dev
```

## Release Builds

```bash
# Android APK
cargo tauri android build --release
# Output: gen/android/app/build/outputs/apk/release/

# Android App Bundle (AAB) for Play Store
cargo tauri android build --release --bundlesaab

# iOS
cargo tauri ios build --release
# Open gen/mobile/ios/Ferro.xcodeproj in Xcode to archive for App Store
```

## Signing

### iOS

Set `developmentTeam` in `tauri.conf.json` → `bundle.iOS.developmentTeam` to your Apple Team ID (e.g. `ABC123DEF4`).

For CI, set environment variables:
- `APPLE_SIGNING_IDENTITY` — certificate name
- `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` — for notarisation

### Android

Create a keystore:
```bash
keytool -genkey -v -keystore release.keystore -alias ferro -keyalg RSA -keysize 2048 -validity 10000
```

Set in `tauri.conf.json` or environment variables:
- `TAURI_ANDROID_KEYSTORE_PATH` — path to `.keystore` file
- `TAURI_ANDROID_KEYSTORE_PASSWORD` — keystore password
- `TAURI_ANDROID_KEY_ALIAS` — key alias

## Platform-Specific Configuration

| Setting | File | Key |
|---|---|---|
| iOS minimum version | `tauri.conf.json` | `bundle.iOS.minimumSystemVersion` |
| iOS team ID | `tauri.conf.json` | `bundle.iOS.developmentTeam` |
| Android min SDK | `tauri.conf.json` | `bundle.android.minSdkVersion` |
| Android permissions | `gen/android/app/src/main/AndroidManifest.xml` | `<uses-permission>` |
| iOS permissions | `gen/mobile/ios/Ferro/Ferro/Info.plist` | `NS*UsageDescription` |

## Build Script

A convenience script is provided:

```bash
./scripts/build-mobile.sh android   # Build Android APK
./scripts/build-mobile.sh ios       # Build iOS archive
```

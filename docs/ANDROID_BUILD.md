# Android Build Guide for Ferro

**Date:** 2026-06-15
**Status:** Environment Ready (SDK + NDK + Rust targets installed)

---

## What's Installed

| Component | Version | Location |
|-----------|---------|----------|
| Java JDK | 17.0.2 | `~/.local/sdk/jdk-17/` |
| Android SDK | 12.0 (cmdline) | `~/.local/android-sdk/` |
| Android NDK | 26.1.10909125 | `~/.local/android-sdk/ndk/26.1.10909125/` |
| Build Tools | 34.0.0 | `~/.local/android-sdk/build-tools/34.0.0/` |
| Platform | android-34 | `~/.local/android-sdk/platforms/android-34/` |
| Platform Tools | 37.0.0 | `~/.local/android-sdk/platform-tools/` |
| Rust aarch64-linux-android | installed | `rustup target list --installed` |
| Rust armv7-linux-androideabi | installed | |
| Rust i686-linux-android | installed | |
| Rust x86_64-linux-android | installed | |

## What's Needed (Requires Physical Access)

### 1. Enable USB Debugging on Android Phone

1. Go to **Settings > About Phone**
2. Tap **Build Number** 7 times (enables Developer Options)
3. Go to **Settings > Developer Options**
4. Enable **USB Debugging**
5. Enable **Wireless Debugging** (optional, for WiFi builds)

### 2. Connect Phone via USB

1. Connect phone to computer via USB cable
2. On phone: tap "Allow USB debugging" when prompted
3. Verify connection:

```bash
source scripts/setup-android-env.sh
adb devices -l
# Should show: SERIAL device usb:xxx product:xxx model:xxx device:xxx transport_id:x
```

### 3. Accept ADB Key on Phone

First connection will prompt to accept RSA key. Tap "Allow" on phone.

---

## Build Process

### Option A: Tauri v2 Android (Recommended)

Tauri v2 has native Android support. The `crates/mobile/` crate is configured for this.

```bash
source scripts/setup-android-env.sh
cd crates/mobile

# Initialize Tauri Android project (first time only)
cargo tauri android init

# Build debug APK and install on connected device
cargo tauri android dev

# Build release APK
cargo tauri android build --release

# The APK will be at: target/release/android/build/outputs/apk/
```

### Option B: Manual Rust Cross-Compilation

If Tauri v2 Android has issues, compile Rust code directly:

```bash
source scripts/setup-android-env.sh

# Build for aarch64 (most modern phones)
cargo build --release --target aarch64-linux-android -p ferro-mobile

# The binary will be at:
# target/aarch64-linux-android/release/ferro-mobile
```

### Option C: Gradle Direct Build

Use the Android SDK directly:

```bash
source scripts/setup-android-env.sh
cd crates/mobile/src/android

# Build debug APK
./gradlew assembleDebug

# Install on connected device
./gradlew installDebug
```

---

## Testing on Device

### 1. Install APK via ADB

```bash
adb install path/to/ferro-mobile.apk
```

### 2. Launch App

```bash
adb shell am start -n com.ferro.mobile/.MainActivity
```

### 3. View Logs

```bash
adb logcat -s ferro_mobile:V
```

### 4. Take Screenshot

```bash
adb shell screencap /sdcard/ferro-screenshot.png
adb pull /sdcard/ferro-screenshot.png /tmp/
```

### 5. WiFi Debugging (No Cable)

```bash
# On phone: Developer Options > Wireless Debugging > Pair
adb pair IP:PORT    # Enter pairing code from phone
adb connect IP:PORT # Connect to device
adb devices         # Should show device
```

---

## Build Targets Matrix

| Target | CPU Arch | Use Case |
|--------|----------|----------|
| `aarch64-linux-android` | ARM64 (v8-A) | Most modern phones (2016+) |
| `armv7-linux-androideabi` | ARMv7 (32-bit) | Older phones, some tablets |
| `i686-linux-android` | x86 (32-bit) | Emulators, Chromebooks |
| `x86_64-linux-android` | x86_64 (64-bit) | Emulators, Chromebooks |

**Primary target:** `aarch64-linux-android` (covers 95%+ of active Android devices)

---

## Known Limitations

1. **No Xcode on Linux** -- iOS builds require macOS with Xcode. Cannot build iOS from this machine.
2. **Tauri v2 Mobile maturity** -- Tauri v2 Android support is relatively new. Some plugins may not work.
3. **No emulator available** -- Android Studio emulator requires KVM/HAXM. Physical device testing only.
4. **Code signing** -- Debug builds use auto-generated keys. Release builds need a keystore.

---

## Environment Variables for CI

For GitHub Actions, set these secrets:
- `ANDROID_KEYSTORE_BASE64` -- Base64-encoded keystore file
- `ANDROID_KEYSTORE_PASSWORD` -- Keystore password
- `ANDROID_KEY_ALIAS` -- Key alias
- `ANDROID_KEY_PASSWORD` -- Key password

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `adb: device not found` | Enable USB debugging, check cable, accept RSA key |
| `SDK not found` | Run `source scripts/setup-android-env.sh` |
| `NDK not found` | Verify `NDK_HOME` points to `~/.local/android-sdk/ndk/26.1.10909125/` |
| `cargo: linker not found` | Install `cargo-ndk` or set `CC_aarch64_linux_android` env var |
| `Gradle: SDK not found` | Set `sdk.dir` in `local.properties` or `ANDROID_HOME` env var |
| APK install fails | Check `adb devices`, try `adb uninstall com.ferro.mobile` first |

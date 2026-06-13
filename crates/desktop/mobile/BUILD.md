# Mobile Build Instructions

## iOS Build Instructions

### Prerequisites

- Xcode 15+ with iOS 17+ SDK
- Rust toolchain with `aarch64-apple-ios` target
- Tauri CLI: `cargo install tauri-cli`
- CocoaPods: `sudo gem install cocoapods`

### Setup

```bash
# Add iOS target
rustup target add aarch64-apple-ios

# Initialize iOS project
cd crates/desktop
cargo tauri ios init

# Add Files Provider extension
cp -r mobile/ios/FerroFileProvider gen/mobile/ios/Ferro/
# Open in Xcode and add extension target
```

### Build

```bash
# Debug build
cargo tauri ios build

# Release build
cargo tauri ios build --release
```

### Files Provider Extension

1. Open `gen/mobile/ios/Ferro.xcodeproj` in Xcode
2. Add new target: File Provider Extension
3. Set bundle identifier: `com.wyattau.ferro.FileProvider`
4. Set App Group: `group.com.ferro.app`
5. Copy Swift files from `mobile/ios/FerroFileProvider/`
6. Set entitlements from `mobile/ios/FerroFileProvider.entitlements`

### Testing

- Install on device via Xcode
- Open Files app -> Browse -> Ferro
- Verify files appear from Ferro server

---

## Android Build Instructions

### Prerequisites

- Android Studio with SDK 34+
- Rust toolchain with `aarch64-linux-android` target
- Tauri CLI: `cargo install tauri-cli`
- JDK 17+

### Setup

```bash
# Add Android target
rustup target add aarch64-linux-android

# Initialize Android project
cd crates/desktop
cargo tauri android init

# Add DocumentsProvider
cp -r mobile/android/app src/main/java/com/ferro/
```

### Build

```bash
# Debug build
cargo tauri android build --debug

# Release build
cargo tauri android build --release
```

### DocumentsProvider

1. Register in `AndroidManifest.xml`
2. Set authority: `com.ferro.fileprovider.documents`
3. Set permission: `android.permission.MANAGE_DOCUMENTS`

### Testing

- Install APK on device
- Open Files app -> Browse -> Ferro
- Verify files appear from Ferro server

---

## App Group Configuration (iOS)

The iOS Files Provider extension communicates with the Tauri app via App Groups:

1. In Xcode, add App Group capability to both targets:
   - Main app: `group.com.ferro.app`
   - File Provider extension: `group.com.ferro.app`

2. The Tauri app writes server connection info to:
   ```
   ~/Library/Group Containers/group.com.ferro.app/server-config.plist
   ```

3. The File Provider extension reads this plist to connect to the server.

---

## Content Provider Configuration (Android)

The Android DocumentsProvider communicates with the Tauri app via SharedPreferences:

1. The Tauri app writes server connection info to:
   ```
   /data/data/com.ferro.app/shared_prefs/ferro_file_provider.xml
   ```

2. The DocumentsProvider reads this SharedPreferences to connect to the server.

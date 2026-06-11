#!/usr/bin/env bash
set -euo pipefail

PLATFORM="${1:-android}"

echo "Building Ferro for $PLATFORM..."

case "$PLATFORM" in
  android)
    echo "Initializing Android project..."
    cargo tauri android init 2>/dev/null || true
    echo "Building Android APK..."
    cargo tauri android build --release
    echo "APK: crates/desktop/gen/android/app/build/outputs/apk/release/"
    ;;
  ios)
    echo "Initializing iOS project..."
    cargo tauri ios init 2>/dev/null || true
    echo "Building iOS..."
    cargo tauri ios build --release
    echo "Build complete. Open gen/mobile/ios/Ferro.xcodeproj in Xcode to archive."
    ;;
  *)
    echo "Usage: $0 [android|ios]"
    exit 1
    ;;
esac

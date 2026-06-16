#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

error() { echo -e "${RED}ERROR: $1${NC}" >&2; exit 1; }
warn()  { echo -e "${YELLOW}WARN: $1${NC}" >&2; }
info()  { echo -e "${GREEN}INFO: $1${NC}"; }

check_tool() {
    if ! command -v "$1" &>/dev/null; then
        error "$2"
    fi
}

info "Checking prerequisites..."

check_tool "rustup" "rustup not found. Install from https://rustup.rs"
check_tool "cargo" "cargo not found. Install Rust from https://rustup.rs"

if ! rustup target list --installed | grep -q "aarch64-linux-android"; then
    info "Adding aarch64-linux-android target..."
    rustup target add aarch64-linux-android || error "Failed to add Android target"
fi

if [ -z "${ANDROID_HOME:-}" ]; then
    if [ -d "$HOME/Android/Sdk" ]; then
        export ANDROID_HOME="$HOME/Android/Sdk"
    elif [ -d "$HOME/Library/Android/sdk" ]; then
        export ANDROID_HOME="$HOME/Library/Android/sdk"
    else
        error "ANDROID_HOME not set and Android SDK not found. Install Android Studio or set ANDROID_HOME."
    fi
fi

if [ ! -d "$ANDROID_HOME" ]; then
    error "Android SDK not found at $ANDROID_HOME. Install Android Studio or set ANDROID_HOME."
fi

if [ -z "${JAVA_HOME:-}" ]; then
    if command -v java &>/dev/null; then
        JAVA_HOME="$(java -XshowSettings:properties -version 2>&1 | grep 'java.home' | awk '{print $3}')"
        export JAVA_HOME
    else
        error "JAVA_HOME not set and java not found. Install JDK 17+ or set JAVA_HOME."
    fi
fi

if ! command -v java &>/dev/null; then
    error "Java not found. Install JDK 17+ (e.g., brew install openjdk@17)"
fi

JAVA_VERSION=$(java -version 2>&1 | head -1 | cut -d'"' -f2 | cut -d'.' -f1)
if [ "$JAVA_VERSION" -lt 17 ] 2>/dev/null; then
    warn "Java $JAVA_VERSION detected. JDK 17+ is recommended."
fi

check_tool "sdkmanager" "sdkmanager not found. Install Android SDK command-line tools."

NDK_VERSION="27.0.12077973"
if [ ! -d "$ANDROID_HOME/ndk/$NDK_VERSION" ]; then
    info "Installing Android NDK $NDK_VERSION..."
    sdkmanager "ndk;$NDK_VERSION" || error "Failed to install NDK"
fi

if ! command -v tauri &>/dev/null; then
    if ! cargo tauri --version &>/dev/null 2>&1; then
        info "Installing Tauri CLI..."
        cargo install tauri-cli --version "^2" --locked || error "Failed to install Tauri CLI"
    fi
fi

BUILD_MODE="${1:-debug}"

info "Build mode: $BUILD_MODE"
info "Initializing Android project..."

cd "$(dirname "$0")/.."
cargo tauri android init 2>/dev/null || true

if [ "$BUILD_MODE" = "release" ]; then
    info "Building Android APK (release)..."
    cargo tauri android build --release --target aarch64-linux-android --features android
else
    info "Building Android APK (debug)..."
    cargo tauri android build --debug --target aarch64-linux-android --features android
fi

info "Build complete."
info "APK location: crates/desktop/gen/android/app/build/outputs/apk/"

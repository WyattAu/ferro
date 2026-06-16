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

if ! rustup target list --installed | grep -q "aarch64-apple-ios"; then
    info "Adding aarch64-apple-ios target..."
    rustup target add aarch64-apple-ios || error "Failed to add iOS target"
fi

if ! rustup target list --installed | grep -q "aarch64-apple-ios-sim"; then
    info "Adding aarch64-apple-ios-sim target..."
    rustup target add aarch64-apple-ios-sim || warn "Failed to add simulator target (non-fatal)"
fi

check_tool "xcodebuild" "Xcode not found. Install Xcode from the Mac App Store"
check_tool "pod" "CocoaPods not found. Install via: sudo gem install cocoapods"

if ! command -v tauri &>/dev/null; then
    if ! cargo tauri --version &>/dev/null 2>&1; then
        info "Installing Tauri CLI..."
        cargo install tauri-cli --version "^2" --locked || error "Failed to install Tauri CLI"
    fi
fi

BUILD_MODE="${1:-debug}"

info "Build mode: $BUILD_MODE"
info "Initializing iOS project..."

cd "$(dirname "$0")/.."
cargo tauri ios init 2>/dev/null || true

if [ "$BUILD_MODE" = "release" ]; then
    info "Building iOS (release)..."
    cargo tauri ios build --release --features ios
else
    info "Building iOS (debug)..."
    cargo tauri ios build --features ios
fi

info "Build complete."
info "Open gen/mobile/ios/Ferro.xcodeproj in Xcode to archive and deploy."

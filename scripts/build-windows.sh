#!/usr/bin/env bash
# build-windows.sh - Build the Ferro desktop app for Windows
#
# Usage:
#   ./scripts/build-windows.sh              # Debug build
#   ./scripts/build-windows.sh --release    # Release build + NSIS installer
#   ./scripts/build-windows.sh --installer  # Build NSIS installer only (release)
#
# Requires: Rust toolchain with x86_64-pc-windows-msvc target
#           (runs on Windows natively or via cross-compilation)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DESKTOP_DIR="$PROJECT_ROOT/crates/desktop"

TARGET="x86_64-pc-windows-msvc"
FEATURES="tauri"

BUILD_MODE="debug"
BUILD_INSTALLER=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --release)
            BUILD_MODE="release"
            shift
            ;;
        --installer)
            BUILD_MODE="release"
            BUILD_INSTALLER=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [--release] [--installer]"
            echo "  --release    Build in release mode"
            echo "  --installer  Build NSIS installer (implies --release)"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "=== Ferro Desktop Windows Build ==="
echo "Target:  $TARGET"
echo "Mode:    $BUILD_MODE"
echo "Features: $FEATURES"
echo ""

# Ensure target is installed
echo "Ensuring target $TARGET is installed..."
rustup target add "$TARGET" 2>/dev/null || true

if [[ "$BUILD_MODE" == "release" ]]; then
    echo "Building release binary..."
    cargo build --locked --release \
        -p ferro-desktop \
        --target "$TARGET" \
        --features "$FEATURES"

    BINARY_PATH="target/$TARGET/release/ferro-desktop.exe"
else
    echo "Building debug binary..."
    cargo build --locked \
        -p ferro-desktop \
        --target "$TARGET" \
        --features "$FEATURES"

    BINARY_PATH="target/$TARGET/debug/ferro-desktop.exe"
fi

echo "Binary: $PROJECT_ROOT/$BINARY_PATH"
ls -lh "$PROJECT_ROOT/$BINARY_PATH" 2>/dev/null || echo "(binary path may vary on Windows)"

if [[ "$BUILD_INSTALLER" == true ]]; then
    echo ""
    echo "Building NSIS installer..."
    cd "$DESKTOP_DIR"
    cargo tauri build \
        --target "$TARGET" \
        --features "$FEATURES" \
        --bundles nsis
    echo ""
    echo "Installer built in: $DESKTOP_DIR/target/release/bundle/nsis/"
    ls -lh "$DESKTOP_DIR/target/release/bundle/nsis/"*.exe 2>/dev/null || echo "(check directory for installer)"
fi

echo ""
echo "=== Build complete ==="

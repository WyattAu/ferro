#!/bin/bash
# Ferro Development Environment Setup
# Installs all required tools for building Ferro (server, desktop, mobile)
# Usage: bash scripts/setup-dev-env.sh

set -e

FERRO_HOME="${FERRO_HOME:-$HOME/dev/src/github.com/WyattAu/ferro}"
LOCAL_SDK="$HOME/.local/sdk"
LOCAL_ANDROID="$HOME/.local/android-sdk"

echo "============================================"
echo "  Ferro Development Environment Setup"
echo "============================================"
echo ""

# --- Java JDK 17 ---
if [ ! -d "$LOCAL_SDK/jdk-17" ]; then
    echo "[1/6] Installing Java JDK 17..."
    mkdir -p "$LOCAL_SDK"
    cd /tmp
    curl -sL "https://download.java.net/java/GA/jdk17.0.2/dfd4a8d0985749f896bed50d7138ee7f/8/GPL/openjdk-17.0.2_linux-x64_bin.tar.gz" -o jdk17.tar.gz
    tar xzf jdk17.tar.gz -C "$LOCAL_SDK/"
    mv "$LOCAL_SDK/jdk-17.0.2" "$LOCAL_SDK/jdk-17"
    rm jdk17.tar.gz
    echo "  JDK 17 installed at $LOCAL_SDK/jdk-17"
else
    echo "[1/6] JDK 17 already installed"
fi

# --- Android SDK ---
if [ ! -d "$LOCAL_ANDROID/cmdline-tools" ]; then
    echo "[2/6] Installing Android SDK..."
    mkdir -p "$LOCAL_ANDROID/cmdline-tools"
    cd /tmp
    curl -sL "https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip" -o cmdtools.zip
    unzip -q cmdtools.zip -d "$LOCAL_ANDROID/cmdline-tools/"
    mv "$LOCAL_ANDROID/cmdline-tools/cmdline-tools" "$LOCAL_ANDROID/cmdline-tools/latest"
    rm cmdtools.zip
    echo "  Android SDK installed at $LOCAL_ANDROID"
else
    echo "[2/6] Android SDK already installed"
fi

# --- Android SDK Packages ---
export ANDROID_HOME="$LOCAL_ANDROID"
export JAVA_HOME="$LOCAL_SDK/jdk-17"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$JAVA_HOME/bin:$PATH"

echo "[3/6] Installing SDK packages..."
yes | sdkmanager --licenses > /dev/null 2>&1 || true
sdkmanager --install "platform-tools" "platforms;android-34" "build-tools;34.0.0" "ndk;26.1.10909125" 2>&1 | grep -E "Installed|Warning" | head -5

# --- Rust Android Targets ---
echo "[4/6] Adding Rust Android targets..."
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android 2>&1 | grep -v "^$"

# --- Tauri CLI ---
echo "[5/6] Installing Tauri CLI..."
cargo install tauri-cli --version "^2" 2>&1 | tail -3

# --- cargo-ndk ---
echo "[6/6] Installing cargo-ndk..."
cargo install cargo-ndk 2>&1 | tail -3

echo ""
echo "============================================"
echo "  Setup Complete!"
echo "============================================"
echo ""
echo "Environment variables (add to ~/.bashrc):"
echo "  export ANDROID_HOME=$LOCAL_ANDROID"
echo "  export JAVA_HOME=$LOCAL_SDK/jdk-17"
echo "  export PATH=\$ANDROID_HOME/cmdline-tools/latest/bin:\$ANDROID_HOME/platform-tools:\$JAVA_HOME/bin:\$PATH"
echo ""
echo "Quick test:"
echo "  source scripts/setup-android-env.sh"
echo "  adb devices"
echo "  cargo tauri android dev"
echo ""

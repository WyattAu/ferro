#!/bin/bash
# Android SDK Environment Setup for Ferro
# Source this file: source scripts/setup-android-env.sh

export ANDROID_HOME="$HOME/.local/android-sdk"
export JAVA_HOME="$HOME/.local/sdk/jdk-17"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/ndk/26.1.10909125:$JAVA_HOME/bin:$PATH"

# Android NDK toolchain paths (for cross-compilation)
export NDK_HOME="$ANDROID_HOME/ndk/26.1.10909125"
export NDK_TOOLCHAIN="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64"
export CC_aarch64_linux_android="$NDK_TOOLCHAIN/bin/aarch64-linux-android34-clang"
export CC_armv7_linux_androideabi="$NDK_TOOLCHAIN/bin/armv7a-linux-androideabi34-clang"
export CC_i686_linux_android="$NDK_TOOLCHAIN/bin/i686-linux-android34-clang"
export CC_x86_64_linux_android="$NDK_TOOLCHAIN/bin/x86_64-linux-android34-clang"

echo "Android environment configured:"
echo "  ANDROID_HOME=$ANDROID_HOME"
echo "  JAVA_HOME=$JAVA_HOME"
echo "  NDK_HOME=$NDK_HOME"
echo ""
echo "Installed targets:"
rustup target list --installed | grep android
echo ""
echo "ADB devices:"
adb devices -l 2>/dev/null || echo "  (adb not running)"
echo ""
echo "Ready for Android development."

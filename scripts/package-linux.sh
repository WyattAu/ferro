#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# Ferro Linux Desktop Packaging Script
#
# Builds the Tauri desktop app and creates:
#   - .deb package (Debian/Ubuntu)
#   - .rpm package (Fedora/RHEL)
#   - AppImage (universal Linux)
#
# Prerequisites:
#   - Rust stable toolchain
#   - Node.js 18+ and npm
#   - cargo-tauri (cargo install tauri-cli)
#   - For .deb: dpkg-deb, fakeroot
#   - For .rpm: rpmbuild (rpm-build package)
#   - For AppImage: appimagetool, linuxdeploy
#
# Usage:
#   ./scripts/package-linux.sh [--release] [--skip-frontend]
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DESKTOP_DIR="$PROJECT_ROOT/crates/desktop"
BUILD_DIR="$PROJECT_ROOT/target/release"
DIST_DIR="$PROJECT_ROOT/dist/linux"

# App metadata
APP_NAME="ferro"
APP_DISPLAY_NAME="Ferro"
APP_VERSION="${VERSION:-$(grep '^version' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)"/\1/')}"
APP_ID="com.ferro.app"
APP_DESCRIPTION="Secure self-hosted file synchronization client"
APP_URL="https://github.com/WyattAu/ferro"
APP_MAINTAINER="Ferro Contributors <ferro@example.com>"
APP_LICENSE="AGPL-3.0-or-later"
APP_CATEGORY="Utility"

RELEASE_MODE="--release"
SKIP_FRONTEND=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --release)
            RELEASE_MODE="--release"
            shift
            ;;
        --debug)
            RELEASE_MODE=""
            shift
            ;;
        --skip-frontend)
            SKIP_FRONTEND=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "============================================"
echo "  Ferro Linux Desktop Packaging"
echo "  Version: $APP_VERSION"
echo "  Mode: ${RELEASE_MODE:---debug}"
echo "============================================"

# ---------------------------------------------------------------------------
# Build frontend assets
# ---------------------------------------------------------------------------
if [[ "$SKIP_FRONTEND" == "false" ]]; then
    echo ""
    echo "--- Building frontend assets ---"
    if [[ -f "$DESKTOP_DIR/package.json" ]]; then
        pushd "$DESKTOP_DIR" > /dev/null
        npm ci
        npm run build
        popd > /dev/null
    fi
fi

# ---------------------------------------------------------------------------
# Build Rust binary
# ---------------------------------------------------------------------------
echo ""
echo "--- Building Rust binary ($RELEASE_MODE) ---"
cargo build -p ferro-desktop --features tauri $RELEASE_MODE

BINARY_PATH="$BUILD_DIR/ferro-desktop"
if [[ ! -f "$BINARY_PATH" ]]; then
    echo "ERROR: Binary not found at $BINARY_PATH"
    exit 1
fi

# ---------------------------------------------------------------------------
# Run tauri build to produce bundles
# ---------------------------------------------------------------------------
echo ""
echo "--- Running Tauri build ---"
pushd "$DESKTOP_DIR" > /dev/null
cargo tauri build $RELEASE_MODE --bundles deb,rpm --features tauri || true
popd > /dev/null

# ---------------------------------------------------------------------------
# Create dist directory
# ---------------------------------------------------------------------------
mkdir -p "$DIST_DIR"

# ---------------------------------------------------------------------------
# Create .deb package
# ---------------------------------------------------------------------------
echo ""
echo "--- Creating .deb package ---"

DEB_DIR="$DIST_DIR/deb"
DEB_NAME="${APP_NAME}_${APP_VERSION}_amd64"
mkdir -p "$DEB_DIR/$DEB_NAME/DEBIAN"
mkdir -p "$DEB_DIR/$DEB_NAME/usr/bin"
mkdir -p "$DEB_DIR/$DEB_NAME/usr/share/applications"
mkdir -p "$DEB_DIR/$DEB_NAME/usr/share/icons/hicolor/256x256/apps"
mkdir -p "$DEB_DIR/$DEB_NAME/usr/share/icons/hicolor/128x128/apps"
mkdir -p "$DEB_DIR/$DEB_NAME/usr/share/icons/hicolor/64x64/apps"

# Copy binary
cp "$BINARY_PATH" "$DEB_DIR/$DEB_NAME/usr/bin/$APP_NAME"

# Create desktop entry
cat > "$DEB_DIR/$DEB_NAME/usr/share/applications/${APP_ID}.desktop" << EOF
[Desktop Entry]
Type=Application
Name=$APP_DISPLAY_NAME
Exec=/usr/bin/$APP_NAME
Icon=$APP_ID
Comment=$APP_DESCRIPTION
Categories=$APP_CATEGORY;
Terminal=false
StartupWMClass=$APP_DISPLAY_NAME
EOF

# Copy icons if available
for size in 256 128 64; do
    ICON_FILE="$DESKTOP_DIR/icons/${size}x${size}.png"
    if [[ -f "$ICON_FILE" ]]; then
        cp "$ICON_FILE" "$DEB_DIR/$DEB_NAME/usr/share/icons/hicolor/${size}x${size}/apps/${APP_ID}.png"
    fi
done

# Create control file
cat > "$DEB_DIR/$DEB_NAME/DEBIAN/control" << EOF
Package: $APP_NAME
Version: $APP_VERSION
Section: utils
Priority: optional
Architecture: amd64
Maintainer: $APP_MAINTAINER
Description: $APP_DESCRIPTION
 $APP_DISPLAY_NAME is a secure, self-hosted file synchronization client
 with end-to-end encryption, WebDAV support, and collaborative editing.
Homepage: $APP_URL
EOF

# Build .deb
(cd "$DEB_DIR" && fakeroot dpkg-deb --build "$DEB_NAME")

echo "Created: $DEB_DIR/${DEB_NAME}.deb"
cp "$DEB_DIR/${DEB_NAME}.deb" "$DIST_DIR/"

# ---------------------------------------------------------------------------
# Create .rpm package
# ---------------------------------------------------------------------------
echo ""
echo "--- Creating .rpm package ---"

RPM_DIR="$DIST_DIR/rpm"
mkdir -p "$RPM_DIR"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

# Copy binary into BUILDROOT layout
RPM_BUILDROOT="$RPM_DIR/BUILDROOT/$APP_NAME-$APP_VERSION-1.x86_64"
mkdir -p "$RPM_BUILDROOT/usr/bin"
mkdir -p "$RPM_BUILDROOT/usr/share/applications"
mkdir -p "$RPM_BUILDROOT/usr/share/icons/hicolor/256x256/apps"

cp "$BINARY_PATH" "$RPM_BUILDROOT/usr/bin/$APP_NAME"

cat > "$RPM_BUILDROOT/usr/share/applications/${APP_ID}.desktop" << EOF
[Desktop Entry]
Type=Application
Name=$APP_DISPLAY_NAME
Exec=/usr/bin/$APP_NAME
Icon=$APP_ID
Comment=$APP_DESCRIPTION
Categories=$APP_CATEGORY;
Terminal=false
EOF

# Create spec file
cat > "$RPM_DIR/SPECS/${APP_NAME}.spec" << EOF
Name:           $APP_NAME
Version:        $APP_VERSION
Release:        1%{?dist}
Summary:        $APP_DISPLAY_NAME - $APP_DESCRIPTION
License:        AGPL-3.0-or-later
URL:            $APP_URL
Source0:        %{name}-%{version}.tar.gz

%description
$APP_DISPLAY_NAME is a secure, self-hosted file synchronization client
with end-to-end encryption, WebDAV support, and collaborative editing.

%install
mkdir -p %{buildroot}/usr/bin
cp $APP_NAME %{buildroot}/usr/bin/$APP_NAME
mkdir -p %{buildroot}/usr/share/applications
cp ${APP_ID}.desktop %{buildroot}/usr/share/applications/
mkdir -p %{buildroot}/usr/share/icons/hicolor/256x256/apps

%files
/usr/bin/$APP_NAME
/usr/share/applications/${APP_ID}.desktop

%changelog
* $(date '+%a %b %d %Y') $APP_MAINTAINER - $APP_VERSION-1
- Initial RPM package
EOF

# Build RPM
rpmbuild --define "_topdir $RPM_DIR" \
         --define "_buildrootdir $RPM_DIR/BUILDROOT" \
         -bb "$RPM_DIR/SPECS/${APP_NAME}.spec" 2>/dev/null || \
    echo "WARNING: RPM build skipped (rpmbuild may not be installed)"

# Copy RPM if built
RPM_FILE=$(find "$RPM_DIR/RPMS" -name "*.rpm" 2>/dev/null | head -1)
if [[ -n "$RPM_FILE" ]]; then
    cp "$RPM_FILE" "$DIST_DIR/"
    echo "Created: $DIST_DIR/$(basename "$RPM_FILE")"
else
    echo "RPM not built (install rpm-build to enable)"
fi

# ---------------------------------------------------------------------------
# Create AppImage
# ---------------------------------------------------------------------------
echo ""
echo "--- Creating AppImage ---"

APPIMAGE_DIR="$DIST_DIR/appimage"
mkdir -p "$APPIMAGE_DIR/usr/bin"
mkdir -p "$APPIMAGE_DIR/usr/share/applications"
mkdir -p "$APPIMAGE_DIR/usr/share/icons/256x256"

cp "$BINARY_PATH" "$APPIMAGE_DIR/usr/bin/$APP_NAME"

cat > "$APPIMAGE_DIR/usr/share/applications/${APP_ID}.desktop" << EOF
[Desktop Entry]
Type=Application
Name=$APP_DISPLAY_NAME
Exec=$APP_NAME
Icon=$APP_ID
Comment=$APP_DESCRIPTION
Categories=$APP_CATEGORY;
Terminal=false
EOF

if [[ -f "$DESKTOP_DIR/icons/256x256.png" ]]; then
    cp "$DESKTOP_DIR/icons/256x256.png" "$APPIMAGE_DIR/usr/share/icons/256x256/${APP_ID}.png"
fi

# Use appimagetool if available
if command -v appimagetool &> /dev/null; then
    ARCH=x86_64 appimagetool "$APPIMAGE_DIR" "$DIST_DIR/${APP_NAME}-${APP_VERSION}-x86_64.AppImage" || \
        echo "WARNING: AppImage creation failed"
    if [[ -f "$DIST_DIR/${APP_NAME}-${APP_VERSION}-x86_64.AppImage" ]]; then
        echo "Created: $DIST_DIR/${APP_NAME}-${APP_VERSION}-x86_64.AppImage"
    fi
else
    echo "WARNING: appimagetool not found, skipping AppImage creation"
    echo "Install with: sudo apt install appimagetool or download from https://github.com/AppImage/AppImageKit"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "============================================"
echo "  Build Complete!"
echo "============================================"
echo ""
echo "Artifacts in $DIST_DIR:"
ls -lh "$DIST_DIR"/*.deb "$DIST_DIR"/*.rpm "$DIST_DIR"/*.AppImage 2>/dev/null || true
echo ""
echo "Install examples:"
echo "  .deb:  sudo dpkg -i $DIST_DIR/${DEB_NAME}.deb"
echo "  .rpm:  sudo rpm -i $DIST_DIR/*.rpm"
echo "  .AppImage: chmod +x $DIST_DIR/*.AppImage && ./$DIST_DIR/*.AppImage"

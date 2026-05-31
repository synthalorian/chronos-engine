#!/usr/bin/env bash
#!/usr/bin/env bash
# Build AppImage for Chronos Engine Editor
# Requires: cargo, linuxdeploy (https://github.com/linuxdeploy/linuxdeploy)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target/appimage"
APPDIR="$BUILD_DIR/AppDir"

APP_NAME="chronos-editor"
APP_VERSION="$(cd "$PROJECT_ROOT" && grep '^version = ' Cargo.toml | head -1 | sed 's/.*= "\(.*\)"/\1/')"
APP_IMAGE="Chronos-Engine-Editor-${APP_VERSION}-x86_64.AppImage"

echo "=== Building AppImage for Chronos Engine Editor v${APP_VERSION} ==="

# Ensure linuxdeploy is available
if ! command -v linuxdeploy &>/dev/null; then
    echo "Error: linuxdeploy not found. Download it from:"
    echo "  https://github.com/linuxdeploy/linuxdeploy/releases"
    exit 1
fi

# Build release binary
echo "Building release binary..."
cd "$PROJECT_ROOT"
cargo build --bin chronos-editor --features editor --release

# Also build the CLI tool
cargo build --bin chronos --features full --release

# Prepare AppDir
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"
mkdir -p "$APPDIR/usr/share/metainfo"

cp "$PROJECT_ROOT/target/release/chronos-editor" "$APPDIR/usr/bin/"
cp "$PROJECT_ROOT/target/release/chronos" "$APPDIR/usr/bin/"

# Desktop entry (use the canonical one from packaging/)
cp "$SCRIPT_DIR/chronos-editor.desktop" "$APPDIR/usr/share/applications/chronos-editor.desktop"
cp "$APPDIR/usr/share/applications/chronos-editor.desktop" "$APPDIR/"

# Copy icon from project root
if [ -f "$PROJECT_ROOT/icon.png" ]; then
    cp "$PROJECT_ROOT/icon.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/chronos-editor.png"
    echo "  ✓ Icon copied from project root"
else
    echo "  ⚠ icon.png not found, creating placeholder"
    touch "$APPDIR/usr/share/icons/hicolor/256x256/apps/chronos-editor.png"
fi

# AppStream metadata
cat > "$APPDIR/usr/share/metainfo/com.chronosengine.editor.appdata.xml" <<XML
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>com.chronosengine.editor</id>
  <metadata_license>MIT</metadata_license>
  <project_license>MIT</project_license>
  <name>Chronos Engine Editor</name>
  <summary>Cross-platform game engine editor</summary>
  <description>
    <p>Chronos Engine is a Rust-based cross-platform game engine with an integrated editor.
    Features ECS core, spatial indexing, 2D/3D physics, rendering, animation, audio,
    scripting, and a full desktop editor.</p>
  </description>
  <url type="homepage">https://github.com/synthalorian/chronos-engine</url>
  <developer_name>synth</developer_name>
  <content_rating type="oars-1.1" />
  <releases>
    <release version="${APP_VERSION}" date="$(date +%Y-%m-%d)"/>
  </releases>
</component>
XML

# Build AppImage
echo "Creating AppImage..."
cd "$BUILD_DIR"
linuxdeploy \
    --appdir "$APPDIR" \
    --desktop-file "$APPDIR/chronos-editor.desktop" \
    --icon-file "$APPDIR/usr/share/icons/hicolor/256x256/apps/chronos-editor.png" \
    --executable "$APPDIR/usr/bin/chronos-editor" \
    --output appimage

echo ""
echo "=== AppImage built ==="
echo "  Output: $BUILD_DIR/$APP_IMAGE"
echo "  Version: ${APP_VERSION}"
echo ""
echo "To test:"
echo "  chmod +x '$BUILD_DIR/$APP_IMAGE'"
echo "  './$BUILD_DIR/$APP_IMAGE'"

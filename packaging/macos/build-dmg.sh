#!/usr/bin/env bash
# Build macOS .app bundle and DMG for Chronos Engine Editor
# Requires: cargo, create-dmg (https://github.com/create-dmg/create-dmg) or hdiutil
# Run on macOS only.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target/macos"
APP_NAME="Chronos Engine Editor"
APP_BUNDLE="$BUILD_DIR/$APP_NAME.app"
DMG_NAME="Chronos-Engine-Editor-x86_64.dmg"
UNIVERSAL_DMG_NAME="Chronos-Engine-Editor-universal.dmg"

echo "=== Building macOS App Bundle for Chronos Engine Editor ==="

# Build for x86_64 (Intel)
echo "Building x86_64 binary..."
cargo build --bin chronos-editor --features editor --release --target x86_64-apple-darwin

# Build for aarch64 (Apple Silicon)
echo "Building aarch64 binary..."
cargo build --bin chronos-editor --features editor --release --target aarch64-apple-darwin

# Create universal binary with lipo
echo "Creating universal binary..."
mkdir -p "$BUILD_DIR"
lipo -create \
    "$PROJECT_ROOT/target/x86_64-apple-darwin/release/chronos-editor" \
    "$PROJECT_ROOT/target/aarch64-apple-darwin/release/chronos-editor" \
    -output "$BUILD_DIR/chronos-editor-universal"

# Create app bundle structure
echo "Creating .app bundle..."
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

# Copy universal binary
cp "$BUILD_DIR/chronos-editor-universal" "$APP_BUNDLE/Contents/MacOS/chronos-editor"

# Create Info.plist
cat > "$APP_BUNDLE/Contents/Info.plist" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>chronos-editor</string>
    <key>CFBundleIdentifier</key>
    <string>com.chronosengine.editor</string>
    <key>CFBundleName</key>
    <string>Chronos Engine Editor</string>
    <key>CFBundleDisplayName</key>
    <string>Chronos Engine Editor</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>MacOSX</string>
    </array>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>MIT License</string>
</dict>
</plist>
EOF

# Copy icon (project should provide an .icns file)
if [ -f "$PROJECT_ROOT/icon.png" ]; then
    # Convert PNG to ICNS if sips is available
    mkdir -p "$APP_BUNDLE/Contents/Resources/icon.iconset"
    sips -z 16 16 "$PROJECT_ROOT/icon.png" \
        --out "$APP_BUNDLE/Contents/Resources/icon.iconset/icon_16x16.png" 2>/dev/null || true
    sips -z 32 32 "$PROJECT_ROOT/icon.png" \
        --out "$APP_BUNDLE/Contents/Resources/icon.iconset/icon_32x32.png" 2>/dev/null || true
    sips -z 128 128 "$PROJECT_ROOT/icon.png" \
        --out "$APP_BUNDLE/Contents/Resources/icon.iconset/icon_128x128.png" 2>/dev/null || true
    sips -z 256 256 "$PROJECT_ROOT/icon.png" \
        --out "$APP_BUNDLE/Contents/Resources/icon.iconset/icon_256x256.png" 2>/dev/null || true
    sips -z 512 512 "$PROJECT_ROOT/icon.png" \
        --out "$APP_BUNDLE/Contents/Resources/icon.iconset/icon_512x512.png" 2>/dev/null || true
    iconutil -c icns \
        "$APP_BUNDLE/Contents/Resources/icon.iconset" \
        -o "$APP_BUNDLE/Contents/Resources/icon.icns" 2>/dev/null || true
    rm -rf "$APP_BUNDLE/Contents/Resources/icon.iconset"
fi

# Copy license
cp "$PROJECT_ROOT/LICENSE" "$APP_BUNDLE/Contents/Resources/LICENSE"

# Sign the app bundle (ad-hoc for distribution)
codesign --force --deep --sign - "$APP_BUNDLE" 2>/dev/null || true

echo "  .app bundle created at: $APP_BUNDLE"

# Create DMG
echo ""
echo "Creating DMG..."
if command -v create-dmg &>/dev/null; then
    create-dmg \
        --volname "$APP_NAME" \
        --volicon "$APP_BUNDLE/Contents/Resources/icon.icns" \
        --window-pos 200 120 \
        --window-size 600 400 \
        --icon-size 100 \
        --icon "$APP_NAME.app" 175 120 \
        --hide-extension "$APP_NAME.app" \
        --app-drop-link 425 120 \
        "$BUILD_DIR/$DMG_NAME" \
        "$APP_BUNDLE" 2>/dev/null || {
        echo "  create-dmg not available, using hdiutil..."
        hdiutil create -volname "$APP_NAME" \
            -srcfolder "$APP_BUNDLE" \
            -ov -format UDZO \
            "$BUILD_DIR/$DMG_NAME"
    }
else
    echo "  Using hdiutil..."
    hdiutil create -volname "$APP_NAME" \
        -srcfolder "$APP_BUNDLE" \
        -ov -format UDZO \
        "$BUILD_DIR/$DMG_NAME"
fi

echo ""
echo "=== Build Complete ==="
echo "  .app bundle: $APP_BUNDLE"
echo "  DMG: $BUILD_DIR/$DMG_NAME"
echo ""
echo "Note: Run this script on macOS with Xcode command line tools installed."

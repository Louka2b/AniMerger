#!/bin/bash
set -e

# Assurer que les dépendances sont téléchargées
./scripts/fetch_deps.sh

APP_NAME="AniMerger"
DIST_DIR="dist"
APP_DIR="$DIST_DIR/$APP_NAME.app"

echo "Building release binary for macOS..."
cargo build --release

echo "Creating macOS .app bundle structure..."
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

cp target/release/animerger "$APP_DIR/Contents/MacOS/$APP_NAME"
chmod +x "$APP_DIR/Contents/MacOS/$APP_NAME"

# Embedding mkvmerge
if [ -f "build/mac/mkvmerge" ]; then
    cp "build/mac/mkvmerge" "$APP_DIR/Contents/MacOS/mkvmerge"
elif command -v mkvmerge &> /dev/null; then
    cp "$(which mkvmerge)" "$APP_DIR/Contents/MacOS/mkvmerge"
fi
chmod +x "$APP_DIR/Contents/MacOS/mkvmerge" || true

# Embedding alass
if [ -f "build/mac/alass" ]; then
    cp "build/mac/alass" "$APP_DIR/Contents/MacOS/alass"
fi
chmod +x "$APP_DIR/Contents/MacOS/alass" || true

if [ -f "assets/animerger.png" ]; then
    cp "assets/animerger.png" "$APP_DIR/Contents/Resources/animerger.png"
fi

cat << EOF > "$APP_DIR/Contents/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundleIconFile</key>
    <string>animerger.png</string>
    <key>CFBundleIdentifier</key>
    <string>com.louka2b.animerger</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>2.4.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
</dict>
</plist>
EOF

echo "macOS App bundle created successfully at: $APP_DIR"
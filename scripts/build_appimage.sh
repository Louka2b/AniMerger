#!/usr/bin/env bash
set -euo pipefail

./scripts/fetch_deps.sh

APP_NAME="AniMerger"
DIST_DIR="dist"
APPDIR="AppDir"

echo "Building release binary for Linux..."
cargo build --release

echo "Creating AppDir structure..."
rm -rf "$APPDIR" "$DIST_DIR/AniMerger*.AppImage"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"

cp target/release/animerger "$APPDIR/usr/bin/animerger"
chmod +x "$APPDIR/usr/bin/animerger"

# Embedding mkvmerge & alass
cp build/linux/mkvmerge "$APPDIR/usr/bin/mkvmerge"
chmod +x "$APPDIR/usr/bin/mkvmerge"

if [ -f "build/linux/alass" ]; then
    cp build/linux/alass "$APPDIR/usr/bin/alass"
    chmod +x "$APPDIR/usr/bin/alass"
fi

cat << EOF > "$APPDIR/animerger.desktop"
[Desktop Entry]
Type=Application
Name=AniMerger
Exec=animerger
Icon=animerger
Categories=AudioVideo;
EOF
cp "$APPDIR/animerger.desktop" "$APPDIR/usr/share/applications/"

if [ -f "assets/animerger.png" ]; then
    cp "assets/animerger.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/animerger.png"
    cp "assets/animerger.png" "$APPDIR/animerger.png"
fi

if [ ! -f "appimagetool-x86_64.AppImage" ]; then
    wget -q https://github.com/AppImage/AppImageKit/releases/download/13/appimagetool-x86_64.AppImage
    chmod +x appimagetool-x86_64.AppImage
fi

ARCH=x86_64 ./appimagetool-x86_64.AppImage "$APPDIR" "$DIST_DIR/AniMerger-x86_64.AppImage"
echo "Linux AppImage created successfully in dist/"
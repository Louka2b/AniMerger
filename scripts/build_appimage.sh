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

cat << 'EOF' > "$APPDIR/AppRun"
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")"
exec "$HERE/usr/bin/animerger" "$@"
EOF
chmod +x "$APPDIR/AppRun"

APPIMAGETOOL="appimagetool-x86_64.AppImage"
if [ ! -f "$APPIMAGETOOL" ]; then
    echo "Downloading the latest appimagetool..."
    appimagetool_url="$(curl --fail --silent --show-error https://api.github.com/repos/AppImage/appimagetool/releases/latest |
        grep -F '"browser_download_url"' |
        grep -F '/appimagetool-x86_64.AppImage"' |
        cut -d '"' -f 4)"
    if [ -z "$appimagetool_url" ]; then
        echo "Error: appimagetool Linux x86_64 asset was not found." >&2
        exit 1
    fi
    curl --fail --location --retry 3 --retry-delay 2 --output "$APPIMAGETOOL" "$appimagetool_url"
    chmod +x "$APPIMAGETOOL"
fi

APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 ./$APPIMAGETOOL "$APPDIR" "$DIST_DIR/AniMerger-x86_64.AppImage"
echo "Linux AppImage created successfully in dist/"
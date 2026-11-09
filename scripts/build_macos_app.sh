#!/usr/bin/env bash
set -euo pipefail

APP_NAME="AniMerger"
BUNDLE_NAME="${APP_NAME}.app"
DIST_DIR="dist"
CONTENTS_DIR="${DIST_DIR}/${BUNDLE_NAME}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo "Building release binary for macOS..."
cargo build --release

echo "Creating macOS .app bundle structure..."
rm -rf "${DIST_DIR}/${BUNDLE_NAME}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

# Copie du binaire principal
cp "target/release/animerger" "${MACOS_DIR}/${APP_NAME}"
chmod +x "${MACOS_DIR}/${APP_NAME}"

# Intégration de mkvmerge (s'il est installé via Homebrew, on l'embarque pour qu'il soit autonome)
MKVMERGE_BIN=$(command -v mkvmerge || true)
if [ -n "${MKVMERGE_BIN}" ]; then
    echo "Embedding mkvmerge from system: ${MKVMERGE_BIN}"
    cp "${MKVMERGE_BIN}" "${MACOS_DIR}/mkvmerge"
    chmod +x "${MACOS_DIR}/mkvmerge"
else
    echo "Warning: mkvmerge not found on system. Make sure to install it via 'brew install mkvtoolnix'."
fi

ALASS_BIN=$(command -v alass || true)
if [ -n "${ALASS_BIN}" ]; then
    echo "Embedding Alass from system: ${ALASS_BIN}"
    cp "${ALASS_BIN}" "${MACOS_DIR}/alass"
    chmod +x "${MACOS_DIR}/alass"
else
    echo "Warning: alass not found on system. Install it separately to enable subtitle synchronization."
fi

# Intégration de l'icône si elle existe
if [ -f "assets/animerger.png" ]; then
    # Optionnel : conversion du PNG en ICNS si iconutil est disponible, sinon on garde le PNG
    cp "assets/animerger.png" "${RESOURCES_DIR}/animerger.png"
fi

# Création du fichier Info.plist requis pour macOS
cat > "${CONTENTS_DIR}/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>AniMerger</string>
    <key>CFBundleIdentifier</key>
    <string>com.louka.animerger</string>
    <key>CFBundleName</key>
    <string>AniMerger</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleIconFile</key>
    <string>animerger.icns</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>HighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

echo "macOS App bundle created successfully at: ${DIST_DIR}/${BUNDLE_NAME}"

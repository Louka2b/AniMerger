#!/usr/bin/env bash
set -euo pipefail

APP="animerger"
ARCH="x86_64"
TARGET_BIN="target/release/${APP}"
APPDIR="build/${APP}.AppDir"
DIST_DIR="dist"

MKVMERGE_BIN=$(command -v mkvmerge || true)
if [ -z "${MKVMERGE_BIN}" ]; then
    echo "Error: mkvmerge not found on system. Run: sudo dnf install -y mkvtoolnix"
    exit 1
fi
ALASS_BIN="${ALASS_BIN:-build/linux/alass}"
if [ ! -f "${ALASS_BIN}" ]; then
    echo "Error: Alass not found at ${ALASS_BIN}. Run: make prepare-linux-alass"
    exit 1
fi

rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin" "${APPDIR}/usr/lib" "${DIST_DIR}"

cp "${TARGET_BIN}" "${APPDIR}/usr/bin/${APP}"
cp "${MKVMERGE_BIN}" "${APPDIR}/usr/bin/mkvmerge"
cp "${ALASS_BIN}" "${APPDIR}/usr/bin/alass"
chmod +x "${APPDIR}/usr/bin/${APP}" "${APPDIR}/usr/bin/mkvmerge" "${APPDIR}/usr/bin/alass"

# Copie de l'icone PNG dans l'AppDir
if [ -f "assets/animerger.png" ]; then
    cp "assets/animerger.png" "${APPDIR}/${APP}.png"
fi

ldd "${MKVMERGE_BIN}" | awk '/=> \// {print $3}' | while read -r lib; do
    base=$(basename "${lib}")
    case "${base}" in
        libc.so*|libm.so*|libdl.so*|libpthread.so*|librt.so*|libresolv.so*|ld-linux*)
            ;;
        *)
            cp -L "${lib}" "${APPDIR}/usr/lib/"
            ;;
    esac
done

cat > "${APPDIR}/AppRun" << 'EOF'
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH:-}"
exec "${HERE}/usr/bin/animerger" "$@"
EOF
chmod +x "${APPDIR}/AppRun"

cat > "${APPDIR}/${APP}.desktop" << EOF
[Desktop Entry]
Type=Application
Name=AniMerger
Comment=Merge high-quality anime video with target language audio and subtitles
Exec=${APP}
Icon=${APP}
Categories=AudioVideo;Video;
Terminal=false
EOF

if ! command -v appimagetool >/dev/null 2>&1; then
    if [ ! -f build/appimagetool ]; then
        echo "Downloading appimagetool..."
        curl -fsSL -o build/appimagetool \
            "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
        chmod +x build/appimagetool
    fi
    APPIMAGETOOL="./build/appimagetool --appimage-extract-and-run"
else
    APPIMAGETOOL="appimagetool"
fi

ARCH=${ARCH} ${APPIMAGETOOL} "${APPDIR}" "${DIST_DIR}/${APP}-${ARCH}.AppImage"
echo "Standalone AppImage created: ${DIST_DIR}/${APP}-${ARCH}.AppImage"
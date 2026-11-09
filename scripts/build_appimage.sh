#!/usr/bin/env bash
set -euo pipefail

APP="animerger"
ARCH="x86_64"
TARGET_BIN="target/release/${APP}"
APPDIR="build/${APP}.AppDir"
DIST_DIR="dist"

MKVMERGE_BIN=$(command -v mkvmerge || true)
if [ -z "${MKVMERGE_BIN}" ]; then
    echo "Erreur: mkvmerge introuvable sur le systeme. Lance: sudo dnf install -y mkvtoolnix"
    exit 1
fi

rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin" "${APPDIR}/usr/lib" "${DIST_DIR}"

cp "${TARGET_BIN}" "${APPDIR}/usr/bin/${APP}"
cp "${MKVMERGE_BIN}" "${APPDIR}/usr/bin/mkvmerge"
chmod +x "${APPDIR}/usr/bin/${APP}" "${APPDIR}/usr/bin/mkvmerge"

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

cat > "${APPDIR}/${APP}.svg" << 'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">
  <rect width="256" height="256" rx="48" fill="#1e1b4b"/>
  <path d="M64 80 L128 144 L192 80 M128 144 L128 200" stroke="#818cf8" stroke-width="20" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
</svg>
EOF

if ! command -v appimagetool >/dev/null 2>&1; then
    if [ ! -f build/appimagetool ]; then
        echo "Telechargement de appimagetool..."
        curl -fsSL -o build/appimagetool \
            "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
        chmod +x build/appimagetool
    fi
    APPIMAGETOOL="./build/appimagetool --appimage-extract-and-run"
else
    APPIMAGETOOL="appimagetool"
fi

ARCH=${ARCH} ${APPIMAGETOOL} "${APPDIR}" "${DIST_DIR}/${APP}-${ARCH}.AppImage"
echo "AppImage autonome cree : ${DIST_DIR}/${APP}-${ARCH}.AppImage"
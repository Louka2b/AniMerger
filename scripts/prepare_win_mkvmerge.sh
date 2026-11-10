#!/usr/bin/env bash
set -euo pipefail

OUT_EXE="build/win/mkvmerge.exe"
OUT_ALASS="build/win/alass.exe"

if [ -f "${OUT_EXE}" ]; then
    echo "mkvmerge.exe déjà présent dans ${OUT_EXE}"
    exit 0
fi

mkdir -p build/win

if [ ! -f "${OUT_ALASS}" ]; then
    ALASS_ARCHIVE="build/win/alass-windows.zip"
    ALASS_URL=$(curl --fail --silent --show-error https://api.github.com/repos/kaegi/alass/releases/latest |
        grep -F '"browser_download_url"' |
        grep -F '/alass-windows64.zip"' |
        cut -d '"' -f 4)
    if [ -z "${ALASS_URL}" ]; then
        echo "Erreur : l'archive Alass Windows est introuvable dans la dernière release."
        exit 1
    fi
    echo "Téléchargement d'Alass pour Windows..."
    curl --fail --location --retry 3 --retry-delay 2 -o "${ALASS_ARCHIVE}" "${ALASS_URL}"
    unzip -p "${ALASS_ARCHIVE}" '*/bin/alass-cli.exe' > "${OUT_ALASS}"
    if [ ! -s "${OUT_ALASS}" ]; then
        echo "Erreur : alass-cli.exe est introuvable dans l'archive Alass."
        rm -f "${OUT_ALASS}"
        exit 1
    fi
    chmod +x "${OUT_ALASS}"
    rm -f "${ALASS_ARCHIVE}"
fi

SEVENZIP=$(command -v 7z || command -v 7za || true)
BSDTAR=$(command -v bsdtar || true)
if [ -z "${SEVENZIP}" ] && [ -z "${BSDTAR}" ]; then
    echo "Erreur : 7z ou bsdtar est requis pour extraire MKVToolNix."
    exit 1
fi

# Utilisation directe d'une version stable connue de MKVToolNix pour éviter les erreurs de parsing HTML
VERSION="88.0"
ARCHIVE="build/win/mkvtoolnix.7z"
URL="https://mkvtoolnix.download/windows/releases/${VERSION}/mkvtoolnix-64-bit-${VERSION}.7z"

echo "Téléchargement de MKVToolNix ${VERSION} pour Windows..."
curl -fL -o "${ARCHIVE}" "${URL}"

echo "Extraction de mkvmerge.exe..."
if [ -n "${SEVENZIP}" ]; then
    "${SEVENZIP}" e -y "${ARCHIVE}" "mkvtoolnix/mkvmerge.exe" -obuild/win/ >/dev/null
else
    "${BSDTAR}" -xOf "${ARCHIVE}" "mkvtoolnix/mkvmerge.exe" > "${OUT_EXE}"
fi
if [ ! -s "${OUT_EXE}" ]; then
    echo "Erreur : mkvmerge.exe est introuvable dans l'archive MKVToolNix."
    exit 1
fi
rm -f "${ARCHIVE}"

echo "mkvmerge.exe prêt dans ${OUT_EXE}"
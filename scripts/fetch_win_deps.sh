#!/bin/bash
set -e

WIN_DIR="build/win"
mkdir -p "$WIN_DIR"

# 1. Télécharger Alass pour Windows (version statique officielle)
if [ ! -f "$WIN_DIR/alass.exe" ]; then
    echo "Downloading Alass for Windows..."
    # Téléchargement de la dernière version d'alass depuis GitHub releases
    ALASS_URL=$(curl -s https://api.github.com/repos/kaegi/alass/releases/latest | grep "browser_download_url.*windows.*exe" | cut -d '"' -f 4)
    if [ -z "$ALASS_URL" ]; then
        # Fallback lien direct stable
        ALASS_URL="https://github.com/kaegi/alass/releases/download/v0.2.2/alass-windows-amd64.exe"
    fi
    curl -L -o "$WIN_DIR/alass.exe" "$ALASS_URL"
    chmod +x "$WIN_DIR/alass.exe"
fi

# 2. Pour mkvmerge (MKVToolNix), comme l'archive officielle est lourde ou change, 
# tu peux placer ton mkvmerge.exe dans build/win/ manuellement une fois, 
# ou le télécharger depuis une source fixe de ton choix.
if [ ! -f "$WIN_DIR/mkvmerge.exe" ]; then
    echo "Please place mkvmerge.exe manually inside build/win/mkvmerge.exe"
    echo "You can extract it from the official MKVToolNix portable version for Windows."
fi

echo "Windows binaries checked in build/win/"
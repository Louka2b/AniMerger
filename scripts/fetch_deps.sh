#!/usr/bin/env bash
set -euo pipefail

download() {
    local url="$1"
    local destination="$2"

    curl --fail --location --retry 3 --retry-delay 2 --output "$destination" "$url"
}

latest_alass_asset() {
    local asset_name="$1"
    local url

    url="$(curl --fail --silent --show-error https://api.github.com/repos/kaegi/alass/releases/latest |
        grep -F "\"browser_download_url\"" |
        grep -F "/${asset_name}\"" |
        cut -d '"' -f 4)"

    if [ -z "$url" ]; then
        echo "Error: Alass asset '${asset_name}' was not found in the latest release." >&2
        exit 1
    fi
    printf '%s\n' "$url"
}

# Dossiers de build
mkdir -p build/mac build/linux build/win

# --- LINUX ---
echo "Checking Linux dependencies..."
if [ ! -f "build/linux/alass" ]; then
    echo "Downloading Alass for Linux..."
    download "$(latest_alass_asset "alass-linux64")" "build/linux/alass"
    chmod +x "build/linux/alass"
fi

if [ ! -f "build/linux/mkvmerge" ]; then
    echo "Locating mkvmerge for Linux..."
    if command -v mkvmerge &> /dev/null; then
        cp "$(which mkvmerge)" build/linux/mkvmerge
    else
        echo "Error: mkvmerge was not found."
        echo "Install MKVToolNix (for example: sudo apt install mkvtoolnix)"
        echo "or place the binary at build/linux/mkvmerge."
        exit 1
    fi
    chmod +x "build/linux/mkvmerge"
fi

# --- MACOS ---
echo "Checking macOS dependencies..."
if [ ! -f "build/mac/alass" ]; then
    echo "Alass for macOS is not published by the current release; using a local installation if available."
    if command -v alass >/dev/null 2>&1; then
        cp "$(command -v alass)" build/mac/alass
        chmod +x "build/mac/alass"
    fi
fi

if [ ! -f "build/mac/mkvmerge" ]; then
    if command -v mkvmerge &> /dev/null; then
        cp "$(which mkvmerge)" build/mac/mkvmerge
    fi
fi
chmod +x "build/mac/mkvmerge" 2>/dev/null || true

# --- WINDOWS ---
echo "Checking Windows dependencies..."
if [ ! -f "build/win/alass.exe" ]; then
    echo "Alass for Windows will be downloaded by prepare_win_mkvmerge.sh."
fi

if [ ! -f "build/win/mkvmerge.exe" ]; then
    echo "mkvmerge.exe is not present yet; the Windows target will download it when possible."
fi

echo "Dependencies check completed!"

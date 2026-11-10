#!/bin/bash
set -e

# Dossiers de build
mkdir -p build/mac build/linux build/win

# --- LINUX ---
echo "Checking Linux dependencies..."
if [ ! -f "build/linux/alass" ]; then
    echo "Downloading Alass for Linux..."
    curl -L -o "build/linux/alass" "https://github.com/kaegi/alass/releases/download/v0.2.2/alass-linux-amd64"
    chmod +x "build/linux/alass"
fi

if [ ! -f "build/linux/mkvmerge" ]; then
    echo "Locating or downloading mkvmerge for Linux..."
    if command -v mkvmerge &> /dev/null; then
        cp "$(which mkvmerge)" build/linux/mkvmerge
    else
        # Fallback si mkvmerge n'est pas installé sur la machine hôte
        echo "Downloading mkvmerge static package..."
        curl -L -o mkvtoolnix.tar.xz "https://mkvtoolnix.download/sources/mkvtoolnix-83.0.tar.xz" || true
        if [ -f "mkvtoolnix.tar.xz" ]; then
            tar -xf mkvtoolnix.tar.xz || true
            if [ -d "mkvtoolnix-83.0" ]; then
                cp mkvtoolnix-83.0/src/propedit/mkvmerge build/linux/mkvmerge || true
                rm -rf mkvtoolnix-83.0
            fi
            rm -f mkvtoolnix.tar.xz
        fi
        # Si le binaire n'a pas pu être compilé depuis les sources, on avertit l'utilisateur
        if [ ! -f "build/linux/mkvmerge" ]; then
            echo "Error: Could not automatically build/fetch mkvmerge for Linux. Please install it via 'sudo apt install mkvtoolnix' or place it in build/linux/mkvmerge"
            exit 1
        fi
    fi
    chmod +x "build/linux/mkvmerge"
fi

# --- MACOS ---
echo "Checking macOS dependencies..."
if [ ! -f "build/mac/alass" ]; then
    echo "Downloading Alass for macOS..."
    curl -L -o "build/mac/alass" "https://github.com/kaegi/alass/releases/download/v0.2.2/alass-macos"
    chmod +x "build/mac/alass"
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
    echo "Downloading Alass for Windows..."
    curl -L -o "build/win/alass.exe" "https://github.com/kaegi/alass/releases/download/v0.2.2/alass-windows-amd64.exe"
    chmod +x "build/win/alass.exe"
fi

if [ ! -f "build/win/mkvmerge.exe" ]; then
    echo "Please ensure mkvmerge.exe is placed in build/win/mkvmerge.exe for the Windows build."
fi

echo "Dependencies check completed!"
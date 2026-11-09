# AniMerger

> A native desktop application for merging anime releases without re-encoding.

AniMerger combines a high-quality video source with target-language audio and subtitles through [MKVToolNix](https://mkvtoolnix.download/). It runs on Linux and Windows and is built with Rust, [eframe](https://github.com/emilk/egui/tree/master/crates/eframe), and egui.

## Features

### Automatic episode matching

Scan two source folders and pair corresponding episodes using common naming formats:

- `S01E01`
- `E01`
- `Ep 01`
- Pure episode numbers

Resolution, codec, and CRC32 tags are ignored during matching.

### Flexible output naming

Choose how the merged files should be named:

- Keep the high-quality source name.
- Keep the target-language source name.
- Use a custom name with automatic episode numbering, such as `_E01`.

### Track selection

Configure the tracks from each source independently:

- **Source A - High Quality:** video, audio, subtitles, chapters, and attachments.
- **Source B - Target Language:** dubbed audio, subtitles, chapters, attachments, and default-track flags.
- Filter tracks by ISO language code, such as `jpn`, `fra`, or `eng`.

### Synchronization and batch processing

- Apply a global sync delay to every matched episode.
- Override the delay for individual episodes.
- Process episodes asynchronously with per-episode status updates.
- Follow the generated output through real-time logs.

## Requirements

The build scripts require the following tools on Linux:

- Rust and Cargo
- GNU Make
- MKVToolNix
- p7zip

For Windows builds, the toolchain also requires MinGW and the Windows GNU Rust target.

### Fedora

```bash
sudo dnf install -y p7zip p7zip-plugins mkvtoolnix
```

### Debian, Ubuntu, or Mint

```bash
sudo apt update
sudo apt install -y build-essential mingw-w64 curl \
  libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxkbcommon-dev libssl-dev \
  mkvtoolnix p7zip-full
```

## Build

Run the commands from the project root. Close any running AniMerger instance before rebuilding:

```bash
pkill animerger || true
```

| Command | Result |
| --- | --- |
| `make all` | Builds the Linux AppImage and Windows executable |
| `make linux` | Builds the native Linux executable |
| `make appimage` | Builds the Linux AppImage |
| `make windows` | Builds the Windows executable |
| `make clean` | Removes generated build files and packages |

Generated packages are written to `dist/`:

- `dist/animerger` - Linux executable
- `dist/animerger-x86_64.AppImage` - Linux AppImage
- `dist/animerger.exe` - Windows executable

## Project structure

```text
animerger/
├── Cargo.toml
├── Makefile
├── README.md
├── scripts/
│   ├── build_appimage.sh
│   └── prepare_win_mkvmerge.sh
└── src/
    └── main.rs
```

## License

AniMerger is released under the [MIT License](LICENSE).
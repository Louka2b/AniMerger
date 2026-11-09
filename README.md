# AniMerger

> A native desktop application for merging anime releases without re-encoding.

AniMerger combines a high-quality video release with audio and subtitles from a
second release by calling [`mkvmerge`](https://mkvtoolnix.download/). It keeps
the selected tracks and places them in a new MKV file; it does not re-encode
the video or audio.

AniMerger is written in Rust with
[eframe](https://github.com/emilk/egui/tree/master/crates/eframe) and egui.
Builds are supported on Linux, Windows, and macOS.

## Features

- Scan two folders and automatically match episodes.
- Match common episode formats such as `S01E01`, `1x01`, `E01`, `Ep 01`,
  `Episode 01`, and episode-only numbers.
- Add or remove pairs manually when automatic matching is not sufficient.
- Select video, audio, subtitles, chapters, and attachments independently
  from each source.
- Select track IDs for audio and subtitles, or leave the fields empty to keep
  all tracks of that type.
- Mark the target-language audio and subtitles as default tracks.
- Choose the output name from the high-quality file, the target-language file,
  or a custom name with automatic episode numbering (`MyAnime_E01.mkv`).
- Apply a global synchronization delay and override it for individual
  episodes.
- Process enabled episodes asynchronously with progress, per-episode status,
  and `mkvmerge` logs.

## Usage

1. Start AniMerger.
2. Select the **High Quality Folder (Source A)**.
3. Select the **Right Language Folder (Source B)**.
4. Select an **Output Folder**.
5. Click **Auto-Match Folders**.
6. Review the matched episodes. Use **Manual Match** to pair files that could
   not be detected automatically, or edit the episode tag and delay directly
   in the table.
7. Choose the tracks and output naming mode.
8. Enable or disable individual episodes, then click **Start Merge Batch**.

Supported input file extensions are `.mkv`, `.mp4`, `.webm`, `.avi`, and
`.m2ts`. Matching is based on the episode number extracted from each file
name. If several files produce the same episode number, only one of them can be
selected for that episode by automatic matching; use **Manual Match** when
duplicate episode numbers need to be disambiguated.

### Track selection

The audio and subtitle filter fields accept `mkvmerge` track IDs, for example
`1,2`. They are not ISO language codes. Leave a field empty to keep all tracks
of that type from the selected source.

By default, AniMerger keeps the video and audio from Source A, and the audio,
subtitles, default flags, and attachments from Source B. Chapters and other
track types can be changed before starting the batch.

## Requirements

All platforms require:

- Rust and Cargo
- GNU Make
- MKVToolNix (`mkvmerge`)

Additional tools are needed only for some packaging targets:

- Linux AppImage: `p7zip` is not required at runtime, but is used by some
  development workflows; `appimagetool` and Alass are downloaded automatically
  if they are not installed.
- Windows: MinGW, `unzip`, and the `x86_64-pc-windows-gnu` Rust target. The
  build scripts also require `7z` or `7za` and download the configured
  MKVToolNix and Alass releases.
- macOS: Homebrew is recommended for installing MKVToolNix. The macOS bundle
  embeds `mkvmerge` when it is available during the build.

### Fedora

```bash
sudo dnf install -y rust cargo make mkvtoolnix p7zip p7zip-plugins
```

### Debian, Ubuntu, or Mint

```bash
sudo apt update
sudo apt install -y build-essential curl mkvtoolnix p7zip-full \
  libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

### macOS

```bash
brew install rust make mkvtoolnix
```

## Build

Run commands from the project root. Close any running AniMerger instance
before rebuilding.

| Command | Result |
| --- | --- |
| `make linux` | Builds the native Linux executable in `dist/animerger`. |
| `make appimage` | Builds the Linux AppImage in `dist/animerger-x86_64.AppImage`. |
| `make windows` | Downloads `mkvmerge.exe` and Alass, then builds `dist/animerger.exe`. |
| `make macos` | Builds `dist/AniMerger.app`. |
| `make all` | Builds the Linux AppImage and Windows executable. |
| `make clean` | Removes Cargo build files, packaging files, and `dist/`. |

The Windows build embeds `mkvmerge.exe` and Alass in the executable. The Linux
AppImage bundles `mkvmerge`, Alass, and the required shared libraries. A macOS
app bundle embeds `mkvmerge` and Alass when they are found on the build
machine; otherwise, install them and rebuild.

## Project structure

```text
animerger/
├── Cargo.toml
├── Makefile
├── README.md
├── assets/
│   └── animerger.png
├── scripts/
│   ├── build_appimage.sh
│   ├── build_macos_app.sh
│   └── prepare_win_mkvmerge.sh
└── src/
    └── main.rs
```

## License

AniMerger is released under the
[AniMerger Non-Commercial Attribution License](LICENSE).

Non-commercial use, modification, and redistribution are permitted only when
the required attribution is preserved. Commercial use requires prior written
permission from Louka2b.

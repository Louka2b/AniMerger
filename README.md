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
- `curl` for dependency downloads
- MKVToolNix (`mkvmerge`)

Additional tools are needed only for some packaging targets:

- Linux AppImage: `wget` is used to download `appimagetool`; Alass is downloaded
  automatically and `mkvmerge` must be installed or copied to
  `build/linux/mkvmerge`.
- Windows: MinGW, `unzip`, `7z` or `7za`, and the
  `x86_64-pc-windows-gnu` Rust target. The build scripts download Alass and
  MKVToolNix when possible.
- macOS: Homebrew is recommended for installing MKVToolNix. The app bundle
  embeds `mkvmerge` when it is available during the build and embeds Alass when
  a local `alass` executable is available.

### Fedora

```bash
sudo dnf install -y rust cargo make mkvtoolnix p7zip p7zip-plugins
```

For the AppImage and Windows targets, also install `curl`, `wget`, and
`unzip` if they are not already available.

### Debian, Ubuntu, or Mint

```bash
sudo apt update
sudo apt install -y build-essential curl wget unzip mkvtoolnix p7zip-full \
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
| `make fetch` | Creates the build directories and checks/downloads platform dependencies. |
| `make linux` | Builds the Linux AppImage in `dist/AniMerger-x86_64.AppImage`. |
| `make windows` | Downloads Alass and MKVToolNix, then builds `target/x86_64-pc-windows-gnu/release/animerger.exe`. |
| `make macos` | Builds `dist/AniMerger.app`. |
| `make all` | Builds the macOS app, Linux AppImage, and Windows executable. |
| `make clean` | Removes Cargo build files, downloaded build dependencies, packaging files, and `dist/`. |

The Windows build embeds `mkvmerge.exe` and Alass in the executable. The Linux
AppImage bundles `mkvmerge` and Alass. A macOS app bundle embeds `mkvmerge` and
Alass when they are found on the build machine; otherwise, install them and
rebuild. The build scripts may leave downloaded dependency files in `build/`;
these files, along with all other build outputs, are ignored by Git.

### Windows cross-compilation

Install the GNU Windows target before running the Windows build:

```bash
rustup target add x86_64-pc-windows-gnu
make windows
```

The Windows packaging script uses the official MKVToolNix archive and requires
`7z` or `7za` to extract `mkvmerge.exe`.

## Matching and synchronization

Automatic matching extracts an episode key from each filename. It recognizes
formats such as `S01E01`, `1x01`, `E01`, `Ep 01`, `Episode 01`, and
episode-only numbers. When a key is duplicated, use the manual matching lists
to select the intended files and assign an episode tag.

The global delay is expressed in milliseconds and can be overridden for each
episode. **Auto-sync subtitles with Alass** is optional; if Alass fails for an
episode, AniMerger logs the error and continues with the original source file.
**Verify integrity post-mux** checks the `mkvmerge` output after each merge.

## Project structure

```text
animerger/
├── Cargo.toml
├── Makefile
├── README.md
├── .gitignore
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

# NomNom

A small, fast local utility hub for **Arch Linux**, **Debian / Ubuntu**, and **Windows 11**.

Everything runs on your own machine. No account, no telemetry, and nothing is sent anywhere except the downloads you ask for and an optional update check (Settings, on by default) that reads NomNom's own release page on GitHub.

## Features

| Module | What it does |
|---|---|
| **NomNom Downloader** | Paste a link, pick video or audio and a quality, download. Uses `yt-dlp` and `ffmpeg` (from your system `PATH` on Linux, or downloaded into NomNom's own tools folder on request). Queue, pause, resume, duplicate check, Firefox cookies, and history. |
| **Quick download** | `Ctrl+Alt+D` reads the link on your clipboard and opens a small popup window. Enter downloads it immediately. |
| **Firefox extension** | Adds a download button on YouTube, X, Instagram and Reddit posts. Talks to NomNom over Firefox Native Messaging and a per-user local socket/pipe (supports Firefox and LibreWolf). |
| **File Organizer** | Sorts a folder (`Downloads`, by default) into `Videos`, `Audio`, `Images` and `Documents`. Shows a full preview first, refuses to touch system or software folders, never overwrites an existing file (`RENAME_NOREPLACE` on Linux, `MOVEFILE_WRITE_THROUGH` without replace on Windows), and every run can be undone. Includes an automatic watch mode for new downloads. |
| **File Tools** | Converts images, audio, video, text, Markdown, CSV/JSON/YAML/TOML and (with LibreOffice) Office documents to PDF. Source files are only ever read; outputs write to temporary files and move atomically under a free name. |
| **Disk Analyzer** | Scans a drive, mount point or folder (optionally elevated via Polkit `pkexec` on Linux or UAC Administrator on Windows) and shows where space went: multi-column TreeGrid with `[files]` and free/system space, split list, cushioned treemap, sunburst ring chart, charts by kind, extension, age, size range and file owner, largest items, advanced search with presets, and duplicate finder (send to Trash / Recycle Bin, delete, move, zip, or replace duplicates with hardlinks). Supports filesystem compression (`FS_IOC_SETFLAGS` on Linux where supported, NTFS compression on Windows), extended attributes / NTFS streams, batch rename, saved scan files, reports (CSV, XLSX, PDF, HTML, JSON), and scheduled scan snapshots. |
| **File manager menu** | Right-click files to convert them or right-click a folder to analyze its disk space. Supports **KDE Dolphin** (`kio/servicemenus`), **GNOME Files / Nautilus** (`nautilus/scripts`), **Cinnamon Nemo** (`nemo/actions`), and **Windows 11 Explorer** (Windows 11 sparse package menu with classic HKCU fallback). |
| **Updates** | Checks GitHub Releases once a day when enabled and verifies downloads against the published `SHA256SUMS.txt`. Applies NSIS updates on Windows, replaces writable `.AppImage` binaries atomically on Linux, or saves `.pkg.tar.zst` / `.deb` packages to `Downloads` for `pacman` or `apt`. |

---

## Install & Build on Linux

### Arch Linux (Primary Linux Target)

See [packaging/arch/README.md](packaging/arch/README.md) for detailed notes on KDE Plasma, GNOME, Hyprland/Sway, Polkit agents, and LibreWolf.

1. Install build and runtime packages:

```bash
sudo pacman -S --needed base-devel rust nodejs npm pkgconf \
  webkit2gtk-4.1 gtk3 glib2 libayatana-appindicator hicolor-icon-theme
```

2. Optional runtime tools (NomNom detects system binaries on `PATH` automatically, or can download static `yt-dlp` and `ffmpeg` builds inside **Settings -> Tools**):

```bash
sudo pacman -S --needed yt-dlp ffmpeg libreoffice-fresh polkit
```

3. Build and install the native `.pkg.tar.zst` package:

```bash
./scripts/build-linux.sh --arch
sudo pacman -U dist/linux/nomnom-*.pkg.tar.zst
```

Or directly with `makepkg`:

```bash
cd packaging/arch
makepkg -si
```

### Debian / Ubuntu

Tested against Ubuntu 22.04 LTS, Ubuntu 24.04 LTS, and Debian 12+.

1. Install build dependencies (plus Rust `stable` via `rustup` and Node.js 20+):

```bash
sudo apt update
sudo apt install -y build-essential pkg-config curl file patchelf \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libssl-dev
```

2. Optional runtime packages:

```bash
sudo apt install -y ffmpeg libreoffice policykit-1
```

3. Build and install the `.deb` package (or portable `.AppImage`):

```bash
./scripts/build-linux.sh --deb
sudo apt install ./dist/linux/NOMNOM_*_amd64.deb

# Or build a self-contained AppImage:
./scripts/build-linux.sh --appimage
```

### Linux Desktop Notes

- **Wayland + NVIDIA GPUs**: On startup, if NomNom detects a Wayland session together with `/proc/driver/nvidia/version` and `WEBKIT_DISABLE_DMABUF_RENDERER` is unset, it sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` automatically to avoid blank WebKitGTK windows on proprietary NVIDIA drivers.
- **Elevated Disk Scans**: Scanning root-owned paths (such as `/var/lib/docker` or `/root`) uses `pkexec` over a `0600` Unix domain socket owned by your UID. Tiling Wayland compositors (Hyprland, Sway) require a running Polkit agent such as `polkit-kde-agent`, `hyprpolkitagent`, or `mate-polkit`.
- **Background Mode**: With no window open (`nomnom --background`), NomNom keeps no WebKitGTK web process alive, keeping idle memory low until you open the main window or press the Quick Download shortcut.

---

## Install & Build on Windows 11

Requirements: Windows 11, Rust (`stable-x86_64-pc-windows-msvc`), Node.js 20 or newer, and the Windows 10/11 SDK (only needed when building the signed sparse MSIX Explorer menu package). WebView2 ships with Windows 11.

```powershell
npm install
scripts\build-release.ps1                 # app, native host, Explorer package, Firefox add-on and NSIS installer
scripts\build-release.ps1 -SkipInstaller  # release binaries only
scripts\bench.ps1                         # memory, idle CPU and window ready time
```

---

## Development & Tests

```bash
npm install
npm run dev                      # Vite + Tauri with hot reload
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run check                    # svelte-check TypeScript + template verification
npm test                         # Vitest unit, accessibility contrast and UI copy tests
npm test -w @nomnom/firefox-extension
```

Linux memory, CPU and startup benchmark on a release build:

```bash
./scripts/build-linux.sh --skip-bundle
./scripts/bench-linux.sh
```

---

## Where NomNom Stores Data

NomNom never writes outside your user profile unless you run an explicit file operation:

| Purpose | Linux (XDG Base Directory) | Windows 11 |
|---|---|---|
| Data, SQLite history, snapshots, logs, managed tools | `$XDG_DATA_HOME/nomnom` (`~/.local/share/nomnom`) | `%LOCALAPPDATA%\NOMNOM` |
| Settings (`settings.json`) | `$XDG_CONFIG_HOME/nomnom` (`~/.config/nomnom`) | `%LOCALAPPDATA%\NOMNOM` |
| Cache & staged updates | `$XDG_CACHE_HOME/nomnom` (`~/.cache/nomnom`) | `%LOCALAPPDATA%\NOMNOM\cache` |
| Local IPC endpoint | `$XDG_RUNTIME_DIR/nomnom.sock` (`0600`) | `\\.\pipe\nomnom-<SID>` (SDDL per-user) |
| Firefox Native Messaging manifest | `~/.mozilla/native-messaging-hosts/com.nomnom.host.json` (plus `~/.librewolf/...` when LibreWolf is present) | `HKCU\Software\Mozilla\NativeMessagingHosts\com.nomnom.host` |

---

## Repository Layout

- `apps/desktop` — Svelte 5 + TypeScript UI and Tauri v2 host (`apps/desktop/src-tauri`)
- `apps/firefox-extension` — Manifest V3 Firefox & LibreWolf add-on
- `crates/nomnom-core` — XDG / Known Folder paths, settings, SQLite history, safe file moves, process groups
- `crates/nomnom-ipc` — Native Messaging framing and per-user Named Pipe / Unix domain socket transport
- `crates/nomnom-native-host` — Bridge binary launched by Firefox
- `crates/nomnom-tools` — `yt-dlp` and `ffmpeg` discovery, SHA-256 verified installs, and GitHub Release updater
- `crates/nomnom-downloads` — Download queue, `yt-dlp` runner, format selection, Firefox cookie discovery
- `crates/nomnom-converter` — Image, audio, video, document and structured text conversions
- `crates/nomnom-organizer` — Preview-first folder organizer with undo journal and system path guards
- `crates/nomnom-storage` — Parallel disk scanner, mount enumeration, duplicates, search, reports, Trash/Recycle Bin, Polkit/UAC elevation
- `crates/nomnom-linux` — Linux XDG autostart, Firefox/LibreWolf host registration, Dolphin/Nautilus/Nemo menus
- `crates/nomnom-win` & `crates/nomnom-shell` — Windows HKCU integrations and Windows 11 Explorer `IExplorerCommand` DLL
- `packaging/arch` — Arch Linux `PKGBUILD`, `.SRCINFO`, and Arch setup guide
- `packaging/linux` — Freedesktop `.desktop` entry and Debian template
- `packaging/sparse` — Windows 11 sparse MSIX manifest template
- `scripts` — Release build and benchmark scripts for Linux (`*.sh`) and Windows (`*.ps1`)

## License

MIT. `ffmpeg`, `yt-dlp` and LibreOffice are separate programs under their own licenses; NomNom uses your system packages when installed or downloads `yt-dlp` and `ffmpeg` on request without bundling them.

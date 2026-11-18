# NomNom

A small, fast utility hub for Windows 11. Everything runs on your PC. No account, no telemetry,
nothing is sent anywhere except the downloads you ask for and an update check (Settings, on by
default) that reads NomNom's own release page on GitHub.

| Part | What it does |
|---|---|
| **NomNom Downloader** | Paste a link, pick video or sound and a quality, download. Uses yt-dlp and FFmpeg, which NomNom fetches for you when you say so. Queue, pause, resume, history. |
| **Quick download** | `Ctrl+Alt+D` reads the link on your clipboard and pops up a small window. Enter downloads it. |
| **Firefox extension** | A download button on YouTube, X, Instagram and Reddit posts. The tab talks to NomNom through a small helper program. |
| **File Organizer** | Sorts a folder (Downloads, by default) into Videos, Audio, Images and Documents. It shows what it would do first, never touches folders that look like software, never replaces a file, and every move can be undone. An automatic mode handles new downloads. |
| **File Tools** | Converts pictures, sound, video, text, Markdown and (with LibreOffice) Office files. Your originals are never changed. |
| **Disk Analyzer** | Scans a drive or folder (optionally as Administrator) and shows where the space went: an expandable multi-column tree with `[files]` and free/system space, a split list, a cushioned treemap, a sunburst ring chart, charts by kind, type, age, size range and NTFS owner, biggest items, advanced search with quick presets, and duplicate files (deletable or replaceable with NTFS hardlinks). Supports in-place NTFS compression, batch rename, ZIP, saved scan files, reports (CSV, XLSX, PDF, HTML, JSON), scheduled snapshots, and an Explorer right-click folder action. |
| **Updates** | Looks for a newer release on GitHub about once a day (you can turn that off, or check by hand). Nothing is installed until you press the button; the installer is checked against the SHA-256 published with the release. |
| **Right-click menu** | Convert from Explorer. Uses the Windows 11 menu when Windows trusts the package, the classic menu otherwise. |

## Install

NomNom is not released yet. When it is, the installer is a per-user install that doesn't need
administrator rights.

## Build from source

Requirements: Windows 11, Rust (stable, MSVC), Node 20 or newer, the Windows SDK (only for the
right-click menu package). WebView2 ships with Windows 11.

```bash
npm install
npm run dev                      # Vite and the app, with live reload
cargo test --workspace
npm test                         # the app's tests
npm run check
```

```powershell
scripts\build-release.ps1        # app, helpers, menu package, extension and installer
scripts\build-release.ps1 -SkipInstaller
scripts\bench.ps1                # memory, CPU and start-up time of the release build
```

## License

MIT. FFmpeg and yt-dlp are separate programs with their own licences; NomNom downloads them on
request and doesn't include them.

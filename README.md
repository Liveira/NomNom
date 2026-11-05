# NOMNOM

A small, fast Windows utility hub: download media from links, organize files safely, and convert
files. Everything runs locally. No account, no telemetry.

**Status:** early development. The app shell, settings, tray and module system work. Downloads,
the organizer, file conversion and the Firefox extension are not built yet.

## Development

Requirements: Windows 11, Rust (stable, MSVC), Node 20+, WebView2 (included in Windows 11).

```bash
npm install
npm run dev          # starts Vite and the Tauri app
cargo test --workspace
npm test
```

Build an executable with the UI embedded: `npx tauri build --no-bundle` in `apps/desktop`.

## License

MIT

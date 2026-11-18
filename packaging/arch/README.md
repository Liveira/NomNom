# NomNom on Arch Linux

This folder contains the native Arch Linux `PKGBUILD` and `.SRCINFO` for building and installing `nomnom` with `pacman`.

## 1. Install Dependencies

Required build and runtime packages from the official Arch `core` and `extra` repositories:

```bash
sudo pacman -S --needed base-devel rust nodejs npm pkgconf \
  webkit2gtk-4.1 gtk3 glib2 libayatana-appindicator hicolor-icon-theme
```

Recommended optional packages:

```bash
sudo pacman -S --needed yt-dlp ffmpeg libreoffice-fresh polkit
```

- `yt-dlp` and `ffmpeg`: When installed via `pacman`, NomNom uses `/usr/bin/yt-dlp` and `/usr/bin/ffmpeg` automatically and leaves updates to `pacman -Syu`. If omitted, you can also install standalone static builds inside **Settings -> Tools**.
- `libreoffice-fresh` (or `libreoffice-still`): Enables Word, Excel, PowerPoint and OpenDocument to PDF conversion in **File Tools**.
- `polkit`: Provides `pkexec` so **Disk Analyzer** can scan root-owned folders (such as `/var/lib/docker`, `/var/cache`, or `/`) when you click **Scan with Polkit**.

## 2. Build and Install

From the repository root:

```bash
./scripts/build-linux.sh --arch
sudo pacman -U dist/linux/nomnom-*.pkg.tar.zst
```

Or directly inside `packaging/arch`:

```bash
cd packaging/arch
makepkg -si
```

To run the full Rust, Svelte and Firefox extension test suites during package creation:

```bash
cd packaging/arch
makepkg -si --check
```

`prepare()` stages a clean copy of the source tree into `src/nomnom`, excluding any host `target/` or `node_modules/` directories, so building on a dual-boot or shared checkout never mixes Windows and Linux artifacts.

## 3. What the Package Installs

| Path | Description |
|---|---|
| `/usr/bin/nomnom` | Main desktop application |
| `/usr/lib/nomnom/nomnom-native-host` | Firefox / LibreWolf Native Messaging bridge |
| `/usr/share/applications/nomnom.desktop` | Application launcher with `Analyze disk space` and `Start in system tray` actions |
| `/usr/share/icons/hicolor/*/apps/nomnom.png` | Application icons (`32x32` through `512x512`) |
| `/usr/share/nomnom/firefox-extension/*.zip` | Packaged Firefox add-on archive |
| `/usr/share/licenses/nomnom/LICENSE` | MIT license |

## 4. Desktop Environment & Wayland Notes

### KDE Plasma 6 (Dolphin)
Enabling **Add NomNom to the file manager right-click menu** in **Settings -> System** writes executable service menus (`0755`) to `~/.local/share/kio/servicemenus/`:
- `nomnom-convert.desktop` — convert selected image, audio, video or document files
- `nomnom-analyze.desktop` — open **Disk Analyzer** directly on the clicked folder

### GNOME (Files / Nautilus) & Cinnamon (Nemo)
The same toggle installs per-user scripts in `~/.local/share/nautilus/scripts/NomNom/` (available under **Right-click -> Scripts -> NomNom**) and Nemo actions in `~/.local/share/nemo/actions/`.

### Hyprland, Sway & Tiling Wayland Compositors
- **System tray**: Ensure your bar provides a `StatusNotifierWatcher` tray module (for example `tray` in `waybar`, `swaync`, or `ags`) so `nomnom --background` has a tray icon.
- **Polkit agent**: To use **Scan with Polkit** in Disk Analyzer on minimal Wayland sessions, make sure an authentication agent is running in your session config (for example `hyprpolkitagent`, `/usr/lib/polkit-kde-authentication-agent-1`, or `mate-polkit`).
- **NVIDIA on Wayland**: NomNom automatically sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` when `/proc/driver/nvidia/version` and `WAYLAND_DISPLAY` are both present. Set `WEBKIT_DISABLE_DMABUF_RENDERER=0` explicitly if you want to override this.

## 5. Connecting Firefox or LibreWolf

1. Open NomNom and go to **Settings -> Browser**.
2. Click **Connect Firefox**. This writes `com.nomnom.host.json` to `~/.mozilla/native-messaging-hosts/` (and `~/.librewolf/native-messaging-hosts/` if `~/.librewolf` exists), pointing to `/usr/lib/nomnom/nomnom-native-host`.
3. Install the signed add-on (or load `/usr/share/nomnom/firefox-extension/nomnom-0.1.1.zip` in Firefox Developer Edition / Nightly / LibreWolf via `about:debugging` for testing).

## 6. Uninstall

Remove the package:

```bash
sudo pacman -Rns nomnom
```

To also remove per-user desktop integrations, settings, logs and cached data:

```bash
rm -rf ~/.local/share/nomnom ~/.config/nomnom ~/.cache/nomnom \
  ~/.config/autostart/nomnom.desktop \
  ~/.mozilla/native-messaging-hosts/com.nomnom.host.json \
  ~/.librewolf/native-messaging-hosts/com.nomnom.host.json \
  ~/.local/share/kio/servicemenus/nomnom-*.desktop \
  ~/.local/share/nautilus/scripts/NomNom \
  ~/.local/share/nemo/actions/nomnom-*.nemo_action
```

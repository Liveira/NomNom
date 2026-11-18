#!/usr/bin/env bash
# Builds NomNom on Linux: the desktop app, Firefox native host, Firefox extension,
# and native packages for Arch Linux (.pkg.tar.zst), Debian/Ubuntu (.deb), or AppImage.
#
# Examples:
#   ./scripts/build-linux.sh                 # auto: .pkg.tar.zst on Arch, .deb on Debian/Ubuntu
#   ./scripts/build-linux.sh --arch          # build Arch Linux package via makepkg
#   ./scripts/build-linux.sh --deb           # build Debian/Ubuntu .deb package
#   ./scripts/build-linux.sh --appimage      # build portable .AppImage bundle
#   ./scripts/build-linux.sh --all           # build both .deb and .AppImage via Tauri
#   ./scripts/build-linux.sh --skip-bundle   # build release binaries + extension only

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export PATH="$HOME/.cargo/bin:$PATH"

MODE=""
RUN_CHECKS=0

usage() {
  cat <<'EOF'
Usage: ./scripts/build-linux.sh [options]

Options:
  --arch          Build an Arch Linux package (.pkg.tar.zst) using makepkg
  --deb           Build a Debian/Ubuntu package (.deb)
  --appimage      Build a portable AppImage (.AppImage)
  --all           Build both .deb and .AppImage bundles
  --skip-bundle   Compile release binaries and Firefox extension without packaging
  --check         Run tests and type checks before building
  -h, --help      Show this help text
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --arch) MODE="arch" ;;
    --deb) MODE="deb" ;;
    --appimage) MODE="appimage" ;;
    --all) MODE="all" ;;
    --skip-bundle) MODE="skip" ;;
    --check) RUN_CHECKS=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 1 ;;
  esac
  shift
done

if [[ -z "$MODE" ]]; then
  if [[ -f /etc/arch-release ]] && command -v makepkg >/dev/null 2>&1; then
    MODE="arch"
  elif command -v dpkg-deb >/dev/null 2>&1; then
    MODE="deb"
  else
    MODE="skip"
  fi
fi

step() {
  printf '\n== %s\n' "$1"
}

print_install_hint() {
  if [[ -f /etc/arch-release ]]; then
    echo "Install build dependencies on Arch Linux:" >&2
    echo "  sudo pacman -S --needed base-devel rust nodejs npm pkgconf webkit2gtk-4.1 gtk3 libayatana-appindicator hicolor-icon-theme" >&2
  else
    echo "Install build dependencies on Debian / Ubuntu:" >&2
    echo "  sudo apt update && sudo apt install -y build-essential pkg-config curl file patchelf libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev" >&2
  fi
}

check_prereqs() {
  local missing=()
  for cmd in cargo rustc node npm pkg-config; do
    command -v "$cmd" >/dev/null 2>&1 || missing+=("$cmd")
  done
  if [[ ${#missing[@]} -gt 0 ]]; then
    echo "Missing required build tools: ${missing[*]}" >&2
    print_install_hint
    exit 1
  fi
  if ! pkg-config --exists webkit2gtk-4.1 gtk+-3.0; then
    echo "Missing system development headers for webkit2gtk-4.1 or gtk+-3.0." >&2
    print_install_hint
    exit 1
  fi
}

if [[ "$MODE" == "arch" ]]; then
  if ! command -v makepkg >/dev/null 2>&1; then
    echo "makepkg was not found. Run this mode on Arch Linux or pass --deb / --appimage / --skip-bundle." >&2
    exit 1
  fi
  check_prereqs
  step "Arch Linux package (makepkg)"
  MAKEPKG_FLAGS=(-sf --noconfirm)
  if [[ "$RUN_CHECKS" -eq 0 ]]; then
    MAKEPKG_FLAGS+=(--nocheck)
  fi
  (
    cd "$ROOT/packaging/arch"
    makepkg "${MAKEPKG_FLAGS[@]}"
  )
  mkdir -p "$ROOT/dist/linux"
  cp -f "$ROOT"/packaging/arch/nomnom-*.pkg.tar.* "$ROOT/dist/linux/"
  step "Done"
  ls -lh "$ROOT"/dist/linux/nomnom-*.pkg.tar.*
  echo ""
  echo "Install with:"
  echo "  sudo pacman -U dist/linux/$(basename "$(ls -1 "$ROOT"/dist/linux/nomnom-*.pkg.tar.* | tail -n 1)")"
  exit 0
fi

check_prereqs

if [[ ! -d "$ROOT/node_modules" ]]; then
  step "Node dependencies"
  npm ci
fi

if [[ "$RUN_CHECKS" -eq 1 ]]; then
  step "Checks and tests"
  npm run check
  npm test
  npm test -w @nomnom/firefox-extension
  cargo test --workspace
fi

step "Firefox extension"
npm run package -w @nomnom/firefox-extension

step "Native messaging host"
cargo build --release -p nomnom-native-host

step "Desktop app ($MODE)"
(
  cd "$ROOT/apps/desktop"
  case "$MODE" in
    skip)
      npx tauri build --no-bundle --config src-tauri/tauri.linux.conf.json
      ;;
    deb)
      npx tauri build --bundles deb --config src-tauri/tauri.linux.conf.json
      ;;
    appimage)
      APPIMAGE_EXTRACT_AND_RUN=1 NO_STRIP=true npx tauri build --bundles appimage --config src-tauri/tauri.linux.conf.json
      ;;
    all)
      APPIMAGE_EXTRACT_AND_RUN=1 NO_STRIP=true npx tauri build --bundles deb,appimage --config src-tauri/tauri.linux.conf.json
      ;;
  esac
)

step "Done"
mkdir -p "$ROOT/dist/linux"
for f in \
  "$ROOT/target/release/bundle/deb/"*.deb \
  "$ROOT/target/release/bundle/appimage/"*.AppImage; do
  if [[ -f "$f" ]]; then
    cp -f "$f" "$ROOT/dist/linux/"
  fi
done

for bin in "$ROOT/target/release/nomnom" "$ROOT/target/release/nomnom-native-host"; do
  if [[ -f "$bin" ]]; then
    ls -lh "$bin"
  fi
done
shopt -s nullglob
for pkg in "$ROOT/dist/linux/"* "$ROOT/dist/extension/"*.zip; do
  ls -lh "$pkg"
done

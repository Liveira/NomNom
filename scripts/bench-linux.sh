#!/usr/bin/env bash
# Measures NomNom's memory, idle CPU and window ready time on Linux.
#
# Examples:
#   ./scripts/bench-linux.sh
#   ./scripts/bench-linux.sh --exe /usr/bin/nomnom --idle-seconds 30

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXE="$ROOT/target/release/nomnom"
IDLE_SECONDS=60
CPU_SECONDS=30

while [[ $# -gt 0 ]]; do
  case "$1" in
    --exe) EXE="$2"; shift ;;
    --idle-seconds) IDLE_SECONDS="$2"; shift ;;
    --cpu-seconds) CPU_SECONDS="$2"; shift ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
  shift
done

if [[ ! -x "$EXE" ]]; then
  echo "Executable not found: $EXE" >&2
  echo "Build first with: ./scripts/build-linux.sh --skip-bundle" >&2
  exit 1
fi

if pgrep -x nomnom >/dev/null 2>&1; then
  echo "NomNom is already running. Quit it from the tray icon first." >&2
  exit 1
fi

descendants() {
  local parent="$1"
  local child
  for child in $(pgrep -P "$parent" 2>/dev/null || true); do
    echo "$child"
    descendants "$child"
  done
}

private_mb() {
  local sum_kb=0
  local pid kb
  for pid in "$@"; do
    [[ -r "/proc/$pid/smaps_rollup" ]] || continue
    kb=$(awk '/^Private_(Clean|Dirty):/ { s += $2 } END { print s + 0 }' "/proc/$pid/smaps_rollup" 2>/dev/null || echo 0)
    sum_kb=$((sum_kb + kb))
  done
  awk -v kb="$sum_kb" 'BEGIN { printf "%.1f", kb / 1024.0 }'
}

cpu_ticks() {
  local pid="$1"
  [[ -r "/proc/$pid/stat" ]] || { echo 0; return; }
  # Fields 14 (utime) and 15 (stime) after the comm field in parentheses.
  awk '{ sub(/^.*\)[[:space:]]+/, ""); print $12 + $13 }' "/proc/$pid/stat"
}

DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/nomnom"
LOG_DIR="$DATA_DIR/logs"

"$EXE" --background >/dev/null 2>&1 &
APP_PID=$!

cleanup() {
  local pids
  pids="$(descendants "$APP_PID") $APP_PID"
  # shellcheck disable=SC2086
  kill -TERM $pids 2>/dev/null || true
}
trap cleanup EXIT

echo "Background mode: waiting ${IDLE_SECONDS}s..."
sleep "$IDLE_SECONDS"

BG_MB="$(private_mb "$APP_PID")"
mapfile -t BG_KIDS < <(descendants "$APP_PID")

CLK_TCK="$(getconf CLK_TCK)"
T0="$(cpu_ticks "$APP_PID")"
sleep "$CPU_SECONDS"
T1="$(cpu_ticks "$APP_PID")"
IDLE_CPU="$(awk -v d="$((T1 - T0))" -v hz="$CLK_TCK" -v s="$CPU_SECONDS" 'BEGIN { printf "%.3f", (d / hz) / s * 100.0 }')"

echo "Opening the window..."
"$EXE" >/dev/null 2>&1 || true
sleep 12

WIN_MB="$(private_mb "$APP_PID")"
mapfile -t WIN_KIDS < <(descendants "$APP_PID")
if [[ ${#WIN_KIDS[@]} -gt 0 ]]; then
  WEBKIT_MB="$(private_mb "${WIN_KIDS[@]}")"
else
  WEBKIT_MB="0.0"
fi

READY_MS="n/a"
LATEST_LOG="$(ls -1t "$LOG_DIR"/nomnom.*.log 2>/dev/null | head -n 1 || true)"
if [[ -n "$LATEST_LOG" ]]; then
  LINE="$(grep "main window ready" "$LATEST_LOG" | tail -n 1 || true)"
  if [[ -n "$LINE" ]]; then
    READY_MS="${LINE##*since_process_start_ms=}"
  fi
fi

printf "%-62s %s\n" "Background: nomnom private MB" "$BG_MB"
printf "%-62s %s\n" "Background: WebKitGTK child processes" "${#BG_KIDS[@]}"
printf "%-62s %s\n" "Idle CPU over ${CPU_SECONDS}s (% of one core)" "$IDLE_CPU"
printf "%-62s %s\n" "Window open: nomnom private MB" "$WIN_MB"
printf "%-62s %s\n" "Window open: WebKitGTK private MB (all child processes)" "$WEBKIT_MB"
printf "%-62s %s\n" "Window open: WebKitGTK child processes" "${#WIN_KIDS[@]}"
printf "%-62s %s\n" "Window ready (ms since the process started)" "$READY_MS"

#!/usr/bin/env bash
# Toggles fullscreen four times on a frontend running on the real GPU, without a monitor, and
# fails if the game loop stops (spike must-item 2; ADR 0001). A headless Weston provides an
# Xwayland display with DRI3, so the GPU driver presents for real; Xvfb cannot show the bug that
# froze front-rust. The toggle is sent through the window manager (wmctrl), which resizes the
# window under a running renderer exactly as Alt+Enter does.
#
#   scripts/fullscreen-check.sh <binary> <out-dir>
#
# Needs `scripts/install-linux-deps.sh --local` and a running PipeWire or PulseAudio server.
set -euo pipefail

usage="usage: $0 <binary> <out-dir>"
binary=$(realpath "${1:?$usage}")
out=${2:?$usage}
mkdir -p "$out"

socket=wl-deadrally-check
weston --backend=headless --renderer=gl --xwayland --width=1920 --height=1080 \
    --socket="$socket" --log="$out/weston.log" > /dev/null 2>&1 &
weston_pid=$!
sink=$(pactl load-module module-null-sink sink_name=deadrally_check)
alsa_conf=$(mktemp)
printf '%s\n' '</usr/share/alsa/alsa.conf>' \
    'pcm.!default { type pulse device deadrally_check }' \
    'ctl.!default { type pulse }' > "$alsa_conf"
app_pid=
cleanup() {
    if [ -n "$app_pid" ]; then kill "$app_pid" 2>/dev/null || true; fi
    kill "$weston_pid" 2>/dev/null || true
    pactl unload-module "$sink" || true
    rm -f "$alsa_conf"
}
trap cleanup EXIT

for _ in $(seq 50); do
    display=$(grep -o 'xserver listening on display :[0-9]*' "$out/weston.log" 2>/dev/null | grep -o ':[0-9]*$' || true)
    [ -n "$display" ] && break
    sleep 0.2
done
if [ -z "$display" ]; then
    echo "error: Weston did not start Xwayland; see $out/weston.log" >&2
    exit 1
fi
export DISPLAY=$display

SDL_AUDIO_DRIVER=pulseaudio PULSE_SINK=deadrally_check ALSA_CONFIG_PATH="$alsa_conf" \
    "$binary" > "$out/stats.log" 2> "$out/stderr.log" &
app_pid=$!
window=$(timeout 20 xdotool search --sync --name '^DR$' | head -n 1)
lines() { grep -c '^t=' "$out/stats.log" || true; }
size() { xdotool getwindowgeometry "$window" | awk '/Geometry/ {print $2}'; }

sleep 3
# SDL replaces its first window while it sets up the renderer; use the one that stayed.
window=$(xdotool search --name '^DR$' | tail -n 1)
start=$(size)
if [ -z "$start" ]; then
    echo "FAIL: no game window; see $out/stderr.log" >&2
    exit 1
fi
echo "start: $start"
for toggle in 1 2 3 4; do
    wmctrl -i -r "$window" -b toggle,fullscreen
    sleep 3
    before=$(lines)
    sleep 2
    after=$(lines)
    now=$(size)
    echo "toggle $toggle: $now, stats lines $before -> $after"
    if [ -z "$now" ]; then
        echo "FAIL: the game window disappeared after toggle $toggle" >&2
        exit 1
    fi
    if [ "$after" = "$before" ]; then
        echo "FAIL: the game loop stopped after toggle $toggle" >&2
        exit 1
    fi
done
echo "PASS: four fullscreen toggles, the game loop kept running"

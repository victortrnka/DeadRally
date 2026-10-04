#!/usr/bin/env bash
# Platform spike helper (spec section 7). Runs a frontend in a virtual X server with a virtual
# sound card, so the automated checks need neither a monitor nor speakers.
#
# It runs the game's M0 test scene (`-testscene`), so it needs no game data.
#
#   scripts/spike-check.sh screens <binary> <out-dir> [soak-seconds]
#       Windowed at 1280x720 and 1024x768: a screenshot of every test-scene mode and of F12
#       smoothing, then a soak run (default 300 s). stats.log gets one cumulative line per second.
#   scripts/spike-check.sh perf <binary> <out-dir>
#       Window resized to 3840x2160, bilinear on, vsync off, 30 s. The last stats line holds the
#       present times.
#
# Needs `scripts/install-linux-deps.sh --local` and a running PipeWire or PulseAudio server.
# Fullscreen and Alt+Enter need a window manager, so they are checked by hand, not here.
set -euo pipefail

usage="usage: $0 screens|perf <binary> <out-dir> [soak-seconds]"
mode=${1:?$usage}
binary=$(realpath "${2:?$usage}")
out=${3:?$usage}
soak=${4:-300}
mkdir -p "$out"

export DISPLAY=:99
Xvfb "$DISPLAY" -screen 0 3840x2160x24 -nolisten tcp &
xvfb_pid=$!
sink=$(pactl load-module module-null-sink sink_name=deadrally_null)
alsa_conf=$(mktemp)
printf '%s\n' '</usr/share/alsa/alsa.conf>' \
    'pcm.!default { type pulse device deadrally_null }' \
    'ctl.!default { type pulse }' > "$alsa_conf"
app_pid=
cleanup() {
    if [ -n "$app_pid" ]; then kill "$app_pid" 2>/dev/null || true; fi
    kill "$xvfb_pid" 2>/dev/null || true
    pactl unload-module "$sink" || true
    rm -f "$alsa_conf"
}
trap cleanup EXIT
sleep 2

launch() {
    # SDL plays through PulseAudio (PULSE_SINK); cpal plays through ALSA (ALSA_CONFIG_PATH).
    SDL_AUDIO_DRIVER=pulseaudio PULSE_SINK=deadrally_null ALSA_CONFIG_PATH="$alsa_conf" \
        "$binary" -window -testscene "$@" > "$out/stats.log" 2> "$out/stderr.log" &
    app_pid=$!
    timeout 20 xdotool search --sync --name '^DR$' > /dev/null || true
    sleep 1
    # SDL replaces its first window while it sets up the renderer; use the one that stayed.
    window=$(xdotool search --name '^DR$' | tail -n 1 || true)
    if [ -z "$window" ]; then
        echo "FAIL: no game window; see $out/stderr.log" >&2
        exit 1
    fi
    xdotool windowfocus --sync "$window"
}
resize() { xdotool windowsize --sync "$window" "$1" "$2"; sleep 1; }
shot() { import -window "$window" "$out/$1.png"; }
press() { xdotool key "$@"; sleep 1; }

case "$mode" in
    screens)
        launch
        resize 1280 720
        shot 1280x720-640x480-nearest
        press Tab
        shot 1280x720-320x200-nearest
        press Tab
        shot 1280x720-640x360-nearest
        resize 1024 768
        shot 1024x768-640x360-nearest
        press Tab F12
        shot 1024x768-640x480-bilinear
        press F12
        sleep "$soak"
        ;;
    perf)
        launch -novsync
        resize 3840 2160
        press F12
        sleep 30
        ;;
    *)
        echo "$usage" >&2
        exit 1
        ;;
esac

if ! kill -0 "$app_pid" 2>/dev/null; then
    echo "error: the frontend exited early; see $out/stderr.log" >&2
    exit 1
fi
tail -n 1 "$out/stats.log"

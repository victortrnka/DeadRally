#!/usr/bin/env bash
# Platform spike helper (spec section 7). Runs a frontend in a virtual X server and writes its
# sound to a file, so the automated checks need neither a monitor nor speakers.
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
# Needs `scripts/install-linux-deps.sh --local`. The sound goes to <out-dir>/audio.raw (SDL's
# disk driver: 48 kHz, 16-bit stereo); that driver's clock is not a sound card's, so the
# underruns in stats.log say nothing about the frontend.
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
alsa_conf=$(mktemp)
printf '%s\n' 'pcm.!default { type null }' > "$alsa_conf"
app_pid=
cleanup() {
    if [ -n "$app_pid" ]; then kill "$app_pid" 2>/dev/null || true; fi
    kill "$xvfb_pid" 2>/dev/null || true
    rm -f "$alsa_conf"
}
trap cleanup EXIT
sleep 2

launch() {
    # SDL3 ignores PULSE_SINK, so a null sink would not keep the sound off the speakers. The
    # disk driver writes it to a file, and the sound servers and ALSA are made unreachable in
    # case SDL ever falls back to another driver.
    SDL_AUDIO_DRIVER=disk SDL_AUDIO_DISK_OUTPUT_FILE="$out/audio.raw" \
        PULSE_SERVER=unix:/nonexistent PIPEWIRE_REMOTE=/nonexistent ALSA_CONFIG_PATH="$alsa_conf" \
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

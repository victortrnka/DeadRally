#!/usr/bin/env bash
# Runs the original dr.exe under Wine on a virtual X display and takes screenshots, so DeadRally
# can be compared with it (spec M1a section 8, M1b section 4.5). The original is only a test
# tool here: nothing reaches a monitor or the speakers, and the game install is never written to.
#
#   scripts/reference-run.sh [--data DIR] [--sound] [--cfg FILE] [--seed N] [--save SLOT:FILE]
#                            SCENARIO OUT_DIR
#
# With --sound the original plays its sound into a PulseAudio null sink, which is recorded to
# OUT_DIR/sound.wav (44.1 kHz, 16-bit stereo) from before the game starts until the last
# scenario line; the run stops if the game's stream is not on that sink.
#
# With --cfg the original starts with FILE as its dr.cfg (volumes, keys, records); without it,
# it writes a fresh one with its defaults.
#
# With --seed the copy of dr.exe seeds its random numbers with N instead of the clock when the
# main menu starts (spec M3a section 3), so the drivers and the races it offers repeat run to run.
# After the run, the dr.cfg and the saved games DR.SG0..DR.SG7 it wrote are copied to OUT_DIR.
#
# With --save (repeatable) the original starts with FILE as its saved game DR.SG<SLOT>.
#
# SCENARIO is a text file of lines "at <ms> key <name>" (an xdotool key name, e.g. space) and
# "at <ms> shot <label>", in time order; times count from the moment the window appears, and
# '#' starts a comment. OUT_DIR gets <label>.png per shot and run.log. Keep it under captures/:
# screenshots of the original's art are never committed.
#
# Needs wine32, Xvfb, xdotool and ImageMagick, and for --sound pactl and parec
# (scripts/install-linux-deps.sh --local), and the Windows version's files, which
# `deadrally-headless check-data` must recognise.
set -euo pipefail

usage() {
    echo "usage: $0 [--data DIR] [--sound] [--cfg FILE] [--seed N] [--save SLOT:FILE] SCENARIO OUT_DIR" >&2
    exit 1
}

data_args=()
sound=false
cfg=
seed=
saves=()
while [[ "${1:-}" == --* ]]; do
    case "$1" in
        --data)
            [[ $# -ge 2 ]] || usage
            data_args=(--data "$2")
            shift 2
            ;;
        --sound)
            sound=true
            shift
            ;;
        --cfg)
            [[ $# -ge 2 ]] || usage
            cfg=$(realpath "$2")
            shift 2
            ;;
        --save)
            [[ $# -ge 2 && "$2" =~ ^[0-7]:. && -f "${2#*:}" ]] || usage
            saves+=("${2%%:*}:$(realpath "${2#*:}")")
            shift 2
            ;;
        --seed)
            [[ $# -ge 2 && "$2" =~ ^[0-9]+$ ]] || usage
            seed=$2
            shift 2
            ;;
        *) usage ;;
    esac
done
[[ $# -eq 2 ]] || usage
scenario=$1
out=$2
[[ -f "$scenario" ]] || { echo "error: no scenario file $scenario" >&2; exit 1; }
[[ -z "$cfg" || -f "$cfg" ]] || { echo "error: no dr.cfg file $cfg" >&2; exit 1; }

tools=(wine Xvfb xdotool xwd convert)
if $sound; then
    tools+=(pactl parec)
fi
for tool in "${tools[@]}"; do
    command -v "$tool" >/dev/null || {
        echo "error: $tool is missing; run scripts/install-linux-deps.sh --local" >&2
        exit 1
    }
done

repo=$(cd "$(dirname "$0")/.." && pwd)
cache="${XDG_CACHE_HOME:-$HOME/.cache}/deadrally/reference"
run="$cache/run"
prefix="$cache/wineprefix"
mkdir -p "$out" "$cache"
out=$(cd "$out" && pwd)
log="$out/run.log"
: >"$log"

# Only a known release is a trustworthy oracle.
report=$(cargo run --quiet --release --manifest-path "$repo/Cargo.toml" -p deadrally-headless -- \
    check-data "${data_args[@]}") || {
    echo "error: check-data did not recognise the game data; see above" >&2
    exit 1
}
dir=$(sed -n 's/^directory: //p' <<<"$report")
[[ -f "$dir/dr.exe" ]] || {
    echo "error: $dir has no dr.exe; the reference runner needs the Windows version" >&2
    exit 1
}
echo "data: $dir" >>"$log"
echo "dr.exe sha256: $(sha256sum "$dir/dr.exe" | cut -d' ' -f1)" >>"$log"

# The original writes dr.cfg next to itself, so it runs from a fresh copy every time.
rm -rf "$run"
mkdir -p "$run"
cp -a "$dir/." "$run/"
if [[ -n "$cfg" ]]; then
    cp "$cfg" "$run/dr.cfg"
    echo "dr.cfg: $cfg" >>"$log"
fi
for save in "${saves[@]}"; do
    cp "${save#*:}" "$run/DR.SG${save%%:*}"
    echo "saved game ${save%%:*}: ${save#*:}" >>"$log"
done
if [[ -n "$seed" ]]; then
    # mainMenu (0x43A020) calls SDL_GetTicks at 0x43A191 and passes the result to srand; the
    # call becomes "mov eax, N". The bytes are checked first, so another build is refused.
    python3 - "$run/dr.exe" "$seed" <<'PATCH'
import struct, sys
path, seed = sys.argv[1], int(sys.argv[2])
data = bytearray(open(path, "rb").read())
offset = 0x43A191 - 0x400000
if data[offset:offset + 5] != bytes.fromhex("e850560000"):
    sys.exit("error: dr.exe has no SDL_GetTicks call before srand at 0x43A191")
data[offset:offset + 5] = b"\xb8" + struct.pack("<I", seed & 0xFFFFFFFF)
open(path, "wb").write(data)
PATCH
    echo "seed: $seed" >>"$log"
fi

display_fd=$(mktemp)
Xvfb -displayfd 3 -screen 0 1024x768x24 -nolisten tcp 3>"$display_fd" 2>>"$log" &
xvfb=$!
sink_module=
recorder=
cleanup() {
    WINEPREFIX="$prefix" wineserver -k 2>/dev/null || true
    kill "$xvfb" 2>/dev/null || true
    if [[ -n "$recorder" ]]; then kill "$recorder" 2>/dev/null || true; fi
    if [[ -n "$sink_module" ]]; then pactl unload-module "$sink_module" 2>/dev/null || true; fi
    rm -f "$display_fd"
}
trap cleanup EXIT
for _ in $(seq 100); do
    [[ -s "$display_fd" ]] && break
    sleep 0.05
done
[[ -s "$display_fd" ]] || { echo "error: Xvfb did not start; see $log" >&2; exit 1; }
export DISPLAY=":$(head -n1 "$display_fd")"

# A 32-bit prefix, created once; Mono and Gecko are disabled so Wine asks no questions.
export WINEPREFIX="$prefix" WINEARCH=win32 WINEDLLOVERRIDES="mscoree,mshtml=" WINEDEBUG=-all
if [[ ! -f "$prefix/system.reg" ]]; then
    echo "creating the Wine prefix $prefix (once)" >&2
    wineboot --init >>"$log" 2>&1
    wineserver -w
fi

sound_args=(-nosound)
if $sound; then
    sink=deadrally_ref_$$
    sink_module=$(pactl load-module module-null-sink "sink_name=$sink" "sink_properties=device.description=$sink")
    parec --device="$sink.monitor" --file-format=wav --format=s16le --rate=44100 --channels=2 \
        "$out/sound.wav" 2>>"$log" &
    recorder=$!
    sound_args=()
    export PULSE_SINK=$sink
fi

(cd "$run" && exec wine dr.exe -window -nogl "${sound_args[@]}") >>"$log" 2>&1 &
game=$!
window=$(timeout 30 xdotool search --sync --onlyvisible --name '.' | head -n1) || {
    echo "error: the original did not open a window within 30 s; see $log" >&2
    exit 1
}
start=$(date +%s%N)
echo "window: $window $(xdotool getwindowgeometry "$window" | tr '\n' ' ')" >>"$log"

if $sound; then
    # The game's stream must be on the null sink; anywhere else it could reach the speakers.
    sink_index=$(pactl list short sinks | awk -v name="$sink" '$2 == name {print $1}')
    stream_sink=
    for _ in $(seq 50); do
        stream_sink=$(pactl list sink-inputs | awk '/^Sink Input/ {sink=""} /^\tSink:/ {sink=$2}
            /application.name = "dr.exe"/ {print sink; exit}')
        [[ -n "$stream_sink" ]] && break
        sleep 0.1
    done
    if [[ "$stream_sink" != "$sink_index" ]]; then
        echo "error: dr.exe plays to sink '$stream_sink', not the null sink $sink_index; stopped" >&2
        exit 1
    fi
    echo "sound: dr.exe plays to null sink $sink ($sink_index), recorded to sound.wav" >>"$log"
fi
xdotool windowfocus --sync "$window" 2>>"$log" || true

shots=()
previous_ms=0
while read -r at ms action arg rest; do
    [[ -z "${at:-}" || "$at" == \#* ]] && continue
    [[ "$at" == at && "$ms" =~ ^[0-9]+$ && -n "${arg:-}" && -z "${rest:-}" ]] || {
        echo "error: bad scenario line: $at $ms $action $arg $rest" >&2
        exit 1
    }
    # Lines run one after another, so a line out of order would run late and its shot would
    # carry a wrong time.
    (( ms >= previous_ms )) || { echo "error: scenario line at $ms ms comes after $previous_ms ms" >&2; exit 1; }
    previous_ms=$ms
    now_ms=$(( ($(date +%s%N) - start) / 1000000 ))
    if (( ms > now_ms )); then
        sleep "$(printf '%d.%03d' $(( (ms - now_ms) / 1000 )) $(( (ms - now_ms) % 1000 )))"
    fi
    kill -0 "$game" 2>/dev/null || { echo "error: the original exited early; see $log" >&2; exit 1; }
    taken_ms=$(( ($(date +%s%N) - start) / 1000000 ))
    case "$action" in
        key) xdotool key "$arg" ;;
        shot)
            xwd -silent -id "$window" -out "$out/$arg.xwd"
            shots+=("$arg")
            ;;
        *) echo "error: unknown action $action" >&2; exit 1 ;;
    esac
    echo "$taken_ms ms: $action $arg (planned $ms)" >>"$log"
done <"$scenario"

if $sound; then
    # SIGINT lets parec finish the WAV header.
    kill -INT "$recorder"
    wait "$recorder" || true
    recorder=
fi

for file in "$run"/dr.cfg "$run"/DR.SG[0-7]; do
    if [[ -f "$file" ]]; then cp "$file" "$out/"; fi
done

for label in "${shots[@]}"; do
    convert "$out/$label.xwd" "$out/$label.png"
    rm "$out/$label.xwd"
done
echo "done: ${#shots[@]} shots in $out" >&2

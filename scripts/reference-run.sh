#!/usr/bin/env bash
# Runs the original dr.exe under Wine on a virtual X display and takes screenshots, so DeadRally
# can be compared with it (spec M1a section 8). The original is only a test tool here: nothing
# reaches a monitor or the speakers, and the game install is never written to.
#
#   scripts/reference-run.sh [--data DIR] SCENARIO OUT_DIR
#
# SCENARIO is a text file of lines "at <ms> key <name>" (an xdotool key name, e.g. space) and
# "at <ms> shot <label>", in time order; times count from the moment the window appears, and
# '#' starts a comment. OUT_DIR gets <label>.png per shot and run.log. Keep it under captures/:
# screenshots of the original's art are never committed.
#
# Needs wine32, Xvfb, xdotool and ImageMagick (scripts/install-linux-deps.sh --local) and the
# Windows version's files, which `deadrally-headless check-data` must recognise.
set -euo pipefail

usage() {
    echo "usage: $0 [--data DIR] SCENARIO OUT_DIR" >&2
    exit 1
}

data_args=()
if [[ "${1:-}" == --data ]]; then
    [[ $# -ge 2 ]] || usage
    data_args=(--data "$2")
    shift 2
fi
[[ $# -eq 2 ]] || usage
scenario=$1
out=$2
[[ -f "$scenario" ]] || { echo "error: no scenario file $scenario" >&2; exit 1; }

for tool in wine Xvfb xdotool xwd convert; do
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

display_fd=$(mktemp)
Xvfb -displayfd 3 -screen 0 1024x768x24 -nolisten tcp 3>"$display_fd" 2>>"$log" &
xvfb=$!
cleanup() {
    WINEPREFIX="$prefix" wineserver -k 2>/dev/null || true
    kill "$xvfb" 2>/dev/null || true
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

(cd "$run" && exec wine dr.exe -window -nogl -nosound) >>"$log" 2>&1 &
game=$!
window=$(timeout 30 xdotool search --sync --onlyvisible --name '.' | head -n1) || {
    echo "error: the original did not open a window within 30 s; see $log" >&2
    exit 1
}
start=$(date +%s%N)
echo "window: $window $(xdotool getwindowgeometry "$window" | tr '\n' ' ')" >>"$log"
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

for label in "${shots[@]}"; do
    convert "$out/$label.xwd" "$out/$label.png"
    rm "$out/$label.xwd"
done
echo "done: ${#shots[@]} shots in $out" >&2

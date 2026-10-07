#!/usr/bin/env bash
# Starts a packed program on its test scene for ten seconds (spec M7c) and fails unless it is
# still running and counting ticks. It needs no game data; the sound goes to SDL's dummy
# driver, so a machine without a sound card works too and nothing reaches the speakers.
# Linux needs a display: run it under `xvfb-run -a`.
#
#   scripts/smoke-test.sh PROGRAM
set -euo pipefail
program=${1:?usage: $0 PROGRAM}
log=$(mktemp)
SDL_AUDIO_DRIVER=dummy "$program" -window -testscene >"$log" 2>&1 &
pid=$!
sleep 10
if ! kill -0 "$pid" 2>/dev/null; then
    cat "$log"
    echo "error: $program stopped within ten seconds" >&2
    exit 1
fi
kill "$pid"
wait "$pid" 2>/dev/null || true
cat "$log"
if ! grep -q "ticks=[1-9]" "$log"; then
    echo "error: $program counted no ticks" >&2
    exit 1
fi
echo "ok: $program ran its test scene"

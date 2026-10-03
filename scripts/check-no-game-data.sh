#!/usr/bin/env bash
# Fails if any tracked file matches .gitignore: original game data (brief §2) or local output.
# A second line of defence, because `git add -f` bypasses .gitignore.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
offenders=$(git ls-files --cached --ignored --exclude-standard)
if [ -n "$offenders" ]; then
    echo "error: these tracked files match .gitignore and must not be committed:" >&2
    echo "$offenders" >&2
    exit 1
fi
echo "ok: no tracked file matches .gitignore"

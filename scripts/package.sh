#!/usr/bin/env bash
# Packs a release build of the game for players (spec M7c): one archive with the program, the
# README and the licence. The game data is never in it.
#
#   scripts/package.sh linux   VERSION OUT_DIR   # target/release/deadrally -> .tar.gz
#   scripts/package.sh windows VERSION OUT_DIR   # target/release/deadrally.exe -> .zip
#   scripts/package.sh macos   VERSION OUT_DIR   # the aarch64 and x86_64 builds -> a universal
#                                                # DeadRally.app, ad-hoc signed, in a .zip
#
# The builds must exist: `cargo build --release -p deadrally --locked`, and on macOS the same
# with `--target aarch64-apple-darwin` and `--target x86_64-apple-darwin`.
set -euo pipefail

usage="usage: $0 linux|windows|macos VERSION OUT_DIR"
system=${1:?$usage}
version=${2:?$usage}
out=${3:?$usage}
repo=$(cd "$(dirname "$0")/.." && pwd)
name="DeadRally-$version-$system"
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir -p "$out"
out=$(cd "$out" && pwd)
mkdir "$stage/$name"
cp "$repo/README.md" "$repo/LICENSE" "$stage/$name/"

case "$system" in
    linux)
        cp "$repo/target/release/deadrally" "$stage/$name/"
        tar -C "$stage" -czf "$out/$name.tar.gz" "$name"
        echo "$out/$name.tar.gz"
        ;;
    windows)
        cp "$repo/target/release/deadrally.exe" "$stage/$name/"
        (cd "$stage" && 7z a -tzip "$out/$name.zip" "$name" >/dev/null)
        echo "$out/$name.zip"
        ;;
    macos)
        app="$stage/$name/DeadRally.app"
        mkdir -p "$app/Contents/MacOS"
        lipo -create -output "$app/Contents/MacOS/deadrally" \
            "$repo/target/aarch64-apple-darwin/release/deadrally" \
            "$repo/target/x86_64-apple-darwin/release/deadrally"
        # A bundle's version is up to three numbers: v1.0.0-rc1 gives 1.0.0, a dev build 0.0.0.
        bundle_version=$(echo "${version#v}" | sed -nE 's/^([0-9]+(\.[0-9]+){0,2}).*/\1/p')
        bundle_version=${bundle_version:-0.0.0}
        cat >"$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key><string>deadrally</string>
    <key>CFBundleIdentifier</key><string>io.github.victortrnka.deadrally</string>
    <key>CFBundleName</key><string>DeadRally</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$bundle_version</string>
    <key>CFBundleVersion</key><string>$bundle_version</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF
        # Unsigned apps do not start on Apple Silicon; an ad-hoc signature lets the player
        # open it once Gatekeeper is told to (README).
        codesign --force --deep --sign - "$app"
        (cd "$stage" && ditto -c -k --keepParent "$name" "$out/$name.zip")
        echo "$out/$name.zip"
        ;;
    *)
        echo "$usage" >&2
        exit 1
        ;;
esac

#!/usr/bin/env bash
# Installs the system packages DeadRally needs on Debian, Ubuntu or Mint. CI and developers use
# this same list. SDL3 is compiled from source (cmake plus the X11/Wayland/audio headers); cpal
# links ALSA and gilrs links udev.
#
#   scripts/install-linux-deps.sh           build dependencies
#   scripts/install-linux-deps.sh --local   also the tools for checking frontends without a
#                                           monitor (Xvfb, screenshots, software Vulkan)
set -euo pipefail

packages=(
    build-essential cmake pkg-config
    libasound2-dev libpulse-dev libpipewire-0.3-dev libudev-dev libdbus-1-dev libibus-1.0-dev
    libx11-dev libxext-dev libxrandr-dev libxcursor-dev libxfixes-dev libxi-dev libxss-dev
    libxtst-dev libxkbcommon-dev libwayland-dev libdecor-0-dev
    libegl-dev libgl-dev libgles-dev libdrm-dev libgbm-dev
)
case "${1:-}" in
    "") ;;
    --local) packages+=(xvfb imagemagick xdotool x11-apps mesa-vulkan-drivers) ;;
    *) echo "usage: $0 [--local]" >&2; exit 1 ;;
esac

sudo apt-get update
sudo apt-get install -y --no-install-recommends "${packages[@]}"

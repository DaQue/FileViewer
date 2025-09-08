#!/usr/bin/env bash
set -euo pipefail
sudo apt update
sudo apt install -y \
  build-essential pkg-config \
  libx11-dev libxrandr-dev libxi-dev libxcursor-dev \
  libgl1-mesa-dev libegl1-mesa-dev \
  libxkbcommon-dev libwayland-dev \
  xdg-desktop-portal xdg-desktop-portal-gtk
echo "Done. Consider restarting your session if portals were newly installed."


#!/usr/bin/env bash
set -euo pipefail

# Capture named screenshots of the app window using gnome-screenshot (if available).
# Saves into assets/screenshots/ with the names referenced by README.

OUT_DIR="assets/screenshots"
mkdir -p "$OUT_DIR"

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing: $1"; exit 1; }; }

if ! command -v gnome-screenshot >/dev/null 2>&1; then
  echo "gnome-screenshot not found. Install it or use another tool (Spectacle, Flameshot)."
  exit 1
fi

echo "When prompted, click the Gemini File Viewer window area to capture."
echo "Saving to $OUT_DIR"

echo "Capture toolbar (ui-toolbar.png)..."
gnome-screenshot -a -f "$OUT_DIR/ui-toolbar.png"

echo "Capture search (ui-search.png)..."
gnome-screenshot -a -f "$OUT_DIR/ui-search.png"

echo "Capture image viewer (ui-image-view.png)..."
gnome-screenshot -a -f "$OUT_DIR/ui-image-view.png"

echo "Done. Add them with: git add assets/screenshots/*.png"


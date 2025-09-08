#!/usr/bin/env bash
set -euo pipefail

# Record a short demo and convert to GIF with good palette.
# Requires ffmpeg. X11 example. For Wayland, use wf-recorder/OBS then convert.

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "ffmpeg not found. Install ffmpeg to record/convert."
  exit 1
fi

REGION="${1:-1024x640+100,100}"
DURATION="${2:-8}"
TMP_MP4="${3:-/tmp/gfv-demo.mp4}"
OUT_GIF="${4:-assets/screenshots/demo.gif}"

mkdir -p "$(dirname "$OUT_GIF")"

echo "Recording region $REGION for ${DURATION}s to $TMP_MP4 (X11 display :0.0)"
ffmpeg -y -f x11grab -s "${REGION%%+*}" -i ":0.0+${REGION#*+}" -r 30 -t "$DURATION" "$TMP_MP4"

echo "Converting MP4 to optimized GIF at $OUT_GIF"
PALETTE=/tmp/gfv-palette.png
ffmpeg -y -i "$TMP_MP4" -vf "fps=15,scale=1024:-1:flags=lanczos,palettegen" "$PALETTE"
ffmpeg -y -i "$TMP_MP4" -i "$PALETTE" -lavfi "fps=15,scale=1024:-1:flags=lanczos [x]; [x][1:v] paletteuse" "$OUT_GIF"

echo "Done. Add with: git add $OUT_GIF"


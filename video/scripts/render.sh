#!/usr/bin/env bash
# Renders both deliverables from the Remotion project.
#   npm run audio   -> regenerate the score (done once, output committed)
#   bash scripts/render.sh
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p out
npx remotion render src/index.ts GeoDeck-Launch-Trailer "out/GeoDeck — Official Launch Trailer.mp4" \
  --codec=h264 --crf=16 --pixel-format=yuv420p --fps=60 --concurrency=4 --muted=false
npx remotion render src/index.ts GeoDeck-Vertical-Trailer "out/GeoDeck — Vertical Launch Trailer.mp4" \
  --codec=h264 --crf=16 --pixel-format=yuv420p --fps=60 --concurrency=4 --muted=false

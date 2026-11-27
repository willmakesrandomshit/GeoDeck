#!/usr/bin/env bash
# Renders both deliverables from the Remotion project, then normalises the
# encodes to limited-range yuv420p (Remotion's output is full-range yuvj420p).
#   npm run audio          -> regenerate the score (public/audio/*.wav)
#   bash scripts/make-narration.sh && bash scripts/make-mix.sh  -> voice-over + final mix
#   bash scripts/render.sh
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p out
LAND="out/GeoDeck — Official Launch Trailer.mp4"
VERT="out/GeoDeck — Vertical Launch Trailer.mp4"
npx remotion render src/index.ts GeoDeck-Launch-Trailer "$LAND" \
  --codec=h264 --crf=16 --pixel-format=yuv420p --fps=60 --concurrency=4
npx remotion render src/index.ts GeoDeck-Vertical-Trailer "$VERT" \
  --codec=h264 --crf=16 --pixel-format=yuv420p --fps=60 --concurrency=4
for f in "$LAND" "$VERT"; do
  ffmpeg -v error -y -i "$f" -c:v libx264 -preset slow -crf 16 -pix_fmt yuv420p \
    -color_range tv -colorspace bt709 -color_primaries bt709 -color_trc bt709 \
    -movflags +faststart -c:a aac -b:a 320k -ar 48000 -ac 2 "$f.tmp.mp4"
  mv "$f.tmp.mp4" "$f"
done

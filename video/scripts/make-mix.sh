#!/usr/bin/env bash
# Final trailer mix.
#   music bed  : Kevin MacLeod "Rynos Theme" (CC BY 4.0), first 60 s, faded
#   sfx        : whooshes, ticks and impact from scripts/make-audio.mjs
#   narration  : Piper neural voice-over from scripts/make-narration.sh
# The music bed ducks under the narration; SFX stay on top. Output is
# normalised to -14 LUFS with a true-peak limiter at -1.5 dBTP.
set -euo pipefail
cd "$(dirname "$0")/.."
# Music bed: first 60 s of the Kevin MacLeod track, loudness-matched.
ffmpeg -v error -y -ss 0 -t 60 -i "public/music/Rynos Theme.mp3" -af "loudnorm=I=-20:TP=-2:LRA=11" \
  -ar 48000 -ac 2 -c:a pcm_s16le public/audio/music-bed.wav
ffmpeg -v error -y -i public/audio/music-bed.wav -i public/audio/trailer-sfx.wav -i public/audio/vo/narration.wav \
  -filter_complex "[0:a]afade=t=in:st=0:d=0.8,afade=t=out:st=57.5:d=2.5,asplit=1[mus];[mus]asplit=1[mus2];[2:a]asplit=2[vk][vm];[mus2][vk]sidechaincompress=threshold=0.02:ratio=6:attack=8:release=380[duck];[duck][vm][1:a]amix=inputs=3:weights=1 1.3 0.9:normalize=0:duration=longest[pre];[pre]loudnorm=I=-14:TP=-1.5:LRA=9,alimiter=limit=0.84:level=0[out]" \
  -map "[out]" -t 60 -c:a pcm_s16le -ar 48000 -ac 2 public/audio/trailer-final.wav
ffmpeg -hide_banner -i public/audio/trailer-final.wav -af ebur128=peak=true -f null - 2>&1 | grep -E "I:|Peak:" | tail -2

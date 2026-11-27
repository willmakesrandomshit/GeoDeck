#!/usr/bin/env bash
# Builds the voice-over track: each line is synthesised with the Piper neural
# voice, then placed at its cue time on a 60 s bed.
# Lines use only facts stated in the README or shown on screen.
set -euo pipefail
cd "$(dirname "$0")/.."
VO=public/audio/vo
mkdir -p "$VO"
MODEL=public/vo-model/en_US-lessac-high.onnx
# cue (seconds) | line
CUES=(
"0.7|Managing Geometry Dash just changed."
"5.3|Meet GeoDeck. One application. Everything Geometry Dash."
"10.5|Your game. Your versions."
"12.8|Steam detection and isolated version slots."
"17.4|Discover. Manage. Customise."
"20.6|Browse the Geode index and share portable modpacks."
"24.4|Your soundtrack. Your way."
"26.9|Replace any track. The original is always kept."
"31.4|Protect your progress."
"33.4|Automatic backups. AES two fifty six encryption."
"38.4|Diagnose. Recover. Stay current."
"41.8|Health checks and crash logs."
"45.4|Built in Rust."
"48.7|Built for Geometry Dash."
"52.9|GeoDeck. Alpha one point oh. Out now on GitHub."
"57.3|Early alpha. Back up your saves."
)
inputs=()
filter=""
i=0
for cue in "${CUES[@]}"; do
  t="${cue%%|*}"; line="${cue#*|}"
  wav="$VO/line-$(printf %02d $i).wav"
  # Neural voice (Piper, en_US lessac high). See ASSETS.md for voice licence status.
  printf '%s\n' "$line" | python3 -m piper -m "$MODEL" -f "$wav" >/dev/null 2>&1
  ms=$(python3 -c "import sys;print(int(float(sys.argv[1])*1000))" "$t")
  dur=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$wav")
  echo "cue ${t}s len ${dur}s  $line"
  if [ -n "${prev_end:-}" ]; then awk -v a="$prev_end" -v b="$t" 'BEGIN{exit !(a>b)}' && { echo "OVERLAP before cue $t"; exit 1; } || true; fi
  prev_end=$(awk -v a="$t" -v d="$dur" 'BEGIN{printf "%.3f", a+d}')
  inputs+=(-i "$wav")
  filter+="[$((i+1)):a]adelay=${ms}|${ms},volume=1.0[v$i];"
  i=$((i+1))
done
mix=""
for j in $(seq 0 $((i-1))); do mix+="[v$j]"; done
ffmpeg -v error -y -f lavfi -t 60 -i anullsrc=r=48000:cl=stereo "${inputs[@]}" \
  -filter_complex "${filter}[0:a]${mix}amix=inputs=$((i+1)):normalize=0,atrim=0:60,aformat=sample_fmts=s16:sample_rates=48000:channel_layouts=stereo[vo]" \
  -map "[vo]" -c:a pcm_s16le "$VO/narration.wav"
echo "wrote $VO/narration.wav"

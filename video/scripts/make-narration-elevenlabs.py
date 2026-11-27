"""Generates the trailer voice-over with ElevenLabs (voice: George, premade).

Reads the same cue list as make-narration.sh (scripts/cues.txt), writes one WAV
per line to public/audio/vo-el/, then builds public/audio/vo/narration.wav with
each line placed at its cue time. The API key is injected by the session's
network proxy; set ELEVENLABS_API_KEY only if you run this outside that proxy.
"""
import json, os, subprocess, sys, urllib.request, pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
VOICE_ID = "JBFqnCBsd6RMkjVDRZzb"  # George - Warm, Captivating Storyteller
MODEL = "eleven_multilingual_v2"
OUT = ROOT / "public/audio/vo-el"
OUT.mkdir(parents=True, exist_ok=True)

cues = []
for raw in (ROOT / "scripts/cues.txt").read_text().splitlines():
    if raw.strip():
        t, line = raw.split("|", 1)
        cues.append((float(t), line))

headers = {"Content-Type": "application/json", "Accept": "audio/mpeg"}
if os.environ.get("ELEVENLABS_API_KEY"):
    headers["xi-api-key"] = os.environ["ELEVENLABS_API_KEY"]

wavs = []
for i, (t, line) in enumerate(cues):
    mp3 = OUT / f"line-{i:02d}.mp3"
    wav = OUT / f"line-{i:02d}.wav"
    body = json.dumps({
        "text": line,
        "model_id": MODEL,
        "voice_settings": {"stability": 0.5, "similarity_boost": 0.8, "style": 0.25, "use_speaker_boost": True},
    }).encode()
    req = urllib.request.Request(
        f"https://api.elevenlabs.io/v1/text-to-speech/{VOICE_ID}?output_format=mp3_44100_128",
        data=body, headers=headers, method="POST")
    with urllib.request.urlopen(req, timeout=120) as r:
        mp3.write_bytes(r.read())
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", str(mp3), "-ar", "48000", "-ac", "2", str(wav)], check=True)
    dur = float(subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(wav)],
                               capture_output=True, text=True, check=True).stdout)
    print(f"cue {t:5.1f}s len {dur:5.2f}s  {line}")
    wavs.append((t, dur, wav))

# Overlap check: each line must end before the next cue starts.
for (t, d, _), (t2, _, _) in zip(wavs, wavs[1:]):
    if t + d > t2:
        sys.exit(f"OVERLAP: line at {t}s ends {t + d:.2f}s, next cue {t2}s")

inputs, parts = ["-f", "lavfi", "-t", "60", "-i", "anullsrc=r=48000:cl=stereo"], []
for i, (t, _, wav) in enumerate(wavs):
    inputs += ["-i", str(wav)]
    ms = int(t * 1000)
    parts.append(f"[{i+1}:a]adelay={ms}|{ms}[v{i}]")
mix = "".join(f"[v{i}]" for i in range(len(wavs)))
flt = ";".join(parts) + f";[0:a]{mix}amix=inputs={len(wavs)+1}:normalize=0,atrim=0:60[vo]"
(ROOT / "public/audio/vo").mkdir(parents=True, exist_ok=True)
subprocess.run(["ffmpeg", "-v", "error", "-y", *inputs, "-filter_complex", flt, "-map", "[vo]",
                "-c:a", "pcm_s16le", str(ROOT / "public/audio/vo/narration.wav")], check=True)
print("wrote public/audio/vo/narration.wav")

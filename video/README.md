# GeoDeck Alpha 1.0.0: Launch Trailer (Remotion project)

This folder is a self-contained Remotion project. It is kept separate from the
application source (`../src`, `../Cargo.toml`); nothing here changes the app.

## Deliverables (written to `out/`)

| File | Spec |
| --- | --- |
| `GeoDeck — Official Launch Trailer.mp4` | 1920×1080, 60 fps, 60 s, H.264 (yuv420p, CRF 16), AAC stereo audio |
| `GeoDeck — Vertical Launch Trailer.mp4` | 1080×1920, 60 fps, 60 s, H.264 (yuv420p, CRF 16), AAC stereo audio. Recomposed for TikTok and Shorts, not cropped. |
| `GeoDeck — Thumbnail.png` | 1920×1080 still |

See `VERIFICATION.md` for the measured results.

## Requirements

- Node.js 18+ and npm
- FFmpeg / FFprobe 5+ on PATH
- A Chromium build for Remotion. Set `REMOTION_BROWSER=/path/to/chrome` to use
  one; `remotion.config.ts` defaults to the path used in the build sandbox.

## Pipeline

1. **Footage:** `bash scripts/capture-shots.sh`. Builds a demo environment
   (`scripts/make-demo-fixture.py`), makes three vault snapshots through the real
   app, and captures the screens into `public/shots/`. Needs the release build
   (`cargo build --release` in the repo root), Xvfb, xdotool, ImageMagick and root
   (it writes `/home/deck` and briefly edits `/etc/hosts`). Details in `ASSETS.md`.
2. **Narration:** `python3 scripts/make-narration-elevenlabs.py`. Reads the timed lines in
   `scripts/cues.txt` and calls the ElevenLabs API (voice "George"). The API key is
   injected by the sandbox proxy; elsewhere set `ELEVENLABS_API_KEY`.
   (`scripts/make-narration.sh` is the older Piper version.)
3. **Music + mix:** the bed is Kevin MacLeod's "Rynos Theme" (`public/music/`, CC BY 4.0).
   `node scripts/make-audio.mjs` regenerates the sound effects, then
   `bash scripts/make-mix.sh` trims the bed to 60 s, ducks it under the voice and
   normalises to -14 LUFS into `public/audio/trailer-final.wav`.
4. **Render:** `bash scripts/render.sh` renders both MP4s and re-encodes them to
   limited-range yuv420p.
5. **Verify:** see `VERIFICATION.md`.

Live preview: `npm run studio`.
Single frame: `npx remotion still src/index.ts GeoDeck-Launch-Trailer --frame=600 --output=out/preview.png`

## Requirements

- Node.js 18+ and npm (`npm install`), FFmpeg/FFprobe 5+
- A Chromium build for Remotion: set `REMOTION_BROWSER=/path/to/chrome`
  (`remotion.config.ts` defaults to the path used in the build sandbox)

## Editing

- `src/timeline.ts`: every scene boundary, in seconds. Scenes are laid out
  relative to these values, so retiming a scene is a one-line change.
- `src/theme.ts`: colours and fonts, copied from the application's `src/ui/theme.rs`.
- `src/scenes/Hook.tsx`, `src/scenes/Features.tsx`: scene copy and camera moves.
  Camera moves are focal-point keyframes over the real screenshots.
- `src/components/`: reusable pieces (`Headline`, `AppWindow`, `Logo`, `Backdrop`).
- `scripts/make-audio.mjs`: the score and sound design. Event times are the
  same beats the scene cuts use (120 BPM, 0.5 s per beat).
- `public/shots/`: screenshots of the application, captured from the built app
  (see `ASSETS.md`).

All animation is driven by `useCurrentFrame()` and `spring()`, so every frame
is deterministic.

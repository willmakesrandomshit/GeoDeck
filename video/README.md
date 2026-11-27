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

## Render

```bash
cd video
npm install
npm run audio          # optional: regenerate the score (public/audio/*.wav)
bash scripts/render.sh # both MP4s into out/
npm run verify         # ffprobe checks + frame extraction
npm run studio         # live preview / editing
```

Single-frame preview: `npx remotion still src/index.ts GeoDeck-Launch-Trailer --frame=600 --output=out/preview.png`

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

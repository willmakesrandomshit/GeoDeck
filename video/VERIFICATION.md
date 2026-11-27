# Verification results

Measured on the final MP4s in `out/` (git-ignored; rendered with `scripts/render.sh`).

## Technical checks (ffprobe / ffmpeg)

| Check | Landscape `GeoDeck — Official Launch Trailer.mp4` | Vertical `GeoDeck — Vertical Launch Trailer.mp4` |
| --- | --- | --- |
| Resolution | 1920×1080 | 1080×1920 |
| Frame rate | 60/1 fps | 60/1 fps |
| Frame count | 3600 | 3600 |
| Duration | 60.010 s | 60.010 s |
| Video codec | H.264, yuv420p, limited range (tv), BT.709 | same |
| Audio | AAC, 48 kHz, stereo | same |
| Loudness | -13.6 LUFS integrated, -1.4 dBFS true peak | same mix |
| Decode test (`ffmpeg -f null`) | passes, no errors | passes, no errors |
| File size | 34.1 MB | 23.4 MB |

Thumbnail: `out/GeoDeck — Thumbnail.png`, 1920×1080, a still at 7.8 s ("Meet GeoDeck").

## Visual inspection

Frames were pulled from the final files at 20 timestamps (landscape) and 11 (vertical), from 2 s to 58.5 s, and reviewed on contact sheets. Checked and OK:

- Hook run, headline and logo mark; Meet; Versions (with the "29 recognised builds" line); the three-tab Mods scene; the populated Songs and Vault screens; Diagnose → Recover → Stay current; the 14-screen montage with real feature labels; the finale with URL, alpha line and music credit.
- No clipped headlines, missing images or placeholder screens in either format.

## Problems found and fixed during production

- Settings screenshot carried debug-only red egui overlays (a debug build was used). Recaptured from the release build.
- Empty-state footage. Replaced with a labelled demo environment (see `ASSETS.md`).
- Sandbox error toast covering the bottom-right. Fixed by making the offline requests hang during capture rather than editing pixels.
- Vault headline wrapped into the bullet list, montage headline overlapped the window, full-range `yuvj420p` output. All fixed.
- Voice-over replaced twice: espeak (robotic) → Piper → ElevenLabs.

## Known limitations

- Footage is a demo environment, not a real player's machine; see `ASSETS.md`. Browse Geode, Demonlist data and update checks are not shown working (no internet at capture).
- Vertical cut uses tight crops, so the smallest UI text is still small on a phone.
- The end-card music credit and alpha disclaimer are small. The credit is also required in the video description.
- Frames were sampled, not every frame inspected; no human has reviewed pacing, music sync or narration delivery.
- ElevenLabs commercial-use terms depend on the account plan. Confirm before publishing.

# Verification results

Measured on the final MP4s in `out/` (re-encoded after the first render; see "Encoding" below).

## Technical checks (ffprobe / ffmpeg)

| Check | Landscape `GeoDeck — Official Launch Trailer.mp4` | Vertical `GeoDeck — Vertical Launch Trailer.mp4` |
| --- | --- | --- |
| Resolution | 1920×1080 | 1080×1920 |
| Frame rate | 60/1 fps | 60/1 fps |
| Frame count | 3600 | 3600 |
| Duration | 60.010 s | 60.010 s |
| Video codec | H.264, yuv420p, limited range (tv), BT.709 | same |
| Audio | AAC, 48 kHz, stereo, 320 kb/s | same |
| Audio level | mean −17.4 dB, peak −3.4 dBFS (no clipping) | same score |
| Decode test (`ffmpeg -f null`) | passes, no errors | passes, no errors |
| File size | 30.7 MB | 22.1 MB |

Thumbnail: `out/GeoDeck — Thumbnail.png`, 1920×1080, a still at 7.8 s (the "Meet GeoDeck" frame).

## Visual inspection

Frames were extracted from the final files at 1, 4, 6, 8.5, 12, 15.5, 20, 27, 34, 41, 47, 50, 53, 55, 58 and 59.5 s for both cuts. Contact sheets are in `out/verify/` (`sheet_landscape.png`, `sheet_vertical.png`).

Checked and OK:
- Opening task run and headline; logo mark appears at ~3.9 s.
- Scene cross-fades land on the score's whooshes at 5, 10, 17, 24, 31, 38 and 45 s.
- Scene copy and screenshots are not clipped; headlines are legible in both formats.
- Finale: lockup, "Alpha 1.0.0 — Out Now", "Available on GitHub", the repository URL, and the early-alpha line at the bottom.

## Problems found and fixed during production

- Vault headline wrapped into the bullet list. Fixed by widening the copy column, reducing the size, and repositioning the bullets.
- Montage headline overlapped the window. Fixed by moving and resizing the window.
- Landscape render was full-range `yuvj420p`. Re-encoded both files to limited-range `yuv420p`.

## Known limitations (not fixed)

- Footage is from a clean sandbox. Several screens are empty states (no Geometry Dash install, no mods, no vault snapshots, no NONG cache). They are shown as the app really renders them, not with invented data.
- The Geode index error toast was painted out of the screenshot copies (see `ASSETS.md`).
- In the vertical cut, screenshots are cropped tightly for legibility, so the smallest UI text is still hard to read on a phone.
- Visual inspection was done on sampled frames, not every frame. Motion smoothness was not judged by eye.
- No human has reviewed the cut for pacing or music sync beyond the frame and level checks above.

## Encoding

Initial renders used Remotion's default pipeline (`--codec=h264 --crf=16 --pixel-format=yuv420p`). The output was full-range `yuvj420p`, so both files were re-encoded with `libx264 -preset slow -crf 16 -pix_fmt yuv420p -color_range tv` and BT.709 tags, `aac 320k`, `-movflags +faststart`. Re-running `bash scripts/render.sh` produces the first-pass output, so add the re-encode step if you need the same final files.

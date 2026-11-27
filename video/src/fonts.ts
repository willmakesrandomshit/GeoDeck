import {continueRender, delayRender, staticFile} from 'remotion';
import {FONT} from './theme';

// Fonts are the same TTFs the application ships with (assets/fonts).
// Loading happens once at module load; Remotion waits for it before each frame.
const handle = delayRender('Loading GeoDeck fonts');

const faces: Array<[string, string, string]> = [
  [FONT.display, 'fonts/SpaceGrotesk-Bold.ttf', '700'],
  [FONT.body, 'fonts/IBMPlexSans-Regular.ttf', '400'],
  [FONT.bodySemi, 'fonts/IBMPlexSans-SemiBold.ttf', '600'],
  [FONT.mono, 'fonts/JetBrainsMono-Regular.ttf', '400'],
];

Promise.all(
  faces.map(async ([family, file, weight]) => {
    const face = new FontFace(family, `url(${staticFile(file)})`, {weight});
    await face.load();
    (document.fonts as unknown as {add: (f: FontFace) => void}).add(face);
  }),
)
  .then(() => continueRender(handle))
  .catch((err) => {
    console.error(err);
    continueRender(handle);
  });

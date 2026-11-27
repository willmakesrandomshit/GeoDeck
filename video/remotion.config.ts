import {Config} from '@remotion/cli/config';

Config.setVideoImageFormat('jpeg');
Config.setOverwriteOutput(true);
Config.setCodec('h264');
Config.setCrf(16);
Config.setPixelFormat('yuv420p');

// The sandbox cannot download Remotion's own Chrome build, so use the
// Chromium that ships with the environment. Override with REMOTION_BROWSER.
Config.setBrowserExecutable(
  process.env.REMOTION_BROWSER || '/opt/pw-browsers/chromium-1243/chrome-linux64/chrome',
);

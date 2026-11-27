import React from 'react';
import {Composition} from 'remotion';
import {Trailer} from './Trailer';
import {DURATION_FRAMES, FPS} from './timeline';

// Two compositions from one timeline: a 16:9 master and a 9:16 cut that is
// recomposed (not cropped) for TikTok and YouTube Shorts.
export const RemotionRoot: React.FC = () => (
  <>
    <Composition
      id="GeoDeck-Launch-Trailer"
      component={Trailer}
      durationInFrames={DURATION_FRAMES}
      fps={FPS}
      width={1920}
      height={1080}
      defaultProps={{vertical: false}}
    />
    <Composition
      id="GeoDeck-Vertical-Trailer"
      component={Trailer}
      durationInFrames={DURATION_FRAMES}
      fps={FPS}
      width={1080}
      height={1920}
      defaultProps={{vertical: true}}
    />
  </>
);

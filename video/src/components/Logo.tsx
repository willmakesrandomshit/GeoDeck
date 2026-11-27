import React from 'react';
import {Img, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig} from 'remotion';

// The official GeoDeck lockup (assets/geodeck_logo_lockup.png, 800x279).
// Reveal: a horizontal light sweep across the mark while it settles from a
// slight scale-down, then the mark holds still. No particles, no bloom.
export const Logo: React.FC<{width: number; delay?: number; sweep?: boolean}> = ({width, delay = 0, sweep = true}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const p = spring({frame: frame - delay, fps, config: {damping: 200, stiffness: 140}});
  const sweepX = interpolate(frame, [delay, delay + 40], [-30, 130], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'});
  const sweepOpacity = sweep ? interpolate(frame, [delay, delay + 8, delay + 40], [0, 0.35, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}) : 0;
  return (
    <div style={{position: 'relative', width, opacity: p, transform: `scale(${0.94 + 0.06 * p})`}}>
      <Img src={staticFile('brand/geodeck_logo_lockup.png')} style={{width: '100%', display: 'block'}} />
      {sweep && (
        <div
          style={{
            position: 'absolute',
            inset: 0,
            background: `linear-gradient(100deg, transparent ${sweepX - 12}%, rgba(255,255,255,${sweepOpacity}) ${sweepX}%, transparent ${sweepX + 12}%)`,
            mixBlendMode: 'screen',
            pointerEvents: 'none',
          }}
        />
      )}
    </div>
  );
};

import React from 'react';
import {AbsoluteFill, interpolate, useCurrentFrame, useVideoConfig} from 'remotion';
import {C} from '../theme';

// Quiet studio backdrop: the application's void colour, a faint accent light
// that drifts slowly, and a hairline grid. No glow stacks, no gradients for
// their own sake.
export const Backdrop: React.FC<{intensity?: number}> = ({intensity = 1}) => {
  const frame = useCurrentFrame();
  const {width, height} = useVideoConfig();
  const drift = interpolate(frame, [0, 3600], [0, 1]);
  const lightX = width * (0.32 + 0.06 * Math.sin(drift * Math.PI * 2));
  const lightY = height * (0.22 + 0.03 * Math.cos(drift * Math.PI * 2));
  const grid = Math.max(64, Math.round(width / 24));
  return (
    <AbsoluteFill style={{backgroundColor: C.void, overflow: 'hidden'}}>
      <AbsoluteFill
        style={{
          background: `radial-gradient(circle at ${lightX}px ${lightY}px, rgba(143,107,255,${0.1 * intensity}) 0px, rgba(143,107,255,0) ${Math.max(width, height) * 0.45}px)`,
        }}
      />
      <AbsoluteFill
        style={{
          backgroundImage: `linear-gradient(${C.border} 1px, transparent 1px), linear-gradient(90deg, ${C.border} 1px, transparent 1px)`,
          backgroundSize: `${grid}px ${grid}px`,
          backgroundPosition: `${-(frame * 0.15) % grid}px ${-(frame * 0.1) % grid}px`,
          opacity: 0.12 * intensity,
          maskImage: 'radial-gradient(ellipse at center, black 30%, transparent 75%)',
          WebkitMaskImage: 'radial-gradient(ellipse at center, black 30%, transparent 75%)',
        }}
      />
    </AbsoluteFill>
  );
};

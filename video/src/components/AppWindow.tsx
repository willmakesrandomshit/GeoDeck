import React from 'react';
import {Img, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig} from 'remotion';
import {C, FONT} from '../theme';

export type Focus = {
  /** focal point in source-image fractions (0..1) */
  x: number;
  y: number;
  /** magnification relative to the window's fit scale */
  zoom: number;
};

type Props = {
  shot: string;
  width: number;
  height: number;
  /** camera keyframes, in frames local to the parent sequence */
  from: Focus;
  to: Focus;
  travel: [number, number];
  delay?: number;
  caption?: string;
  /** source image size (screenshots are 1600x1000) */
  source?: [number, number];
  showChrome?: boolean;
  entrance?: 'rise' | 'none';
};

// A real application screenshot shown in a window frame. The camera is a
// focal-point crop: it can slowly push in or pan across the genuine interface.
export const AppWindow: React.FC<Props> = ({
  shot,
  width,
  height,
  from,
  to,
  travel,
  delay = 0,
  caption,
  source = [1600, 1000],
  showChrome = true,
  entrance = 'rise',
}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const ease = (v: number) => (v < 0 ? 0 : v > 1 ? 1 : v * v * (3 - 2 * v));
  const p = ease(interpolate(frame, travel, [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}));
  const focus = {
    x: from.x + (to.x - from.x) * p,
    y: from.y + (to.y - from.y) * p,
    zoom: from.zoom + (to.zoom - from.zoom) * p,
  };
  const enter = spring({frame: frame - delay, fps, config: {damping: 200, stiffness: 120}});
  const chromeH = showChrome ? Math.round(height * 0.045) : 0;
  const viewH = height - chromeH;
  const [sw, sh] = source;
  // Scale so the image covers the view at zoom 1 (cover fit), then magnify.
  const base = Math.max(width / sw, viewH / sh);
  const scale = base * focus.zoom;
  const imgW = sw * scale;
  const imgH = sh * scale;
  const left = width / 2 - focus.x * imgW;
  const top = viewH / 2 - focus.y * imgH;
  const rise = entrance === 'rise' ? (1 - enter) * 60 : 0;

  return (
    <div
      style={{
        width,
        height,
        borderRadius: 14,
        overflow: 'hidden',
        background: C.surface,
        border: `1px solid ${C.borderStrong}`,
        boxShadow: '0 40px 90px rgba(0,0,0,0.55)',
        transform: `translateY(${rise}px) scale(${0.985 + 0.015 * enter})`,
        opacity: enter,
        position: 'relative',
      }}
    >
      {showChrome && (
        <div
          style={{
            height: chromeH,
            background: C.sidebar,
            borderBottom: `1px solid ${C.border}`,
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '0 18px',
            fontFamily: FONT.mono,
            fontSize: Math.max(12, chromeH * 0.36),
            color: C.textMuted,
          }}
        >
          <span style={{width: 10, height: 10, borderRadius: 5, background: C.borderStrong}} />
          <span style={{width: 10, height: 10, borderRadius: 5, background: C.borderStrong}} />
          <span style={{width: 10, height: 10, borderRadius: 5, background: C.borderStrong}} />
          <span style={{marginLeft: 12, color: C.textSecondary}}>GeoDeck</span>
          <span style={{marginLeft: 6}}>1.0.0</span>
          {caption && <span style={{marginLeft: 'auto'}}>{caption}</span>}
        </div>
      )}
      <div style={{position: 'absolute', left: 0, top: chromeH, width, height: viewH, overflow: 'hidden'}}>
        <Img
          src={staticFile(`shots/${shot}.png`)}
          style={{position: 'absolute', left, top, width: imgW, height: imgH, maxWidth: 'none'}}
        />
      </div>
    </div>
  );
};

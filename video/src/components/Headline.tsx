import React from 'react';
import {interpolate, spring, useCurrentFrame, useVideoConfig} from 'remotion';
import {C, FONT} from '../theme';

type Props = {
  lines: string[];
  /** frame (local to the parent sequence) at which the first line starts */
  delay?: number;
  /** frames between successive lines */
  stagger?: number;
  size: number;
  color?: string;
  accentLine?: number;
  align?: 'left' | 'center';
  maxWidth?: number;
  weight?: number;
  letterSpacing?: string;
  lineHeight?: number;
  /** optional exit: frame at which the block starts to leave */
  exitAt?: number;
};

// Each line rises out of a mask with a damped spring. Deterministic: the
// value depends only on the frame number.
export const Headline: React.FC<Props> = ({
  lines,
  delay = 0,
  stagger = 6,
  size,
  color = C.text,
  accentLine,
  align = 'left',
  maxWidth,
  weight = 700,
  letterSpacing = '-0.03em',
  lineHeight = 1.02,
  exitAt,
}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const exitP =
    exitAt === undefined
      ? 0
      : interpolate(frame, [exitAt, exitAt + 14], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'});
  return (
    <div
      style={{
        maxWidth,
        textAlign: align,
        opacity: 1 - exitP,
        transform: `translateY(${-exitP * 24}px)`,
      }}
    >
      {lines.map((line, i) => {
        const p = spring({
          frame: frame - delay - i * stagger,
          fps,
          config: {damping: 200, stiffness: 180, mass: 0.9},
        });
        const isAccent = accentLine === i;
        return (
          <div key={line} style={{overflow: 'hidden', paddingBottom: size * 0.08, marginBottom: -size * 0.08}}>
            <div
              style={{
                fontFamily: FONT.display,
                fontWeight: weight,
                fontSize: size,
                lineHeight,
                letterSpacing,
                color: isAccent ? C.accentBright : color,
                transform: `translateY(${(1 - p) * size * 0.9}px)`,
                opacity: interpolate(p, [0, 0.35], [0, 1], {extrapolateRight: 'clamp'}),
                whiteSpace: 'pre-wrap',
              }}
            >
              {line}
            </div>
          </div>
        );
      })}
    </div>
  );
};

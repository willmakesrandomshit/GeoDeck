import React from 'react';
import {Img, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig, Sequence} from 'remotion';
import {Headline} from '../components/Headline';
import {Logo} from '../components/Logo';
import {C, FONT} from '../theme';
import {f} from '../timeline';

const TASKS = ['Locate the install', 'Switch the version', 'Verify the files', 'Install the mods', 'Back up the saves', 'Read the crash log'];

// 00:00-00:05. Dark screen, a rapid run of management tasks, then the headline
// and the GeoDeck mark.
export const Hook: React.FC<{vertical: boolean}> = ({vertical}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const taskWindow = f(2.6);
  const step = Math.round(fps * 0.42);
  const idx = Math.min(TASKS.length - 1, Math.floor(frame / step));
  const inTasks = frame < taskWindow;
  const taskLocal = frame % step;
  const taskP = spring({frame: taskLocal, fps, config: {damping: 200, stiffness: 260}});
  const taskOut = inTasks ? 1 : 1 - interpolate(frame, [taskWindow, taskWindow + 10], [0, 1], {extrapolateRight: 'clamp'});
  const fs = vertical ? 74 : 92;
  const headlineSize = vertical ? 84 : 96;
  const titleLines = vertical
    ? ['Managing Geometry', 'Dash just changed.']
    : ['Managing Geometry Dash', 'just changed.'];
  return (
    <div style={{position: 'absolute', inset: 0, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center'}}>
      {/* task run */}
      <div style={{position: 'absolute', opacity: inTasks ? taskOut : 0, transform: `translateY(${(1 - taskP) * 18}px)`}}>
        {inTasks && (
          <div style={{display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 18}}>
            <div style={{fontFamily: FONT.mono, fontSize: vertical ? 26 : 22, color: C.textMuted, letterSpacing: '0.08em', textTransform: 'uppercase'}}>
              {`0${idx + 1} / 06`}
            </div>
            <div style={{fontFamily: FONT.display, fontWeight: 700, fontSize: fs, color: C.text, letterSpacing: '-0.03em', opacity: taskP}}>
              {TASKS[idx]}
            </div>
          </div>
        )}
      </div>
      {/* headline */}
      <Sequence from={taskWindow} layout="none">
        <div style={{position: 'absolute', textAlign: 'center'}}>
          <Headline lines={titleLines} size={headlineSize} align="center" stagger={5} />
        </div>
      </Sequence>
      {/* logo mark */}
      <Sequence from={f(3.9)} layout="none">
        <div style={{position: 'absolute', bottom: vertical ? 520 : 150, left: 0, right: 0, display: 'flex', justifyContent: 'center'}}>
          <Img src={staticFile('brand/geodeck_icon.png')} style={{width: vertical ? 170 : 150, height: vertical ? 170 : 150}} />
        </div>
      </Sequence>
    </div>
  );
};

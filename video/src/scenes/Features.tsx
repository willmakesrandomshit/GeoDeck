import React from 'react';
import {AbsoluteFill, Img, Sequence, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig} from 'remotion';
import {AppWindow, Focus} from '../components/AppWindow';
import {Headline} from '../components/Headline';
import {Logo} from '../components/Logo';
import {C, FONT} from '../theme';
import {f} from '../timeline';

// ---------------------------------------------------------------------------
// Layout. Landscape: copy on the left, the window on the right.
// Vertical (1080x1920): copy stacked at the top, the window recomposed below
// with a tighter crop so the interface stays legible on a phone.
// ---------------------------------------------------------------------------
export type Box = {x: number; y: number; w: number; h: number};
export const layout = (vertical: boolean, width: number, height: number) => {
  if (vertical) {
    return {
      text: {x: 88, y: 210, w: width - 176, h: 680} as Box,
      win: {x: 60, y: 940, w: width - 120, h: 880} as Box,
      copySize: 116,
      subSize: 40,
    };
  }
  return {
    text: {x: 120, y: 170, w: 600, h: height - 340} as Box,
    win: {x: 770, y: 186, w: 1100, h: 688} as Box,
    copySize: 104,
    subSize: 30,
  };
};

const zoomFor = (vertical: boolean, focus: Focus): Focus =>
  vertical ? {...focus, zoom: focus.zoom * 1.35} : focus;

// Shared copy block: headline + optional subline, with delayed reveal.
const Copy: React.FC<{
  lines: string[];
  accentLine?: number;
  sub?: string[];
  subDelay?: number;
  vertical: boolean;
  delay?: number;
  size: number;
  subSize: number;
  box: Box;
  exitAt?: number;
}> = ({lines, accentLine, sub, subDelay = 24, vertical, delay = 0, size, subSize, box, exitAt}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  return (
    <div style={{position: 'absolute', left: box.x, top: box.y, width: box.w, height: box.h, display: 'flex', flexDirection: 'column', justifyContent: vertical ? 'flex-start' : 'center'}}>
      <Headline lines={lines} size={size} accentLine={accentLine} delay={delay} stagger={7} maxWidth={box.w} exitAt={exitAt} />
      {sub &&
        sub.map((line, i) => {
          const p = spring({frame: frame - delay - subDelay - i * 10, fps, config: {damping: 200}});
          const exitP = exitAt === undefined ? 0 : interpolate(frame, [exitAt, exitAt + 14], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'});
          return (
            <div
              key={line}
              style={{
                marginTop: i === 0 ? size * 0.36 : 10,
                fontFamily: FONT.body,
                fontSize: subSize,
                lineHeight: 1.4,
                color: C.textSecondary,
                opacity: p * (1 - exitP),
                transform: `translateY(${(1 - p) * 20 - exitP * 16}px)`,
                maxWidth: box.w,
              }}
            >
              {line}
            </div>
          );
        })}
    </div>
  );
};

// A label chip used to annotate real UI (e.g. "Automatic backups").
const Chip: React.FC<{text: string; delay: number; x?: number; y?: number}> = ({text, delay, x = 0, y = 0}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const p = spring({frame: frame - delay, fps, config: {damping: 200}});
  return (
    <div
      style={{
        position: 'absolute',
        left: x,
        top: y,
        opacity: p,
        transform: `translateY(${(1 - p) * 14}px)`,
        padding: '10px 18px',
        borderRadius: 999,
        background: C.elevated,
        border: `1px solid ${C.borderStrong}`,
        fontFamily: FONT.mono,
        fontSize: 22,
        color: C.textSecondary,
        whiteSpace: 'nowrap',
      }}
    >
      <span style={{display: 'inline-block', width: 8, height: 8, borderRadius: 4, background: C.accent, marginRight: 12, verticalAlign: 'middle'}} />
      {text}
    </div>
  );
};

type ScenePropsBase = {vertical: boolean; width: number; height: number};

// 00:05-00:10  "Meet GeoDeck." with a cinematic zoom-out on the real Home screen.
export const Meet: React.FC<ScenePropsBase> = ({vertical, width, height}) => {
  const L = layout(vertical, width, height);
  return (
    <AbsoluteFill>
      <Copy
        vertical={vertical}
        box={L.text}
        size={L.copySize}
        subSize={L.subSize}
        lines={['Meet', 'GeoDeck.']}
        accentLine={1}
        sub={['One application.', 'Everything Geometry Dash.']}
        exitAt={f(4.3)}
      />
      <div style={{position: 'absolute', left: L.win.x, top: L.win.y}}>
        <AppWindow
          shot="home"
          width={L.win.w}
          height={L.win.h}
          from={zoomFor(vertical, {x: 0.3, y: 0.28, zoom: 1.9})}
          to={zoomFor(vertical, {x: 0.5, y: 0.5, zoom: 1})}
          travel={[0, f(3.2)]}
          delay={2}
        />
      </div>
    </AbsoluteFill>
  );
};

// 00:10-00:17  "Your game. Your versions." Installs, Steam detection, version range.
export const Versions: React.FC<ScenePropsBase> = ({vertical, width, height}) => {
  const L = layout(vertical, width, height);
  return (
    <AbsoluteFill>
      <Copy
        vertical={vertical}
        box={L.text}
        size={L.copySize}
        subSize={L.subSize}
        lines={['Your game.', 'Your versions.']}
        accentLine={1}
        sub={['Steam library detection and isolated version slots.', 'Geometry Dash 1.900 through 2.2091: 29 recognised builds.']}
        exitAt={f(6.4)}
      />
      <div style={{position: 'absolute', left: L.win.x, top: L.win.y}}>
        <AppWindow
          shot="installs"
          width={L.win.w}
          height={L.win.h}
          from={zoomFor(vertical, {x: 0.3, y: 0.3, zoom: 1.35})}
          to={zoomFor(vertical, {x: 0.45, y: 0.36, zoom: 1.05})}
          travel={[0, f(7)]}
        />
        {!vertical && <Chip text="Steam detection" delay={f(3.2)} x={-10} y={L.win.h + 24} />}
        {!vertical && <Chip text="Version slots" delay={f(4.2)} x={220} y={L.win.h + 24} />}
      </div>
    </AbsoluteFill>
  );
};

// 00:17-00:24  "Discover. Manage. Customise." Installed mods, dependency audit, .gdpack profiles.
export const Mods: React.FC<ScenePropsBase> = ({vertical, width, height}) => {
  const L = layout(vertical, width, height);
  const cuts = [
    {shot: 'mods', from: 0, to: f(2.4), a: {x: 0.3, y: 0.3, zoom: 1.4}, b: {x: 0.5, y: 0.45, zoom: 1.12}},
    {shot: 'audit', from: f(2.4), to: f(4.7), a: {x: 0.35, y: 0.3, zoom: 1.3}, b: {x: 0.5, y: 0.42, zoom: 1.1}},
    {shot: 'profiles', from: f(4.7), to: f(7), a: {x: 0.35, y: 0.32, zoom: 1.3}, b: {x: 0.62, y: 0.4, zoom: 1.1}},
  ];
  return (
    <AbsoluteFill>
      <Copy
        vertical={vertical}
        box={L.text}
        size={vertical ? 112 : 96}
        subSize={L.subSize}
        lines={['Discover.', 'Manage.', 'Customise.']}
        accentLine={2}
        sub={['Browse the Geode index, audit dependencies,', 'and move whole setups as portable .gdpack modpacks.']}
        exitAt={f(6.4)}
      />
      {cuts.map((c, i) => (
        <Sequence key={c.shot} from={c.from} durationInFrames={c.to - c.from} layout="none">
          <div style={{position: 'absolute', left: L.win.x, top: L.win.y}}>
            <AppWindow
              shot={c.shot}
              width={L.win.w}
              height={L.win.h}
              from={zoomFor(vertical, c.a)}
              to={zoomFor(vertical, c.b)}
              travel={[0, c.to - c.from]}
              entrance={i === 0 ? 'rise' : 'none'}
            />
          </div>
        </Sequence>
      ))}
    </AbsoluteFill>
  );
};

// 00:24-00:31  "Your soundtrack. Your way." NONG song library and track replacement.
export const Songs: React.FC<ScenePropsBase> = ({vertical, width, height}) => {
  const L = layout(vertical, width, height);
  return (
    <AbsoluteFill>
      <Copy
        vertical={vertical}
        box={L.text}
        size={L.copySize}
        subSize={L.subSize}
        lines={['Your soundtrack.', 'Your way.']}
        accentLine={1}
        sub={['NONG song library with metadata and search.', 'Replace any track, with the original kept safe.']}
        exitAt={f(6.4)}
      />
      <div style={{position: 'absolute', left: L.win.x, top: L.win.y}}>
        <AppWindow
          shot="songs"
          width={L.win.w}
          height={L.win.h}
          from={zoomFor(vertical, {x: 0.45, y: 0.5, zoom: 1.25})}
          to={zoomFor(vertical, {x: 0.5, y: 0.7, zoom: 1.12})}
          travel={[0, f(7)]}
        />
      </div>
    </AbsoluteFill>
  );
};

// 00:31-00:38  "Protect your progress." Vault: automatic backups, AES-256-GCM, integrity.
export const Vault: React.FC<ScenePropsBase> = ({vertical, width, height}) => {
  const L = layout(vertical, width, height);
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const bullets = ['Automatic backups', 'AES-256-GCM encryption', 'Save integrity verification'];
  return (
    <AbsoluteFill>
      <Copy
        vertical={vertical}
        box={vertical ? {x: L.text.x, y: L.text.y, w: L.text.w, h: 420} : {x: L.text.x, y: 300, w: 720, h: 260}}
        size={vertical ? 104 : 88}
        subSize={L.subSize}
        lines={['Protect', 'your progress.']}
        accentLine={1}
        exitAt={f(6.4)}
      />
      <div style={{position: 'absolute', left: L.text.x, top: vertical ? 560 : 610, display: 'flex', flexDirection: 'column', gap: 18}}>
        {bullets.map((b, i) => {
          const p = spring({frame: frame - 24 - i * 12, fps, config: {damping: 200}});
          return (
            <div key={b} style={{display: 'flex', alignItems: 'center', gap: 18, opacity: p, transform: `translateX(${(1 - p) * -30}px)`, fontFamily: FONT.bodySemi, fontSize: vertical ? 40 : 30, color: C.text}}>
              <span style={{width: 12, height: 12, borderRadius: 6, background: C.ok}} />
              {b}
            </div>
          );
        })}
      </div>
      <div style={{position: 'absolute', left: L.win.x, top: L.win.y}}>
        <AppWindow
          shot="backups"
          width={L.win.w}
          height={L.win.h}
          from={zoomFor(vertical, {x: 0.45, y: 0.33, zoom: 1.45})}
          to={zoomFor(vertical, {x: 0.55, y: 0.55, zoom: 1.2})}
          travel={[0, f(7)]}
        />
      </div>
    </AbsoluteFill>
  );
};

// 00:38-00:45  Diagnostics and updates: quick, clean cuts with dynamic type.
export const Diagnostics: React.FC<ScenePropsBase> = ({vertical, width, height}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const L = layout(vertical, width, height);
  const cuts: Array<{shot: string; from: number; to: number; word: string; sub: string}> = [
    {shot: 'health', from: 0, to: f(2.4), word: 'Diagnose.', sub: '8-point health check. Plain-English error explainer.'},
    {shot: 'crash', from: f(2.4), to: f(4.6), word: 'Recover.', sub: 'Crash log analyzer with one-click Disable Faulty Mod.'},
    {shot: 'settings', from: f(4.6), to: f(7), word: 'Stay current.', sub: 'Automatic updates, verified with SHA-256.'},
  ];
  return (
    <AbsoluteFill>
      {cuts.map((c) => {
        const local = frame - c.from;
        const wordP = spring({frame: local, fps, config: {damping: 14, stiffness: 300}});
        return (
          <Sequence key={c.shot} from={c.from} durationInFrames={c.to - c.from} layout="none">
            <Copy
              vertical={vertical}
              box={L.text}
              size={vertical ? 124 : 112}
              subSize={L.subSize}
              lines={[c.word]}
              accentLine={0}
              sub={[c.sub]}
              subDelay={6}
              exitAt={c.to - c.from - 8}
            />
            <div style={{position: 'absolute', left: L.win.x, top: L.win.y, transform: `scale(${0.96 + 0.04 * wordP})`, opacity: 1}}>
              <AppWindow
                shot={c.shot}
                width={L.win.w}
                height={L.win.h}
                from={zoomFor(vertical, {x: 0.45, y: 0.3, zoom: 1.25})}
                to={zoomFor(vertical, {x: 0.5, y: 0.5, zoom: 1.0})}
                travel={[0, c.to - c.from]}
                entrance="none"
              />
            </div>
          </Sequence>
        );
      })}
    </AbsoluteFill>
  );
};

// 00:45-00:52  Montage that builds to the musical climax.
export const Montage: React.FC<ScenePropsBase> = ({vertical, width, height}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const beat = Math.round(fps * 0.5);
  // Every label below names a feature that exists in the repository.
  type Slot = {shot: string; label: string; x: number; y: number; zoom: number};
  const slots: Slot[] = [
    {shot: 'home', label: 'Launcher', x: 0.5, y: 0.5, zoom: 1.0},
    {shot: 'installs', label: 'Version slots · Steam detection', x: 0.5, y: 0.5, zoom: 1.0},
    {shot: 'mods', label: 'Geode Mod Studio', x: 0.5, y: 0.5, zoom: 1.0},
    {shot: 'profiles', label: '.gdpack modpacks · launch presets', x: 0.5, y: 0.5, zoom: 1.0},
    {shot: 'songs', label: 'NONG song library', x: 0.5, y: 0.5, zoom: 1.0},
    {shot: 'backups', label: 'Encrypted vault · AES-256-GCM', x: 0.5, y: 0.5, zoom: 1.0},
    {shot: 'slots', label: 'Save slots · Save Doctor', x: 0.5, y: 0.5, zoom: 1.0},
    {shot: 'compactor', label: 'Save compactor · .geodesync bundles', x: 0.5, y: 0.5, zoom: 1.0},
    {shot: 'health', label: '8-point health check', x: 0.5, y: 0.5, zoom: 1.0},
    {shot: 'crash', label: 'Crash log analyzer', x: 0.5, y: 0.5, zoom: 1.0},
    {shot: 'audit', label: 'Dependency audit · Keybind doctor', x: 0.5, y: 0.5, zoom: 1.0},
    {shot: 'levels', label: 'Level export (.gmd)', x: 0.5, y: 0.5, zoom: 1.0},
    {shot: 'settings', label: 'Discord Rich Presence · auto updates', x: 0.5, y: 0.5, zoom: 1.0},
    {shot: 'textures', label: 'Texture packs · PNG integrity', x: 0.4, y: 0.3, zoom: 1.7},
  ];
  const idx = Math.min(slots.length - 1, Math.floor(frame / beat));
  const slot = slots[idx];
  const local = frame % beat;
  const p = spring({frame: local, fps, config: {damping: 200, stiffness: 220}});
  const inRust = frame < f(3.6);
  const word = inRust ? 'Built in Rust.' : 'Built for Geometry Dash.';
  const wordP = spring({frame: frame - (inRust ? 0 : f(3.6)), fps, config: {damping: 200}});
  const winW = vertical ? width - 120 : 1240;
  const winH = vertical ? Math.round(winW * 1.1) : 700;
  return (
    <AbsoluteFill>
      <div
        style={{
          position: 'absolute',
          left: (width - winW) / 2,
          top: vertical ? 420 : 300,
          transform: `scale(${1.02 - 0.02 * p})`,
          opacity: 0.55 + 0.45 * p,
        }}
      >
        <AppWindow
          key={`${slot.shot}-${idx}`}
          shot={slot.shot}
          width={winW}
          height={winH}
          from={{x: slot.x, y: slot.y, zoom: slot.zoom * (vertical ? 1.5 : 1.0)}}
          to={{x: slot.x, y: slot.y, zoom: slot.zoom * (vertical ? 1.55 : 1.05)}}
          travel={[0, beat]}
          entrance="none"
        />
      </div>
      <div
        style={{
          position: 'absolute',
          left: 0,
          right: 0,
          top: vertical ? 200 : 110,
          textAlign: 'center',
          fontFamily: FONT.display,
          fontWeight: 700,
          fontSize: vertical ? 110 : 96,
          letterSpacing: '-0.03em',
          color: C.text,
          opacity: wordP,
          transform: `translateY(${(1 - wordP) * 30}px)`,
          textShadow: '0 4px 40px rgba(0,0,0,0.6)',
        }}
      >
        {word}
      </div>
      <div
        style={{
          position: 'absolute',
          left: 0,
          right: 0,
          bottom: vertical ? 120 : 60,
          textAlign: 'center',
          fontFamily: FONT.mono,
          fontSize: 24,
          letterSpacing: '0.12em',
          textTransform: 'uppercase',
          color: C.accentBright,
        }}
      >
        {slot.label}
      </div>
    </AbsoluteFill>
  );
};

// 00:52-01:00  Final reveal, call to action and early-alpha disclaimer.
export const Finale: React.FC<ScenePropsBase> = ({vertical, width, height}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const logoW = vertical ? 820 : 760;
  const reveal = (d: number) => spring({frame: frame - d, fps, config: {damping: 200}});
  const line = (d: number, text: string, style: React.CSSProperties) => {
    const p = reveal(d);
    return (
      <div style={{opacity: p, transform: `translateY(${(1 - p) * 18}px)`, ...style}}>{text}</div>
    );
  };
  return (
    <AbsoluteFill style={{alignItems: 'center', justifyContent: 'center', flexDirection: 'column', display: 'flex'}}>
      <div style={{marginTop: vertical ? -260 : -120}}>
        <Logo width={logoW} delay={f(0.4)} />
      </div>
      <div style={{marginTop: vertical ? 90 : 50, textAlign: 'center', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: vertical ? 22 : 16}}>
        {line(f(2.2), 'Alpha 1.0.0 — Out Now', {fontFamily: FONT.display, fontWeight: 700, fontSize: vertical ? 64 : 58, color: C.accentBright, letterSpacing: '-0.02em'})}
        {line(f(3.6), 'Available on GitHub', {fontFamily: FONT.bodySemi, fontSize: vertical ? 40 : 34, color: C.text})}
        {line(
          f(4.8),
          'github.com/willmakesrandomshit/GeoDeck',
          {
            fontFamily: FONT.mono,
            fontSize: vertical ? 34 : 30,
            color: C.text,
            padding: '16px 28px',
            border: `1px solid ${C.accent}`,
            borderRadius: 999,
            background: C.accentSoft,
            marginTop: 10,
          },
        )}
        {line(f(5.6), 'Download it, try it, and tell us what breaks.', {fontFamily: FONT.body, fontSize: vertical ? 34 : 28, color: C.textSecondary})}
      </div>
      <div
        style={{
          position: 'absolute',
          left: 0,
          right: 0,
          bottom: vertical ? 150 : 56,
          textAlign: 'center',
          fontFamily: FONT.mono,
          fontSize: vertical ? 24 : 20,
          color: C.textMuted,
          opacity: reveal(f(6.4)),
          padding: '0 60px',
          lineHeight: 1.5,
        }}
      >
        Early alpha: expect rough edges. Back up your saves before you test.
        <div style={{marginTop: 8, fontSize: vertical ? 20 : 17, color: C.textMuted}}>
          Music: "Rynos Theme" by Kevin MacLeod (incompetech.com), CC BY 4.0
        </div>
      </div>
    </AbsoluteFill>
  );
};

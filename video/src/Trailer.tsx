import React from 'react';
import {AbsoluteFill, Audio, Sequence, interpolate, staticFile, useCurrentFrame} from 'remotion';
import {Backdrop} from './components/Backdrop';
import {Diagnostics, Finale, Meet, Montage, Mods, Songs, Vault, Versions} from './scenes/Features';
import {Hook} from './scenes/Hook';
import {f, SCENES} from './timeline';
import './fonts';

// Wraps each scene with a short cross-fade so cuts land on the score's
// whoosh and beat accents rather than popping.
const Scene: React.FC<{from: number; to: number; children: React.ReactNode}> = ({from, to, children}) => {
  const frame = useCurrentFrame();
  const local = frame - f(from);
  const len = f(to) - f(from);
  const fade = 8;
  const opacity = interpolate(local, [0, fade, len - fade, len], [0, 1, 1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'});
  return (
    <Sequence from={f(from)} durationInFrames={len} layout="none">
      <AbsoluteFill style={{opacity}}>{children}</AbsoluteFill>
    </Sequence>
  );
};

export type TrailerProps = {vertical?: boolean};

export const Trailer: React.FC<TrailerProps> = ({vertical = false}) => {
  const W = vertical ? 1080 : 1920;
  const H = vertical ? 1920 : 1080;
  const common = {vertical, width: W, height: H};
  return (
    <AbsoluteFill style={{backgroundColor: '#0D0E11'}}>
      <Backdrop />
      <Audio src={staticFile('audio/trailer-final.wav')} />
      <Scene from={SCENES.hook[0]} to={SCENES.hook[1]}><Hook vertical={vertical} /></Scene>
      <Scene from={SCENES.meet[0]} to={SCENES.meet[1]}><Meet {...common} /></Scene>
      <Scene from={SCENES.versions[0]} to={SCENES.versions[1]}><Versions {...common} /></Scene>
      <Scene from={SCENES.mods[0]} to={SCENES.mods[1]}><Mods {...common} /></Scene>
      <Scene from={SCENES.songs[0]} to={SCENES.songs[1]}><Songs {...common} /></Scene>
      <Scene from={SCENES.vault[0]} to={SCENES.vault[1]}><Vault {...common} /></Scene>
      <Scene from={SCENES.diagnostics[0]} to={SCENES.diagnostics[1]}><Diagnostics {...common} /></Scene>
      <Scene from={SCENES.montage[0]} to={SCENES.montage[1]}><Montage {...common} /></Scene>
      <Scene from={SCENES.finale[0]} to={SCENES.finale[1]}><Finale {...common} /></Scene>
    </AbsoluteFill>
  );
};

// Generates the GeoDeck trailer score and sound design from scratch.
// Everything here is synthesised by this script (no samples, no third-party
// audio), so the track is original work owned by the project. Output:
//   public/audio/trailer-score.wav   - full mix (music + sound effects), 60 s, 48 kHz stereo
//   public/audio/trailer-music.wav   - music only (for editors who want to re-mix)
//
// Timing is locked to 120 BPM (1 beat = 0.5 s, 1 bar = 2 s) so every scene
// boundary in src/timeline.ts falls on a beat.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const outDir = path.join(here, "..", "public", "audio");
fs.mkdirSync(outDir, { recursive: true });

const SR = 48000;
const DUR = 60;
const N = SR * DUR;
const BEAT = 0.5;
const TAU = Math.PI * 2;

const music = [new Float32Array(N), new Float32Array(N)];
const sfx = [new Float32Array(N), new Float32Array(N)];

// Deterministic noise so renders are reproducible.
let seed = 1337;
const rnd = () => {
  seed = (seed * 1664525 + 1013904223) >>> 0;
  return seed / 4294967296;
};
const noise = () => rnd() * 2 - 1;

const midi = (n) => 440 * Math.pow(2, (n - 69) / 12);

function addTo(buf, t, sample, pan = 0) {
  const i = Math.floor(t * SR);
  if (i < 0 || i >= N) return;
  const l = Math.cos(((pan + 1) * Math.PI) / 4);
  const r = Math.sin(((pan + 1) * Math.PI) / 4);
  buf[0][i] += sample * l;
  buf[1][i] += sample * r;
}

// Render a voice of duration `len` seconds starting at `t`.
function voice(buf, t, len, fn, pan = 0, gain = 1) {
  const start = Math.floor(t * SR);
  const count = Math.floor(len * SR);
  for (let k = 0; k < count; k++) {
    const i = start + k;
    if (i < 0 || i >= N) continue;
    const s = fn(k / SR, k) * gain;
    const l = Math.cos(((pan + 1) * Math.PI) / 4);
    const r = Math.sin(((pan + 1) * Math.PI) / 4);
    buf[0][i] += s * l;
    buf[1][i] += s * r;
  }
}

const expEnv = (x, rate) => Math.exp(-x * rate);

function kick(t, gain = 0.9) {
  voice(music, t, 0.32, (x) => {
    const f = 42 + 90 * expEnv(x, 28);
    const phase = TAU * (42 * x + (90 / 28) * (1 - expEnv(x, 28)));
    return Math.sin(phase) * expEnv(x, 9) * 1.0 + 0 * f;
  }, 0, gain);
}

function hat(t, gain = 0.09, pan = 0.3) {
  let prev = 0;
  voice(music, t, 0.09, (x) => {
    const n = noise();
    const hp = n - prev; // crude high-pass
    prev = n;
    return hp * expEnv(x, 60) * 0.9;
  }, pan, gain);
}

function bass(t, note, len) {
  const f = midi(note);
  let lp = 0;
  voice(music, t, len, (x) => {
    const saw = 2 * ((f * x) % 1) - 1;
    const cutoff = 0.08 + 0.5 * expEnv(x, 6);
    lp += cutoff * (saw - lp);
    const env = Math.min(1, x / 0.01) * expEnv(x, 2.2);
    return lp * env * 0.9;
  }, 0, 0.42);
}

function pad(t, notes, len, gain = 0.07) {
  for (const n of notes) {
    const f = midi(n);
    for (const det of [-0.0042, 0, 0.0042]) {
      voice(music, t, len, (x) => {
        const tri = (2 / Math.PI) * Math.asin(Math.sin(TAU * f * (1 + det) * x));
        const env = Math.min(1, x / 0.6) * Math.min(1, (len - x) / 0.8);
        return tri * env;
      }, det * 200, gain);
    }
  }
}

function pluck(t, note, gain = 0.12, pan = 0) {
  const f = midi(note);
  voice(music, t, 0.5, (x) => {
    const tri = (2 / Math.PI) * Math.asin(Math.sin(TAU * f * x));
    return tri * expEnv(x, 7) * 0.9;
  }, pan, gain);
}

function whoosh(t, dur = 0.9, gain = 0.35) {
  let lp = 0;
  voice(sfx, t, dur, (x) => {
    const p = x / dur;
    const cutoff = 0.005 + 0.3 * Math.pow(p, 2.2);
    lp += cutoff * (noise() - lp);
    return lp * Math.sin(Math.PI * p) * 1.6;
  }, 0, gain);
}

function tick(t, gain = 0.22, pan = -0.2) {
  voice(sfx, t, 0.05, (x) => Math.sin(TAU * 2400 * x) * expEnv(x, 120), pan, gain);
}

function uiBlip(t, gain = 0.18) {
  voice(sfx, t, 0.14, (x) => Math.sin(TAU * (880 + 1320 * x * 6) * x) * expEnv(x, 22), 0, gain);
}

function impact(t, gain = 0.9) {
  voice(sfx, t, 2.2, (x) => {
    const f = 55 * Math.pow(0.35, x / 2.2 * 6) + 30;
    return (Math.sin(TAU * f * x) * expEnv(x, 2.4) + 0.35 * noise() * expEnv(x, 9)) * 1.0;
  }, 0, gain);
}

function riser(t, dur, gain = 0.25) {
  let lp = 0;
  voice(sfx, t, dur, (x) => {
    const p = x / dur;
    const cutoff = 0.002 + 0.45 * p * p;
    lp += cutoff * (noise() - lp);
    return lp * Math.pow(p, 1.5) * 1.4;
  }, 0, gain);
}

// ---- Harmony: A minor loop, one chord per bar (2 s) ----------------------
const chords = [
  [57, 60, 64], // Am
  [53, 57, 60], // F
  [48, 52, 55], // C
  [55, 59, 62], // G
];
const bassLine = [45, 45, 48, 45, 41, 41, 43, 43]; // per half bar pattern
const lead = [69, 72, 76, 74, 72, 69, 67, 69, 72, 76, 79, 76, 74, 72, 71, 72];

// Section plan (seconds)
// 0-5   hook: pad + drone, task-card ticks
// 5-10  kick enters
// 10-17 bass + hats
// 17-24 arpeggio + kick
// 24-31 lead melody
// 31-38 breakdown (pad, lead, no kick)
// 38-45 full groove
// 45-52 build (riser, snares, 16th hats)
// 52-60 drop/chord, fade
const sections = [
  { from: 0, to: 5, pad: 1, kick: 0, bass: 0, hats: 0, arp: 0, lead: 0 },
  { from: 5, to: 10, pad: 1, kick: 1, bass: 0, hats: 1, arp: 0, lead: 0 },
  { from: 10, to: 17, pad: 1, kick: 1, bass: 1, hats: 1, arp: 0, lead: 0 },
  { from: 17, to: 24, pad: 0.7, kick: 1, bass: 1, hats: 1, arp: 1, lead: 0 },
  { from: 24, to: 31, pad: 0.7, kick: 1, bass: 1, hats: 1, arp: 1, lead: 1 },
  { from: 31, to: 38, pad: 1, kick: 0, bass: 0, hats: 0, arp: 0, lead: 1 },
  { from: 38, to: 45, pad: 0.8, kick: 1, bass: 1, hats: 1, arp: 1, lead: 1 },
  { from: 45, to: 52, pad: 0.6, kick: 1, bass: 0, hats: 1, arp: 1, lead: 0 },
  { from: 52, to: 60, pad: 1.2, kick: 0, bass: 1, hats: 0, arp: 0, lead: 1 },
];
const sectionAt = (t) => sections.find((s) => t >= s.from && t < s.to) ?? sections[sections.length - 1];

const totalBeats = Math.floor(DUR / BEAT);
for (let b = 0; b < totalBeats; b++) {
  const t = b * BEAT;
  if (t >= DUR) break;
  const sec = sectionAt(t);
  const bar = Math.floor(b / 4);
  const chord = chords[bar % 4];
  const beatInBar = b % 4;

  if (beatInBar === 0 && t >= 0) {
    pad(t, chord.map((n) => n + (t >= 52 ? 12 : 0)), 2 * 1.0, 0.08 * sec.pad);
  }
  if (sec.kick && (beatInBar !== 2 || sec.from >= 17)) kick(t, 0.85);
  if (sec.hats) {
    hat(t + BEAT / 2, 0.07);
    if (sec.from >= 38) {
      hat(t + BEAT / 4, 0.04, -0.3);
      hat(t + (3 * BEAT) / 4, 0.04, 0.3);
    }
  }
  if (sec.bass) {
    const n = bassLine[(b * 2) % bassLine.length];
    bass(t, n, BEAT * 0.9);
    bass(t + BEAT / 2, bassLine[(b * 2 + 1) % bassLine.length], BEAT * 0.4);
  }
  if (sec.arp) {
    for (let s = 0; s < 4; s++) {
      const n = chord[s % 3] + (s >= 2 ? 12 : 0);
      pluck(t + s * (BEAT / 4), n, 0.07, s % 2 ? 0.4 : -0.4);
    }
  }
  if (sec.lead) {
    const idx = (b * 2) % lead.length;
    voice(music, t, BEAT * 0.8, (x) => {
      const f = midi(lead[idx]);
      const sq = Math.sign(Math.sin(TAU * f * x));
      return (0.6 * sq + 0.4 * Math.sin(TAU * f * x)) * expEnv(x, 3.2) * 0.8;
    }, -0.15, 0.05);
  }
}

// Build-up snare + riser (45-52)
for (let b = 0; b < 14; b++) {
  const t = 45 + b * 0.5 + BEAT / 2;
  voice(sfx, t, 0.22, (x) => (noise() * 0.8 + Math.sin(TAU * 190 * x) * 0.2) * expEnv(x, 18), 0, 0.12 + b * 0.008);
}
riser(44.5, 7.5, 0.22);

// Hook tick sequence (task cards every 0.5 s during the opening)
for (let i = 0; i < 9; i++) tick(0.25 + i * 0.5, 0.18 + (i % 3) * 0.03, i % 2 ? 0.3 : -0.3);

// Scene transitions (whooshes) and reveal clicks
[5, 10, 17, 24, 31, 38, 45].forEach((t) => whoosh(t - 0.2, 0.9, 0.32));
[4.8, 9.8, 16.8, 23.8, 30.8, 37.8].forEach((t) => uiBlip(t, 0.12));
// Logo reveal sound
uiBlip(4.9, 0.2);
// Final impact + chord bloom at 52 s
impact(52.0, 0.85);
whoosh(51.2, 0.8, 0.2);
pad(52.0, [57, 60, 64, 69], 8, 0.09);
uiBlip(56.0, 0.08);

// Master: sum, gentle fade in/out, soft limiter.
const mixOut = (buffers, withSfx) => {
  const L = new Float32Array(N);
  const R = new Float32Array(N);
  for (let i = 0; i < N; i++) {
    L[i] = buffers.music[0][i] + (withSfx ? buffers.sfx[0][i] : 0);
    R[i] = buffers.music[1][i] + (withSfx ? buffers.sfx[1][i] : 0);
  }
  const fadeIn = 0.4 * SR;
  const fadeOut = 2.0 * SR;
  let peak = 0;
  for (let i = 0; i < N; i++) {
    let g = 1;
    if (i < fadeIn) g = i / fadeIn;
    if (i > N - fadeOut) g *= (N - i) / fadeOut;
    L[i] *= g;
    R[i] *= g;
    peak = Math.max(peak, Math.abs(L[i]), Math.abs(R[i]));
  }
  const norm = peak > 0.89 ? 0.89 / peak : 1;
  for (let i = 0; i < N; i++) {
    L[i] = Math.tanh(L[i] * norm * 1.1) * 0.9;
    R[i] = Math.tanh(R[i] * norm * 1.1) * 0.9;
  }
  return [L, R];
};

function writeWav(file, [L, R]) {
  const dataBytes = N * 2 * 2;
  const buf = Buffer.alloc(44 + dataBytes);
  buf.write("RIFF", 0);
  buf.writeUInt32LE(36 + dataBytes, 4);
  buf.write("WAVE", 8);
  buf.write("fmt ", 12);
  buf.writeUInt32LE(16, 16);
  buf.writeUInt16LE(1, 20);
  buf.writeUInt16LE(2, 22);
  buf.writeUInt32LE(SR, 24);
  buf.writeUInt32LE(SR * 4, 28);
  buf.writeUInt16LE(4, 32);
  buf.writeUInt16LE(16, 34);
  buf.write("data", 36);
  buf.writeUInt32LE(dataBytes, 40);
  for (let i = 0; i < N; i++) {
    buf.writeInt16LE(Math.max(-32767, Math.min(32767, Math.round(L[i] * 32767))), 44 + i * 4);
    buf.writeInt16LE(Math.max(-32767, Math.min(32767, Math.round(R[i] * 32767))), 46 + i * 4);
  }
  fs.writeFileSync(file, buf);
}

writeWav(path.join(outDir, "trailer-score.wav"), mixOut({ music, sfx }, true));
writeWav(path.join(outDir, "trailer-sfx.wav"), mixOut({ music: [new Float32Array(N), new Float32Array(N)], sfx }, true));
writeWav(path.join(outDir, "trailer-music.wav"), mixOut({ music, sfx: [new Float32Array(N), new Float32Array(N)] }, false));
console.log("wrote", path.join(outDir, "trailer-score.wav"));

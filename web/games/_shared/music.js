// Generative background music for the web family. Everything is synthesized in
// WebAudio (no samples, no licences): a seeded composer writes 8-bar sections
// (A A' B A'' ...) over a chord progression, with a remembered motif so the tune
// is recognizable, and a small band of instruments plays it through a generated
// reverb. Each game picks a style preset.
//
//   const m = createMusic({ ctx, dest, style: 'zen', seed: 7 });  m.start(); m.stop(); m.duck(2)
//   music.setup('zen')   // convenience for pages built on gc.js: button #music-btn, key M, saved choice
//
// renderOffline(style, seconds) renders into an OfflineAudioContext for checks.
import { rng } from './gc.js';

// ------------------------------------------------------------------ theory
const MODES = {
  ionian: [0, 2, 4, 5, 7, 9, 11], dorian: [0, 2, 3, 5, 7, 9, 10], mixolydian: [0, 2, 4, 5, 7, 9, 10],
  aeolian: [0, 2, 3, 5, 7, 8, 10], lydian: [0, 2, 4, 6, 7, 9, 11],
  yo: [0, 2, 5, 7, 9], miyako: [0, 1, 5, 7, 8], majpent: [0, 2, 4, 7, 9],
};
const mtof = (m) => 440 * 2 ** ((m - 69) / 12);
/** Pitch of scale index i (0 = tonic) in a mode, relative to key. */
function degree(mode, i) {
  const s = MODES[mode], n = s.length;
  const o = Math.floor(i / n), k = ((i % n) + n) % n;
  return s[k] + 12 * o;
}
function chordTones(mode, root, size) { const out = []; for (let k = 0; k < size; k++) out.push(root + 2 * k); return out; } // scale indices

// ------------------------------------------------------------------ styles
export const STYLES = {
  cozy:     { bpm: 76, beats: 4, key: 53, mode: 'ionian', A: [0, 5, 1, 4], B: [3, 2, 1, 4], seventh: true, swing: 0.1, lead: 'epiano', comp: 'stabs', compInst: 'epiano', bass: 'bass', drums: 'brush', reverb: 0.22, density: 0.7, leadOct: 2 },
  funfair:  { bpm: 168, beats: 3, key: 55, soft: 1.4, mode: 'ionian', A: [0, 0, 4, 4, 4, 4, 0, 0], B: [3, 3, 0, 0, 1, 4, 0, 0], lead: 'calliope', comp: 'waltz', compInst: 'calliope', bass: 'tuba', drums: 'tick', reverb: 0.15, density: 0.85, leadOct: 2 },
  zen:      { bpm: 58, beats: 4, key: 62, soft: 1.8, mode: 'miyako', A: [0, 0, 3, 0], B: [2, 3, 1, 0], lead: 'koto', alt: 'shaku', comp: 'drone', compInst: 'pad', bass: null, drums: null, reverb: 0.5, density: 0.42, leadOct: 1 },
  lamp:     { bpm: 66, beats: 4, key: 50, mode: 'dorian', A: [0, 3, 0, 4], B: [5, 6, 3, 4], seventh: true, lead: 'bell', comp: 'arp', compInst: 'epiano', bass: 'bass', drums: null, reverb: 0.38, density: 0.45, leadOct: 2 },
  river:    { bpm: 62, beats: 4, key: 62, mode: 'yo', A: [0, 3, 1, 0], B: [2, 3, 4, 0], lead: 'koto', alt: 'shaku', comp: 'pad', compInst: 'pad', bass: 'sub', drums: null, reverb: 0.55, density: 0.48, leadOct: 1 },
  stars:    { bpm: 52, beats: 4, key: 53, mode: 'lydian', A: [0, 1, 0, 1], B: [5, 4, 3, 1], seventh: true, lead: 'celesta', comp: 'pad', compInst: 'pad', bass: 'sub', drums: null, reverb: 0.65, density: 0.32, leadOct: 2 },
  picnic:   { bpm: 108, beats: 4, key: 55, mode: 'ionian', A: [0, 4, 5, 3], B: [3, 4, 0, 5], lead: 'marimba', comp: 'arp', compInst: 'marimba', bass: 'bass', drums: 'wood', reverb: 0.18, density: 0.75, leadOct: 2 },
  stroll:   { bpm: 96, beats: 4, key: 55, mode: 'ionian', A: [0, 5, 1, 4], B: [3, 3, 0, 4], seventh: true, swing: 0.14, lead: 'clarinet', comp: 'oompah', compInst: 'accordion', bass: 'bass', drums: 'brush', reverb: 0.2, density: 0.7, leadOct: 2 },
  workshop: { bpm: 72, beats: 4, key: 53, mode: 'ionian', A: [0, 3, 4, 0], B: [5, 3, 1, 4], lead: 'musicbox', comp: 'arp', compInst: 'musicbox', bass: 'sub', drums: null, reverb: 0.3, density: 0.45, leadOct: 2, soft: 0.85 },
  factory:  { bpm: 112, beats: 4, key: 45, mode: 'dorian', A: [0, 6, 3, 0], B: [3, 4, 6, 0], lead: 'square', comp: 'pulse', compInst: 'pluck', bass: 'synthbass', drums: 'beat', reverb: 0.16, density: 0.6, leadOct: 3 },
  kaffeehaus: { bpm: 156, beats: 3, key: 45, mode: 'ionian', A: [0, 0, 4, 4, 4, 4, 0, 0], B: [3, 3, 0, 0, 1, 4, 0, 4], seventh: true, lead: 'violin', comp: 'waltz', compInst: 'epiano', bass: 'bass', drums: null, reverb: 0.3, density: 0.8, leadOct: 3 },
  mossbrook:  { bpm: 92, beats: 3, key: 50, mode: 'mixolydian', A: [0, 6, 3, 0], B: [3, 0, 6, 4], lead: 'flute', alt: 'pluck', comp: 'arp', compInst: 'pluck', bass: 'sub', drums: null, reverb: 0.35, density: 0.6, leadOct: 3 },
  inn:        { bpm: 126, beats: 3, key: 50, mode: 'dorian', A: [0, 0, 6, 6, 0, 0, 3, 4], B: [2, 2, 6, 6, 3, 3, 4, 4], lead: 'violin', alt: 'flute', comp: 'waltz', compInst: 'pluck', bass: 'bass', drums: 'brush', reverb: 0.22, density: 0.85, leadOct: 3 },
  deep:       { bpm: 48, beats: 4, key: 45, mode: 'aeolian', A: [0, 5, 0, 4], B: [3, 5, 6, 4], lead: 'bell', comp: 'pad', compInst: 'pad', bass: 'sub', drums: null, reverb: 0.6, density: 0.28, leadOct: 3, soft: 0.8 },
};

// ------------------------------------------------------------------ instruments
function env(ctx, g, t, { a = 0.01, peak = 0.2, d = 0.2, s = 0, hold = 0, r = 0.2 }) {
  g.gain.setValueAtTime(0, t);
  g.gain.linearRampToValueAtTime(peak, t + a);
  if (s > 0) { g.gain.setTargetAtTime(peak * s, t + a, d / 3); g.gain.setValueAtTime(peak * s, t + a + Math.max(d, hold)); g.gain.setTargetAtTime(0, t + a + Math.max(d, hold), r / 3); }
  else g.gain.setTargetAtTime(0, t + a, d / 3);
}
const stopAfter = (o, t) => { o.start(t[0]); o.stop(t[1]); };
function osc(ctx, type, f, t0, t1, detune = 0) { const o = ctx.createOscillator(); o.type = type; o.frequency.setValueAtTime(f, t0); o.detune.value = detune; stopAfter(o, [t0, t1]); return o; }

const INST = {
  // FM electric piano: sine carrier, decaying sine modulator at 1x plus a bit of 14x tine
  epiano(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + dur + 1.5;
    const car = osc(ctx, 'sine', f, t, end), mod = osc(ctx, 'sine', f, t, end), mg = ctx.createGain();
    mg.gain.setValueAtTime(f * 1.4 * v, t); mg.gain.setTargetAtTime(f * 0.25 * v, t, 0.15);
    mod.connect(mg).connect(car.frequency);
    const tine = osc(ctx, 'sine', f * 14, t, t + 0.3), tg = ctx.createGain(); env(ctx, tg, t, { a: 0.002, peak: 0.02 * v, d: 0.08 }); tine.connect(tg).connect(out);
    const g = ctx.createGain(); env(ctx, g, t, { a: 0.004, peak: 0.16 * v, d: 1.2, s: 0.35, hold: dur, r: 0.5 }); car.connect(g).connect(out);
  },
  bell(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + 3.5;
    const car = osc(ctx, 'sine', f, t, end), mod = osc(ctx, 'sine', f * 3.5, t, end), mg = ctx.createGain();
    mg.gain.setValueAtTime(f * 2 * v, t); mg.gain.setTargetAtTime(0, t, 0.4); mod.connect(mg).connect(car.frequency);
    const g = ctx.createGain(); env(ctx, g, t, { a: 0.003, peak: 0.12 * v, d: 2.8 }); car.connect(g).connect(out);
  },
  celesta(ctx, out, t, m, dur, v) { INST.bell(ctx, out, t, m + 12, dur, v * 0.8); INST.bell(ctx, out, t + 0.002, m + 24, dur, v * 0.15); },
  musicbox(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + 2.2;
    for (const [h, a] of [[1, 1], [3.9, 0.25], [8.1, 0.08]]) { const o = osc(ctx, 'sine', f * h, t, end), g = ctx.createGain(); env(ctx, g, t, { a: 0.002, peak: 0.2 * v * a, d: 1.6 / h ** 0.5 }); o.connect(g).connect(out); }
  },
  marimba(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + 1.2;
    const o = osc(ctx, 'sine', f, t, end), g = ctx.createGain(); env(ctx, g, t, { a: 0.003, peak: 0.2 * v, d: 0.55 }); o.connect(g).connect(out);
    const o2 = osc(ctx, 'sine', f * 4, t, t + 0.2), g2 = ctx.createGain(); env(ctx, g2, t, { a: 0.001, peak: 0.07 * v, d: 0.06 }); o2.connect(g2).connect(out);
  },
  koto(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + 2.4;
    const o = osc(ctx, 'sawtooth', f, t, end), o2 = osc(ctx, 'triangle', f * 2, t, end);
    o.frequency.setValueAtTime(f * 1.012, t); o.frequency.exponentialRampToValueAtTime(f, t + 0.06);
    const lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.Q.value = 2; lp.frequency.setValueAtTime(f * 9, t); lp.frequency.exponentialRampToValueAtTime(f * 1.6, t + 0.6);
    const g = ctx.createGain(); env(ctx, g, t, { a: 0.002, peak: 0.13 * v, d: 1.6 });
    const g2 = ctx.createGain(); g2.gain.value = 0.25; o2.connect(g2).connect(lp);
    o.connect(lp).connect(g).connect(out);
  },
  pluck(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + 1.0;
    const o = osc(ctx, 'triangle', f, t, end), o2 = osc(ctx, 'square', f, t, end, 4);
    const lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.setValueAtTime(f * 6, t); lp.frequency.exponentialRampToValueAtTime(f * 1.2, t + 0.35);
    const g = ctx.createGain(); env(ctx, g, t, { a: 0.002, peak: 0.09 * v, d: 0.7 }); const g2 = ctx.createGain(); g2.gain.value = 0.3;
    o.connect(lp); o2.connect(g2).connect(lp); lp.connect(g).connect(out);
  },
  // breathy wind voices: sine core, a little 2nd/3rd harmonic, delayed vibrato, noise breath
  flute(ctx, out, t, m, dur, v, opt = {}) {
    const f = mtof(m), end = t + dur + 0.4;
    const o = osc(ctx, 'sine', f, t, end), o2 = osc(ctx, 'sine', f * 2, t, end), o3 = osc(ctx, 'sine', f * 3, t, end);
    const vib = osc(ctx, 'sine', opt.vibRate ?? 5.2, t, end), vg = ctx.createGain();
    vg.gain.setValueAtTime(0, t); vg.gain.linearRampToValueAtTime(f * (opt.vib ?? 0.008), t + Math.min(0.4, dur * 0.6));
    vib.connect(vg); vg.connect(o.frequency); vg.connect(o2.frequency);
    const g = ctx.createGain(); env(ctx, g, t, { a: opt.attack ?? 0.07, peak: 0.11 * v, d: 0.3, s: 0.85, hold: dur, r: 0.25 });
    const h2 = ctx.createGain(); h2.gain.value = opt.h2 ?? 0.18; const h3 = ctx.createGain(); h3.gain.value = opt.h3 ?? 0.06;
    o.connect(g); o2.connect(h2).connect(g); o3.connect(h3).connect(g); g.connect(out);
    const n = noiseSrc(ctx, t, end), bp = ctx.createBiquadFilter(); bp.type = 'bandpass'; bp.frequency.value = f * 2; bp.Q.value = 1.2;
    const ng = ctx.createGain(); env(ctx, ng, t, { a: 0.03, peak: (opt.breath ?? 0.03) * v, d: 0.25, s: 0.4, hold: dur, r: 0.2 }); n.connect(bp).connect(ng).connect(out);
  },
  shaku(ctx, out, t, m, dur, v) {
    // shakuhachi: breathier, a slow scoop up into the note
    INST.flute(ctx, out, t, m, dur, v * 0.9, { breath: 0.09, vib: 0.012, vibRate: 4.2, attack: 0.18, h2: 0.1, h3: 0.03 });
  },
  clarinet(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + dur + 0.4;
    const o = osc(ctx, 'square', f, t, end), lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = f * 3.2; lp.Q.value = 0.7;
    const vib = osc(ctx, 'sine', 5, t, end), vg = ctx.createGain(); vg.gain.setValueAtTime(0, t); vg.gain.linearRampToValueAtTime(f * 0.006, t + 0.35); vib.connect(vg).connect(o.frequency);
    const g = ctx.createGain(); env(ctx, g, t, { a: 0.05, peak: 0.07 * v, d: 0.2, s: 0.8, hold: dur, r: 0.15 });
    o.connect(lp).connect(g).connect(out);
  },
  violin(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + dur + 0.6;
    const o = osc(ctx, 'sawtooth', f, t, end), o2 = osc(ctx, 'sawtooth', f, t, end, 7);
    const vib = osc(ctx, 'sine', 5.6, t, end), vg = ctx.createGain(); vg.gain.setValueAtTime(0, t); vg.gain.linearRampToValueAtTime(f * 0.009, t + 0.3); vib.connect(vg); vg.connect(o.frequency); vg.connect(o2.frequency);
    const lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = Math.min(6000, f * 5); lp.Q.value = 0.8;
    const body = ctx.createBiquadFilter(); body.type = 'peaking'; body.frequency.value = 2800; body.gain.value = 4;
    const g = ctx.createGain(); env(ctx, g, t, { a: 0.09, peak: 0.05 * v, d: 0.3, s: 0.85, hold: dur, r: 0.3 });
    o.connect(lp); o2.connect(lp); lp.connect(body).connect(g).connect(out);
  },
  calliope(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + dur + 0.2;
    const o = osc(ctx, 'triangle', f, t, end), o2 = osc(ctx, 'square', f * 2, t, end);
    const vib = osc(ctx, 'sine', 6.5, t, end), vg = ctx.createGain(); vg.gain.value = f * 0.01; vib.connect(vg); vg.connect(o.frequency);
    const lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = f * 4;
    const g = ctx.createGain(); env(ctx, g, t, { a: 0.02, peak: 0.1 * v, d: 0.15, s: 0.7, hold: dur * 0.85, r: 0.08 });
    const g2 = ctx.createGain(); g2.gain.value = 0.15;
    o.connect(lp); o2.connect(g2).connect(lp); lp.connect(g).connect(out);
  },
  accordion(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + dur + 0.2;
    const o = osc(ctx, 'sawtooth', f, t, end, -6), o2 = osc(ctx, 'sawtooth', f, t, end, 6);
    const bp = ctx.createBiquadFilter(); bp.type = 'lowpass'; bp.frequency.value = Math.min(4000, f * 4);
    const g = ctx.createGain(); env(ctx, g, t, { a: 0.03, peak: 0.035 * v, d: 0.1, s: 0.8, hold: dur * 0.8, r: 0.08 });
    o.connect(bp); o2.connect(bp); bp.connect(g).connect(out);
  },
  square(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + dur + 0.3;
    const o = osc(ctx, 'square', f, t, end), lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.setValueAtTime(f * 6, t); lp.frequency.exponentialRampToValueAtTime(f * 2, t + 0.2);
    const g = ctx.createGain(); env(ctx, g, t, { a: 0.005, peak: 0.045 * v, d: 0.2, s: 0.5, hold: dur * 0.8, r: 0.1 });
    o.connect(lp).connect(g).connect(out);
  },
  pad(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + dur + 2.5;
    const lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = Math.min(2200, f * 3); lp.Q.value = 0.5;
    for (const d of [-9, 0, 8]) osc(ctx, 'sawtooth', f, t, end, d).connect(lp);
    const g = ctx.createGain(); env(ctx, g, t, { a: Math.min(1.6, dur * 0.4), peak: 0.022 * v, d: 0.5, s: 0.9, hold: dur, r: 1.6 });
    lp.connect(g).connect(out);
  },
  bass(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + dur + 0.3;
    const o = osc(ctx, 'triangle', f, t, end), s = osc(ctx, 'sine', f / 2, t, end);
    const g = ctx.createGain(); env(ctx, g, t, { a: 0.008, peak: 0.2 * v, d: 0.4, s: 0.5, hold: dur * 0.8, r: 0.1 });
    const sg = ctx.createGain(); sg.gain.value = 0.4; o.connect(g); s.connect(sg).connect(g); g.connect(out);
  },
  tuba(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + dur + 0.2;
    const o = osc(ctx, 'sawtooth', f, t, end), lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.setValueAtTime(f * 5, t); lp.frequency.exponentialRampToValueAtTime(f * 2, t + 0.15);
    const g = ctx.createGain(); env(ctx, g, t, { a: 0.02, peak: 0.12 * v, d: 0.15, s: 0.6, hold: dur * 0.6, r: 0.08 });
    o.connect(lp).connect(g).connect(out);
  },
  sub(ctx, out, t, m, dur, v) {
    const o = osc(ctx, 'sine', mtof(m), t, t + dur + 1.5), g = ctx.createGain(); env(ctx, g, t, { a: 0.3, peak: 0.13 * v, d: 0.5, s: 0.8, hold: dur, r: 1.0 }); o.connect(g).connect(out);
  },
  synthbass(ctx, out, t, m, dur, v) {
    const f = mtof(m), end = t + dur + 0.2;
    const o = osc(ctx, 'sawtooth', f, t, end), lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.Q.value = 4; lp.frequency.setValueAtTime(f * 8, t); lp.frequency.exponentialRampToValueAtTime(f * 1.5, t + 0.18);
    const g = ctx.createGain(); env(ctx, g, t, { a: 0.004, peak: 0.12 * v, d: 0.2, s: 0.4, hold: dur * 0.7, r: 0.05 });
    o.connect(lp).connect(g).connect(out);
  },
};
let noiseBuf = null;
function noiseSrc(ctx, t0, t1) {
  if (!noiseBuf || noiseBuf.sampleRate !== ctx.sampleRate) { noiseBuf = ctx.createBuffer(1, ctx.sampleRate * 2, ctx.sampleRate); const d = noiseBuf.getChannelData(0); for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1; }
  const n = ctx.createBufferSource(); n.buffer = noiseBuf; n.loop = true; n.start(t0); n.stop(t1); return n;
}
const DRUM = {
  hat(ctx, out, t, v) { const n = noiseSrc(ctx, t, t + 0.1), hp = ctx.createBiquadFilter(); hp.type = 'highpass'; hp.frequency.value = 7000; const g = ctx.createGain(); env(ctx, g, t, { a: 0.001, peak: 0.05 * v, d: 0.04 }); n.connect(hp).connect(g).connect(out); },
  brush(ctx, out, t, v) { const n = noiseSrc(ctx, t, t + 0.3), bp = ctx.createBiquadFilter(); bp.type = 'bandpass'; bp.frequency.value = 3500; bp.Q.value = 0.6; const g = ctx.createGain(); env(ctx, g, t, { a: 0.02, peak: 0.035 * v, d: 0.18 }); n.connect(bp).connect(g).connect(out); },
  kick(ctx, out, t, v) { const o = osc(ctx, 'sine', 120, t, t + 0.4); o.frequency.exponentialRampToValueAtTime(42, t + 0.12); const g = ctx.createGain(); env(ctx, g, t, { a: 0.002, peak: 0.35 * v, d: 0.25 }); o.connect(g).connect(out); },
  snare(ctx, out, t, v) { const n = noiseSrc(ctx, t, t + 0.25), bp = ctx.createBiquadFilter(); bp.type = 'bandpass'; bp.frequency.value = 1800; const g = ctx.createGain(); env(ctx, g, t, { a: 0.001, peak: 0.08 * v, d: 0.12 }); n.connect(bp).connect(g).connect(out); },
  wood(ctx, out, t, v) { const o = osc(ctx, 'sine', 1100, t, t + 0.1); o.frequency.exponentialRampToValueAtTime(800, t + 0.05); const g = ctx.createGain(); env(ctx, g, t, { a: 0.001, peak: 0.08 * v, d: 0.04 }); o.connect(g).connect(out); },
  tick(ctx, out, t, v) { const o = osc(ctx, 'triangle', 2400, t, t + 0.05), g = ctx.createGain(); env(ctx, g, t, { a: 0.001, peak: 0.025 * v, d: 0.02 }); o.connect(g).connect(out); },
};

// ------------------------------------------------------------------ composer
const RHYTHMS = {
  4: [[2, 2, 2, 2], [3, 1, 2, 2], [2, 1, 1, 4], [1, 1, 2, 2, 2], [4, 2, 2], [2, 2, 4], [3, 3, 2], [6, 2], [2, 2, 1, 1, 2]],
  3: [[2, 2, 2], [3, 1, 2], [4, 2], [2, 4], [1, 1, 2, 2], [2, 1, 1, 2]],
};
const SLOW = { 4: [[8], [4, 4], [6, 2], [2, 6], [4, 2, 2]], 3: [[6], [4, 2], [2, 4], [3, 3]] };

/** Writes one 8-bar section: chords per bar and melody notes (in eighths). Pure. */
export function composeSection(style, letter, r, motifs) {
  const S = STYLES[style];
  const prog = letter === 'B' ? S.B : S.A;
  const bars = 8, eighths = S.beats * 2;
  const chords = []; for (let b = 0; b < bars; b++) chords.push(prog[Math.floor(b * prog.length / bars) % prog.length]);
  // motif: rhythm + scale-step contour for 2 bars, remembered per section letter
  if (!motifs[letter]) {
    const pool = S.density < 0.5 ? SLOW[S.beats] : RHYTHMS[S.beats];
    const rh = [r.pick(pool), r.pick(pool)];
    const steps = rh.flat().map((_, i) => (i === 0 ? 0 : r.pick([-2, -1, -1, 1, 1, 2, 0, 3, -3, 4, -2])));
    motifs[letter] = { rh, steps };
  }
  const M = motifs[letter];
  const notes = [];
  const n = MODES[S.mode].length;
  // each phrase leans toward a target height, so the line rises to a peak and comes home
  const arc = letter === 'B' ? [3, 5, 2, 0] : [0, 2, 4, 0];
  let cur = n + 2;
  for (let phrase = 0; phrase < 4; phrase++) {
    const last = phrase === 3, varied = phrase === 1 || phrase === 3;
    cur = Math.round((cur + n + 1 + arc[phrase]) / 2);
    let k = 0;
    for (let pb = 0; pb < 2; pb++) {
      const bar = phrase * 2 + pb;
      const root = chords[bar];
      const tones = chordTones(S.mode, root, 3).map((x) => x + n); // chord tones in the lead octave
      let pos = 0;
      const rh = varied && pb === 1 ? r.pick(RHYTHMS[S.beats]) : M.rh[pb];
      for (let i = 0; i < rh.length; i++, k++) {
        const dur = rh[i];
        // rests make sparse styles breathe
        if (pos > 0 && r() > S.density + 0.25) { pos += dur; continue; }
        const strong = pos === 0 || (S.beats === 4 && pos === 4);
        let step = M.steps[k % M.steps.length];
        if (varied && r() < 0.35) step = r.pick([-1, 1, 2, -2]);
        cur += step;
        if (strong) { // snap to the nearest chord tone
          let best = tones[0], bd = 99;
          for (const tn of tones) for (const o of [-n, 0, n]) { const d = Math.abs(tn + o - cur); if (d < bd) { bd = d; best = tn + o; } }
          cur = best;
        }
        cur = Math.max(n - 2, Math.min(n * 2 + 2, cur));
        let d2 = dur;
        if (last && pb === 1 && i === rh.length - 1) { cur = tones[0]; d2 = Math.max(dur, 4); }
        notes.push({ bar, pos, dur: d2, idx: cur, vel: strong ? 1 : 0.8 });
        pos += dur;
      }
    }
  }
  return { chords, notes, letter };
}
const FORM = ['A', 'A', 'B', 'A'];

// ------------------------------------------------------------------ player
function makeReverb(ctx, seconds) {
  const len = Math.floor(ctx.sampleRate * seconds), buf = ctx.createBuffer(2, len, ctx.sampleRate);
  for (let c = 0; c < 2; c++) { const d = buf.getChannelData(c); for (let i = 0; i < len; i++) d[i] = (Math.random() * 2 - 1) * (1 - i / len) ** 2.6; }
  const cv = ctx.createConvolver(); cv.buffer = buf; return cv;
}

export function createMusic({ ctx, dest, style, seed = 1, volume = 0.5 }) {
  const S = STYLES[style];
  const spb = 60 / S.bpm, eighth = spb / 2, barLen = spb * S.beats;
  const out = ctx.createGain(); out.gain.value = 0;
  const dry = ctx.createGain(); dry.gain.value = 1 - S.reverb * 0.5;
  const rev = makeReverb(ctx, 1.5 + S.reverb * 4), wet = ctx.createGain(); wet.gain.value = S.reverb;
  const comp = ctx.createDynamicsCompressor(); comp.threshold.value = -18; comp.ratio.value = 3;
  const bus = ctx.createGain();
  bus.connect(dry).connect(comp); bus.connect(rev).connect(wet).connect(comp); comp.connect(out).connect(dest);
  const r = rng(seed);
  const motifs = {};
  let section = null, sectionIdx = 0, barInSection = 0, nextBarT = 0, timer = null, running = false;
  const level = volume * (S.soft ?? 1);

  const at = (bar0, pos) => Math.max(0, bar0 + pos * eighth + (S.swing && pos % 2 === 1 ? S.swing * eighth : 0) + (r() - 0.5) * 0.012);
  // scale index n (one octave up) lands at key + 12*leadOct - 12: the middle of the lead's range
  const pitch = (idx, octShift = 0) => S.key + 12 * (S.leadOct + octShift) + degree(S.mode, idx) - 24;
  const chordBase = S.key + (S.key < 50 ? 12 : 0);
  function playBar(t0) {
    if (!section || barInSection >= 8) {
      // after each full A A B A, write a new bridge; every third time a new main tune too
      if (sectionIdx > 0 && sectionIdx % FORM.length === 0) { delete motifs.B; if (sectionIdx % (FORM.length * 3) === 0) delete motifs.A; }
      section = composeSection(style, FORM[sectionIdx % FORM.length] + '', r, motifs); sectionIdx++; barInSection = 0;
    }
    const b = barInSection++;
    const root = section.chords[b];
    const n = MODES[S.mode].length;
    const tones = chordTones(S.mode, root, S.seventh ? 4 : 3);
    const chordMidi = tones.map((i) => chordBase + degree(S.mode, i));
    const bassMidi = S.key - 12 + degree(S.mode, root) + (degree(S.mode, root) > 7 ? -12 : 0);
    const leadInst = S.alt && section.letter === 'B' ? S.alt : S.lead;
    // lead
    for (const nt of section.notes) if (nt.bar === b) INST[leadInst](ctx, bus, at(t0, nt.pos), pitch(nt.idx), nt.dur * eighth * 0.95, nt.vel);
    // accompaniment
    const ci = INST[S.compInst];
    const E = S.beats * 2;
    switch (S.comp) {
      case 'stabs': for (const p of [2, 6].filter((p) => p < E)) for (const m of chordMidi) ci(ctx, bus, at(t0, p), m, eighth * 1.6, 0.45); break;
      case 'waltz': for (const p of [2, 4]) for (const m of chordMidi.slice(0, 3)) ci(ctx, bus, at(t0, p), m + 12 * (S.compInst === 'calliope' ? 0 : 0), eighth * 1.4, 0.4); break;
      case 'oompah': for (const p of [2, 6]) for (const m of chordMidi.slice(0, 3)) ci(ctx, bus, at(t0, p), m, eighth * 1.5, 0.6); break;
      case 'arp': { const seq = [...chordMidi, chordMidi[1] + 12, chordMidi[2]]; for (let p = 0; p < E; p++) if (p % 2 === 0 || r() < 0.4) ci(ctx, bus, at(t0, p), seq[p % seq.length], eighth * 1.8, 0.35); break; }
      case 'pad': for (const m of chordMidi) ci(ctx, bus, t0, m, barLen * 0.98, 0.9); break;
      case 'drone': { if (b % 2 === 0) for (const m of [chordMidi[0] - 12, chordMidi[0] - 5]) ci(ctx, bus, t0, m, barLen * 2, 0.8); if (r() < 0.5) INST.koto(ctx, bus, at(t0, 4), chordMidi[2] - 12, eighth * 3, 0.35); break; }
      case 'pulse': for (let p = 0; p < E; p++) ci(ctx, bus, at(t0, p), chordMidi[[0, 2, 1, 2][p % 4]] + 12, eighth * 0.8, 0.3); break;
      default: break;
    }
    // bass
    if (S.bass) {
      const bi = INST[S.bass];
      if (S.comp === 'waltz' || S.comp === 'oompah') { bi(ctx, bus, at(t0, 0), bassMidi, eighth * 1.6, 0.9); if (S.beats === 4) bi(ctx, bus, at(t0, 4), bassMidi + 7, eighth * 1.6, 0.8); }
      else if (S.bass === 'synthbass') { for (let p = 0; p < E; p++) if (p % 2 === 0 || p === E - 1) bi(ctx, bus, at(t0, p), bassMidi + (p === 6 ? 12 : 0), eighth * 0.9, p === 0 ? 1 : 0.7); }
      else if (S.bass === 'sub') { if (b % 2 === 0) bi(ctx, bus, t0, bassMidi, barLen * 2, 0.9); }
      else { bi(ctx, bus, at(t0, 0), bassMidi, eighth * 3, 1); if (S.beats === 4) bi(ctx, bus, at(t0, 4), bassMidi + (r() < 0.5 ? 7 : 0), eighth * 3, 0.85); }
    }
    // drums
    const E2 = S.beats * 2;
    switch (S.drums) {
      case 'brush': for (let p = 0; p < E2; p++) DRUM.brush(ctx, bus, at(t0, p), p % 4 === 2 ? 1 : 0.45); break;
      case 'wood': DRUM.wood(ctx, bus, at(t0, 2), 1); DRUM.wood(ctx, bus, at(t0, 6), 1); for (let p = 0; p < E2; p++) DRUM.hat(ctx, bus, at(t0, p), 0.35); break;
      case 'beat': DRUM.kick(ctx, bus, at(t0, 0), 1); DRUM.kick(ctx, bus, at(t0, 4), 0.9); DRUM.snare(ctx, bus, at(t0, 2), 0.6); DRUM.snare(ctx, bus, at(t0, 6), 0.6); for (let p = 0; p < E2; p++) DRUM.hat(ctx, bus, at(t0, p), p % 2 ? 0.35 : 0.6); break;
      case 'tick': for (const p of [0, 2, 4]) DRUM.tick(ctx, bus, at(t0, p), p === 0 ? 1 : 0.5); break;
      default: break;
    }
    void n;
  }
  function pump() {
    while (nextBarT < ctx.currentTime + 0.35) { playBar(Math.max(nextBarT, ctx.currentTime + 0.05)); nextBarT = Math.max(nextBarT, ctx.currentTime + 0.05) + barLen; }
  }
  return {
    style,
    ctx,
    start() {
      if (running) return;
      running = true;
      nextBarT = ctx.currentTime + 0.1;
      out.gain.cancelScheduledValues(ctx.currentTime);
      out.gain.setValueAtTime(out.gain.value, ctx.currentTime);
      out.gain.linearRampToValueAtTime(level, ctx.currentTime + 3);
      pump();
      if (typeof setInterval === 'function' && !(ctx instanceof OfflineAudioContext)) timer = setInterval(pump, 60);
    },
    stop() {
      if (!running) return;
      running = false;
      out.gain.cancelScheduledValues(ctx.currentTime);
      out.gain.setValueAtTime(out.gain.value, ctx.currentTime);
      out.gain.linearRampToValueAtTime(0, ctx.currentTime + 0.8);
      clearInterval(timer); timer = null;
    },
    /** Quietly step back while the game plays something the player must hear. */
    duck(seconds = 2, to = 0.15) {
      if (!running) return;
      const t = ctx.currentTime;
      out.gain.cancelScheduledValues(t); out.gain.setValueAtTime(out.gain.value, t);
      out.gain.linearRampToValueAtTime(level * to, t + 0.15);
      out.gain.setValueAtTime(level * to, t + seconds);
      out.gain.linearRampToValueAtTime(level, t + seconds + 1.2);
    },
    /** Offline use: schedule everything up to `seconds` at once. */
    scheduleAll(seconds) { running = true; out.gain.value = level; nextBarT = 0; while (nextBarT < seconds) { playBar(nextBarT); nextBarT += barLen; } },
    get running() { return running; },
  };
}

/** Note ranges a style actually produces over a full A A B A form (for checks). */
export function describe(style, seed = 1) {
  const S = STYLES[style]; const r = rng(seed); const motifs = {};
  let lo = 999, hi = -999, notes = 0;
  for (const L of FORM) for (const nt of composeSection(style, L, r, motifs).notes) {
    const m = S.key + 12 * S.leadOct + degree(S.mode, nt.idx) - 24 + (S.lead === 'celesta' ? 12 : 0);
    lo = Math.min(lo, m); hi = Math.max(hi, m); notes++;
  }
  return { lo, hi, notes, bars: FORM.length * 8, seconds: +(FORM.length * 8 * S.beats * 60 / S.bpm).toFixed(1) };
}

/** Render a style offline and measure it: used by scripts/music-check.ts. */
export async function renderOffline(style, seconds = 24, seed = 1) {
  const sr = 22050;
  const ctx = new OfflineAudioContext(2, sr * seconds, sr);
  const m = createMusic({ ctx, dest: ctx.destination, style, seed });
  m.scheduleAll(seconds - 2);
  const buf = await ctx.startRendering();
  const L = buf.getChannelData(0), R = buf.getChannelData(1);
  let peak = 0, sum = 0, nan = 0, silentSec = 0;
  const win = sr;
  for (let i = 0; i < L.length; i++) { const v = Math.max(Math.abs(L[i]), Math.abs(R[i])); if (Number.isNaN(v)) nan++; else { peak = Math.max(peak, v); sum += L[i] * L[i]; } }
  for (let s = 2; s < seconds - 3; s++) { let e = 0; for (let i = s * win; i < (s + 1) * win; i++) e += L[i] * L[i]; if (Math.sqrt(e / win) < 0.002) silentSec++; }
  return { style, peak: +peak.toFixed(3), rms: +Math.sqrt(sum / L.length).toFixed(4), nan, silentSec, buffer: buf };
}

// ------------------------------------------------------------------ page convenience
const ON_KEY = 'gamecraft.music.on';
/**
 * For gc.js pages: starts after the first gesture (browser rule), never in ?verify=1,
 * wires a #music-btn if the page has one and the M key, remembers the choice.
 */
export function setupMusic(style, { seed, sfx, volume } = {}) {
  const verify = new URLSearchParams(location.search).has('verify');
  const stub = { duck() {}, start() {}, stop() {}, get on() { return false; } };
  if (verify) return stub;
  let on = true;
  try { on = localStorage.getItem(ON_KEY) !== '0'; } catch { /* storage blocked */ }
  let m = null;
  const btn = document.getElementById('music-btn');
  const paint = () => { if (btn) { btn.classList.toggle('on', on); btn.title = on ? 'Music on (M)' : 'Music off (M)'; } };
  const ensure = () => {
    if (m) return m;
    let ctx = sfx?.ctx;
    if (!ctx) { try { ctx = new AudioContext(); } catch { return null; } if (sfx) sfx.ctx = ctx; }
    m = createMusic({ ctx, dest: ctx.destination, style, seed: seed ?? Math.floor(Math.random() * 1e6), volume });
    return m;
  };
  const apply = () => { const mm = ensure(); if (!mm) return; if (mm.ctx.state === 'suspended') mm.ctx.resume(); if (on) mm.start(); else mm.stop(); paint(); };
  const first = () => { removeEventListener('pointerdown', first, true); removeEventListener('keydown', first, true); setTimeout(apply, 0); };
  addEventListener('pointerdown', first, true); addEventListener('keydown', first, true);
  const toggle = () => { on = !on; try { localStorage.setItem(ON_KEY, on ? '1' : '0'); } catch { /* ignore */ } apply(); };
  btn?.addEventListener('click', (e) => { e.stopPropagation(); toggle(); });
  addEventListener('keydown', (e) => { if ((e.key === 'm' || e.key === 'M') && !(e.target instanceof HTMLSelectElement || e.target instanceof HTMLInputElement)) toggle(); });
  document.addEventListener('visibilitychange', () => { if (!m) return; if (document.hidden) m.stop(); else if (on) m.start(); });
  paint();
  return { duck: (s, to) => m?.duck(s, to), start: () => { on = true; apply(); }, stop: () => { on = false; apply(); }, get on() { return on; } };
}

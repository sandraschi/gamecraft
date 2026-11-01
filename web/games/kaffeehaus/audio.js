// Kaffeehaus sound: everything synthesised with the Web Audio API, no asset files.
// Ambience (room murmur, machine hiss), one-shot effects, and a salon waltz from the
// shared music engine (violin, e-piano, bass; ../_shared/music.js, style 'kaffeehaus').
// Nothing is created until init() runs from a user gesture (browser autoplay rules).
import { createMusic } from '../_shared/music.js';

const midi = (n) => 440 * Math.pow(2, (n - 69) / 12);

export function createAudio() {
  let ctx = null, master, sfxBus, musicBus, ambBus, noise;
  let murmur = null, hiss = null;
  let sfxOn = true, musicOn = true;
  let band = null;
  const last = new Map();

  function init() {
    if (ctx) { if (ctx.state === 'suspended') ctx.resume(); return; }
    const AC = window.AudioContext || window.webkitAudioContext;
    if (!AC) return;
    ctx = new AC();
    master = ctx.createGain(); master.gain.value = 0.8; master.connect(ctx.destination);
    sfxBus = ctx.createGain(); sfxBus.connect(master);
    musicBus = ctx.createGain(); musicBus.gain.value = 0.5; musicBus.connect(master);
    ambBus = ctx.createGain(); ambBus.connect(sfxBus);
    // two seconds of white noise, reused by every noisy sound
    noise = ctx.createBuffer(1, ctx.sampleRate * 2, ctx.sampleRate);
    const d = noise.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
    // room murmur: noise through two voice-band filters, level follows the crowd
    murmur = loopNoise([{ type: 'bandpass', frequency: 320, Q: 0.8 }, { type: 'lowpass', frequency: 1200, Q: 0.5 }], 0);
    // espresso machine hiss, level follows the brewing groups in use
    hiss = loopNoise([{ type: 'highpass', frequency: 2600, Q: 0.7 }], 0);
    band = createMusic({ ctx, dest: musicBus, style: 'kaffeehaus', seed: 1873, volume: 0.9 });
  }

  function loopNoise(filters, level) {
    const src = ctx.createBufferSource();
    src.buffer = noise; src.loop = true;
    let node = src;
    for (const f of filters) {
      const bq = ctx.createBiquadFilter();
      bq.type = f.type; bq.frequency.value = f.frequency; bq.Q.value = f.Q;
      node.connect(bq); node = bq;
    }
    const g = ctx.createGain(); g.gain.value = level;
    node.connect(g); g.connect(ambBus);
    src.start();
    return g;
  }

  // ---------------------------------------------------------------- building blocks
  function tone(freq, { at = 0, dur = 0.3, type = 'sine', vol = 0.2, attack = 0.005, glide = null, bus = sfxBus } = {}) {
    const t = ctx.currentTime + at;
    const o = ctx.createOscillator(), g = ctx.createGain();
    o.type = type; o.frequency.setValueAtTime(freq, t);
    if (glide) o.frequency.exponentialRampToValueAtTime(glide, t + dur);
    g.gain.setValueAtTime(0, t);
    g.gain.linearRampToValueAtTime(vol, t + attack);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    o.connect(g); g.connect(bus);
    o.start(t); o.stop(t + dur + 0.05);
  }
  function burst({ at = 0, dur = 0.2, vol = 0.2, type = 'bandpass', freq = 1000, Q = 1, sweep = null } = {}) {
    const t = ctx.currentTime + at;
    const src = ctx.createBufferSource(); src.buffer = noise;
    const f = ctx.createBiquadFilter(); f.type = type; f.frequency.setValueAtTime(freq, t); f.Q.value = Q;
    if (sweep) f.frequency.exponentialRampToValueAtTime(sweep, t + dur);
    const g = ctx.createGain();
    g.gain.setValueAtTime(0, t); g.gain.linearRampToValueAtTime(vol, t + 0.01); g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    src.connect(f); f.connect(g); g.connect(sfxBus);
    src.start(t, Math.random()); src.stop(t + dur + 0.05);
  }

  const SOUNDS = {
    bell() { // the door bell
      tone(1318, { dur: 1.3, vol: 0.09 }); tone(1975, { dur: 0.9, vol: 0.05 }); tone(2637, { dur: 0.4, vol: 0.03 });
      tone(1318, { at: 0.12, dur: 1.0, vol: 0.06 });
    },
    chair() { burst({ dur: 0.28, vol: 0.12, freq: 260, Q: 2, sweep: 180 }); },
    clink() { // cup on saucer on marble
      tone(2640, { dur: 0.12, vol: 0.08, type: 'triangle' }); tone(3960, { dur: 0.08, vol: 0.04 });
      tone(2480, { at: 0.07, dur: 0.15, vol: 0.06, type: 'triangle' });
    },
    paper() { for (let i = 0; i < 4; i++) burst({ at: i * 0.07, dur: 0.12, vol: 0.07, freq: 2400 + i * 300, Q: 0.8 }); },
    coin() { // a small cash register
      burst({ dur: 0.08, vol: 0.08, type: 'highpass', freq: 3000 });
      tone(1568, { at: 0.05, dur: 0.25, vol: 0.06, type: 'triangle' }); tone(2093, { at: 0.12, dur: 0.5, vol: 0.07, type: 'triangle' });
    },
    grumble() { // someone leaves angry
      tone(120, { dur: 0.5, vol: 0.12, type: 'sawtooth', glide: 80 });
      burst({ at: 0.05, dur: 0.4, vol: 0.06, freq: 300, Q: 3 });
    },
    upset() { tone(440, { dur: 0.35, vol: 0.06, type: 'triangle', glide: 330 }); },
    happy() { tone(660, { dur: 0.2, vol: 0.05, type: 'triangle' }); tone(880, { at: 0.1, dur: 0.3, vol: 0.05, type: 'triangle' }); },
    bill() { tone(988, { dur: 0.35, vol: 0.05 }); tone(1319, { at: 0.09, dur: 0.4, vol: 0.04 }); },
    steam() { burst({ dur: 1.4, vol: 0.06, type: 'highpass', freq: 2000, Q: 0.5 }); },
    click() { tone(1800, { dur: 0.04, vol: 0.03, type: 'square' }); },
    closing() { [988, 784, 659, 494].forEach((f, i) => tone(f, { at: i * 0.22, dur: 0.9, vol: 0.06, type: 'triangle' })); },
  };

  return {
    init,
    get ready() { return !!ctx; },
    play(name) {
      if (!ctx || !sfxOn || !SOUNDS[name]) return;
      const now = ctx.currentTime;
      if (now - (last.get(name) ?? -1) < 0.12) return; // no machine-gun effects at 4x
      last.set(name, now);
      SOUNDS[name]();
    },
    // called every frame: crowd size 0..1, brewing groups in use 0..1, playing = not paused
    update({ crowd = 0, brewing = 0, playing = true } = {}) {
      if (!ctx) return;
      const t = ctx.currentTime;
      const amb = sfxOn && playing ? 1 : 0;
      murmur.gain.setTargetAtTime(amb * (0.015 + crowd * 0.07) * (0.85 + 0.15 * Math.sin(t * 0.7)), t, 0.4);
      hiss.gain.setTargetAtTime(amb * brewing * 0.025, t, 0.15);
      if (musicOn && playing) band.start(); else band.stop();
    },
    setSfx(v) { sfxOn = v; },
    setMusic(v) { musicOn = v; },
    get sfx() { return sfxOn; },
    get music() { return musicOn; },
  };
}

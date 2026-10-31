// Mossbrook sound: synthesised with the Web Audio API, no asset files.
// Place-based ambience, footsteps by surface, the troll's snore and the stir
// heartbeat, story effects, and a quiet lute tune outdoors.
// Nothing is created until init() runs from a user gesture.

const midi = (n) => 440 * Math.pow(2, (n - 69) / 12);
// D dorian, a slow walking tune: degree offsets from D4 per eighth note, null = rest
const TUNE = [0, 3, 5, 7, 5, 3, null, 2, 0, -2, 0, 3, 5, null, 3, null,
  7, 9, 10, 9, 7, 5, null, 3, 5, 7, 5, 3, 2, 0, null, null];
const DORIAN = [0, 2, 3, 5, 7, 9, 10];
const scaleNote = (deg) => {
  if (deg === null) return null;
  const oct = Math.floor(deg / 7), i = ((deg % 7) + 7) % 7;
  return 62 + oct * 12 + DORIAN[i];
};

export function createAudio() {
  let ctx = null, master, sfx, music, amb, noise;
  const loops = {};
  let sfxOn = true, musicOn = true;
  let nextNote = 0, noteIdx = 0;
  let nextBird = 0, nextCricket = 0, nextDrip = 0, nextCrackle = 0, nextSnore = 0, nextBeat = 0;
  const last = new Map();

  function init() {
    if (ctx) { if (ctx.state === 'suspended') ctx.resume(); return; }
    const AC = window.AudioContext || window.webkitAudioContext;
    if (!AC) return;
    ctx = new AC();
    master = ctx.createGain(); master.gain.value = 0.8; master.connect(ctx.destination);
    sfx = ctx.createGain(); sfx.connect(master);
    music = ctx.createGain(); music.gain.value = 0.45; music.connect(master);
    amb = ctx.createGain(); amb.connect(sfx);
    noise = ctx.createBuffer(1, ctx.sampleRate * 2, ctx.sampleRate);
    const d = noise.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
    loops.wind = loopNoise([{ type: 'lowpass', f: 500, q: 0.4 }]);
    loops.river = loopNoise([{ type: 'bandpass', f: 900, q: 0.6 }, { type: 'lowpass', f: 2500, q: 0.4 }]);
    loops.murmur = loopNoise([{ type: 'bandpass', f: 330, q: 0.9 }]);
    loops.drone = loopTone(55, 'sine');
    loops.hum = loopTone(110, 'sine', 110.7);
    nextNote = ctx.currentTime + 0.5;
  }
  function loopNoise(filters) {
    const src = ctx.createBufferSource(); src.buffer = noise; src.loop = true;
    let node = src;
    for (const f of filters) { const b = ctx.createBiquadFilter(); b.type = f.type; b.frequency.value = f.f; b.Q.value = f.q; node.connect(b); node = b; }
    const g = ctx.createGain(); g.gain.value = 0; node.connect(g); g.connect(amb);
    src.start(0, Math.random() * 1.5);
    return g;
  }
  function loopTone(f, type, f2) {
    const g = ctx.createGain(); g.gain.value = 0; g.connect(amb);
    for (const freq of f2 ? [f, f2] : [f]) { const o = ctx.createOscillator(); o.type = type; o.frequency.value = freq; o.connect(g); o.start(); }
    return g;
  }
  function tone(freq, { at = 0, dur = 0.3, type = 'sine', vol = 0.2, attack = 0.005, glide = null, bus = sfx, detune = 0 } = {}) {
    const t = ctx.currentTime + at;
    const o = ctx.createOscillator(), g = ctx.createGain();
    o.type = type; o.frequency.setValueAtTime(freq, t); o.detune.value = detune;
    if (glide) o.frequency.exponentialRampToValueAtTime(glide, t + dur);
    g.gain.setValueAtTime(0, t); g.gain.linearRampToValueAtTime(vol, t + attack); g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    o.connect(g); g.connect(bus); o.start(t); o.stop(t + dur + 0.05);
  }
  function burst({ at = 0, dur = 0.2, vol = 0.2, type = 'bandpass', f = 1000, q = 1, sweep = null, bus = sfx } = {}) {
    const t = ctx.currentTime + at;
    const src = ctx.createBufferSource(); src.buffer = noise;
    const b = ctx.createBiquadFilter(); b.type = type; b.frequency.setValueAtTime(f, t); b.Q.value = q;
    if (sweep) b.frequency.exponentialRampToValueAtTime(sweep, t + dur);
    const g = ctx.createGain();
    g.gain.setValueAtTime(0, t); g.gain.linearRampToValueAtTime(vol, t + 0.01); g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    src.connect(b); b.connect(g); g.connect(bus); src.start(t, Math.random()); src.stop(t + dur + 0.05);
  }

  const STEP = {
    grass: () => burst({ dur: 0.09, vol: 0.05, f: 1800, q: 0.7 }),
    gravel: () => { burst({ dur: 0.07, vol: 0.06, type: 'highpass', f: 2500, q: 0.5 }); burst({ at: 0.02, dur: 0.05, vol: 0.04, f: 4000, q: 2 }); },
    wood: () => { tone(140, { dur: 0.09, vol: 0.09, type: 'triangle', glide: 110 }); burst({ dur: 0.04, vol: 0.03, f: 800, q: 2 }); },
    stone: () => { tone(220, { dur: 0.05, vol: 0.05, type: 'triangle', glide: 160 }); burst({ dur: 0.05, vol: 0.05, type: 'highpass', f: 1800, q: 1 }); },
  };
  const SOUNDS = {
    talk() { tone(523, { dur: 0.12, vol: 0.05, type: 'triangle' }); tone(659, { at: 0.06, dur: 0.15, vol: 0.04, type: 'triangle' }); },
    objective() { [587, 740, 880].forEach((f, i) => tone(f, { at: i * 0.09, dur: 0.6, vol: 0.05, type: 'triangle' })); },
    ember() { burst({ dur: 0.5, vol: 0.12, f: 600, q: 0.6, sweep: 2200 }); tone(220, { dur: 0.4, vol: 0.05, type: 'sawtooth', glide: 330 }); },
    glimmer() { [1568, 2093, 2637, 3136].forEach((f, i) => tone(f, { at: i * 0.05, dur: 0.6, vol: 0.03 })); },
    hush() { tone(330, { dur: 1.2, vol: 0.06, glide: 220 }); burst({ dur: 0.9, vol: 0.04, type: 'lowpass', f: 600, q: 0.3 }); },
    reveal() { for (let i = 0; i < 8; i++) tone(1200 + i * 220, { at: i * 0.06, dur: 0.8, vol: 0.025 }); burst({ at: 0.3, dur: 0.8, vol: 0.06, type: 'lowpass', f: 300, q: 0.5 }); },
    burn() { for (let i = 0; i < 14; i++) burst({ at: i * 0.07 + Math.random() * 0.05, dur: 0.06, vol: 0.08, f: 1500 + Math.random() * 2500, q: 1.5 }); burst({ dur: 1.3, vol: 0.08, type: 'highpass', f: 1200, q: 0.5 }); },
    toss() { tone(70, { dur: 0.8, vol: 0.25, type: 'sine', glide: 40 }); burst({ dur: 0.6, vol: 0.15, type: 'lowpass', f: 400, q: 0.5 }); burst({ at: 0.35, dur: 0.25, vol: 0.1, f: 900, q: 0.8 }); },
    song() { // Odo sings. Badly, by design.
      const notes = [62, 66, 64, 69, 67, 71, 62, 74, 73, 69];
      notes.forEach((n, i) => {
        const f = midi(n) * (1 + (Math.random() - 0.5) * 0.06);
        tone(f, { at: i * 0.32, dur: 0.38, vol: 0.07, type: 'sawtooth', detune: (Math.random() - 0.5) * 80, glide: f * (0.94 + Math.random() * 0.1) });
      });
      tone(55, { at: 3.6, dur: 2.5, vol: 0.12, type: 'sine', glide: 45 }); // the Keeper wakes
    },
    ending() {
      [62, 66, 69, 74, 78, 81].forEach((n, i) => tone(midi(n), { at: i * 0.18, dur: 3, vol: 0.05, type: 'triangle' }));
      [1318, 1760, 1318, 1976].forEach((f, i) => tone(f, { at: 1.2 + i * 0.5, dur: 2.2, vol: 0.04 }));
    },
  };

  function playTune() {
    const spn = 60 / 84 / 2; // eighth notes at 84 bpm
    while (nextNote < ctx.currentTime + 0.25) {
      const deg = TUNE[noteIdx % TUNE.length];
      const n = scaleNote(deg);
      const t = nextNote - ctx.currentTime;
      if (n !== null) {
        tone(midi(n), { at: t, dur: 0.9, vol: 0.04, type: 'triangle', bus: music });
        tone(midi(n) * 2, { at: t, dur: 0.25, vol: 0.01, bus: music });
      }
      if (noteIdx % 8 === 0) tone(midi(50 + (Math.floor(noteIdx / 16) % 2 ? 5 : 0)), { at: t, dur: 2.4, vol: 0.035, type: 'sine', bus: music });
      nextNote += spn;
      noteIdx++;
    }
  }

  return {
    init,
    get ready() { return !!ctx; },
    play(name) {
      if (!ctx || !sfxOn || !SOUNDS[name]) return;
      if (ctx.currentTime - (last.get(name) ?? -1) < 0.12) return;
      last.set(name, ctx.currentTime);
      SOUNDS[name]();
    },
    step(surface) { if (ctx && sfxOn && STEP[surface]) STEP[surface](); },
    // s: { layer, indoors, night, dusk, river, fire, ring, crowd, troll, trollAwake, stir, playing }
    update(s) {
      if (!ctx) return;
      const t = ctx.currentTime;
      const on = sfxOn && s.playing ? 1 : 0;
      const above = s.layer === 'above';
      const set = (g, v, tc = 0.5) => g.gain.setTargetAtTime(on * v, t, tc);
      set(loops.wind, above && !s.indoors ? 0.05 + 0.02 * Math.sin(t * 0.3) : 0);
      set(loops.river, above ? 0.12 * s.river : 0);
      set(loops.murmur, above && s.indoors ? 0.02 + 0.04 * s.crowd : 0);
      set(loops.drone, s.layer === 'cave' ? 0.05 : s.layer === 'below' ? 0.025 : 0);
      set(loops.hum, above ? 0.05 * s.ring : 0);
      if (!on) { nextNote = t + 0.3; return; }
      // birds by day, crickets at dusk and night, outdoors only
      if (above && !s.indoors && !s.night && !s.dusk && t > nextBird) {
        nextBird = t + 1.5 + Math.random() * 4;
        const f = 2400 + Math.random() * 1600;
        for (let i = 0; i < 2 + Math.floor(Math.random() * 3); i++) tone(f * (1 + i * 0.08), { at: i * 0.09, dur: 0.08, vol: 0.02, glide: f * 1.3 });
      }
      if (above && !s.indoors && (s.night || s.dusk) && t > nextCricket) {
        nextCricket = t + 0.6 + Math.random() * 1.2;
        for (let i = 0; i < 4; i++) tone(4400 + Math.random() * 300, { at: i * 0.045, dur: 0.03, vol: 0.012, type: 'square' });
      }
      // water dripping underground
      if (!above && t > nextDrip) {
        nextDrip = t + 0.8 + Math.random() * 2.5;
        tone(1200 + Math.random() * 900, { dur: 0.12, vol: 0.04, glide: 600 });
      }
      // fire crackle by any hearth, torch or campfire in view
      if (s.fire > 0.05 && t > nextCrackle) {
        nextCrackle = t + 0.05 + Math.random() * 0.3 / s.fire;
        burst({ dur: 0.04, vol: 0.06 * s.fire, f: 2000 + Math.random() * 3000, q: 2 });
      }
      // the Keeper: a snore you can hear across the cave, a heartbeat as he stirs
      if (s.layer === 'cave' && !s.trollAwake && t > nextSnore) {
        nextSnore = t + 3.4;
        const v = 0.05 + 0.15 * s.troll;
        burst({ dur: 1.6, vol: v, type: 'lowpass', f: 220, q: 1.5, sweep: 120 });
        tone(48, { dur: 1.5, vol: v * 0.7, type: 'sawtooth', glide: 40 });
      }
      if (s.stir > 0.25 && t > nextBeat) {
        nextBeat = t + 0.9 - s.stir * 0.5;
        tone(58, { dur: 0.16, vol: 0.12 * s.stir }); tone(52, { at: 0.17, dur: 0.18, vol: 0.09 * s.stir });
      }
      if (musicOn && above && !s.indoors) playTune();
      else nextNote = t + 0.3;
    },
    setSfx(v) { sfxOn = v; }, setMusic(v) { musicOn = v; },
    get sfx() { return sfxOn; }, get music() { return musicOn; },
  };
}

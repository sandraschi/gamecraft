// Shared harness for the calm-games batch (no game.json here, so the arcade and
// the verifier skip this folder; games import it as ../_shared/gc.js).
//
// It owns the hook contract from web/README.md so each game only supplies:
//   tick(dt)   fixed 1/60 s simulation step (animation, physics, timers)
//   bot(dt)    the scripted tour; called before tick while autoplay is on
//   render()   draw one frame
//   checks     [{ name, when(), check() }] asserted in order during autoplay
//   selfTest() optional pure design check
//
// Verify mode (?verify=1): no own loop, no audio, no saves, overlays hidden.
import * as THREE from 'three';

export const params = new URLSearchParams(location.search);
export const VERIFY = params.has('verify');
export const AUTO = params.has('autoplay');
export const DT = 1 / 60;
export const $ = (id) => document.getElementById(id);

if (VERIFY) document.documentElement.classList.add('verify');

// ------------------------------------------------------------------ random
export function rng(seed) {
  let a = seed >>> 0;
  const r = () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  r.int = (n) => Math.floor(r() * n);
  r.range = (lo, hi) => lo + r() * (hi - lo);
  r.pick = (arr) => arr[Math.floor(r() * arr.length)];
  r.shuffle = (arr) => {
    const a2 = arr.slice();
    for (let i = a2.length - 1; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [a2[i], a2[j]] = [a2[j], a2[i]]; }
    return a2;
  };
  return r;
}
export function seedFromUrl(fallback) {
  if (VERIFY) return fallback;
  const s = Number(params.get('seed'));
  return Number.isFinite(s) && s > 0 ? s : 1 + Math.floor(Math.random() * 1e9);
}

// ------------------------------------------------------------------ storage
export function load(key, def) {
  if (VERIFY) return def;
  try { const raw = localStorage.getItem(key); return raw ? { ...def, ...JSON.parse(raw) } : def; } catch { return def; }
}
export function save(key, val) {
  if (VERIFY) return;
  try { localStorage.setItem(key, JSON.stringify(val)); } catch { /* private mode */ }
}

// ------------------------------------------------------------------ rendering
export function makeRenderer(canvas, { shadows = false, clear = 0x101014 } = {}) {
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: !VERIFY, preserveDrawingBuffer: true });
  renderer.setPixelRatio(VERIFY ? 1 : Math.min(devicePixelRatio, 2));
  renderer.setClearColor(clear);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.05;
  if (shadows) { renderer.shadowMap.enabled = true; renderer.shadowMap.type = THREE.PCFSoftShadowMap; }
  return renderer;
}

/** Keeps renderer + perspective camera matched to the window. onResize(w, h) is optional. */
export function autoResize(renderer, camera, onResize) {
  const fn = () => {
    const w = innerWidth, h = innerHeight;
    renderer.setSize(w, h, false);
    if (camera) { camera.aspect = w / h; camera.updateProjectionMatrix(); }
    if (onResize) onResize(w, h);
  };
  addEventListener('resize', fn);
  fn();
  return fn;
}

const _ray = new THREE.Raycaster();
const _ndc = new THREE.Vector2();
/** Raycast from client pixel coords against objects. rect defaults to the whole window. */
export function pickAt(x, y, camera, objects, rect = { left: 0, top: 0, width: innerWidth, height: innerHeight }) {
  _ndc.set(((x - rect.left) / rect.width) * 2 - 1, -((y - rect.top) / rect.height) * 2 + 1);
  _ray.setFromCamera(_ndc, camera);
  return _ray.intersectObjects(objects, true);
}
export function rayAt(x, y, camera, rect = { left: 0, top: 0, width: innerWidth, height: innerHeight }) {
  _ndc.set(((x - rect.left) / rect.width) * 2 - 1, -((y - rect.top) / rect.height) * 2 + 1);
  _ray.setFromCamera(_ndc, camera);
  return _ray.ray;
}
/** World point -> client pixels (and whether it is in front of the camera). */
export function toScreen(v, camera, rect = { left: 0, top: 0, width: innerWidth, height: innerHeight }) {
  const p = v.clone().project(camera);
  return { x: rect.left + (p.x + 1) / 2 * rect.width, y: rect.top + (1 - p.y) / 2 * rect.height, front: p.z < 1 && p.z > -1, ndc: p };
}
export function std(color, extra = {}) { return new THREE.MeshStandardMaterial({ color, roughness: 0.8, metalness: 0, ...extra }); }
/**
 * MeshStandardMaterial from a PBR texture set, the same files the Godot side uses
 * (a TextureCraft "Godot 4 (ORM)" export): albedo, packed ORM and a tangent-space normal
 * map. ORM packs occlusion in R, roughness in G and metallic in B, which is exactly what
 * three.js reads from aoMap, roughnessMap and metalnessMap, so one image feeds all three
 * (with the roughness and metalness factors at 1, as the maps then carry the values).
 * Only the albedo is sRGB. `repeat` is tiles per UV unit; textures load lazily.
 */
export function pbr({ albedo, orm, normal }, { repeat = 1, color = 0xffffff, normalScale = 1 } = {}) {
  const loader = new THREE.TextureLoader();
  const tex = (url, srgb) => {
    const t = loader.load(url);
    t.wrapS = t.wrapT = THREE.RepeatWrapping;
    t.repeat.set(repeat, repeat);
    t.anisotropy = 4;
    if (srgb) t.colorSpace = THREE.SRGBColorSpace;
    return t;
  };
  const mat = new THREE.MeshStandardMaterial({ color, roughness: orm ? 1 : 0.8, metalness: 0 });
  if (albedo) mat.map = tex(albedo, true);
  if (orm) {
    mat.aoMap = mat.roughnessMap = mat.metalnessMap = tex(orm, false);
    mat.metalness = 1;
  }
  if (normal) {
    mat.normalMap = tex(normal, false);
    mat.normalScale.set(normalScale, normalScale);
  }
  return mat;
}

// ------------------------------------------------------------------ sound
// Tiny synth. No AudioContext until the first user gesture, never in verify mode.
export const sfx = {
  on: true,
  ctx: null,
  arm() {
    if (VERIFY) return;
    const start = () => {
      if (!this.ctx) { try { this.ctx = new AudioContext(); } catch { this.ctx = null; } }
      if (this.ctx && this.ctx.state === 'suspended') this.ctx.resume();
    };
    addEventListener('pointerdown', start, { capture: true });
    addEventListener('keydown', start, { capture: true });
  },
  tone(freq, dur = 0.25, { type = 'sine', vol = 0.15, at = 0, attack = 0.01 } = {}) {
    if (!this.on || !this.ctx) return;
    const c = this.ctx, t0 = c.currentTime + at;
    const o = c.createOscillator(), g = c.createGain();
    o.type = type; o.frequency.value = freq;
    g.gain.setValueAtTime(0, t0);
    g.gain.linearRampToValueAtTime(vol, t0 + attack);
    g.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);
    o.connect(g).connect(c.destination);
    o.start(t0); o.stop(t0 + dur + 0.05);
  },
  good() { this.tone(660, 0.18); this.tone(990, 0.3, { at: 0.09 }); },
  soft() { this.tone(330, 0.15, { type: 'triangle', vol: 0.08 }); },
  done() { [523, 659, 784, 1047].forEach((f, i) => this.tone(f, 0.6, { at: i * 0.12, vol: 0.12 })); },
};

// ------------------------------------------------------------------ ui
export function toast(msg, ms = 1800) {
  let el = $('gc-toast');
  if (!el) { el = document.createElement('div'); el.id = 'gc-toast'; document.body.appendChild(el); }
  el.textContent = msg;
  el.classList.add('show');
  clearTimeout(el._t);
  el._t = setTimeout(() => el.classList.remove('show'), ms);
}
/** Wires a #help overlay with a #help-go button and an optional #help-btn. */
export function wireHelp(onClose) {
  const help = $('help');
  if (!help) return { open: () => {}, isOpen: () => false };
  const open = () => { help.hidden = false; };
  const close = () => { help.hidden = true; if (onClose) onClose(); };
  $('help-go')?.addEventListener('click', close);
  $('help-btn')?.addEventListener('click', open);
  addEventListener('keydown', (e) => { if (e.key === 'Escape' && !help.hidden) close(); });
  if (VERIFY || AUTO) help.hidden = true;
  return { open, close, isOpen: () => !help.hidden };
}

// ------------------------------------------------------------------ harness
export function harness({ checks, tick, bot, render, extraState, selfTest, winNote, stuckSteps = 36000, paused }) {
  const st = { ready: false, steps: 0, checkpoints: [], log: [], won: false, failed: null };
  let auto = false;
  let next = 0;
  let sinceCheck = 0;
  const mark = (m) => { const l = `GameCraft: ${m}`; st.log.push(l); console.log(l); };

  function fixedStep() {
    if (auto && bot) bot(DT);
    tick(DT);
    st.steps++;
    if (!auto) return;
    sinceCheck++;
    if (next < checks.length && checks[next].when()) {
      const c = checks[next++];
      let ok = false;
      try { ok = !!c.check(); } catch (e) { mark(`check threw at ${c.name}: ${e.message}`); }
      if (!ok) { st.failed = `check failed at ${c.name}`; mark(st.failed); auto = false; return; }
      st.checkpoints.push({ name: c.name, step: st.steps });
      mark(`checkpoint ${c.name}`);
      sinceCheck = 0;
      if (next === checks.length) {
        st.won = true;
        if (winNote) mark(winNote());
        mark('tour complete, win');
        auto = false;
      }
    } else if (sinceCheck > stuckSteps) {
      st.failed = `autoplay stuck before ${checks[next]?.name}`;
      mark(st.failed);
      auto = false;
    }
  }

  const api = {
    version: 1,
    get ready() { return st.ready; },
    state: () => ({ ready: st.ready, steps: st.steps, won: st.won, failed: st.failed, checkpoints: st.checkpoints, log: st.log, autoplay: auto, ...(extraState ? extraState() : {}) }),
    autoplay: () => { auto = true; mark('autoplay start'); return true; },
    step: (n = 1) => {
      const seen = st.checkpoints.length;
      for (let i = 0; i < n && auto; i++) { fixedStep(); if (st.checkpoints.length !== seen) break; }
      return api.state();
    },
    render: () => { render(); return true; },
  };
  if (selfTest) api.selfTest = selfTest;
  window.__gamecraft = api;

  return {
    mark,
    isAuto: () => auto,
    start() {
      st.ready = true;
      mark('ready');
      if (VERIFY) { render(); return; }
      if (AUTO) api.autoplay();
      sfx.arm();
      let last = performance.now();
      let acc = 0;
      const frame = (now) => {
        const dt = Math.min(0.1, (now - last) / 1000);
        last = now;
        if (!(paused && paused())) {
          acc += dt;
          let n = 0;
          while (acc >= DT && n < 8) { fixedStep(); acc -= DT; n++; }
          if (n === 8) acc = 0;
        }
        render();
        requestAnimationFrame(frame);
      };
      requestAnimationFrame(frame);
    },
  };
}

// Tōrō nagashi: paper lanterns float down a night river. Hold the pointer on the
// water to breathe on them; nudge them into the cove below the shrine. Lanterns
// you miss just float on to the sea. Twelve at the shrine lights the torii.
import * as THREE from 'three';
import { VERIFY, $, seedFromUrl, makeRenderer, toScreen, rayAt, harness, sfx, toast, wireHelp, load, save } from '../_shared/gc.js';
import { setupMusic } from '../_shared/music.js';
const music = setupMusic('river', { sfx });
import * as S from './sim.js';

const SAVE_KEY = 'gamecraft.lanterns.v1';
const canvas = $('view');
const renderer = makeRenderer(canvas, { clear: 0x070b1a });
renderer.toneMappingExposure = 1.2;
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x070b1a);
scene.fog = new THREE.Fog(0x070b1a, 26, 60);
const camera = new THREE.PerspectiveCamera(44, 1, 0.1, 200);
camera.position.set(1, 14, 15.5); camera.lookAt(2.5, 0, -1.2);

scene.add(new THREE.HemisphereLight(0x5a6ab8, 0x141a24, 1.7));
const moonLight = new THREE.DirectionalLight(0xb8c8ff, 1.5); moonLight.position.set(-10, 20, -15); scene.add(moonLight);
const moon = new THREE.Mesh(new THREE.CircleGeometry(2.2, 40), new THREE.MeshBasicMaterial({ color: 0xf6f0d8, fog: false }));
moon.position.set(-18, 16, -40); scene.add(moon);

// stars
{
  const n = 600, pos = new Float32Array(n * 3);
  for (let i = 0; i < n; i++) { const a = Math.random() * Math.PI, e = Math.random() * 0.5 + 0.05; pos.set([Math.cos(a) * 90 * -1, Math.sin(e) * 70 + 5, -Math.sin(a) * 60 - 30], i * 3); }
  const g = new THREE.BufferGeometry(); g.setAttribute('position', new THREE.BufferAttribute(pos, 3));
  scene.add(new THREE.Points(g, new THREE.PointsMaterial({ color: 0xffffff, size: 0.35, fog: false })));
}

// ground and river
const ground = new THREE.Mesh(new THREE.PlaneGeometry(120, 80), new THREE.MeshStandardMaterial({ color: 0x1e3a2c, roughness: 1 }));
ground.rotation.x = -Math.PI / 2; ground.position.y = -0.05; scene.add(ground);
const rippleTex = (() => {
  const c = document.createElement('canvas'); c.width = 256; c.height = 64;
  const x = c.getContext('2d'); x.fillStyle = '#0c1a3a'; x.fillRect(0, 0, 256, 64);
  for (let i = 0; i < 70; i++) {
    x.strokeStyle = `rgba(140,170,255,${0.05 + Math.random() * 0.12})`; x.lineWidth = 1 + Math.random();
    const y = Math.random() * 64, x0 = Math.random() * 256; x.beginPath(); x.moveTo(x0, y); x.quadraticCurveTo(x0 + 12, y - 2, x0 + 24 + Math.random() * 20, y); x.stroke();
  }
  const t = new THREE.CanvasTexture(c); t.wrapS = t.wrapT = THREE.RepeatWrapping; t.colorSpace = THREE.SRGBColorSpace; return t;
})();
function riverGeometry() {
  const pos = [], uv = [], idx = [];
  const N = 120, x0 = -32, x1 = 34;
  for (let i = 0; i <= N; i++) {
    const x = x0 + (x1 - x0) * (i / N), c = S.center(x);
    pos.push(x, 0, c - S.HW - 0.15, x, 0, c + S.HW + 0.15);
    uv.push(i / N * 12, 0, i / N * 12, 1);
    if (i < N) { const a = i * 2; idx.push(a, a + 1, a + 2, a + 1, a + 3, a + 2); }
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  g.setAttribute('uv', new THREE.Float32BufferAttribute(uv, 2));
  g.setIndex(idx); g.computeVertexNormals();
  return g;
}
const waterMat = new THREE.MeshStandardMaterial({ map: rippleTex, color: 0x7a98e8, roughness: 0.15, metalness: 0.2, emissive: 0x3a5ab0, emissiveMap: rippleTex });
const river = new THREE.Mesh(riverGeometry(), waterMat); river.position.y = 0.01; scene.add(river);
const pondTex = rippleTex.clone(); pondTex.needsUpdate = true; pondTex.repeat.set(1.2, 1.2);
const pond = new THREE.Mesh(new THREE.CircleGeometry(S.COVE.r + 0.35, 40), new THREE.MeshStandardMaterial({ map: pondTex, color: 0x7a98e8, roughness: 0.1, metalness: 0.2, emissive: 0x3a5ab0, emissiveMap: pondTex }));
pond.rotation.x = -Math.PI / 2; pond.position.set(S.COVE.x, 0.012, S.COVE.z); scene.add(pond);
const waterPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);

// bank stones
{
  const geo = new THREE.SphereGeometry(1, 8, 6);
  const im = new THREE.InstancedMesh(geo, new THREE.MeshStandardMaterial({ color: 0x4a5058, roughness: 0.9 }), 260);
  const m = new THREE.Matrix4(), q = new THREE.Quaternion(), s = new THREE.Vector3(), p = new THREE.Vector3();
  let k = 0;
  for (let x = -30; x < 32 && k < 250; x += 0.55) {
    for (const side of [-1, 1]) {
      if (side < 0 && x > S.OPEN.x0 - 0.2 && x < S.OPEN.x1 + 0.2) continue;
      const c = S.center(x), r = 0.25 + Math.abs(Math.sin(x * 12.9)) * 0.3;
      p.set(x + Math.sin(x * 7.1) * 0.15, 0, c + side * (S.HW + 0.25 + r * 0.5)); s.set(r, r * 0.55, r);
      m.compose(p, q, s); im.setMatrixAt(k++, m);
    }
  }
  for (let a = 0; a < Math.PI * 2 && k < 260; a += 0.32) {
    const px = S.COVE.x + Math.cos(a) * (S.COVE.r + 0.5), pz = S.COVE.z + Math.sin(a) * (S.COVE.r + 0.5);
    if (pz > S.center(px) - S.HW - 0.2) continue;
    p.set(px, 0, pz); s.set(0.35, 0.2, 0.35); m.compose(p, q, s); im.setMatrixAt(k++, m);
  }
  im.count = k; scene.add(im);
}
// rocks in the stream
for (const rk of S.ROCKS) {
  const m = new THREE.Mesh(new THREE.DodecahedronGeometry(rk.r, 0), new THREE.MeshStandardMaterial({ color: 0x5a6068, roughness: 0.85 }));
  m.position.set(rk.x, 0.15, rk.z); m.scale.y = 0.6; m.rotation.y = rk.x; scene.add(m);
}
// torii and stone lanterns (tōrō) around the cove
const red = new THREE.MeshStandardMaterial({ color: 0xc8301e, roughness: 0.6, emissive: 0x000000 });
const torii = new THREE.Group(); torii.position.set(S.COVE.x, 0, S.COVE.z - S.COVE.r - 1.4); scene.add(torii);
for (const sx of [-1.3, 1.3]) { const p = new THREE.Mesh(new THREE.CylinderGeometry(0.16, 0.19, 3.4, 12), red); p.position.set(sx, 1.7, 0); torii.add(p); }
const kasagi = new THREE.Mesh(new THREE.BoxGeometry(3.9, 0.24, 0.34), new THREE.MeshStandardMaterial({ color: 0x1a1a1a })); kasagi.position.y = 3.5; torii.add(kasagi);
const shimaki = new THREE.Mesh(new THREE.BoxGeometry(3.5, 0.2, 0.3), red); shimaki.position.y = 3.3; torii.add(shimaki);
const nuki = new THREE.Mesh(new THREE.BoxGeometry(3.1, 0.15, 0.2), red); nuki.position.y = 2.7; torii.add(nuki);
const toriiGlow = new THREE.PointLight(0xff9a50, 0, 9, 1.5); toriiGlow.position.set(0, 2.4, 1); torii.add(toriiGlow);
const toros = [];
for (let i = 0; i < 6; i++) {
  const a = Math.PI * (1.02 + i * 0.193);
  const g = new THREE.Group(); g.position.set(S.COVE.x + Math.cos(a) * (S.COVE.r + 1.1), 0, S.COVE.z + Math.sin(a) * (S.COVE.r + 1.1) * 0.9); scene.add(g);
  const stone = new THREE.MeshStandardMaterial({ color: 0x7a7a72, roughness: 0.95 });
  const parts = [[0.22, 0.28, 0.5, 0.25], [0.08, 0.08, 0.5, 0.75]];
  for (const [rt, rb, h, y] of parts) { const m = new THREE.Mesh(new THREE.CylinderGeometry(rt, rb, h, 6), stone); m.position.y = y; g.add(m); }
  const box = new THREE.Mesh(new THREE.BoxGeometry(0.34, 0.3, 0.34), new THREE.MeshStandardMaterial({ color: 0x2a2620, emissive: 0x000000 })); box.position.y = 1.15; g.add(box);
  const roof = new THREE.Mesh(new THREE.ConeGeometry(0.36, 0.28, 4), stone); roof.position.y = 1.44; roof.rotation.y = Math.PI / 4; g.add(roof);
  toros.push(box);
}

// fireflies
const flyN = 60, flyPos = new Float32Array(flyN * 3), flySeed = [];
for (let i = 0; i < flyN; i++) flySeed.push([Math.random() * 50 - 25, Math.random() * 1.6 + 0.3, Math.random() * 18 - 12, Math.random() * 6]);
const flyGeo = new THREE.BufferGeometry(); flyGeo.setAttribute('position', new THREE.BufferAttribute(flyPos, 3));
const flies = new THREE.Points(flyGeo, new THREE.PointsMaterial({ color: 0xd8ff8a, size: 0.18, transparent: true, opacity: 0.8, blending: THREE.AdditiveBlending, depthWrite: false }));
scene.add(flies);

// lantern visuals
const glowTex = (() => {
  const c = document.createElement('canvas'); c.width = c.height = 64;
  const x = c.getContext('2d'); const g = x.createRadialGradient(32, 32, 0, 32, 32, 32);
  g.addColorStop(0, 'rgba(255,200,120,1)'); g.addColorStop(0.3, 'rgba(255,150,60,0.45)'); g.addColorStop(1, 'rgba(255,120,40,0)');
  x.fillStyle = g; x.fillRect(0, 0, 64, 64);
  return new THREE.CanvasTexture(c);
})();
const paper = new THREE.MeshStandardMaterial({ color: 0xffe0b0, emissive: 0xff9a40, emissiveIntensity: 1.6, roughness: 0.8 });
const wood = new THREE.MeshStandardMaterial({ color: 0x3a2412, roughness: 0.8 });
const views = new Map();
function lanternView() {
  const g = new THREE.Group();
  const base = new THREE.Mesh(new THREE.BoxGeometry(0.62, 0.08, 0.62), wood); base.position.y = 0.04; g.add(base);
  const body = new THREE.Mesh(new THREE.BoxGeometry(0.46, 0.5, 0.46), paper); body.position.y = 0.33; g.add(body);
  const top = new THREE.Mesh(new THREE.BoxGeometry(0.52, 0.05, 0.52), wood); top.position.y = 0.6; g.add(top);
  const glow = new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTex, blending: THREE.AdditiveBlending, depthWrite: false, transparent: true }));
  glow.scale.set(2.2, 2.2, 1); glow.position.y = 0.35; g.add(glow);
  const refl = new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTex, blending: THREE.AdditiveBlending, depthWrite: false, transparent: true, opacity: 0.45 }));
  refl.scale.set(0.9, 2.4, 1); refl.position.set(0, -0.2, 0.7); g.add(refl);
  scene.add(g);
  return g;
}
// breath ripple
const ring = new THREE.Mesh(new THREE.RingGeometry(0.9, 1, 48), new THREE.MeshBasicMaterial({ color: 0xbfd0ff, transparent: true, opacity: 0, depthWrite: false }));
ring.rotation.x = -Math.PI / 2; ring.position.y = 0.05; scene.add(ring);

// ------------------------------------------------------------------ state
let seed = seedFromUrl(808), sim = S.createSim(seed);
let lit = false, litT = 0, ringT = 0;
const meta = load(SAVE_KEY, { nights: 0, best: 0 });
let pointer = null;   // client coords while held

function gustFrom(x, y) {
  const ray = rayAt(x, y, camera); const p = new THREE.Vector3();
  return ray.intersectPlane(waterPlane, p) ? { x: p.x, z: p.z } : null;
}
canvas.addEventListener('pointerdown', (e) => { if (api.isAuto()) return; pointer = { x: e.clientX, y: e.clientY }; canvas.setPointerCapture(e.pointerId); });
canvas.addEventListener('pointermove', (e) => { if (pointer) pointer = { x: e.clientX, y: e.clientY }; });
canvas.addEventListener('pointerup', () => { pointer = null; });
canvas.addEventListener('pointercancel', () => { pointer = null; });
$('again').addEventListener('click', () => { $('lit').hidden = true; });

let prevHome = 0;
function tick(dt) {
  if (!api.isAuto()) sim.gust = pointer ? gustFrom(pointer.x, pointer.y) : null;
  S.step(sim, dt);
  if (sim.home > prevHome) {
    prevHome = sim.home;
    sfx.tone(523.25 * 2 ** ((sim.home % 6) * 2 / 12), 1.2, { vol: 0.08 });
    if (sim.home === S.GOAL) { lit = true; litT = 0; meta.nights++; meta.best = Math.max(meta.best, sim.home); save(SAVE_KEY, meta); setTimeout(() => sfx.done(), 400); }
    else if (sim.home < S.GOAL) toast(`${sim.home} of ${S.GOAL} at the shrine`);
  }
  // visuals follow the sim
  const alive = new Set();
  for (const l of sim.lanterns) {
    alive.add(l.id);
    let v = views.get(l.id); if (!v) { v = lanternView(); views.set(l.id, v); }
    v.position.set(l.x, 0.02 + Math.sin(sim.t * 1.6 + l.bob) * 0.04, l.z);
    v.rotation.y = Math.sin(sim.t * 0.4 + l.bob) * 0.4 + l.bob;
    v.rotation.z = Math.sin(sim.t * 1.3 + l.bob) * 0.05;
  }
  for (const [id, v] of views) if (!alive.has(id)) { scene.remove(v); views.delete(id); }
  rippleTex.offset.x -= dt * 0.09; pondTex.offset.x -= dt * 0.01;
  for (let i = 0; i < 6; i++) toros[i].material.emissive.setHex(sim.home >= (i + 1) * 2 ? 0xff9a40 : 0x000000);
  if (lit) {
    litT += dt;
    toriiGlow.intensity = Math.min(18, litT * 10);
    red.emissive.setRGB(Math.min(0.35, litT * 0.2), 0.04, 0);
    if (litT > 2.5 && !$('lit').dataset.shown) { $('lit').dataset.shown = '1'; $('lit-text').textContent = `${S.GOAL} lanterns rest below the torii. ${sim.passed} floated on to the sea, carrying their light further.`; if (!VERIFY) $('lit').hidden = false; }
  }
  if (sim.gust) { ring.position.x = sim.gust.x; ring.position.z = sim.gust.z; ringT = (ringT + dt * 1.5) % 1; ring.scale.setScalar(0.4 + ringT * S.GUST_R); ring.material.opacity = 0.5 * (1 - ringT); }
  else ring.material.opacity = Math.max(0, ring.material.opacity - dt * 2);
  for (let i = 0; i < flyN; i++) { const [x, y, z, p] = flySeed[i]; flyPos.set([x + Math.sin(sim.t * 0.3 + p) * 1.5, y + Math.sin(sim.t * 0.9 + p * 2) * 0.3, z + Math.cos(sim.t * 0.25 + p) * 1.2], i * 3); }
  flyGeo.attributes.position.needsUpdate = true;
  flies.material.opacity = 0.55 + Math.sin(sim.t * 3) * 0.25;
  hudT += dt; if (hudT > 0.25) { hudT = 0; hud(); }
}
let hudT = 0;
function hud() {
  $('home').textContent = `${Math.min(sim.home, S.GOAL)} / ${S.GOAL}`;
  $('home-extra').textContent = sim.home > S.GOAL ? `+${sim.home - S.GOAL}` : '';
  $('floated').textContent = `${sim.passed} floated on`;
}
function render() { renderer.render(scene, camera); }
function resize() {
  renderer.setSize(innerWidth, innerHeight, false);
  camera.aspect = innerWidth / innerHeight;
  camera.fov = camera.aspect < 1 ? 75 : 44;
  camera.updateProjectionMatrix();
}
addEventListener('resize', resize); resize();

// ------------------------------------------------------------------ tour
// First the bot watches one lantern drift past untouched (the current alone does
// not bring them home), then it breathes on them the way a player would: through
// the screen, at the water just on the river side of each lantern.
const bot = { phase: 'watch', watched: null };
function botStep() {
  if (bot.phase === 'watch') {
    sim.gust = null;
    const first = sim.lanterns.find((l) => l.id === 0);
    if (first && first.x > S.COVE.x + 3) { bot.watched = { home: first.home, x: first.x }; bot.phase = 'help'; }
    return;
  }
  const g = S.greedyGust(sim);
  if (!g) { sim.gust = null; return; }
  // go through the screen like a pointer would
  camera.updateMatrixWorld();
  const p = toScreen(new THREE.Vector3(g.x, 0, g.z), camera);
  sim.gust = gustFrom(p.x, p.y);
}
const checks = [
  { name: 'first-lantern', when: () => sim.lanterns.some((l) => l.x > -12), check: () => sim.spawned >= 1 && sim.lanterns[0].vx > 0.5 },
  { name: 'drifts-past', when: () => bot.phase === 'help', check: () => bot.watched && !bot.watched.home && sim.home === 0 },
  { name: 'first-home', when: () => sim.home >= 1, check: () => toros.every((t) => t.material.emissive.getHex() === 0) },
  { name: 'half-lit', when: () => sim.home >= 6, check: () => toros.filter((t) => t.material.emissive.getHex() !== 0).length === 3 },
  { name: 'shrine-lit', when: () => lit && litT > 2.6, check: () => sim.home >= S.GOAL && toriiGlow.intensity > 10 && $('lit').dataset.shown === '1' },
];

const help = wireHelp();
const api = harness({
  checks, tick, render, selfTest: S.selfTest, bot: botStep,
  paused: () => help.isOpen() || !$('lit').hidden,
  extraState: () => ({ home: sim.home, passed: sim.passed, spawned: sim.spawned, t: +sim.t.toFixed(1) }),
  winNote: () => `shrine lit with ${sim.home} lanterns, ${sim.passed} floated on`,
  stuckSteps: 9000,
});
if (!VERIFY && meta.nights === 0 && !new URLSearchParams(location.search).has('autoplay')) help.open();
api.start();

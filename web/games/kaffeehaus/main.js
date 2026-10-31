// Kaffeehaus: rendering, input and HUD over the pure simulation in sim.js.

import * as THREE from 'three';
import * as S from './sim.js';
import { createAudio } from './audio.js';

const params = new URLSearchParams(location.search);
const VERIFY = params.has('verify');
const SAVE_KEY = 'gamecraft.kaffeehaus.v1';
const $ = (id) => document.getElementById(id);

// ---------------------------------------------------------------- save state
function loadMeta() {
  if (VERIFY) return S.defaultMeta();
  try { const raw = localStorage.getItem(SAVE_KEY); if (raw) return { ...S.defaultMeta(), ...JSON.parse(raw) }; } catch { /* private mode */ }
  return S.defaultMeta();
}
function saveMeta(m) { if (VERIFY) return; try { localStorage.setItem(SAVE_KEY, JSON.stringify(m)); } catch { /* ignore */ } }
let meta = loadMeta();

// ---------------------------------------------------------------- sound
const audio = createAudio();
const SOUND_KEY = 'gamecraft.kaffeehaus.sound';
try { const s = JSON.parse(localStorage.getItem(SOUND_KEY) ?? 'null'); if (s) { audio.setSfx(s.sfx); audio.setMusic(s.music); } } catch { /* defaults */ }
function saveSound() { try { localStorage.setItem(SOUND_KEY, JSON.stringify({ sfx: audio.sfx, music: audio.music })); } catch { /* ignore */ } }
if (!VERIFY) {
  // browsers only allow audio after a gesture; the first click or key starts it
  addEventListener('pointerdown', () => audio.init(), true);
  addEventListener('keydown', () => audio.init(), true);
}
const EVENT_SOUND = { arrive: 'bell', money: 'coin', angry: 'grumble', upset: 'upset', happy: 'happy', bill: 'bill', order: 'click' };
let soundSeq = 0, lastBrewing = 0;
function soundEvents() {
  for (const e of sim.events) {
    if (e.seq <= soundSeq) continue;
    soundSeq = e.seq;
    if (EVENT_SOUND[e.kind]) audio.play(EVENT_SOUND[e.kind]);
  }
  const brewing = sim.brew.active.length;
  if (brewing > lastBrewing) audio.play('steam');
  lastBrewing = brewing;
  const crowd = sim.guests.filter((g) => g.state !== 'gone').length / Math.max(1, sim.tables.length);
  audio.update({ crowd: Math.min(1, crowd), brewing: brewing / sim.brew.slots, playing: speed > 0 && !state.dayOver && $('help').hidden });
}
function setSoundButtons() {
  $('sfx-btn').classList.toggle('on', audio.sfx);
  $('music-btn').classList.toggle('on', audio.music);
}
let sim = null;
let speed = 1;
let autoplay = false;

// ---------------------------------------------------------------- renderer
const canvas = $('view');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: !VERIFY, preserveDrawingBuffer: true });
renderer.setPixelRatio(VERIFY ? 1 : Math.min(devicePixelRatio, 2));
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFShadowMap;
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x1a1410);
const camera = new THREE.PerspectiveCamera(36, 1, 0.5, 200);
camera.position.set(0, 15, 15.5);
camera.lookAt(0, 0, 0.4);
const hemi = new THREE.HemisphereLight(0xfff0d8, 0x3a2a1c, 1.1);
scene.add(hemi);
const sun = new THREE.DirectionalLight(0xffe2b8, 1.6);
sun.position.set(6, 14, 10);
sun.castShadow = true;
sun.shadow.mapSize.set(2048, 2048);
Object.assign(sun.shadow.camera, { left: -11, right: 11, top: 11, bottom: -11, near: 1, far: 40 });
sun.shadow.bias = -0.0005;
scene.add(sun);

const matCache = new Map();
const mat = (c) => { let m = matCache.get(c); if (!m) { m = new THREE.MeshStandardMaterial({ color: c, roughness: 0.75, flatShading: true }); matCache.set(c, m); } return m; };
const glow = (c, i = 1.5) => new THREE.MeshStandardMaterial({ color: c, emissive: c, emissiveIntensity: i });
function box(p, w, h, d, x, y, z, c, o = {}) {
  const m = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), o.material ?? mat(c));
  m.position.set(x, y, z); m.castShadow = o.cast ?? true; m.receiveShadow = true; p.add(m); return m;
}
function cyl(p, rt, rb, h, x, y, z, c, seg = 16, o = {}) {
  const m = new THREE.Mesh(new THREE.CylinderGeometry(rt, rb, h, seg), o.material ?? mat(c));
  m.position.set(x, y, z); m.castShadow = o.cast ?? true; m.receiveShadow = true; p.add(m); return m;
}

// ---------------------------------------------------------------- the room
const room = new THREE.Group();
scene.add(room);
// parquet, two tones in a herringbone-ish checker
for (let x = -8; x < 8; x += 1) for (let z = -6; z < 6; z += 1) {
  box(room, 1, 0.1, 1, x + 0.5, -0.05, z + 0.5, (Math.floor(x) + Math.floor(z)) % 2 ? 0x8a5a34 : 0x7a4c2a, { cast: false });
}
// street outside the window wall
box(room, 22, 0.1, 4, 0, -0.06, 8.2, 0x6d6862, { cast: false });
box(room, 22, 0.12, 0.3, 0, 0, 6.35, 0x8d877c, { cast: false });
const panel = 0x5a2e22, cream = 0xe9dcc0, brass = 0xc8a040;
// back wall: wainscot, mirrors, shelves
box(room, 16, 3.6, 0.3, 0, 1.8, -6.15, cream);
box(room, 16, 1.1, 0.12, 0, 0.55, -5.95, panel);
for (const mx of [-6, -3.8]) { box(room, 1.6, 1.4, 0.06, mx, 2.2, -5.95, 0x3a4048); box(room, 1.8, 1.6, 0.04, mx, 2.2, -5.98, brass); }
// side walls
for (const sx of [-1, 1]) {
  box(room, 0.3, 3.6, 12, sx * 8.15, 1.8, 0, cream);
  box(room, 0.12, 1.1, 12, sx * 7.95, 0.55, 0, panel);
}
for (const pz of [-3.6, 2.6]) { box(room, 0.05, 1.0, 1.4, 7.9, 2.1, pz, 0x2a3a2a); box(room, 0.04, 1.15, 1.55, 7.93, 2.1, pz, brass); }
// window wall (low, so the camera sees in), with the door gap
for (const [a, b] of [[-8, -0.85], [0.85, 8]]) {
  box(room, b - a, 0.9, 0.25, (a + b) / 2, 0.45, 6.05, panel);
  box(room, b - a, 0.08, 0.35, (a + b) / 2, 0.92, 6.05, 0x3a1e16);
}
for (const px of [-0.85, 0.85]) box(room, 0.14, 2.5, 0.3, px, 1.25, 6.05, 0x3a1e16);
// counter with marble top, the machine, the vitrine, bottles
box(room, 8, 1.05, 0.8, 1, 0.52, -5.0, 0x5a3420);
box(room, 8.2, 0.08, 0.95, 1, 1.08, -5.0, 0xe8e4dc);
box(room, 8, 0.08, 0.05, 1, 0.35, -4.58, brass, { cast: false });
const machine = new THREE.Group();
machine.position.set(2.2, 1.12, -5.15);
room.add(machine);
cyl(machine, 0.45, 0.5, 0.9, 0, 0.45, 0, 0xb8b0a0);
cyl(machine, 0.47, 0.47, 0.12, 0, 0.92, 0, brass);
const dome = new THREE.Mesh(new THREE.SphereGeometry(0.42, 16, 8, 0, Math.PI * 2, 0, Math.PI / 2), mat(brass));
dome.position.y = 0.96; dome.castShadow = true; machine.add(dome);
box(machine, 0.12, 0.3, 0.12, 0, 1.5, 0, brass);
for (const gx of [-0.25, 0.25]) box(machine, 0.12, 0.14, 0.25, gx, 0.35, 0.42, 0x2a2a2a);
const machine2 = machine.clone();
machine2.position.x = 3.4;
machine2.visible = false;
room.add(machine2);
const steam = [];
for (let i = 0; i < 8; i++) {
  const s = new THREE.Mesh(new THREE.SphereGeometry(0.09, 6, 4), new THREE.MeshStandardMaterial({ color: 0xffffff, transparent: true, opacity: 0.5 }));
  s.visible = false; room.add(s); steam.push(s);
}
const vitrine = new THREE.Group();
vitrine.position.set(-1.6, 1.12, -5.0);
room.add(vitrine);
box(vitrine, 1.6, 0.06, 0.7, 0, 0.0, 0, 0xd8d0c0);
const glass = new THREE.Mesh(new THREE.BoxGeometry(1.6, 0.6, 0.7), new THREE.MeshStandardMaterial({ color: 0xcfe8ff, transparent: true, opacity: 0.18, roughness: 0.1 }));
glass.position.y = 0.33; vitrine.add(glass);
const cakeColors = { sacher: 0x3a2014, strudel: 0xd8a860, topfen: 0xf0e4c0 };
const cakeSlots = { sacher: [], strudel: [], topfen: [] };
S.CAKES.forEach((k, ci) => {
  for (let i = 0; i < 12; i++) {
    const m = box(vitrine, 0.14, 0.1, 0.14, -0.65 + (i % 6) * 0.26, 0.09 + Math.floor(i / 6) * 0.0, -0.2 + ci * 0.2, cakeColors[k], { cast: false });
    m.position.y = 0.09; m.position.z = -0.22 + ci * 0.22; m.position.x = -0.68 + (i % 12) * 0.12;
    cakeSlots[k].push(m);
  }
});
for (let i = 0; i < 14; i++) {
  cyl(room, 0.06, 0.07, 0.36, -2.5 + i * 0.5, 2.0 + (i % 2) * 0.55, -5.9, [0x3a6a3a, 0x7a2a2a, 0xc8a040, 0x2a3a6a][i % 4], 8);
}
box(room, 7.5, 0.05, 0.25, 1, 1.8, -5.9, 0x5a3420);
box(room, 7.5, 0.05, 0.25, 1, 2.35, -5.9, 0x5a3420);
// ready items on the pass, drawn as cups and plates
const passGroup = new THREE.Group();
room.add(passGroup);
// newspaper rack
const rack = new THREE.Group();
rack.position.set(-7.75, 0, -1.4);
room.add(rack);
box(rack, 0.2, 1.8, 1.4, 0, 0.9, 0, 0x5a3420);
for (let i = 0; i < 5; i++) { box(rack, 0.06, 0.04, 0.9, 0.18, 0.7 + i * 0.22, 0, 0x8a5a34); box(rack, 0.03, 0.2, 0.7, 0.2, 0.6 + i * 0.22, 0, 0xeee6d0, { cast: false }); }
// coat stand, plants, chandeliers
cyl(room, 0.05, 0.05, 1.9, 7.3, 0.95, 5.2, 0x2a1a10, 8);
for (const [cx, cz] of [[-7.4, 5.3], [7.3, -5.3]]) { cyl(room, 0.3, 0.22, 0.5, cx, 0.25, cz, 0x8a4a2a, 10); const l = new THREE.Mesh(new THREE.IcosahedronGeometry(0.5, 0), mat(0x3f7a3a)); l.position.set(cx, 0.9, cz); room.add(l); }
const lamps = [];
for (const [lx, lz] of [[-4, -1], [2, -1], [-4, 3], [3, 2.6]]) {
  const bulb = new THREE.Mesh(new THREE.SphereGeometry(0.16, 8, 6), glow(0xffd28a, 2));
  bulb.position.set(lx, 3.3, lz); room.add(bulb);
  const l = new THREE.PointLight(0xffc880, 3, 9, 1.6);
  l.position.set(lx, 3.0, lz); room.add(l); lamps.push(l);
}
// tables and chairs
const tableObjs = new Map();
const pickables = [];
function makeTable(t) {
  const g = new THREE.Group();
  g.position.set(t.x, 0, t.z);
  room.add(g);
  cyl(g, 0.48, 0.48, 0.05, 0, 0.76, 0, 0xf2eee6, 24);
  cyl(g, 0.05, 0.05, 0.72, 0, 0.38, 0, 0x1a1a1a, 8);
  cyl(g, 0.28, 0.3, 0.04, 0, 0.02, 0, 0x1a1a1a, 12);
  for (const sx of [-1, 1]) {
    const c = new THREE.Group();
    c.position.set(sx * 0.68, 0, 0);
    c.rotation.y = sx < 0 ? Math.PI / 2 : -Math.PI / 2;
    g.add(c);
    cyl(c, 0.21, 0.21, 0.05, 0, 0.46, 0, 0x3a2414, 14);
    const back = new THREE.Mesh(new THREE.TorusGeometry(0.18, 0.025, 6, 14, Math.PI), mat(0x3a2414));
    back.position.set(0, 0.62, -0.19); g.add(back); back.position.x = sx * 0.88; back.rotation.y = Math.PI / 2;
    for (const lx of [-0.13, 0.13]) for (const lz of [-0.13, 0.13]) box(c, 0.03, 0.46, 0.03, lx, 0.23, lz, 0x3a2414);
  }
  const ring = new THREE.Mesh(new THREE.RingGeometry(0.62, 0.78, 32), new THREE.MeshBasicMaterial({ color: 0xf0c040, transparent: true, opacity: 0.85, side: THREE.DoubleSide }));
  ring.rotation.x = -Math.PI / 2; ring.position.y = 0.02; ring.visible = false; g.add(ring);
  const label = document.createElement('div');
  label.className = 'tlabel'; label.textContent = t.window ? `${t.id} window` : String(t.id);
  $('labels').appendChild(label);
  const hit = new THREE.Mesh(new THREE.CylinderGeometry(0.9, 0.9, 1.2, 12), new THREE.MeshBasicMaterial({ visible: false }));
  hit.position.y = 0.6; hit.userData.table = t.id; g.add(hit); pickables.push(hit);
  const items = new THREE.Group(); g.add(items);
  tableObjs.set(t.id, { g, ring, label, items, key: '' });
}

// ---------------------------------------------------------------- characters
function makeCharacter(o) {
  const g = new THREE.Group(), body = new THREE.Group();
  g.add(body);
  const w = o.wide ?? 1;
  const add = (p, bw, bh, bd, c, x, y, z) => box(p, bw, bh, bd, x, y, z, c);
  const legL = new THREE.Group(), legR = new THREE.Group();
  legL.position.set(-0.13 * w, 0.78, 0); legR.position.set(0.13 * w, 0.78, 0);
  for (const leg of [legL, legR]) { add(leg, 0.19, 0.62, 0.21, o.legs, 0, -0.33, 0); add(leg, 0.21, 0.16, 0.28, o.boots ?? 0x1e1a16, 0, -0.7, 0.03); body.add(leg); }
  if (o.skirt) add(body, 0.56 * w, 0.5, 0.36, o.skirt, 0, 0.62, 0);
  add(body, 0.52 * w, 0.62, 0.3, o.body, 0, 1.08, 0);
  if (o.vest) add(body, 0.54 * w, 0.5, 0.32, o.vest, 0, 1.06, 0.005);
  if (o.tie) add(body, 0.08, 0.36, 0.02, o.tie, 0, 1.15, 0.16);
  if (o.bow) add(body, 0.18, 0.07, 0.03, o.bow, 0, 1.36, 0.16);
  if (o.scarf) add(body, 0.4, 0.12, 0.34, o.scarf, 0, 1.36, 0.01);
  if (o.backpack) add(body, 0.4, 0.5, 0.18, o.backpack, 0, 1.08, -0.24);
  const armL = new THREE.Group(), armR = new THREE.Group();
  armL.position.set(-0.34 * w, 1.34, 0); armR.position.set(0.34 * w, 1.34, 0);
  for (const arm of [armL, armR]) { add(arm, 0.15, 0.58, 0.17, o.sleeves ?? o.body, 0, -0.28, 0); add(arm, 0.13, 0.13, 0.13, o.skin, 0, -0.62, 0); body.add(arm); }
  const head = new THREE.Group(); head.position.y = 1.42; body.add(head);
  add(head, 0.36, 0.38, 0.36, o.skin, 0, 0.2, 0);
  for (const sx of [-0.08, 0.08]) add(head, 0.06, 0.06, 0.02, 0x1a1a1a, sx, 0.23, 0.185);
  if (o.glasses) add(head, 0.3, 0.05, 0.02, 0x1a1a1a, 0, 0.23, 0.195);
  add(head, 0.4, 0.12, 0.4, o.hair, 0, 0.42, 0);
  add(head, 0.4, 0.3, 0.1, o.hair, 0, 0.28, -0.16);
  if (o.bun) add(head, 0.18, 0.18, 0.18, o.hair, 0, 0.45, -0.2);
  if (o.longHair) add(head, 0.34, 0.5, 0.08, o.hair, 0, 0.05, -0.2);
  if (o.beard) add(head, 0.3, 0.22, 0.08, o.beard, 0, 0.06, 0.18);
  if (o.hat) { add(head, 0.5, 0.04, 0.5, o.hat, 0, 0.47, 0); add(head, 0.34, 0.22, 0.34, o.hat, 0, 0.59, 0); }
  if (o.beret) add(head, 0.44, 0.1, 0.44, o.beret, 0.04, 0.5, 0);
  if (o.camera) add(body, 0.18, 0.12, 0.08, 0x1a1a1a, 0, 1.0, 0.2);
  const tray = new THREE.Group(); tray.position.set(0, -0.55, 0.25); armR.add(tray);
  cyl(tray, 0.25, 0.25, 0.02, 0, 0, 0, 0xc0c0c0, 16);
  cyl(tray, 0.05, 0.04, 0.08, 0, 0.05, 0, 0xffffff, 8);
  tray.visible = false;
  const paper = box(armL, 0.3, 0.02, 0.4, 0, -0.62, 0.15, 0xeee6d0, { cast: false });
  paper.visible = false;
  g.scale.setScalar(o.scale ?? 1);
  return { group: g, body, legL, legR, armL, armR, tray, paper, phase: 0 };
}
function pose(ch, moving, sitting, dt) {
  if (sitting) {
    ch.legL.rotation.x = ch.legR.rotation.x = -1.45;
    ch.armL.rotation.x = ch.armR.rotation.x = -0.6;
    ch.body.position.y = -0.3;
    return;
  }
  if (moving) ch.phase += dt * 9;
  const s = moving ? Math.sin(ch.phase) * 0.6 : 0;
  const k = Math.min(1, dt * 14);
  ch.legL.rotation.x += (s - ch.legL.rotation.x) * k;
  ch.legR.rotation.x += (-s - ch.legR.rotation.x) * k;
  ch.armL.rotation.x += (-s * 0.8 - ch.armL.rotation.x) * k;
  ch.armR.rotation.x += ((ch.tray.visible ? -1.2 : s * 0.8) - ch.armR.rotation.x) * k;
  ch.body.position.y = moving ? Math.abs(Math.cos(ch.phase)) * 0.05 : 0;
}

const SKIN = [0xf0c8a8, 0xe0b090, 0xc89070, 0x9a6a4a, 0x6a4a32];
const BRIGHT = [0xd84a3a, 0x3a8ad8, 0xe8c040, 0x4ab06a, 0xd86aa8, 0xf08a30];
const REG_LOOKS = {
  weiss: { body: 0x6a6a70, legs: 0x4a4a52, hair: 0xe8e8e8, beard: 0xe8e8e8, hat: 0x2a2a2a, tie: 0x6a1a1a, skin: SKIN[0] },
  lenz: { body: 0x6a3a7a, skirt: 0x5a2a6a, legs: 0x3a2a3a, hair: 0xb0b0b0, bun: true, glasses: true, skin: SKIN[0] },
  ostrowski: { body: 0x7a5a3a, legs: 0x4a3a2a, hair: 0x5a4a3a, beard: 0x5a4a3a, wide: 1.1, skin: SKIN[1] },
  mitzi: { body: 0xb83a3a, skirt: 0x2a2a2a, legs: 0x2a2a2a, hair: 0x2a1a10, longHair: true, beret: 0x2a2a2a, skin: SKIN[0] },
  poeschl: { body: 0x2a2a34, legs: 0x2a2a34, hair: 0x3a3a3a, glasses: true, tie: 0x2a4a8a, skin: SKIN[2] },
};
function lookFor(g) {
  if (g.regular) return REG_LOOKS[g.regular.key];
  const r = g.look, skin = SKIN[r % SKIN.length], hair = [0x2a1a10, 0x6a3a1a, 0xc89a50, 0x1a1a1a][r % 4];
  if (g.type === 'tourist') return { skin, hair, body: BRIGHT[r % BRIGHT.length], legs: 0x8a7a5a, backpack: r % 2 ? 0x3a5a3a : null, camera: r % 3 === 0 };
  if (g.type === 'business') return { skin, hair, body: [0x3a3a44, 0x4a4a52, 0x2a2a3a][r % 3], legs: 0x2a2a34, tie: [0x8a1a1a, 0x1a3a8a][r % 2] };
  return { skin, hair, body: [0x5a6a3a, 0x7a5a3a, 0x3a5a6a][r % 3], legs: 0x3a4a6a, scarf: BRIGHT[(r + 2) % BRIGHT.length], longHair: r % 2 === 0 };
}
const waiterCh = makeCharacter({ skin: SKIN[1], body: 0xf4f4f0, vest: 0x1a1a1a, legs: 0x1a1a1a, hair: 0x2a1a10, bow: 0x1a1a1a, sleeves: 0xf4f4f0 });
scene.add(waiterCh.group);
const piccoloCh = makeCharacter({ skin: SKIN[0], body: 0xf4f4f0, legs: 0x1a1a1a, hair: 0xc89a50, scale: 0.88 });
scene.add(piccoloCh.group);
const guestViews = new Map();

// ---------------------------------------------------------------- day setup
function seedFor(day) { return 1 + day * 0 + (VERIFY ? 0 : Math.floor(Math.random() * 1e6)); }
function startDay() {
  sim = S.createSim({ seed: VERIFY ? 1 : seedFor(meta.day), meta, policy: autoplay ? S.greedyPolicy : null });
  for (const v of guestViews.values()) { scene.remove(v.ch.group); v.bubble.remove(); }
  guestViews.clear();
  for (const o of tableObjs.values()) { room.remove(o.g); o.label.remove(); }
  tableObjs.clear();
  pickables.length = 0;
  for (const t of sim.tables) makeTable(t);
  machine2.visible = meta.upgrades.machine;
  piccoloCh.group.visible = !!sim.piccolo;
  selected = null;
  soundSeq = 0; lastBrewing = 0;
  state.dayOver = false;
  $('dayend').hidden = true;
  updateHud(true);
}

// ---------------------------------------------------------------- input
let selected = null; // guest id waiting for a table pick
const ray = new THREE.Raycaster();
const ndc = new THREE.Vector2();
canvas.addEventListener('pointerdown', (ev) => {
  if (selected === null) return;
  ndc.set((ev.clientX / innerWidth) * 2 - 1, -(ev.clientY / innerHeight) * 2 + 1);
  ray.setFromCamera(ndc, camera);
  const hit = ray.intersectObjects(pickables, false)[0];
  if (!hit) return;
  const tid = hit.object.userData.table;
  if (sim.enqueue({ kind: 'seat', guest: selected, table: tid })) selected = null;
});
addEventListener('keydown', (ev) => {
  if (ev.code === 'Escape') { selected = null; setHelp(false); }
  if (ev.code === 'Space') { ev.preventDefault(); setSpeed(speed === 0 ? 1 : 0); }
  if (ev.code === 'KeyH') setHelp($('help').hidden);
  if (ev.code === 'KeyS') { audio.setSfx(!audio.sfx); saveSound(); setSoundButtons(); }
  if (ev.code === 'KeyM') { audio.setMusic(!audio.music); saveSound(); setSoundButtons(); }
  const sp = { Digit1: 1, Digit2: 2, Digit3: 4 }[ev.code];
  if (sp) setSpeed(sp);
});
function setSpeed(v) {
  speed = v;
  document.querySelectorAll('[data-speed]').forEach((b) => b.classList.toggle('on', Number(b.dataset.speed) === v));
}
document.querySelectorAll('[data-speed]').forEach((b) => b.addEventListener('click', () => setSpeed(Number(b.dataset.speed))));
function setHelp(v) { $('help').hidden = !v; }
$('help-btn').addEventListener('click', () => setHelp(true));
$('sfx-btn').addEventListener('click', () => { audio.setSfx(!audio.sfx); saveSound(); setSoundButtons(); });
$('music-btn').addEventListener('click', () => { audio.setMusic(!audio.music); saveSound(); setSoundButtons(); });
setSoundButtons();
$('help-close').addEventListener('click', () => setHelp(false));
$('newgame').addEventListener('click', () => {
  if (!confirm('Start over from day 1? Your café and regulars reset.')) return;
  meta = S.defaultMeta(); saveMeta(meta); startDay();
});

const ACTION_LABEL = { seat: 'Seat', order: 'Take order', serve: 'Serve', bill: 'Bill', paper: 'Newspaper', hurry: 'Rush bill' };
function onAction(g, kind) {
  audio.play('click');
  if (kind === 'seat') { selected = g.id; return; }
  sim.enqueue({ kind, guest: g.id });
}

// ---------------------------------------------------------------- per-frame sync
const proj = new THREE.Vector3();
function screenOf(x, y, z) {
  proj.set(x, y, z).project(camera);
  return [(proj.x * 0.5 + 0.5) * innerWidth, (-proj.y * 0.5 + 0.5) * innerHeight];
}
function syncGuests(dt) {
  const alive = new Set();
  for (const g of sim.guests) {
    if (g.state === 'gone') continue;
    alive.add(g.id);
    let v = guestViews.get(g.id);
    if (!v) {
      const look = lookFor(g);
      const ch = makeCharacter(look);
      const ch2 = g.size === 2 ? makeCharacter(lookFor({ ...g, look: g.look + 7 })) : null;
      scene.add(ch.group); if (ch2) scene.add(ch2.group);
      const bubble = document.createElement('div');
      bubble.className = `gb ${g.regular ? 'reg' : g.type}`;
      $('bubbles').appendChild(bubble);
      v = { ch, ch2, bubble, px: g.x, pz: g.z, key: '', prevState: g.state, prevPaper: false };
      guestViews.set(g.id, v);
    }
    if (g.state !== v.prevState) {
      if (v.prevState === 'queue' && g.state !== 'leaving') audio.play('chair');
      if (v.prevState === 'ordered' && g.state === 'eating') audio.play('clink');
      v.prevState = g.state;
    }
    if (g.hasPaper && !v.prevPaper) audio.play('paper');
    v.prevPaper = g.hasPaper;
    const moving = Math.hypot(g.x - v.px, g.z - v.pz) > 0.001;
    const sitting = ['seated', 'ordered', 'eating', 'wantsBill'].includes(g.state);
    const table = sim.tables.find((t) => t.id === g.table);
    let rot = Math.atan2(g.x - v.px, g.z - v.pz);
    if (sitting && table) rot = Math.PI / 2;
    if (!moving && !sitting) rot = Math.PI;
    v.ch.group.position.set(g.x, 0, g.z);
    v.ch.group.rotation.y = moving || sitting ? rot : v.ch.group.rotation.y;
    pose(v.ch, moving, sitting, dt);
    v.ch.paper.visible = g.hasPaper && sitting;
    if (v.ch2) {
      const p2 = sitting && table ? S.seat2Pos(table) : { x: g.x + 0.55, z: g.z + 0.25 };
      v.ch2.group.position.set(p2.x, 0, p2.z);
      v.ch2.group.rotation.y = sitting ? -Math.PI / 2 : v.ch.group.rotation.y;
      pose(v.ch2, moving, sitting, dt);
    }
    v.px = g.x; v.pz = g.z;
    // bubble: name, wish, patience, actions
    const acts = g.state === 'leaving' ? [] : sim.actionsFor(g);
    const phase = { queue: 'queue', seated: 'order', ordered: 'serve', wantsBill: 'bill' }[g.state];
    const frac = phase ? Math.min(1, (sim.t - g.since) / g.patience[phase]) : 0;
    const status = { queue: 'waiting at the door', seated: 'wants to order', ordered: g.ready === g.order.length ? 'order is ready' : 'waiting for coffee', eating: g.hasPaper ? 'reading' : 'enjoying it', wantsBill: 'wants to pay', leaving: g.mood < 30 ? 'leaving, annoyed' : 'leaving' }[g.state];
    const key = `${g.state}|${acts.join(',')}|${selected === g.id}|${g.hasPaper}|${g.ready}`;
    if (key !== v.key) {
      v.key = key;
      const wish = g.state === 'queue' || g.state === 'seated' ? g.wish.map((i) => S.MENU[i].name).join(', ') : '';
      v.bubble.innerHTML = `<div class="who"><span class="ring"></span><b></b></div><div class="st"></div>${wish ? '<div class="wish"></div>' : ''}<div class="acts"></div>`;
      v.bubble.querySelector('b').textContent = g.regular ? g.name : `${S.TYPE_LABEL[g.type]}${g.size === 2 ? ' x2' : ''}`;
      v.bubble.querySelector('.st').textContent = status;
      if (wish) v.bubble.querySelector('.wish').textContent = wish;
      const box = v.bubble.querySelector('.acts');
      for (const a of acts) {
        const b = document.createElement('button');
        b.textContent = selected === g.id && a === 'seat' ? 'pick a table...' : ACTION_LABEL[a];
        b.className = a === 'hurry' ? 'warn' : '';
        b.dataset.testid = `act-${a}-${g.id}`;
        b.addEventListener('click', (ev) => { ev.stopPropagation(); onAction(g, a); });
        box.appendChild(b);
      }
      v.bubble.classList.toggle('quiet', !acts.length);
    }
    const ring = v.bubble.querySelector('.ring');
    ring.style.background = phase ? `conic-gradient(${frac > 0.75 ? '#e05040' : frac > 0.5 ? '#e0a040' : '#60c070'} ${(1 - frac) * 360}deg, rgba(255,255,255,.15) 0)` : 'transparent';
    const [sx, sy] = screenOf(g.x, sitting ? 1.7 : 2.25, g.z);
    v.bubble.style.transform = `translate(-50%, -100%) translate(${sx}px, ${sy}px)`;
    v.bubble.style.opacity = g.state === 'leaving' ? 0.5 : 1;
  }
  for (const [id, v] of guestViews) {
    if (alive.has(id)) continue;
    scene.remove(v.ch.group); if (v.ch2) scene.remove(v.ch2.group); v.bubble.remove();
    guestViews.delete(id);
  }
}
let wPrev = { x: 0, z: 0 }, pPrev = { x: 0, z: 0 };
function syncStaff(dt) {
  for (const [w, ch, prev] of [[sim.waiter, waiterCh, wPrev], [sim.piccolo, piccoloCh, pPrev]]) {
    if (!w) continue;
    const moving = Math.hypot(w.x - prev.x, w.z - prev.z) > 0.001;
    if (moving) ch.group.rotation.y = Math.atan2(w.x - prev.x, w.z - prev.z);
    ch.group.position.set(w.x, 0, w.z);
    ch.tray.visible = w.carrying === 'tray' && w.leg >= 2;
    ch.paper.visible = w.carrying === 'paper' && w.leg >= 2;
    pose(ch, moving, false, dt);
    prev.x = w.x; prev.z = w.z;
  }
}
function syncTables() {
  for (const t of sim.tables) {
    const o = tableObjs.get(t.id);
    const free = !t.guest && !t.reserved;
    const g = sim.guests.find((x) => x.id === t.guest);
    const sel = selected !== null ? sim.guests.find((x) => x.id === selected) : null;
    o.ring.visible = !!sel && free;
    if (o.ring.visible) o.ring.material.color.setHex(sel.regular && sel.regular.table === t.id ? 0xffe070 : t.window && sel.type === 'tourist' ? 0x80d0ff : 0xf0c040);
    const [sx, sy] = screenOf(t.x, 0.9, t.z + 0.6);
    o.label.style.transform = `translate(-50%, 0) translate(${sx}px, ${sy}px)`;
    o.label.style.display = sel && free ? 'block' : 'none';
    const key = g && ['eating', 'wantsBill'].includes(g.state) ? `${g.id}:${g.billed.join(',')}:${g.hasPaper}` : '';
    if (key !== o.key) {
      o.key = key;
      o.items.clear();
      if (g && key) g.order.forEach((item, i) => {
        const a = (i / Math.max(1, g.order.length)) * Math.PI * 2;
        const px = Math.cos(a) * 0.22, pz = Math.sin(a) * 0.22;
        if (S.MENU[item].kind === 'coffee') { cyl(o.items, 0.07, 0.06, 0.1, px, 0.84, pz, 0xffffff, 10); cyl(o.items, 0.12, 0.12, 0.01, px, 0.79, pz, 0xffffff, 12); }
        else { cyl(o.items, 0.13, 0.13, 0.01, px, 0.79, pz, 0xffffff, 12); box(o.items, 0.12, 0.07, 0.1, px, 0.83, pz, cakeColors[item], { cast: false }); }
        cyl(o.items, 0.04, 0.04, 0.12, px + 0.12, 0.85, pz, 0xcfe8ff, 8); // the glass of water
      });
    }
  }
}
function syncBar(t) {
  const busy = sim.brew.active.length;
  steam.forEach((s, i) => {
    s.visible = busy > i % 4 || (i >= 4 && busy > 2);
    const ph = (t * 0.7 + i * 0.25) % 1;
    const mx = i >= 4 ? 3.4 : 2.2;
    s.position.set(mx + Math.sin(i * 2 + t) * 0.12, 2.6 + ph * 1.1, -5.1);
    s.material.opacity = 0.45 * (1 - ph);
  });
  for (const k of S.CAKES) cakeSlots[k].forEach((m, i) => { m.visible = i < sim.cakes[k]; });
  const passKey = sim.pass.map((p) => p.item).join(',');
  if (passKey !== syncBar.key) {
    syncBar.key = passKey;
    passGroup.clear();
    sim.pass.slice(0, 10).forEach((p, i) => {
      const x = -0.2 + i * 0.32;
      if (S.MENU[p.item].kind === 'coffee') cyl(passGroup, 0.07, 0.06, 0.1, x, 1.17, -4.65, 0xffffff, 10);
      else box(passGroup, 0.14, 0.07, 0.12, x, 1.16, -4.65, cakeColors[p.item], { cast: false });
    });
  }
}

// ---------------------------------------------------------------- HUD
const fmtClock = (t) => `${String(Math.floor(t / 60)).padStart(2, '0')}:${String(Math.floor(t % 60)).padStart(2, '0')}`;
const stars = (n) => '★'.repeat(Math.round(n)) + '☆'.repeat(Math.max(0, 5 - Math.round(n)));
let hudTick = 0, lastEvent = 0;
function updateHud(force = false) {
  if (!force && hudTick++ % 4) return;
  $('day').textContent = `Day ${meta.day}`;
  $('clock').textContent = fmtClock(sim.t);
  $('cash').textContent = `€ ${(meta.cash + sim.stats.revenue + sim.stats.tips).toFixed(2)}`;
  const moods = sim.stats.moods;
  const today = moods.length ? moods.reduce((a, b) => a + b, 0) / moods.length / 20 : meta.reputation;
  $('stars').textContent = stars(today);
  $('stars').title = `Today ${today.toFixed(1)} / 5, reputation ${meta.reputation.toFixed(1)}`;
  // waiter queue
  const q = $('queue');
  const rows = [];
  const name = (id) => { const g = sim.guests.find((x) => x.id === id); return g ? (g.regular ? g.name : `${S.TYPE_LABEL[g.type]} ${g.id}`) : '?'; };
  if (sim.waiter.task) rows.push({ now: true, text: `${ACTION_LABEL[sim.waiter.task.kind]}: ${(sim.waiter.task.batch ?? [sim.waiter.task.guest]).map(name).join(', ')}` });
  if (sim.piccolo?.task) rows.push({ now: true, text: `Piccolo serves: ${(sim.piccolo.task.batch ?? [sim.piccolo.task.guest]).map(name).join(', ')}` });
  sim.queue.forEach((t, i) => rows.push({ i, text: `${ACTION_LABEL[t.kind]}: ${name(t.guest)}${t.table ? ` to table ${t.table}` : ''}` }));
  const key = rows.map((r) => r.text).join('|');
  if (key !== q.dataset.key) {
    q.dataset.key = key;
    q.innerHTML = '';
    if (!rows.length) q.innerHTML = '<div class="empty">Herr Ober is free. Click a guest\'s button to give him a job.</div>';
    for (const r of rows) {
      const d = document.createElement('div');
      d.className = r.now ? 'row now' : 'row';
      d.textContent = r.text;
      if (!r.now) {
        const up = document.createElement('button'); up.textContent = '↑'; up.title = 'Do this next';
        up.onclick = () => { const [t] = sim.queue.splice(r.i, 1); sim.queue.unshift(t); };
        const x = document.createElement('button'); x.textContent = '✕'; x.title = 'Cancel';
        x.onclick = () => { const [t] = sim.queue.splice(r.i, 1); if (t.kind === 'seat') { const tb = sim.tables.find((y) => y.id === t.table); if (tb?.reserved === t.guest) tb.reserved = null; } };
        d.append(up, x);
      }
      q.appendChild(d);
    }
  }
  // bar
  const b = sim.brew;
  $('brew').innerHTML = '';
  for (let i = 0; i < b.slots; i++) {
    const job = b.active[i];
    const d = document.createElement('div');
    d.className = 'slot';
    d.innerHTML = job ? `<span></span><i style="width:${Math.round((1 - job.left / S.MENU[job.item].brew) * 100)}%"></i>` : '<span>idle</span>';
    if (job) d.querySelector('span').textContent = S.MENU[job.item].name;
    $('brew').appendChild(d);
  }
  $('brew-wait').textContent = b.waiting.length ? `${b.waiting.length} more waiting` : '';
  $('cakes').textContent = S.CAKES.map((k) => `${S.MENU[k].name} ${sim.cakes[k]}`).join('  ·  ');
  const ready = new Set(sim.pass.map((p) => p.guest)).size;
  $('pass').textContent = ready ? `${sim.pass.length} item(s) on the pass` : 'Pass is empty';
  // log
  if (sim.events.length !== lastEvent) {
    lastEvent = sim.events.length;
    $('log').innerHTML = '';
    for (const e of sim.events.slice(-6).reverse()) {
      const d = document.createElement('div');
      d.className = `ev ${e.kind}`;
      d.textContent = `${fmtClock(e.t)}  ${e.text}`;
      $('log').appendChild(d);
    }
  }
  $('pick').hidden = selected === null;
  if (selected !== null) { const g = sim.guests.find((x) => x.id === selected); if (!g || g.state !== 'queue') selected = null; else $('pick-name').textContent = g.regular ? g.name : S.TYPE_LABEL[g.type]; }
}

function showDayEnd() {
  const rep = sim.report();
  const before = meta;
  meta = S.applyDayEnd(meta, rep);
  saveMeta(meta);
  state.dayOver = true;
  state.lastReport = rep;
  audio.play('closing');
  $('de-title').textContent = `Day ${rep.day} is done`;
  $('de-stats').innerHTML = `
    <div><b>€ ${rep.revenue.toFixed(2)}</b> takings</div><div><b>€ ${rep.tips.toFixed(2)}</b> Trinkgeld</div>
    <div><b>€ ${rep.rent}</b> rent</div><div><b>€ ${rep.profit.toFixed(2)}</b> profit</div>
    <div><b>${rep.paid}</b> guests paid</div><div><b>${rep.angry}</b> gave up, <b>${rep.walkouts}</b> walked out</div>
    <div><b>${stars(rep.stars)}</b> today</div><div><b>${rep.cakesSold}</b> cakes sold</div>`;
  const regs = $('de-regs');
  regs.innerHTML = '';
  for (const r of rep.regulars) {
    const d = document.createElement('div');
    const loy = meta.loyalty[r.key];
    const delta = r.loyaltyDelta ? (r.loyaltyDelta > 0 ? ` (+${r.loyaltyDelta})` : ` (${r.loyaltyDelta})`) : '';
    d.textContent = `${r.name}: ${r.came ? (r.angry ? 'left angry' : r.hurried ? 'felt rushed' : r.mood >= 70 ? 'content' : 'so-so') : 'did not come'}  ${'♥'.repeat(loy)}${'♡'.repeat(5 - loy)}${delta}`;
    regs.appendChild(d);
  }
  const shop = $('de-shop');
  shop.innerHTML = '';
  for (const u of S.UPGRADES) {
    const b = document.createElement('button');
    const owned = meta.upgrades[u.key];
    b.disabled = owned || meta.cash < u.cost;
    b.innerHTML = `<b></b><span></span><em></em>`;
    b.querySelector('b').textContent = u.name;
    b.querySelector('span').textContent = u.desc;
    b.querySelector('em').textContent = owned ? 'owned' : `€ ${u.cost}`;
    b.onclick = () => { if (meta.cash < u.cost || meta.upgrades[u.key]) return; meta.cash -= u.cost; meta.upgrades[u.key] = true; saveMeta(meta); showShopOnly(); };
    shop.appendChild(b);
  }
  $('de-cash').textContent = `Cash now € ${meta.cash.toFixed(2)}. Reputation ${meta.reputation.toFixed(1)}, was ${before.reputation.toFixed(1)}.`;
  $('dayend').hidden = false;
}
function showShopOnly() {
  for (const [i, u] of S.UPGRADES.entries()) {
    const b = $('de-shop').children[i];
    const owned = meta.upgrades[u.key];
    b.disabled = owned || meta.cash < u.cost;
    b.querySelector('em').textContent = owned ? 'owned' : `€ ${u.cost}`;
  }
  $('de-cash').textContent = `Cash now € ${meta.cash.toFixed(2)}.`;
}
$('de-next').addEventListener('click', () => startDay());

// ---------------------------------------------------------------- loop
const state = { ready: false, dayOver: false, steps: 0, checkpoints: [], log: [], won: false, failed: null, lastReport: null };
function mark(msg) { const l = `GameCraft: ${msg}`; state.log.push(l); console.log(l); }
const CHECKS = [
  { name: 'first-seated', when: () => sim.stats.moods.length + sim.guests.filter((g) => g.table).length > 0 && sim.tables.some((t) => t.guest), check: () => sim.tables.some((t) => t.guest) },
  { name: 'morning-rush', when: () => sim.t >= 9 * 60, check: () => sim.stats.paid >= 2 && sim.tables.filter((t) => t.guest).length >= 1 },
  { name: 'midday', when: () => sim.t >= 12 * 60 + 30, check: () => sim.stats.paid >= 6 && sim.stats.revenue > 0 },
  { name: 'jause', when: () => sim.t >= 16 * 60, check: () => sim.stats.cakesSold > 0 },
  { name: 'closing', when: () => sim.t >= S.DOORS_CLOSE, check: () => sim.arrivals.length === 0 },
  { name: 'day-end', when: () => state.dayOver, check: () => !$('dayend').hidden && state.lastReport.guests > 0 },
];
let nextCheck = 0;
function simStep(dt) {
  if (state.dayOver) return;
  sim.tick(dt);
  state.steps++;
  if (sim.over) showDayEnd();
  if (autoplay && nextCheck < CHECKS.length && CHECKS[nextCheck].when()) {
    const c = CHECKS[nextCheck++];
    if (!c.check()) { state.failed = `check failed at ${c.name}`; mark(state.failed); autoplay = false; return; }
    state.checkpoints.push({ name: c.name, step: state.steps, t: fmtClock(sim.t) });
    mark(`checkpoint ${c.name}`);
    if (c.name === 'day-end') {
      const r = state.lastReport;
      if (r.stars >= 2.5 && r.walkouts <= 2) { state.won = true; mark(`day complete, ${r.stars} stars, € ${r.profit} profit`); mark('tour complete, win'); }
      else { state.failed = `day ended at ${r.stars} stars, ${r.walkouts} walkouts`; mark(state.failed); }
      autoplay = false;
    }
  }
}
function render(dt = 1 / 60) {
  soundEvents();
  syncStaff(dt); syncGuests(dt); syncTables(); syncBar(performance.now() / 1000); updateHud();
  for (const l of lamps) l.intensity = 3;
  renderer.render(scene, camera);
}
function resize() {
  renderer.setSize(innerWidth, innerHeight, false);
  camera.aspect = innerWidth / innerHeight;
  camera.fov = camera.aspect < 1 ? 58 : 36;
  camera.updateProjectionMatrix();
}
addEventListener('resize', resize);
resize();

window.__gamecraft = {
  version: 1,
  get ready() { return state.ready; },
  state: () => ({ ready: state.ready, steps: state.steps, won: state.won, failed: state.failed, checkpoints: state.checkpoints, log: state.log, autoplay, clock: fmtClock(sim.t), day: meta.day, report: state.lastReport }),
  autoplay: () => { autoplay = true; sim.policy = S.greedyPolicy; mark('autoplay start'); return true; },
  step: (n = 1) => {
    const seen = state.checkpoints.length;
    for (let i = 0; i < n && autoplay; i++) { simStep(S.TICK); if (state.checkpoints.length !== seen) break; }
    return window.__gamecraft.state();
  },
  render: () => { render(); return true; },
  selfTest: () => S.selfTest(),
};

startDay();
state.ready = true;
mark('ready');
if (!VERIFY) {
  if (params.has('autoplay')) { window.__gamecraft.autoplay(); setSpeed(4); } else if (meta.day === 1 && !meta.history.length) setHelp(true);
  let last = performance.now();
  const frame = (now) => {
    const dt = Math.min(0.1, (now - last) / 1000);
    last = now;
    if (!$('help').hidden || state.dayOver) { render(dt); requestAnimationFrame(frame); return; }
    const gameDt = dt * speed * S.MIN_PER_SEC;
    const n = Math.ceil(gameDt / S.TICK - 1e-9);
    for (let i = 0; i < n; i++) simStep(gameDt / n);
    render(dt);
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
} else {
  render();
}

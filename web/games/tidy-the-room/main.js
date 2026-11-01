// Tidy the Room: the Spot the Difference room after a lively afternoon. Six
// things ended up on the floor; the photo in the corner shows where they live.
// Drag each one home. It snaps when it is close and stays wherever else you put it.
import * as THREE from 'three';
import { VERIFY, $, rng, seedFromUrl, makeRenderer, toScreen, rayAt, harness, sfx, toast, wireHelp, load, save } from '../_shared/gc.js';
import { setupMusic } from '../_shared/music.js';
const music = setupMusic('cozy', { sfx });
import { buildRoom, roomLights, roomCamera } from '../_shared/room.js';

const N_MESS = 6;
const SNAP = 0.22;             // world units: how close counts as home
const MOVABLE = ['cushion-1', 'cushion-2', 'cushion-3', 'mug', 'teapot', 'vase', 'book-stack', 'yarn', 'slippers', 'magazines'];
const FLOOR = { x0: -2.3, x1: 2.4, z0: -1.25, z1: 1.9 };
const TABLE = { x0: -1.2, x1: 0.4, z0: -1.1, z1: -0.15 };
const SAVE_KEY = 'gamecraft.tidy.v1';

const canvas = $('view');
const renderer = makeRenderer(canvas, { shadows: !VERIFY, clear: 0x1a1612 });
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x2a221a); roomLights(scene, { shadows: !VERIFY });
const photoScene = new THREE.Scene(); photoScene.background = new THREE.Color(0x2a221a); roomLights(photoScene);
const camera = roomCamera(innerWidth / innerHeight);
const photoCam = roomCamera(4 / 3);

// invisible surfaces things can be put on: [name, w, d, x, y, z]
const surfaces = [];
for (const [name, w, d, x, y, z] of [
  ['floor', 6, 5, 0, 0, 0], ['table', 1.3, 0.66, -0.4, 0.45, -0.62], ['sofa', 1.9, 0.62, -0.45, 0.46, -1.9],
  ['sideboard', 0.45, 1.3, -2.76, 0.7, 1.2], ['sill', 1.6, 0.22, 1.55, 0.945, -2.4],
]) {
  const m = new THREE.Mesh(new THREE.PlaneGeometry(w, d), new THREE.MeshBasicMaterial({ visible: false }));
  m.rotation.x = -Math.PI / 2; m.position.set(x, y + 0.001, z); m.userData.surface = name;
  scene.add(m); surfaces.push(m);
}

let room = null, photo = null;
let mess = [];           // [{ id, label, home: {pos, rot}, done }]
let carried = null;      // { item, entry, offset }
let moves = 0, roundT = 0, tidyOver = false, round = 0;
let seed = seedFromUrl(777);
let photoBig = false;
const meta = load(SAVE_KEY, { rooms: 0, best: null });

// ------------------------------------------------------------------ setup
/** Pure: where the mess goes for a seed. Returns [{id, x, z, yaw}] on open floor,
 *  away from each item's home and inside a 4:3 view (the narrowest we design for). */
const REF_CAM = roomCamera(4 / 3);
const HOMES = (() => { const r0 = buildRoom(); const m = new Map(MOVABLE.map((id) => [id, r0.items.get(id).position.clone()])); dispose(r0.root); return m; })();
const underPhoto = (q) => q.x > 0.35 && q.y < -0.1; // bottom-right corner belongs to the photo
function scatter(s) {
  const r = rng(s);
  const _q = new THREE.Vector3();
  const ids = r.shuffle(MOVABLE).slice(0, N_MESS);
  const spots = [];
  for (const id of ids) {
    for (let k = 0; k < 200; k++) {
      const x = r.range(FLOOR.x0, FLOOR.x1), z = r.range(FLOOR.z0, FLOOR.z1);
      if (x > TABLE.x0 - 0.15 && x < TABLE.x1 + 0.15 && z > TABLE.z0 - 0.15 && z < TABLE.z1 + 0.15) continue;
      if (Math.abs(x + 0.35) < 0.45 && Math.abs(z - 0.55) < 0.35) continue; // the cat
      if (spots.some((p) => Math.hypot(p.x - x, p.z - z) < 0.5)) continue;
      const home = HOMES.get(id);
      if (Math.hypot(home.x - x, home.z - z) < SNAP * 3) continue;
      _q.set(x, 0.05, z).project(REF_CAM);
      if (Math.abs(_q.x) > 0.85 || Math.abs(_q.y) > 0.6) continue; // 0.6: still on screen when a 16:9 window crops the 4:3 view
      if (underPhoto(_q)) continue;
      spots.push({ id, x, z, yaw: r.range(-Math.PI, Math.PI), tilt: r() < 0.4 });
      break;
    }
  }
  return spots;
}

function dispose(o) { o.traverse((c) => { if (c.isMesh) { c.geometry.dispose(); c.material.dispose(); } }); }
function newRoom() {
  if (room) { scene.remove(room.root); dispose(room.root); }
  if (!photo) { photo = buildRoom(); photoScene.add(photo.root); }
  room = buildRoom(); scene.add(room.root);
  mess = [];
  for (const sp of scatter(seed)) {
    const g = room.items.get(sp.id);
    mess.push({ id: sp.id, label: g.userData.label, home: { pos: g.position.clone(), rot: g.rotation.clone() }, done: false });
    g.position.set(sp.x, g.userData.base, sp.z);
    g.rotation.set(sp.tilt ? 0.35 : 0, sp.yaw, 0);
  }
  carried = null; moves = 0; roundT = 0; tidyOver = false; round++;
  $('done').hidden = true;
  hud();
}

// ------------------------------------------------------------------ input
const _hits = [];
function itemAt(x, y) {
  const ray = rayAt(x, y, camera);
  const rc = new THREE.Raycaster(ray.origin, ray.direction);
  const objs = mess.filter((m) => !m.done).map((m) => room.items.get(m.id));
  _hits.length = 0;
  const hits = rc.intersectObjects(objs, true);
  if (!hits.length) return null;
  let o = hits[0].object;
  while (o && !o.userData.itemId) o = o.parent;
  return o;
}
function surfaceAt(x, y) {
  const ray = rayAt(x, y, camera);
  const rc = new THREE.Raycaster(ray.origin, ray.direction);
  const h = rc.intersectObjects(surfaces, false)[0];
  return h ? { point: h.point, name: h.object.userData.surface } : null;
}

function pick(x, y) {
  if (tidyOver || carried) return false;
  const g = itemAt(x, y);
  if (!g) return false;
  carried = { item: g, entry: mess.find((m) => m.id === g.userData.itemId) };
  g.rotation.set(0, carried.entry.home.rot.y + g.rotation.y * 0.0, 0);
  g.rotation.copy(carried.entry.home.rot); // picked up things right themselves
  sfx.soft();
  hover(x, y);
  return true;
}
function hover(x, y) {
  if (!carried) return;
  const s = surfaceAt(x, y);
  if (!s) return;
  const g = carried.item;
  g.position.set(s.point.x, s.point.y + g.userData.base + 0.06, s.point.z);
  const near = g.position.distanceTo(carried.entry.home.pos) < SNAP + 0.06;
  $('view').style.cursor = near ? 'copy' : 'grabbing';
}
function drop() {
  if (!carried) return null;
  const { item: g, entry } = carried;
  carried = null;
  $('view').style.cursor = '';
  g.position.y -= 0.06;
  moves++;
  if (g.position.distanceTo(entry.home.pos) < SNAP) {
    g.position.copy(entry.home.pos); g.rotation.copy(entry.home.rot);
    entry.done = true;
    sfx.good();
    toast(`The ${entry.label} is back where it belongs`);
    if (mess.every((m) => m.done)) finish();
    hud();
    return 'home';
  }
  hud();
  return 'elsewhere';
}
canvas.addEventListener('pointerdown', (e) => { if (!api.isAuto() && !inPhoto(e.clientX, e.clientY)) pick(e.clientX, e.clientY); });
addEventListener('pointermove', (e) => { if (!api.isAuto()) hover(e.clientX, e.clientY); });
addEventListener('pointerup', () => { if (!api.isAuto()) drop(); });
$('photo-frame').addEventListener('click', () => { photoBig = !photoBig; layout(); });
$('new').addEventListener('click', () => { seed = (seed * 7 + 13) % 2147483647; newRoom(); });
$('next').addEventListener('click', () => { seed = (seed * 7 + 13) % 2147483647; newRoom(); });

function finish() {
  tidyOver = true;
  sfx.done();
  meta.rooms++;
  if (meta.best === null || roundT < meta.best) meta.best = Math.round(roundT);
  save(SAVE_KEY, meta);
  const extra = moves - N_MESS;
  $('done-text').textContent = `Everything is home in ${fmt(roundT)} with ${moves} moves${extra ? ` (${extra} detour${extra > 1 ? 's' : ''})` : ', not a single detour'}.`;
  $('done').hidden = false;
}

// ------------------------------------------------------------------ layout, render
const photoRect = { left: 0, top: 0, width: 300, height: 225 };
function layout() {
  const w = innerWidth, h = innerHeight;
  const pw = Math.round((photoBig ? 0.55 : 0.24) * Math.max(w, 420));
  photoRect.width = Math.min(pw, w - 24); photoRect.height = Math.round(photoRect.width * 0.75);
  photoRect.left = w - photoRect.width - 12; photoRect.top = h - photoRect.height - 12;
  Object.assign($('photo-frame').style, { left: `${photoRect.left - 6}px`, top: `${photoRect.top - 6}px`, width: `${photoRect.width + 12}px`, height: `${photoRect.height + 12}px` });
  $('photo-frame').style.cursor = photoBig ? 'zoom-out' : 'zoom-in';
}
const inPhoto = (x, y) => x >= photoRect.left && x <= photoRect.left + photoRect.width && y >= photoRect.top && y <= photoRect.top + photoRect.height;
// keep the horizontal view about the same as the 4:3 photo so wide windows do not see past the walls
function fitCamera() {
  renderer.setSize(innerWidth, innerHeight, false);
  camera.aspect = innerWidth / innerHeight;
  const hfov = 2 * Math.atan(Math.tan(THREE.MathUtils.degToRad(48) / 2) * 4 / 3);
  camera.fov = THREE.MathUtils.clamp(THREE.MathUtils.radToDeg(2 * Math.atan(Math.tan(hfov / 2) / camera.aspect)), 34, 62);
  camera.updateProjectionMatrix();
  layout();
}
addEventListener('resize', fitCamera);

function render() {
  const H = innerHeight;
  renderer.setViewport(0, 0, innerWidth, H);
  renderer.render(scene, camera);
  renderer.setScissorTest(true);
  const r = photoRect;
  renderer.setViewport(r.left, H - r.top - r.height, r.width, r.height);
  renderer.setScissor(r.left, H - r.top - r.height, r.width, r.height);
  photoCam.aspect = r.width / r.height; photoCam.updateProjectionMatrix();
  renderer.render(photoScene, photoCam);
  renderer.setScissorTest(false);
}
const fmt = (t) => `${Math.floor(t / 60)}:${String(Math.floor(t % 60)).padStart(2, '0')}`;
function hud() {
  $('left').textContent = `${mess.filter((m) => !m.done).length} to go`;
  $('time').textContent = fmt(roundT);
  $('moves').textContent = `${moves} moves`;
}
let hudT = 0;
function tick(dt) {
  if (!tidyOver) roundT += dt;
  hudT += dt; if (hudT > 0.25) { hudT = 0; hud(); }
}

// ------------------------------------------------------------------ tour
const homeScreen = (entry) => toScreen(new THREE.Vector3(entry.home.pos.x, entry.home.pos.y - room.items.get(entry.id).userData.base, entry.home.pos.z), camera);
function itemScreen(id) {
  const g = room.items.get(id);
  const b = new THREE.Box3().setFromObject(g);
  return toScreen(b.getCenter(new THREE.Vector3()), camera);
}
const bot = { t: 0, phase: 'wrong', prevIds: '', wrongResult: null, wrongStillUntidy: false };
function botStep(dt) {
  bot.t += dt;
  if (bot.t < 0.5) return;
  bot.t = 0;
  camera.updateMatrixWorld();
  if (bot.phase === 'wrong') {
    // carry the first thing to the wrong corner of the rug: it must stay untidy
    const e = mess[0];
    const p = itemScreen(e.id);
    if (!pick(p.x, p.y)) { bot.wrongResult = 'pick failed'; bot.phase = 'tidy'; return; }
    const away = toScreen(new THREE.Vector3(e.home.pos.x > 0 ? -1.5 : 1.6, 0, 1.2), camera);
    hover(away.x, away.y);
    bot.wrongResult = drop();
    bot.wrongStillUntidy = !e.done && room.items.get(e.id).position.distanceTo(e.home.pos) > SNAP;
    bot.phase = 'tidy';
  } else if (bot.phase === 'tidy') {
    const e = mess.find((m) => !m.done);
    if (!e) { bot.phase = 'admire'; return; }
    const p = itemScreen(e.id);
    if (!pick(p.x, p.y)) { bot.stuck = e.id; return; }
    const h = homeScreen(e);
    hover(h.x, h.y);
    drop();
  } else if (bot.phase === 'admire') {
    bot.prevIds = mess.map((m) => m.id).sort().join(',');
    $('next').click();
    bot.phase = 'second';
  }
}

const checks = [
  { name: 'messy-room', when: () => true, check: () => mess.length === N_MESS && mess.every((m) => room.items.get(m.id).position.distanceTo(m.home.pos) > SNAP * 1.5) },
  { name: 'wrong-place', when: () => bot.phase !== 'wrong', check: () => bot.wrongResult === 'elsewhere' && bot.wrongStillUntidy && moves === 1 },
  { name: 'half-tidy', when: () => mess.filter((m) => m.done).length >= 3, check: () => mess.filter((m) => m.done).every((m) => room.items.get(m.id).position.equals(m.home.pos)) },
  { name: 'all-tidy', when: () => tidyOver, check: () => !$('done').hidden && mess.every((m) => m.done) && moves === N_MESS + 1 && photoMatches() },
  { name: 'next-mess', when: () => bot.phase === 'second', check: () => mess.length === N_MESS && !tidyOver && mess.map((m) => m.id).sort().join(',') !== bot.prevIds },
];
function photoMatches() {
  for (const [id, g] of photo.items) {
    const h = room.items.get(id);
    if (h.position.distanceTo(g.position) > 1e-6 || Math.abs(h.rotation.y - g.rotation.y) > 1e-6) return false;
  }
  return true;
}

function selfTest() {
  const c = { never_pre_solved: true, always_six: true, on_open_floor: true, spread_out: true, varied: true, on_screen: true };
  const sets = new Set();
  const tmp = buildRoom(); const sc = new THREE.Scene(); sc.add(tmp.root); sc.updateMatrixWorld(true);
  const cam = roomCamera(4 / 3);
  for (let s = 1; s <= 80; s++) {
    const sp = scatter(s * 131);
    if (sp.length !== N_MESS) c.always_six = false;
    sets.add(sp.map((p) => p.id).sort().join(','));
    for (const p of sp) {
      const home = tmp.items.get(p.id).position;
      if (Math.hypot(home.x - p.x, home.z - p.z) < SNAP * 1.5) c.never_pre_solved = false;
      if (p.x < FLOOR.x0 || p.x > FLOOR.x1 || p.z < FLOOR.z0 || p.z > FLOOR.z1) c.on_open_floor = false;
      const q = new THREE.Vector3(p.x, 0.05, p.z).project(cam);
      if (Math.abs(q.x) > 0.9 || Math.abs(q.y) > 0.65 || underPhoto(q)) c.on_screen = false;
    }
    for (let i = 0; i < sp.length; i++) for (let j = i + 1; j < sp.length; j++) if (Math.hypot(sp[i].x - sp[j].x, sp[i].z - sp[j].z) < 0.5) c.spread_out = false;
  }
  c.varied = sets.size >= 40;
  dispose(tmp.root);
  return { pass: Object.values(c).every(Boolean), checks: c, summary: `80 seeds, ${sets.size} distinct messes` };
}

fitCamera();
newRoom();
const help = wireHelp();
const api = harness({
  checks, tick, render, selfTest, bot: botStep,
  paused: () => help.isOpen(),
  extraState: () => ({ moves, round, left: mess.filter((m) => !m.done).length, stuck: bot.stuck ?? null }),
  winNote: () => `room 1 tidy in ${N_MESS + 1} moves (one deliberate detour), room ${round} messed up differently`,
});
if (!VERIFY && meta.rooms === 0 && !new URLSearchParams(location.search).has('autoplay')) help.open();
api.start();

// Bento: pack every piece of food into the box. Pieces turn in 90 degree steps;
// every box was cut from a full one, so there is always a way. No timer.
import * as THREE from 'three';
import { VERIFY, $, seedFromUrl, makeRenderer, toScreen, rayAt, pickAt, harness, sfx, toast, wireHelp, load, save } from '../_shared/gc.js';
import { setupMusic } from '../_shared/music.js';
const music = setupMusic('picnic', { sfx });
import { LEVELS, generate, rotate, covered, fits, selfTest } from './puzzle.js';

const SAVE_KEY = 'gamecraft.bento.v1';
const CELL = 1;
const canvas = $('view');
const renderer = makeRenderer(canvas, { shadows: !VERIFY, clear: 0xe8dcc8 });
const scene = new THREE.Scene();
scene.background = new THREE.Color(0xe8dcc8);
const camera = new THREE.PerspectiveCamera(38, 1, 0.1, 100);
scene.add(new THREE.HemisphereLight(0xfff8ee, 0x8a7a68, 1.4));
const sun = new THREE.DirectionalLight(0xfff0dc, 2.0); sun.position.set(-5, 12, 6);
if (!VERIFY) { sun.castShadow = true; sun.shadow.mapSize.set(2048, 2048); Object.assign(sun.shadow.camera, { left: -12, right: 12, top: 12, bottom: -12 }); }
scene.add(sun);

const M = (c, r = 0.7, extra = {}) => new THREE.MeshStandardMaterial({ color: c, roughness: r, ...extra });
const table = new THREE.Mesh(new THREE.PlaneGeometry(60, 40), M(0xc8a878, 0.8)); table.rotation.x = -Math.PI / 2; table.receiveShadow = true; scene.add(table);
const mat = new THREE.Mesh(new THREE.PlaneGeometry(30, 16), M(0x6a8a6a, 0.95)); mat.rotation.x = -Math.PI / 2; mat.position.y = 0.005; mat.receiveShadow = true; scene.add(mat);

// ------------------------------------------------------------------ food
const G = {
  box: new THREE.BoxGeometry(1, 1, 1), ball: new THREE.SphereGeometry(1, 16, 12), cyl: new THREE.CylinderGeometry(1, 1, 1, 16),
  cone: new THREE.ConeGeometry(1, 1, 3), tri: new THREE.CylinderGeometry(1, 1, 1, 3), capsule: new THREE.CapsuleGeometry(1, 1, 4, 12), round: new THREE.CylinderGeometry(1, 1, 1, 24),
};
const PAL = [0xf2b8c6, 0xb8d8f2, 0xf2e2a0, 0xc8e8b8, 0xe0c8f2, 0xf2d0b0, 0xb8f0e0, 0xf0c0c0];
function mesh(geo, color, sx, sy, sz, x, y, z, rough = 0.7) { const m = new THREE.Mesh(geo, M(color, rough)); m.scale.set(sx, sy, sz); m.position.set(x, y, z); m.castShadow = true; m.receiveShadow = true; return m; }
function foodCell(food, i, n) {
  const g = new THREE.Group();
  switch (food) {
    case 'rice': {
      g.add(mesh(G.box, 0xf8f6f0, 0.92, 0.42, 0.92, 0, 0.21, 0, 0.95));
      for (let k = 0; k < 5; k++) g.add(mesh(G.ball, 0x2a2420, 0.025, 0.012, 0.04, -0.3 + ((k * 37) % 60) / 100, 0.43, -0.3 + ((k * 53) % 60) / 100));
      if (i === Math.floor(n / 2)) g.add(mesh(G.ball, 0xc8203a, 0.17, 0.14, 0.17, 0, 0.46, 0, 0.4));
      break;
    }
    case 'onigiri': {
      // an upright rice triangle with a nori band at the bottom
      const tri = mesh(G.tri, 0xf8f6f0, 0.42, 0.34, 0.42, 0, 0.23, 0, 0.95); tri.rotation.x = -Math.PI / 2; g.add(tri);
      const nori = mesh(G.box, 0x1a2a1a, 0.3, 0.2, 0.36, 0, 0.11, 0, 0.6); g.add(nori);
      break;
    }
    case 'tamagoyaki': {
      g.add(mesh(G.box, 0xf6d24a, 0.9, 0.5, 0.9, 0, 0.25, 0, 0.6));
      for (let k = 0; k < 3; k++) g.add(mesh(G.box, 0xfae890, 0.91, 0.04, 0.02, 0, 0.12 + k * 0.13, 0.451));
      break;
    }
    case 'sausage': {
      const s = mesh(G.capsule, 0xd8483a, 0.16, 0.5, 0.16, 0, 0.18, 0, 0.45); s.rotation.z = Math.PI / 2; s.rotation.y = 0.4 * (i % 2 ? 1 : -1); g.add(s);
      const s2 = s.clone(); s2.position.z = 0.25; s2.position.x = 0.05; g.add(s2);
      break;
    }
    case 'edamame': {
      for (let k = 0; k < 3; k++) { const p = mesh(G.capsule, 0x6ab84a, 0.1, 0.38, 0.1, -0.25 + k * 0.25, 0.12, (k % 2) * 0.15 - 0.07, 0.6); p.rotation.x = Math.PI / 2; p.rotation.z = 0.3 * k; g.add(p); }
      break;
    }
    case 'karaage': {
      for (let k = 0; k < 2; k++) g.add(mesh(G.ball, 0xa8642a, 0.3, 0.24, 0.3, -0.12 + k * 0.24, 0.24, (k ? 0.12 : -0.12), 0.95));
      if (i === 0) { const lemon = mesh(G.round, 0xf6e04a, 0.18, 0.04, 0.18, 0.25, 0.47, 0.2, 0.5); lemon.rotation.x = 1.1; g.add(lemon); }
      break;
    }
    case 'salmon': {
      g.add(mesh(G.box, 0xf2885a, 0.88, 0.32, 0.88, 0, 0.16, 0, 0.5));
      for (let k = 0; k < 3; k++) { const l = mesh(G.box, 0xfad0b8, 0.89, 0.02, 0.05, 0, 0.33, -0.3 + k * 0.3); l.rotation.y = 0.5; g.add(l); }
      break;
    }
    case 'tonkatsu': {
      g.add(mesh(G.box, 0xc88a3a, 0.9, 0.36, 0.9, 0, 0.18, 0, 1));
      for (let k = 0; k < 6; k++) g.add(mesh(G.ball, 0xe0a85a, 0.08, 0.04, 0.08, -0.3 + (k % 3) * 0.3, 0.37, k < 3 ? -0.18 : 0.18, 1));
      if (i === 0) g.add(mesh(G.box, 0x6a3a1a, 0.6, 0.02, 0.12, 0, 0.38, 0));
      break;
    }
    default: g.add(mesh(G.box, 0xffffff, 0.9, 0.4, 0.9, 0, 0.2, 0));
  }
  return g;
}
function pieceView(pc, color) {
  const g = new THREE.Group();
  const shape = pc.shape;
  const base = M(color, 0.9);
  pc.cellMeshes = [];
  shape.forEach(([x, y], i) => {
    const cell = new THREE.Group(); cell.position.set(x * CELL, 0, y * CELL);
    const tile = new THREE.Mesh(G.box, base); tile.scale.set(0.98, 0.06, 0.98); tile.position.y = 0.03; tile.receiveShadow = true; cell.add(tile);
    cell.add(foodCell(pc.food, i, shape.length));
    g.add(cell);
    pc.cellMeshes.push(cell);
  });
  g.traverse((o) => { if (o.isMesh) o.userData.piece = pc; });
  pc.baseMat = base;
  return g;
}
function layoutPiece(pc) {
  // move each cell to where rotate() puts it (same rule, same order), and turn its food to match
  let cells = pc.shape.map(([x, y]) => [x, y]);
  for (let i = 0; i < pc.rot; i++) cells = cells.map(([x, y]) => [-y, x]);
  const mx = Math.min(...cells.map((c) => c[0])), my = Math.min(...cells.map((c) => c[1]));
  cells.forEach(([x, y], i) => { pc.cellMeshes[i].position.set((x - mx + 0.5) * CELL, 0, (y - my + 0.5) * CELL); pc.cellMeshes[i].rotation.y = -pc.rot * Math.PI / 2; });
}

// ------------------------------------------------------------------ box
const boxGroup = new THREE.Group(); scene.add(boxGroup);
let P = null, grid = null, level = 0, seed = seedFromUrl(321), held = null, hover = null, moves = 0, rejects = 0, levelDone = false, doneT = 0;
const boxPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -0.12);
const meta = load(SAVE_KEY, { boxes: 0 });
let boxOrigin = new THREE.Vector3();

function buildBox() {
  for (const c of [...boxGroup.children]) boxGroup.remove(c);
  const W = P.w * CELL, H = P.h * CELL, wall = 0.18;
  boxOrigin = new THREE.Vector3(-W / 2 - 2.6, 0, -H / 2);
  boxGroup.position.copy(boxOrigin);
  const red = M(0xa8201a, 0.35, { metalness: 0.1 }), black = M(0x161210, 0.3);
  const floor = new THREE.Mesh(G.box, red); floor.scale.set(W + wall * 2, 0.1, H + wall * 2); floor.position.set(W / 2, 0.05, H / 2); floor.receiveShadow = true; boxGroup.add(floor);
  for (const [sx, sz, x, z] of [[W + wall * 2, wall, W / 2, -wall / 2], [W + wall * 2, wall, W / 2, H + wall / 2], [wall, H, -wall / 2, H / 2], [wall, H, W + wall / 2, H / 2]]) {
    const w = new THREE.Mesh(G.box, black); w.scale.set(sx, 0.62, sz); w.position.set(x, 0.31, z); w.castShadow = true; w.receiveShadow = true; boxGroup.add(w);
  }
  // faint grid lines on the floor of the box
  const pts = [];
  for (let x = 0; x <= P.w; x++) pts.push(x, 0.105, 0, x, 0.105, H);
  for (let y = 0; y <= P.h; y++) pts.push(0, 0.105, y, W, 0.105, y);
  const lg = new THREE.BufferGeometry(); lg.setAttribute('position', new THREE.Float32BufferAttribute(pts, 3));
  boxGroup.add(new THREE.LineSegments(lg, new THREE.LineBasicMaterial({ color: 0xd8584a, transparent: true, opacity: 0.6 })));
}
function trayLayout() {
  // pieces not in the box sit in rows to the right of it
  let x = 0.8, z = -P.h / 2 - 0.2, rowH = 0;
  const maxX = 10.5;
  for (const pc of P.pieces) {
    if (pc.placed || pc === held) continue;
    const r = rotate(pc.shape, pc.rot);
    const w = Math.max(...r.map((c) => c[0])) + 1, h = Math.max(...r.map((c) => c[1])) + 1;
    if (x + w > maxX) { x = 0.8; z += rowH + 0.5; rowH = 0; }
    pc.trayPos = new THREE.Vector3(x, 0.01, z);
    x += w + 0.5; rowH = Math.max(rowH, h);
  }
}
function startLevel(idx) {
  level = idx;
  for (const pc of P?.pieces ?? []) scene.remove(pc.view);
  P = generate(level, seed + level * 97);
  grid = Array.from({ length: P.h }, () => Array(P.w).fill(-1));
  buildBox();
  P.pieces.forEach((pc, i) => {
    pc.rot = pc.startRot; pc.placed = false; pc.at = null;
    pc.view = pieceView(pc, PAL[i % PAL.length]); layoutPiece(pc); scene.add(pc.view);
  });
  trayLayout();
  for (const pc of P.pieces) pc.view.position.copy(pc.trayPos);
  held = null; hover = null; moves = 0; rejects = 0; levelDone = false; doneT = 0;
  frame();
  $('box-name').textContent = P.name;
  $('lvl').textContent = `${level + 1} / ${LEVELS.length}`;
  $('done').hidden = true;
  hud();
}
function frame() {
  // fit the box and the tray: widest extent of either, with a little air
  const done = levelDone;
  const x0 = boxOrigin.x - 1.2, x1 = done ? boxOrigin.x + P.w + 1.2 : 11.5, z0 = -P.h / 2 - 0.8, z1 = Math.max(P.h / 2, 3.5) + 0.8;
  const cx = (x0 + x1) / 2, cz = (z0 + z1) / 2;
  const vfov = THREE.MathUtils.degToRad(camera.fov), hfov = 2 * Math.atan(Math.tan(vfov / 2) * camera.aspect);
  const dist = Math.max(((x1 - x0) / 2) / Math.tan(hfov / 2), ((z1 - z0) / 2) / Math.tan(vfov / 2) * 1.25) * 1.05;
  camera.position.set(cx, dist * 0.88, cz + dist * 0.48);
  camera.lookAt(cx, 0, cz);
  camera.updateMatrixWorld();
}

// ------------------------------------------------------------------ interaction
const anchorOf = (pc) => { const r = rotate(pc.shape, pc.rot); return r[Math.floor(r.length / 2)]; };
function cellAt(x, y) {
  const ray = rayAt(x, y, camera); const p = new THREE.Vector3();
  if (!ray.intersectPlane(boxPlane, p)) return null;
  return { cx: Math.floor(p.x - boxOrigin.x), cy: Math.floor(p.z - boxOrigin.z), world: p };
}
function pick(pc) {
  if (levelDone || !pc) return false;
  if (held) drop();
  if (pc.placed) { for (const [x, y] of covered(pc, pc.at.x, pc.at.y, pc.rot)) grid[y][x] = -1; pc.placed = false; pc.at = null; }
  held = pc; sfx.soft();
  trayLayout(); hud();
  return true;
}
function turn(dir = 1) {
  if (!held) return;
  held.rot = (held.rot + dir + 4) % 4; layoutPiece(held);
  sfx.tone(560, 0.07, { type: 'triangle', vol: 0.05 });
  if (hover) hoverAt(hover.sx, hover.sy);
}
function hoverAt(sx, sy) {
  hover = { sx, sy, ok: false, x: 0, y: 0 };
  if (!held) return;
  const c = cellAt(sx, sy);
  if (!c) return;
  const a = anchorOf(held);
  const inBox = c.cx >= -1 && c.cy >= -1 && c.cx <= P.w && c.cy <= P.h;
  if (inBox) {
    hover.x = c.cx - a[0]; hover.y = c.cy - a[1];
    hover.ok = fits(grid, P.w, P.h, covered(held, hover.x, hover.y, held.rot));
    held.view.position.set(boxOrigin.x + hover.x, 0.45, boxOrigin.z + hover.y);
    held.baseMat.emissive.setHex(hover.ok ? 0x1a5a1a : 0x6a1a1a);
  } else {
    held.view.position.set(c.world.x - a[0] - 0.5, 0.6, c.world.z - a[1] - 0.5);
    held.baseMat.emissive.setHex(0);
    hover.out = true;
  }
}
function place() {
  if (!held || !hover) return 'nothing';
  if (hover.out) { held.baseMat.emissive.setHex(0); held = null; trayLayout(); return 'tray'; }
  if (!hover.ok) { rejects++; sfx.soft(); toast('It does not fit there.'); return 'rejected'; }
  const pc = held;
  for (const [x, y] of covered(pc, hover.x, hover.y, pc.rot)) grid[y][x] = pc.id;
  pc.placed = true; pc.at = { x: hover.x, y: hover.y };
  pc.view.position.set(boxOrigin.x + hover.x, 0.1, boxOrigin.z + hover.y);
  pc.baseMat.emissive.setHex(0);
  held = null; moves++;
  sfx.tone(440 * 2 ** (P.pieces.filter((p) => p.placed).length / 12), 0.25, { vol: 0.08 });
  trayLayout();
  if (P.pieces.every((p) => p.placed)) { levelDone = true; doneT = 0; meta.boxes++; save(SAVE_KEY, meta); setTimeout(() => sfx.done(), 200); }
  hud();
  return 'placed';
}
function drop() { if (!held) return; held.baseMat.emissive.setHex(0); held = null; trayLayout(); hud(); }

canvas.addEventListener('pointermove', (e) => { if (!api.isAuto()) hoverAt(e.clientX, e.clientY); });
canvas.addEventListener('pointerdown', (e) => {
  if (api.isAuto()) return;
  if (e.button === 2) { turn(1); return; }
  if (held) { place(); return; }
  const hit = pickAt(e.clientX, e.clientY, camera, P.pieces.map((p) => p.view))[0];
  if (hit) { pick(hit.object.userData.piece); hoverAt(e.clientX, e.clientY); }
});
canvas.addEventListener('contextmenu', (e) => e.preventDefault());
canvas.addEventListener('wheel', (e) => { if (held) { e.preventDefault(); turn(e.deltaY > 0 ? 1 : -1); } }, { passive: false });
addEventListener('keydown', (e) => { if (e.key === 'r' || e.key === 'R' || e.key === ' ') { e.preventDefault(); turn(1); } if (e.key === 'Escape') drop(); });
$('turn').addEventListener('click', () => turn(1));
$('next').addEventListener('click', () => { if (level + 1 < LEVELS.length) startLevel(level + 1); else { seed = (seed * 7 + 1) % 2147483647; startLevel(0); } });

// ------------------------------------------------------------------ loop
function tick(dt) {
  for (const pc of P.pieces) if (!pc.placed && pc !== held && pc.trayPos) pc.view.position.lerp(pc.trayPos, Math.min(1, dt * 8));
  if (levelDone) {
    if (doneT === 0) frame();   // close in on the finished box
    doneT += dt;
    if (doneT > 1.2 && $('done').hidden) {
      $('done-text').textContent = `${P.pieces.length} pieces, ${moves} moves${rejects ? `, ${rejects} that did not fit` : ''}. Itadakimasu!`;
      $('next').textContent = level + 1 < LEVELS.length ? 'A bigger box' : 'Start again';
      $('done').hidden = false;
    }
  }
  hudT += dt; if (hudT > 0.3) { hudT = 0; hud(); }
}
let hudT = 0;
function hud() {
  const n = P.pieces.filter((p) => p.placed).length;
  $('left').textContent = `${P.pieces.length - n} to pack`;
  $('turn').disabled = !held;
}
function render() { renderer.render(scene, camera); }
function resize() { renderer.setSize(innerWidth, innerHeight, false); camera.aspect = innerWidth / innerHeight; camera.fov = camera.aspect < 1 ? 64 : 38; camera.updateProjectionMatrix(); if (P) frame(); }
addEventListener('resize', resize); resize();

// ------------------------------------------------------------------ tour
// Clicks through the screen: pick a piece, try one bad spot on purpose, then
// turn each piece to its cut and set it where it came from.
const bot = { t: 0, phase: 'pack', triedBad: false, badRes: null, results: [] };
const screenOfCell = (cx, cy) => toScreen(new THREE.Vector3(boxOrigin.x + cx + 0.5, 0.12, boxOrigin.z + cy + 0.5), camera);
function botStep(dt) {
  bot.t += dt;
  if (bot.t < 0.35) return;
  bot.t = 0;
  camera.updateMatrixWorld();
  if (bot.phase === 'pack') {
    const pc = P.pieces.find((p) => !p.placed);
    if (!pc) { bot.phase = 'wait'; return; }
    const s = toScreen(pc.view.position.clone().add(new THREE.Vector3(0.5, 0.2, 0.5)), camera);
    const hit = pickAt(s.x, s.y, camera, [pc.view])[0];
    if (!hit) bot.pickMiss = (bot.pickMiss ?? 0) + 1;
    pick(pc);
    const target = held;
    // a deliberate bad spot once there is something in the box to collide with
    if (!bot.triedBad && P.pieces.some((p) => p.placed)) {
      // hold the piece so the cell under the pointer is one that is already taken
      const other = P.pieces.find((p) => p.placed);
      const [ox, oy] = covered(other, other.at.x, other.at.y, other.rot)[0];
      const q = screenOfCell(ox, oy);
      hoverAt(q.x, q.y); bot.badRes = place(); bot.triedBad = true;
    }
    const need = (4 - target.rot) % 4;   // home is the uncut orientation
    for (let k = 0; k < need; k++) $('turn').click();
    const a = anchorOf(target);
    const q = screenOfCell(target.home.x + a[0], target.home.y + a[1]);
    hoverAt(q.x, q.y);
    bot.results.push(place());
    return;
  }
  if (bot.phase === 'wait' && !$('done').hidden) { $('next').click(); bot.phase = level === 1 ? 'pack' : 'done'; }
}
const full = () => grid.every((row) => row.every((v) => v >= 0));
const checks = [
  { name: 'empty-box', when: () => true, check: () => P.pieces.length >= 2 && grid.flat().every((v) => v < 0) && P.pieces.some((p) => p.rot !== 0) },
  { name: 'does-not-fit', when: () => bot.badRes !== null, check: () => bot.badRes === 'rejected' && rejects === 1 },
  { name: 'first-box', when: () => levelDone && level === 0 && !$('done').hidden, check: () => full() && bot.results.every((r) => r === 'placed') },
  { name: 'second-box', when: () => levelDone && level === 1 && doneT > 0.5, check: () => full() && P.w === LEVELS[1].w && P.h === LEVELS[1].h },
];

startLevel(0);
const help = wireHelp();
const api = harness({
  checks, tick, render, selfTest, bot: botStep,
  paused: () => help.isOpen(),
  extraState: () => ({ level, moves, rejects, packed: P.pieces.filter((p) => p.placed).length, botPickMisses: bot.pickMiss ?? 0 }),
  winNote: () => 'two boxes packed by clicking pieces in, one deliberate misfit rejected',
});
if (!VERIFY && meta.boxes === 0 && !new URLSearchParams(location.search).has('autoplay')) help.open();
api.start();

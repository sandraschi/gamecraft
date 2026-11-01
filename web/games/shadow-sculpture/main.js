// Kage-e (shadow pictures): a heap of junk blocks hangs under a lamp. Turn it
// until its shadow on the wall becomes a picture. The sculpture is built from
// the picture: every block covers part of the silhouette but sits at its own
// depth, so only one way of looking at it makes sense.
import * as THREE from 'three';
import { VERIFY, $, rng, seedFromUrl, makeRenderer, harness, sfx, toast, wireHelp, load, save } from '../_shared/gc.js';
import { setupMusic } from '../_shared/music.js';
const music = setupMusic('lamp', { sfx });

const SHAPES = [
  { name: 'Mount Fuji', jp: '富士山', art: [
    '.........##..##.........',
    '........###..###........',
    '.......##########.......',
    '......############......',
    '.....##############.....',
    '....################....',
    '...##################...',
    '..####################..',
    '.######################.',
    '########################',
  ] },
  { name: 'a little house', jp: '家', art: [
    '..........##....',
    '.......##.##....',
    '......#####.....',
    '.....#######....',
    '....#########...',
    '...###########..',
    '..#############.',
    '################',
    '.##############.',
    '.##..######..##.',
    '.##..######..##.',
    '.##########..##.',
    '.##########..##.',
    '.##########..##.',
  ] },
  { name: 'a cat', jp: '猫', art: [
    '.#.....#..........',
    '.##...##..........',
    '.#######..........',
    '.##.#.##..........',
    '.#######..........',
    '..#####...........',
    '.#######..........',
    '#########.........',
    '#########......##.',
    '#########.....##..',
    '##########...##...',
    '##########..##....',
    '###########.#.....',
    '###########.......',
    '.#########........',
    '.##.##.##.........',
  ] },
  { name: 'a teapot', jp: '急須', art: [
    '.........##..........',
    '........####.........',
    '......########.......',
    '.###.##########......',
    '.#..############...##',
    '.#..#############.##.',
    '.#..#############.#..',
    '..#.###############..',
    '...################..',
    '....##############...',
    '.....############....',
    '......##########.....',
  ] },
  { name: 'a rocket', jp: 'ロケット', art: [
    '.....##.....',
    '....####....',
    '....####....',
    '...######...',
    '...##..##...',
    '...##..##...',
    '...######...',
    '...######...',
    '...######...',
    '..########..',
    '.##########.',
    '##.######.##',
    '#..######..#',
    '....#..#....',
    '...##..##...',
    '..##....##..',
  ] },
  { name: 'a familiar face', jp: 'ベニー', art: [
    '.#......#.......',
    '.##....##.......',
    '.###..###.......',
    '.########.......',
    '##########......',
    '###.#######.....',
    '############....',
    '##############..',
    '###############.',
    '.##############.',
    '..########......',
    '..#######.......',
    '.#########......',
    '###########.....',
  ] },
];
const CELL = 0.13;
const SOLVE_IOU = 0.9;          // a degree or so off still reads as the picture, then it settles exactly
const ASSIST_IOU = 0.7;         // once this close, letting go lets the sculpture drift the rest of the way
const RES = 2;                 // mask samples per cell along each axis
const SAVE_KEY = 'gamecraft.kagee.v1';

// ------------------------------------------------------------------ pure geometry
export function maskOf(shape) {
  const rows = shape.art, h = rows.length, w = rows[0].length;
  const filled = (x, y) => rows[y]?.[x] === '#';
  return { w, h, filled };
}
/** Greedy rectangle cover of the art, each rectangle a block at its own depth. */
export function planBlocks(shape, seed) {
  const r = rng(seed);
  const { w, h, filled } = maskOf(shape);
  const used = Array.from({ length: h }, () => Array(w).fill(false));
  const blocks = [];
  const cells = [];
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) if (filled(x, y)) cells.push([x, y]);
  for (const [x, y] of r.shuffle(cells)) {
    if (used[y][x]) continue;
    const maxW = 1 + r.int(4), maxH = 1 + r.int(3);
    let bw = 1, bh = 1;
    while (bw < maxW && x + bw < w && filled(x + bw, y) && !used[y][x + bw]) bw++;
    outer: while (bh < maxH && y + bh < h) { for (let k = 0; k < bw; k++) if (!filled(x + k, y + bh) || used[y + bh][x + k]) break outer; bh++; }
    for (let yy = y; yy < y + bh; yy++) for (let xx = x; xx < x + bw; xx++) used[yy][xx] = true;
    blocks.push({ x, y, bw, bh, z: r.range(-1.25, 1.25), d: r.range(0.15, 0.7), tone: r.int(6) });
  }
  const yaw = (r() < 0.5 ? -1 : 1) * r.range(0.9, 2.3), pitch = (r() < 0.5 ? -1 : 1) * r.range(0.35, 0.8);
  return { blocks, w, h, start: { yaw, pitch } };
}
/** Projected silhouette vs target, sampled on a grid in the wall plane. */
export function silhouetteIoU(plan, q) {
  const { w, h } = plan;
  const hulls = plan.blocks.map((b) => {
    const x0 = (b.x - w / 2) * CELL, x1 = x0 + b.bw * CELL, y1 = (h / 2 - b.y) * CELL, y0 = y1 - b.bh * CELL;
    const pts = [];
    for (const xx of [x0, x1]) for (const yy of [y0, y1]) for (const zz of [b.z - b.d / 2, b.z + b.d / 2]) {
      const v = new THREE.Vector3(xx, yy, zz).applyQuaternion(q); pts.push([v.x, v.y]);
    }
    return hull(pts);
  });
  const pad = 6, W = (w + pad * 2) * RES, H = (h + pad * 2) * RES;
  let inter = 0, uni = 0;
  for (let j = 0; j < H; j++) for (let i = 0; i < W; i++) {
    const cx = (i + 0.5) / RES - pad, cy = (j + 0.5) / RES - pad;   // in cell units
    const target = cx >= 0 && cy >= 0 && cx < w && cy < h && plan.mask(Math.floor(cx), Math.floor(cy));
    const px = (cx - w / 2) * CELL, py = (h / 2 - cy) * CELL;
    let got = false;
    for (const hl of hulls) if (inHull(hl, px, py)) { got = true; break; }
    if (got && target) inter++;
    if (got || target) uni++;
  }
  return uni ? inter / uni : 0;
}
function hull(pts) {
  pts.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const cross = (o, a, b) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const lo = [], up = [];
  for (const p of pts) { while (lo.length >= 2 && cross(lo[lo.length - 2], lo[lo.length - 1], p) <= 0) lo.pop(); lo.push(p); }
  for (let i = pts.length - 1; i >= 0; i--) { const p = pts[i]; while (up.length >= 2 && cross(up[up.length - 2], up[up.length - 1], p) <= 0) up.pop(); up.push(p); }
  return lo.slice(0, -1).concat(up.slice(0, -1));
}
function inHull(hl, x, y) {
  for (let i = 0; i < hl.length; i++) {
    const a = hl[i], b = hl[(i + 1) % hl.length];
    if ((b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0]) < -1e-9) return false;
  }
  return hl.length > 2;
}
const qFrom = (pitch, yaw) => new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(1, 0, 0), pitch).multiply(new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), yaw));

// ------------------------------------------------------------------ scene
const canvas = $('view');
const renderer = makeRenderer(canvas, { shadows: true, clear: 0x1a1820 });
renderer.shadowMap.type = THREE.PCFShadowMap;
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x1a1820);
const camera = new THREE.PerspectiveCamera(42, 1, 0.1, 60);
camera.position.set(6.2, 1.5, 6.0); camera.lookAt(-0.9, 0.0, -1.7);

const wallMat = new THREE.MeshStandardMaterial({ color: 0xf2e6cc, roughness: 0.95 });
const wall = new THREE.Mesh(new THREE.PlaneGeometry(14, 9), wallMat);
wall.position.set(0, 0.5, -4); wall.receiveShadow = true; scene.add(wall);
const floor = new THREE.Mesh(new THREE.PlaneGeometry(16, 16), new THREE.MeshStandardMaterial({ color: 0x6a4a34, roughness: 0.8 }));
floor.rotation.x = -Math.PI / 2; floor.position.y = -2.6; floor.receiveShadow = true; scene.add(floor);
const side = new THREE.Mesh(new THREE.PlaneGeometry(16, 9), new THREE.MeshStandardMaterial({ color: 0x8a7a68, roughness: 0.95 }));
side.rotation.y = Math.PI / 2; side.position.set(-6, 0.5, 2); scene.add(side);
scene.add(new THREE.HemisphereLight(0xd8d0ff, 0x4a3a30, 0.8));
const lamp = new THREE.DirectionalLight(0xfff0d0, 2.4);
lamp.position.set(0, 0, 12); lamp.target.position.set(0, 0, 0);
lamp.castShadow = true;
lamp.shadow.mapSize.set(2048, 2048);
Object.assign(lamp.shadow.camera, { left: -3.5, right: 3.5, top: 3.5, bottom: -3.5, near: 1, far: 30 });
lamp.shadow.bias = -0.0008;
scene.add(lamp, lamp.target);
const fill = new THREE.PointLight(0x8aa8ff, 6, 20); fill.position.set(5, 3, 2); scene.add(fill);
// a hanging wire and a lamp body for the eye
const wire = new THREE.Mesh(new THREE.CylinderGeometry(0.01, 0.01, 4, 6), new THREE.MeshBasicMaterial({ color: 0x555555 })); wire.position.set(0, 3.6, 0); scene.add(wire);

const sculpt = new THREE.Group(); scene.add(sculpt);
const outline = new THREE.Group(); outline.position.set(0, 0, -3.99); scene.add(outline);
const TONES = [0x8a6a4a, 0xb8b0a0, 0x6a7a8a, 0xa8483a, 0xc8a050, 0x4a6a4a];
const mats = TONES.map((c, i) => new THREE.MeshStandardMaterial({ color: c, roughness: 0.55, metalness: i === 2 ? 0.6 : 0.05 }));

let level = 0, seed = seedFromUrl(4711), plan = null, iou = 0, solved = false, solvedT = 0, peeks = 0, drags = 0, levelT = 0;
let target = null;
const meta = load(SAVE_KEY, { solved: 0 });

function startLevel(idx) {
  level = idx;
  const shape = SHAPES[level];
  plan = planBlocks(shape, seed * 17 + level * 101);
  const m = maskOf(shape); plan.mask = m.filled;
  for (const c of [...sculpt.children]) { sculpt.remove(c); c.geometry.dispose(); }
  for (const b of plan.blocks) {
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(b.bw * CELL, b.bh * CELL, b.d), mats[b.tone]);
    mesh.position.set((b.x + b.bw / 2 - plan.w / 2) * CELL, (plan.h / 2 - b.y - b.bh / 2) * CELL, b.z);
    mesh.castShadow = true; mesh.receiveShadow = true;
    sculpt.add(mesh);
  }
  sculpt.quaternion.copy(qFrom(plan.start.pitch, plan.start.yaw));
  buildOutline(shape);
  outline.visible = false;
  solved = false; solvedT = 0; peeks = 0; drags = 0; levelT = 0; target = null;
  iou = silhouetteIoU(plan, sculpt.quaternion);
  $('shape').textContent = `${shape.jp}`;
  $('level').textContent = `${level + 1} / ${SHAPES.length}`;
  $('reveal').textContent = '';
  $('done').hidden = true;
  wallMat.color.setHex(0xf2e6cc);
  hud();
}
function buildOutline(shape) {
  for (const c of [...outline.children]) { outline.remove(c); c.geometry.dispose(); }
  const { w, h, filled } = maskOf(shape);
  const pts = [];
  for (let y = 0; y <= h; y++) for (let x = 0; x <= w; x++) {
    const X = (x - w / 2) * CELL, Y = (h / 2 - y) * CELL;
    if (x < w && filled(x, y) !== filled(x, y - 1)) pts.push(X, Y, 0, X + CELL, Y, 0);
    if (y < h && filled(x, y) !== filled(x - 1, y)) pts.push(X, Y, 0, X, Y - CELL, 0);
  }
  const g = new THREE.BufferGeometry(); g.setAttribute('position', new THREE.Float32BufferAttribute(pts, 3));
  outline.add(new THREE.LineSegments(g, new THREE.LineBasicMaterial({ color: 0xc87a3a, transparent: true, opacity: 0.7 })));
}

// ------------------------------------------------------------------ input
const AX = new THREE.Vector3(1, 0, 0), AY = new THREE.Vector3(0, 1, 0), _qq = new THREE.Quaternion();
const K = 0.008;
function dragBy(dx, dy) {
  if (solved) return;
  sculpt.quaternion.premultiply(_qq.setFromAxisAngle(AY, dx * K));
  sculpt.quaternion.premultiply(_qq.setFromAxisAngle(AX, dy * K));
  sculpt.quaternion.normalize();
  drags++;
}
let last = null;
canvas.addEventListener('pointerdown', (e) => { if (api.isAuto()) return; last = { x: e.clientX, y: e.clientY }; canvas.setPointerCapture(e.pointerId); });
canvas.addEventListener('pointermove', (e) => { if (!last) return; dragBy(e.clientX - last.x, e.clientY - last.y); last = { x: e.clientX, y: e.clientY }; });
canvas.addEventListener('pointerup', () => { last = null; });
addEventListener('keydown', (e) => {
  const s = 12;
  if (e.key === 'ArrowLeft') dragBy(-s, 0); if (e.key === 'ArrowRight') dragBy(s, 0);
  if (e.key === 'ArrowUp') dragBy(0, -s); if (e.key === 'ArrowDown') dragBy(0, s);
});
$('peek').addEventListener('click', () => { outline.visible = !outline.visible; if (outline.visible) peeks++; });
$('next').addEventListener('click', () => { if (level + 1 < SHAPES.length) startLevel(level + 1); else { seed = (seed * 7 + 5) % 2147483647; startLevel(0); } });

// ------------------------------------------------------------------ sim
let iouT = 0;
function tick(dt) {
  levelT += dt;
  iouT += dt;
  if (!solved && iouT > 0.1) {
    iouT = 0;
    iou = silhouetteIoU(plan, sculpt.quaternion);
    if (iou >= SOLVE_IOU) {
      solved = true; solvedT = 0;
      // settle into whichever exact pose is closest (a mirror pose counts for symmetric pictures)
      target = nearestExact();
      sfx.done();
      meta.solved++; save(SAVE_KEY, meta);
    }
    hud();
  }
  if (solved) {
    solvedT += dt;
    sculpt.quaternion.slerp(target, Math.min(1, dt * 4));
    wallMat.color.lerp(new THREE.Color(0xffe2a8), Math.min(1, dt * 2));
    if (solvedT > 0.6) $('reveal').textContent = SHAPES[level].name;
    if (solvedT > 1.8 && $('done').hidden) {
      $('done-text').textContent = `It was ${SHAPES[level].name} all along. ${peeks ? `${peeks} peek${peeks > 1 ? 's' : ''}.` : 'No peeking!'}`;
      $('next').textContent = level + 1 < SHAPES.length ? 'Next sculpture' : 'Again, with new junk';
      $('done').hidden = false;
    }
  } else {
    sculpt.position.y = Math.sin(levelT * 0.8) * 0.03;
    if (!last && iou >= ASSIST_IOU) sculpt.quaternion.slerp(nearestExact(), Math.min(1, dt * 1.2));
  }
}
function nearestExact() {
  const cands = [new THREE.Quaternion(), new THREE.Quaternion().setFromAxisAngle(AY, Math.PI)];
  // only the mirror pose if the picture is symmetric
  if (!SHAPES[level].art.every((row) => row === [...row].reverse().join(''))) cands.pop();
  return cands.sort((a, b) => sculpt.quaternion.angleTo(a) - sculpt.quaternion.angleTo(b))[0];
}
function render() { renderer.render(scene, camera); }
function hud() {
  const pct = Math.round(iou * 100);
  $('match').textContent = `${pct}%`;
  $('bar').style.width = `${Math.max(2, pct)}%`;
}
function resize() {
  renderer.setSize(innerWidth, innerHeight, false);
  camera.aspect = innerWidth / innerHeight; camera.fov = camera.aspect < 1 ? 64 : 42; camera.updateProjectionMatrix();
}
addEventListener('resize', resize); resize();

// ------------------------------------------------------------------ tour
// The bot drags like a hand would: first the wrong way, then it undoes pitch and yaw.
const bot = { t: 0, phase: 'wrong', iouBefore: 0, iouAfterWrong: 0, wrongYaw: 0, queue: [] };
function enqueue(dx, dy, total) { const n = Math.ceil(Math.max(Math.abs(dx), Math.abs(dy)) / 20); for (let i = 0; i < n; i++) bot.queue.push([dx / n, dy / n]); void total; }
function botStep(dt) {
  bot.t += dt;
  if (bot.queue.length) { const [dx, dy] = bot.queue.shift(); dragBy(dx, dy); return; }
  if (bot.t < 0.4) return;
  bot.t = 0;
  if (bot.phase === 'wrong') {
    bot.iouBefore = silhouetteIoU(plan, sculpt.quaternion);
    bot.wrongYaw = 0.5;
    enqueue(bot.wrongYaw / K, 0);
    bot.phase = 'measure';
  } else if (bot.phase === 'measure') {
    bot.iouAfterWrong = silhouetteIoU(plan, sculpt.quaternion);
    bot.phase = 'undo';
  } else if (bot.phase === 'undo') {
    enqueue(-bot.wrongYaw / K, 0);
    enqueue(0, -plan.start.pitch / K);
    enqueue(-plan.start.yaw / K, 0);
    bot.phase = 'wait';
  } else if (bot.phase === 'wait' && !$('done').hidden) {
    $('next').click();
    bot.phase = level === 1 ? 'second' : 'done';
  } else if (bot.phase === 'second') {
    enqueue(0, -plan.start.pitch / K); enqueue(-plan.start.yaw / K, 0);
    bot.phase = 'wait2';
  } else if (bot.phase === 'wait2' && solved && solvedT > 1) {
    bot.phase = 'done';
  }
}
const checks = [
  { name: 'junk-heap', when: () => true, check: () => plan.blocks.length >= 20 && iou < 0.6 && !solved },
  { name: 'wrong-way', when: () => bot.phase === 'undo', check: () => !solved && bot.iouAfterWrong < 0.7 },
  { name: 'fuji', when: () => solved && solvedT > 1.9 && level === 0, check: () => iou >= SOLVE_IOU && !$('done').hidden && sculpt.quaternion.angleTo(target) < 0.05 },
  { name: 'house', when: () => level === 1 && solved && solvedT > 1.2, check: () => silhouetteIoU(plan, sculpt.quaternion) > 0.99 },
];

function selfTest() {
  const c = { assist_basin_reachable: true, exact_pose_is_perfect: true, start_is_a_jumble: true, side_view_is_a_jumble: true, asymmetric_mirror_fails: true, near_pose_solves: true, enough_blocks: true };
  let worstStart = 0;
  for (const [si, shape] of SHAPES.entries()) {
    const m = maskOf(shape);
    const symmetric = shape.art.every((row) => row === [...row].reverse().join(''));
    for (let s = 1; s <= 6; s++) {
      const pl = planBlocks(shape, s * 7919 + si); pl.mask = m.filled;
      if (pl.blocks.length < 12) c.enough_blocks = false;
      if (silhouetteIoU(pl, new THREE.Quaternion()) < 0.999) c.exact_pose_is_perfect = false;
      const st = silhouetteIoU(pl, qFrom(pl.start.pitch, pl.start.yaw));
      worstStart = Math.max(worstStart, st);
      if (st >= 0.62) c.start_is_a_jumble = false;
      if (silhouetteIoU(pl, qFrom(0, Math.PI / 2)) >= SOLVE_IOU - 0.15) c.side_view_is_a_jumble = false;
      if (!symmetric && silhouetteIoU(pl, qFrom(0, Math.PI)) >= SOLVE_IOU) c.asymmetric_mirror_fails = false;
      if (silhouetteIoU(pl, qFrom(0.015, -0.015)) < SOLVE_IOU) c.near_pose_solves = false;
      // a hand can get within ~3 degrees; from there the assist must take over
      if (silhouetteIoU(pl, qFrom(0.05, -0.05)) < ASSIST_IOU) c.assist_basin_reachable = false;
    }
  }
  return { pass: Object.values(c).every(Boolean), checks: c, summary: `${SHAPES.length} pictures x 6 seeds; worst starting match ${Math.round(worstStart * 100)}%` };
}

startLevel(0);
const help = wireHelp();
const api = harness({
  checks, tick, render, selfTest, bot: botStep,
  paused: () => help.isOpen(),
  extraState: () => ({ level, iou: +iou.toFixed(3), solved, drags }),
  winNote: () => 'Fuji and the house both cast from a jumble by dragging alone',
});
if (!VERIFY && meta.solved === 0 && !new URLSearchParams(location.search).has('autoplay')) help.open();
api.start();

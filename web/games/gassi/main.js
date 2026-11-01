// Gassi: Benny shows you his walk through a stylized Alsergrund once, then you
// walk it from memory. A wrong turn just gets a puzzled look; nothing is lost.
import * as THREE from 'three';
import { VERIFY, $, seedFromUrl, makeRenderer, toScreen, rayAt, harness, sfx, toast, wireHelp, load, save, rng } from '../_shared/gc.js';
import { setupMusic } from '../_shared/music.js';
const music = setupMusic('stroll', { sfx });
import { buildDog } from '../where-is-benny/dogs.js';
import * as R from './route.js';

const SP = 7;                    // metres between intersections
const SAVE_KEY = 'gamecraft.gassi.v1';
const W = (R.COLS - 1) * SP, H = (R.ROWS - 1) * SP;
const pos = ([x, y]) => new THREE.Vector3(x * SP - W / 2, 0, y * SP - H / 2);

const canvas = $('view');
const renderer = makeRenderer(canvas, { shadows: !VERIFY, clear: 0xbfd8ea });
const scene = new THREE.Scene();
scene.background = new THREE.Color(0xbfd8ea);
scene.fog = new THREE.Fog(0xbfd8ea, 70, 140);
const camera = new THREE.PerspectiveCamera(40, 1, 0.5, 300);
scene.add(new THREE.HemisphereLight(0xf2f6ff, 0x6a6a5a, 1.3));
const sun = new THREE.DirectionalLight(0xfff0d8, 2.0); sun.position.set(-20, 40, 15);
if (!VERIFY) { sun.castShadow = true; sun.shadow.mapSize.set(2048, 2048); Object.assign(sun.shadow.camera, { left: -26, right: 26, top: 26, bottom: -26 }); }
scene.add(sun);
const M = (c, r = 0.85) => new THREE.MeshStandardMaterial({ color: c, roughness: r });
function box(w, h, d, c, x, y, z, parent = scene) { const m = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), M(c)); m.position.set(x, y, z); m.castShadow = true; m.receiveShadow = true; parent.add(m); return m; }

// ------------------------------------------------------------------ city
const ground = new THREE.Mesh(new THREE.PlaneGeometry(160, 160), M(0x8a8a86)); ground.rotation.x = -Math.PI / 2; ground.receiveShadow = true; scene.add(ground);
for (let x = 0; x < R.COLS; x++) box(2.4, 0.02, H + 6, 0x5a5a5e, x * SP - W / 2, 0.01, 0);
for (let y = 0; y < R.ROWS; y++) box(W + 6, 0.021, 2.4, 0x5a5a5e, 0, 0.011, y * SP - H / 2);
// facade texture: rows of windows on a pastel wall
function facade(color, seed) {
  const r = rng(seed);
  const c = document.createElement('canvas'); c.width = 64; c.height = 128;
  const x = c.getContext('2d'); x.fillStyle = color; x.fillRect(0, 0, 64, 128);
  x.fillStyle = 'rgba(255,255,255,0.35)'; x.fillRect(0, 26, 64, 3); x.fillRect(0, 60, 64, 3); x.fillRect(0, 94, 64, 3);
  for (let row = 0; row < 4; row++) for (let col = 0; col < 3; col++) {
    x.fillStyle = r() < 0.15 ? '#f6e6a8' : '#3a4a5a'; x.fillRect(8 + col * 20, 6 + row * 32, 9, 16);
    x.fillStyle = 'rgba(255,255,255,0.6)'; x.fillRect(6 + col * 20, 4 + row * 32, 13, 2);
  }
  const t = new THREE.CanvasTexture(c); t.colorSpace = THREE.SRGBColorSpace; t.wrapS = THREE.RepeatWrapping; return t;
}
const WALLS = ['#e8d8b0', '#f0e2c8', '#d8c8a8', '#e8c8b0', '#d8d8c8', '#f2e8d8', '#e0b8a0', '#c8d0c0'];
const landmarkBlocks = new Set(['0,3', '1,0', '3,0', '2,2']); // blocks given over to landmarks below
for (let bx = 0; bx < R.COLS - 1; bx++) for (let by = 0; by < R.ROWS - 1; by++) {
  if (landmarkBlocks.has(`${bx},${by}`)) continue;
  const cx = (bx + 0.5) * SP - W / 2, cz = (by + 0.5) * SP - H / 2;
  const r = rng(bx * 17 + by * 5 + 3);
  for (const [ox, oz] of [[-1, -1], [1, -1], [-1, 1], [1, 1]]) {
    const h = r.range(3.2, 5.2);
    const tex = facade(WALLS[r.int(WALLS.length)], bx * 100 + by * 10 + ox + oz);
    tex.repeat.set(1, 1);
    const b = new THREE.Mesh(new THREE.BoxGeometry(2.1, h, 2.1), new THREE.MeshStandardMaterial({ map: tex, roughness: 0.9 }));
    b.position.set(cx + ox * 1.12, h / 2, cz + oz * 1.12); b.castShadow = true; b.receiveShadow = true; scene.add(b);
    const roof = box(2.2, 0.35, 2.2, 0x9a4a3a, cx + ox * 1.12, h + 0.17, cz + oz * 1.12); roof.material.color.offsetHSL(r.range(-0.02, 0.02), 0, r.range(-0.08, 0.05));
  }
}
// trees along the streets
{
  const trunk = new THREE.InstancedMesh(new THREE.CylinderGeometry(0.1, 0.13, 1.4, 6), M(0x6a4a2a), 120);
  const crown = new THREE.InstancedMesh(new THREE.SphereGeometry(0.75, 8, 6), M(0x5a9a4a), 120);
  const m = new THREE.Matrix4(); let k = 0;
  for (let y = 0; y < R.ROWS - 1; y++) for (const x of [0, R.COLS - 1]) for (const f of [0.3, 0.7]) {
    const px = x * SP - W / 2 + (x ? 1.6 : -1.6), pz = (y + f) * SP - H / 2;
    m.makeTranslation(px, 0.7, pz); trunk.setMatrixAt(k, m); m.makeTranslation(px, 1.9, pz); crown.setMatrixAt(k, m); k++;
  }
  trunk.count = crown.count = k; trunk.castShadow = crown.castShadow = true; scene.add(trunk, crown);
}
// landmarks, each near its intersection
const blockCenter = (bx, by) => [(bx + 0.5) * SP - W / 2, (by + 0.5) * SP - H / 2];
{
  // Volksoper: wide red-cream theatre (block 0,0 corner area handled by placing it in block 1,0? keep it at top-left outside)
  const [vx, vz] = [-W / 2 - 3.2, -H / 2 - 3.2];
  box(5, 4.2, 4, 0xc8786a, vx, 2.1, vz); box(5.4, 0.5, 4.4, 0xf2e8d8, vx, 4.4, vz); box(1.6, 1.2, 0.2, 0xf2d27a, vx + 1.2, 2.6, vz + 2.05);
  // Franz-Josefs-Bahnhof: long glass hall top-right
  const [fx, fz] = [W / 2 + 3.4, -H / 2 - 3.2];
  box(5, 2.4, 3.6, 0xb8c0c8, fx, 1.2, fz); const glass = box(5.2, 0.6, 3.8, 0x8ab8d8, fx, 2.7, fz); glass.material.metalness = 0.4;
  // Strudlhofstiege: stairs with lamps in block (1,0)
  const [sx, sz] = blockCenter(1, 0);
  for (let i = 0; i < 7; i++) box(3.2, 0.25 + i * 0.25, 0.6, 0xe8e0d0, sx, (0.25 + i * 0.25) / 2, sz + 1.8 - i * 0.6);
  for (const lx of [-1.8, 1.8]) { box(0.08, 2.4, 0.08, 0x2a3a2a, sx + lx, 1.2, sz); const l = new THREE.Mesh(new THREE.SphereGeometry(0.2, 10, 8), new THREE.MeshStandardMaterial({ color: 0xfff2c0, emissive: 0x8a7a40 })); l.position.set(sx + lx, 2.5, sz); scene.add(l); }
  // Palais Liechtenstein: pale baroque palace with a garden, block (3,0)
  const [px, pz] = blockCenter(3, 0);
  box(4.8, 3.2, 2.2, 0xf2ead8, px, 1.6, pz - 1.2); box(1.6, 3.8, 2.4, 0xf2ead8, px, 1.9, pz - 1.2); box(5, 0.4, 2.4, 0x6a7a8a, px, 3.4, pz - 1.2);
  box(4.8, 0.05, 2.6, 0x6aa85a, px, 0.03, pz + 1.3);
  // Berggasse 19: a facade with a red plaque, block (2,2) corner
  const [bx2, bz2] = blockCenter(2, 2);
  box(2.6, 4.6, 2.6, 0xe8dcc0, bx2 + 1.4, 2.3, bz2 + 1.4); box(0.7, 0.5, 0.05, 0xb8201a, bx2 + 1.4, 1.6, bz2 + 2.72);
  box(2.6, 4, 2.6, 0xd8c8a8, bx2 - 1.2, 2, bz2 - 1.2);
  // Sigmund-Freud-Park, block (0,3), with the Votivkirche spires beyond the corner
  const [gx, gz] = blockCenter(0, 3);
  box(5.6, 0.06, 5.6, 0x6ab85a, gx, 0.03, gz);
  for (const [tx, tz] of [[-1.5, -1.5], [1.4, -1], [-1, 1.6], [1.6, 1.5]]) { box(0.2, 1.2, 0.2, 0x6a4a2a, gx + tx, 0.6, gz + tz); const c = new THREE.Mesh(new THREE.SphereGeometry(0.9, 10, 8), M(0x4a8a3a)); c.position.set(gx + tx, 1.9, gz + tz); c.castShadow = true; scene.add(c); }
  const [vkx, vkz] = [-W / 2 - 3.6, H / 2 + 3.6];
  box(4, 3.4, 5, 0xd8d0c0, vkx, 1.7, vkz);
  for (const s of [-1.3, 1.3]) { box(1, 7, 1, 0xd8d0c0, vkx + s, 3.5, vkz - 2); const sp = new THREE.Mesh(new THREE.ConeGeometry(0.72, 3.2, 4), M(0x8a8a90)); sp.position.set(vkx + s, 8.6, vkz - 2); sp.rotation.y = Math.PI / 4; sp.castShadow = true; scene.add(sp); }
  // café: little terrace with umbrellas bottom-right
  const [cx, cz] = [W / 2 + 3, H / 2 + 3];
  box(3.4, 2.6, 3, 0xf2e2b8, cx + 0.6, 1.3, cz + 0.6);
  for (const [ux, uz] of [[-1.4, -1.2], [0.4, -1.6]]) { box(0.06, 1.6, 0.06, 0x3a3a3a, cx + ux, 0.8, cz + uz); const u = new THREE.Mesh(new THREE.ConeGeometry(0.9, 0.4, 8), M(0xb8303a)); u.position.set(cx + ux, 1.7, cz + uz); scene.add(u); }
}
// street name signs at the edge
function label(text, x, z, rot = 0) {
  const c = document.createElement('canvas'); c.width = 512; c.height = 64;
  const g = c.getContext('2d'); g.fillStyle = 'rgba(255,255,255,0)'; g.fillRect(0, 0, 512, 64);
  g.font = 'bold 34px Segoe UI, sans-serif'; g.fillStyle = '#2a3a5a'; g.textAlign = 'center'; g.fillText(text, 256, 44);
  const t = new THREE.CanvasTexture(c); t.colorSpace = THREE.SRGBColorSpace;
  const m = new THREE.Mesh(new THREE.PlaneGeometry(8, 1), new THREE.MeshBasicMaterial({ map: t, transparent: true, depthWrite: false }));
  m.rotation.x = -Math.PI / 2; m.rotation.z = rot; m.position.set(x, 0.05, z); scene.add(m);
}
R.NS_STREETS.forEach((n, i) => label(n, i * SP - W / 2, H / 2 + 6.4, Math.PI / 2));
R.EW_STREETS.forEach((n, i) => label(n, -W / 2 - 6.6, i * SP - H / 2, 0));

// intersections: soft discs the player clicks
const discs = [];
for (let y = 0; y < R.ROWS; y++) for (let x = 0; x < R.COLS; x++) {
  const d = new THREE.Mesh(new THREE.CircleGeometry(1.15, 24), new THREE.MeshBasicMaterial({ color: 0xffffff, transparent: true, opacity: 0.0, depthWrite: false }));
  d.rotation.x = -Math.PI / 2; d.position.copy(pos([x, y])).setY(0.04); d.userData.at = [x, y]; scene.add(d); discs.push(d);
}
const discAt = (p) => discs[p[1] * R.COLS + p[0]];

// trail: glowing dots along the route during the demo, paw prints for the player
const trail = new THREE.Group(); scene.add(trail);
const dotMat = new THREE.MeshBasicMaterial({ color: 0xffb030, transparent: true, opacity: 0.9, depthWrite: false });
const pawMat = new THREE.MeshBasicMaterial({ color: 0x6a3a1a, transparent: true, opacity: 0.8, depthWrite: false });
function dropMark(p, mat, size) { const m = new THREE.Mesh(new THREE.CircleGeometry(size, 12), mat); m.rotation.x = -Math.PI / 2; m.position.copy(p).setY(0.06); trail.add(m); return m; }
const treat = new THREE.Group(); scene.add(treat);
{ const b = new THREE.Mesh(new THREE.CapsuleGeometry(0.18, 0.5, 4, 8), M(0xe8c8a0, 0.6)); b.rotation.z = Math.PI / 2; b.position.y = 0.4; treat.add(b); for (const s of [-1, 1]) { const k = new THREE.Mesh(new THREE.SphereGeometry(0.2, 10, 8), M(0xe8c8a0, 0.6)); k.position.set(s * 0.42, 0.4, 0); treat.add(k); } }
treat.visible = false;

// Benny
const benny = buildDog({ breed: 'gsd', neck: 'red-collar', carry: 'none' });
benny.scale.setScalar(3.4); scene.add(benny);
// a soft halo so he is easy to spot from above
const halo = new THREE.Mesh(new THREE.RingGeometry(1.0, 1.35, 32), new THREE.MeshBasicMaterial({ color: 0xffb030, transparent: true, opacity: 0.85, depthWrite: false }));
halo.rotation.x = -Math.PI / 2; scene.add(halo);

// ------------------------------------------------------------------ state
let level = 0, seed = seedFromUrl(1909), route = [], phase = 'demo', demoT = 0, stepIdx = 0, mistakes = 0, here = null, walk = null, levelT = 0;
const meta = load(SAVE_KEY, { walks: 0 });
const DEMO_STEP = 0.95;

function startLevel(i) {
  level = i;
  route = R.makeRoute(R.LEVELS[level], seed + level * 13);
  for (const c of [...trail.children]) { trail.remove(c); c.geometry.dispose(); }
  phase = 'demo'; demoT = 0; stepIdx = 0; mistakes = 0; levelT = 0; walk = null;
  here = route[0];
  benny.position.copy(pos(here));
  treat.visible = false;
  $('from').textContent = R.placeName(route[0]);
  $('to').textContent = R.placeName(route[route.length - 1]);
  $('lvl').textContent = `Walk ${level + 1} of ${R.LEVELS.length} · ${route.length - 1} blocks`;
  $('phase').textContent = 'Watch where Benny goes...';
  $('done').hidden = true;
  frame();
}
function frame() {
  const span = Math.max(W, H) + 18;
  const vfov = THREE.MathUtils.degToRad(camera.fov);
  const d = (span / 2) / Math.tan(vfov / 2) / Math.min(1, camera.aspect) * 1.0;
  camera.position.set(0, d * 0.86, d * 0.55); camera.lookAt(0, 0, 1.5); camera.updateMatrixWorld();
}

function headTo(p, done) { walk = { from: benny.position.clone(), to: pos(p), t: 0, done }; }
function tryStep(p) {
  if (phase !== 'walk' || walk) return 'busy';
  if (Math.abs(p[0] - here[0]) + Math.abs(p[1] - here[1]) !== 1) return 'not-adjacent';
  const next = route[stepIdx + 1];
  if (R.same(p, next)) {
    stepIdx++;
    const from = here; here = p;
    headTo(p, () => {
      const a = pos(from), b = pos(p);
      for (let k = 1; k <= 4; k++) dropMark(a.clone().lerp(b, k / 5).add(new THREE.Vector3((k % 2 ? 0.25 : -0.25), 0, 0)), pawMat, 0.16);
      if (stepIdx === route.length - 1) finish();
    });
    sfx.tone(520 + stepIdx * 25, 0.12, { type: 'triangle', vol: 0.06 });
    return 'step';
  }
  mistakes++;
  sfx.soft();
  // Benny takes half a step, looks puzzled, comes back
  const look = pos(here).lerp(pos(p), 0.3);
  walk = { from: benny.position.clone(), to: look, t: 0, back: true };
  toast(mistakesHere() >= 2 ? `Benny sniffs toward ${dirName(here, next)}...` : 'Benny looks at you. That is not the way you went.', 2400);
  return 'wrong';
}
let wrongAtStep = { i: -1, n: 0 };
function mistakesHere() { if (wrongAtStep.i !== stepIdx) wrongAtStep = { i: stepIdx, n: 0 }; return ++wrongAtStep.n; }
function dirName(a, b) { const dx = b[0] - a[0], dy = b[1] - a[1]; return dx > 0 ? R.EW_STREETS[a[1]] + ' eastward' : dx < 0 ? R.EW_STREETS[a[1]] + ' westward' : dy > 0 ? R.NS_STREETS[a[0]] + ' southward' : R.NS_STREETS[a[0]] + ' northward'; }
function finish() {
  phase = 'done';
  treat.visible = true; treat.position.copy(pos(here)).add(new THREE.Vector3(0.8, 0, 0.8));
  meta.walks++; save(SAVE_KEY, meta);
  sfx.done();
  $('done-text').textContent = `${route.length - 1} blocks to ${R.placeName(here)}, ${mistakes ? `${mistakes} wrong turn${mistakes > 1 ? 's' : ''}` : 'not a single wrong turn'}. Benny gets a treat.`;
  $('next').textContent = level + 1 < R.LEVELS.length ? 'A longer walk' : 'New walks';
}

// input: click an intersection, or arrow keys
canvas.addEventListener('pointerdown', (e) => {
  if (api.isAuto()) return;
  const ray = rayAt(e.clientX, e.clientY, camera); const p = new THREE.Vector3();
  if (!ray.intersectPlane(new THREE.Plane(new THREE.Vector3(0, 1, 0), 0), p)) return;
  const gx = Math.round((p.x + W / 2) / SP), gy = Math.round((p.z + H / 2) / SP);
  if (R.inGrid([gx, gy]) && pos([gx, gy]).distanceTo(p) < 2.6) tryStep([gx, gy]);
});
addEventListener('keydown', (e) => {
  const d = { ArrowUp: R.DIRS.N, ArrowDown: R.DIRS.S, ArrowLeft: R.DIRS.W, ArrowRight: R.DIRS.E }[e.key];
  if (d && here) { e.preventDefault(); const p = [here[0] + d[0], here[1] + d[1]]; if (R.inGrid(p)) tryStep(p); }
});
$('replay').addEventListener('click', () => { if (phase === 'walk' && stepIdx === 0) { phase = 'demo'; demoT = 0; for (const c of [...trail.children]) { trail.remove(c); c.geometry.dispose(); } } });
$('next').addEventListener('click', () => { if (level + 1 < R.LEVELS.length) startLevel(level + 1); else { seed = (seed * 7 + 11) % 2147483647; startLevel(0); } });

// ------------------------------------------------------------------ loop
function tick(dt) {
  levelT += dt;
  if (phase === 'demo') {
    demoT += dt;
    const k = demoT / DEMO_STEP, i = Math.min(route.length - 1, Math.floor(k)), f = Math.min(1, k - i);
    const a = pos(route[i]), b = pos(route[Math.min(route.length - 1, i + 1)]);
    benny.position.copy(a).lerp(b, f);
    face(b.clone().sub(a));
    if (Math.floor(demoT / 0.16) > trail.children.length && i < route.length - 1) dropMark(benny.position.clone(), dotMat, 0.38);
    if (k >= route.length - 1 + 0.6) {
      // fade the trail and send Benny back to the start
      dotMat.opacity = Math.max(0, dotMat.opacity - dt * 0.9);
      if (dotMat.opacity <= 0) {
        for (const c of [...trail.children]) { trail.remove(c); c.geometry.dispose(); }
        dotMat.opacity = 0.9;
        phase = 'walk'; here = route[0]; benny.position.copy(pos(here));
        $('phase').textContent = 'Your turn. Lead the way.';
      }
    }
  }
  if (walk) {
    walk.t = Math.min(1, walk.t + dt * (walk.back ? 2.2 : 1.6));
    const s = walk.back ? Math.sin(walk.t * Math.PI) : walk.t;
    benny.position.copy(walk.from).lerp(walk.to, s);
    face(walk.to.clone().sub(walk.from));
    if (walk.t >= 1) { const d = walk.done; walk = null; if (d) d(); }
  }
  // legs trot while moving
  const moving = phase === 'demo' || !!walk;
  benny.userData.legs.forEach((l, i) => { l.rotation.z = moving ? Math.sin(levelT * 12 + (i % 2 ? Math.PI : 0) + (i > 1 ? 1.5 : 0)) * 0.5 : 0; });
  benny.userData.tail.rotation.x = Math.sin(levelT * (phase === 'done' ? 14 : 5)) * 0.4;
  for (const d of discs) {
    const adj = phase === 'walk' && here && Math.abs(d.userData.at[0] - here[0]) + Math.abs(d.userData.at[1] - here[1]) === 1;
    d.material.opacity = adj ? 0.35 + Math.sin(levelT * 4) * 0.1 : 0;
  }
  halo.position.set(benny.position.x, 0.07, benny.position.z); halo.scale.setScalar(1 + Math.sin(levelT * 3) * 0.06);
  if (treat.visible) treat.rotation.y += dt * 2;
  if (phase === 'done' && $('done').hidden) { doneT += dt; if (doneT > 1.2) { $('done').hidden = false; doneT = 0; } }
  $('steps').textContent = phase === 'walk' || phase === 'done' ? `${stepIdx} / ${route.length - 1}` : '';
}
let doneT = 0;
function face(v) { if (v.lengthSq() > 1e-6) benny.rotation.y = Math.atan2(-v.z, v.x); }
function render() { renderer.render(scene, camera); }
function resize() { renderer.setSize(innerWidth, innerHeight, false); camera.aspect = innerWidth / innerHeight; camera.updateProjectionMatrix(); frame(); }
addEventListener('resize', resize); resize();

// ------------------------------------------------------------------ tour
// Watches the demo, takes one wrong turn on purpose, then walks the route by
// clicking intersections on screen.
const bot = { t: 0, wrongRes: null, results: [], walks: 0 };
function botStep(dt) {
  bot.t += dt;
  if (phase !== 'walk' || walk || bot.t < 0.3) {
    if (phase === 'done' && !$('done').hidden && bot.t > 0.6) { bot.walks++; bot.t = 0; $('next').click(); }
    return;
  }
  bot.t = 0;
  camera.updateMatrixWorld();
  const clickCell = (p) => {
    const s = toScreen(pos(p), camera);
    const ray = rayAt(s.x, s.y, camera); const q = new THREE.Vector3(); ray.intersectPlane(new THREE.Plane(new THREE.Vector3(0, 1, 0), 0), q);
    return tryStep([Math.round((q.x + W / 2) / SP), Math.round((q.z + H / 2) / SP)]);
  };
  if (bot.wrongRes === null) {
    const wrongWay = R.neighbours(here).find((n) => !R.same(n, route[1]));
    bot.wrongRes = clickCell(wrongWay);
    return;
  }
  bot.results.push(clickCell(route[stepIdx + 1]));
}
const checks = [
  { name: 'demo-walk', when: () => phase === 'demo' && demoT > DEMO_STEP * 2.5, check: () => trail.children.length > 5 && !R.same(here, [-1, -1]) },
  { name: 'your-turn', when: () => phase === 'walk', check: () => trail.children.length === 0 && benny.position.distanceTo(pos(route[0])) < 0.01 },
  { name: 'wrong-turn', when: () => bot.wrongRes !== null && !walk, check: () => bot.wrongRes === 'wrong' && stepIdx === 0 && mistakes === 1 && R.same(here, route[0]) },
  { name: 'arrived', when: () => phase === 'done' && level === 0, check: () => R.same(here, route[route.length - 1]) && treat.visible && bot.results.every((r) => r === 'step') },
  { name: 'longer-walk', when: () => phase === 'done' && level === 1, check: () => route.length === R.LEVELS[1] + 1 && bot.results.every((r) => r === 'step') },
];

startLevel(0);
const help = wireHelp();
const api = harness({
  checks, tick, render, selfTest: R.selfTest, bot: botStep,
  paused: () => help.isOpen(),
  extraState: () => ({ level, phase, stepIdx, mistakes }),
  winNote: () => `walked ${R.LEVELS[0]} and ${R.LEVELS[1]} blocks from memory`,
});
if (!VERIFY && meta.walks === 0 && !new URLSearchParams(location.search).has('autoplay')) help.open();
api.start();

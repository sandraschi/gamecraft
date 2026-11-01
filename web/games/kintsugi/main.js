// Kintsugi: a bowl lies in pieces on a cloth. Turn each shard the right way,
// slide it home, and the cracks fill with gold. No timer, no way to lose.
import * as THREE from 'three';
import { VERIFY, $, rng, seedFromUrl, makeRenderer, toScreen, pickAt, rayAt, harness, sfx, toast, wireHelp, load, save } from '../_shared/gc.js';
import { setupMusic } from '../_shared/music.js';
const music = setupMusic('zen', { sfx });

const STEP = Math.PI / 12;        // one turn = 15 degrees
const SNAP_DIST = 0.32;           // how close (xz) a shard must come
const SNAP_ANGLE = STEP * 1.01;   // within one turn of correct
const SAVE_KEY = 'gamecraft.kintsugi.v1';

const BOWLS = [
  { name: 'Chawan', sub: 'tea bowl', shards: 4, glaze: 0x9db8a0, inner: 0xd8e2cc, rim: 0x5a4a3a, profile: (t) => [0.42 + 0.48 * Math.sin(t * Math.PI / 2) ** 0.7, 0.85 * t] },
  { name: 'Donburi', sub: 'rice bowl', shards: 5, glaze: 0x2a4a7a, inner: 0xf0ece0, rim: 0x1a2a4a, profile: (t) => [0.3 + 0.75 * (1 - (1 - t) ** 2), 0.7 * t] },
  { name: 'Hachi', sub: 'serving bowl', shards: 6, glaze: 0xb8643a, inner: 0xe8c8a0, rim: 0x6a2a1a, profile: (t) => [0.38 + 0.82 * Math.sqrt(t), 0.55 * t] },
  { name: 'Yunomi', sub: 'tall cup', shards: 5, glaze: 0x4a4a52, inner: 0xd8d0c0, rim: 0x2a2a2a, profile: (t) => [0.4 + 0.12 * t, 1.0 * t] },
];
const T0 = 0.16;   // the foot (base ring) stays in place

const canvas = $('view');
const renderer = makeRenderer(canvas, { shadows: true, clear: 0x15100c });
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x15100c);
const camera = new THREE.PerspectiveCamera(40, 1, 0.1, 60);
camera.position.set(0, 4.6, 5.4); camera.lookAt(0, 0.2, 0.3);

scene.add(new THREE.HemisphereLight(0xfff0dc, 0x2a1a10, 0.7));
const key = new THREE.SpotLight(0xffe8c8, 60, 20, 0.6, 0.5, 1.5);
key.position.set(-2.5, 7, 3); key.target.position.set(0, 0, 0);
key.castShadow = true; key.shadow.mapSize.set(VERIFY ? 512 : 2048, VERIFY ? 512 : 2048); key.shadow.bias = -0.0004;
scene.add(key, key.target);
const rimLight = new THREE.DirectionalLight(0x9ab8ff, 0.6); rimLight.position.set(4, 3, -5); scene.add(rimLight);

// table and indigo cloth
const table = new THREE.Mesh(new THREE.BoxGeometry(14, 0.4, 9), new THREE.MeshStandardMaterial({ color: 0x4a2e1a, roughness: 0.6 }));
table.position.y = -0.2; table.receiveShadow = true; scene.add(table);
const clothTex = (() => {
  const c = document.createElement('canvas'); c.width = c.height = 64;
  const x = c.getContext('2d'); x.fillStyle = '#1e2a4a'; x.fillRect(0, 0, 64, 64);
  x.strokeStyle = 'rgba(220,230,255,0.18)'; x.lineWidth = 2;
  for (let i = -64; i < 128; i += 16) { x.beginPath(); x.moveTo(i, 0); x.lineTo(i + 64, 64); x.stroke(); x.beginPath(); x.moveTo(i + 64, 0); x.lineTo(i, 64); x.stroke(); }
  const t = new THREE.CanvasTexture(c); t.wrapS = t.wrapT = THREE.RepeatWrapping; t.repeat.set(8, 6); t.colorSpace = THREE.SRGBColorSpace; return t;
})();
const cloth = new THREE.Mesh(new THREE.PlaneGeometry(9, 6.5), new THREE.MeshStandardMaterial({ map: clothTex, roughness: 0.95 }));
cloth.rotation.x = -Math.PI / 2; cloth.position.y = 0.002; cloth.receiveShadow = true; scene.add(cloth);
const dragPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);

// ------------------------------------------------------------------ fracture (pure)
/** Crack lines between shards: phi(t) for each boundary, jagged but never crossing. */
export function planFracture(seed, n) {
  const r = rng(seed);
  const base = r.range(0, Math.PI * 2);
  const bounds = [];
  for (let i = 0; i < n; i++) {
    const phi0 = base + (i / n) * Math.PI * 2 + r.range(-0.25, 0.25) * (Math.PI * 2 / n);
    const waves = [0, 1, 2].map(() => ({ a: r.range(0.03, 0.09), f: r.range(2, 9), p: r.range(0, 6.28) }));
    bounds.push({ phi0, waves });
  }
  const phiAt = (b, t) => b.phi0 + b.waves.reduce((s, w) => s + w.a * Math.sin(w.f * t * Math.PI + w.p), 0);
  // scatter spots and starting turns
  const spots = [], turns = [];
  for (let i = 0; i < n; i++) {
    let a = 0, rad = 0;
    for (let k = 0; k < 300; k++) {
      a = r.range(0, Math.PI * 2); rad = r.range(2.3, 3.4);
      const x = Math.cos(a) * rad * 1.15, z = Math.sin(a) * rad * 0.75;
      if (z > 2.6 || z < -2.4 || Math.abs(x) > 4.0) continue;
      if (spots.every((s) => Math.hypot(s.x - x, s.z - z) > 1.5)) { spots.push({ x, z }); break; }
    }
    if (spots.length <= i) spots.push({ x: -3.6 + i * 1.6, z: 2.3 });
    let turn; do { turn = r.int(24) - 12; } while (Math.abs(turn) < 3);
    turns.push({ turn, flip: r() < 0.4 });
  }
  return { bounds, phiAt, spots, turns };
}

// ------------------------------------------------------------------ geometry
const THICK = 0.06;
function shardGeometry(profile, phiA, phiB, t0, t1, rows = 18, cols = 14) {
  const pos = [], idx = [];
  const at = (t, u, inner) => {
    const [rr, y] = profile(t);
    const phi = phiA(t) + (phiB(t) - phiA(t)) * u;
    const R = inner ? rr - THICK : rr;
    return [R * Math.cos(phi), inner ? y + THICK * 0.6 : y, R * Math.sin(phi)];
  };
  function grid(fn, nr, nc, flipWind) {
    const start = pos.length / 3;
    for (let i = 0; i <= nr; i++) for (let j = 0; j <= nc; j++) pos.push(...fn(i / nr, j / nc));
    for (let i = 0; i < nr; i++) for (let j = 0; j < nc; j++) {
      const a = start + i * (nc + 1) + j, b = a + 1, c = a + nc + 1, d = c + 1;
      if (flipWind) idx.push(a, c, b, b, c, d); else idx.push(a, b, c, b, d, c);
    }
  }
  const T = (v) => t0 + (t1 - t0) * v;
  grid((v, u) => at(T(v), u, false), rows, cols, false);           // outside
  grid((v, u) => at(T(v), u, true), rows, cols, true);             // inside
  grid((s, u) => { const o = at(t1, u, false), n = at(t1, u, true); return o.map((x, k) => x + (n[k] - x) * s); }, 1, cols, false); // rim
  grid((s, u) => { const o = at(t0, u, false), n = at(t0, u, true); return o.map((x, k) => x + (n[k] - x) * s); }, 1, cols, true);  // lower edge
  grid((v, s) => { const o = at(T(v), 0, false), n = at(T(v), 0, true); return o.map((x, k) => x + (n[k] - x) * s); }, rows, 1, true); // crack A
  grid((v, s) => { const o = at(T(v), 1, false), n = at(T(v), 1, true); return o.map((x, k) => x + (n[k] - x) * s); }, rows, 1, false); // crack B
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  g.setIndex(idx);
  g.computeVertexNormals();
  return g;
}
function seamCurve(profile, phiFn, t0, t1) {
  const pts = [];
  for (let i = 0; i <= 40; i++) { const t = t0 + (t1 - t0) * (i / 40); const [rr, y] = profile(t); const phi = phiFn(t); pts.push(new THREE.Vector3((rr + 0.004) * Math.cos(phi), y, (rr + 0.004) * Math.sin(phi))); }
  return new THREE.CatmullRomCurve3(pts);
}

// ------------------------------------------------------------------ level state
const bowlGroup = new THREE.Group(); scene.add(bowlGroup);
const gold = new THREE.MeshStandardMaterial({ color: 0xf2c24a, metalness: 1, roughness: 0.25, emissive: 0x5a3a00, emissiveIntensity: 0.4 });
let level = 0, seed = seedFromUrl(1203);
let shards = [];        // { mesh, home: Vector3, turn, flip, placed, i }
let seams = [];         // { mesh, a, b, grow }
let selected = null, dragging = false;
let turnsUsed = 0, bowlDone = false, doneT = 0, levelT = 0;
const meta = load(SAVE_KEY, { bowls: 0 });

function clearBowl() {
  for (const c of [...bowlGroup.children]) { bowlGroup.remove(c); c.traverse?.((o) => { if (o.isMesh) o.geometry.dispose(); }); }
  for (const s of shards) { scene.remove(s.mesh); s.mesh.geometry.dispose(); }
  shards = []; seams = [];
}
function shardMaterials(b) {
  return new THREE.MeshPhysicalMaterial({ color: b.glaze, roughness: 0.32, clearcoat: 0.7, clearcoatRoughness: 0.25, side: THREE.DoubleSide });
}
function startBowl(idx) {
  clearBowl();
  level = idx;
  const b = BOWLS[level];
  const fr = planFracture(seed * 13 + level * 7, b.shards);
  bowlGroup.rotation.y = 0;
  // the foot ring, already in place
  const foot = new THREE.Mesh(shardGeometry(b.profile, () => 0, () => Math.PI * 2, 0, T0, 4, 48), shardMaterials(b));
  foot.castShadow = true; foot.receiveShadow = true; bowlGroup.add(foot);
  const bottom = new THREE.Mesh(new THREE.CircleGeometry(b.profile(0)[0], 40), new THREE.MeshPhysicalMaterial({ color: b.inner, roughness: 0.4, clearcoat: 0.5 }));
  bottom.rotation.x = -Math.PI / 2; bottom.position.y = THICK * 0.6 + 0.002; bowlGroup.add(bottom);
  const n = b.shards;
  for (let i = 0; i < n; i++) {
    const A = fr.bounds[i], B = fr.bounds[(i + 1) % n];
    const wrap = i === n - 1 ? Math.PI * 2 : 0;
    const geo = shardGeometry(b.profile, (t) => fr.phiAt(A, t), (t) => fr.phiAt(B, t) + wrap, T0, 1);
    geo.computeBoundingBox();
    const c = geo.boundingBox.getCenter(new THREE.Vector3());
    geo.translate(-c.x, -c.y, -c.z);
    const mesh = new THREE.Mesh(geo, shardMaterials(b));
    mesh.castShadow = true; mesh.receiveShadow = true;
    const sh = { mesh, home: c.clone(), turn: fr.turns[i].turn, flip: fr.turns[i].flip, placed: false, i, glow: 0 };
    mesh.userData.shard = sh;
    applyPose(sh);
    const sp = fr.spots[i];
    mesh.position.set(sp.x, 0, sp.z); restOnCloth(sh);
    scene.add(mesh); shards.push(sh);
  }
  // seams: one per crack line plus the ring around the foot, hidden until both sides are home
  for (let i = 0; i < n; i++) {
    const curve = seamCurve(b.profile, (t) => fr.phiAt(fr.bounds[i], t) + (i === 0 ? 0 : 0), T0, 1);
    const m = new THREE.Mesh(new THREE.TubeGeometry(curve, 60, 0.018, 6, false), gold);
    m.geometry.setDrawRange(0, 0); bowlGroup.add(m);
    seams.push({ mesh: m, a: (i - 1 + n) % n, b: i, grow: 0, total: m.geometry.index.count });
  }
  const [rf, yf] = b.profile(T0);
  const ringPts = []; for (let k = 0; k <= 64; k++) { const p = (k / 64) * Math.PI * 2; ringPts.push(new THREE.Vector3((rf + 0.004) * Math.cos(p), yf, (rf + 0.004) * Math.sin(p))); }
  const ring = new THREE.Mesh(new THREE.TubeGeometry(new THREE.CatmullRomCurve3(ringPts, true), 96, 0.016, 6, true), gold);
  ring.geometry.setDrawRange(0, 0); bowlGroup.add(ring);
  seams.push({ mesh: ring, a: -1, b: -1, grow: 0, total: ring.geometry.index.count, ring: true });
  selected = null; turnsUsed = 0; bowlDone = false; doneT = 0; levelT = 0;
  $('bowl-name').textContent = `${b.name}`;
  $('bowl-sub').textContent = b.sub;
  $('done').hidden = true;
  hud();
}

function applyPose(sh) {
  sh.mesh.rotation.set(sh.flip ? Math.PI : 0, sh.turn * STEP, 0, 'YXZ');
}
const _bb = new THREE.Box3();
function restOnCloth(sh) {
  sh.mesh.position.y = 0; sh.mesh.updateMatrixWorld(true);
  _bb.setFromObject(sh.mesh);
  sh.mesh.position.y = -_bb.min.y + 0.002;
}
/** Pure snap rule, shared by play and the self-test. */
export function wouldSnap(dx, dz, turn, flip) {
  const t = ((turn % 24) + 24) % 24;
  const off = Math.min(t, 24 - t) * STEP;
  return !flip && off < SNAP_ANGLE && Math.hypot(dx, dz) < SNAP_DIST;
}

// ------------------------------------------------------------------ actions
function select(sh) {
  if (bowlDone) return false;
  if (selected && selected !== sh) selected.mesh.material.emissive.setHex(0);
  selected = sh && !sh.placed ? sh : null;
  if (selected) { selected.mesh.material.emissive.setHex(0x3a2a10); sfx.soft(); }
  hud();
  return !!selected;
}
function turn(dir) {
  if (!selected) return;
  selected.turn += dir; turnsUsed++;
  applyPose(selected);
  if (!dragging) restOnCloth(selected);
  sfx.tone(dir > 0 ? 520 : 470, 0.08, { type: 'triangle', vol: 0.05 });
  hud();
}
function flip() {
  if (!selected) return;
  selected.flip = !selected.flip; turnsUsed++;
  applyPose(selected);
  if (!dragging) restOnCloth(selected);
  sfx.tone(380, 0.12, { type: 'triangle', vol: 0.06 });
  hud();
}
const _hit = new THREE.Vector3();
function moveTo(x, y) {
  if (!selected) return;
  const ray = rayAt(x, y, camera);
  if (!ray.intersectPlane(dragPlane, _hit)) return;
  selected.mesh.position.x = THREE.MathUtils.clamp(_hit.x, -4.4, 4.4);
  selected.mesh.position.z = THREE.MathUtils.clamp(_hit.z, -3.1, 3.1);
  const near = Math.hypot(selected.mesh.position.x - selected.home.x, selected.mesh.position.z - selected.home.z) < SNAP_DIST * 2.5;
  selected.mesh.position.y = near ? Math.max(selected.home.y + 0.15, 0.6) : 0.5;
}
function release() {
  if (!selected) return 'none';
  const sh = selected;
  dragging = false;
  if (wouldSnap(sh.mesh.position.x - sh.home.x, sh.mesh.position.z - sh.home.z, sh.turn, sh.flip)) {
    sh.placed = true; sh.turn = 0; applyPose(sh);
    sh.mesh.material.emissive.setHex(0);
    sh.fly = { from: sh.mesh.position.clone(), t: 0 };
    selected = null;
    sfx.tone(784, 0.5, { vol: 0.1 }); sfx.tone(1175, 0.7, { vol: 0.06, at: 0.08 });
    if (shards.every((s) => s.placed)) finishBowl();
    hud();
    return 'placed';
  }
  restOnCloth(sh);
  if (Math.hypot(sh.mesh.position.x - sh.home.x, sh.mesh.position.z - sh.home.z) < SNAP_DIST * 2) toast(sh.flip ? 'Right place, but it is upside down.' : 'Right place. Turn it until the edges match.');
  hud();
  return 'rested';
}
function finishBowl() {
  bowlDone = true; doneT = 0;
  meta.bowls++; save(SAVE_KEY, meta);
  setTimeout(() => sfx.done(), 300);
}

// pointer
let down = null;
canvas.addEventListener('pointerdown', (e) => {
  if (api.isAuto()) return;
  down = { x: e.clientX, y: e.clientY };
  const hit = pickAt(e.clientX, e.clientY, camera, shards.filter((s) => !s.placed).map((s) => s.mesh))[0];
  if (hit) { select(hit.object.userData.shard); dragging = true; canvas.setPointerCapture(e.pointerId); }
  else select(null);
});
canvas.addEventListener('pointermove', (e) => { if (dragging && !api.isAuto()) moveTo(e.clientX, e.clientY); });
canvas.addEventListener('pointerup', () => { if (dragging && !api.isAuto()) release(); });
canvas.addEventListener('wheel', (e) => { if (selected) { e.preventDefault(); turn(e.deltaY > 0 ? 1 : -1); } }, { passive: false });
addEventListener('keydown', (e) => {
  if (e.key === 'q' || e.key === 'Q' || e.key === 'ArrowLeft') turn(-1);
  if (e.key === 'e' || e.key === 'E' || e.key === 'ArrowRight') turn(1);
  if (e.key === 'f' || e.key === 'F' || e.key === ' ') { e.preventDefault(); flip(); }
});
$('turn-l').addEventListener('click', () => turn(-1));
$('turn-r').addEventListener('click', () => turn(1));
$('flip').addEventListener('click', flip);
$('next').addEventListener('click', () => { if (level + 1 < BOWLS.length) startBowl(level + 1); else { seed = (seed * 7 + 3) % 2147483647; startBowl(0); } });

// ------------------------------------------------------------------ sim, render
function tick(dt) {
  levelT += dt;
  for (const s of shards) {
    if (s.fly) {
      s.fly.t = Math.min(1, s.fly.t + dt * 2.2);
      const k = 1 - (1 - s.fly.t) ** 3;
      s.mesh.position.lerpVectors(s.fly.from, s.home, k);
      s.mesh.position.y += Math.sin(k * Math.PI) * 0.4;
      if (s.fly.t >= 1) { s.mesh.position.copy(s.home); s.fly = null; }
    }
  }
  // gold flows into a crack once both neighbours are home
  for (const sm of seams) {
    const ready = sm.ring ? bowlDone : (shards[sm.a]?.placed && shards[sm.b]?.placed && !shards[sm.a].fly && !shards[sm.b].fly);
    if (ready && sm.grow < 1) { sm.grow = Math.min(1, sm.grow + dt * 0.7); sm.mesh.geometry.setDrawRange(0, Math.floor(sm.total * sm.grow / 6) * 6); }
  }
  if (bowlDone) {
    doneT += dt;
    // placed shards belong to the bowl now, so they turn with it
    for (const s of shards) if (s.mesh.parent !== bowlGroup && !s.fly) { bowlGroup.attach(s.mesh); }
    bowlGroup.rotation.y += dt * 0.35;
    gold.emissiveIntensity = 0.4 + Math.sin(doneT * 2) * 0.15;
    if (doneT > 2.4 && $('done').hidden) {
      $('done-text').textContent = `${BOWLS[level].shards} shards, ${turnsUsed} turns. The cracks are part of its story now.`;
      $('next').textContent = level + 1 < BOWLS.length ? 'Next bowl' : 'Start again with new cracks';
      $('done').hidden = false;
    }
  } else gold.emissiveIntensity = 0.4;
}
function render() { renderer.render(scene, camera); }
function hud() {
  $('left').textContent = `${shards.filter((s) => !s.placed).length} shards left`;
  const on = !!selected && !bowlDone;
  for (const id of ['turn-l', 'turn-r', 'flip']) $(id).disabled = !on;
}
function resize() {
  renderer.setSize(innerWidth, innerHeight, false);
  camera.aspect = innerWidth / innerHeight;
  camera.fov = camera.aspect < 1 ? 62 : 40;
  camera.updateProjectionMatrix();
}
addEventListener('resize', resize); resize();

// ------------------------------------------------------------------ tour
// The bot works like a person: pick a shard, turn it with the buttons, drag it home.
const bot = { t: 0, phase: 'first-wrong', wrongRes: null, results: [] };
function botStep(dt) {
  bot.t += dt;
  if (bot.t < 0.45) return;
  bot.t = 0;
  camera.updateMatrixWorld();
  if (bot.phase === 'first-wrong') {
    // carry a shard home without turning it first: it must not snap
    const s = shards.find((x) => !x.placed);
    const p = toScreen(s.mesh.position, camera);
    const hit = pickAt(p.x, p.y, camera, [s.mesh])[0];
    if (!hit) { bot.wrongRes = 'pick failed'; bot.phase = 'solve'; return; }
    select(s); dragging = true;
    const h = toScreen(new THREE.Vector3(s.home.x, 0, s.home.z), camera); moveTo(h.x, h.y);
    bot.wrongRes = release();
    bot.phase = 'solve';
    return;
  }
  if (bot.phase === 'solve') {
    const s = shards.find((x) => !x.placed);
    if (!s) { bot.phase = 'admire'; return; }
    const p = toScreen(s.mesh.position, camera);
    if (!pickAt(p.x, p.y, camera, [s.mesh])[0]) { bot.miss = (bot.miss ?? 0) + 1; }
    select(s);
    if (s.flip) $('flip').click();
    const t = ((s.turn % 24) + 24) % 24;
    const n = t <= 12 ? t : 24 - t, dir = t <= 12 ? -1 : 1;
    for (let k = 0; k < n; k++) (dir < 0 ? $('turn-l') : $('turn-r')).click();
    dragging = true;
    const h = toScreen(new THREE.Vector3(s.home.x, 0, s.home.z), camera); moveTo(h.x, h.y);
    bot.results.push(release());
    return;
  }
  if (bot.phase === 'admire' && !$('done').hidden) { bot.level0 = level; $('next').click(); bot.phase = 'next'; }
}
const goldGrown = () => seams.every((s) => s.grow >= 1);
const checks = [
  { name: 'shattered', when: () => true, check: () => shards.length === BOWLS[0].shards && shards.every((s) => !wouldSnap(s.mesh.position.x - s.home.x, s.mesh.position.z - s.home.z, s.turn, s.flip)) },
  { name: 'unturned-shard', when: () => bot.wrongRes !== null, check: () => bot.wrongRes === 'rested' && shards.every((s) => !s.placed) },
  { name: 'first-shard', when: () => shards.some((s) => s.placed && !s.fly), check: () => shards.filter((s) => s.placed).length === 1 && bot.results[0] === 'placed' },
  { name: 'gold-seams', when: () => bowlDone && goldGrown(), check: () => shards.every((s) => s.placed) && seams.length === BOWLS[0].shards + 1 && bot.results.every((r) => r === 'placed') },
  { name: 'next-bowl', when: () => bot.phase === 'next', check: () => level === 1 && shards.length === BOWLS[1].shards && !bowlDone },
];

function selfTest() {
  const c = { cracks_never_cross: true, shards_never_start_solved: true, scatter_on_cloth: true, scatter_apart: true, right_pose_snaps: true, wrong_turn_rejected: true, flipped_rejected: true, far_rejected: true };
  for (let s = 1; s <= 80; s++) {
    const n = BOWLS[s % BOWLS.length].shards;
    const fr = planFracture(s * 7907, n);
    for (let i = 0; i < n; i++) {
      const A = fr.bounds[i], B = fr.bounds[(i + 1) % n];
      for (let k = 0; k <= 20; k++) {
        const t = T0 + (1 - T0) * k / 20;
        const width = fr.phiAt(B, t) + (i === n - 1 ? Math.PI * 2 : 0) - fr.phiAt(A, t);
        if (width < 0.35) c.cracks_never_cross = false;
      }
      const tr = fr.turns[i];
      if (wouldSnap(0, 0, tr.turn, tr.flip)) c.shards_never_start_solved = false;
      const sp = fr.spots[i];
      if (Math.abs(sp.x) > 4.2 || sp.z > 2.8 || sp.z < -2.8) c.scatter_on_cloth = false;
      for (let j = 0; j < i; j++) if (Math.hypot(fr.spots[j].x - sp.x, fr.spots[j].z - sp.z) < 1.4) c.scatter_apart = false;
    }
  }
  if (!wouldSnap(0.1, -0.1, 0, false) || !wouldSnap(0, 0, 24, false)) c.right_pose_snaps = false;
  if (wouldSnap(0, 0, 2, false) || wouldSnap(0, 0, -3, false)) c.wrong_turn_rejected = false;
  if (wouldSnap(0, 0, 0, true)) c.flipped_rejected = false;
  if (wouldSnap(0.5, 0, 0, false)) c.far_rejected = false;
  return { pass: Object.values(c).every(Boolean), checks: c, summary: '80 fractures; snap accepts only the true pose' };
}

startBowl(0);
const help = wireHelp();
const api = harness({
  checks, tick, render, selfTest, bot: botStep,
  paused: () => help.isOpen(),
  extraState: () => ({ level, placed: shards.filter((s) => s.placed).length, turnsUsed, botMisses: bot.miss ?? 0 }),
  winNote: () => `first bowl mended with gold (${BOWLS[0].shards} shards), second bowl ready`,
});
if (!VERIFY && meta.bowls === 0 && !new URLSearchParams(location.search).has('autoplay')) help.open();
api.start();

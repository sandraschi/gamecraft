// Where is Benny? A sunny funfair, a crowd, lots of dogs, several German
// Shepherds, and exactly one of them is Benny. The poster says what he is wearing
// and carrying today; the look-alikes each get one detail wrong.
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { VERIFY, $, rng, seedFromUrl, makeRenderer, toScreen, pickAt, harness, sfx, toast, wireHelp, load, save } from '../_shared/gc.js';
import { setupMusic } from '../_shared/music.js';
const music = setupMusic('funfair', { sfx });
import { NECKWEAR, CARRY, castDogs, isBenny, whyNot, buildDog } from './dogs.js';

const ROUNDS = [{ people: 130, dogs: 24 }, { people: 200, dogs: 34 }, { people: 270, dogs: 44 }];
const AREA = { x0: -27, x1: 27, z0: -14, z1: 14 };
const HINT_AFTER = 30;
const SAVE_KEY = 'gamecraft.benny.v1';

// ------------------------------------------------------------------ renderer, scene
const canvas = $('view');
const renderer = makeRenderer(canvas, { shadows: !VERIFY, clear: 0x9ad0f0 });
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x9ad0f0);
scene.fog = new THREE.Fog(0x9ad0f0, 70, 140);
const camera = new THREE.PerspectiveCamera(45, innerWidth / innerHeight, 0.2, 300);
camera.position.set(0, 30, 36);
const controls = new OrbitControls(camera, canvas);
controls.target.set(0, 0, 0);
controls.enableDamping = !VERIFY;
controls.maxPolarAngle = 1.38; controls.minDistance = 3; controls.maxDistance = 75;
controls.screenSpacePanning = false;
controls.update();

scene.add(new THREE.HemisphereLight(0xdff2ff, 0x6a8a4a, 1.3));
const sun = new THREE.DirectionalLight(0xfff2d8, 2.2);
sun.position.set(-20, 40, 18);
if (!VERIFY) {
  sun.castShadow = true; sun.shadow.mapSize.set(2048, 2048);
  Object.assign(sun.shadow.camera, { left: -36, right: 36, top: 26, bottom: -26, near: 1, far: 120 });
  sun.shadow.bias = -0.0005;
}
scene.add(sun);

const M = (c, r = 0.85) => new THREE.MeshStandardMaterial({ color: c, roughness: r });
function mesh(geo, mat, x, y, z, parent = scene) { const m = new THREE.Mesh(geo, mat); m.position.set(x, y, z); m.castShadow = true; m.receiveShadow = true; parent.add(m); return m; }

// ground
const grass = new THREE.Mesh(new THREE.PlaneGeometry(220, 160), M(0x7cba5a));
grass.rotation.x = -Math.PI / 2; grass.receiveShadow = true; scene.add(grass);
const plaza = new THREE.Mesh(new THREE.PlaneGeometry(60, 34), M(0xe6d6b4));
plaza.rotation.x = -Math.PI / 2; plaza.position.y = 0.01; plaza.receiveShadow = true; scene.add(plaza);

// stalls with striped awnings
function stripes(a, b) {
  const c = document.createElement('canvas'); c.width = 128; c.height = 16;
  const x = c.getContext('2d');
  for (let i = 0; i < 8; i++) { x.fillStyle = i % 2 ? a : b; x.fillRect(i * 16, 0, 16, 16); }
  const t = new THREE.CanvasTexture(c); t.colorSpace = THREE.SRGBColorSpace; return t;
}
const STALLS = [
  { name: 'the Langos stand', x: -18, z: -17, a: '#d8342c', b: '#fff6e8', goods: 0xe8b04a },
  { name: 'the candy floss stall', x: -6, z: -17, a: '#f27aa8', b: '#fff6e8', goods: 0xf6b8d0 },
  { name: 'the gingerbread hearts', x: 6, z: -17, a: '#2a62c8', b: '#fff6e8', goods: 0xa85a2a },
  { name: 'the lucky-dip booth', x: 18, z: -17, a: '#2aa04a', b: '#fff6e8', goods: 0xf2c230 },
  { name: 'the balloon seller', x: -12, z: 17, a: '#f2a02a', b: '#fff6e8', goods: 0xd8342c },
  { name: 'the roast chestnut stand', x: 12, z: 17, a: '#7a3ac8', b: '#fff6e8', goods: 0x6a3a1a },
];
const obstacles = []; // {x0,x1,z0,z1} or {cx,cz,r}
for (const s of STALLS) {
  const g = new THREE.Group(); g.position.set(s.x, 0, s.z); if (s.z > 0) g.rotation.y = Math.PI; scene.add(g);
  mesh(new THREE.BoxGeometry(5, 1.1, 1.6), M(0xb07a4a), 0, 0.55, 0, g);
  mesh(new THREE.BoxGeometry(5, 0.1, 1.8), M(0xf2e8d0), 0, 1.15, 0.1, g);
  for (let i = 0; i < 6; i++) mesh(new THREE.SphereGeometry(0.2, 10, 8), M(s.goods), -2 + i * 0.8, 1.38, 0.2, g);
  for (const px of [-2.4, 2.4]) for (const pz of [-0.8, 0.8]) mesh(new THREE.CylinderGeometry(0.06, 0.06, 2.8, 8), M(0xf2f2f2), px, 1.4, pz, g);
  const aw = mesh(new THREE.BoxGeometry(5.4, 0.08, 2.6), new THREE.MeshStandardMaterial({ map: stripes(s.a, s.b), roughness: 0.8 }), 0, 2.85, 0.4, g);
  aw.rotation.x = 0.22;
  obstacles.push({ x0: s.x - 2.9, x1: s.x + 2.9, z0: s.z - 1.4, z1: s.z + 1.4 });
}

// carousel
const carousel = new THREE.Group(); scene.add(carousel);
mesh(new THREE.CylinderGeometry(4.6, 4.8, 0.5, 32), M(0xf2e2c0), 0, 0.25, 0, carousel);
mesh(new THREE.CylinderGeometry(0.35, 0.35, 4.2, 12), M(0xe8c040, 0.4), 0, 2.4, 0, carousel);
const canopy = mesh(new THREE.ConeGeometry(5.2, 2.0, 16), new THREE.MeshStandardMaterial({ map: stripes('#d8342c', '#fff6e8'), roughness: 0.8 }), 0, 5.4, 0, carousel);
canopy.material.map.wrapS = THREE.RepeatWrapping; canopy.material.map.repeat.set(4, 1);
const horses = [];
for (let i = 0; i < 8; i++) {
  const a = (i / 8) * Math.PI * 2;
  const h = new THREE.Group(); h.position.set(Math.cos(a) * 3.4, 1.4, Math.sin(a) * 3.4); h.rotation.y = -a; carousel.add(h);
  mesh(new THREE.CylinderGeometry(0.04, 0.04, 3.6, 6), M(0xe8c040, 0.4), 0, 0.9, 0, h);
  const col = [0xf6f2ea, 0x8a5a3a, 0x2a2a2a, 0xe8b0c8][i % 4];
  mesh(new THREE.BoxGeometry(0.35, 0.4, 1.1), M(col), 0, 0, 0, h);
  mesh(new THREE.BoxGeometry(0.25, 0.45, 0.3), M(col), 0, 0.35, 0.55, h);
  horses.push(h);
}
obstacles.push({ cx: 0, cz: 0, r: 5.4 });

// Ferris wheel in the background
const wheel = new THREE.Group(); wheel.position.set(10, 12, -34); scene.add(wheel);
const rim = new THREE.Mesh(new THREE.TorusGeometry(10, 0.2, 8, 48), M(0xf2f2f2, 0.4)); wheel.add(rim);
const spin = new THREE.Group(); wheel.add(spin);
const gondolas = [];
for (let i = 0; i < 12; i++) {
  const a = (i / 12) * Math.PI * 2;
  const sp = new THREE.Mesh(new THREE.BoxGeometry(0.12, 10, 0.12), M(0xf2f2f2, 0.4)); sp.rotation.z = a; sp.position.set(Math.cos(a + Math.PI / 2) * 5, Math.sin(a + Math.PI / 2) * 5, 0); spin.add(sp);
  const gd = new THREE.Mesh(new THREE.BoxGeometry(1.4, 1.2, 1.4), M([0xd8342c, 0x2a62c8, 0xf2c230, 0x2aa04a][i % 4])); gd.userData.a = a; spin.add(gd); gondolas.push(gd);
}
for (const s of [-1, 1]) { const leg = mesh(new THREE.BoxGeometry(0.4, 13, 0.4), M(0xd0d0d0), 10 + s * 4, 6, -34); leg.rotation.z = s * 0.3; }

// trees and bunting
const trees = [[-34, -10], [-33, 6], [34, -6], [33, 10], [-25, 24], [26, 24], [-8, 26], [0, -28], [-24, -28], [36, -24]];
for (const [x, z] of trees) {
  mesh(new THREE.CylinderGeometry(0.35, 0.45, 3, 8), M(0x6a4a2a), x, 1.5, z);
  mesh(new THREE.SphereGeometry(2.8, 12, 10), M(0x4a9a3a), x, 4.6, z);
}
const flagGeo = new THREE.ConeGeometry(0.28, 0.5, 3);
const flagCols = [0xd8342c, 0xf2c230, 0x2a62c8, 0x2aa04a, 0xf27aa8];
for (const zLine of [-11, 11]) {
  for (const x of [-27, 27]) mesh(new THREE.CylinderGeometry(0.08, 0.08, 5, 8), M(0xf2f2f2), x, 2.5, zLine);
  const flags = new THREE.InstancedMesh(flagGeo, new THREE.MeshStandardMaterial({ roughness: 0.8 }), 54);
  const m4 = new THREE.Matrix4(), c = new THREE.Color();
  for (let i = 0; i < 54; i++) {
    const x = -26.5 + i, y = 4.8 - Math.sin((i / 53) * Math.PI) * 1.2;
    m4.makeRotationZ(Math.PI); m4.setPosition(x, y, zLine);
    flags.setMatrixAt(i, m4); flags.setColorAt(i, c.setHex(flagCols[i % flagCols.length]));
  }
  scene.add(flags);
}

// ------------------------------------------------------------------ crowd (instanced)
const MAXP = 280;
const SHIRTS = [0xd8342c, 0x2a62c8, 0xf2c230, 0x2aa04a, 0xf2f2f2, 0x1e1e20, 0xf27aa8, 0x7a3ac8, 0xf2782a, 0x3ab8c8, 0x8a5a3a, 0xc8c8d0];
const PANTS = [0x2a3a5a, 0x1e1e20, 0x6a6a72, 0xb8a888, 0x3a4a2a];
const SKIN = [0xf2d0b0, 0xe0b090, 0xc08a60, 0x8a5a3a, 0x5a3a24];
const crowd = {
  body: new THREE.InstancedMesh(new THREE.CapsuleGeometry(0.24, 0.55, 4, 10), new THREE.MeshStandardMaterial({ roughness: 0.85 }), MAXP),
  legs: new THREE.InstancedMesh(new THREE.CylinderGeometry(0.2, 0.17, 0.85, 10), new THREE.MeshStandardMaterial({ roughness: 0.85 }), MAXP),
  head: new THREE.InstancedMesh(new THREE.SphereGeometry(0.15, 12, 10), new THREE.MeshStandardMaterial({ roughness: 0.7 }), MAXP),
  hat: new THREE.InstancedMesh(new THREE.CylinderGeometry(0.12, 0.2, 0.14, 12), new THREE.MeshStandardMaterial({ roughness: 0.8 }), MAXP),
  balloon: new THREE.InstancedMesh(new THREE.SphereGeometry(0.3, 12, 10), new THREE.MeshStandardMaterial({ roughness: 0.3 }), MAXP),
};
for (const im of Object.values(crowd)) { im.castShadow = true; im.frustumCulled = false; scene.add(im); }
const peopleMeshes = Object.values(crowd);

// ------------------------------------------------------------------ placement (pure)
function blocked(x, z, pad = 0.4) {
  if (x < AREA.x0 || x > AREA.x1 || z < AREA.z0 || z > AREA.z1) return true;
  for (const o of obstacles) {
    if (o.r) { if (Math.hypot(x - o.cx, z - o.cz) < o.r + pad) return true; }
    else if (x > o.x0 - pad && x < o.x1 + pad && z > o.z0 - pad && z < o.z1 + pad) return true;
  }
  return false;
}
function freeSpot(r) {
  for (let k = 0; k < 500; k++) { const x = r.range(AREA.x0, AREA.x1), z = r.range(AREA.z0, AREA.z1); if (!blocked(x, z, 1.2)) return { x, z }; }
  return { x: 0, z: 10 };
}
/** Pure round setup: people, dogs, owners, colours. Same seed gives the same fair. */
export function planRound(seed, roundIdx) {
  const r = rng(seed);
  const cfg = ROUNDS[roundIdx];
  const people = [];
  for (let i = 0; i < cfg.people; i++) {
    const p = freeSpot(r);
    people.push({ ...p, tx: p.x, tz: p.z, speed: r.range(0.5, 1.2), wait: r.range(0, 6), still: r() < 0.25,
      h: r.range(0.88, 1.08), shirt: r.pick(SHIRTS), pants: r.pick(PANTS), skin: r.pick(SKIN),
      hat: r() < 0.3 ? r.pick(SHIRTS) : null, balloon: r() < 0.08 ? r.pick(SHIRTS) : null });
  }
  const { want, dogs } = castDogs(r, cfg.dogs);
  const owners = r.shuffle(people.map((_, i) => i)).slice(0, dogs.length);
  const plan = dogs.map((d, i) => ({ ...d, owner: owners[i], side: r() < 0.5 ? -1 : 1, lag: r.range(0.9, 1.5), sit: r() < 0.3 }));
  return { want, people, dogs: plan };
}

// ------------------------------------------------------------------ round state
let round = 0, seed = seedFromUrl(20261101), roundT = 0, sinceHint = 0, wrong = 0, found = false;
let plan = null, people = [], dogs = [];   // dogs: [{ g, spec, x, z, heading, phase }]
let hintShown = false;
const meta = load(SAVE_KEY, { fairs: 0, bestTotal: null });
const times = [];
let dayOver = false;

function clearDogs() { for (const d of dogs) scene.remove(d.g); dogs = []; }
function startRound(idx) {
  round = idx;
  plan = planRound(seed * 31 + idx * 7919, idx);
  people = plan.people.map((p) => ({ ...p }));
  clearDogs();
  for (const spec of plan.dogs) {
    const g = buildDog(spec);
    const o = people[spec.owner];
    const d = { g, spec, x: o.x + spec.side * spec.lag, z: o.z + 0.4, heading: 0, phase: Math.random() * 6 };
    g.position.set(d.x, 0, d.z);
    g.traverse((m) => { if (m.isMesh) m.userData.dog = d; });
    scene.add(g); dogs.push(d);
  }
  const c = new THREE.Color();
  for (let i = 0; i < MAXP; i++) {
    const p = people[i];
    for (const im of peopleMeshes) im.count = people.length;
    if (!p) continue;
    crowd.body.setColorAt(i, c.setHex(p.shirt)); crowd.legs.setColorAt(i, c.setHex(p.pants));
    crowd.head.setColorAt(i, c.setHex(p.skin)); crowd.hat.setColorAt(i, c.setHex(p.hat ?? 0)); crowd.balloon.setColorAt(i, c.setHex(p.balloon ?? 0));
  }
  for (const im of peopleMeshes) if (im.instanceColor) im.instanceColor.needsUpdate = true;
  roundT = 0; sinceHint = 0; wrong = 0; found = false; hintShown = false;
  syncCrowd();
  showPoster();
  $('found').hidden = true;
  camera.position.set(0, 30, 36); controls.target.set(0, 0, 0); controls.update();
  hud();
}

// ------------------------------------------------------------------ simulation
const _m = new THREE.Matrix4(), _q = new THREE.Quaternion(), _s = new THREE.Vector3(), _p = new THREE.Vector3();
const ZERO = new THREE.Vector3(0, 0, 0);
function setInst(im, i, x, y, z, sx, sy, sz) { _s.set(sx, sy, sz); _p.set(x, y, z); _m.compose(_p, _q, _s); im.setMatrixAt(i, _m); }
function syncCrowd() {
  for (let i = 0; i < people.length; i++) {
    const p = people[i], h = p.h;
    setInst(crowd.legs, i, p.x, 0.43 * h, p.z, 1, h, 1);
    setInst(crowd.body, i, p.x, 1.18 * h, p.z, 1, h, 1);
    setInst(crowd.head, i, p.x, 1.68 * h, p.z, 1, 1, 1);
    if (p.hat) setInst(crowd.hat, i, p.x, 1.82 * h, p.z, 1, 1, 1); else { _m.compose(ZERO, _q, ZERO); crowd.hat.setMatrixAt(i, _m); }
    if (p.balloon) setInst(crowd.balloon, i, p.x + 0.3, 2.9 * h, p.z, 1, 1.15, 1); else { _m.compose(ZERO, _q, ZERO); crowd.balloon.setMatrixAt(i, _m); }
  }
  for (const im of peopleMeshes) { im.instanceMatrix.needsUpdate = true; im.computeBoundingSphere(); }
}
const walkRng = rng(99);
let crowdT = 0;
function tick(dt) {
  if (!found && !dayOver) { roundT += dt; sinceHint += dt; }
  carousel.rotation.y += dt * 0.35;
  horses.forEach((h, i) => { h.position.y = 1.4 + Math.sin(carousel.rotation.y * 4 + i) * 0.25; });
  spin.rotation.z += dt * 0.06;
  for (const gd of gondolas) { const a = gd.userData.a + spin.rotation.z; gd.position.set(Math.cos(a) * 10, Math.sin(a) * 10 - 0.8, 0); gd.rotation.z = -spin.rotation.z; }
  // people wander between free spots, pausing now and then
  for (const p of people) {
    if (p.still) continue;
    if (p.wait > 0) { p.wait -= dt; continue; }
    const dx = p.tx - p.x, dz = p.tz - p.z, dist = Math.hypot(dx, dz);
    if (dist < 0.2) { const s = freeSpot(walkRng); p.tx = s.x; p.tz = s.z; p.wait = walkRng.range(1, 8); continue; }
    const nx = p.x + (dx / dist) * p.speed * dt, nz = p.z + (dz / dist) * p.speed * dt;
    if (blocked(nx, nz, 0.3)) { const s = freeSpot(walkRng); p.tx = s.x; p.tz = s.z; continue; }
    p.x = nx; p.z = nz;
  }
  // dogs trot after their people
  for (const d of dogs) {
    const o = people[d.spec.owner];
    const gx = o.x + d.spec.side * d.spec.lag, gz = o.z + 0.5;
    const dx = gx - d.x, dz = gz - d.z, dist = Math.hypot(dx, dz);
    let moving = false;
    if (dist > 0.25 && !(d.spec.sit && dist < 2)) {
      const sp = Math.min(dist * 1.8, 2.2) * dt;
      const nx = d.x + (dx / dist) * sp, nz = d.z + (dz / dist) * sp;
      if (!blocked(nx, nz, 0.1)) { d.x = nx; d.z = nz; }
      d.heading = Math.atan2(-dz, dx);
      moving = true;
    }
    d.phase += dt * (moving ? 10 : 0);
    d.g.position.set(d.x, 0, d.z);
    d.g.rotation.y = d.heading;
    d.g.userData.legs.forEach((l, i) => { l.rotation.z = moving ? Math.sin(d.phase + (i % 2 ? Math.PI : 0) + (i > 1 ? Math.PI / 2 : 0)) * 0.5 : 0; });
    d.g.userData.tail.rotation.x = Math.sin(d.phase * 0.6 + roundT * 6) * 0.35;
  }
  crowdT += dt;
  if (crowdT > 1 / 30 || VERIFY) { crowdT = 0; syncCrowd(); }
  controls.update();
  if (portrait) portrait.rotation.y += dt * 0.6;
  hudT += dt; if (hudT > 0.25) { hudT = 0; hud(); }
}
let hudT = 0;

// ------------------------------------------------------------------ poster with a turning portrait
const pScene = new THREE.Scene();
pScene.add(new THREE.HemisphereLight(0xffffff, 0x8a8a8a, 1.6));
const pSun = new THREE.DirectionalLight(0xffffff, 1.6); pSun.position.set(2, 3, 2); pScene.add(pSun);
const pCam = new THREE.PerspectiveCamera(30, 1.3, 0.1, 20); pCam.position.set(0.3, 0.95, 2.7); pCam.lookAt(0.05, 0.45, 0);
let portrait = null;
function showPoster() {
  if (portrait) pScene.remove(portrait);
  portrait = buildDog({ breed: 'gsd', neck: plan.want.neck, carry: plan.want.carry });
  pScene.add(portrait);
  const neck = NECKWEAR.find((n) => n.id === plan.want.neck).name, carry = CARRY.find((c) => c.id === plan.want.carry).name;
  $('want').textContent = `German Shepherd, wearing ${neck}, carrying ${carry}.`;
  $('round').textContent = `Round ${round + 1} of ${ROUNDS.length}`;
}
// the poster sits above the main canvas, so the portrait gets its own small renderer
const pRenderer = makeRenderer($('portrait'), { clear: 0xf6efe0 });
function sizePortrait() {
  const el = $('portrait');
  const w = el.clientWidth || 216, h = el.clientHeight || 166;
  pRenderer.setSize(w, h, false); pCam.aspect = w / h; pCam.updateProjectionMatrix();
}

// ------------------------------------------------------------------ clicks
function clickAt(x, y) {
  if (found || dayOver) return { kind: 'ignored' };
  const hits = pickAt(x, y, camera, [...peopleMeshes, ...dogs.map((d) => d.g)]);
  const h = hits.find((hh) => hh.object.isInstancedMesh ? hh.object.count > hh.instanceId : true);
  if (!h) return { kind: 'nothing' };
  if (h.object.isInstancedMesh) { sfx.soft(); toast('Just someone enjoying the fair.'); return { kind: 'person' }; }
  const d = h.object.userData.dog;
  if (!d) return { kind: 'nothing' };
  if (isBenny(d.spec, plan.want)) {
    found = true;
    times.push(roundT);
    sfx.done();
    // a happy bounce
    d.spec.sit = true;
    $('found-text').textContent = `Found him in ${fmt(roundT)}${wrong ? ` after ${wrong} look-alike${wrong > 1 ? 's' : ''}` : ', first try'}.`;
    $('found-next').textContent = round + 1 < ROUNDS.length ? 'Next round, bigger crowd' : 'See the day';
    $('found').hidden = false;
    controls.target.set(d.x, 0.5, d.z);
    return { kind: 'benny' };
  }
  wrong++;
  sfx.soft();
  const msg = whyNot(d.spec, plan.want);
  toast(msg, 2600);
  return { kind: 'wrong', msg, breed: d.spec.breed };
}
let downAt = null;
canvas.addEventListener('pointerdown', (e) => { downAt = { x: e.clientX, y: e.clientY }; });
canvas.addEventListener('pointerup', (e) => {
  if (!downAt || api.isAuto()) return;
  if (Math.hypot(e.clientX - downAt.x, e.clientY - downAt.y) < 6) clickAt(e.clientX, e.clientY);
  downAt = null;
});
$('found-next').addEventListener('click', () => {
  if (round + 1 < ROUNDS.length) startRound(round + 1);
  else endDay();
});
$('again').addEventListener('click', () => { seed = (seed * 7 + 13) % 2147483647; times.length = 0; dayOver = false; $('day').hidden = true; startRound(0); });
$('hint').addEventListener('click', hint);

function nearestLandmark(x, z) {
  const marks = [...STALLS.map((s) => ({ name: s.name, x: s.x, z: s.z })), { name: 'the carousel', x: 0, z: 0 }];
  marks.sort((a, b) => Math.hypot(a.x - x, a.z - z) - Math.hypot(b.x - x, b.z - z));
  return marks[0].name;
}
function hint() {
  if (sinceHint < HINT_AFTER || found) return;
  const b = dogs.find((d) => isBenny(d.spec, plan.want));
  sinceHint = 0; hintShown = true;
  toast(`Someone saw a shepherd near ${nearestLandmark(b.x, b.z)}.`, 4000);
}
function endDay() {
  dayOver = true;
  $('found').hidden = true;
  const total = times.reduce((a, b) => a + b, 0);
  meta.fairs++;
  if (meta.bestTotal === null || total < meta.bestTotal) meta.bestTotal = Math.round(total);
  save(SAVE_KEY, meta);
  $('day-text').textContent = `Three crowds, three Bennys, ${fmt(total)} in total${meta.bestTotal === Math.round(total) && meta.fairs > 1 ? ' (a new best)' : ''}.`;
  $('day').hidden = false;
}

// ------------------------------------------------------------------ hud, render
const fmt = (t) => `${Math.floor(t / 60)}:${String(Math.floor(t % 60)).padStart(2, '0')}`;
function hud() {
  $('time').textContent = fmt(roundT);
  $('hint').disabled = found || sinceHint < HINT_AFTER;
  $('hint').textContent = sinceHint < HINT_AFTER && !found ? `Hint in ${Math.ceil(HINT_AFTER - sinceHint)}` : 'Hint';
}
function render() {
  renderer.setViewport(0, 0, innerWidth, innerHeight);
  renderer.render(scene, camera);
  pRenderer.render(pScene, pCam);
}
function resize() { renderer.setSize(innerWidth, innerHeight, false); camera.aspect = innerWidth / innerHeight; camera.updateProjectionMatrix(); sizePortrait(); }
addEventListener('resize', resize); resize();
pScene.background = new THREE.Color(0xf6efe0);

// ------------------------------------------------------------------ tour
// The bot does what a person does: swing the camera close to a dog until nothing
// is in the way, then click it on screen.
const _c = new THREE.Vector3();
function lookAtDog(d) {
  for (let k = 0; k < 16; k++) {
    const a = (k / 8) * Math.PI * 2 + d.heading, el = k < 8 ? 1.9 : 4.5;
    controls.target.set(d.x, 0.4, d.z);
    camera.position.set(d.x + Math.cos(a) * 3.6, el, d.z - Math.sin(a) * 3.6);
    camera.lookAt(controls.target); camera.updateMatrixWorld();
    d.g.updateMatrixWorld(true);
    const p = toScreen(new THREE.Box3().setFromObject(d.g).getCenter(_c), camera);
    const hit = pickAt(p.x, p.y, camera, [...peopleMeshes, ...dogs.map((dd) => dd.g)]).find((hh) => !hh.object.isInstancedMesh || hh.object.count > hh.instanceId);
    if (hit && hit.object.userData.dog === d) return p;
  }
  return null;
}
const bot = { t: 0, phase: 'survey', wrongRes: null, results: [] };
function botStep(dt) {
  bot.t += dt;
  if (bot.t < 0.6) return;
  bot.t = 0;
  if (bot.phase === 'survey') { bot.phase = 'decoy'; return; }
  if (bot.phase === 'decoy') {
    const d = dogs.find((dd) => dd.spec.breed === 'gsd' && !isBenny(dd.spec, plan.want));
    const p = d && lookAtDog(d);
    bot.wrongRes = p ? clickAt(p.x, p.y) : { kind: 'no view' };
    bot.phase = 'find';
    return;
  }
  if (bot.phase === 'find') {
    const d = dogs.find((dd) => isBenny(dd.spec, plan.want));
    const p = lookAtDog(d);
    bot.results.push(p ? clickAt(p.x, p.y).kind : 'no view');
    bot.phase = 'celebrate';
    return;
  }
  if (bot.phase === 'celebrate') {
    $('found-next').click();
    bot.phase = dayOver ? 'done' : 'find';
  }
}
const oneBenny = () => dogs.filter((d) => isBenny(d.spec, plan.want)).length === 1;
const checks = [
  { name: 'fair-open', when: () => true, check: () => oneBenny() && dogs.length === ROUNDS[0].dogs && crowd.body.count === ROUNDS[0].people && dogs.filter((d) => d.spec.breed === 'gsd').length >= 5 },
  { name: 'look-alike', when: () => bot.wrongRes !== null, check: () => bot.wrongRes.kind === 'wrong' && bot.wrongRes.breed === 'gsd' && !found && wrong === 1 },
  { name: 'found-1', when: () => found && round === 0, check: () => bot.results[0] === 'benny' && !$('found').hidden },
  { name: 'found-2', when: () => found && round === 1, check: () => bot.results[1] === 'benny' && dogs.length === ROUNDS[1].dogs && oneBenny() },
  { name: 'found-3', when: () => found && round === 2, check: () => bot.results[2] === 'benny' && crowd.body.count === ROUNDS[2].people },
  { name: 'day-over', when: () => dayOver, check: () => !$('day').hidden && times.length === 3 },
];

function selfTest() {
  const c = { exactly_one_benny: true, four_plus_shepherd_lookalikes: true, lookalikes_differ_by_one: true, malinois_is_not_benny: true, every_wrong_dog_explained: true, nobody_inside_stalls: true, varied_outfits: true };
  const outfits = new Set();
  for (let s = 1; s <= 60; s++) {
    const pl = planRound(s * 104729, s % 3);
    const bennies = pl.dogs.filter((d) => isBenny(d, pl.want));
    if (bennies.length !== 1) c.exactly_one_benny = false;
    const gsds = pl.dogs.filter((d) => d.breed === 'gsd' && !isBenny(d, pl.want));
    if (gsds.length < 4) c.four_plus_shepherd_lookalikes = false;
    if (gsds.filter((d) => (d.neck === pl.want.neck) !== (d.carry === pl.want.carry)).length < 3) c.lookalikes_differ_by_one = false;
    const mal = pl.dogs.find((d) => d.breed === 'malinois');
    if (!mal || isBenny(mal, pl.want) || mal.neck !== pl.want.neck) c.malinois_is_not_benny = false;
    for (const d of pl.dogs) if (!isBenny(d, pl.want) && !whyNot(d, pl.want)) c.every_wrong_dog_explained = false;
    for (const p of pl.people) if (blocked(p.x, p.z, 0.3)) c.nobody_inside_stalls = false;
    outfits.add(`${pl.want.neck}/${pl.want.carry}`);
  }
  c.varied_outfits = outfits.size >= 12;
  return { pass: Object.values(c).every(Boolean), checks: c, summary: `60 fairs, ${outfits.size} different Benny outfits` };
}

startRound(0);
const help = wireHelp();
const api = harness({
  checks, tick, render, selfTest, bot: botStep,
  paused: () => help.isOpen(),
  extraState: () => ({ round, wrong, found, times }),
  winNote: () => `three rounds, Benny found each time among ${ROUNDS.map((r) => r.dogs).join('/')} dogs`,
  stuckSteps: 20000,
});
if (!VERIFY && meta.fairs === 0 && !new URLSearchParams(location.search).has('autoplay')) help.open();
api.start();

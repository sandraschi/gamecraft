// Hoshi (星): a night sky with real star positions. Each round names a
// constellation and shows its stick figure; find its stars among the others and
// connect them. Western and Japanese names, a line of lore when it is complete.
import * as THREE from 'three';
import { VERIFY, $, rng, makeRenderer, toScreen, harness, sfx, toast, wireHelp, load, save } from '../_shared/gc.js';
import { setupMusic } from '../_shared/music.js';
const music = setupMusic('stars', { sfx });

// RA in hours, Dec in degrees, visual magnitude (approximate, J2000)
const STARS = {
  Dubhe: [11.062, 61.75, 1.8], Merak: [11.031, 56.38, 2.4], Phecda: [11.897, 53.69, 2.4], Megrez: [12.257, 57.03, 3.3], Alioth: [12.900, 55.96, 1.8], Mizar: [13.399, 54.93, 2.2], Alkaid: [13.792, 49.31, 1.9],
  Caph: [0.153, 59.15, 2.3], Schedar: [0.675, 56.54, 2.2], GammaCas: [0.945, 60.72, 2.2], Ruchbah: [1.430, 60.24, 2.7], Segin: [1.907, 63.67, 3.4],
  Betelgeuse: [5.919, 7.41, 0.5], Bellatrix: [5.419, 6.35, 1.6], Alnitak: [5.679, -1.94, 1.8], Alnilam: [5.604, -1.20, 1.7], Mintaka: [5.533, -0.30, 2.2], Saiph: [5.796, -9.67, 2.1], Rigel: [5.242, -8.20, 0.1], Meissa: [5.585, 9.93, 3.4],
  Vega: [18.616, 38.78, 0.0], Altair: [19.846, 8.87, 0.8], Deneb: [20.690, 45.28, 1.3], Tarazed: [19.771, 10.61, 2.7], Alshain: [19.922, 6.41, 3.7],
  Sadr: [20.370, 40.26, 2.2], Albireo: [19.512, 27.96, 3.1], Gienah: [20.770, 33.97, 2.5], DeltaCyg: [19.750, 45.13, 2.9],
  Alcyone: [3.791, 24.11, 2.9], Atlas: [3.820, 24.05, 3.6], Electra: [3.748, 24.11, 3.7], Maia: [3.763, 24.37, 3.9], Merope: [3.772, 23.95, 4.2], Taygeta: [3.753, 24.47, 4.3],
  Antares: [16.490, -26.43, 1.0], Dschubba: [16.006, -22.62, 2.3], Acrab: [16.091, -19.81, 2.6], PiSco: [15.981, -26.11, 2.9], TauSco: [16.598, -28.22, 2.8], EpsSco: [16.836, -34.29, 2.3], MuSco: [16.864, -38.05, 3.0], ZetaSco: [16.900, -42.36, 3.6], EtaSco: [17.203, -43.24, 3.3], Sargas: [17.622, -43.00, 1.9], IotaSco: [17.793, -40.13, 3.0], KappaSco: [17.708, -39.03, 2.4], Shaula: [17.560, -37.10, 1.6],
};
const CONSTELLATIONS = [
  { id: 'dipper', en: 'The Big Dipper', jp: '北斗七星', kana: 'hokuto shichisei', fov: 34,
    lines: [['Dubhe', 'Merak'], ['Merak', 'Phecda'], ['Phecda', 'Megrez'], ['Megrez', 'Dubhe'], ['Megrez', 'Alioth'], ['Alioth', 'Mizar'], ['Mizar', 'Alkaid']],
    lore: 'Seven stars of the north. Follow Merak through Dubhe and you arrive at the Pole Star, the one that does not move.' },
  { id: 'cassiopeia', en: 'Cassiopeia', jp: '山形星', kana: 'yamagata-boshi, the mountain-shape stars', fov: 26,
    lines: [['Caph', 'Schedar'], ['Schedar', 'GammaCas'], ['GammaCas', 'Ruchbah'], ['Ruchbah', 'Segin']],
    lore: 'A queen on her throne in Greece, a mountain outline in Japan. Either way: a W, on the far side of the Pole from the Dipper.' },
  { id: 'orion', en: 'Orion', jp: '鼓星', kana: 'tsuzumi-boshi, the hand-drum stars', fov: 34,
    lines: [['Meissa', 'Betelgeuse'], ['Meissa', 'Bellatrix'], ['Betelgeuse', 'Alnitak'], ['Bellatrix', 'Mintaka'], ['Mintaka', 'Alnilam'], ['Alnilam', 'Alnitak'], ['Alnitak', 'Saiph'], ['Mintaka', 'Rigel']],
    lore: 'The hunter of winter. In Japan the narrow waist made it a tsuzumi drum, and red Betelgeuse and white Rigel were the Heike and Genji banners.' },
  { id: 'tanabata', en: 'Tanabata: Vega and Altair', jp: '織姫と彦星', kana: 'Orihime to Hikoboshi', fov: 66,
    lines: [['Vega', 'Deneb'], ['Deneb', 'Altair'], ['Altair', 'Vega']],
    lore: 'The weaver Orihime (Vega) and the herdsman Hikoboshi (Altair) are kept apart by the River of Heaven. On the seventh night of the seventh month, magpies make a bridge; Deneb is often said to be one of them.' },
  { id: 'cygnus', en: 'Cygnus, the Northern Cross', jp: '白鳥座', kana: 'hakuchō-za', fov: 30,
    lines: [['Deneb', 'Sadr'], ['Sadr', 'Albireo'], ['Gienah', 'Sadr'], ['Sadr', 'DeltaCyg']],
    lore: 'A swan flying down the Milky Way. Its beak, Albireo, is a gold and blue double star in any small telescope.' },
  { id: 'subaru', en: 'The Pleiades', jp: '昴', kana: 'Subaru', fov: 2.6,
    lines: [['Alcyone', 'Atlas'], ['Alcyone', 'Merope'], ['Merope', 'Electra'], ['Electra', 'Maia'], ['Maia', 'Taygeta'], ['Maia', 'Alcyone']],
    lore: 'Subaru means "gathered together". A thousand years ago, Sei Shōnagon began her list of stars with Subaru.' },
  { id: 'scorpius', en: 'Scorpius', jp: '釣り星', kana: 'tsuri-boshi, the fish-hook stars', fov: 40,
    lines: [['Acrab', 'Dschubba'], ['PiSco', 'Dschubba'], ['Dschubba', 'Antares'], ['Antares', 'TauSco'], ['TauSco', 'EpsSco'], ['EpsSco', 'MuSco'], ['MuSco', 'ZetaSco'], ['ZetaSco', 'EtaSco'], ['EtaSco', 'Sargas'], ['Sargas', 'IotaSco'], ['IotaSco', 'KappaSco'], ['KappaSco', 'Shaula']],
    lore: 'In fishing villages of western Japan, the curl of the scorpion was a fish-hook cast into the summer sky, red Antares its bait.' },
];
const PICK_PX = 22;
const SAVE_KEY = 'gamecraft.hoshi.v1';

export function dirOf([ra, dec]) {
  const a = ra * 15 * Math.PI / 180, d = dec * Math.PI / 180;
  return new THREE.Vector3(Math.cos(d) * Math.cos(a), Math.sin(d), -Math.cos(d) * Math.sin(a));
}
/** Background stars: seeded, kept clear of every named star so clicks are never ambiguous. */
export function backgroundStars(seed, n = 2400) {
  const r = rng(seed);
  const named = Object.values(STARS).map(dirOf);
  const out = [];
  while (out.length < n) {
    const u = r() * 2 - 1, th = r() * Math.PI * 2, s = Math.sqrt(1 - u * u);
    const v = new THREE.Vector3(s * Math.cos(th), u, s * Math.sin(th));
    const near = named.some((m) => m.angleTo(v) < 0.022);
    if (near) continue;
    out.push({ dir: v, mag: 3.6 + r() ** 0.5 * 2.6, tint: r() });
  }
  return out;
}
function centroidDir(c) {
  const ids = [...new Set(c.lines.flat())];
  const v = new THREE.Vector3(); for (const id of ids) v.add(dirOf(STARS[id]));
  return v.normalize();
}

// ------------------------------------------------------------------ scene
const canvas = $('view');
const renderer = makeRenderer(canvas, { clear: 0x03050d });
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x03050d);
const camera = new THREE.PerspectiveCamera(40, 1, 0.1, 200);
const R = 100;
const sky = new THREE.Group(); scene.add(sky);

const dotTex = (() => {
  const c = document.createElement('canvas'); c.width = c.height = 64;
  const x = c.getContext('2d'); const g = x.createRadialGradient(32, 32, 0, 32, 32, 32);
  g.addColorStop(0, 'rgba(255,255,255,1)'); g.addColorStop(0.25, 'rgba(255,255,255,0.75)'); g.addColorStop(1, 'rgba(255,255,255,0)');
  x.fillStyle = g; x.fillRect(0, 0, 64, 64); return new THREE.CanvasTexture(c);
})();
const starMat = new THREE.ShaderMaterial({
  uniforms: { map: { value: dotTex }, scale: { value: 1 }, time: { value: 0 } },
  vertexShader: `attribute float size; attribute vec3 color; attribute float phase; varying vec3 vColor; uniform float scale; uniform float time;
    void main() { vColor = color * (0.85 + 0.15 * sin(time * 2.0 + phase)); vec4 mv = modelViewMatrix * vec4(position, 1.0); gl_PointSize = size * scale; gl_Position = projectionMatrix * mv; }`,
  fragmentShader: `uniform sampler2D map; varying vec3 vColor; void main() { vec4 t = texture2D(map, gl_PointCoord); gl_FragColor = vec4(vColor, t.a); if (t.a < 0.02) discard; }`,
  transparent: true, depthWrite: false, blending: THREE.AdditiveBlending,
});
const bg = backgroundStars(77);
const named = Object.entries(STARS).map(([id, s]) => ({ id, dir: dirOf(s), mag: s[2], tint: 0.5 }));
const all = [...named, ...bg];
{
  const pos = new Float32Array(all.length * 3), size = new Float32Array(all.length), col = new Float32Array(all.length * 3), ph = new Float32Array(all.length);
  const c = new THREE.Color();
  all.forEach((s, i) => {
    pos.set([s.dir.x * R, s.dir.y * R, s.dir.z * R], i * 3);
    size[i] = Math.max(2.2, 15 - s.mag * 2.4);
    c.setHSL(s.tint < 0.3 ? 0.08 : s.tint > 0.8 ? 0.6 : 0.12, s.tint < 0.3 || s.tint > 0.8 ? 0.5 : 0.1, 0.85);
    if (s.id === 'Antares' || s.id === 'Betelgeuse') c.setHSL(0.03, 0.8, 0.7);
    if (s.id === 'Rigel' || s.id === 'Vega') c.setHSL(0.6, 0.5, 0.9);
    col.set([c.r, c.g, c.b], i * 3); ph[i] = (i * 2.39) % 6.28;
  });
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.BufferAttribute(pos, 3)); g.setAttribute('size', new THREE.BufferAttribute(size, 1));
  g.setAttribute('color', new THREE.BufferAttribute(col, 3)); g.setAttribute('phase', new THREE.BufferAttribute(ph, 1));
  sky.add(new THREE.Points(g, starMat));
}
// a faint Milky Way band (galactic plane, roughly through Cygnus, Aquila, Scorpius, Orion's side)
{
  const pts = [];
  const pole = dirOf([12.857, 27.13]); // north galactic pole
  const a = new THREE.Vector3(0, 1, 0).cross(pole).normalize(), b = pole.clone().cross(a).normalize();
  const r = rng(5);
  for (let i = 0; i < 9000; i++) {
    const th = r() * Math.PI * 2, off = (r() + r() + r() - 1.5) * 0.12;
    const v = a.clone().multiplyScalar(Math.cos(th)).addScaledVector(b, Math.sin(th)).addScaledVector(pole, off).normalize().multiplyScalar(R * 1.01);
    pts.push(v.x, v.y, v.z);
  }
  const g = new THREE.BufferGeometry(); g.setAttribute('position', new THREE.Float32BufferAttribute(pts, 3));
  sky.add(new THREE.Points(g, new THREE.PointsMaterial({ color: 0x8a9ad8, size: 0.6, transparent: true, opacity: 0.22, depthWrite: false, blending: THREE.AdditiveBlending })));
}
// horizon silhouette: a low hill line with a pagoda, fixed to the view
const hillScene = new THREE.Scene();
const hillCam = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
{
  const shape = new THREE.Shape(); shape.moveTo(-1, -1);
  for (let i = 0; i <= 40; i++) { const x = -1 + i / 20; shape.lineTo(x, -0.82 + Math.sin(i * 0.7) * 0.025 + Math.sin(i * 0.23) * 0.04); }
  shape.lineTo(1, -1);
  hillScene.add(new THREE.Mesh(new THREE.ShapeGeometry(shape), new THREE.MeshBasicMaterial({ color: 0x020308 })));
  const pag = new THREE.Group(); pag.position.set(0.62, -0.8, 0);
  for (let i = 0; i < 5; i++) { const w = 0.07 - i * 0.01; const roof = new THREE.Mesh(new THREE.PlaneGeometry(w * 2, 0.012), new THREE.MeshBasicMaterial({ color: 0x020308 })); roof.position.y = 0.02 + i * 0.035; pag.add(roof); const body = new THREE.Mesh(new THREE.PlaneGeometry(w, 0.03), new THREE.MeshBasicMaterial({ color: 0x020308 })); body.position.y = 0.005 + i * 0.035; pag.add(body); }
  const spire = new THREE.Mesh(new THREE.PlaneGeometry(0.004, 0.06), new THREE.MeshBasicMaterial({ color: 0x020308 })); spire.position.y = 0.2; pag.add(spire);
  hillScene.add(pag);
}

// lines
const lineGroup = new THREE.Group(); sky.add(lineGroup);
const goldLine = new THREE.LineBasicMaterial({ color: 0xffd27a, transparent: true, opacity: 0.9 });
const doneLine = new THREE.LineBasicMaterial({ color: 0x8aa8ff, transparent: true, opacity: 0.35 });
function lineBetween(a, b, mat) {
  const g = new THREE.BufferGeometry().setFromPoints([a.clone().multiplyScalar(R * 0.995), b.clone().multiplyScalar(R * 0.995)]);
  return new THREE.Line(g, mat);
}
const wrongLines = [];
const marker = new THREE.Mesh(new THREE.RingGeometry(1.0, 1.2, 32), new THREE.MeshBasicMaterial({ color: 0xffd27a, transparent: true, opacity: 0.9, side: THREE.DoubleSide, depthWrite: false }));
marker.visible = false; sky.add(marker);

// ------------------------------------------------------------------ view control
const view = { yaw: 0, pitch: 0, fov: 40, aim: null };
function applyView() {
  camera.fov = view.fov; camera.updateProjectionMatrix();
  const d = new THREE.Vector3(Math.cos(view.pitch) * Math.cos(view.yaw), Math.sin(view.pitch), -Math.cos(view.pitch) * Math.sin(view.yaw));
  camera.position.set(0, 0, 0); camera.up.set(0, 1, 0); camera.lookAt(d);
  camera.updateMatrixWorld();
}
function aimAt(dir, fov) {
  view.aim = { yaw: Math.atan2(-dir.z, dir.x), pitch: Math.asin(THREE.MathUtils.clamp(dir.y, -1, 1)), fov };
}

// ------------------------------------------------------------------ rounds
let round = 0, found = new Set(), selected = null, wrong = 0, complete = false, roundT = 0;
const meta = load(SAVE_KEY, { skies: 0 });
const key = (a, b) => [a, b].sort().join('|');
function startRound(i) {
  round = i;
  const c = CONSTELLATIONS[round];
  found = new Set(); selected = null; wrong = 0; complete = false; roundT = 0;
  for (const l of [...lineGroup.children]) { if (l.userData.round === round || l.userData.temp) { lineGroup.remove(l); l.geometry.dispose(); } }
  aimAt(centroidDir(c), c.fov);
  $('name-en').textContent = c.en; $('name-jp').textContent = c.jp; $('name-kana').textContent = c.kana;
  $('lore').textContent = '';
  $('done').hidden = true;
  drawFigure(c);
  hud();
}
function drawFigure(c) {
  const cv = $('figure'), x = cv.getContext('2d');
  const w = cv.width, h = cv.height;
  x.clearRect(0, 0, w, h);
  const center = centroidDir(c);
  const up0 = new THREE.Vector3(0, 1, 0), right = center.clone().cross(up0).normalize(), up = right.clone().cross(center).normalize();
  const ids = [...new Set(c.lines.flat())];
  const p2 = Object.fromEntries(ids.map((id) => { const d = dirOf(STARS[id]); return [id, [d.dot(right), d.dot(up)]]; }));
  const xs = ids.map((i) => p2[i][0]), ys = ids.map((i) => p2[i][1]);
  const minX = Math.min(...xs), maxX = Math.max(...xs), minY = Math.min(...ys), maxY = Math.max(...ys);
  const s = Math.min((w - 30) / (maxX - minX || 1), (h - 30) / (maxY - minY || 1));
  const P = (id) => [w / 2 + (p2[id][0] - (minX + maxX) / 2) * s, h / 2 - (p2[id][1] - (minY + maxY) / 2) * s];
  x.strokeStyle = 'rgba(255,210,122,0.75)'; x.lineWidth = 1.5;
  for (const [a, b] of c.lines) { const [x1, y1] = P(a), [x2, y2] = P(b); x.beginPath(); x.moveTo(x1, y1); x.lineTo(x2, y2); x.stroke(); }
  for (const id of ids) { const [px, py] = P(id); const r = Math.max(2, 5.5 - STARS[id][2]); x.fillStyle = '#fff'; x.beginPath(); x.arc(px, py, r, 0, 7); x.fill(); }
}

function starAt(cx, cy) {
  let best = null, bd = PICK_PX;
  for (const s of all) {
    if (s.dir.dot(camera.getWorldDirection(_fw)) < 0.5) continue;
    const p = toScreen(s.dir.clone().multiplyScalar(R), camera);
    const d = Math.hypot(p.x - cx, p.y - cy) - (s.id ? 4 : 0); // named stars are a little easier to hit
    if (d < bd) { bd = d; best = s; }
  }
  return best;
}
const _fw = new THREE.Vector3();
function clickStar(s) {
  if (complete) return 'complete';
  if (!s) { selected = null; marker.visible = false; return 'empty'; }
  if (!selected || selected === s) { selected = s; showMarker(s); sfx.tone(880, 0.15, { vol: 0.05 }); return 'selected'; }
  const c = CONSTELLATIONS[round];
  const k = s.id && selected.id ? key(selected.id, s.id) : null;
  const isPart = k && c.lines.some(([a, b]) => key(a, b) === k);
  if (isPart && !found.has(k)) {
    found.add(k);
    const l = lineBetween(selected.dir, s.dir, goldLine); l.userData.round = round; lineGroup.add(l);
    sfx.tone(660 * 2 ** (found.size / 12), 0.4, { vol: 0.08 });
    selected = s; showMarker(s);
    if (found.size === c.lines.length) finishRound();
    hud();
    return 'line';
  }
  if (isPart) { selected = s; showMarker(s); return 'again'; }
  wrong++;
  const l = lineBetween(selected.dir, s.dir, new THREE.LineBasicMaterial({ color: 0xff7a6a, transparent: true, opacity: 0.8 }));
  l.userData.temp = true; l.userData.t = 0; lineGroup.add(l); wrongLines.push(l);
  sfx.soft();
  selected = s; showMarker(s);
  hud();
  return 'wrong';
}
function showMarker(s) { marker.visible = true; marker.position.copy(s.dir).multiplyScalar(R * 0.99); marker.lookAt(0, 0, 0); }
function finishRound() {
  complete = true; marker.visible = false; selected = null;
  const c = CONSTELLATIONS[round];
  $('lore').textContent = c.lore;
  setTimeout(() => sfx.done(), 200);
  for (const l of lineGroup.children) if (l.userData.round === round) l.material = goldLine;
}
$('next').addEventListener('click', () => {
  // finished figures stay in the sky, dimmed
  for (const l of lineGroup.children) if (l.userData.round === round) { l.material = doneLine; l.userData.round = -1; }
  if (round + 1 < CONSTELLATIONS.length) startRound(round + 1);
  else { meta.skies++; save(SAVE_KEY, meta); $('end').hidden = false; $('done').hidden = true; }
});
$('again').addEventListener('click', () => { for (const l of [...lineGroup.children]) { lineGroup.remove(l); l.geometry.dispose(); } $('end').hidden = true; startRound(0); });

// pointer: drag looks around, click picks a star, wheel zooms
let down = null;
canvas.addEventListener('pointerdown', (e) => { if (api.isAuto()) return; down = { x: e.clientX, y: e.clientY, sx: e.clientX, sy: e.clientY }; canvas.setPointerCapture(e.pointerId); view.aim = null; });
canvas.addEventListener('pointermove', (e) => {
  if (!down) return;
  const k = (view.fov * Math.PI / 180) / innerHeight;
  view.yaw += (e.clientX - down.x) * k; view.pitch = THREE.MathUtils.clamp(view.pitch + (e.clientY - down.y) * k, -0.5, 1.45);
  down.x = e.clientX; down.y = e.clientY;
});
canvas.addEventListener('pointerup', (e) => {
  if (!down) return;
  if (Math.hypot(e.clientX - down.sx, e.clientY - down.sy) < 5) { applyView(); clickStar(starAt(e.clientX, e.clientY)); }
  down = null;
});
canvas.addEventListener('wheel', (e) => { e.preventDefault(); view.aim = null; view.fov = THREE.MathUtils.clamp(view.fov * (e.deltaY > 0 ? 1.1 : 0.9), 2, 75); }, { passive: false });
$('recenter').addEventListener('click', () => aimAt(centroidDir(CONSTELLATIONS[round]), CONSTELLATIONS[round].fov));

// ------------------------------------------------------------------ loop
function tick(dt) {
  roundT += dt;
  if (view.aim) {
    const k = Math.min(1, dt * 2.5);
    let dy = view.aim.yaw - view.yaw; dy = Math.atan2(Math.sin(dy), Math.cos(dy));
    view.yaw += dy * k; view.pitch += (view.aim.pitch - view.pitch) * k; view.fov += (view.aim.fov - view.fov) * k;
    if (Math.abs(dy) < 1e-4 && Math.abs(view.aim.pitch - view.pitch) < 1e-4 && Math.abs(view.aim.fov - view.fov) < 1e-3) view.aim = null;
  }
  applyView();
  starMat.uniforms.time.value += dt;
  starMat.uniforms.scale.value = THREE.MathUtils.clamp(40 / view.fov, 1, 3.2) ** 0.5 * (VERIFY ? 1 : Math.min(devicePixelRatio, 2));
  marker.scale.setScalar(view.fov / 40);
  for (let i = wrongLines.length - 1; i >= 0; i--) {
    const l = wrongLines[i]; l.userData.t += dt; l.material.opacity = Math.max(0, 0.8 - l.userData.t * 0.8);
    if (l.userData.t > 1) { lineGroup.remove(l); l.geometry.dispose(); wrongLines.splice(i, 1); }
  }
  if (complete && $('done').hidden && roundT > 0) { doneT += dt; if (doneT > 1.6) { $('done').hidden = false; doneT = 0; $('next').textContent = round + 1 < CONSTELLATIONS.length ? 'Next constellation' : 'The whole sky'; } } else if (!complete) doneT = 0;
}
let doneT = 0;
function hud() {
  const c = CONSTELLATIONS[round];
  $('count').textContent = `${found.size} / ${c.lines.length} lines`;
  $('round').textContent = `${round + 1} of ${CONSTELLATIONS.length}`;
}
function render() {
  renderer.autoClear = true;
  renderer.render(scene, camera);
  renderer.autoClear = false;
  if (view.pitch > -0.05 && view.pitch < 0.55) renderer.render(hillScene, hillCam); // only when looking near the horizon
}
function resize() { renderer.setSize(innerWidth, innerHeight, false); camera.aspect = innerWidth / innerHeight; camera.updateProjectionMatrix(); }
addEventListener('resize', resize); resize();

// ------------------------------------------------------------------ tour
// The bot clicks on the screen like a person: once a wrong pair, then every line of
// the figure in order, after the camera has finished turning toward the stars.
const bot = { t: 0, queue: [], phase: 'wait', wrongRes: null, results: [] };
function screenOf(id) { applyView(); return toScreen(dirOf(STARS[id]).multiplyScalar(R), camera); }
function botStep(dt) {
  bot.t += dt;
  if (view.aim || bot.t < 0.35) return;
  bot.t = 0;
  const c = CONSTELLATIONS[round];
  if (bot.phase === 'wait') {
    // a wrong pair first: two real stars of the figure that are not joined
    const ids = [...new Set(c.lines.flat())];
    let pair = null;
    for (const a of ids) for (const b of ids) if (a < b && !c.lines.some(([x, y]) => key(x, y) === key(a, b))) pair = pair ?? [a, b];
    if (pair && round === 0) {
      const p = screenOf(pair[0]); clickStar(starAt(p.x, p.y));
      const q = screenOf(pair[1]); bot.wrongRes = clickStar(starAt(q.x, q.y));
      clickStar(null);
    }
    bot.queue = c.lines.map(([a, b]) => [a, b]);
    bot.phase = 'draw';
    return;
  }
  if (bot.phase === 'draw') {
    const next = bot.queue.shift();
    if (!next) { bot.phase = 'admire'; return; }
    const [a, b] = next;
    clickStar(null);
    const p = screenOf(a), q = screenOf(b);
    const sa = starAt(p.x, p.y); clickStar(sa);
    bot.results.push(clickStar(starAt(q.x, q.y)));
    return;
  }
  if (bot.phase === 'admire' && !$('done').hidden) { $('next').click(); bot.phase = 'wait'; }
}
const allLines = () => bot.results.every((r) => r === 'line');
const checks = [
  { name: 'night-sky', when: () => !view.aim, check: () => all.length > 2000 && CONSTELLATIONS[0].lines.every(([a, b]) => STARS[a] && STARS[b]) },
  { name: 'wrong-pair', when: () => bot.wrongRes !== null, check: () => bot.wrongRes === 'wrong' && found.size === 0 && wrong === 1 },
  { name: 'big-dipper', when: () => complete && round === 0, check: () => found.size === 7 && allLines() },
  { name: 'cassiopeia', when: () => complete && round === 1, check: () => found.size === 4 && allLines() },
  { name: 'orion', when: () => complete && round === 2 && !$('done').hidden, check: () => found.size === 8 && allLines() && $('lore').textContent.length > 20 },
];

function selfTest() {
  const c = { every_line_uses_real_stars: true, figures_are_connected: true, no_duplicate_lines: true, background_never_crowds_a_named_star: true, figure_fits_its_view: true, named_stars_separable_on_screen: true };
  const misfits = [];
  for (const con of CONSTELLATIONS) {
    const keys = con.lines.map(([a, b]) => key(a, b));
    if (new Set(keys).size !== keys.length) c.no_duplicate_lines = false;
    for (const [a, b] of con.lines) if (!STARS[a] || !STARS[b]) c.every_line_uses_real_stars = false;
    const ids = [...new Set(con.lines.flat())];
    const seen = new Set([ids[0]]); let grew = true;
    while (grew) { grew = false; for (const [a, b] of con.lines) { if (seen.has(a) !== seen.has(b)) { seen.add(a); seen.add(b); grew = true; } } }
    if (seen.size !== ids.length) c.figures_are_connected = false;
    // view test at a reference 1280x720 screen
    const cam = new THREE.PerspectiveCamera(con.fov, 16 / 9, 0.1, 200);
    const d = centroidDir(con); cam.lookAt(d); cam.updateMatrixWorld();
    const rect = { left: 0, top: 0, width: 1280, height: 720 };
    const pts = ids.map((id) => toScreen(dirOf(STARS[id]).multiplyScalar(R), cam, rect));
    if (pts.some((p) => p.x < 40 || p.x > 1240 || p.y < 90 || p.y > 680)) { c.figure_fits_its_view = false; misfits.push(`${con.id}:${pts.map((p) => `${p.x | 0},${p.y | 0}`).join(' ')}`); }
    for (let i = 0; i < pts.length; i++) for (let j = i + 1; j < pts.length; j++) if (Math.hypot(pts[i].x - pts[j].x, pts[i].y - pts[j].y) < PICK_PX * 1.2) c.named_stars_separable_on_screen = false;
  }
  for (const s of bg) for (const n of named) if (n.dir.angleTo(s.dir) < 0.02) c.background_never_crowds_a_named_star = false;
  return { pass: Object.values(c).every(Boolean), checks: c, summary: `${CONSTELLATIONS.length} constellations, ${named.length} named stars, ${bg.length} background stars${misfits.length ? ` misfits ${misfits.join(' / ')}` : ''}` };
}

applyView();
startRound(0);
view.yaw = view.aim.yaw - 0.6; view.pitch = 0.15; view.fov = 60;
const help = wireHelp();
const api = harness({
  checks, tick, render, selfTest, bot: botStep,
  paused: () => help.isOpen(),
  extraState: () => ({ round, lines: found.size, wrong }),
  winNote: () => 'Big Dipper, Cassiopeia and Orion drawn star to star by clicking the sky',
});
if (!VERIFY && meta.skies === 0 && !new URLSearchParams(location.search).has('autoplay')) help.open();
api.start();

// Orgelbauer: a little organ is missing its pipes. Listen to the tune, then stand
// the pipes in their holes so the organ plays it left to right. Longer pipe,
// lower note. All tunes are traditional or long out of copyright.
import * as THREE from 'three';
import { VERIFY, $, rng, seedFromUrl, makeRenderer, toScreen, pickAt, harness, sfx, toast, wireHelp, load, save } from '../_shared/gc.js';
import { setupMusic } from '../_shared/music.js';
const music = setupMusic('workshop', { sfx });

const TUNES = [
  { name: 'Frère Jacques', notes: [60, 62, 64, 60, 60, 62, 64, 60] },
  { name: 'Alle meine Entchen', notes: [60, 62, 64, 65, 67, 67] },
  { name: 'Ah vous dirai-je, Maman', notes: [60, 60, 67, 67, 69, 69, 67] },
  { name: 'Ode an die Freude', notes: [64, 64, 65, 67, 67, 65, 64, 62] },
  { name: 'Hänschen klein', notes: [67, 64, 64, 65, 62, 62, 60, 62, 64, 65, 67, 67, 67] },
];
const NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
const noteName = (m) => `${NAMES[m % 12]}${Math.floor(m / 12) - 1}`;
const freq = (m) => 440 * 2 ** ((m - 69) / 12);
/** Physical rule of thumb: pipe length is inversely proportional to frequency. */
export const pipeLength = (m) => 2.6 * freq(60) / freq(m);
const SAVE_KEY = 'gamecraft.orgel.v1';

// ------------------------------------------------------------------ sound: a small flue-pipe voice
function organNote(m, at = 0, dur = 0.45, vol = 0.12) {
  const c = sfx.ctx; if (!c || !sfx.on) return;
  const t0 = c.currentTime + at, f = freq(m);
  const g = c.createGain(); g.gain.setValueAtTime(0, t0); g.gain.linearRampToValueAtTime(vol, t0 + 0.04); g.gain.setValueAtTime(vol, t0 + dur); g.gain.exponentialRampToValueAtTime(0.0001, t0 + dur + 0.25);
  g.connect(c.destination);
  for (const [h, a] of [[1, 1], [2, 0.45], [3, 0.18], [4, 0.08]]) {
    const o = c.createOscillator(); o.type = 'sine'; o.frequency.value = f * h;
    const ga = c.createGain(); ga.gain.value = a; o.connect(ga).connect(g); o.start(t0); o.stop(t0 + dur + 0.3);
  }
  // breathy chiff at the start
  const n = c.createBufferSource(); const buf = c.createBuffer(1, c.sampleRate * 0.06, c.sampleRate); const d = buf.getChannelData(0);
  for (let i = 0; i < d.length; i++) d[i] = (Math.random() * 2 - 1) * (1 - i / d.length);
  n.buffer = buf; const bp = c.createBiquadFilter(); bp.type = 'bandpass'; bp.frequency.value = f * 3; bp.Q.value = 3;
  const gn = c.createGain(); gn.gain.value = vol * 0.5; n.connect(bp).connect(gn).connect(c.destination); n.start(t0);
}

// ------------------------------------------------------------------ scene
const canvas = $('view');
const renderer = makeRenderer(canvas, { shadows: !VERIFY, clear: 0x2a1e18 });
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x2a1e18);
const camera = new THREE.PerspectiveCamera(40, 1, 0.1, 100);
scene.add(new THREE.HemisphereLight(0xfff0dc, 0x3a2a20, 1.0));
const key = new THREE.SpotLight(0xffe8c8, 120, 40, 0.7, 0.6, 1.4); key.position.set(-4, 10, 9); key.target.position.set(0, 2, 0);
if (!VERIFY) { key.castShadow = true; key.shadow.mapSize.set(1024, 1024); }
scene.add(key, key.target);
const fillL = new THREE.PointLight(0xffc890, 20, 30); fillL.position.set(6, 4, 6); scene.add(fillL);
const M = (c, r = 0.6, m = 0) => new THREE.MeshStandardMaterial({ color: c, roughness: r, metalness: m });
const floor = new THREE.Mesh(new THREE.PlaneGeometry(60, 30), M(0x5a3a24, 0.8)); floor.rotation.x = -Math.PI / 2; floor.receiveShadow = true; scene.add(floor);
const wall = new THREE.Mesh(new THREE.PlaneGeometry(60, 20), M(0x6a4a3a, 0.95)); wall.position.set(0, 10, -4); scene.add(wall);

const organ = new THREE.Group(); scene.add(organ);
const benchG = new THREE.Group(); scene.add(benchG);
const pipeMat = M(0xd8d0c0, 0.25, 0.85);
const selMat = M(0xf2d27a, 0.25, 0.85);
const footGeo = new THREE.ConeGeometry(0.16, 0.5, 16);

function makePipe(m) {
  const L = pipeLength(m);
  const g = new THREE.Group();
  const foot = new THREE.Mesh(footGeo, pipeMat); foot.rotation.x = Math.PI; foot.position.y = 0.25; g.add(foot);
  const body = new THREE.Mesh(new THREE.CylinderGeometry(0.16, 0.16, L, 20), pipeMat); body.position.y = 0.5 + L / 2; g.add(body);
  const mouth = new THREE.Mesh(new THREE.BoxGeometry(0.18, 0.12, 0.05), M(0x1a1410)); mouth.position.set(0, 0.62, 0.15); g.add(mouth);
  const lip = new THREE.Mesh(new THREE.BoxGeometry(0.2, 0.03, 0.06), pipeMat); lip.position.set(0, 0.69, 0.16); g.add(lip);
  g.traverse((o) => { if (o.isMesh) { o.castShadow = true; o.receiveShadow = true; } });
  g.userData = { note: m, len: L };
  return g;
}

// ------------------------------------------------------------------ level
let level = 0, seed = seedFromUrl(1770), tune = null, slots = [], bench = [], held = null, plays = 0, solved = false, playing = null, solvedT = 0;
const meta = load(SAVE_KEY, { tunes: 0 });
const SLOT_GAP = 0.62;
function clearLevel() {
  for (const c of [...organ.children]) organ.remove(c);
  for (const c of [...benchG.children]) benchG.remove(c);
}
/** Pure: the bench order for a tune. Never already solved. */
export function shuffleFor(notes, s) {
  const r = rng(s);
  for (let k = 0; k < 50; k++) { const b = r.shuffle(notes); if (!b.every((m, i) => m === notes[i])) return b; }
  return notes.slice().reverse();
}
function startLevel(i) {
  level = i; tune = TUNES[level];
  clearLevel();
  const n = tune.notes.length, width = (n - 1) * SLOT_GAP;
  // the case: a windchest with holes, carved side towers, a cornice
  const chest = new THREE.Mesh(new THREE.BoxGeometry(width + 1.6, 1.0, 1.2), M(0x6a3a1e, 0.55)); chest.position.set(0, 0.5, 0); chest.castShadow = chest.receiveShadow = true; organ.add(chest);
  const top = new THREE.Mesh(new THREE.BoxGeometry(width + 1.8, 0.12, 1.3), M(0x8a5a2e, 0.5)); top.position.set(0, 1.06, 0); organ.add(top);
  for (const s of [-1, 1]) {
    const tower = new THREE.Mesh(new THREE.BoxGeometry(0.5, 4.8, 1.2), M(0x6a3a1e, 0.55)); tower.position.set(s * (width / 2 + 1.1), 2.4, 0); tower.castShadow = true; organ.add(tower);
    const cap = new THREE.Mesh(new THREE.ConeGeometry(0.42, 0.7, 4), M(0xc8a040, 0.3, 0.7)); cap.position.set(s * (width / 2 + 1.1), 5.15, 0); cap.rotation.y = Math.PI / 4; organ.add(cap);
  }
  slots = [];
  for (let k = 0; k < n; k++) {
    const x = -width / 2 + k * SLOT_GAP;
    const hole = new THREE.Mesh(new THREE.CylinderGeometry(0.17, 0.17, 0.02, 20), M(0x0e0a08, 0.9)); hole.position.set(x, 1.13, 0.1); organ.add(hole);
    const tag = new THREE.Mesh(new THREE.CircleGeometry(0.1, 16), M(0xe8d8b0, 0.6)); tag.position.set(x, 0.55, 0.61); organ.add(tag);
    hole.userData.slot = k; tag.userData.slot = k;
    slots.push({ x, pipe: null, hole, tag });
  }
  bench = shuffleFor(tune.notes, seed + level * 101).map((m) => makePipe(m));
  for (const p of bench) benchG.add(p);
  layoutBench();
  for (const p of bench) { p.position.copy(p.userData.home); p.rotation.z = Math.PI / 2; }
  held = null; plays = 0; solved = false; playing = null; solvedT = 0;
  $('tune').textContent = tune.name; $('lvl').textContent = `${level + 1} / ${TUNES.length}`;
  $('done').hidden = true;
  frame(width);
  hud();
}
function layoutBench() {
  // loose pipes lie on the floor in two columns in front of the organ, so they never hide the holes
  const free = bench.filter((p) => !slots.some((s) => s.pipe === p));
  free.forEach((p, i) => {
    const col = i % 2 ? 1 : -1, row = Math.floor(i / 2);
    const len = 0.5 + p.userData.len;
    p.userData.home = new THREE.Vector3(col * 2.3 + len / 2, 0.17, 1.7 + row * 0.42);
  });
}
function frame(width) {
  const span = Math.max(width + 4, 7);
  camera.position.set(0, 5.2 + span * 0.18, 5.0 + span * 0.8); camera.lookAt(0, 1.5, 1.6);
  camera.updateMatrixWorld();
}

// ------------------------------------------------------------------ actions
export const isSolved = (placed, notes) => placed.length === notes.length && placed.every((m, i) => m === notes[i]);
function placedNotes() { return slots.map((s) => (s.pipe ? s.pipe.userData.note : null)); }
function grab(p) {
  if (solved || playing) return false;
  for (const s of slots) if (s.pipe === p) s.pipe = null;
  if (held && held !== p) held.traverse((o) => { if (o.isMesh && o.material === selMat) o.material = pipeMat; });
  held = p;
  p.traverse((o) => { if (o.isMesh && o.material === pipeMat) o.material = selMat; });
  music.duck(1.2);   // the pipe's note matters more than the background
  organNote(p.userData.note, 0, 0.35, 0.1);
  layoutBench(); hud();
  return true;
}
function putIn(k) {
  if (!held || solved || playing) return 'nothing';
  const s = slots[k];
  const was = s.pipe;
  s.pipe = held;
  held.traverse((o) => { if (o.isMesh && o.material === selMat) o.material = pipeMat; });
  held = null;
  if (was) grab(was); // swap: the old pipe comes into your hand
  layoutBench(); hud();
  sfx.tone(300, 0.06, { type: 'triangle', vol: 0.05 });
  return was ? 'swapped' : 'placed';
}
function listen() {
  if (playing) return;
  music.duck(tune.notes.length * 0.42 + 0.6);
  tune.notes.forEach((m, i) => organNote(m, i * 0.42, 0.36, 0.11));
}
function playOrgan() {
  if (playing || solved) return null;
  plays++;
  music.duck(slots.length * 0.42 + 0.8);
  playing = { i: -1, t: 0, notes: placedNotes() };
  return playing;
}
$('listen').addEventListener('click', listen);
$('play').addEventListener('click', () => playOrgan());
$('next').addEventListener('click', () => { if (level + 1 < TUNES.length) startLevel(level + 1); else { seed = (seed * 7 + 9) % 2147483647; startLevel(0); } });
canvas.addEventListener('pointerdown', (e) => {
  if (api.isAuto()) return;
  const hits = pickAt(e.clientX, e.clientY, camera, [...bench, ...slots.map((s) => s.hole), ...slots.map((s) => s.tag)]);
  const h = hits[0]; if (!h) return;
  let o = h.object;
  if (o.userData.slot !== undefined) { if (held) putIn(o.userData.slot); else if (slots[o.userData.slot].pipe) grab(slots[o.userData.slot].pipe); return; }
  while (o && o.userData.note === undefined) o = o.parent;
  if (!o) return;
  const inSlot = slots.findIndex((s) => s.pipe === o);
  if (held && inSlot >= 0) putIn(inSlot); else grab(o);
});
addEventListener('keydown', (e) => { if (e.key === 'l' || e.key === 'L') listen(); if (e.key === 'p' || e.key === 'P' || e.key === 'Enter') playOrgan(); });

// ------------------------------------------------------------------ loop
let lastResult = null;
function tick(dt) {
  // pipes glide to where they belong
  for (const p of bench) {
    const k = slots.findIndex((s) => s.pipe === p);
    const target = k >= 0 ? new THREE.Vector3(slots[k].x, 1.12, 0.1) : p === held ? p.userData.home.clone().add(new THREE.Vector3(0, 0.35, 0)) : p.userData.home;
    p.position.lerp(target, Math.min(1, dt * 9));
    // standing in a hole, lying on the floor otherwise
    const lie = k >= 0 ? 0 : Math.PI / 2;
    p.rotation.z += (lie - p.rotation.z) * Math.min(1, dt * 9);
  }
  if (playing) {
    playing.t += dt;
    const i = Math.floor(playing.t / 0.42);
    if (i !== playing.i && i < slots.length) {
      playing.i = i;
      const s = slots[i], m = playing.notes[i];
      if (m !== null) organNote(m, 0, 0.36, 0.12);
      const right = m === tune.notes[i];
      s.tag.material.color.setHex(m === null ? 0x6a6a6a : right ? 0x8ad86a : 0xe87a6a);
      if (s.pipe) s.pipe.position.y += 0.08;
    }
    if (playing.t > slots.length * 0.42 + 0.3) {
      lastResult = isSolved(playing.notes, tune.notes);
      if (lastResult) { solved = true; solvedT = 0; meta.tunes++; save(SAVE_KEY, meta); setTimeout(() => sfx.done(), 150); }
      else toast(playing.notes.some((m) => m === null) ? 'Some holes are still empty.' : 'Close. The red tags are the wrong notes.', 2400);
      playing = null;
    }
  }
  if (solved) {
    solvedT += dt;
    if (solvedT > 0.8 && $('done').hidden) {
      $('done-text').textContent = `${tune.name}, ${tune.notes.length} pipes, played ${plays} time${plays > 1 ? 's' : ''}.`;
      $('next').textContent = level + 1 < TUNES.length ? 'Next tune' : 'Start again';
      $('done').hidden = false;
    }
  } else if (!playing) for (const s of slots) s.tag.material.color.lerp(new THREE.Color(0xe8d8b0), Math.min(1, dt * 0.8));
  hudT += dt; if (hudT > 0.3) { hudT = 0; hud(); }
}
let hudT = 0;
function hud() {
  $('filled').textContent = `${slots.filter((s) => s.pipe).length} / ${slots.length} pipes`;
  $('holding').textContent = held ? `holding ${noteName(held.userData.note)}` : '';
  $('play').disabled = !!playing || solved;
}
function render() { renderer.render(scene, camera); }
function resize() { renderer.setSize(innerWidth, innerHeight, false); camera.aspect = innerWidth / innerHeight; camera.fov = camera.aspect < 1 ? 66 : 40; camera.updateProjectionMatrix(); }
addEventListener('resize', resize); resize();

// ------------------------------------------------------------------ tour
// The bot cannot hear, so it reads pipe lengths like a careful builder would,
// but deliberately swaps two different notes first and plays that.
const bot = { t: 0, phase: 'fill', wrong: null, swapped: false };
const clickObj = (obj) => {
  camera.updateMatrixWorld(); obj.updateMatrixWorld(true);
  const b = new THREE.Box3().setFromObject(obj); const c = b.getCenter(new THREE.Vector3());
  const s = toScreen(c, camera);
  const hit = pickAt(s.x, s.y, camera, [...bench, ...slots.map((q) => q.hole), ...slots.map((q) => q.tag)])[0];
  return hit ? hit.object : null;
};
function botStep(dt) {
  bot.t += dt;
  if (bot.t < 0.25 || playing) return;
  bot.t = 0;
  const notes = tune.notes;
  if (bot.phase === 'fill') {
    const k = slots.findIndex((s) => !s.pipe);
    if (k < 0) { bot.phase = level === 0 && !bot.swapped ? 'play-wrong' : 'play'; return; }
    // the order the bot fills in: the true tune, except slots 0 and 1 swapped in the first tune
    let want = notes[k];
    if (level === 0 && !bot.swapped && (k === 1 || k === 2)) want = notes[k === 1 ? 2 : 1];
    const p = bench.find((q) => !slots.some((s) => s.pipe === q) && Math.abs(q.userData.len - pipeLength(want)) < 1e-6);
    const got = clickObj(p); grab(p); void got;
    const tgt = clickObj(slots[k].tag);
    putIn(tgt && tgt.userData.slot !== undefined ? tgt.userData.slot : k);
    return;
  }
  if (bot.phase === 'play-wrong') { playOrgan(); bot.phase = 'judge-wrong'; return; }
  if (bot.phase === 'judge-wrong') {
    bot.wrong = lastResult;
    // swap the two pipes back: pick slot 1, put into slot 2 (which hands slot 2's pipe over), put that into slot 1
    grab(slots[1].pipe); putIn(2); putIn(1);
    bot.swapped = true; bot.phase = 'play'; return;
  }
  if (bot.phase === 'play') { playOrgan(); bot.phase = 'wait'; return; }
  if (bot.phase === 'wait' && !$('done').hidden) { $('next').click(); bot.phase = 'fill'; }
}
const checks = [
  { name: 'empty-organ', when: () => true, check: () => slots.length === TUNES[0].notes.length && bench.length === slots.length && !isSolved(bench.map((p) => p.userData.note), TUNES[0].notes) },
  { name: 'wrong-notes', when: () => bot.wrong !== null, check: () => bot.wrong === false && !solved && plays === 1 },
  { name: 'frere-jacques', when: () => solved && level === 0 && !$('done').hidden, check: () => isSolved(placedNotes(), TUNES[0].notes) && plays === 2 },
  { name: 'entchen', when: () => solved && level === 1 && solvedT > 0.5, check: () => isSolved(placedNotes(), TUNES[1].notes) && plays === 1 },
];

function selfTest() {
  const c = { bench_holds_exactly_the_tune: true, longer_pipe_lower_note: true, pipes_fit_the_case: true, bench_never_presolved: true, blind_order_is_unlikely: true, checker_rejects_a_swap: true };
  let worst = 0;
  for (const [ti, t] of TUNES.entries()) {
    for (let s = 1; s <= 20; s++) {
      const b = shuffleFor(t.notes, s * 13 + ti);
      if ([...b].sort().join() !== [...t.notes].sort().join()) c.bench_holds_exactly_the_tune = false;
      if (isSolved(b, t.notes)) c.bench_never_presolved = false;
    }
    // chance of the right order by luck = 1 / (distinct orderings)
    const counts = {}; for (const m of t.notes) counts[m] = (counts[m] ?? 0) + 1;
    const fact = (n) => (n <= 1 ? 1 : n * fact(n - 1));
    const orderings = fact(t.notes.length) / Object.values(counts).reduce((a, k) => a * fact(k), 1);
    worst = Math.max(worst, 1 / orderings);
    const i = t.notes.findIndex((m, k) => k > 0 && m !== t.notes[k - 1]);
    const sw = t.notes.slice(); [sw[i - 1], sw[i]] = [sw[i], sw[i - 1]];
    if (isSolved(sw, t.notes)) c.checker_rejects_a_swap = false;
  }
  for (let m = 55; m < 80; m++) if (pipeLength(m + 1) >= pipeLength(m)) c.longer_pipe_lower_note = false;
  const all = TUNES.flatMap((t) => t.notes);
  if (Math.max(...all.map(pipeLength)) > 3.0 || Math.min(...all.map(pipeLength)) < 1.4) c.pipes_fit_the_case = false;
  c.blind_order_is_unlikely = worst < 0.01;
  return { pass: Object.values(c).every(Boolean), checks: c, summary: `${TUNES.length} tunes; best odds of a lucky order ${(worst * 100).toFixed(2)}%` };
}

startLevel(0);
const help = wireHelp();
const api = harness({
  checks, tick, render, selfTest, bot: botStep,
  paused: () => help.isOpen(),
  extraState: () => ({ level, plays, solved, placed: placedNotes().map((m) => (m === null ? '-' : noteName(m))).join(' ') }),
  winNote: () => 'Frère Jacques fixed after one wrong try, Alle meine Entchen first time',
});
if (!VERIFY && meta.tunes === 0 && !new URLSearchParams(location.search).has('autoplay')) help.open();
api.start();

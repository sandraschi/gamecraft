// Spot the Difference: the same cluttered room rendered twice, five changes in
// the right-hand copy. Every round picks new differences from the room's items,
// so there is no fixed answer key.
import * as THREE from 'three';
import { VERIFY, $, rng, seedFromUrl, makeRenderer, toScreen, harness, sfx, toast, wireHelp, load, save } from '../_shared/gc.js';
import { setupMusic } from '../_shared/music.js';
const music = setupMusic('cozy', { sfx });
import { buildRoom, roomLights, roomCamera, applyMut, signature } from '../_shared/room.js';

const N_DIFF = 5;
const HIT_PAD = 10;          // px of slack around a difference's screen box
const MIN_AREA = 450;        // px^2 at the reference layout: nothing too tiny
const REF = { left: 0, top: 0, width: 620, height: 600 };
const SAVE_KEY = 'gamecraft.spot.v1';

const canvas = $('view');
const renderer = makeRenderer(canvas, { shadows: !VERIFY, clear: 0x1a1612 });
const scenes = [new THREE.Scene(), new THREE.Scene()];
for (const s of scenes) { s.background = new THREE.Color(0x2a221a); roomLights(s, { shadows: !VERIFY }); }
const camera = roomCamera(1);

let rooms = [null, null];
let diffs = [];
let found = 0, misses = 0, hints = 0, roundT = 0, sinceFind = 0, round = 0;
let seed = seedFromUrl(4242);
let roundOver = false;
const meta = load(SAVE_KEY, { rounds: 0, best: null });
const marks = []; // transient miss markers {el, t}

// ------------------------------------------------------------------ layout
const rects = [{ ...REF }, { ...REF }];
function layout() {
  const w = innerWidth, h = innerHeight, gap = 8, top = 64, bottom = 56;
  if (w >= h) {
    const hw = (w - gap * 3) / 2, hh = h - top - bottom;
    rects[0] = { left: gap, top, width: hw, height: hh };
    rects[1] = { left: gap * 2 + hw, top, width: hw, height: hh };
  } else {
    const hh = (h - top - bottom - gap) / 2;
    rects[0] = { left: gap, top, width: w - gap * 2, height: hh };
    rects[1] = { left: gap, top: top + hh + gap, width: w - gap * 2, height: hh };
  }
  for (const [i, r] of rects.entries()) {
    const f = $(i ? 'frame-b' : 'frame-a');
    Object.assign(f.style, { left: `${r.left}px`, top: `${r.top}px`, width: `${r.width}px`, height: `${r.height}px` });
  }
}
renderer.setSize(innerWidth, innerHeight, false);
addEventListener('resize', () => { renderer.setSize(innerWidth, innerHeight, false); layout(); });

// matrixWorld matters too: before the first render it is still identity
function useRect(r) { camera.aspect = r.width / r.height; camera.updateProjectionMatrix(); camera.updateMatrixWorld(); }

const _bb = new THREE.Box3();
const _v = new THREE.Vector3();
/** Screen box of an item (always measured on the unchanged left copy). */
function screenBox(g, rect) {
  useRect(rect);
  _bb.setFromObject(g);
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  for (let i = 0; i < 8; i++) {
    _v.set(i & 1 ? _bb.max.x : _bb.min.x, i & 2 ? _bb.max.y : _bb.min.y, i & 4 ? _bb.max.z : _bb.min.z);
    const p = toScreen(_v, camera, rect);
    x0 = Math.min(x0, p.x); y0 = Math.min(y0, p.y); x1 = Math.max(x1, p.x); y1 = Math.max(y1, p.y);
  }
  return { x0, y0, x1, y1, area: (x1 - x0) * (y1 - y0), cx: (x0 + x1) / 2, cy: (y0 + y1) / 2 };
}
const overlap = (a, b) => Math.max(0, Math.min(a.x1, b.x1) - Math.max(a.x0, b.x0)) * Math.max(0, Math.min(a.y1, b.y1) - Math.max(a.y0, b.y0));

/** Pure choice of differences for a seed, against the reference layout.
 *  A shuffle can paint itself into a corner (overlaps), so retry deterministically. */
function chooseDiffs(room, s) {
  let best = [];
  for (let k = 0; k < 40; k++) {
    const c = tryChoose(room, s + k * 7919);
    if (c.length === N_DIFF) return c;
    if (c.length > best.length) best = c;
  }
  return best;
}
function tryChoose(room, s) {
  const r = rng(s);
  const cands = r.shuffle([...room.items.values()].filter((g) => g.userData.muts.length));
  const chosen = [];
  let booksUsed = 0;
  for (const g of cands) {
    if (chosen.length === N_DIFF) break;
    const isBook = g.userData.itemId.startsWith('book-');
    if (isBook && booksUsed >= 1) continue;
    const box = screenBox(g, REF);
    if (box.area < MIN_AREA || box.x0 < 4 || box.y0 < 4 || box.x1 > REF.width - 4 || box.y1 > REF.height - 4) continue;
    if (chosen.some((c) => overlap(c.box, box) > 0)) continue;
    const mut = g.userData.muts[r.int(g.userData.muts.length)];
    chosen.push({ id: g.userData.itemId, label: g.userData.label, mut, box, found: false, mutSeed: r.int(1e9) });
    if (isBook) booksUsed++;
  }
  return chosen;
}

function disposeRoom(room, scene) {
  if (!room) return;
  scene.remove(room.root);
  room.root.traverse((o) => { if (o.isMesh) { o.geometry.dispose(); o.material.dispose(); } });
}

function newRound() {
  for (let i = 0; i < 2; i++) { disposeRoom(rooms[i], scenes[i]); rooms[i] = buildRoom(); scenes[i].add(rooms[i].root); }
  for (const s of scenes) s.updateMatrixWorld(true);
  diffs = chooseDiffs(rooms[0], seed);
  for (const d of diffs) applyMut(rooms[1].items.get(d.id), d.mut, rng(d.mutSeed));
  found = 0; misses = 0; hints = 0; roundT = 0; sinceFind = 0; roundOver = false; round++;
  $('rings').innerHTML = '';
  $('done').hidden = true;
  hud();
}

// ------------------------------------------------------------------ clicks
const inRect = (b, r) => b.x0 > r.left && b.y0 > r.top && b.x1 < r.left + r.width && b.y1 < r.top + r.height;
function rectAt(x, y) { return rects.findIndex((r) => x >= r.left && x <= r.left + r.width && y >= r.top && y <= r.top + r.height); }
function clickAt(x, y) {
  if (roundOver) return 'over';
  const side = rectAt(x, y);
  if (side < 0) return 'outside';
  let best = null;
  for (const d of diffs) {
    if (d.found) continue;
    // a moved or grown thing counts where it is now (right copy) as well as where it was
    const boxes = [screenBox(rooms[0].items.get(d.id), rects[side])];
    if (side === 1 && (d.mut.kind === 'move' || d.mut.kind === 'scale')) boxes.push(screenBox(rooms[1].items.get(d.id), rects[1]));
    for (const b of boxes) {
      if (x >= b.x0 - HIT_PAD && x <= b.x1 + HIT_PAD && y >= b.y0 - HIT_PAD && y <= b.y1 + HIT_PAD && (!best || b.area < best.b.area)) best = { d, b };
    }
  }
  if (best) {
    best.d.found = true; found++; sinceFind = 0;
    sfx.good();
    toast(`Found: the ${best.d.label}`);
    for (let i = 0; i < 2; i++) { const el = document.createElement('div'); el.className = 'ring'; el.dataset.id = best.d.id; $('rings').appendChild(el); }
    if (found === N_DIFF) finishRound();
    hud();
    return 'found';
  }
  misses++;
  sfx.soft();
  const el = document.createElement('div'); el.className = 'miss'; el.style.left = `${x}px`; el.style.top = `${y}px`;
  document.body.appendChild(el); marks.push({ el, t: 0 });
  hud();
  return 'miss';
}
canvas.addEventListener('pointerdown', (e) => { if (!api.isAuto()) clickAt(e.clientX, e.clientY); });

function hint() {
  const d = diffs.find((x) => !x.found);
  if (!d || sinceFind < 20) return;
  hints++; sinceFind = 0;
  const b = screenBox(rooms[0].items.get(d.id), rects[1]);
  const el = document.createElement('div'); el.className = 'hint';
  const size = Math.max(b.x1 - b.x0, b.y1 - b.y0) + 120;
  Object.assign(el.style, { left: `${b.cx + 30 - size / 2}px`, top: `${b.cy - 20 - size / 2}px`, width: `${size}px`, height: `${size}px` });
  document.body.appendChild(el); marks.push({ el, t: 0, life: 3 });
  hud();
}
$('hint').addEventListener('click', hint);
const reseed = () => { seed = (seed * 7 + 13) % 2147483647; newRound(); };
$('new').addEventListener('click', reseed);
$('next').addEventListener('click', reseed);

function finishRound() {
  roundOver = true;
  sfx.done();
  meta.rounds++;
  if (meta.best === null || roundT < meta.best) meta.best = Math.round(roundT);
  save(SAVE_KEY, meta);
  $('done-text').textContent = `All five in ${fmt(roundT)}, ${misses} ${misses === 1 ? 'miss' : 'misses'}, ${hints} ${hints === 1 ? 'hint' : 'hints'}.`;
  $('done-list').textContent = `Changed: ${diffs.map((d) => d.label).join(', ')}.`;
  $('done').hidden = false;
}

// ------------------------------------------------------------------ hud, render
const fmt = (t) => `${Math.floor(t / 60)}:${String(Math.floor(t % 60)).padStart(2, '0')}`;
function hud() {
  $('found').textContent = `${found} / ${N_DIFF}`;
  $('time').textContent = fmt(roundT);
  $('hint').disabled = roundOver || sinceFind < 20;
}
function placeRings() {
  const els = $('rings').children;
  for (let i = 0; i < els.length; i++) {
    const b = screenBox(rooms[0].items.get(els[i].dataset.id), rects[i % 2]);
    const size = Math.max(b.x1 - b.x0, b.y1 - b.y0) + 18;
    Object.assign(els[i].style, { left: `${b.cx - size / 2}px`, top: `${b.cy - size / 2}px`, width: `${size}px`, height: `${size}px` });
  }
}
function render() {
  const H = innerHeight;
  renderer.setScissorTest(true);
  for (let i = 0; i < 2; i++) {
    const r = rects[i];
    useRect(r);
    renderer.setViewport(r.left, H - r.top - r.height, r.width, r.height);
    renderer.setScissor(r.left, H - r.top - r.height, r.width, r.height);
    renderer.render(scenes[i], camera);
  }
  renderer.setScissorTest(false);
  placeRings();
}
let hudT = 0;
function tick(dt) {
  if (!roundOver) { roundT += dt; sinceFind += dt; }
  for (let i = marks.length - 1; i >= 0; i--) {
    const m = marks[i]; m.t += dt;
    m.el.style.opacity = String(Math.max(0, 1 - m.t / (m.life ?? 0.9)));
    if (m.t > (m.life ?? 0.9)) { m.el.remove(); marks.splice(i, 1); }
  }
  hudT += dt; if (hudT > 0.25) { hudT = 0; hud(); }
}

// ------------------------------------------------------------------ tour
const bot = { t: 0, phase: 'look', i: 0, prevIds: '' };
function botStep(dt) {
  bot.t += dt;
  if (bot.phase === 'look' && bot.t > 0.6) {
    // a deliberate wrong click on something that did not change
    const taken = diffs.flatMap((d) => [screenBox(rooms[0].items.get(d.id), rects[1]), screenBox(rooms[1].items.get(d.id), rects[1])]);
    const decoy = [...rooms[0].items.values()].map((g) => ({ g, b: screenBox(g, rects[1]) }))
      .find(({ g, b }) => !diffs.some((d) => d.id === g.userData.itemId) && b.area > MIN_AREA && inRect(b, rects[1]) && taken.every((t) => overlap({ x0: t.x0 - HIT_PAD, y0: t.y0 - HIT_PAD, x1: t.x1 + HIT_PAD, y1: t.y1 + HIT_PAD }, b) === 0));
    bot.missResult = decoy ? clickAt(decoy.b.cx, decoy.b.cy) : 'no decoy';
    bot.phase = 'find'; bot.t = 0;
  } else if (bot.phase === 'find' && bot.t > 0.8) {
    const d = diffs.find((x) => !x.found);
    if (d) { const b = screenBox(rooms[0].items.get(d.id), rects[bot.i++ % 2]); clickAt(b.cx, b.cy); }
    bot.t = 0;
    if (roundOver) { bot.phase = 'admire'; }
  } else if (bot.phase === 'admire' && bot.t > 1.2) {
    bot.prevIds = diffs.map((d) => d.id).sort().join(',');
    $('next').click();
    bot.phase = 'second'; bot.t = 0;
  }
}

function roomsAgree() {
  let changed = 0;
  for (const [id, g] of rooms[0].items) {
    const same = signature(g) === signature(rooms[1].items.get(id));
    const isDiff = diffs.some((d) => d.id === id);
    if (isDiff === same) return false;
    if (!same) changed++;
  }
  return changed === N_DIFF;
}

const checks = [
  { name: 'room-built', when: () => true, check: () => rooms[0].items.size === rooms[1].items.size && diffs.length === N_DIFF && roomsAgree() },
  { name: 'wrong-click', when: () => bot.phase !== 'look', check: () => bot.missResult === 'miss' && misses === 1 && found === 0 },
  { name: 'first-found', when: () => found >= 1, check: () => $('rings').children.length === 2 && diffs.filter((d) => d.found).length === 1 },
  { name: 'three-found', when: () => found >= 3, check: () => $('rings').children.length === 6 },
  { name: 'all-found', when: () => roundOver, check: () => found === N_DIFF && !$('done').hidden && misses === 1 },
  { name: 'next-room', when: () => bot.phase === 'second', check: () => found === 0 && diffs.length === N_DIFF && roomsAgree() && diffs.map((d) => d.id).sort().join(',') !== bot.prevIds },
];

function selfTest() {
  const results = { seeds: 0, five_every_time: true, distinct_items: true, nothing_tiny: true, no_overlaps: true, all_visible_changes: true, unchanged_room_matches: true, varied_rounds: true };
  const a = buildRoom(), sc = new THREE.Scene(); sc.add(a.root); sc.updateMatrixWorld(true);
  const sets = new Set();
  for (let s = 1; s <= 60; s++) {
    const ds = chooseDiffs(a, s * 977);
    results.seeds++;
    if (ds.length !== N_DIFF) results.five_every_time = false;
    if (new Set(ds.map((d) => d.id)).size !== ds.length) results.distinct_items = false;
    if (ds.some((d) => d.box.area < MIN_AREA)) results.nothing_tiny = false;
    for (let i = 0; i < ds.length; i++) for (let j = i + 1; j < ds.length; j++) if (overlap(ds[i].box, ds[j].box) > 0) results.no_overlaps = false;
    sets.add(ds.map((d) => d.id).sort().join(','));
    if (s <= 6) {
      const b = buildRoom(); const sb = new THREE.Scene(); sb.add(b.root); sb.updateMatrixWorld(true);
      if ([...a.items].some(([id, g]) => signature(g) !== signature(b.items.get(id)))) results.unchanged_room_matches = false;
      for (const d of ds) applyMut(b.items.get(d.id), d.mut, rng(d.mutSeed));
      for (const d of ds) if (signature(a.items.get(d.id)) === signature(b.items.get(d.id))) results.all_visible_changes = false;
      b.root.traverse((o) => { if (o.isMesh) { o.geometry.dispose(); o.material.dispose(); } });
    }
  }
  results.varied_rounds = sets.size >= 50;
  a.root.traverse((o) => { if (o.isMesh) { o.geometry.dispose(); o.material.dispose(); } });
  const checks2 = Object.fromEntries(Object.entries(results).filter(([k]) => k !== 'seeds'));
  const pass = Object.values(checks2).every(Boolean);
  return { pass, checks: checks2, summary: `${results.seeds} seeds, ${sets.size} distinct difference sets` };
}

layout();
newRound();
const help = wireHelp();
const api = harness({
  checks, tick, render, selfTest,
  bot: botStep,
  paused: () => help.isOpen(),
  extraState: () => ({ found, misses, round, diffs: diffs.map((d) => `${d.id}:${d.mut.kind}`) }),
  winNote: () => `round 1 solved (5 of 5, 1 deliberate miss), round ${round} dealt with a new difference set`,
});
if (!VERIFY && meta.rounds === 0 && !new URLSearchParams(location.search).has('autoplay')) help.open();
api.start();

// Hnefatafl (Copenhagen rules, 11x11) against a fixed-depth alpha-beta opponent.
// rules.js is the rules engine, ai.js the search, rulecheck.js the rule battery
// the gate runs before it plays one full AI-vs-AI game and replays it.
import * as THREE from 'three';
import { VERIFY, $, seedFromUrl, makeRenderer, autoResize, harness, toast, wireHelp, load, save, pickAt, sfx, params } from '../_shared/gc.js';
import {
  Game, ATT, DEF, KING, SIZE, N, THRONE, sq, rowOf, colOf, isCorner, countPieces, moveText, replay,
} from './rules.js';
import { bestMove } from './ai.js';
import { ruleChecks } from './rulecheck.js';

const SAVE_KEY = 'gamecraft.hnefatafl.v2';
const LEVELS = {
  easy: { depth: 1, random: 0.25, label: 'Easy' },
  normal: { depth: 2, random: 0, label: 'Normal' },
  hard: { depth: 3, random: 0, label: 'Hard' },
};
// The gate game: attackers search 2 plies, defenders 1. At equal depth the defenders'
// king race beats this AI's attackers almost every time, so equal depths end in a fast
// escape; this pairing is a fixed, deterministic, decisive game and is stated as such.
const TOUR_DEPTH = { [ATT]: 2, [DEF]: 1 };
const TOUR_SEED = 7;

const prefs = load(SAVE_KEY, { human: DEF, level: 'normal' });
{
  const side = params.get('side');
  if (side === 'attackers') prefs.human = ATT;
  if (side === 'defenders') prefs.human = DEF;
  const lv = params.get('level');
  if (LEVELS[lv]) prefs.level = lv;
}

// ------------------------------------------------------------------ scene
const canvas = $('view');
const renderer = makeRenderer(canvas, { shadows: !VERIFY, clear: 0x141210 });
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x141210);
const camera = new THREE.PerspectiveCamera(40, 1, 0.1, 100);
scene.add(new THREE.HemisphereLight(0xfff2dd, 0x33281a, 1.25));
const sun = new THREE.DirectionalLight(0xffe8c0, 1.9);
sun.position.set(-6, 14, 8);
sun.castShadow = !VERIFY;
sun.shadow.mapSize.set(1024, 1024);
Object.assign(sun.shadow.camera, { left: -9, right: 9, top: 9, bottom: -9, near: 1, far: 40 });
scene.add(sun);

const view = { yaw: 0, pitch: 0.95, dist: 15.5 };
function placeCamera() {
  const r = view.dist;
  camera.position.set(Math.sin(view.yaw) * Math.cos(view.pitch) * r, Math.sin(view.pitch) * r, Math.cos(view.yaw) * Math.cos(view.pitch) * r);
  camera.lookAt(0, 0, 0);
}
placeCamera();
autoResize(renderer, camera);

const xOf = (s) => colOf(s) - 5;
const zOf = (s) => rowOf(s) - 5;
const board3d = new THREE.Group();
scene.add(board3d);
const base = new THREE.Mesh(new THREE.BoxGeometry(11.7, 0.34, 11.7), new THREE.MeshStandardMaterial({ color: 0x2a1c10, roughness: 0.9 }));
base.position.y = -0.2;
base.receiveShadow = true;
board3d.add(base);

const tileGeo = new THREE.BoxGeometry(0.96, 0.12, 0.96);
const tiles = [];
for (let s = 0; s < N; s++) {
  const corner = isCorner(s);
  const color = corner ? 0xc8a028 : s === THRONE ? 0x6a4a1a : (rowOf(s) + colOf(s)) % 2 ? 0x3d3326 : 0x4a3f2d;
  const m = new THREE.Mesh(tileGeo, new THREE.MeshStandardMaterial({ color, roughness: 0.85, emissive: 0x000000 }));
  m.position.set(xOf(s), 0, zOf(s));
  m.receiveShadow = true;
  m.userData.sq = s;
  board3d.add(m);
  tiles.push(m);
  if (corner || s === THRONE) {
    const mark = new THREE.Mesh(new THREE.CylinderGeometry(0.28, 0.34, 0.05, 4), new THREE.MeshStandardMaterial({ color: corner ? 0xf0cf5a : 0x9a7428, roughness: 0.5, metalness: 0.2 }));
    mark.rotation.y = Math.PI / 4;
    mark.position.set(xOf(s), 0.09, zOf(s));
    board3d.add(mark);
  }
}

const MAT = {
  att: new THREE.MeshStandardMaterial({ color: 0x8a2a20, roughness: 0.55 }),
  def: new THREE.MeshStandardMaterial({ color: 0xe9ddc0, roughness: 0.5 }),
  king: new THREE.MeshStandardMaterial({ color: 0xf2e6c4, roughness: 0.4 }),
  gold: new THREE.MeshStandardMaterial({ color: 0xd8b03a, roughness: 0.3, metalness: 0.5 }),
  dot: new THREE.MeshBasicMaterial({ color: 0x6ad88a, transparent: true, opacity: 0.85 }),
  hint: new THREE.MeshBasicMaterial({ color: 0x6ab8f2, transparent: true, opacity: 0.9 }),
  ring: new THREE.MeshBasicMaterial({ color: 0xffe27a, side: THREE.DoubleSide }),
};
const GEO = {
  att: new THREE.CylinderGeometry(0.3, 0.37, 0.5, 24),
  def: new THREE.CylinderGeometry(0.27, 0.34, 0.56, 24),
  king: new THREE.CylinderGeometry(0.3, 0.38, 0.86, 24),
  crownRim: new THREE.TorusGeometry(0.2, 0.05, 8, 20),
  crownTip: new THREE.ConeGeometry(0.06, 0.16, 8),
  dot: new THREE.CylinderGeometry(0.15, 0.15, 0.03, 20),
  ring: new THREE.RingGeometry(0.4, 0.5, 32),
};

function makePiece(kind) {
  const g = new THREE.Group();
  if (kind === ATT) {
    const body = new THREE.Mesh(GEO.att, MAT.att);
    body.position.y = 0.31;
    g.add(body);
  } else if (kind === DEF) {
    const body = new THREE.Mesh(GEO.def, MAT.def);
    body.position.y = 0.34;
    g.add(body);
  } else {
    const body = new THREE.Mesh(GEO.king, MAT.king);
    body.position.y = 0.49;
    g.add(body);
    const rim = new THREE.Mesh(GEO.crownRim, MAT.gold);
    rim.rotation.x = Math.PI / 2;
    rim.position.y = 0.96;
    g.add(rim);
    for (let i = 0; i < 5; i++) {
      const tip = new THREE.Mesh(GEO.crownTip, MAT.gold);
      const a = (i / 5) * Math.PI * 2;
      tip.position.set(Math.cos(a) * 0.2, 1.06, Math.sin(a) * 0.2);
      g.add(tip);
    }
  }
  g.traverse((o) => { if (o.isMesh) { o.castShadow = !VERIFY; } });
  return g;
}

// ------------------------------------------------------------------ game state
const game = new Game();
let human = prefs.human; // ATT, DEF, or 0 while the gate plays both sides
let tour = false;
let selected = -1;
let targets = [];
let hint = null;
let last = null;
let thinking = false;
const pieces = new Map(); // square -> group
const anims = [];
const overlayDots = new THREE.Group();
scene.add(overlayDots);
const selRing = new THREE.Mesh(GEO.ring, MAT.ring);
selRing.rotation.x = -Math.PI / 2;
selRing.visible = false;
scene.add(selRing);

function clearPieces() {
  for (const g of pieces.values()) scene.remove(g);
  pieces.clear();
}
function rebuildPieces() {
  clearPieces();
  for (let s = 0; s < N; s++) {
    const p = game.board[s];
    if (!p) continue;
    const g = makePiece(p);
    g.position.set(xOf(s), 0.06, zOf(s));
    g.userData.sq = s;
    scene.add(g);
    pieces.set(s, g);
  }
}
function slidePiece(from, to) {
  const g = pieces.get(to);
  if (!g || VERIFY) return;
  g.position.set(xOf(from), 0.06, zOf(from));
  anims.push({ g, x0: xOf(from), z0: zOf(from), x1: xOf(to), z1: zOf(to), t0: performance.now(), dur: 220 });
}
function fadeCaptured(squares, before) {
  if (VERIFY) return;
  for (const s of squares) {
    const kind = before[s];
    const ghost = makePiece(kind);
    ghost.position.set(xOf(s), 0.06, zOf(s));
    scene.add(ghost);
    anims.push({ g: ghost, ghost: true, t0: performance.now() + 120, dur: 260 });
  }
}
function stepAnims(now) {
  for (let i = anims.length - 1; i >= 0; i--) {
    const a = anims[i];
    const k = Math.max(0, Math.min(1, (now - a.t0) / a.dur));
    if (a.ghost) {
      const sc = 1 - k;
      a.g.scale.set(sc, sc, sc);
      if (k >= 1) { scene.remove(a.g); anims.splice(i, 1); }
    } else {
      const e = k * k * (3 - 2 * k);
      a.g.position.x = a.x0 + (a.x1 - a.x0) * e;
      a.g.position.z = a.z0 + (a.z1 - a.z0) * e;
      a.g.position.y = 0.06 + Math.sin(k * Math.PI) * 0.25;
      if (k >= 1) { a.g.position.set(a.x1, 0.06, a.z1); anims.splice(i, 1); }
    }
  }
}

function refreshOverlay() {
  while (overlayDots.children.length) overlayDots.remove(overlayDots.children[0]);
  const addDot = (s, mat) => {
    const d = new THREE.Mesh(GEO.dot, mat);
    d.position.set(xOf(s), 0.11, zOf(s));
    overlayDots.add(d);
  };
  for (const t of targets) addDot(t, MAT.dot);
  if (hint) { addDot(hint.from, MAT.hint); addDot(hint.to, MAT.hint); }
  if (selected >= 0) {
    selRing.visible = true;
    selRing.position.set(xOf(selected), 0.11, zOf(selected));
  } else selRing.visible = false;
  for (let s = 0; s < N; s++) {
    const isLast = last && (s === last.from || s === last.to);
    tiles[s].material.emissive.setHex(isLast ? 0x3a2a08 : 0x000000);
  }
}

// ------------------------------------------------------------------ HUD
const sideName = (side) => (side === ATT ? 'Attackers' : 'Defenders');
const REASONS = {
  escape: 'The king reached a corner.',
  'king-captured': 'The king was captured.',
  'no-moves': 'The other side has no legal move.',
  repetition: 'The same position came up three times.',
};
function updateHud() {
  const sideBtn = $('side-btn');
  sideBtn.textContent = tour ? 'Both sides: AI' : `You: ${sideName(human)}`;
  $('level-btn').textContent = LEVELS[prefs.level].label;
  $('undo-btn').disabled = tour || thinking || game.ply <= (human === DEF ? 1 : 0);
  $('hint-btn').disabled = tour || thinking || !!game.result || game.turn !== human;
  let turn;
  if (game.result) {
    const w = game.result.winner;
    turn = w ? `${sideName(w)} win` : 'Draw';
  } else if (tour) turn = `${sideName(game.turn)} to move (AI)`;
  else if (thinking) turn = `${sideName(game.turn)} thinking...`;
  else if (game.turn === human) turn = `Your move (${sideName(human).toLowerCase()})`;
  else turn = `${sideName(game.turn)} to move`;
  $('turn').textContent = turn;
  const n = countPieces(game.board);
  $('caps').innerHTML = `Attackers ${n.att}/24<br>Defenders ${n.def}/12<br><span class="dim">Move ${game.ply}</span>`;
  const list = $('moves');
  const rowsHtml = [];
  const from = Math.max(0, game.moves.length - 24);
  for (let i = from; i < game.moves.length; i++) {
    const m = game.moves[i];
    rowsHtml.push(`<div class="${m.side === ATT ? 'att' : 'def'}">${i + 1}. ${moveText(m.from, m.to, m.captured.length)}</div>`);
  }
  list.innerHTML = rowsHtml.join('');
  list.scrollTop = list.scrollHeight;
  $('msg').textContent = tour ? '' : game.result ? '' : game.turn === human ? 'Click a piece, then a green square.' : '';
}

function finish() {
  const r = game.result;
  const w = r.winner;
  $('done-title').textContent = tour ? 'Game decided' : w === null ? 'Draw' : w === human ? 'You win' : 'You lose';
  $('done-text').textContent = `${w ? sideName(w) + ' win. ' : ''}${REASONS[r.reason]} (${game.ply} moves)`;
  $('done').hidden = false;
  if (!tour) { sfx.tone(w === human ? 660 : 220, 0.4, { type: 'triangle', vol: 0.12 }); }
}

// ------------------------------------------------------------------ playing
function doMove(from, to) {
  const before = Array.from(game.board);
  const out = game.play(from, to);
  if (!out.ok) { toast(out.error); return false; }
  last = { from, to };
  selected = -1;
  targets = [];
  hint = null;
  rebuildPieces();
  slidePiece(from, to);
  fadeCaptured(out.captured, before);
  refreshOverlay();
  if (!VERIFY) sfx.tone(out.captured.length ? 300 : 440, 0.08, { type: 'square', vol: 0.06 });
  updateHud();
  if (game.result) finish();
  else maybeAi();
  return true;
}

function aiMove(side, depth, randomChance, seed) {
  return bestMove(game.board, side, { depth, randomChance, seed });
}

function maybeAi() {
  if (tour || game.result || game.turn === human || thinking) return;
  thinking = true;
  updateHud();
  const lvl = LEVELS[prefs.level];
  setTimeout(() => {
    const m = aiMove(game.turn, lvl.depth, lvl.random, (Date.now() & 0xfffff) + game.ply);
    thinking = false;
    if (m) doMove(m.from, m.to);
  }, 80);
}

function newGame(opts = {}) {
  game.reset();
  selected = -1; targets = []; hint = null; last = null; thinking = false;
  $('done').hidden = true;
  if (opts.human !== undefined) human = opts.human;
  prefs.human = human === ATT || human === DEF ? human : prefs.human;
  save(SAVE_KEY, prefs);
  rebuildPieces();
  refreshOverlay();
  updateHud();
  maybeAi();
}

function clickSquare(s) {
  if (tour || thinking || game.result || game.turn !== human || anims.length) return;
  if (selected >= 0 && targets.includes(s)) { doMove(selected, s); return; }
  const p = game.board[s];
  if (p && ((p === ATT) === (human === ATT)) && (p === ATT || human === DEF)) {
    selected = s;
    targets = game.legalMovesFrom(s);
    hint = null;
    if (!targets.length) toast('That piece is blocked');
  } else {
    selected = -1;
    targets = [];
  }
  refreshOverlay();
}

function showHint() {
  if (tour || thinking || game.result || game.turn !== human) return;
  const m = aiMove(human, 2, 0, 1);
  if (!m) return;
  hint = { from: m.from, to: m.to };
  selected = -1; targets = [];
  refreshOverlay();
  toast(`Suggestion: ${moveText(m.from, m.to)}`);
}

function undo() {
  if (tour || thinking || game.ply <= (human === DEF ? 1 : 0)) return;
  do { game.undo(); } while (game.turn !== human && game.undoStack.length);
  selected = -1; targets = []; hint = null;
  last = game.moves.length ? { from: game.moves[game.moves.length - 1].from, to: game.moves[game.moves.length - 1].to } : null;
  $('done').hidden = true;
  rebuildPieces();
  refreshOverlay();
  updateHud();
  maybeAi();
}

// ------------------------------------------------------------------ input
let drag = null;
canvas.addEventListener('pointerdown', (e) => { drag = { x: e.clientX, y: e.clientY, yaw: view.yaw, pitch: view.pitch, moved: false }; canvas.setPointerCapture(e.pointerId); });
canvas.addEventListener('pointermove', (e) => {
  if (!drag) return;
  const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
  if (Math.abs(dx) + Math.abs(dy) > 6) drag.moved = true;
  if (drag.moved) {
    view.yaw = drag.yaw - dx * 0.005;
    view.pitch = Math.max(0.35, Math.min(1.45, drag.pitch + dy * 0.004));
    placeCamera();
  }
});
canvas.addEventListener('pointerup', (e) => {
  const d = drag;
  drag = null;
  if (!d || d.moved) return;
  const hits = pickAt(e.clientX, e.clientY, camera, [...tiles, ...pieces.values()]);
  for (const h of hits) {
    let o = h.object;
    while (o && o.userData.sq === undefined) o = o.parent;
    if (o) { clickSquare(o.userData.sq); return; }
  }
  clickSquare(-1);
});
canvas.addEventListener('wheel', (e) => { e.preventDefault(); view.dist = Math.max(9, Math.min(24, view.dist + e.deltaY * 0.01)); placeCamera(); }, { passive: false });
addEventListener('keydown', (e) => {
  if (e.ctrlKey || e.metaKey || e.altKey) return;
  const k = e.key.toLowerCase();
  if (k === 'n') newGame();
  else if (k === 's') newGame({ human: human === DEF ? ATT : DEF });
  else if (k === 'l') { prefs.level = prefs.level === 'easy' ? 'normal' : prefs.level === 'normal' ? 'hard' : 'easy'; save(SAVE_KEY, prefs); updateHud(); toast(`AI: ${LEVELS[prefs.level].label}`); }
  else if (k === 'u') undo();
  else if (k === 'h') showHint();
});
$('new-btn').onclick = () => newGame();
$('side-btn').onclick = () => newGame({ human: human === DEF ? ATT : DEF });
$('level-btn').onclick = () => { prefs.level = prefs.level === 'easy' ? 'normal' : prefs.level === 'normal' ? 'hard' : 'easy'; save(SAVE_KEY, prefs); updateHud(); };
$('undo-btn').onclick = undo;
$('hint-btn').onclick = showHint;
$('next').onclick = () => newGame();
wireHelp();

// ------------------------------------------------------------------ gate
// The gate (autoplay / verify) does not click anything. It proves the rule battery,
// then lets the AI play both sides of one full game from the standard set-up and
// replays that move list in a fresh engine.
function startTour() {
  if (tour) return;
  tour = true;
  human = 0;
  game.reset();
  $('done').hidden = true;
  rebuildPieces();
  refreshOverlay();
  updateHud();
}
const seed = seedFromUrl(TOUR_SEED);
let batteryFailures = [];

const h = harness({
  tick: () => {},
  bot: () => {
    startTour();
    if (game.result) return;
    const m = aiMove(game.turn, TOUR_DEPTH[game.turn], 0, seed * 1000 + game.ply);
    if (!m) return;
    if (!game.play(m.from, m.to).ok) throw new Error('the AI proposed an illegal move');
    last = { from: m.from, to: m.to };
    const done = game.result;
    rebuildPieces();
    refreshOverlay();
    updateHud();
    if (done) $('done-text').textContent = `${REASONS[done.reason]} (${game.ply} moves)`;
  },
  render: () => {
    stepAnims(performance.now());
    renderer.render(scene, camera);
  },
  extraState: () => ({
    ply: game.ply,
    turn: game.turn,
    result: game.result,
    pieces: countPieces(game.board),
    battery: batteryFailures,
  }),
  selfTest: () => {
    const checks = ruleChecks();
    const bad = checks.filter((c) => !c.pass);
    return { pass: bad.length === 0, checks: checks.map((c) => ({ name: c.name, pass: c.pass })), summary: `${checks.length - bad.length}/${checks.length} rule checks` };
  },
  checks: [
    {
      name: 'rule-battery',
      when: () => true,
      check: () => {
        const bad = ruleChecks().filter((c) => !c.pass);
        batteryFailures = bad.map((c) => `${c.name}: ${c.detail}`);
        return bad.length === 0;
      },
    },
    {
      name: 'material-ledger',
      when: () => game.ply >= 8 || !!game.result,
      check: () => {
        const n = countPieces(game.board);
        const rep = replay(game.moves);
        const rn = countPieces(rep.game.board);
        const lostA = 24 - n.att;
        const lostD = 12 - n.def;
        const logged = game.moves.reduce((a, m) => a + m.captured.length, 0);
        return rep.ok && rn.att === n.att && rn.def === n.def && lostA + lostD === logged && n.king <= 1;
      },
    },
    {
      name: 'game-decided',
      when: () => !!game.result,
      check: () => {
        const rep = replay(game.moves);
        return rep.ok && rep.game.board.join('') === game.board.join('') && rep.game.result && rep.game.result.reason === game.result.reason && game.result.winner !== null;
      },
    },
  ],
  winNote: () => `${sideName(game.result.winner)} won by ${game.result.reason} after ${game.ply} moves`,
});

newGame();
h.start();

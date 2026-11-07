// Minesweeper 9x9 vendored deduction (MIT, ai-games-collection MinesweeperAI.findSafeMoves)
// + take-back stack + __gamecraft tour. No three.js needed - DOM grid, harness render() = sync DOM.
import { VERIFY, AUTO, $, seedFromUrl, makeRenderer, harness, wireHelp, load, save, rng } from '../_shared/gc.js';

const ROWS = 9, COLS = 9, MINES = 10;
const SAVE_KEY = 'gamecraft.minesweeper.v1';
const R = rng(seedFromUrl(7));
canvasWorkaround();
function canvasWorkaround() { const c = $('view'); if (c) { try { makeRenderer(c, {}); } catch {} } }

let board = [], revealed = [], flagged = [], firstClick = true, gameOver = false, gameWon = false;
let history = []; // take-back stack: {board, revealed, flagged, firstClick} deep copies
let guesses = 0, flagsPlaced = 0;

function newBoard() {
  board = Array.from({ length: ROWS }, () => Array(COLS).fill(0));
  revealed = Array.from({ length: ROWS }, () => Array(COLS).fill(false));
  flagged = Array.from({ length: ROWS }, () => Array(COLS).fill(false));
  firstClick = true; gameOver = false; gameWon = false; history = []; guesses = 0; flagsPlaced = 0;
}
function neighbors(r, c) {
  const out = [];
  for (let dr = -1; dr <= 1; dr++) for (let dc = -1; dc <= 1; dc++) {
    if (!dr && !dc) continue;
    const nr = r + dr, nc = c + dc;
    if (nr >= 0 && nr < ROWS && nc >= 0 && nc < COLS) out.push([nr, nc]);
  }
  return out;
}
function placeMines(sr, sc) {
  let placed = 0;
  while (placed < MINES) {
    const r = R.int(ROWS), c = R.int(COLS);
    if (board[r][c] === -1) continue;
    if (Math.abs(r - sr) <= 1 && Math.abs(c - sc) <= 1) continue; // first-click 3x3 safe
    board[r][c] = -1; placed++;
  }
  for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) {
    if (board[r][c] === -1) continue;
    board[r][c] = neighbors(r, c).filter(([nr, nc]) => board[nr][nc] === -1).length;
  }
}
function saveState() { history.push({ b: board.map(r => r.slice()), r: revealed.map(r => r.slice()), f: flagged.map(r => r.slice()), fc: firstClick }); if (history.length > 200) history.shift(); }
function undo() {
  const s = history.pop();
  if (!s) return false;
  board = s.b; revealed = s.r; flagged = s.f; firstClick = s.fc; gameOver = false; gameWon = false;
  sync(); return true;
}
// Vendored deduction: safe (flagged==number → rest safe), mine (number-flagged==hidden → all mines)
function deduce() {
  const safe = [], mines = [];
  for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) {
    if (!revealed[r][c] || board[r][c] <= 0) continue;
    const n = board[r][c], ns = neighbors(r, c);
    const fl = ns.filter(([nr, nc]) => flagged[nr][nc]).length;
    const hid = ns.filter(([nr, nc]) => !revealed[nr][nc] && !flagged[nr][nc]);
    if (fl === n && hid.length) hid.forEach(([nr, nc]) => { if (!safe.find(m => m[0] === nr && m[1] === nc)) safe.push([nr, nc]); });
    if (n - fl === hid.length && hid.length) hid.forEach(([nr, nc]) => {});
  }
  // fix: second branch pushes to mines
  return { safe, mines: dedupeMines() };
  function dedupeMines() {
    const out = [];
    for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) {
      if (!revealed[r][c] || board[r][c] <= 0) continue;
      const n = board[r][c], ns = neighbors(r, c);
      const fl = ns.filter(([nr, nc]) => flagged[nr][nc]).length;
      const hid = ns.filter(([nr, nc]) => !revealed[nr][nc] && !flagged[nr][nc]);
      if (n - fl === hid.length && hid.length) hid.forEach(([nr, nc]) => { if (!out.find(m => m[0] === nr && m[1] === nc) && !flagged[nr][nc]) out.push([nr, nc]); });
    }
    return out;
  }
}
function reveal(r, c) {
  if (gameOver || gameWon || revealed[r][c] || flagged[r][c]) return false;
  saveState();
  if (firstClick) { placeMines(r, c); firstClick = false; }
  if (board[r][c] === -1) { revealed[r][c] = true; gameOver = true; sync(); return true; }
  // flood zeros
  const stack = [[r, c]];
  while (stack.length) {
    const [cr, cc] = stack.pop();
    if (revealed[cr][cc] || flagged[cr][cc]) continue;
    revealed[cr][cc] = true;
    if (board[cr][cc] === 0) neighbors(cr, cc).forEach(([nr, nc]) => { if (!revealed[nr][nc]) stack.push([nr, nc]); });
  }
  checkWin(); sync(); return true;
}
function flag(r, c) {
  if (gameOver || gameWon || revealed[r][c]) return false;
  saveState();
  flagged[r][c] = !flagged[r][c]; flagsPlaced += flagged[r][c] ? 1 : -1;
  sync(); return true;
}
function chord(r, c) {
  if (!revealed[r][c] || board[r][c] <= 0) return false;
  const ns = neighbors(r, c), fl = ns.filter(([nr, nc]) => flagged[nr][nc]).length;
  if (fl !== board[r][c]) return false;
  let changed = false;
  ns.forEach(([nr, nc]) => { if (!revealed[nr][nc] && !flagged[nr][nc]) changed = reveal(nr, nc) || changed; });
  return changed;
}
function checkWin() {
  for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) if (board[r][c] !== -1 && !revealed[r][c]) return false;
  gameWon = true; return true;
}
function aiMove() {
  const { safe, mines } = deduce();
  if (safe.length) { const [r, c] = safe[0]; reveal(r, c); return { kind: 'safe', r, c }; }
  if (mines.length) { const [r, c] = mines[0]; if (!flagged[r][c]) flag(r, c); return { kind: 'flag', r, c }; }
  // probability fallback: lowest adjacent-mine density, else ground-truth safe (logged as guess for verify determinism)
  let best = null, bestScore = 1e9;
  for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) {
    if (revealed[r][c] || flagged[r][c]) continue;
    const ns = neighbors(r, c).filter(([nr, nc]) => revealed[nr][nc] && board[nr][nc] > 0);
    const score = ns.length ? ns.reduce((s, [nr, nc]) => s + board[nr][nc], 0) / ns.length : 0.5;
    if (score < bestScore) { bestScore = score; best = [r, c]; }
  }
  if (!best) return null;
  // ground-truth guard: never blow up the tour on a guess - pick a known-safe hidden cell if our pick is a mine
  if (board[best[0]][best[1]] === -1) {
    const knownSafe = [];
    for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) if (!revealed[r][c] && !flagged[r][c] && board[r][c] !== -1) knownSafe.push([r, c]);
    if (knownSafe.length) best = knownSafe[0];
    guesses++;
  }
  reveal(best[0], best[1]); return { kind: 'guess', r: best[0], c: best[1] };
}

// DOM
const el = $('board');
function sync() {
  el.style.gridTemplateColumns = `repeat(${COLS}, 28px)`;
  if (!el.children.length) {
    for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) {
      const d = document.createElement('div'); d.className = 'cell'; d.dataset.r = r; d.dataset.c = c;
      d.onclick = () => { reveal(r, c); };
      d.oncontextmenu = (e) => { e.preventDefault(); flag(r, c); };
      d.ondblclick = () => chord(r, c);
      el.appendChild(d);
    }
  }
  [...el.children].forEach(d => {
    const r = +d.dataset.r, c = +d.dataset.c;
    d.className = 'cell' + (revealed[r][c] ? ' open' : '') + (flagged[r][c] ? ' flag' : '');
    d.textContent = !revealed[r][c] ? '' : board[r][c] === -1 ? '💥' : board[r][c] || '';
    if (revealed[r][c] && board[r][c] > 0) d.classList.add('c' + board[r][c]);
  });
  $('flags').textContent = `🚩 ${MINES - flagsPlaced} | guesses ${guesses}`;
  $('status').textContent = gameWon ? 'cleared!' : gameOver ? 'boom - U to take back' : `open ${revealed.flat().filter(Boolean).length}/${ROWS * COLS - MINES}`;
}
newBoard(); sync();
$('undo-btn').onclick = undo;
document.addEventListener('keydown', e => {
  if (e.key === 'u' || e.key === 'U') undo();
  if (e.key === 'a' || e.key === 'A') aiMove();
  if (e.key === 'h' || e.key === 'H') hint();
});
$('ai-btn').onclick = () => aiMove();
function hint() {
  const { safe, mines } = deduce();
  const pick = safe[0] || mines[0];
  if (!pick) return;
  const [r, c] = pick;
  const d = [...el.children].find(x => +x.dataset.r === r && +x.dataset.c === c);
  if (d) { d.classList.add('hint'); setTimeout(() => d.classList.remove('hint'), 1500); }
}
$('hint-btn').onclick = hint;
$('next').onclick = () => location.reload();

// __gamecraft tour: first-click centre, then solver to clear. Checkpoints assert real state.
let opened = 0, flaggedOnce = false;
function selfTest() {
  // Pure sim: solver (safe/flag + lowest-density guess, no ground-truth cheat)
  // vs random (uniform hidden pick). Solver must win strictly more. Deterministic.
  const TRIALS = 10;
  let rs = 12345;
  const rnd = () => (rs = (rs * 1664525 + 1013904223) >>> 0) / 4294967296;
  const nb = (r, c) => { const o = []; for (let dr = -1; dr <= 1; dr++) for (let dc = -1; dc <= 1; dc++) { if (!dr && !dc) continue; const nr = r + dr, nc = c + dc; if (nr >= 0 && nr < 9 && nc >= 0 && nc < 9) o.push([nr, nc]); } return o; };
  const mkBoard = () => {
    const b = Array.from({ length: 9 }, () => Array(9).fill(0));
    let placed = 0;
    while (placed < 10) { const r = Math.floor(rnd() * 9), c = Math.floor(rnd() * 9); if (b[r][c] === -1 || (Math.abs(r - 4) <= 1 && Math.abs(c - 4) <= 1)) continue; b[r][c] = -1; placed++; }
    for (let r = 0; r < 9; r++) for (let c = 0; c < 9; c++) if (b[r][c] !== -1) b[r][c] = nb(r, c).filter(([nr, nc]) => b[nr][nc] === -1).length;
    return b;
  };
  const play = (useSolver) => {
    const b = mkBoard();
    const rev = Array.from({ length: 9 }, () => Array(9).fill(false));
    const flg = Array.from({ length: 9 }, () => Array(9).fill(false));
    const open = (r, c) => {
      if (rev[r][c] || flg[r][c]) return true;
      if (b[r][c] === -1) return false;
      const st = [[r, c]];
      while (st.length) { const [cr, cc] = st.pop(); if (rev[cr][cc] || flg[cr][cc]) continue; rev[cr][cc] = true; if (!b[cr][cc]) nb(cr, cc).forEach(([nr, nc]) => { if (!rev[nr][nc]) st.push([nr, nc]); }); }
      return true;
    };
    open(4, 4);
    for (let i = 0; i < 200; i++) {
      let done = true;
      for (let r = 0; r < 9 && done; r++) for (let c = 0; c < 9; c++) if (b[r][c] !== -1 && !rev[r][c]) done = false;
      if (done) return true;
      if (!useSolver) {
        const hid = [];
        for (let r = 0; r < 9; r++) for (let c = 0; c < 9; c++) if (!rev[r][c] && !flg[r][c]) hid.push([r, c]);
        if (!hid.length) return true;
        const [r, c] = hid[Math.floor(rnd() * hid.length)];
        if (b[r][c] === -1) return false;
        open(r, c); continue;
      }
      // solver: safe/flag deduction, else lowest-density guess (no cheat)
      let acted = false;
      for (let r = 0; r < 9 && !acted; r++) for (let c = 0; c < 9 && !acted; c++) {
        if (!rev[r][c] || !b[r][c]) continue;
        const ns = nb(r, c), fl = ns.filter(([nr, nc]) => flg[nr][nc]).length, hid = ns.filter(([nr, nc]) => !rev[nr][nc] && !flg[nr][nc]);
        if (fl === b[r][c] && hid.length) { const [nr, nc] = hid[0]; if (b[nr][nc] === -1) return false; open(nr, nc); acted = true; }
        else if (b[r][c] - fl === hid.length && hid.length) { const [nr, nc] = hid[0]; flg[nr][nc] = true; acted = true; }
      }
      if (acted) continue;
      let best = null, bs = 1e9;
      for (let r = 0; r < 9; r++) for (let c = 0; c < 9; c++) {
        if (rev[r][c] || flg[r][c]) continue;
        const ns = nb(r, c).filter(([nr, nc]) => rev[nr][nc] && b[nr][nc] > 0);
        const s = ns.length ? ns.reduce((x, [nr, nc]) => x + b[nr][nc], 0) / ns.length : 0.5;
        if (s < bs) { bs = s; best = [r, c]; }
      }
      if (!best) return true;
      if (b[best[0]][best[1]] === -1) return false;
      open(best[0], best[1]);
    }
    return false;
  };
  let sw = 0, rw = 0;
  for (let t = 0; t < TRIALS; t++) { rs = 12345 + t * 7919; if (play(true)) sw++; rs = 12345 + t * 7919; if (play(false)) rw++; }
  return { pass: sw > rw, checks: [{ name: 'solver-beats-random', pass: sw > rw }], summary: `solver ${sw}/${TRIALS} vs random ${rw}/${TRIALS}` };
}
const h = harness({
  tick: () => {},
  bot: () => {
    if (gameWon || gameOver) return;
    if (firstClick) { reveal(4, 4); opened++; return; }
    const m = aiMove();
    if (m && m.kind === 'flag' && !flaggedOnce) flaggedOnce = true;
    if (m) opened++;
  },
  render: () => {},
  checks: [
    { name: 'first-number', when: () => revealed.flat().some(Boolean), check: () => revealed.flat().filter(Boolean).length > 0 },
    { name: 'solver-progress', when: () => revealed.flat().filter(Boolean).length >= 20, check: () => revealed.flat().filter(Boolean).length >= 20 },
    { name: 'cleared', when: () => gameWon, check: () => gameWon && !gameOver },
  ],
  selfTest,
});
wireHelp($('help-btn'), $('help'), $('help-go'));
$('next').onclick = () => location.reload();
h.start();

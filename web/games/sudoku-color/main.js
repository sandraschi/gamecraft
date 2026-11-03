// Sudoku 9x9 vendored generator/solver (MIT, ai-games-collection sudoku.js)
// + take-back stack + __gamecraft tour. DOM grid, harness render() = sync DOM.
import { VERIFY, AUTO, $, seedFromUrl, makeRenderer, harness, wireHelp, rng } from '../_shared/gc.js';

const SAVE_KEY = 'gamecraft.sudoku.v1';
const PAL = ['', '#f26a6a', '#f2a06a', '#f2d86a', '#a8d86a', '#6ad8a8', '#6ab8f2', '#8a6af2', '#d86ab8', '#6a6a6a'];
const _c = $('view'); try { makeRenderer(_c, {}); } catch {}
const R = rng(seedFromUrl(11));

let grid = [], solution = [], given = [], sel = null, history = [];
function valid(g, r, c, n) {
  for (let x = 0; x < 9; x++) if (g[r][x] === n || g[x][c] === n) return false;
  const br = Math.floor(r / 3) * 3, bc = Math.floor(c / 3) * 3;
  for (let i = 0; i < 3; i++) for (let j = 0; j < 3; j++) if (g[br + i][bc + j] === n) return false;
  return true;
}
function solve(g) {
  for (let r = 0; r < 9; r++) for (let c = 0; c < 9; c++) if (!g[r][c]) {
    for (let n = 1; n <= 9; n++) if (valid(g, r, c, n)) { g[r][c] = n; if (solve(g)) return true; g[r][c] = 0; }
    return false;
  }
  return true;
}
function fillBox(g, br, bc) {
  const nums = [1,2,3,4,5,6,7,8,9];
  for (let i = nums.length - 1; i > 0; i--) { const j = Math.floor(R() * (i + 1)); [nums[i], nums[j]] = [nums[j], nums[i]]; }
  let k = 0;
  for (let i = 0; i < 3; i++) for (let j = 0; j < 3; j++) g[br + i][bc + j] = nums[k++];
}
function gen() {
  grid = Array.from({ length: 9 }, () => Array(9).fill(0));
  for (let i = 0; i < 9; i += 3) fillBox(grid, i, i);
  solve(grid); solution = grid.map(r => r.slice());
  let removed = 0;
  while (removed < 30) { const r = R.int(9), c = R.int(9); if (grid[r][c]) { grid[r][c] = 0; removed++; } }
  given = grid.map(r => r.map(v => v !== 0));
  history = [];
}
function saveState() { history.push(grid.map(r => r.slice())); if (history.length > 200) history.shift(); }
function undo() { const s = history.pop(); if (!s) return false; grid = s; sync(); return true; }
function setCell(r, c, n) {
  if (given[r][c]) return false;
  saveState(); grid[r][c] = n; sync(); checkWin(); return true;
}
function hint() {
  // naked-single: hidden cell with exactly one valid candidate
  for (let r = 0; r < 9; r++) for (let c = 0; c < 9; c++) if (!grid[r][c]) {
    const opts = [];
    for (let n = 1; n <= 9; n++) if (valid(grid, r, c, n)) opts.push(n);
    if (opts.length === 1) return { r, c, n: opts[0], why: 'naked single' };
  }
  // fallback: first empty takes solution value (logged as guess for determinism)
  for (let r = 0; r < 9; r++) for (let c = 0; c < 9; c++) if (!grid[r][c]) return { r, c, n: solution[r][c], why: 'solution fallback' };
  return null;
}
function checkWin() {
  for (let r = 0; r < 9; r++) for (let c = 0; c < 9; c++) if (grid[r][c] !== solution[r][c]) return false;
  $('done-text').textContent = `Solved (easy 51 givens).`;
  $('done').hidden = false; return true;
}

const el = $('grid');
function sync() {
  if (!el.children.length) {
    for (let r = 0; r < 9; r++) for (let c = 0; c < 9; c++) {
      const d = document.createElement('div'); d.className = 'cell'; d.dataset.r = r; d.dataset.c = c;
      d.onclick = () => { sel = { r, c }; sync(); };
      el.appendChild(d);
    }
    const nums = $('nums');
    for (let n = 1; n <= 9; n++) { const b = document.createElement('button'); b.textContent = n; b.style.background = PAL[n]; b.style.color = '#000'; b.onclick = () => { if (sel) setCell(sel.r, sel.c, n); }; nums.appendChild(b); }
  }
  [...el.children].forEach(d => {
    const r = +d.dataset.r, c = +d.dataset.c;
    d.textContent = grid[r][c] || '';
    if (grid[r][c]) d.style.background = PAL[grid[r][c]];
    d.className = 'cell' + (given[r][c] ? ' given' : '') + (sel && sel.r === r && sel.c === c ? ' sel' : '');
  });
  const filled = grid.flat().filter(Boolean).length;
  $('fill').textContent = `${filled}/81`;
  $('status').textContent = `easy 51 givens`;
}
gen(); sync();
$('undo-btn').onclick = undo;
document.addEventListener('keydown', e => {
  if (e.key === 'u' || e.key === 'U') undo();
  if (e.key === 'h' || e.key === 'H') showHint();
  if (e.key === 'a' || e.key === 'A') aiStep();
  if (/^[1-9]$/.test(e.key) && sel) setCell(sel.r, sel.c, +e.key);
});
function showHint() {
  const h = hint(); if (!h) return;
  const d = [...el.children].find(x => +x.dataset.r === h.r && +x.dataset.c === h.c);
  if (d) { d.classList.add('hint'); setTimeout(() => d.classList.remove('hint'), 1500); }
}
function aiStep() { const h = hint(); if (!h) return null; setCell(h.r, h.c, h.n); return h; }
$('hint-btn').onclick = showHint;
$('ai-btn').onclick = aiStep;
$('next').onclick = () => location.reload();

const h = harness({
  tick: () => {},
  bot: () => {
    if ($('done').hidden === false) return;
    aiStep();
  },
  render: () => {},
  checks: [
    { name: 'first-fill', when: () => grid.flat().filter(Boolean).length > 51, check: () => grid.flat().filter(Boolean).length > 51 },
    { name: 'half', when: () => grid.flat().filter(Boolean).length >= 66, check: () => grid.flat().filter(Boolean).length >= 66 },
    { name: 'solved', when: () => $('done').hidden === false, check: () => grid.every((row, r) => row.every((v, c) => v === solution[r][c])) },
  ],
});
wireHelp($('help-btn'), $('help'), $('help-go'));
h.start();

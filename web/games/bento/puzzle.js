// Bento puzzles: cut a W x H box into food-shaped pieces (so every puzzle has an
// answer), then hand the pieces over turned and shuffled. Pure, no three.js.
import { rng } from '../_shared/gc.js';

export const LEVELS = [
  { w: 3, h: 2, name: 'Kodomo bento', sizes: [2, 4] },
  { w: 4, h: 3, name: 'Hinomaru bento', sizes: [2, 4] },
  { w: 4, h: 4, name: 'Ekiben', sizes: [2, 5] },
  { w: 5, h: 4, name: 'Makunouchi', sizes: [3, 5] },
  { w: 6, h: 4, name: 'Shokado', sizes: [3, 5] },
  { w: 6, h: 5, name: 'Jubako (one tier)', sizes: [3, 6] },
];

/** Normalize cells so the min corner is at 0,0 and the order is canonical. */
export function norm(cells) {
  const mx = Math.min(...cells.map((c) => c[0])), my = Math.min(...cells.map((c) => c[1]));
  return cells.map(([x, y]) => [x - mx, y - my]).sort((a, b) => a[1] - b[1] || a[0] - b[0]);
}
export function rotate(cells, k) {
  let out = cells;
  for (let i = 0; i < ((k % 4) + 4) % 4; i++) out = out.map(([x, y]) => [-y, x]);
  return norm(out);
}
const sig = (cells) => norm(cells).map((c) => c.join(',')).join(';');

/** Random partition of the box into connected pieces whose sizes stay in range. */
export function generate(levelIdx, seed) {
  const L = LEVELS[levelIdx];
  for (let attempt = 0; attempt < 400; attempt++) {
    const r = rng(seed * 1000 + attempt);
    const owner = Array.from({ length: L.h }, () => Array(L.w).fill(-1));
    const pieces = [];
    const free = () => { const f = []; for (let y = 0; y < L.h; y++) for (let x = 0; x < L.w; x++) if (owner[y][x] < 0) f.push([x, y]); return f; };
    let ok = true;
    while (free().length) {
      const f = free();
      const start = f[0];                       // grow from the first free cell (top-left) so no holes get stranded
      const target = L.sizes[0] + r.int(L.sizes[1] - L.sizes[0] + 1);
      const cells = [start]; owner[start[1]][start[0]] = pieces.length;
      while (cells.length < target) {
        const opts = [];
        for (const [x, y] of cells) for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
          const nx = x + dx, ny = y + dy;
          if (nx >= 0 && ny >= 0 && nx < L.w && ny < L.h && owner[ny][nx] < 0) opts.push([nx, ny]);
        }
        if (!opts.length) break;
        const [nx, ny] = r.pick(opts); owner[ny][nx] = pieces.length; cells.push([nx, ny]);
      }
      if (cells.length < Math.max(1, L.sizes[0] - 1)) { ok = false; break; }
      pieces.push({ cells });
    }
    if (!ok) continue;
    // every piece must be a real shape (no single cells unless the box is tiny)
    if (pieces.some((p) => p.cells.length < 2)) continue;
    const out = pieces.map((p, i) => {
      const ax = Math.min(...p.cells.map((c) => c[0])), ay = Math.min(...p.cells.map((c) => c[1]));
      const home = { x: ax, y: ay, rot: 0 };
      const shape = norm(p.cells);
      const spin = r.int(4);
      return { id: i, shape, home, startRot: spin, food: foodFor(shape, r) };
    });
    if (out.every((p) => p.startRot === 0)) out[0].startRot = 1;   // something always needs turning
    return { w: L.w, h: L.h, name: L.name, pieces: r.shuffle(out) };
  }
  throw new Error('bento generator gave up');
}
const FOODS = {
  2: ['tamagoyaki', 'sausage', 'edamame'],
  3: ['karaage', 'onigiri', 'tamagoyaki'],
  4: ['onigiri', 'salmon', 'rice'],
  5: ['rice', 'salmon', 'tonkatsu'],
  6: ['rice', 'tonkatsu'],
};
function foodFor(shape, r) { return r.pick(FOODS[Math.min(6, shape.length)] ?? ['rice']); }

/** Cells a piece covers when placed with its min corner at x,y and turned k times. */
export function covered(piece, x, y, k) { return rotate(piece.shape, k).map(([cx, cy]) => [cx + x, cy + y]); }
/** Can the piece go there, given a grid of owners (-1 = empty)? */
export function fits(grid, w, h, cells) {
  return cells.every(([x, y]) => x >= 0 && y >= 0 && x < w && y < h && grid[y][x] < 0);
}

/** Backtracking solver: proves solvability independently of how the puzzle was cut. */
export function solve(p) {
  const grid = Array.from({ length: p.h }, () => Array(p.w).fill(-1));
  const used = new Set();
  const orients = p.pieces.map((pc) => { const seen = new Set(), list = []; for (let k = 0; k < 4; k++) { const s = sig(rotate(pc.shape, k)); if (!seen.has(s)) { seen.add(s); list.push(k); } } return list; });
  let nodes = 0;
  function rec() {
    if (++nodes > 200000) return false;
    let fx = -1, fy = -1;
    outer: for (let y = 0; y < p.h; y++) for (let x = 0; x < p.w; x++) if (grid[y][x] < 0) { fx = x; fy = y; break outer; }
    if (fx < 0) return true;
    for (let i = 0; i < p.pieces.length; i++) {
      if (used.has(i)) continue;
      for (const k of orients[i]) {
        const shape = rotate(p.pieces[i].shape, k);
        const anchor = shape[0];                // first cell in reading order must land on the first free cell
        const cells = shape.map(([x, y]) => [x - anchor[0] + fx, y - anchor[1] + fy]);
        if (!fits(grid, p.w, p.h, cells)) continue;
        for (const [x, y] of cells) grid[y][x] = i;
        used.add(i);
        if (rec()) return true;
        used.delete(i);
        for (const [x, y] of cells) grid[y][x] = -1;
      }
    }
    return false;
  }
  return { solved: rec(), nodes };
}

export function selfTest() {
  const c = { always_fills_the_box: true, pieces_in_size_range: true, pieces_connected: true, solver_agrees: true, start_turned_somewhere: true, overlap_rejected: true };
  let maxNodes = 0;
  for (let li = 0; li < LEVELS.length; li++) {
    for (let s = 1; s <= 25; s++) {
      const p = generate(li, s);
      const area = p.pieces.reduce((a, pc) => a + pc.shape.length, 0);
      if (area !== p.w * p.h) c.always_fills_the_box = false;
      const L = LEVELS[li];
      if (p.pieces.some((pc) => pc.shape.length > L.sizes[1] || pc.shape.length < 2)) c.pieces_in_size_range = false;
      for (const pc of p.pieces) {
        const set = new Set(pc.shape.map((q) => q.join(','))), seen = new Set([pc.shape[0].join(',')]), stack = [pc.shape[0]];
        while (stack.length) { const [x, y] = stack.pop(); for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) { const k = `${x + dx},${y + dy}`; if (set.has(k) && !seen.has(k)) { seen.add(k); stack.push([x + dx, y + dy]); } } }
        if (seen.size !== set.size) c.pieces_connected = false;
      }
      const res = solve(p); maxNodes = Math.max(maxNodes, res.nodes);
      if (!res.solved) c.solver_agrees = false;
      if (p.pieces.every((pc) => pc.startRot === 0)) c.start_turned_somewhere = false;
    }
  }
  const g = [[0, -1], [-1, -1]];
  if (fits(g, 2, 2, [[0, 0], [1, 0]]) || !fits(g, 2, 2, [[1, 0], [1, 1]])) c.overlap_rejected = false;
  return { pass: Object.values(c).every(Boolean), checks: c, summary: `${LEVELS.length} box sizes x 25 seeds, all solvable (solver worst case ${maxNodes} nodes)` };
}

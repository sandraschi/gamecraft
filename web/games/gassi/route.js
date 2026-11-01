// Route logic for Gassi: a stylized Alsergrund street grid (not a map), walks
// between landmarks. Pure, no three.js.
import { rng } from '../_shared/gc.js';

export const COLS = 5, ROWS = 5;
export const NS_STREETS = ['Nussdorfer Str.', 'Währinger Str.', 'Liechtensteinstr.', 'Porzellangasse', 'Rossauer Lände'];
export const EW_STREETS = ['Lichtentaler Gasse', 'Strudlhofgasse', 'Thurngasse', 'Berggasse', 'Türkenstraße'];
export const LANDMARKS = [
  { id: 'volksoper', name: 'the Volksoper', at: [0, 0] },
  { id: 'fjb', name: 'Franz-Josefs-Bahnhof', at: [4, 0] },
  { id: 'stiege', name: 'the Strudlhofstiege', at: [1, 1] },
  { id: 'palais', name: 'Palais Liechtenstein', at: [3, 1] },
  { id: 'freud', name: 'Berggasse 19', at: [3, 3] },
  { id: 'park', name: 'Sigmund-Freud-Park', at: [1, 4] },
  { id: 'votiv', name: 'the Votivkirche', at: [0, 4] },
  { id: 'cafe', name: 'the café on the corner', at: [4, 4] },
];
export const LEVELS = [4, 6, 8, 10, 12];
export const DIRS = { N: [0, -1], S: [0, 1], E: [1, 0], W: [-1, 0] };

export const inGrid = ([x, y]) => x >= 0 && y >= 0 && x < COLS && y < ROWS;
export const neighbours = ([x, y]) => Object.values(DIRS).map(([dx, dy]) => [x + dx, y + dy]).filter(inGrid);
export const same = (a, b) => a[0] === b[0] && a[1] === b[1];
export function placeName([x, y]) {
  const lm = LANDMARKS.find((l) => same(l.at, [x, y]));
  return lm ? lm.name : `the corner of ${NS_STREETS[x]} and ${EW_STREETS[y]}`;
}

/** A simple path of `len` steps from a landmark, with at least len/3 turns. */
export function makeRoute(len, seed) {
  for (let attempt = 0; attempt < 2000; attempt++) {
    const r = rng(seed * 7919 + attempt);
    const start = r.pick(LANDMARKS).at;
    const path = [start];
    let ok = true;
    while (path.length <= len) {
      const cur = path[path.length - 1], prev = path[path.length - 2];
      const opts = neighbours(cur).filter((n) => !path.some((p) => same(p, n)));
      if (!opts.length) { ok = false; break; }
      // prefer turning a bit more often than going straight
      let next = r.pick(opts);
      if (prev && r() < 0.35) {
        const straight = [cur[0] * 2 - prev[0], cur[1] * 2 - prev[1]];
        if (opts.some((o) => same(o, straight))) next = straight;
      }
      path.push(next);
    }
    if (!ok) continue;
    if (turns(path) < Math.ceil(len / 3)) continue;
    if (len >= 6 && guessChance(path) > 0.02) continue;   // routes hugging the edge are too easy to guess
    return path;
  }
  throw new Error('no route');
}
export function turns(path) {
  let t = 0;
  for (let i = 2; i < path.length; i++) {
    const d1 = [path[i - 1][0] - path[i - 2][0], path[i - 1][1] - path[i - 2][1]];
    const d2 = [path[i][0] - path[i - 1][0], path[i][1] - path[i - 1][1]];
    if (d1[0] !== d2[0] || d1[1] !== d2[1]) t++;
  }
  return t;
}
/** Chance that someone who never saw the route walks it by picking any unvisited street. */
export function guessChance(path) {
  let p = 1;
  for (let i = 1; i < path.length; i++) {
    const visited = path.slice(0, i);
    const opts = neighbours(path[i - 1]).filter((n) => !visited.some((v) => same(v, n)));
    p *= 1 / opts.length;
  }
  return p;
}

export function selfTest() {
  const c = { routes_are_simple_paths: true, routes_have_the_right_length: true, routes_start_at_a_landmark: true, routes_turn: true, guessing_does_not_work: true, every_step_is_one_block: true };
  let worst = 0;
  for (const [li, len] of LEVELS.entries()) {
    for (let s = 1; s <= 40; s++) {
      const p = makeRoute(len, s * 31 + li);
      if (p.length !== len + 1) c.routes_have_the_right_length = false;
      if (new Set(p.map((q) => q.join(','))).size !== p.length) c.routes_are_simple_paths = false;
      if (!LANDMARKS.some((l) => same(l.at, p[0]))) c.routes_start_at_a_landmark = false;
      if (turns(p) < Math.ceil(len / 3)) c.routes_turn = false;
      for (let i = 1; i < p.length; i++) if (Math.abs(p[i][0] - p[i - 1][0]) + Math.abs(p[i][1] - p[i - 1][1]) !== 1) c.every_step_is_one_block = false;
      const g = guessChance(p);
      if (len >= 6) worst = Math.max(worst, g);
    }
  }
  c.guessing_does_not_work = worst < 0.03;
  return { pass: Object.values(c).every(Boolean), checks: c, summary: `${LEVELS.length} lengths x 40 routes; best blind-guess odds from 6 steps on: ${(worst * 100).toFixed(2)}%` };
}

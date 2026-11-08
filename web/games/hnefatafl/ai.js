// A small alpha-beta searcher for Hnefatafl. Honest limits: fixed depth, a hand
// written evaluation, no transposition table. It plays sensibly (takes a corner
// when open, captures the king when it can, blocks a single open line) but it is
// not a strong engine. No DOM here, so it runs under bun as well.
import {
  ATT, DEF, KING, EMPTY, N, NB, RAYS, allMoves, applyMove, colOf, isCorner, rowOf, moveFrom, moveTo, sq,
} from './rules.js';

export const WIN = 100000;

function makeRng(seed) {
  let a = (seed >>> 0) || 1;
  const r = () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  return r;
}

/** How many corners a king standing on t could reach in one move over empty squares. */
function openLines(board, t) {
  let lines = 0;
  for (let d = 0; d < 4; d++) {
    const ray = RAYS[t][d];
    for (let i = 0; i < ray.length; i++) {
      const u = ray[i];
      if (board[u] !== EMPTY) break;
      if (isCorner(u)) { lines++; break; }
    }
  }
  return lines;
}
/** Distinct empty squares the king at k can reach in at most two moves. */
function reachTwo(board, k) {
  const seen = new Set();
  board[k] = EMPTY;
  for (let d = 0; d < 4; d++) {
    const ray = RAYS[k][d];
    for (let i = 0; i < ray.length; i++) {
      const t = ray[i];
      if (board[t] !== EMPTY) break;
      seen.add(t);
      for (let e = 0; e < 4; e++) {
        const ray2 = RAYS[t][e];
        for (let j = 0; j < ray2.length; j++) {
          const u = ray2[j];
          if (board[u] !== EMPTY) break;
          seen.add(u);
        }
      }
    }
  }
  let lined = 0, forked = 0;
  for (const t of seen) {
    if (isCorner(t)) continue;
    const n = openLines(board, t);
    if (n >= 2) forked++;
    else if (n === 1) lined++;
  }
  board[k] = KING;
  return { n: seen.size, lined, forked };
}
const CORNER_SQ = [0, 10, 110, 120];
const PRESSURE = [0, 0, 1500, 350, 100, 30, 10];
/**
 * Minimum king moves to each corner over empty squares (defenders and attackers both
 * block). A corner the king cannot reach at all is sealed and scores against the
 * defenders; a near corner scores for them.
 */
function cornerPressure(board, k) {
  const dist = new Int8Array(N).fill(-1);
  const q = [k];
  dist[k] = 0;
  board[k] = EMPTY;
  for (let head = 0; head < q.length; head++) {
    const s = q[head];
    const ds = dist[s];
    if (ds >= 6) continue;
    for (let d = 0; d < 4; d++) {
      const ray = RAYS[s][d];
      for (let i = 0; i < ray.length; i++) {
        const t = ray[i];
        if (board[t] !== EMPTY) break;
        if (dist[t] < 0) {
          dist[t] = ds + 1;
          if (!isCorner(t)) q.push(t);
        }
      }
    }
  }
  board[k] = KING;
  let total = 0;
  for (const c of CORNER_SQ) total += dist[c] < 0 ? -200 : PRESSURE[dist[c]];
  return total;
}const SEALS = [
  [[0, 2], [1, 1], [2, 0]],
  [[0, 8], [1, 9], [2, 10]],
  [[8, 0], [9, 1], [10, 2]],
  [[8, 10], [9, 9], [10, 8]],
].map((cells) => cells.map(([r, c]) => sq(r, c)));

/** Static score from the defenders' point of view (positive = good for DEF). */
export function evaluate(board, sideToMove, ply) {
  let att = 0, def = 0, k = -1;
  for (let s = 0; s < N; s++) {
    const p = board[s];
    if (p === ATT) att++;
    else if (p === DEF) def++;
    else if (p === KING) k = s;
  }
  if (k < 0) return -WIN + ply;
  let open = 0, mob = 0;
  for (let d = 0; d < 4; d++) {
    const ray = RAYS[k][d];
    for (let i = 0; i < ray.length; i++) {
      const t = ray[i];
      if (board[t] !== EMPTY) break;
      mob++;
      if (isCorner(t)) open++;
    }
  }
  if (sideToMove === DEF && open > 0) return WIN - ply - 1;
  // Fork squares: king destinations from which two corners would be open at once. The
  // defenders win if they reach one and the attackers cannot capture the king first.
  board[k] = EMPTY;
  let forks = 0, singles = 0;
  for (let d = 0; d < 4; d++) {
    const ray = RAYS[k][d];
    for (let i = 0; i < ray.length; i++) {
      const t = ray[i];
      if (board[t] !== EMPTY) break;
      if (isCorner(t)) continue;
      const lines = openLines(board, t);
      if (lines >= 2) forks++;
      else if (lines === 1) singles++;
    }
  }
  board[k] = KING;
  let score = 24 * def - 12 * att;
  score += open >= 2 ? 5000 : open === 1 ? 600 : 0;
  score += (sideToMove === DEF ? forks * 3000 : forks * 1200) + singles * 40;
  const r2 = reachTwo(board, k);
  score += mob * 2 + r2.n * 3 + r2.forked * 100;
  score += cornerPressure(board, k);
  const kr = rowOf(k), kc = colOf(k);
  const dist = Math.min(kr, 10 - kr) + Math.min(kc, 10 - kc);
  score += (20 - dist) * 5;
  let adj = 0;
  for (let d = 0; d < 4; d++) {
    const n = NB[k][d];
    if (n >= 0 && board[n] === ATT) adj++;
  }
  score -= adj * adj * 20;
  for (const c of [0, 10, 110, 120]) {
    for (let d = 0; d < 4; d++) {
      const n = NB[c][d];
      if (n >= 0 && board[n] === ATT) score -= 18;
    }
  }
  // Corner seals: attackers on the diagonal (0,2) (1,1) (2,0) close a corner for good.
  for (const seal of SEALS) {
    let held = 0;
    for (const t of seal) if (board[t] === ATT) held++;
    score -= held === 3 ? 900 : held * 120;
  }
  return score;
}

function children(board, side) {
  const kids = [];
  for (const m of allMoves(board, side)) {
    const from = moveFrom(m), to = moveTo(m);
    const r = applyMove(board, from, to);
    const key = r.kingEscaped || r.kingCaptured ? 1000 : r.captured.length * 10 + (board[from] === KING ? 1 : 0);
    kids.push({ from, to, r, key });
  }
  kids.sort((a, b) => b.key - a.key);
  return kids;
}

function value(kid, side, depth, alpha, beta, ply, ctx) {
  if (kid.r.kingEscaped) return WIN - ply;
  if (kid.r.kingCaptured) return -WIN + ply;
  return search(kid.r.board, side === DEF ? ATT : DEF, depth - 1, alpha, beta, ply + 1, ctx);
}

function search(board, side, depth, alpha, beta, ply, ctx) {
  ctx.nodes++;
  if (depth === 0) return evaluate(board, side, ply);
  const kids = children(board, side);
  if (kids.length === 0) return side === DEF ? -WIN + ply : WIN - ply; // no move: the side to move loses
  if (side === DEF) {
    let best = -Infinity;
    for (const kid of kids) {
      const s = value(kid, side, depth, alpha, beta, ply, ctx);
      if (s > best) best = s;
      if (best > alpha) alpha = best;
      if (alpha >= beta) break;
    }
    return best;
  }
  let best = Infinity;
  for (const kid of kids) {
    const s = value(kid, side, depth, alpha, beta, ply, ctx);
    if (s < best) best = s;
    if (best < beta) beta = best;
    if (alpha >= beta) break;
  }
  return best;
}

/**
 * Pick a move for `side`. opts: depth (plies, default 2), seed (tie-breaks),
 * randomChance (0..1: play a random legal move instead, for the easy level), seen\n * (Game.seen, to avoid repeating positions).
 * Returns { from, to, score, nodes } or null when the side has no move.
 */
export function bestMove(board, side, opts = {}) {
  const depth = opts.depth ?? 2;
  const rand = makeRng(opts.seed ?? 1);
  const kids = children(board, side);
  if (kids.length === 0) return null;
  if (opts.randomChance && rand() < opts.randomChance) {
    const k = kids[Math.floor(rand() * kids.length)];
    return { from: k.from, to: k.to, score: 0, nodes: 0 };
  }
  const ctx = { nodes: 0 };
  // Prefer moves that do not walk back into a position already played (opts.seen is
  // Game.seen); only when every move repeats do the repeating ones compete.
  const nextSide = side === ATT ? DEF : ATT;
  const fresh = opts.seen
    ? kids.filter((kid) => !opts.seen.get(kid.r.board.join('') + nextSide))
    : kids;
  const field = fresh.length ? fresh : kids;
  let bestScore = side === DEF ? -Infinity : Infinity;
  let pool = [];
  for (const kid of field) {
    const s = value(kid, side, depth, -Infinity, Infinity, 0, ctx);
    const better = side === DEF ? s > bestScore : s < bestScore;
    if (better) { bestScore = s; pool = [kid]; } else if (s === bestScore) pool.push(kid);
  }
  const pick = pool[Math.floor(rand() * pool.length)];
  return { from: pick.from, to: pick.to, score: bestScore, nodes: ctx.nodes };
}

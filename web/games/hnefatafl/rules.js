// Hnefatafl, Copenhagen rules on 11x11. Pure rules: no DOM, no three.js, so the
// same module runs in the browser, in the AI search and under `bun test`.
//
// Implemented (Copenhagen Hnefatafl):
//  - 11x11 board, 24 attackers, 12 defenders and a king on the throne; attackers move first.
//  - Every piece moves like a rook: any distance in a line, no jumping.
//  - Only the king may stop on the throne or on a corner. Any piece may pass over the
//    EMPTY throne.
//  - Custodian capture: a move that closes a line of enemy, victim, own piece (or a
//    hostile square) captures the victim. Moving between two enemies is safe.
//    Hostile squares: the corners always, the throne while it is empty. The king is
//    armed (it can capture and it can be the far side of a capture).
//  - The king is not taken by a sandwich: attackers need all four sides, or the three
//    free sides when the king stands next to the empty throne. On the board edge the
//    king cannot be captured.
//  - The defenders win when the king reaches a corner; the attackers win by capturing
//    the king. A side with no legal move loses.
//  - Shieldwall: two or more enemy pieces in a row along an edge are captured together
//    when both ends of the row are hostile (a piece of the mover's side or a corner), every
//    piece of the row has a piece of the mover's side directly in front of it (towards the
//    centre), and the move just played completed that. The king is never taken this way.
//  - Edge fort (defenders win): the king touches an edge, can move, and no defender can
//    ever be captured (see isEdgeFort for the exact, conservative test).
//  - Encirclement (attackers win): an unbroken chain of attackers (diagonal links count)
//    shuts the king and every defender inside, away from the board edge.
//  - Threefold repetition of a position (same side to move) is declared a draw.
//
// Still approximate: the edge-fort test is sufficient but not necessary (it can miss an
// exotic fort, it never declares a false one); a position is judged on the board alone.

export const SIZE = 11;
export const N = SIZE * SIZE;
export const EMPTY = 0;
export const ATT = 1;
export const DEF = 2;
export const KING = 3;
export const THRONE = 60;
export const CORNERS = Object.freeze([0, 10, 110, 120]);

export const sq = (r, c) => r * SIZE + c;
export const rowOf = (s) => Math.floor(s / SIZE);
export const colOf = (s) => s % SIZE;
export const isCorner = (s) => s === 0 || s === 10 || s === 110 || s === 120;
/** Side a piece belongs to: ATT, DEF (king included) or EMPTY. */
export const sideOf = (p) => (p === ATT ? ATT : p === DEF || p === KING ? DEF : EMPTY);
export const enemy = (side) => (side === ATT ? DEF : ATT);

const DR = [-1, 1, 0, 0];
const DC = [0, 0, -1, 1];
/** NB[s][d]: the neighbour of s in direction d (up, down, left, right), or -1 off the board. */
export const NB = [];
/** RAYS[s][d]: every square from s outwards in direction d. */
export const RAYS = [];
for (let s = 0; s < N; s++) {
  const r = rowOf(s), c = colOf(s);
  NB.push([]);
  RAYS.push([]);
  for (let d = 0; d < 4; d++) {
    const nr = r + DR[d], nc = c + DC[d];
    NB[s].push(nr >= 0 && nr < SIZE && nc >= 0 && nc < SIZE ? sq(nr, nc) : -1);
    const ray = [];
    for (let k = 1; k < SIZE; k++) {
      const rr = r + DR[d] * k, cc = c + DC[d] * k;
      if (rr < 0 || rr >= SIZE || cc < 0 || cc >= SIZE) break;
      ray.push(sq(rr, cc));
    }
    RAYS[s].push(ray);
  }
}

/** The standard Copenhagen set-up. */
export function initialBoard() {
  const b = new Int8Array(N);
  const att = [];
  for (let i = 3; i <= 7; i++) att.push([0, i], [10, i], [i, 0], [i, 10]);
  att.push([1, 5], [9, 5], [5, 1], [5, 9]);
  for (const [r, c] of att) b[sq(r, c)] = ATT;
  const def = [[3, 5], [4, 4], [4, 5], [4, 6], [5, 3], [5, 4], [5, 6], [5, 7], [6, 4], [6, 5], [6, 6], [7, 5]];
  for (const [r, c] of def) b[sq(r, c)] = DEF;
  b[THRONE] = KING;
  return b;
}

/** Build a board from 11 strings: . empty, a attacker, d defender, K king. */
export function boardFrom(rows) {
  if (rows.length !== SIZE) throw new Error(`need ${SIZE} rows`);
  const b = new Int8Array(N);
  rows.forEach((row, r) => {
    if (row.length !== SIZE) throw new Error(`row ${r} needs ${SIZE} columns`);
    for (let c = 0; c < SIZE; c++) {
      const ch = row[c];
      b[sq(r, c)] = ch === 'a' ? ATT : ch === 'd' ? DEF : ch === 'K' ? KING : EMPTY;
    }
  });
  return b;
}

/** Moves are packed as (from << 7) | to. */
export const moveFrom = (m) => m >> 7;
export const moveTo = (m) => m & 127;
export const packMove = (from, to) => (from << 7) | to;

/** Append the legal moves of the piece on `from` to `out`. */
export function movesFrom(board, from, out = []) {
  const p = board[from];
  if (p === EMPTY) return out;
  for (let d = 0; d < 4; d++) {
    const ray = RAYS[from][d];
    for (let i = 0; i < ray.length; i++) {
      const t = ray[i];
      if (board[t] !== EMPTY) break;
      if (t === THRONE && p !== KING) continue; // the empty throne can be crossed, not entered
      if (isCorner(t) && p !== KING) break; // corners end the line anyway
      out.push(packMove(from, t));
    }
  }
  return out;
}

export function allMoves(board, side) {
  const out = [];
  for (let s = 0; s < N; s++) if (sideOf(board[s]) === side) movesFrom(board, s, out);
  return out;
}

/** Attackers (or the empty throne) on every side of the king at k. */
function kingSurrounded(board, k) {
  for (let d = 0; d < 4; d++) {
    const n = NB[k][d];
    if (n < 0) return false; // the edge never closes the king in
    if (board[n] === ATT) continue;
    if (n === THRONE && board[n] === EMPTY) continue;
    return false;
  }
  return true;
}
export const isKingCaptured = kingSurrounded;

const onEdge = (s) => rowOf(s) === 0 || rowOf(s) === SIZE - 1 || colOf(s) === 0 || colOf(s) === SIZE - 1;

/** The four edges: their squares in order, and the square directly in front of one (inward). */
const EDGES = (() => {
  const top = [], bottom = [], left = [], right = [];
  for (let i = 0; i < SIZE; i++) {
    top.push(sq(0, i));
    bottom.push(sq(SIZE - 1, i));
    left.push(sq(i, 0));
    right.push(sq(i, SIZE - 1));
  }
  return [
    { cells: top, front: (s) => s + SIZE },
    { cells: bottom, front: (s) => s - SIZE },
    { cells: left, front: (s) => s + 1 },
    { cells: right, front: (s) => s - 1 },
  ];
})();

/**
 * Shieldwall captures completed by a move of `mover` to `to`. Mutates `nb` (removes the
 * captured pieces) and returns their squares.
 */
function shieldwall(nb, to, mover) {
  const victim = mover === ATT ? DEF : ATT; // the king (KING) is never in a shieldwall
  const taken = [];
  for (const edge of EDGES) {
    const { cells } = edge;
    let i = 0;
    while (i < SIZE) {
      if (nb[cells[i]] !== victim) { i++; continue; }
      let j = i;
      while (j + 1 < SIZE && nb[cells[j + 1]] === victim) j++;
      if (j > i && i > 0 && j < SIZE - 1) {
        const left = cells[i - 1], right = cells[j + 1];
        const hostile = (s) => sideOf(nb[s]) === mover || isCorner(s);
        let ok = hostile(left) && hostile(right);
        let touched = to === left || to === right;
        for (let k = i; ok && k <= j; k++) {
          const f = edge.front(cells[k]);
          if (sideOf(nb[f]) !== mover) ok = false;
          if (f === to) touched = true;
        }
        if (ok && touched) {
          for (let k = i; k <= j; k++) { taken.push(cells[k]); nb[cells[k]] = EMPTY; }
        }
      }
      i = j + 1;
    }
  }
  return taken;
}

/**
 * Have the attackers shut the king and every defender inside an unbroken chain of
 * attackers (diagonal links count)? Flood from the king over every square that is not an
 * attacker; if the flood reaches the board edge, or misses a defender, they have not.
 */
export function isEncircled(board) {
  const k = findKing(board);
  if (k < 0) return false;
  const seen = new Uint8Array(N);
  const stack = [k];
  seen[k] = 1;
  let defenders = 0;
  while (stack.length) {
    const s = stack.pop();
    if (onEdge(s)) return false;
    if (board[s] === DEF) defenders++;
    for (let d = 0; d < 4; d++) {
      const n = NB[s][d];
      if (n >= 0 && !seen[n] && board[n] !== ATT) { seen[n] = 1; stack.push(n); }
    }
  }
  return defenders === countPieces(board).def;
}

/**
 * The edge-fort win for the defenders: the king touches an edge, has a legal move, and no
 * defender can ever be captured.
 *
 * "Can never be captured" is decided conservatively. Attackers can only use squares they
 * can reach (empty squares connected to an attacker through empty squares). A defender is
 * safe on an axis when one side is off the board, holds another safe defender, or is a
 * square the attackers can never reach (and is not a hostile corner or empty throne).
 * Safe-ness is the greatest fixed point of that rule, and every defender must be in it.
 * A row of two or more defenders on an edge must also resist a shieldwall: one end or one
 * front square must be something the attackers can never occupy. A fort this test cannot
 * prove is simply not declared; a declared fort is real.
 */
export function isEdgeFort(board) {
  const k = findKing(board);
  if (k < 0 || !onEdge(k) || movesFrom(board, k).length === 0) return false;
  const free = (s) => (board[s] === EMPTY || s === k) && !isCorner(s);
  const reach = new Uint8Array(N);
  const stack = [];
  for (let s = 0; s < N; s++) {
    if (board[s] !== ATT) continue;
    for (let d = 0; d < 4; d++) {
      const n = NB[s][d];
      if (n >= 0 && free(n) && !reach[n]) { reach[n] = 1; stack.push(n); }
    }
  }
  while (stack.length) {
    const s = stack.pop();
    for (let d = 0; d < 4; d++) {
      const n = NB[s][d];
      if (n >= 0 && free(n) && !reach[n]) { reach[n] = 1; stack.push(n); }
    }
  }
  const sealed = (s) => free(s) && !reach[s] && !(s === THRONE && s !== k);
  const safe = new Uint8Array(N);
  const defenders = [];
  for (let s = 0; s < N; s++) if (board[s] === DEF) { safe[s] = 1; defenders.push(s); }
  const protects = (n) => n < 0 || safe[n] === 1 || sealed(n);
  let changed = true;
  while (changed) {
    changed = false;
    for (const s of defenders) {
      if (!safe[s]) continue;
      const [up, down, left, right] = NB[s];
      if (!(protects(up) || protects(down)) || !(protects(left) || protects(right))) {
        safe[s] = 0;
        changed = true;
      }
    }
  }
  // A fort is a structure: a lone king, or a king with one escort, is not one.
  if (defenders.length < 2 || defenders.some((s) => !safe[s])) return false;
  for (const edge of EDGES) {
    const { cells } = edge;
    let i = 0;
    while (i < SIZE) {
      if (board[cells[i]] !== DEF) { i++; continue; }
      let j = i;
      while (j + 1 < SIZE && board[cells[j + 1]] === DEF) j++;
      if (j > i) {
        const guard = (s) => sealed(s) || board[s] === KING || board[s] === DEF;
        let proof = guard(cells[i - 1]) || guard(cells[j + 1]);
        for (let t = i; !proof && t <= j; t++) proof = guard(edge.front(cells[t]));
        if (!proof) return false;
      }
      i = j + 1;
    }
  }
  return true;
}

/**
 * Play from -> to on a copy of the board. Does not check legality (use movesFrom).
 * Returns the new board and what happened. `terminal` is the decided result of the
 * position, if any: { winner, reason } for escape, king capture, edge fort or encirclement.
 */
export function applyMove(board, from, to) {
  const nb = board.slice();
  const p = nb[from];
  const mover = sideOf(p);
  nb[from] = EMPTY;
  nb[to] = p;
  const captured = [];
  let kingCaptured = false;
  for (let d = 0; d < 4; d++) {
    const n = NB[to][d];
    if (n < 0) continue;
    const v = nb[n];
    if (v === EMPTY) continue;
    if (mover === ATT && v === KING) {
      if (kingSurrounded(nb, n)) kingCaptured = true;
      continue;
    }
    if (sideOf(v) === mover) continue;
    const b = NB[n][d];
    if (b < 0) continue; // the edge is not hostile
    const w = nb[b];
    const hostile = sideOf(w) === mover || isCorner(b) || (b === THRONE && w === EMPTY);
    if (hostile) {
      nb[n] = EMPTY;
      captured.push(n);
    }
  }
  for (const s of shieldwall(nb, to, mover)) captured.push(s);
  const kingEscaped = p === KING && isCorner(to);
  let terminal = null;
  if (kingEscaped) terminal = { winner: DEF, reason: 'escape' };
  else if (kingCaptured) terminal = { winner: ATT, reason: 'king-captured' };
  else if (mover === DEF && isEdgeFort(nb)) terminal = { winner: DEF, reason: 'edge-fort' };
  else if (mover === ATT && isEncircled(nb)) terminal = { winner: ATT, reason: 'encirclement' };
  return { board: nb, captured, kingCaptured, kingEscaped, mover, terminal };
}

export function findKing(board) {
  for (let s = 0; s < N; s++) if (board[s] === KING) return s;
  return -1;
}

export function countPieces(board) {
  let att = 0, def = 0, king = 0;
  for (let s = 0; s < N; s++) {
    const p = board[s];
    if (p === ATT) att++;
    else if (p === DEF) def++;
    else if (p === KING) king++;
  }
  return { att, def, king };
}

const FILES = 'abcdefghijk';
/** Square name: files a-k left to right, ranks 11 (top) to 1 (bottom). */
export const squareName = (s) => `${FILES[colOf(s)]}${SIZE - rowOf(s)}`;
export const moveText = (from, to, captured = 0) => `${squareName(from)}-${squareName(to)}${captured ? 'x'.repeat(1) + captured : ''}`;

const keyOf = (board, turn) => board.join('') + turn;

/** A game in progress: turn, result, move list, undo and repetition tracking. */
export class Game {
  constructor(board = initialBoard(), turn = ATT) {
    this.reset(board, turn);
  }

  reset(board = initialBoard(), turn = ATT) {
    this.board = Int8Array.from(board);
    this.turn = turn;
    this.ply = 0;
    this.result = null; // { winner: ATT | DEF | null, reason }
    this.moves = []; // { from, to, side, captured: [squares] }
    this.undoStack = [];
    this.seen = new Map();
    this.seen.set(keyOf(this.board, this.turn), 1);
  }

  static fromBoard(board, turn) {
    return new Game(board, turn);
  }

  legalMovesFrom(from) {
    if (this.result || sideOf(this.board[from]) !== this.turn) return [];
    return movesFrom(this.board, from).map(moveTo);
  }

  legalMoves() {
    return this.result ? [] : allMoves(this.board, this.turn);
  }

  /** Play a move. Returns { ok, error?, captured, result }. */
  play(from, to) {
    if (this.result) return { ok: false, error: 'game over' };
    if (sideOf(this.board[from]) !== this.turn) return { ok: false, error: 'not your piece' };
    if (!this.legalMovesFrom(from).includes(to)) return { ok: false, error: 'illegal move' };
    this.undoStack.push({
      board: this.board, turn: this.turn, ply: this.ply, result: this.result,
      key: keyOf(this.board, this.turn),
    });
    const r = applyMove(this.board, from, to);
    this.board = r.board;
    this.moves.push({ from, to, side: r.mover, captured: r.captured });
    this.ply++;
    this.turn = enemy(r.mover);
    if (r.terminal) this.result = r.terminal;
    else if (allMoves(this.board, this.turn).length === 0) this.result = { winner: r.mover, reason: 'no-moves' };
    else {
      const key = keyOf(this.board, this.turn);
      const n = (this.seen.get(key) || 0) + 1;
      this.seen.set(key, n);
      if (n >= 3) this.result = { winner: null, reason: 'repetition' };
    }
    return { ok: true, captured: r.captured, result: this.result };
  }

  undo() {
    const s = this.undoStack.pop();
    if (!s) return false;
    const key = keyOf(this.board, this.turn);
    const n = this.seen.get(key) || 0;
    if (n <= 1) this.seen.delete(key);
    else this.seen.set(key, n - 1);
    this.board = s.board;
    this.turn = s.turn;
    this.ply = s.ply;
    this.result = s.result;
    this.moves.pop();
    return true;
  }
}

/** Replay a move list from the start with a fresh Game; the independent check of a transcript. */
export function replay(moves) {
  const g = new Game();
  for (let i = 0; i < moves.length; i++) {
    const out = g.play(moves[i].from, moves[i].to);
    if (!out.ok) return { ok: false, failedAt: i, error: out.error, game: g };
  }
  return { ok: true, game: g };
}

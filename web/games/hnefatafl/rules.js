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
//  - Threefold repetition of a position (same side to move) is declared a draw.
//
// NOT implemented, on purpose and stated in the help panel: shieldwall captures, the
// edge-fort win, and the encirclement win.

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

/**
 * Play from -> to on a copy of the board. Does not check legality (use movesFrom).
 * Returns the new board and what happened.
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
  return { board: nb, captured, kingCaptured, kingEscaped: p === KING && isCorner(to), mover };
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
    if (r.kingEscaped) this.result = { winner: DEF, reason: 'escape' };
    else if (r.kingCaptured) this.result = { winner: ATT, reason: 'king-captured' };
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

// The rule battery: hand-built positions, one assertion per rule. Used by the
// bun tests and by the in-browser gate, so the page proves its own rules before
// the AI-vs-AI game starts. Each check returns true or a string saying what broke.
import {
  ATT, DEF, KING, EMPTY, THRONE, Game, allMoves, applyMove, boardFrom, countPieces,
  initialBoard, isCorner, moveFrom, moveTo, movesFrom, rowOf, colOf, sq,
} from './rules.js';
import { bestMove } from './ai.js';

const BLANK = '...........';
/** Eleven rows from a sparse { rowNumber: 'pattern' } map; short patterns are padded with dots. */
const rows = (spec) => {
  const out = Array(11).fill(BLANK);
  for (const [r, text] of Object.entries(spec)) out[Number(r)] = text.padEnd(11, '.');
  return out;
};
const dests = (board, r, c) => movesFrom(board, sq(r, c)).map(moveTo);
const captures = (spec, from, to) => applyMove(boardFrom(rows(spec)), from, to);

/** Independent rook-move test, written without the engine's ray tables. */
function rookLegal(board, from, to) {
  if (from === to || board[to] !== EMPTY) return false;
  const piece = board[from];
  if (piece !== KING && (to === THRONE || isCorner(to))) return false;
  const fr = rowOf(from), fc = colOf(from), tr = rowOf(to), tc = colOf(to);
  if (fr !== tr && fc !== tc) return false;
  const dr = Math.sign(tr - fr), dc = Math.sign(tc - fc);
  for (let r = fr + dr, c = fc + dc; r !== tr || c !== tc; r += dr, c += dc) {
    if (board[sq(r, c)] !== EMPTY) return false;
  }
  return true;
}

export function ruleChecks() {
  const res = [];
  const check = (name, fn) => {
    let pass = false, detail = '';
    try {
      const out = fn();
      pass = out === true;
      if (!pass) detail = String(out);
    } catch (e) {
      detail = `threw: ${e.message}`;
    }
    res.push({ name, pass, detail });
  };

  check('setup-24-12-king-centre', () => {
    const n = countPieces(initialBoard());
    const b = initialBoard();
    return n.att === 24 && n.def === 12 && n.king === 1 && b[THRONE] === KING && new Game().turn === ATT
      ? true : `counts ${JSON.stringify(n)}`;
  });
  check('setup-symmetric', () => {
    const b = initialBoard();
    for (let r = 0; r < 11; r++) {
      for (let c = 0; c < 11; c++) {
        if (b[sq(r, c)] !== b[sq(10 - r, 10 - c)] || b[sq(r, c)] !== b[sq(c, r)]) return `asymmetric at ${r},${c}`;
      }
    }
    return true;
  });
  check('rook-moves-do-not-jump', () => {
    const d = dests(initialBoard(), 0, 3);
    const ok = d.includes(sq(1, 3)) && d.includes(sq(4, 3)) && !d.includes(sq(5, 3)) && !d.includes(sq(6, 3)) && !d.includes(sq(0, 5));
    return ok ? true : `moves ${d}`;
  });
  check('only-the-king-stops-on-a-corner', () => {
    const att = dests(boardFrom(rows({ 0: '.....a.....', 3: '.....K.....' })), 0, 5);
    const king = dests(boardFrom(rows({ 0: '.....K.....' })), 0, 5);
    const def = dests(boardFrom(rows({ 0: '.....d.....', 5: '.......K...' })), 0, 5);
    if (att.includes(0) || att.includes(10) || !att.includes(sq(0, 1))) return 'attacker corner rule';
    if (!king.includes(0) || !king.includes(10)) return 'king cannot reach corners';
    if (def.includes(0) || def.includes(10) || !def.includes(sq(0, 1))) return 'defender corner rule';
    return true;
  });
  check('empty-throne-can-be-crossed-not-entered', () => {
    const def = dests(boardFrom(rows({ 5: '..d........', 9: '..........K' })), 5, 2);
    const att = dests(boardFrom(rows({ 5: '..a........', 9: '..........K' })), 5, 2);
    const king = dests(boardFrom(rows({ 5: '..K........' })), 5, 2);
    if (!def.includes(sq(5, 6)) || def.includes(THRONE)) return 'defender throne rule';
    if (!att.includes(sq(5, 6)) || att.includes(THRONE)) return 'attacker throne rule';
    return king.includes(THRONE) ? true : 'king cannot return to the throne';
  });
  check('occupied-throne-blocks-the-line', () => {
    const d = dests(boardFrom(rows({ 5: '..d..K.....' })), 5, 2);
    return d.includes(sq(5, 4)) && !d.includes(sq(5, 6)) ? true : `moves ${d}`;
  });
  check('custodian-capture-horizontal', () => {
    const r = captures({ 3: '...ad....a.', 9: '.........K.' }, sq(3, 9), sq(3, 5));
    return r.captured.length === 1 && r.captured[0] === sq(3, 4) && r.board[sq(3, 4)] === EMPTY ? true : `captured ${r.captured}`;
  });
  check('custodian-capture-vertical', () => {
    const r = captures({ 6: '..a........', 7: '..d........', 8: '.........a.', 9: '..........K' }, sq(8, 9), sq(8, 2));
    return r.captured.length === 1 && r.captured[0] === sq(7, 2) ? true : `captured ${r.captured}`;
  });
  check('moving-between-two-enemies-is-safe', () => {
    const r = captures({ 0: '....d......', 3: '...a.a.....', 9: '..........K' }, sq(0, 4), sq(3, 4));
    return r.captured.length === 0 && r.board[sq(3, 4)] === DEF ? true : 'the mover was removed or captured something';
  });
  check('corner-is-hostile-to-both-sides', () => {
    const a = captures({ 0: '.d.........', 4: '..a........', 9: '..........K' }, sq(4, 2), sq(0, 2));
    const b = captures({ 4: '..d........', 10: '.a.........', 9: '..........K' }, sq(4, 2), sq(10, 2));
    if (a.captured[0] !== sq(0, 1) || a.captured.length !== 1) return 'defender at the corner survived';
    return b.captured[0] === sq(10, 1) && b.captured.length === 1 ? true : 'attacker at the corner survived';
  });
  check('empty-throne-is-hostile-to-both-sides', () => {
    const a = captures({ 1: '...a.......', 5: '....d......', 9: '..........K' }, sq(1, 3), sq(5, 3));
    const b = captures({ 1: '...d.......', 5: '....a......', 9: '..........K' }, sq(1, 3), sq(5, 3));
    if (a.captured[0] !== sq(5, 4)) return 'defender beside the empty throne survived';
    return b.captured[0] === sq(5, 4) ? true : 'attacker beside the empty throne survived';
  });
  check('occupied-throne-is-not-hostile-to-defenders', () => {
    const r = captures({ 1: '...a.......', 5: '....dK.....' }, sq(1, 3), sq(5, 3));
    return r.captured.length === 0 ? true : 'defender captured against its own king';
  });
  check('the-edge-is-not-hostile', () => {
    const r = captures({ 0: '.....d.....', 1: '.........a.', 9: '..........K' }, sq(1, 9), sq(1, 5));
    return r.captured.length === 0 && r.board[sq(0, 5)] === DEF ? true : 'edge captured a piece';
  });
  check('king-is-armed-as-hammer', () => {
    const r = captures({ 7: '.......da.K' }, sq(7, 10), sq(7, 9));
    return r.captured.length === 1 && r.captured[0] === sq(7, 8) ? true : `captured ${r.captured}`;
  });
  check('king-is-armed-as-anvil', () => {
    const r = captures({ 7: '.......Ka..', 9: '.........d.' }, sq(9, 9), sq(7, 9));
    return r.captured.length === 1 && r.captured[0] === sq(7, 8) ? true : `captured ${r.captured}`;
  });
  check('a-mover-never-captures-its-own-pieces', () => {
    const king = captures({ 2: '.......d...', 7: '........Kd.' }, sq(2, 7), sq(7, 7));
    if (king.captured.length !== 0 || king.board[sq(7, 8)] !== KING) return 'defender mover took its own king';
    const att = captures({ 0: '.a.........', 4: '..a........', 9: '..........K' }, sq(4, 2), sq(0, 2));
    return att.captured.length === 0 && att.board[sq(0, 1)] === ATT ? true : 'attacker mover took its own piece';
  });
  check('king-is-not-taken-by-a-sandwich', () => {
    const r = captures({ 3: '..aK.......', 8: '....a......' }, sq(8, 4), sq(3, 4));
    return r.kingCaptured === false && r.captured.length === 0 ? true : 'king taken by two attackers';
  });
  check('king-taken-on-four-sides', () => {
    const r = captures({ 2: '...a.......', 3: '..aK.......', 4: '...a.......', 8: '....a......' }, sq(8, 4), sq(3, 4));
    return r.kingCaptured === true ? true : 'king survived four attackers';
  });
  check('king-taken-by-three-and-the-empty-throne', () => {
    const r = captures({ 1: '...a.......', 4: '....a......', 5: '....K......', 6: '....a......' }, sq(1, 3), sq(5, 3));
    return r.kingCaptured === true ? true : 'king beside the throne survived';
  });
  check('king-on-the-throne-needs-four', () => {
    const three = captures({ 1: '....a......', 4: '.....a.....', 5: '.....K.....', 6: '.....a.....' }, sq(1, 4), sq(5, 4));
    const four = captures({ 4: '.....a.....', 5: '....aK.....', 6: '.....a.....', 8: '......a....' }, sq(8, 6), sq(5, 6));
    if (three.kingCaptured) return 'three attackers took the king on the throne';
    return four.kingCaptured ? true : 'four attackers did not take the king on the throne';
  });
  check('king-on-the-edge-cannot-be-captured', () => {
    const r = captures({ 0: '....aK.....', 1: '.....a.....', 4: '......a....' }, sq(4, 6), sq(0, 6));
    return r.kingCaptured === false ? true : 'king captured on the edge';
  });
  check('corner-escape-wins-for-the-defenders', () => {
    const g = Game.fromBoard(boardFrom(rows({ 0: '.....K.....', 8: 'a..........' })), DEF);
    const out = g.play(sq(0, 5), sq(0, 10));
    return out.ok && g.result && g.result.winner === DEF && g.result.reason === 'escape' ? true : JSON.stringify(g.result);
  });
  check('capturing-the-king-wins-for-the-attackers', () => {
    const g = Game.fromBoard(boardFrom(rows({ 2: '...a.......', 3: '..aK.......', 4: '...a.......', 8: '....a......' })), ATT);
    const out = g.play(sq(8, 4), sq(3, 4));
    return out.ok && g.result && g.result.winner === ATT && g.result.reason === 'king-captured' ? true : JSON.stringify(g.result);
  });
  check('a-side-with-no-legal-move-loses', () => {
    const g = Game.fromBoard(boardFrom(rows({ 0: '....aKa....', 1: '.....a.....', 10: '...a.......' })), ATT);
    const out = g.play(sq(10, 3), sq(10, 4));
    return out.ok && g.result && g.result.winner === ATT && g.result.reason === 'no-moves' ? true : JSON.stringify(g.result);
  });
  check('threefold-repetition-is-a-draw', () => {
    const g = new Game();
    const cycle = [[sq(0, 3), sq(1, 3)], [sq(3, 5), sq(2, 5)], [sq(1, 3), sq(0, 3)], [sq(2, 5), sq(3, 5)]];
    for (let i = 0; i < 8; i++) {
      const [f, t] = cycle[i % 4];
      if (!g.play(f, t).ok) return `illegal move at ply ${i}`;
      if (i < 7 && g.result) return `ended early at ply ${i}`;
    }
    return g.result && g.result.winner === null && g.result.reason === 'repetition' ? true : JSON.stringify(g.result);
  });
  check('illegal-moves-are-refused', () => {
    const g = new Game();
    const bad = [g.play(sq(5, 5), sq(5, 5)), g.play(sq(3, 5), sq(2, 5)), g.play(sq(0, 3), sq(5, 3)), g.play(sq(0, 3), sq(0, 0))];
    return bad.every((o) => !o.ok) && g.ply === 0 ? true : 'an illegal move was accepted';
  });
  check('undo-restores-the-position', () => {
    const g = new Game();
    const start = g.board.join('');
    g.play(sq(0, 3), sq(1, 3));
    g.play(sq(3, 5), sq(2, 5));
    g.play(sq(1, 3), sq(0, 3));
    for (let i = 0; i < 3; i++) if (!g.undo()) return 'undo failed';
    return g.board.join('') === start && g.turn === ATT && g.ply === 0 && g.moves.length === 0 && !g.undo() ? true : 'state differs';
  });
  check('every-opening-move-agrees-with-an-independent-rook-test', () => {
    const b = initialBoard();
    const mv = allMoves(b, ATT);
    const set = new Set(mv);
    for (const m of mv) if (!rookLegal(b, moveFrom(m), moveTo(m))) return `engine allows ${moveFrom(m)}->${moveTo(m)}`;
    // and the other way: the independent test finds no move the engine missed
    for (let from = 0; from < 121; from++) {
      if (b[from] !== ATT) continue;
      for (let to = 0; to < 121; to++) {
        if (rookLegal(b, from, to) && !set.has((from << 7) | to)) return `engine misses ${from}->${to}`;
      }
    }
    const g = new Game();
    for (const m of mv) {
      if (!g.play(moveFrom(m), moveTo(m)).ok) return `play refused ${moveFrom(m)}->${moveTo(m)}`;
      g.undo();
    }
    return mv.length > 60 ? true : `only ${mv.length} opening moves`;
  });
  check('shieldwall-captures-a-row-on-the-edge', () => {
    const spec = { 0: '...addd....', 1: '....aaa....', 5: '.......a...', 9: '..........K' };
    const r = captures(spec, sq(5, 7), sq(0, 7));
    const gone = [sq(0, 4), sq(0, 5), sq(0, 6)].every((s) => r.board[s] === EMPTY);
    if (r.captured.length !== 3 || !gone) return `captured ${r.captured}`;
    const open = captures({ ...spec, 1: '....a.a....' }, sq(5, 7), sq(0, 7));
    return open.captured.length === 0 ? true : 'a row with an unguarded front square was taken';
  });
  check('shieldwall-needs-both-ends-hostile', () => {
    const r = captures({ 0: '....ddd....', 1: '....aaa....', 5: '.......a...', 9: '..........K' }, sq(5, 7), sq(0, 7));
    return r.captured.length === 0 ? true : `captured ${r.captured} with an open end`;
  });
  check('shieldwall-may-end-in-a-corner', () => {
    const r = captures({ 0: '.dd........', 1: '.aa........', 5: '...a.......', 9: '..........K' }, sq(5, 3), sq(0, 3));
    return r.captured.length === 2 && r.board[sq(0, 1)] === EMPTY && r.board[sq(0, 2)] === EMPTY ? true : `captured ${r.captured}`;
  });
  check('shieldwall-never-takes-the-king', () => {
    const r = captures({ 0: '...adKd....', 1: '....aaa....', 5: '.......a...' }, sq(5, 7), sq(0, 7));
    return !r.kingCaptured && r.board[sq(0, 5)] === KING ? true : 'the king was taken in a shieldwall';
  });
  check('encirclement-wins-for-the-attackers', () => {
    const ring = { 3: '.....a.....', 4: '....a.a....', 5: '...adK.a...', 6: '....a.a....', 9: '.....a.....' };
    const r = captures(ring, sq(9, 5), sq(7, 5));
    if (!r.terminal || r.terminal.winner !== ATT || r.terminal.reason !== 'encirclement') return JSON.stringify(r.terminal);
    const gap = captures({ ...ring, 6: '....a......' }, sq(9, 5), sq(7, 5));
    if (gap.terminal) return 'a ring with a gap was declared encircled';
    const stray = captures({ ...ring, 9: '.d...a.....' }, sq(9, 5), sq(7, 5));
    return stray.terminal ? 'a defender outside the ring was ignored' : true;
  });
  check('edge-fort-wins-for-the-defenders', () => {
    const fort = { 2: '.....a.....', 5: '.......d...', 9: '....ddd....', 10: '....dK.d...' };
    const r = captures(fort, sq(5, 7), sq(9, 7));
    if (!r.terminal || r.terminal.winner !== DEF || r.terminal.reason !== 'edge-fort') return JSON.stringify(r.terminal);
    const loose = captures({ ...fort, 9: '....d.d....' }, sq(5, 7), sq(9, 7));
    return loose.terminal ? 'a breakable wall was declared a fort' : true;
  });
  check('a-lone-king-on-the-edge-is-not-a-fort', () => {
    const lone = captures({ 0: '.....K.....', 5: 'a..........' }, sq(0, 5), sq(0, 6));
    const escort = captures({ 0: '.....K.....', 5: 'a......d...' }, sq(0, 5), sq(0, 6));
    return !lone.terminal && !escort.terminal ? true : 'a king with at most one defender was given the fort';
  });
  check('edge-fort-needs-a-king-that-can-move', () => {
    const boxed = captures({ 2: '.....a.....', 5: '.......d...', 9: '....ddd.d..', 10: '....dKd.d..' }, sq(5, 7), sq(9, 7));
    return !boxed.terminal ? true : 'a king with no move was given the fort';
  });
  check('ai-takes-the-corner-when-it-is-open', () => {
    const m = bestMove(boardFrom(rows({ 0: '.....K.....', 5: 'a..........' })), DEF, { depth: 2, seed: 1 });
    return m && isCorner(m.to) ? true : 'did not escape';
  });
  check('ai-captures-the-king-when-it-can', () => {
    const b = boardFrom(rows({ 2: '...a.......', 3: '..aK.......', 4: '...a.......', 8: '....a......' }));
    const m = bestMove(b, ATT, { depth: 2, seed: 1 });
    return m && m.from === sq(8, 4) && m.to === sq(3, 4) ? true : 'missed the capture';
  });
  check('ai-blocks-a-single-open-line', () => {
    const b = boardFrom(rows({ 0: '..a..K.....', 5: '........a..' }));
    const m = bestMove(b, ATT, { depth: 2, seed: 1 });
    if (!m) return 'no move';
    const after = applyMove(b, m.from, m.to).board;
    const open = movesFrom(after, sq(0, 5)).map(moveTo).filter(isCorner);
    return open.length === 0 ? true : `${open.length} corner(s) still open`;
  });
  return res;
}

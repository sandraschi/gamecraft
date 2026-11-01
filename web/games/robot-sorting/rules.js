// Boomy's sorting rules: parcels, a tiny rule language, levels with a goal and a
// reference answer. Pure, no three.js.
import { rng } from '../_shared/gc.js';

export const COLORS = ['red', 'blue', 'green', 'yellow'];
export const SIZES = ['small', 'big'];
export const DEST = ['left', 'right', 'pass'];
export const CONDS = [
  { id: 'any', text: 'any parcel', test: () => true },
  ...COLORS.map((c) => ({ id: `color:${c}`, text: `${c}`, test: (p) => p.color === c })),
  { id: 'size:big', text: 'big', test: (p) => p.size === 'big' },
  { id: 'size:small', text: 'small', test: (p) => p.size === 'small' },
  { id: 'star', text: 'has a star', test: (p) => p.star },
  { id: 'nostar', text: 'no star', test: (p) => !p.star },
];
export const cond = (id) => CONDS.find((c) => c.id === id);

/** A rule is { a, b, to }: condition a AND (optional) condition b, then go to `to`. First match wins; otherwise pass. */
export function route(program, p) {
  for (const r of program) {
    if (!r || !r.a) continue;
    if (cond(r.a).test(p) && (!r.b || cond(r.b).test(p))) return r.to;
  }
  return 'pass';
}

export const LEVELS = [
  { title: 'Red to the left', goal: 'Red parcels go to the left bin. Everything else rides on.', slots: 2,
    want: (p) => (p.color === 'red' ? 'left' : 'pass'),
    answer: [{ a: 'color:red', to: 'left' }] },
  { title: 'Two colours', goal: 'Red goes left, blue goes right, the rest ride on.', slots: 3,
    want: (p) => (p.color === 'red' ? 'left' : p.color === 'blue' ? 'right' : 'pass'),
    answer: [{ a: 'color:red', to: 'left' }, { a: 'color:blue', to: 'right' }] },
  { title: 'Big ones', goal: 'Big parcels go right, whatever their colour. Small yellow ones go left. Everything else rides on.', slots: 3,
    want: (p) => (p.size === 'big' ? 'right' : p.color === 'yellow' ? 'left' : 'pass'),
    answer: [{ a: 'size:big', to: 'right' }, { a: 'color:yellow', to: 'left' }],
    naive: [{ a: 'color:yellow', to: 'left' }, { a: 'size:big', to: 'right' }] },
  { title: 'Stars', goal: 'Parcels with a star go left, except big ones, which always go right. Green parcels without a star also go left.', slots: 4,
    want: (p) => (p.size === 'big' ? 'right' : p.star ? 'left' : p.color === 'green' ? 'left' : 'pass'),
    answer: [{ a: 'size:big', to: 'right' }, { a: 'star', to: 'left' }, { a: 'color:green', to: 'left' }],
    naive: [{ a: 'star', to: 'left' }, { a: 'size:big', to: 'right' }, { a: 'color:green', to: 'left' }] },
  { title: 'The busy afternoon', goal: 'Small red parcels go left. Other red ones go right. Blue parcels with a star go left. Every other big parcel goes right. The rest ride on.', slots: 4,
    want: (p) => (p.color === 'red' ? (p.size === 'small' ? 'left' : 'right') : p.color === 'blue' && p.star ? 'left' : p.size === 'big' ? 'right' : 'pass'),
    answer: [{ a: 'color:red', b: 'size:small', to: 'left' }, { a: 'color:red', to: 'right' }, { a: 'color:blue', b: 'star', to: 'left' }, { a: 'size:big', to: 'right' }],
    naive: [{ a: 'size:big', to: 'right' }, { a: 'color:red', b: 'size:small', to: 'left' }, { a: 'color:red', to: 'right' }, { a: 'color:blue', b: 'star', to: 'left' }] },
];

/** Every parcel kind there is (4 colours x 2 sizes x star or not). */
export const KINDS = COLORS.flatMap((color) => SIZES.flatMap((size) => [false, true].map((star) => ({ color, size, star }))));

/** A shift of parcels: covers every destination the level uses, then fills with random ones. */
export function shift(levelIdx, seed, n = 14) {
  const r = rng(seed);
  const L = LEVELS[levelIdx];
  const out = [];
  // make sure every distinct outcome of the goal shows up at least twice
  const byDest = {};
  for (const k of KINDS) (byDest[L.want(k)] ??= []).push(k);
  for (const d of Object.keys(byDest)) for (let i = 0; i < 2; i++) out.push({ ...r.pick(byDest[d]) });
  // and anything that would fool a wrong-order program
  if (L.naive) for (const k of KINDS) if (route(L.naive, k) !== L.want(k) && out.length < n) out.push({ ...k });
  while (out.length < n) out.push({ ...r.pick(KINDS) });
  return r.shuffle(out).map((p, i) => ({ ...p, id: i }));
}
export function score(levelIdx, program, parcels) {
  const L = LEVELS[levelIdx];
  return parcels.filter((p) => route(program, p) === L.want(p)).length;
}

export function selfTest() {
  const c = { answer_sorts_everything: true, answer_fits_the_slots: true, doing_nothing_is_not_enough: true, wrong_order_fails: true, every_outcome_appears: true, answer_right_for_every_kind: true };
  for (const [li, L] of LEVELS.entries()) {
    if (L.answer.length > L.slots) c.answer_fits_the_slots = false;
    if (KINDS.some((k) => route(L.answer, k) !== L.want(k))) c.answer_right_for_every_kind = false;
    for (let s = 1; s <= 15; s++) {
      const ps = shift(li, s * 977 + li);
      if (score(li, L.answer, ps) !== ps.length) c.answer_sorts_everything = false;
      if (score(li, [], ps) === ps.length) c.doing_nothing_is_not_enough = false;
      if (L.naive && score(li, L.naive, ps) === ps.length) c.wrong_order_fails = false;
      const dests = new Set(KINDS.map((k) => L.want(k)));
      for (const d of dests) if (ps.filter((p) => L.want(p) === d).length < 2) c.every_outcome_appears = false;
    }
  }
  return { pass: Object.values(c).every(Boolean), checks: c, summary: `${LEVELS.length} levels x 15 shifts; reference programs sort all ${KINDS.length} parcel kinds, empty and wrong-order programs fail` };
}

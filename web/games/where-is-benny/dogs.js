// Dogs for Where is Benny. Pure data first (breeds, features, who is who), then
// a builder that turns a spec into a low-poly three.js dog.
import * as THREE from 'three';

export const BREEDS = {
  gsd:       { name: 'German Shepherd', size: 1.0, len: 1.0, legs: 0.42, body: 0xb57a3c, saddle: 0x1c1712, mask: 0x1c1712, ears: 'up', tail: 'bushy', snout: 1.0 },
  malinois:  { name: 'Belgian Malinois', size: 0.95, len: 0.95, legs: 0.44, body: 0xc28a4a, saddle: null, mask: 0x1c1712, ears: 'up', tail: 'straight', snout: 1.0 },
  lab:       { name: 'Labrador', size: 0.95, len: 0.95, legs: 0.38, body: 0xe2c07a, saddle: null, mask: null, ears: 'flop', tail: 'straight', snout: 0.9 },
  blacklab:  { name: 'black Labrador', size: 0.95, len: 0.95, legs: 0.38, body: 0x1e1e20, saddle: null, mask: null, ears: 'flop', tail: 'straight', snout: 0.9 },
  beagle:    { name: 'Beagle', size: 0.7, len: 0.75, legs: 0.28, body: 0xf2ece0, saddle: 0x2a2018, mask: 0xb06a2a, ears: 'flop', tail: 'up', snout: 0.8 },
  husky:     { name: 'Husky', size: 0.95, len: 0.95, legs: 0.42, body: 0xe8e8ea, saddle: 0x6a6e78, mask: null, ears: 'up', tail: 'curl', snout: 0.9 },
  poodle:    { name: 'Poodle', size: 0.75, len: 0.7, legs: 0.42, body: 0xf6f2ea, saddle: null, mask: null, ears: 'flop', tail: 'up', snout: 0.9, fluffy: true },
  dachshund: { name: 'Dachshund', size: 0.55, len: 1.0, legs: 0.13, body: 0x8a4a22, saddle: null, mask: null, ears: 'flop', tail: 'straight', snout: 1.1 },
  corgi:     { name: 'Corgi', size: 0.6, len: 0.85, legs: 0.15, body: 0xd88a3a, saddle: null, mask: null, ears: 'up', tail: 'none', snout: 0.8, belly: 0xf6efe2 },
  dalmatian: { name: 'Dalmatian', size: 0.95, len: 0.95, legs: 0.44, body: 0xf6f4ee, saddle: null, mask: null, ears: 'flop', tail: 'straight', snout: 0.9, spots: true },
  collie:    { name: 'Border Collie', size: 0.8, len: 0.85, legs: 0.36, body: 0x18181a, saddle: null, mask: null, ears: 'tip', tail: 'bushy', snout: 0.9, belly: 0xf4f2ee },
};
export const NECKWEAR = [
  { id: 'red-bandana', name: 'a red bandana', kind: 'bandana', color: 0xd8342c },
  { id: 'blue-bandana', name: 'a blue bandana', kind: 'bandana', color: 0x2a62c8 },
  { id: 'yellow-bandana', name: 'a yellow bandana', kind: 'bandana', color: 0xf2c230 },
  { id: 'red-collar', name: 'a red collar', kind: 'collar', color: 0xd8342c },
  { id: 'green-collar', name: 'a green collar', kind: 'collar', color: 0x2aa04a },
  { id: 'purple-collar', name: 'a purple collar', kind: 'collar', color: 0x8a3ac8 },
];
export const CARRY = [
  { id: 'ball', name: 'a tennis ball' },
  { id: 'stick', name: 'a stick' },
  { id: 'frisbee', name: 'an orange frisbee' },
  { id: 'none', name: 'nothing in the mouth' },
];

export const isBenny = (d, want) => d.breed === 'gsd' && d.neck === want.neck && d.carry === want.carry;

/** Why a clicked dog is not Benny, in words. */
export function whyNot(d, want) {
  const b = BREEDS[d.breed];
  if (d.breed === 'malinois') return 'Close! But that is a Malinois: no black saddle on the back.';
  if (d.breed !== 'gsd') return `That is a ${b.name}.`;
  const neck = NECKWEAR.find((n) => n.id === d.neck), carry = CARRY.find((c) => c.id === d.carry);
  const wantNeck = NECKWEAR.find((n) => n.id === want.neck), wantCarry = CARRY.find((c) => c.id === want.carry);
  if (d.neck !== want.neck && d.carry !== want.carry) return `A shepherd, but with ${neck.name} and ${carry.name}.`;
  if (d.neck !== want.neck) return `A shepherd with ${carry.name}, but wearing ${neck.name}, not ${wantNeck.name}.`;
  return `A shepherd in ${neck.name}, but with ${carry.name}, not ${wantCarry.name}.`;
}

/** Pure cast list for a round: Benny, look-alikes that differ by one thing, and everyone else. */
export function castDogs(r, nDogs) {
  const want = { neck: r.pick(NECKWEAR).id, carry: r.pick(CARRY.filter((c) => c.id !== 'none')).id };
  const otherNeck = () => r.pick(NECKWEAR.filter((n) => n.id !== want.neck)).id;
  const otherCarry = () => r.pick(CARRY.filter((c) => c.id !== want.carry)).id;
  const dogs = [
    { breed: 'gsd', neck: want.neck, carry: want.carry, benny: true },
    { breed: 'gsd', neck: want.neck, carry: otherCarry() },
    { breed: 'gsd', neck: otherNeck(), carry: want.carry },
    { breed: 'gsd', neck: otherNeck(), carry: otherCarry() },
    { breed: 'gsd', neck: want.neck, carry: otherCarry() },
    { breed: 'malinois', neck: want.neck, carry: want.carry },
  ];
  const others = Object.keys(BREEDS).filter((b) => b !== 'gsd' && b !== 'malinois'); // one deliberate Malinois only
  while (dogs.length < nDogs) {
    const breed = r.pick(others);
    dogs.push({ breed, neck: r() < 0.7 ? r.pick(NECKWEAR).id : null, carry: r() < 0.3 ? r.pick(CARRY).id : 'none' });
  }
  return { want, dogs: r.shuffle(dogs) };
}

// ------------------------------------------------------------------ builder
const matCache = new Map();
function mat(color, rough = 0.85) {
  const k = `${color}:${rough}`;
  if (!matCache.has(k)) matCache.set(k, new THREE.MeshStandardMaterial({ color, roughness: rough }));
  return matCache.get(k);
}
const G = {
  box: new THREE.BoxGeometry(1, 1, 1),
  ball: new THREE.SphereGeometry(1, 14, 10),
  cyl: new THREE.CylinderGeometry(1, 1, 1, 10),
  cone: new THREE.ConeGeometry(1, 1, 8),
  capsule: new THREE.CapsuleGeometry(1, 1, 4, 10),
  torus: new THREE.TorusGeometry(1, 0.14, 6, 18),
  disc: new THREE.CylinderGeometry(1, 1, 0.12, 16),
};
function part(geo, color, sx, sy, sz, x, y, z, rough) {
  const m = new THREE.Mesh(geo, mat(color, rough));
  m.scale.set(sx, sy, sz); m.position.set(x, y, z);
  m.castShadow = true;
  return m;
}

/** Builds a dog facing +x, standing on y=0. Returns the group; legs in userData for trotting. */
export function buildDog(spec) {
  const b = BREEDS[spec.breed];
  const g = new THREE.Group();
  const s = b.size * 0.62, L = b.len * 0.62, leg = b.legs * 0.8;
  const bodyY = leg + s * 0.3;
  const fluff = b.fluffy ? 1.25 : 1;
  // body: a capsule lying along x
  const body = part(G.capsule, b.body, s * 0.3 * fluff, L * 0.4, s * 0.28 * fluff, 0, bodyY, 0);
  body.rotation.z = Math.PI / 2; g.add(body);
  if (b.belly) { const bl = part(G.capsule, b.belly, s * 0.25, L * 0.3, s * 0.26, 0.02, bodyY - s * 0.09, 0); bl.rotation.z = Math.PI / 2; g.add(bl); }
  if (b.saddle) { const sd = part(G.capsule, b.saddle, s * 0.27, L * 0.3, s * 0.295, -L * 0.04, bodyY + s * 0.07, 0); sd.rotation.z = Math.PI / 2; g.add(sd); }
  if (b.spots) for (let i = 0; i < 9; i++) g.add(part(G.ball, 0x1a1a1a, 0.04, 0.04, 0.04, -L * 0.3 + (i % 3) * L * 0.28, bodyY + 0.05 + ((i * 7) % 3) * 0.03, (i % 2 ? 1 : -1) * s * 0.26));
  // neck and head
  const hx = L * 0.5, hy = bodyY + s * 0.3;
  const neck = part(G.capsule, b.body, s * 0.13, s * 0.22, s * 0.13, hx - 0.05, bodyY + s * 0.15, 0); neck.rotation.z = -0.7; g.add(neck);
  g.add(part(G.ball, b.body, s * 0.2 * fluff, s * 0.19 * fluff, s * 0.17 * fluff, hx + 0.02, hy, 0));
  const snoutCol = b.mask ?? b.body;
  g.add(part(G.box, snoutCol, s * 0.24 * b.snout, s * 0.13, s * 0.14, hx + s * 0.2, hy - s * 0.05, 0));
  g.add(part(G.ball, 0x101010, 0.028, 0.025, 0.03, hx + s * 0.2 + s * 0.12 * b.snout, hy - s * 0.01, 0));
  for (const z of [-1, 1]) g.add(part(G.ball, 0x101010, 0.018, 0.018, 0.018, hx + s * 0.12, hy + s * 0.05, z * s * 0.1));
  // ears
  const earCol = b.saddle && spec.breed !== 'beagle' ? b.saddle : (b.mask ?? (spec.breed === 'beagle' ? 0x8a4a22 : b.body));
  for (const z of [-1, 1]) {
    if (b.ears === 'up' || b.ears === 'tip') {
      const e = part(G.cone, earCol, s * 0.07, s * 0.2, s * 0.05, hx - s * 0.02, hy + s * 0.2, z * s * 0.09);
      e.rotation.x = z * 0.25; if (b.ears === 'tip') { e.rotation.z = 0.7; e.position.x += 0.02; }
      g.add(e);
    } else {
      const e = part(G.box, earCol, s * 0.06, s * 0.22, s * 0.04, hx - s * 0.02, hy - s * 0.04, z * s * 0.19);
      e.rotation.x = z * 0.2; g.add(e);
    }
  }
  // legs (pivot at the hip so they can swing)
  const legs = [];
  for (const [lx, lz] of [[L * 0.36, 1], [L * 0.36, -1], [-L * 0.36, 1], [-L * 0.36, -1]]) {
    const pivot = new THREE.Group(); pivot.position.set(lx, bodyY, lz * s * 0.12);
    pivot.add(part(G.cyl, b.body === 0xf6f4ee || spec.breed === 'beagle' ? 0xf2ece0 : b.body, s * 0.075 * fluff, bodyY, s * 0.075 * fluff, 0, -bodyY / 2, 0));
    g.add(pivot); legs.push(pivot);
  }
  // tail
  const tail = new THREE.Group(); tail.position.set(-L * 0.5, bodyY + s * 0.08, 0); g.add(tail);
  const tc = b.saddle && spec.breed === 'gsd' ? 0x2a2018 : b.body;
  if (b.tail === 'bushy') { const t = part(G.capsule, tc, s * 0.07, s * 0.35, s * 0.07, -s * 0.12, -s * 0.15, 0); t.rotation.z = -0.6; tail.add(t); }
  else if (b.tail === 'straight') { const t = part(G.capsule, tc, s * 0.04, s * 0.32, s * 0.04, -s * 0.15, -s * 0.06, 0); t.rotation.z = -1.1; tail.add(t); }
  else if (b.tail === 'up') { const t = part(G.capsule, tc, s * 0.04, s * 0.25, s * 0.04, -s * 0.05, s * 0.15, 0); t.rotation.z = 0.4; tail.add(t); if (b.fluffy) tail.add(part(G.ball, tc, 0.06, 0.06, 0.06, -0.05, 0.32 * s, 0)); }
  else if (b.tail === 'curl') { const t = new THREE.Mesh(G.torus, mat(tc)); t.scale.set(s * 0.12, s * 0.12, s * 0.3); t.position.set(-0.02, s * 0.12, 0); tail.add(t); }
  // neckwear
  const nw = NECKWEAR.find((n) => n.id === spec.neck);
  if (nw) {
    const nx = hx - 0.07, ny = bodyY + s * 0.22;
    if (nw.kind === 'collar') {
      const c = new THREE.Mesh(G.torus, mat(nw.color, 0.5)); c.scale.set(s * 0.16, s * 0.16, s * 1.1);
      c.position.set(nx, ny, 0); c.rotation.set(0, Math.PI / 2, 0.7); c.castShadow = true; g.add(c);
      g.add(part(G.disc, 0xe8c040, 0.03, 0.03, 0.03, nx + 0.06, ny - s * 0.16, 0));
    } else {
      // a triangle hanging in front of the chest, big enough to read from a distance
      const c = part(G.cone, nw.color, s * 0.24, s * 0.34, s * 0.2, nx + s * 0.12, ny - s * 0.16, 0);
      c.rotation.z = Math.PI + 0.25; g.add(c);
      const band = new THREE.Mesh(G.torus, mat(nw.color)); band.scale.set(s * 0.16, s * 0.16, s * 1.2); band.position.set(nx, ny, 0); band.rotation.set(0, Math.PI / 2, 0.7); g.add(band);
    }
  }
  // what it carries
  const mx = hx + s * 0.28, my = hy - s * 0.1;
  if (spec.carry === 'ball') g.add(part(G.ball, 0xd8f040, 0.065, 0.065, 0.065, mx + 0.04, my, 0, 0.6));
  else if (spec.carry === 'stick') { const st = part(G.cyl, 0x7a5228, 0.022, 0.55, 0.022, mx, my, 0); st.rotation.x = Math.PI / 2; g.add(st); }
  else if (spec.carry === 'frisbee') { const f = part(G.disc, 0xf2782a, 0.13, 0.6, 0.13, mx + 0.06, my, 0, 0.5); f.rotation.z = Math.PI / 2; g.add(f); }

  g.userData = { legs, tail, spec, breedName: b.name, height: hy + s * 0.25 };
  return g;
}

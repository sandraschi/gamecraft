// Mossbrook world: layout data, terrain, buildings, interiors, cellars, tunnel,
// cave, and the collision spaces the movement code walks in.
// Everything is procedural and seeded. No external assets.

import * as THREE from 'three';

// ---------------------------------------------------------------- constants
export const FLOOR_Y = 0.15;
export const WALL_H = 3.2;
export const UPPER_Y = 3.35;
export const UPPER_H = 2.8;
export const BELOW_Y = -3.0;
export const WALL_T = 0.3;
export const DOOR_W = 1.8;
export const DOOR_H = 2.3;
export const WORLD_HALF = 196;

// ---------------------------------------------------------------- math
export const clamp = (v, a, b) => Math.min(b, Math.max(a, v));
export const lerp = (a, b, t) => a + (b - a) * t;
export function smoothstep(e0, e1, x) {
  const t = clamp((x - e0) / (e1 - e0), 0, 1);
  return t * t * (3 - 2 * t);
}
export function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
function hash2(i, j) {
  let h = (Math.imul(i, 374761393) + Math.imul(j, 668265263)) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}
export function vnoise(x, z) {
  const i = Math.floor(x), j = Math.floor(z);
  const fx = x - i, fz = z - j;
  const u = fx * fx * (3 - 2 * fx), v = fz * fz * (3 - 2 * fz);
  const a = hash2(i, j), b = hash2(i + 1, j), c = hash2(i, j + 1), d = hash2(i + 1, j + 1);
  return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
}
export const inRect = (r, x, z) => x >= r[0] && x <= r[1] && z >= r[2] && z <= r[3];
function distToSeg(px, pz, a, b) {
  const vx = b[0] - a[0], vz = b[1] - a[1];
  const t = clamp(((px - a[0]) * vx + (pz - a[1]) * vz) / (vx * vx + vz * vz), 0, 1);
  return Math.hypot(px - (a[0] + vx * t), pz - (a[1] + vz * t));
}

// ---------------------------------------------------------------- layout
// Doors are all on the south wall (+z), the side the camera looks from.
export const BUILDINGS = [
  { id: 'tavern', name: 'The Copper Kettle', kind: 'Tavern and rooms', x: 0, z: -16, w: 16, d: 12, floors: 2, wallH: WALL_H, wall: 0xdccdaa, timber: 0x5a3b24, roof: 0x8c3a28, chimney: [5.5, -19] },
  { id: 'smithy', name: "Bertil's Smithy", kind: 'Forge', x: 22, z: 4, w: 12, d: 10, floors: 1, wallH: WALL_H, wall: 0x9b948a, timber: 0x3a2c20, roof: 0x4b5260, chimney: [17.5, 0.75] },
  { id: 'weaver', name: 'Weaver Cottage', kind: 'Home of Ada Thorne', x: -20, z: 6, w: 10, d: 8, floors: 2, wallH: WALL_H, wall: 0xe8dec6, timber: 0x6b4a2e, roof: 0x6f5b3b },
  { id: 'bakery', name: "Pim's Bakery", kind: 'Bakehouse', x: -6, z: 22, w: 9, d: 8, floors: 1, wallH: WALL_H, wall: 0xf0e2c0, timber: 0x7a5232, roof: 0xa0522d, chimney: [-9.6, 19.4] },
  { id: 'archive', name: 'The Lantern Archive', kind: 'Library', x: 24, z: -22, w: 10, d: 14, floors: 1, wallH: 4.4, wall: 0xc9c2b8, timber: 0x403530, roof: 0x3d4a5c },
  { id: 'mill', name: 'Ferrow Mill House', kind: 'Grain store', x: 10, z: 26, w: 10, d: 8, floors: 1, wallH: WALL_H, wall: 0xd9c7a0, timber: 0x5c4630, roof: 0x5e6b3f },
];
for (const b of BUILDINGS) {
  b.x0 = b.x - b.w / 2; b.x1 = b.x + b.w / 2; b.z0 = b.z - b.d / 2; b.z1 = b.z + b.d / 2;
  b.doorX = b.x;
  b.inner = [b.x0 + WALL_T / 2, b.x1 - WALL_T / 2, b.z0 + WALL_T / 2, b.z1 - WALL_T / 2];
}

// rect = [x0, x1, z0, z1]. t = 0 at lowAt, 1 at highAt along axis.
// rails: colliders (and their visuals) that stop you walking into the stairwell sideways.
export const STAIRS = [
  {
    id: 'tavern-up', building: 'tavern', rect: [-7.7, -6.2, -20, -14], axis: 'z', lowAt: -20, highAt: -14,
    low: 'outdoor', high: 'upper:tavern', yLow: FLOOR_Y, yHigh: UPPER_Y,
    rails: [
      { space: 'outdoor', r: [-6.2, -6.0, -19.5, -14], h: UPPER_Y - FLOOR_Y },
      { space: 'outdoor', r: [-7.7, -6.0, -14, -13.8], h: UPPER_Y - FLOOR_Y },
      { space: 'upper:tavern', r: [-6.2, -6.0, -20, -14.6], h: 1 },
      { space: 'upper:tavern', r: [-7.7, -6.0, -20.2, -20], h: 1 },
    ],
  },
  {
    id: 'tavern-cellar', building: 'tavern', rect: [6.1, 7.6, -21.6, -17.6], axis: 'z', lowAt: -21.6, highAt: -17.6,
    low: 'below', high: 'outdoor', yLow: BELOW_Y, yHigh: FLOOR_Y,
    rails: [
      { space: 'outdoor', r: [5.9, 6.1, -21.6, -17.9], h: 1 },
      { space: 'below', r: [5.9, 6.1, -21.4, -17.6], h: 3.15 },
      { space: 'below', r: [5.9, 7.8, -17.6, -17.4], h: 3.15 },
    ],
  },
  {
    id: 'smithy-cellar', building: 'smithy', rect: [21, 25, -0.6, 1.0], axis: 'x', lowAt: 21, highAt: 25,
    low: 'below', high: 'outdoor', yLow: BELOW_Y, yHigh: FLOOR_Y,
    rails: [
      { space: 'outdoor', r: [20.8, 25, 1.0, 1.2], h: 1 },
      { space: 'outdoor', r: [20.8, 21, -0.6, 1.0], h: 1 },
      { space: 'below', r: [21, 25.2, 1.0, 1.2], h: 3.15 },
      { space: 'below', r: [25, 25.2, -0.6, 1.0], h: 3.15 },
    ],
  },
  {
    id: 'weaver-up', building: 'weaver', rect: [-23.5, -17.5, 2.3, 3.8], axis: 'x', lowAt: -23.5, highAt: -17.5,
    low: 'outdoor', high: 'upper:weaver', yLow: FLOOR_Y, yHigh: UPPER_Y,
    rails: [
      { space: 'outdoor', r: [-23.5, -17.3, 3.8, 4.0], h: UPPER_Y - FLOOR_Y },
      { space: 'outdoor', r: [-17.5, -17.3, 2.3, 3.8], h: UPPER_Y - FLOOR_Y },
      { space: 'upper:weaver', r: [-23.7, -18.0, 3.8, 4.0], h: 1 },
      { space: 'upper:weaver', r: [-23.7, -23.5, 2.3, 3.8], h: 1 },
    ],
  },
];
export function stairT(s, x, z) {
  return clamp(((s.axis === 'x' ? x : z) - s.lowAt) / (s.highAt - s.lowAt), 0, 1);
}

export const BELOW_ROOMS = [
  { id: 'cellar-kettle', name: 'Cellar of the Copper Kettle', r: [-6, 7.8, -24, -16] },
  { id: 'tunnel-a', name: "Smugglers' tunnel", r: [1, 3.5, -16.2, 4.5] },
  { id: 'tunnel-b', name: "Smugglers' tunnel", r: [1, 17.2, 2, 4.5] },
  { id: 'cellar-forge', name: 'Forge cellar', r: [17, 27, -0.6, 8] },
  { id: 'keepers-cellar', name: "The Keeper's Cellar", r: [4.4, 9.5, -5, 0.8] },
  { id: 'keepers-door', name: "Smugglers' tunnel", r: [5, 6.6, 0.8, 2.05] },
];
export const CAVE_ROOMS = [
  { id: 'cave-mouth', name: 'Knucklehollow, the mouth', r: [-57, -46, -7.5, -4.5] },
  { id: 'cave-hall', name: 'Knucklehollow Cave', r: [-72, -56.5, -17, 4] },
  { id: 'cave-passage', name: 'Root passage', r: [-66, -63, 4, 10] },
  { id: 'cave-hollow', name: "The Keeper's Hollow", r: [-71, -58, 10, 18] },
];
export const MOUNTAIN = { x: -80, z: -6, r: 32, h: 24, block: 30 };
// Hysteresis: enter below enterX, leave above exitX, so the boundary never flickers.
export const CAVE = { enterX: -48.6, exitX: -47.8, z0: -7.4, z1: -4.6, troll: [-67, -11] };
export const PLAZA = { x: 4, z: 0 };

export const ROADS = [
  [[4, 0], [0, -9.2]],
  [[4, 0], [14, 12]], [[14, 12], [22, 12]], [[22, 12], [22, 9.4]],
  [[4, 0], [-12, 12.5]], [[-12, 12.5], [-20, 12]], [[-20, 12], [-20, 10.4]],
  [[4, 0], [2, 16]], [[2, 16], [0, 28.5]], [[0, 28.5], [-6, 28.5]], [[-6, 28.5], [-6, 26.4]],
  [[0, 28.5], [10, 31]], [[10, 31], [10, 30.4]],
  [[4, 0], [24, -14.6]], [[24, -14.6], [70, -10]],
  [[4.5, 3.5], [-48.8, -6]], [[14, 12], [4.5, 3.5]],
  [[70, -10], [120, -4]], [[-6, 28.5], [-17, 38]], [[-17, 38], [-8, 62]], [[-8, 62], [-8, 68]],
  [[64, -10.6], [70, -16]], [[70, -16], [72, -21]],
];
export const riverX = (z) => 62 + 5 * Math.sin(z * 0.035);
export const MOONRING = { x: -8, z: 72, r: 7 };
export const HUT = { x: 76, z: -25, w: 6, d: 5 };
export function roadDist(x, z) {
  let d = 1e9;
  for (const [a, b] of ROADS) d = Math.min(d, distToSeg(x, z, a, b));
  return d;
}

// ---------------------------------------------------------------- terrain
export function plotMask(x, z) {
  let m = 0;
  for (const b of BUILDINGS) {
    const dx = Math.max(Math.abs(x - b.x) - b.w / 2, 0);
    const dz = Math.max(Math.abs(z - b.z) - b.d / 2, 0);
    m = Math.max(m, 1 - smoothstep(0.6, 4.5, Math.hypot(dx, dz)));
  }
  m = Math.max(m, 1 - smoothstep(6.5, 10.5, Math.hypot(x - PLAZA.x, z - PLAZA.z)));
  return m;
}
export function mountainHeight(x, z) {
  const d = Math.hypot(x - MOUNTAIN.x, z - MOUNTAIN.z);
  if (d >= MOUNTAIN.r) return 0;
  const t = 1 - d / MOUNTAIN.r;
  return MOUNTAIN.h * Math.pow(t, 1.25) * (0.8 + 0.4 * vnoise(x * 0.12, z * 0.12));
}
export function terrainHeight(x, z) {
  const dv = Math.hypot(x - 2, z - 2);
  const rim = smoothstep(40, 80, dv) * (5 + 6 * vnoise(x * 0.04 + 7, z * 0.04 - 3));
  const roll = 0.7 * Math.sin(x * 0.19 + 1.3) * Math.cos(z * 0.16 - 0.4) + 0.9 * (vnoise(x * 0.11, z * 0.11) - 0.5);
  const village = (roll + 0.55) * (1 - plotMask(x, z));
  const far = 7 * (vnoise(x * 0.015 + 3, z * 0.015 - 9) - 0.5) * smoothstep(80, 140, dv);
  const ring = 1 - smoothstep(9, 15, Math.hypot(x - MOONRING.x, z - MOONRING.z));
  const hut = 1 - smoothstep(5, 9, Math.hypot(x - HUT.x, z - HUT.z));
  let h = village + (rim + far) * (1 - Math.max(ring, hut) * 0.85) + mountainHeight(x, z);
  h -= 4.5 * (1 - smoothstep(3.5, 7.5, Math.abs(x - riverX(z))));
  return h;
}

// ---------------------------------------------------------------- collision
// A space is a separate walkable layer: 'outdoor' (ground floors share it, because
// doors connect them), 'upper:<id>', 'below' (cellars + tunnel), 'cave'.
// walk === null means open ground; otherwise the circle's bounding square must sit
// inside the union of walk rects. Boxes and circles are solid.
export class Collision {
  constructor() { this.spaces = new Map(); }
  space(id) {
    let s = this.spaces.get(id);
    if (!s) { s = { id, walk: null, boxes: [], circles: [] }; this.spaces.set(id, s); }
    return s;
  }
  walk(id, r) { (this.space(id).walk ??= []).push(r); }
  box(id, x0, x1, z0, z1) {
    const b = [Math.min(x0, x1), Math.max(x0, x1), Math.min(z0, z1), Math.max(z0, z1)];
    this.space(id).boxes.push(b);
    return b;
  }
  remove(id, b) { const s = this.space(id); s.boxes = s.boxes.filter((o) => o !== b); }
  block(id, fn) { (this.space(id).funcs ??= []).push(fn); }
  circle(id, x, z, r) { this.space(id).circles.push([x, z, r]); }
  canStand(id, x, z, r) {
    if (Math.abs(x) > WORLD_HALF || Math.abs(z) > WORLD_HALF) return false;
    const s = this.spaces.get(id);
    if (!s) return false;
    if (s.walk) {
      for (const [cx, cz] of [[x - r, z - r], [x + r, z - r], [x - r, z + r], [x + r, z + r]]) {
        if (!s.walk.some((w) => inRect(w, cx, cz))) return false;
      }
    }
    for (const b of s.boxes) {
      const qx = clamp(x, b[0], b[1]), qz = clamp(z, b[2], b[3]);
      if ((x - qx) ** 2 + (z - qz) ** 2 < r * r) return false;
    }
    for (const c of s.circles) if (Math.hypot(x - c[0], z - c[1]) < c[2] + r) return false;
    if (s.funcs) for (const f of s.funcs) if (f(x, z, r)) return false;
    return true;
  }
}

export function buildingAt(x, z) {
  return BUILDINGS.find((b) => inRect(b.inner, x, z)) ?? null;
}
export function floorY(world, space, x, z) {
  if (space === 'outdoor' && world.bridge && inRect(world.bridge.rect, x, z)) return world.bridge.y;
  if (space === 'outdoor') return buildingAt(x, z) ? FLOOR_Y : terrainHeight(x, z);
  if (space.startsWith('upper:')) return UPPER_Y;
  if (space === 'below') return BELOW_Y;
  return world.caveY;
}

// ---------------------------------------------------------------- materials + primitives
const matCache = new Map();
export function mat(color) {
  let m = matCache.get(color);
  if (!m) {
    m = new THREE.MeshStandardMaterial({ color, roughness: 0.88, metalness: 0, flatShading: true });
    matCache.set(color, m);
  }
  return m;
}
export function glow(color, intensity = 1.5) {
  return new THREE.MeshStandardMaterial({ color, emissive: color, emissiveIntensity: intensity, roughness: 0.6 });
}
export const WINDOW_MAT = new THREE.MeshStandardMaterial({ color: 0x2b3340, emissive: 0xffb35a, emissiveIntensity: 0, roughness: 0.35 });
const geoCache = new Map();
function boxGeo(w, h, d) {
  const k = `${w.toFixed(3)}|${h.toFixed(3)}|${d.toFixed(3)}`;
  let g = geoCache.get(k);
  if (!g) { g = new THREE.BoxGeometry(w, h, d); geoCache.set(k, g); }
  return g;
}
export function box(parent, w, h, d, x, y, z, color, o = {}) {
  const m = new THREE.Mesh(boxGeo(w, h, d), o.material ?? mat(color));
  m.position.set(x, y, z);
  m.castShadow = o.cast ?? true;
  m.receiveShadow = o.receive ?? true;
  if (parent) parent.add(m);
  return m;
}
export function cyl(parent, rt, rb, h, x, y, z, color, seg = 10, o = {}) {
  const m = new THREE.Mesh(new THREE.CylinderGeometry(rt, rb, h, seg), o.material ?? mat(color));
  m.position.set(x, y, z);
  m.castShadow = o.cast ?? true;
  m.receiveShadow = o.receive ?? true;
  if (parent) parent.add(m);
  return m;
}
function rock(parent, x, y, z, s, color = 0x6f6a64, rng = Math.random) {
  const m = new THREE.Mesh(new THREE.DodecahedronGeometry(s, 0), mat(color));
  m.position.set(x, y, z);
  m.rotation.set(rng() * 3, rng() * 3, rng() * 3);
  m.scale.set(1, 0.65 + rng() * 0.5, 1);
  m.castShadow = true; m.receiveShadow = true;
  parent.add(m);
  return m;
}
function slab(parent, r, yTop, th, color) {
  const [x0, x1, z0, z1] = r;
  return box(parent, x1 - x0, th, z1 - z0, (x0 + x1) / 2, yTop - th / 2, (z0 + z1) / 2, color, { cast: false });
}
function rectMinus(r, h) {
  if (!h) return [r];
  const [x0, x1, z0, z1] = r, [hx0, hx1, hz0, hz1] = h;
  const out = [];
  if (hz0 > z0) out.push([x0, x1, z0, hz0]);
  if (hz1 < z1) out.push([x0, x1, hz1, z1]);
  if (hx0 > x0) out.push([x0, hx0, hz0, hz1]);
  if (hx1 < x1) out.push([hx1, x1, hz0, hz1]);
  return out;
}
function intervalsMinus(a, c, gaps) {
  let segs = [[a, c]];
  for (const [g0, g1] of gaps) {
    const next = [];
    for (const [s0, s1] of segs) {
      if (g1 <= s0 || g0 >= s1) { next.push([s0, s1]); continue; }
      if (g0 > s0) next.push([s0, g0]);
      if (g1 < s1) next.push([g1, s1]);
    }
    segs = next;
  }
  return segs.filter(([s0, s1]) => s1 - s0 > 0.05);
}
function fadeable(b, key, mesh) {
  mesh.material = mesh.material.clone();
  b.fades[key].push(mesh);
  return mesh;
}

// ---------------------------------------------------------------- furniture kit
function group(parent, x, y, z, rotY = 0) {
  const g = new THREE.Group();
  g.position.set(x, y, z);
  g.rotation.y = rotY;
  parent.add(g);
  return g;
}
function table(p, x, y, z, w, d, color = 0x6b4a2b) {
  const g = group(p, x, y, z);
  box(g, w, 0.08, d, 0, 0.78, 0, color);
  for (const sx of [-1, 1]) for (const sz of [-1, 1]) box(g, 0.09, 0.74, 0.09, sx * (w / 2 - 0.12), 0.37, sz * (d / 2 - 0.12), 0x4a3220);
  return g;
}
function chair(p, x, y, z, rot, color = 0x5a3d24) {
  const g = group(p, x, y, z, rot);
  box(g, 0.46, 0.06, 0.46, 0, 0.46, 0, color);
  box(g, 0.46, 0.55, 0.06, 0, 0.75, -0.2, color);
  for (const sx of [-1, 1]) for (const sz of [-1, 1]) box(g, 0.06, 0.44, 0.06, sx * 0.19, 0.22, sz * 0.19, 0x3c2816);
  return g;
}
function stool(p, x, y, z) {
  const g = group(p, x, y, z);
  cyl(g, 0.22, 0.22, 0.07, 0, 0.62, 0, 0x6b4a2b, 8);
  for (const a of [0, 2.1, 4.2]) box(g, 0.05, 0.6, 0.05, Math.cos(a) * 0.14, 0.3, Math.sin(a) * 0.14, 0x3c2816);
  return g;
}
function bed(p, x, y, z, w, d, blanket) {
  const g = group(p, x, y, z);
  box(g, w, 0.35, d, 0, 0.18, 0, 0x5c3f27);
  box(g, w - 0.12, 0.18, d - 0.12, 0, 0.44, 0, 0xe9e2d0);
  box(g, w - 0.08, 0.08, d * 0.62, 0, 0.56, d * 0.17, blanket);
  box(g, w * 0.55, 0.14, 0.38, 0, 0.6, -d / 2 + 0.3, 0xf6f2e8);
  box(g, w, 0.9, 0.1, 0, 0.45, -d / 2 + 0.05, 0x4a3220);
  return g;
}
function barrel(p, x, y, z, r = 0.38, h = 0.95) {
  const g = group(p, x, y, z);
  cyl(g, r * 0.92, r * 0.92, h, 0, h / 2, 0, 0x7b5533, 12);
  cyl(g, r * 0.97, r * 0.97, 0.06, 0, h * 0.2, 0, 0x2f2a26, 12);
  cyl(g, r * 0.97, r * 0.97, 0.06, 0, h * 0.8, 0, 0x2f2a26, 12);
  return g;
}
function crate(p, x, y, z, s = 0.7, rot = 0) {
  const g = group(p, x, y, z, rot);
  box(g, s, s, s, 0, s / 2, 0, 0x9a7444);
  box(g, s + 0.02, 0.08, s + 0.02, 0, s - 0.1, 0, 0x6b4a2b);
  box(g, s + 0.02, 0.08, s + 0.02, 0, 0.1, 0, 0x6b4a2b);
  return g;
}
const TRINKETS = [0x8a2d2d, 0x2d5a8a, 0x3f7a3a, 0xc9a646, 0x6a4a8a, 0xd8d0c0, 0x2a2a2a];
function shelf(p, x, y, z, w, d, h, rng, alongX = true) {
  const g = group(p, x, y, z, alongX ? 0 : Math.PI / 2);
  box(g, w, h, 0.06, 0, h / 2, -d / 2 + 0.03, 0x4a3220);
  for (const sx of [-1, 1]) box(g, 0.06, h, d, sx * (w / 2 - 0.03), h / 2, 0, 0x4a3220);
  for (let k = 1; k <= 3; k++) {
    const sy = (h * k) / 3.4;
    box(g, w, 0.05, d, 0, sy, 0, 0x5c3f27);
    let cx = -w / 2 + 0.15;
    while (cx < w / 2 - 0.2) {
      const iw = 0.08 + rng() * 0.14, ih = 0.12 + rng() * 0.22;
      if (rng() > 0.2) box(g, iw, ih, d * 0.6, cx + iw / 2, sy + ih / 2 + 0.025, 0, TRINKETS[Math.floor(rng() * TRINKETS.length)], { cast: false });
      cx += iw + 0.02 + rng() * 0.08;
    }
  }
  return g;
}
function rug(p, x, y, z, w, d, color) {
  const g = group(p, x, y, z);
  box(g, w, 0.02, d, 0, 0.01, 0, color, { cast: false });
  box(g, w - 0.3, 0.025, d - 0.3, 0, 0.012, 0, 0xd8b45a, { cast: false });
  box(g, w - 0.4, 0.03, d - 0.4, 0, 0.015, 0, color, { cast: false });
  return g;
}
function chest(p, x, y, z, rot = 0) {
  const g = group(p, x, y, z, rot);
  box(g, 0.9, 0.5, 0.5, 0, 0.25, 0, 0x7b5533);
  box(g, 0.92, 0.12, 0.52, 0, 0.55, 0, 0x5c3f27);
  box(g, 0.12, 0.14, 0.04, 0, 0.42, 0.26, 0xc9a646);
  return g;
}
function candle(p, x, y, z) {
  box(p, 0.06, 0.16, 0.06, x, y + 0.08, z, 0xf2ead6, { cast: false });
  const f = box(p, 0.05, 0.08, 0.05, x, y + 0.2, z, 0, { material: glow(0xffc070, 2.5), cast: false });
  return f;
}
function sack(p, x, y, z) {
  const g = group(p, x, y, z);
  const m = new THREE.Mesh(new THREE.SphereGeometry(0.35, 7, 5), mat(0xc8b48a));
  m.scale.set(1, 1.3, 1); m.position.y = 0.42; m.castShadow = true; g.add(m);
  box(g, 0.2, 0.12, 0.2, 0, 0.88, 0, 0xa8946a);
  return g;
}
function plant(p, x, y, z) {
  const g = group(p, x, y, z);
  cyl(g, 0.22, 0.16, 0.35, 0, 0.17, 0, 0xa0522d, 8);
  const leaves = new THREE.Mesh(new THREE.IcosahedronGeometry(0.38, 0), mat(0x4f8a3a));
  leaves.position.y = 0.6; leaves.castShadow = true; g.add(leaves);
  return g;
}
function lantern(p, x, y, z) {
  box(p, 0.04, 0.5, 0.04, x, y + 0.25, z, 0x2a2420, { cast: false });
  box(p, 0.22, 0.28, 0.22, x, y - 0.12, z, 0, { material: glow(0xffb050, 2.2), cast: false });
}
function fireplace(p, x, y, z, along, ctx, w = 2) {
  // A hearth set against an east or west wall; along = 'E' means the back is east.
  const g = group(p, x, y, z, along === 'E' ? -Math.PI / 2 : Math.PI / 2);
  box(g, w, 2.2, 0.75, 0, 1.1, 0, 0x7d7670);
  box(g, w - 0.7, 1.0, 0.3, 0, 0.55, 0.25, 0x141110, { cast: false });
  box(g, w + 0.3, 0.15, 0.9, 0, 2.25, 0.05, 0x5a3b24);
  const fire = new THREE.Mesh(new THREE.ConeGeometry(0.3, 0.7, 6), glow(0xff7a2a, 3));
  fire.position.set(0, 0.45, 0.25); g.add(fire);
  for (const sx of [-0.25, 0.05, 0.3]) box(g, 0.5, 0.1, 0.12, sx, 0.12, 0.25, 0x3a2414, { cast: false });
  const light = new THREE.PointLight(0xff8a3a, 6, 9, 1.6);
  light.position.set(0, 0.9, 0.9); g.add(light);
  ctx.fires.push({ light, base: 6, flame: fire });
  return g;
}
function torch(p, x, y, z, ctx, intensity = 5, range = 9) {
  box(p, 0.08, 0.5, 0.08, x, y, z, 0x3a2414, { cast: false });
  const f = new THREE.Mesh(new THREE.ConeGeometry(0.12, 0.3, 6), glow(0xffa040, 3));
  f.position.set(x, y + 0.38, z); p.add(f);
  const light = new THREE.PointLight(0xff9a4a, intensity, range, 1.5);
  light.position.set(x, y + 0.6, z); p.add(light);
  ctx.fires.push({ light, base: intensity, flame: f });
}

// ---------------------------------------------------------------- buildings
const NORMALS = { N: [0, -1], S: [0, 1], E: [1, 0], W: [-1, 0] };
function buildWalls(b, parent, side, y0, y1, gaps, fadeKey, space, ctx) {
  const t = WALL_T;
  const horiz = side === 'N' || side === 'S';
  const line = side === 'N' ? b.z0 : side === 'S' ? b.z1 : side === 'E' ? b.x1 : b.x0;
  const [a, c] = horiz ? [b.x0 - t / 2, b.x1 + t / 2] : [b.z0 + t / 2, b.z1 - t / 2];
  const h = y1 - y0;
  const n = NORMALS[side];
  for (const [s0, s1] of intervalsMinus(a, c, gaps)) {
    const len = s1 - s0, mid = (s0 + s1) / 2;
    const m = horiz ? box(parent, len, h, t, mid, y0 + h / 2, line, b.wall) : box(parent, t, h, len, line, y0 + h / 2, mid, b.wall);
    if (fadeKey) fadeable(b, fadeKey, m);
    if (space) {
      if (horiz) ctx.col.box(space, s0, s1, line - t / 2, line + t / 2);
      else ctx.col.box(space, line - t / 2, line + t / 2, s0, s1);
    }
    // timber band at the top, stone plinth at the bottom, windows: all children so they fade with it
    box(m, horiz ? len : t + 0.06, 0.2, horiz ? t + 0.06 : len, 0, h / 2 - 0.1, 0, b.timber);
    if (y0 < 0.5) box(m, horiz ? len : t + 0.08, 0.45, horiz ? t + 0.08 : len, 0, -h / 2 + 0.225, 0, 0x6d665e);
    if (len > 2.4) {
      const count = Math.max(1, Math.floor(len / 3.2));
      const ly = 1.65 - h / 2;
      for (let i = 0; i < count; i++) {
        const off = -len / 2 + (i + 0.5) * (len / count);
        const fx = horiz ? off : n[0] * (t / 2 + 0.03);
        const fz = horiz ? n[1] * (t / 2 + 0.03) : off;
        box(m, horiz ? 1.15 : 0.06, 1.05, horiz ? 0.06 : 1.15, fx, ly, fz, b.timber, { cast: false });
        const pane = new THREE.Mesh(boxGeo(horiz ? 0.85 : 0.07, 0.78, horiz ? 0.07 : 0.85), WINDOW_MAT);
        pane.position.set(fx + n[0] * 0.012, ly, fz + n[1] * 0.012);
        m.add(pane);
      }
    }
  }
}

function buildRoof(b, top) {
  const r = b.roofGroup;
  const halfD = b.d / 2 + 0.5, rise = b.d * 0.36;
  const a = Math.atan2(rise, halfD), L = Math.hypot(halfD, rise) + 0.1;
  const north = box(r, b.w + 1, 0.25, L, b.x, top + rise / 2, b.z - halfD / 2, b.roof);
  north.rotation.x = -a;
  const south = box(r, b.w + 1, 0.25, L, b.x, top + rise / 2, b.z + halfD / 2, b.roof);
  south.rotation.x = a;
  box(r, b.w + 1.1, 0.2, 0.3, b.x, top + rise + 0.05, b.z, b.timber);
  const shape = new THREE.Shape();
  shape.moveTo(-b.d / 2, 0); shape.lineTo(b.d / 2, 0); shape.lineTo(0, rise); shape.closePath();
  const gableGeo = new THREE.ExtrudeGeometry(shape, { depth: WALL_T, bevelEnabled: false });
  for (const gx of [b.x0 - WALL_T / 2, b.x1 - WALL_T / 2]) {
    const gm = new THREE.Mesh(gableGeo, mat(b.wall));
    gm.rotation.y = Math.PI / 2;
    gm.position.set(gx, top, b.z);
    gm.castShadow = true; gm.receiveShadow = true;
    r.add(gm);
  }
  if (b.chimney) {
    const [cx, cz] = b.chimney;
    box(r, 0.8, rise + 2.2, 0.8, cx, top + (rise + 2.2) / 2 - 0.4, cz, 0x7d7670);
    box(r, 0.95, 0.2, 0.95, cx, top + rise + 1.7, cz, 0x5d5650);
  }
  for (const m of r.children) fadeable(b, 'roof', m);
}

function buildRails(stair, b, ctx) {
  for (const rail of stair.rails) {
    ctx.col.box(rail.space, ...rail.r);
    const [x0, x1, z0, z1] = rail.r;
    const parent = rail.space === 'below' ? ctx.groups.below : rail.space.startsWith('upper:') ? b.upper : b.ground;
    const base = rail.space === 'below' ? BELOW_Y : rail.space.startsWith('upper:') ? UPPER_Y : FLOOR_Y;
    const color = rail.space === 'below' ? 0x5e5952 : rail.h > 1.2 ? b.wall : b.timber;
    box(parent, x1 - x0, rail.h, z1 - z0, (x0 + x1) / 2, base + rail.h / 2, (z0 + z1) / 2, color);
  }
}
function buildStairSteps(stair, parent, color) {
  const n = 12;
  const [x0, x1, z0, z1] = stair.rect;
  const span = stair.highAt - stair.lowAt;
  const base = Math.min(stair.yLow, stair.yHigh) - 0.15;
  for (let i = 0; i < n; i++) {
    const a = stair.lowAt + (span * i) / n, c = stair.lowAt + (span * (i + 1)) / n;
    const top = stair.yLow + ((stair.yHigh - stair.yLow) * (i + 1)) / n;
    const h = top - base;
    const lo = Math.min(a, c), len = Math.abs(c - a);
    if (stair.axis === 'z') box(parent, x1 - x0 - 0.04, h, len, (x0 + x1) / 2, base + h / 2, lo + len / 2, color);
    else box(parent, len, h, z1 - z0 - 0.04, lo + len / 2, base + h / 2, (z0 + z1) / 2, color);
  }
}

function buildBuilding(b, ctx) {
  const g = new THREE.Group(), ground = new THREE.Group(), upper = new THREE.Group(), roof = new THREE.Group();
  g.name = b.id;
  g.add(ground, upper, roof);
  ctx.groups.above.add(g);
  Object.assign(b, { group: g, ground, upper, roofGroup: roof, fades: { S0: [], S1: [], roof: [] } });
  const two = b.floors === 2;
  const groundTop = two ? UPPER_Y : FLOOR_Y + b.wallH;
  const top = two ? UPPER_Y + UPPER_H : groundTop;
  b.top = top;

  const holeDown = STAIRS.find((s) => s.building === b.id && s.low === 'below');
  for (const r of rectMinus(b.inner, holeDown?.rect)) slab(ground, r, FLOOR_Y, 0.16, 0x7a5a3c);
  if (holeDown) {
    const [x0, x1, z0, z1] = holeDown.rect;
    box(ground, x1 - x0, 0.1, z1 - z0, (x0 + x1) / 2, -0.7, (z0 + z1) / 2, 0x0d0b0a, { cast: false });
  }
  const doorGap = [[b.doorX - DOOR_W / 2, b.doorX + DOOR_W / 2]];
  buildWalls(b, ground, 'N', 0, groundTop, [], null, 'outdoor', ctx);
  buildWalls(b, ground, 'E', 0, groundTop, [], null, 'outdoor', ctx);
  buildWalls(b, ground, 'W', 0, groundTop, [], null, 'outdoor', ctx);
  buildWalls(b, ground, 'S', 0, groundTop, doorGap, 'S0', 'outdoor', ctx);
  fadeable(b, 'S0', box(ground, DOOR_W, groundTop - DOOR_H, WALL_T, b.doorX, DOOR_H + (groundTop - DOOR_H) / 2, b.z1, b.wall));
  for (const sx of [-1, 1]) fadeable(b, 'S0', box(ground, 0.16, DOOR_H, WALL_T + 0.1, b.doorX + sx * (DOOR_W / 2 + 0.02), DOOR_H / 2, b.z1, b.timber));
  // open door leaf, swung inward against the west jamb
  const leafX = b.doorX - DOOR_W / 2 + 0.06;
  box(ground, 0.08, DOOR_H - 0.1, DOOR_W - 0.1, leafX, FLOOR_Y + (DOOR_H - 0.1) / 2, b.z1 - WALL_T / 2 - (DOOR_W - 0.1) / 2, 0x6b4426);
  ctx.col.box('outdoor', leafX - 0.05, leafX + 0.05, b.z1 - WALL_T / 2 - DOOR_W, b.z1 - WALL_T / 2);
  box(ground, DOOR_W + 0.4, 0.12, 0.6, b.doorX, 0.06, b.z1 + 0.35, 0x6d665e);
  for (const [cx, cz] of [[b.x0, b.z0], [b.x1, b.z0], [b.x0, b.z1], [b.x1, b.z1]]) {
    const post = box(ground, 0.42, top + 0.05, 0.42, cx, (top + 0.05) / 2, cz, b.timber);
    if (cz === b.z1) fadeable(b, 'S0', post);
  }
  if (two) {
    const holeUp = STAIRS.find((s) => s.building === b.id && s.high === `upper:${b.id}`);
    for (const r of rectMinus(b.inner, holeUp?.rect)) slab(upper, r, UPPER_Y, 0.2, 0x6e5034);
    buildWalls(b, upper, 'N', UPPER_Y, top, [], null, null, ctx);
    buildWalls(b, upper, 'E', UPPER_Y, top, [], null, null, ctx);
    buildWalls(b, upper, 'W', UPPER_Y, top, [], null, null, ctx);
    buildWalls(b, upper, 'S', UPPER_Y, top, [], 'S1', null, ctx);
    ctx.col.walk(`upper:${b.id}`, b.inner);
    // jettied floor line on the outside
    box(ground, b.w + 0.5, 0.25, b.d + 0.5, b.x, UPPER_Y - 0.05, b.z, b.timber, { cast: false });
  }
  for (const s of STAIRS.filter((st) => st.building === b.id)) {
    if (s.low === 'below') buildStairSteps(s, ctx.groups.below, 0x6e675f);
    else buildStairSteps(s, ground, 0x6b4a2b);
    buildRails(s, b, ctx);
    if (s.low === 'below') ctx.col.walk('below', s.rect);
    else ctx.col.walk(s.high, s.rect);
  }
  buildRoof(b, top);
  b.bounds = new THREE.Box3(new THREE.Vector3(b.x0 - 0.6, 0, b.z0 - 1), new THREE.Vector3(b.x1 + 0.6, top + b.d * 0.4, b.z1 + 0.6));
}

// ---------------------------------------------------------------- interiors
function furnishTavern(b, ctx, rng) {
  const g = b.ground, y = FLOOR_Y, sp = 'outdoor';
  // bar counter with taps and mugs
  const counter = group(g, 1, y, -20.2);
  box(counter, 8, 1.0, 0.8, 0, 0.5, 0, 0x6b4426);
  box(counter, 8.2, 0.08, 0.95, 0, 1.04, 0, 0x4a2c18);
  for (let i = 0; i < 4; i++) cyl(counter, 0.06, 0.06, 0.4, -2.5 + i * 0.6, 1.25, -0.2, 0xb8902e, 6);
  for (let i = 0; i < 5; i++) cyl(counter, 0.09, 0.08, 0.18, 0.5 + i * 0.55, 1.17, 0.15, 0xd8c8a0, 8);
  ctx.col.box(sp, -3, 5, -20.6, -19.8);
  shelf(g, 1, y, -21.55, 6, 0.3, 2.2, rng);
  ctx.col.box(sp, -2, 4, -21.7, -21.4);
  barrel(g, 5.5, y, -21.1); ctx.col.box(sp, 5.1, 5.9, -21.5, -20.7);
  barrel(g, -2.7, y, -21.1); ctx.col.box(sp, -3.1, -2.3, -21.5, -20.7);
  for (const [tx, tz] of [[-2, -13], [3.5, -13.5], [-3.5, -17]]) {
    table(g, tx, y, tz, 1.2, 1.2);
    ctx.col.box(sp, tx - 0.6, tx + 0.6, tz - 0.6, tz + 0.6);
    chair(g, tx, y, tz - 0.95, 0); chair(g, tx, y, tz + 0.95, Math.PI);
    chair(g, tx - 0.95, y, tz, Math.PI / 2); chair(g, tx + 0.95, y, tz, -Math.PI / 2);
    candle(g, tx, y + 0.82, tz);
    cyl(g, 0.09, 0.08, 0.18, tx + 0.3, y + 0.91, tz - 0.2, 0xd8c8a0, 8);
  }
  fireplace(g, 7.45, y, -13, 'E', ctx);
  ctx.col.box(sp, 7.05, 7.85, -14, -12);
  const arm = group(g, 5.6, y, -13, -Math.PI / 2);
  box(arm, 0.9, 0.45, 0.9, 0, 0.22, 0, 0x8a2d2d);
  box(arm, 0.9, 0.7, 0.2, 0, 0.75, -0.35, 0x8a2d2d);
  for (const sx of [-1, 1]) box(arm, 0.15, 0.35, 0.9, sx * 0.4, 0.55, 0, 0x7a2525);
  ctx.col.box(sp, 5.15, 6.05, -13.45, -12.55);
  rug(g, 4.6, y, -13, 2.2, 2.6, 0x7a2c2c);
  rug(g, 0, y, -11.4, 2.4, 1.4, 0x2c4a7a);
  for (const [lx, lz] of [[-2, -14.5], [3, -16], [0, -18.5]]) lantern(g, lx, y + 2.45, lz);
  // notice board on the west wall
  box(g, 0.06, 0.9, 1.4, -7.8, y + 1.6, -11.6, 0x5c3f27);
  for (let i = 0; i < 3; i++) box(g, 0.02, 0.25, 0.22, -7.76, y + 1.5 + (i % 2) * 0.25, -12 + i * 0.35, 0xeee6d0, { cast: false });
  stool(g, -1.5, y, -19.2); stool(g, 0.5, y, -19.2); stool(g, 2.5, y, -19.2);

  // upper floor: hall on the west, two rooms on the east
  const u = b.upper, uy = UPPER_Y, us = 'upper:tavern';
  const part = (x0, x1, z0, z1) => {
    box(u, x1 - x0, 2.5, z1 - z0, (x0 + x1) / 2, uy + 1.25, (z0 + z1) / 2, 0xcdbd9a);
    ctx.col.box(us, x0, x1, z0, z1);
  };
  for (const [z0, z1] of intervalsMinus(-21.7, -10.3, [[-13.8, -12.2], [-19.8, -18.2]])) part(-0.1, 0.1, z0, z1);
  part(0.1, 7.7, -16.1, -15.9);
  bed(u, 6.55, uy, -20.6, 2.0, 2.1, 0x2c5a3a); ctx.col.box(us, 5.55, 7.55, -21.65, -19.55);
  chest(u, 3.0, uy, -21.2); ctx.col.box(us, 2.55, 3.45, -21.45, -20.95);
  table(u, 6.9, uy, -17.0, 0.7, 0.7); ctx.col.box(us, 6.55, 7.25, -17.35, -16.65);
  candle(u, 6.9, uy + 0.82, -17.0);
  rug(u, 4.2, uy, -18.8, 2.2, 1.8, 0x5a3a6a);
  bed(u, 6.55, uy, -11.4, 2.0, 1.9, 0x7a2c2c); ctx.col.box(us, 5.55, 7.55, -12.35, -10.45);
  chest(u, 3.6, uy, -10.85, Math.PI); ctx.col.box(us, 3.15, 4.05, -11.1, -10.6);
  box(u, 0.6, 0.9, 0.6, 7.2, uy + 0.45, -14.8, 0x6b4a2b); ctx.col.box(us, 6.9, 7.5, -15.1, -14.5);
  cyl(u, 0.22, 0.16, 0.12, 7.2, uy + 0.96, -14.8, 0xd8d0c0, 10);
  const bench = group(u, -3, uy, -21.3);
  box(bench, 2, 0.08, 0.45, 0, 0.45, 0, 0x6b4a2b);
  for (const sx of [-0.85, 0.85]) box(bench, 0.08, 0.45, 0.4, sx, 0.22, 0, 0x4a3220);
  ctx.col.box(us, -4, -2, -21.55, -21.05);
  plant(u, -1.0, uy, -10.8); ctx.col.box(us, -1.3, -0.7, -11.1, -10.5);
  rug(u, -3.5, uy, -16, 1.6, 7, 0x6a4a2a);
  lantern(u, -3.5, uy + 2.2, -16);
}

function furnishSmithy(b, ctx, rng) {
  const g = b.ground, y = FLOOR_Y, sp = 'outdoor';
  const forge = group(g, 17.5, y, 0.75);
  box(forge, 2, 0.9, 2.5, 0, 0.45, 0, 0x6d665e);
  box(forge, 1.4, 0.12, 1.8, 0, 0.96, 0, 0, { material: glow(0xff5a1a, 2.6), cast: false });
  box(forge, 1.6, 0.9, 1.6, 0, 2.7, 0, 0x4a4540);
  box(forge, 0.7, 1.2, 0.7, 0, 3.6, 0, 0x4a4540);
  const fl = new THREE.PointLight(0xff6a2a, 8, 10, 1.6);
  fl.position.set(0.6, 1.5, 0.6); forge.add(fl);
  ctx.fires.push({ light: fl, base: 8 });
  ctx.col.box(sp, 16.5, 18.5, -0.5, 2.0);
  const anvil = group(g, 19.5, y, 4);
  cyl(anvil, 0.35, 0.4, 0.5, 0, 0.25, 0, 0x5c3f27, 8);
  box(anvil, 0.8, 0.22, 0.32, 0, 0.62, 0, 0x2e2e33);
  box(anvil, 0.4, 0.18, 0.2, 0, 0.45, 0, 0x2e2e33);
  ctx.col.box(sp, 19.05, 19.95, 3.75, 4.25);
  barrel(g, 16.9, y, 4.6, 0.4, 0.8);
  box(g, 0.6, 0.02, 0.6, 16.9, y + 0.78, 4.6, 0x2a4a6a, { cast: false });
  ctx.col.box(sp, 16.5, 17.3, 4.2, 5.0);
  const rack = group(g, 27.4, y, 5);
  box(rack, 0.2, 2.0, 3, 0, 1.0, 0, 0x4a3220);
  for (let i = 0; i < 5; i++) box(rack, 0.08, 1.2, 0.08, -0.15, 1.1, -1.2 + i * 0.6, 0x9aa0a8);
  ctx.col.box(sp, 27.15, 27.7, 3.5, 6.5);
  table(g, 25.5, y, 8.25, 3, 0.7, 0x5c3f27);
  ctx.col.box(sp, 24, 27, 7.9, 8.6);
  for (let i = 0; i < 4; i++) box(g, 0.3, 0.05, 0.12, 24.4 + i * 0.7, y + 0.85, 8.2, 0x8a8f96, { cast: false });
  crate(g, 26.9, y, 0.2, 0.7); ctx.col.box(sp, 26.55, 27.25, -0.15, 0.55);
  void rng;
}

function furnishWeaver(b, ctx, rng) {
  const g = b.ground, y = FLOOR_Y, sp = 'outdoor';
  const loom = group(g, -22.5, y, 7.6);
  for (const sx of [-0.9, 0.9]) box(loom, 0.12, 1.6, 1.2, sx, 0.8, 0, 0x6b4a2b);
  box(loom, 1.9, 0.1, 0.1, 0, 1.55, 0, 0x6b4a2b);
  box(loom, 1.7, 0.9, 0.04, 0, 0.95, 0, 0x8a5aa0);
  ctx.col.box(sp, -23.5, -21.5, 6.9, 8.3);
  table(g, -17.5, y, 7.5, 1.4, 1.2); ctx.col.box(sp, -18.2, -16.8, 6.9, 8.1);
  chair(g, -17.5, y, 8.5, Math.PI); candle(g, -17.5, y + 0.82, 7.5);
  for (let i = 0; i < 4; i++) box(g, 0.3, 0.12, 0.3, -18 + i * 0.32, y + 0.88, 7.2, TRINKETS[i], { cast: false });
  const wheel = group(g, -23.8, y, 5);
  cyl(wheel, 0.45, 0.45, 0.06, 0, 0.8, 0, 0x6b4a2b, 14).rotation.z = Math.PI / 2;
  box(wheel, 0.8, 0.4, 0.3, 0, 0.3, 0, 0x5c3f27);
  ctx.col.box(sp, -24.4, -23.2, 4.5, 5.5);
  rug(g, -20, y, 6.5, 2.6, 1.8, 0x3a6a5a);
  const u = b.upper, uy = UPPER_Y, us = 'upper:weaver';
  bed(u, -23.7, uy, 8.5, 1.9, 2.2, 0x8a5aa0); ctx.col.box(us, -24.65, -22.75, 7.4, 9.6);
  chest(u, -17, uy, 9.3, Math.PI); ctx.col.box(us, -17.45, -16.55, 9.05, 9.55);
  rug(u, -20.5, uy, 7, 2.4, 2, 0xa06a3a);
  plant(u, -16, uy, 5); ctx.col.box(us, -16.3, -15.7, 4.7, 5.3);
  void rng;
}

function furnishBakery(b, ctx, rng) {
  const g = b.ground, y = FLOOR_Y, sp = 'outdoor';
  const oven = group(g, -9.6, y, 19.4);
  const dome = new THREE.Mesh(new THREE.SphereGeometry(0.85, 10, 6, 0, Math.PI * 2, 0, Math.PI / 2), mat(0xa86a48));
  dome.position.y = 0.9; dome.castShadow = true; oven.add(dome);
  box(oven, 1.6, 0.9, 1.6, 0, 0.45, 0, 0x8d877c);
  box(oven, 0.5, 0.35, 0.1, 0, 1.05, 0.8, 0, { material: glow(0xff7a2a, 2.4), cast: false });
  const ol = new THREE.PointLight(0xff8a3a, 4, 7, 1.6);
  ol.position.set(0, 1.1, 1.2); oven.add(ol);
  ctx.fires.push({ light: ol, base: 4 });
  ctx.col.box(sp, -10.4, -8.8, 18.6, 20.2);
  const counter = group(g, -3.6, y, 21.2);
  box(counter, 2.6, 0.95, 0.8, 0, 0.47, 0, 0x8a6a44);
  for (let i = 0; i < 6; i++) {
    const loaf = new THREE.Mesh(new THREE.SphereGeometry(0.16, 6, 4), mat(0xc88a4a));
    loaf.scale.set(1.5, 0.7, 1); loaf.position.set(-1 + i * 0.4, 1.02, 0); counter.add(loaf);
  }
  ctx.col.box(sp, -4.9, -2.3, 20.8, 21.6);
  shelf(g, -6, y, 18.4, 2.4, 0.5, 1.8, rng);
  ctx.col.box(sp, -7.2, -4.8, 18.15, 18.65);
  sack(g, -2.4, y, 24.8); sack(g, -2.0, y, 24.2); ctx.col.box(sp, -2.8, -1.65, 23.8, 25.2);
  table(g, -8.5, y, 24, 1.4, 1); ctx.col.box(sp, -9.2, -7.8, 23.5, 24.5);
}

function furnishArchive(b, ctx, rng) {
  const g = b.ground, y = FLOOR_Y, sp = 'outdoor';
  for (const z of [-26, -22.5, -19]) {
    shelf(g, 19.45, y, z, 2.6, 0.5, 3.4, rng, false); ctx.col.box(sp, 19.15, 19.75, z - 1.3, z + 1.3);
    shelf(g, 28.55, y, z, 2.6, 0.5, 3.4, rng, false); ctx.col.box(sp, 28.25, 28.85, z - 1.3, z + 1.3);
  }
  shelf(g, 24, y, -28.6, 4, 0.5, 3.4, rng); ctx.col.box(sp, 22, 26, -28.85, -28.35);
  table(g, 24, y, -22, 2.4, 1.2, 0x5c3f27); ctx.col.box(sp, 22.8, 25.2, -22.6, -21.4);
  for (const sx of [-0.7, 0.7]) chair(g, 24 + sx, y, -21.1, Math.PI);
  candle(g, 23.5, y + 0.82, -22); candle(g, 24.6, y + 0.82, -22.1);
  box(g, 0.5, 0.06, 0.35, 24.2, y + 0.86, -22.2, 0xe8dcc0, { cast: false });
  const lect = group(g, 24, y, -26.2);
  box(lect, 0.3, 1.1, 0.3, 0, 0.55, 0, 0x4a3220);
  box(lect, 0.7, 0.06, 0.5, 0, 1.15, 0, 0x5c3f27).rotation.x = -0.4;
  ctx.col.box(sp, 23.6, 24.4, -26.5, -25.9);
  const globe = group(g, 21.4, y, -16.6);
  cyl(globe, 0.06, 0.18, 0.8, 0, 0.4, 0, 0x4a3220, 6);
  const sph = new THREE.Mesh(new THREE.IcosahedronGeometry(0.35, 1), mat(0x3a6a8a));
  sph.position.y = 1.05; globe.add(sph);
  ctx.col.box(sp, 21.0, 21.8, -17.0, -16.2);
  for (const [lx, lz] of [[24, -18], [24, -25]]) lantern(g, lx, y + 3.4, lz);
  rug(g, 24, y, -18.5, 2.4, 3, 0x2c3a5a);
}

function furnishMill(b, ctx, rng) {
  const g = b.ground, y = FLOOR_Y, sp = 'outdoor';
  const stone = group(g, 8, y, 24.6);
  cyl(stone, 1.0, 1.0, 0.35, 0, 0.18, 0, 0x8d877c, 14);
  cyl(stone, 0.95, 0.95, 0.3, 0, 0.5, 0, 0x9d978c, 14);
  box(stone, 0.12, 1.2, 0.12, 0, 1.0, 0, 0x4a3220);
  ctx.col.box(sp, 7, 9, 23.6, 25.6);
  for (let i = 0; i < 6; i++) sack(g, 12.2 + (i % 3) * 0.7, y, 23.4 + Math.floor(i / 3) * 0.7);
  ctx.col.box(sp, 11.8, 14.2, 23.0, 24.6);
  for (const bx of [12, 13.8]) { box(g, 1.4, 1.3, 1.2, bx, y + 0.65, 28.9, 0x7b5533); }
  ctx.col.box(sp, 11.3, 14.5, 28.3, 29.5);
  table(g, 6.6, y, 28.6, 1.4, 1); ctx.col.box(sp, 5.9, 7.3, 28.1, 29.1);
  void rng;
}

// ---------------------------------------------------------------- underground + cave
function boundaryRuns(rooms, r) {
  // For each side of rect r, the stretches not opening into another room.
  const [x0, x1, z0, z1] = r;
  const eps = 0.05;
  const runs = [];
  const covered = (side) => {
    const gaps = [];
    for (const o of rooms) {
      if (o === r) continue;
      if (side === 'N' && o[2] <= z0 - eps && o[3] >= z0 - eps) gaps.push([o[0], o[1]]);
      if (side === 'S' && o[2] <= z1 + eps && o[3] >= z1 + eps) gaps.push([o[0], o[1]]);
      if (side === 'W' && o[0] <= x0 - eps && o[1] >= x0 - eps) gaps.push([o[2], o[3]]);
      if (side === 'E' && o[0] <= x1 + eps && o[1] >= x1 + eps) gaps.push([o[2], o[3]]);
    }
    return gaps;
  };
  for (const side of ['N', 'S', 'W', 'E']) {
    const horiz = side === 'N' || side === 'S';
    const [a, c] = horiz ? [x0, x1] : [z0, z1];
    for (const [s0, s1] of intervalsMinus(a, c, covered(side))) runs.push({ side, s0, s1, line: { N: z0, S: z1, W: x0, E: x1 }[side] });
  }
  return runs;
}

function buildBelow(ctx, rng) {
  const g = ctx.groups.below;
  const rects = BELOW_ROOMS.map((r) => r.r);
  for (const room of BELOW_ROOMS) {
    ctx.col.walk('below', room.r);
    const tunnel = room.id.startsWith('tunnel');
    slab(g, room.r, BELOW_Y, 0.3, tunnel ? 0x5a4a3a : 0x6a645c);
    for (const run of boundaryRuns(rects, room.r)) {
      const horiz = run.side === 'N' || run.side === 'S';
      const out = run.side === 'N' || run.side === 'W' ? -0.3 : 0.3;
      const len = run.s1 - run.s0 + 0.6, mid = (run.s0 + run.s1) / 2;
      const h = 3.1;
      if (horiz) box(g, len, h, 0.6, mid, BELOW_Y + h / 2, run.line + out, tunnel ? 0x5a4c3e : 0x77706a);
      else box(g, 0.6, h, len, run.line + out, BELOW_Y + h / 2, mid, tunnel ? 0x5a4c3e : 0x77706a);
    }
  }
  // tunnel shoring: posts on both sides every few metres
  for (let z = -14; z < 2; z += 3.5) for (const x of [1.05, 3.45]) box(g, 0.18, 2.4, 0.18, x, BELOW_Y + 1.2, z, 0x4a3220);
  for (let x = 5; x < 16; x += 3.5) for (const z of [2.05, 4.45]) box(g, 0.18, 2.4, 0.18, x, BELOW_Y + 1.2, z, 0x4a3220);
  // cellar under the tavern
  const y = BELOW_Y;
  for (const [bx, bz] of [[-4.5, -23.2], [-3.4, -23.2], [-2.3, -23.2], [-4.5, -22.1]]) barrel(g, bx, y, bz, 0.45, 1.05);
  ctx.col.box('below', -5.0, -1.8, -23.7, -22.7); ctx.col.box('below', -5.0, -4.0, -22.6, -21.6);
  const rackG = group(g, -5.6, y, -19.5);
  box(rackG, 0.5, 2.0, 3, 0, 1.0, 0, 0x4a3220);
  for (let i = 0; i < 12; i++) cyl(rackG, 0.06, 0.06, 0.35, 0.05, 0.4 + (i % 4) * 0.45, -1.2 + Math.floor(i / 4) * 1.1, 0x2a4a2a, 6).rotation.z = Math.PI / 2;
  ctx.col.box('below', -5.85, -5.35, -21, -18);
  crate(g, -4.6, y, -17, 0.8); crate(g, -3.6, y, -16.9, 0.7, 0.4); ctx.col.box('below', -5.0, -3.2, -17.4, -16.2);
  crate(g, 5.0, y, -17.0, 0.75); ctx.col.box('below', 4.6, 5.4, -17.4, -16.6);
  sack(g, -0.5, y, -23.4); sack(g, 0.2, y, -23.5); ctx.col.box('below', -0.9, 0.6, -23.9, -23.0);
  torch(g, 0, y + 1.8, -23.65, ctx);
  torch(g, 7.5, y + 1.8, -16.5, ctx, 3, 7);
  // tunnel
  torch(g, 1.2, y + 1.8, -6, ctx, 4, 8);
  torch(g, 9, y + 1.8, 4.3, ctx, 4, 8);
  crate(g, 3.05, y, -1.0, 0.5, 0.3); crate(g, 12.5, y, 2.35, 0.5, 0.8);
  ctx.col.box('below', 2.8, 3.3, -1.25, -0.75); ctx.col.box('below', 12.25, 12.75, 2.05, 2.6);
  // the note, on the floor of the tunnel
  box(g, 0.32, 0.02, 0.24, 2.25, y + 0.01, -8, 0xeee6d0, { cast: false });
  ctx.items.push({ id: 'note', name: 'A damp note', space: 'below', x: 2.25, z: -8, lines: [
    '"Third barrel from the left is not wine. Bertil knows. Mags does not need to."',
    'The handwriting is careful. The spelling is not.',
  ] });
  // cellar under the forge
  crate(g, 25, y, 6, 0.8); crate(g, 23.6, y, 7.1, 0.7, 0.5); crate(g, 25.4, y, 7.2, 0.6, 0.2);
  ctx.col.box('below', 23.1, 25.8, 5.5, 7.6);
  for (let i = 0; i < 8; i++) rock(g, 19 + rng() * 1.6, y + 0.15, 6.5 + rng() * 1.2, 0.25 + rng() * 0.15, 0x2a2826, rng);
  ctx.col.box('below', 18.6, 21, 6.2, 8);
  box(g, 1.6, 0.9, 0.7, 26.4, y + 0.45, 2.4, 0x5c3f27); ctx.col.box('below', 25.6, 27, 2.05, 2.75);
  torch(g, 26.7, y + 1.8, 4.5, ctx, 4, 8);
}

function buildCave(ctx, rng) {
  const g = ctx.groups.cave;
  const y = ctx.caveY;
  const rects = CAVE_ROOMS.map((r) => r.r);
  for (const room of CAVE_ROOMS) {
    ctx.col.walk('cave', room.r);
    slab(g, room.r, y, 0.5, 0x4a4540);
    for (const run of boundaryRuns(rects, room.r)) {
      const horiz = run.side === 'N' || run.side === 'S';
      const out = run.side === 'N' || run.side === 'W' ? -0.9 : 0.9;
      for (let s = run.s0; s <= run.s1; s += 1.3) {
        const px = horiz ? s : run.line + out, pz = horiz ? run.line + out : s;
        rock(g, px, y + 0.9, pz, 1.1 + rng() * 0.6, rng() > 0.5 ? 0x5e5953 : 0x6d6862, rng);
        if (rng() > 0.4) rock(g, px + (rng() - 0.5), y + 2.2, pz + (rng() - 0.5), 0.8 + rng() * 0.5, 0x55504a, rng);
      }
    }
  }
  // the troll, asleep on straw
  box(g, 9, 0.25, 5, -67, y + 0.12, -11, 0xc8a85a, { cast: false });
  for (let i = 0; i < 18; i++) box(g, 0.8 + rng(), 0.08, 0.12, -71 + rng() * 8, y + 0.28, -13.4 + rng() * 4.8, 0xd8b86a, { cast: false }).rotation.y = rng() * 3;
  ctx.col.box('cave', -71, -63, -13, -9);
  // campfire with a spit
  const fire = group(g, -62, y, -2);
  for (let i = 0; i < 7; i++) rock(fire, Math.cos(i * 0.9) * 0.75, 0.15, Math.sin(i * 0.9) * 0.75, 0.22, 0x5e5953, rng);
  const flame = new THREE.Mesh(new THREE.ConeGeometry(0.4, 0.9, 6), glow(0xff7a2a, 3));
  flame.position.y = 0.45; fire.add(flame);
  for (const sx of [-1, 1]) box(fire, 0.08, 1.3, 0.08, sx * 0.9, 0.65, 0, 0x3a2414);
  box(fire, 2.0, 0.06, 0.06, 0, 1.25, 0, 0x3a2414);
  const roast = new THREE.Mesh(new THREE.SphereGeometry(0.3, 6, 4), mat(0x8a4a2a));
  roast.scale.set(1.8, 1, 1); roast.position.y = 1.1; fire.add(roast);
  const fl = new THREE.PointLight(0xff8a3a, 10, 16, 1.4);
  fl.position.set(0, 1.4, 0); fire.add(fl);
  ctx.fires.push({ light: fl, base: 10, flame });
  ctx.col.circle('cave', -62, -2, 0.95);
  // crystals and mushrooms
  const crystalMat = glow(0x5ac8ff, 1.6);
  for (const [cx, cz] of [[-70.5, 2.5], [-71, 0.5], [-58, -15.5], [-57.5, -14]]) {
    for (let i = 0; i < 4; i++) {
      const c = new THREE.Mesh(new THREE.OctahedronGeometry(0.25 + rng() * 0.25, 0), crystalMat);
      c.position.set(cx + (rng() - 0.5) * 0.8, y + 0.4 + rng() * 0.4, cz + (rng() - 0.5) * 0.8);
      c.scale.y = 2; c.rotation.z = (rng() - 0.5) * 0.6; g.add(c);
    }
  }
  const cl = new THREE.PointLight(0x5ac8ff, 4, 10, 1.5);
  cl.position.set(-70.5, y + 1.5, 1.5); g.add(cl);
  const shroomMat = glow(0xb8ff7a, 1.2);
  for (let i = 0; i < 26; i++) {
    const sx = -71 + rng() * 14, sz = -16.5 + rng() * 20;
    if (inRect([-71.5, -62.5, -13.5, -8.5], sx, sz) || Math.hypot(sx + 62, sz + 2) < 1.6) continue;
    if (Math.abs(sz + 7) < 2 && sx > -64) continue;
    const s = new THREE.Mesh(new THREE.SphereGeometry(0.12 + rng() * 0.1, 6, 3, 0, Math.PI * 2, 0, Math.PI / 2), shroomMat);
    s.position.set(sx, y + 0.12, sz); g.add(s);
    box(g, 0.05, 0.12, 0.05, sx, y + 0.06, sz, 0xe8e0d0, { cast: false });
  }
  // bones and the hoard he is sleeping next to
  for (let i = 0; i < 10; i++) box(g, 0.5 + rng() * 0.4, 0.1, 0.1, -64 + rng() * 6, y + 0.05, -16 + rng() * 3, 0xe8e0c8, { cast: false }).rotation.y = rng() * 3;
  const coinMat = glow(0xd8b440, 0.5);
  for (let i = 0; i < 40; i++) {
    const c = new THREE.Mesh(new THREE.CylinderGeometry(0.1, 0.1, 0.03, 8), coinMat);
    c.position.set(-70.6 + rng() * 1.6, y + 0.05 + rng() * 0.5, -16.2 + rng() * 1.6);
    c.rotation.set(rng() * 3, 0, rng() * 3); g.add(c);
  }
  chest(g, -69.5, y, -15.6, 0.3);
  ctx.col.box('cave', -71, -68.8, -16.8, -14.8);
  // the outside of the mouth, seen from the village
  const a = ctx.groups.above;
  const my = terrainHeight(-50.3, -6);
  box(a, 0.4, 2.8, 3.2, -50.4, my + 1.2, -6, 0x0a0908, { cast: false });
  for (let i = 0; i < 9; i++) {
    const ang = (i / 8) * Math.PI;
    rock(a, -50.2, my + 0.3 + Math.sin(ang) * 2.9, -6 + Math.cos(ang) * 2.2, 0.7 + rng() * 0.4, 0x6d6862, rng);
  }
}

// ---------------------------------------------------------------- outdoors
function buildOutdoorProps(ctx, rng) {
  const a = ctx.groups.above;
  // the well
  const well = group(a, PLAZA.x, terrainHeight(PLAZA.x, PLAZA.z), PLAZA.z);
  cyl(well, 1.1, 1.2, 0.9, 0, 0.45, 0, 0x8d877c, 14);
  cyl(well, 0.85, 0.85, 0.92, 0, 0.47, 0, 0x1a2a3a, 14);
  for (const sx of [-1, 1]) box(well, 0.14, 2.0, 0.14, sx * 0.95, 1.3, 0, 0x4a3220);
  const wr = box(well, 2.6, 0.12, 1.6, 0, 2.35, 0, 0x8c3a28);
  wr.rotation.z = 0.02;
  box(well, 2.0, 0.08, 0.08, 0, 1.8, 0, 0x4a3220);
  ctx.col.circle('outdoor', PLAZA.x, PLAZA.z, 1.35);
  // market stalls
  for (const [sx, sz, color] of [[10, -4, 0xb83a3a], [-3, -5, 0x3a6ab8]]) {
    const st = group(a, sx, terrainHeight(sx, sz), sz);
    box(st, 2.4, 0.9, 1.4, 0, 0.45, 0, 0x7b5533);
    for (const px of [-1.1, 1.1]) for (const pz of [-0.6, 0.6]) box(st, 0.1, 2.2, 0.1, px, 1.1, pz, 0x4a3220);
    const aw = box(st, 2.7, 0.08, 1.8, 0, 2.25, 0, color);
    aw.rotation.x = 0.15;
    for (let i = 0; i < 6; i++) {
      const f = new THREE.Mesh(new THREE.SphereGeometry(0.13, 6, 4), mat([0xd84a2a, 0xe8c040, 0x6aa040][i % 3]));
      f.position.set(-0.9 + i * 0.36, 1.0, 0.1); st.add(f);
    }
    ctx.col.box('outdoor', sx - 1.2, sx + 1.2, sz - 0.8, sz + 0.8);
  }
  // lamp posts
  for (const [lx, lz] of [[0, 4], [8, 4]]) {
    const ly = terrainHeight(lx, lz);
    box(a, 0.14, 2.8, 0.14, lx, ly + 1.4, lz, 0x2a2420);
    box(a, 0.36, 0.42, 0.36, lx, ly + 2.95, lz, 0, { material: ctx.lampMat, cast: false });
    const l = new THREE.PointLight(0xffb060, 0, 12, 1.5);
    l.position.set(lx, ly + 2.9, lz); a.add(l);
    ctx.lamps.push(l);
    ctx.col.circle('outdoor', lx, lz, 0.2);
  }
  const bench = group(a, 4, terrainHeight(4, -4.5), -4.5);
  box(bench, 1.6, 0.08, 0.5, 0, 0.45, 0, 0x6b4a2b);
  box(bench, 1.6, 0.4, 0.08, 0, 0.7, -0.22, 0x6b4a2b);
  ctx.col.box('outdoor', 3.2, 4.8, -4.75, -4.25);
  // signpost on the west road
  const sy = terrainHeight(-30, 0.5);
  box(a, 0.14, 2.2, 0.14, -30, sy + 1.1, 0.5, 0x4a3220);
  box(a, 1.3, 0.3, 0.06, -30.5, sy + 1.8, 0.5, 0x8a6a44).rotation.y = 0.2;
  ctx.col.circle('outdoor', -30, 0.5, 0.2);
  // crates and hay outside the smithy and mill
  crate(a, 29.6, 0, 2.5, 0.8, 0.3); ctx.col.box('outdoor', 29.1, 30.1, 2.0, 3.0);
  for (const [hx, hz] of [[16.5, 31.5], [17.6, 31.2]]) {
    const hay = cyl(a, 0.55, 0.55, 1.0, hx, 0.5, hz, 0xd8b86a, 10);
    hay.rotation.z = Math.PI / 2;
  }
  ctx.col.box('outdoor', 15.9, 18.2, 30.6, 32.1);
  // fences along the south of the village
  for (let x = -14; x < 18; x += 2.2) {
    const fy = terrainHeight(x, 35);
    box(a, 0.12, 1.0, 0.12, x, fy + 0.5, 35, 0x6b4a2b);
    box(a, 2.2, 0.08, 0.06, x + 1.1, fy + 0.75, 35, 0x6b4a2b);
  }
}

function buildTrees(ctx, rng) {
  const spots = [];
  for (let i = 0; i < 9000 && spots.length < 760; i++) {
    const x = (rng() * 2 - 1) * 192, z = (rng() * 2 - 1) * 192;
    if (Math.abs(x - riverX(z)) < 9) continue;
    if (Math.hypot(x - MOONRING.x, z - MOONRING.z) < 14 || Math.hypot(x - HUT.x, z - HUT.z) < 10) continue;
    if (Math.hypot(x - 2, z - 2) < 24) continue;
    if (plotMask(x, z) > 0.02 || roadDist(x, z) < 3.6) continue;
    if (Math.hypot(x - MOUNTAIN.x, z - MOUNTAIN.z) < MOUNTAIN.r + 1.5) continue;
    if (x < -24 && z > -14 && z < 2) continue;
    if (spots.some((s) => Math.hypot(s[0] - x, s[1] - z) < 3.4)) continue;
    spots.push([x, z, 0.8 + rng() * 0.7, rng()]);
  }
  const pines = spots.filter((s) => s[3] < 0.55), rounds = spots.filter((s) => s[3] >= 0.55);
  const dummy = new THREE.Object3D();
  const color = new THREE.Color();
  const trunkGeo = new THREE.CylinderGeometry(0.18, 0.28, 1.6, 6);
  const trunks = new THREE.InstancedMesh(trunkGeo, mat(0x5a3d24), spots.length);
  const pineA = new THREE.InstancedMesh(new THREE.ConeGeometry(1.6, 2.6, 7), new THREE.MeshStandardMaterial({ roughness: 0.9, flatShading: true }), pines.length);
  const pineB = new THREE.InstancedMesh(new THREE.ConeGeometry(1.15, 2.0, 7), new THREE.MeshStandardMaterial({ roughness: 0.9, flatShading: true }), pines.length);
  const crowns = new THREE.InstancedMesh(new THREE.IcosahedronGeometry(1.5, 0), new THREE.MeshStandardMaterial({ roughness: 0.9, flatShading: true }), rounds.length);
  spots.forEach(([x, z, s], i) => {
    const y = terrainHeight(x, z);
    dummy.position.set(x, y + 0.8 * s, z); dummy.scale.setScalar(s); dummy.rotation.set(0, 0, 0); dummy.updateMatrix();
    trunks.setMatrixAt(i, dummy.matrix);
    ctx.col.circle('outdoor', x, z, 0.35 * s);
  });
  pines.forEach(([x, z, s, k], i) => {
    const y = terrainHeight(x, z);
    color.setHSL(0.3 + k * 0.05, 0.45, 0.22 + k * 0.08);
    dummy.scale.setScalar(s); dummy.rotation.set(0, k * 6, 0);
    dummy.position.set(x, y + 2.3 * s, z); dummy.updateMatrix(); pineA.setMatrixAt(i, dummy.matrix); pineA.setColorAt(i, color);
    dummy.position.set(x, y + 3.5 * s, z); dummy.updateMatrix(); pineB.setMatrixAt(i, dummy.matrix); pineB.setColorAt(i, color);
  });
  rounds.forEach(([x, z, s, k], i) => {
    const y = terrainHeight(x, z);
    color.setHSL(0.22 + (k - 0.55) * 0.2, 0.5, 0.3 + (k - 0.55) * 0.2);
    dummy.scale.set(s, s * 0.9, s); dummy.rotation.set(k * 3, k * 5, 0);
    dummy.position.set(x, y + 2.4 * s, z); dummy.updateMatrix(); crowns.setMatrixAt(i, dummy.matrix); crowns.setColorAt(i, color);
  });
  for (const m of [trunks, pineA, pineB, crowns]) { m.castShadow = true; m.receiveShadow = true; ctx.groups.above.add(m); }
  // boulders at the foot of the mountain
  for (let i = 0; i < 40; i++) {
    const ang = rng() * Math.PI * 2, rr = MOUNTAIN.block + 1 + rng() * 6;
    const x = MOUNTAIN.x + Math.cos(ang) * rr, z = MOUNTAIN.z + Math.sin(ang) * rr;
    if (x > -56 && z > -12 && z < 0) continue;
    if (roadDist(x, z) < 3) continue;
    rock(ctx.groups.above, x, terrainHeight(x, z) + 0.3, z, 0.6 + rng() * 0.9, 0x7a756f, rng);
  }
}

function buildTerrain(ctx) {
  const S = 400, N = 200;
  const geo = new THREE.PlaneGeometry(S, S, N, N);
  geo.rotateX(-Math.PI / 2);
  const pos = geo.attributes.position;
  const colors = new Float32Array(pos.count * 3);
  const c = new THREE.Color();
  const grass = new THREE.Color(0x5d8c3a), grass2 = new THREE.Color(0x7ca84c), dirt = new THREE.Color(0x9b7a50);
  const cobble = new THREE.Color(0x8d877c), stone = new THREE.Color(0x77716b), snow = new THREE.Color(0xeef2f5);
  for (let i = 0; i < pos.count; i++) {
    const x = pos.getX(i), z = pos.getZ(i);
    pos.setY(i, terrainHeight(x, z));
    c.copy(grass).lerp(grass2, vnoise(x * 0.15, z * 0.15));
    c.lerp(dirt, (1 - smoothstep(0.9, 2.1, roadDist(x, z))) * 0.9);
    c.lerp(cobble, 1 - smoothstep(5.5, 7.5, Math.hypot(x - PLAZA.x, z - PLAZA.z)));
    const mh = mountainHeight(x, z);
    c.lerp(stone, smoothstep(1.5, 6, mh));
    c.lerp(snow, smoothstep(16, 20, mh));
    colors[i * 3] = c.r; colors[i * 3 + 1] = c.g; colors[i * 3 + 2] = c.b;
  }
  geo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  geo.computeVertexNormals();
  const m = new THREE.Mesh(geo, new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.95, metalness: 0 }));
  m.receiveShadow = true;
  ctx.groups.above.add(m);
  // block the mountain for walkers; the cave mouth is handled by a space switch
  ctx.col.circle('outdoor', MOUNTAIN.x, MOUNTAIN.z, MOUNTAIN.block);
  return m;
}


// ---------------------------------------------------------------- prompt 4: the wider valley and the secrets
function buildRiver(ctx) {
  const a = ctx.groups.above;
  // water ribbon following the channel
  const pos = [], idx = [];
  let i = 0;
  for (let z = -WORLD_HALF; z <= WORLD_HALF; z += 4, i++) {
    const cx = riverX(z), y = terrainHeight(cx + 9, z) - 2.6;
    pos.push(cx - 6.5, y, z, cx + 6.5, y, z);
    if (i > 0) { const k = (i - 1) * 2; idx.push(k, k + 2, k + 1, k + 1, k + 2, k + 3); }
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  g.setIndex(idx);
  g.computeVertexNormals();
  const water = new THREE.Mesh(g, new THREE.MeshStandardMaterial({ color: 0x3a6a8a, roughness: 0.2, metalness: 0.1, transparent: true, opacity: 0.85, side: THREE.DoubleSide }));
  water.receiveShadow = true;
  a.add(water);
  // the bridge
  const b = ctx.bridge;
  const [x0, x1, z0, z1] = b.rect;
  box(a, x1 - x0, 0.3, z1 - z0, (x0 + x1) / 2, b.y - 0.15, (z0 + z1) / 2, 0x7a5a3a);
  for (const zz of [z0 + 0.1, z1 - 0.1]) {
    box(a, x1 - x0, 0.1, 0.1, (x0 + x1) / 2, b.y + 0.95, zz, 0x5a3d24);
    for (let x = x0 + 0.3; x < x1; x += 2.1) box(a, 0.14, 1.0, 0.14, x, b.y + 0.5, zz, 0x5a3d24);
  }
  for (const x of [x0 + 5.5, x1 - 5.5]) box(a, 0.5, 4, 0.5, x, b.y - 2.2, (z0 + z1) / 2, 0x6d665e);
  // the river blocks walking except across the deck
  ctx.col.block('outdoor', (x, z, r) => {
    if (Math.abs(x - riverX(z)) >= 5.5 + r) return false;
    return !(x >= x0 && x <= x1 && z - r >= z0 + 0.25 && z + r <= z1 - 0.25);
  });
}

function buildMoonring(ctx, rng) {
  const a = ctx.groups.above;
  const { x: cx, z: cz, r } = MOONRING;
  for (let k = 0; k < 8; k++) {
    const ang = ((k + 0.5) * Math.PI) / 4;
    const x = cx + Math.cos(ang) * r, z = cz + Math.sin(ang) * r;
    const st = box(a, 1.1, 3.2 + rng() * 1.2, 0.7, x, terrainHeight(x, z) + 1.7, z, 0x8a8680);
    st.rotation.y = -ang + Math.PI / 2;
    ctx.col.circle('outdoor', x, z, 0.75);
  }
  const ay = terrainHeight(cx, cz);
  box(a, 2, 0.6, 1.4, cx, ay + 0.3, cz, 0x6d6862);
  const rune = box(a, 1.2, 0.04, 0.6, cx, ay + 0.62, cz, 0, { material: glow(0xb08aff, 1.4), cast: false });
  ctx.col.circle('outdoor', cx, cz, 1.0);
  const l = new THREE.PointLight(0xb08aff, 3, 10, 1.6);
  l.position.set(cx, ay + 1.5, cz); a.add(l);
  ctx.story.altar = { rune, light: l };
}

function buildWillowbank(ctx) {
  const a = ctx.groups.above;
  const { x, z, w, d } = HUT;
  const y = terrainHeight(x, z);
  box(a, w, 2.6, d, x, y + 1.3, z, 0x8a6a44);
  const roof = box(a, w + 0.8, 0.25, d + 1.2, x, y + 3.0, z, 0x5e6b3f);
  roof.rotation.z = 0.12;
  box(a, 1, 1.9, 0.1, x - 1, y + 0.95, z + d / 2 + 0.05, 0x3a2414);
  ctx.col.box('outdoor', x - w / 2, x + w / 2, z - d / 2, z + d / 2);
  // jetty and a rowing boat
  const jx = riverX(-17) + 4.2;
  box(a, 3, 0.15, 1.2, jx, terrainHeight(jx + 3, -17) - 0.4, -17, 0x6b4a2b);
  const boat = box(a, 2.6, 0.4, 1, riverX(-21) + 2, terrainHeight(riverX(-21) + 9, -21) - 2.4, -21, 0x7a4a2a);
  boat.rotation.y = 0.3;
  for (let i = 0; i < 6; i++) crate(a, x + 4 + (i % 2) * 0.8, terrainHeight(x + 4, z), z - 1 + i * 0.4, 0.5, i);
}

function buildSecrets(ctx, rng) {
  const g = ctx.groups.below, y = BELOW_Y, st = ctx.story;
  // the false wall between the tunnel and the Keeper's Cellar, with a glyph only Glimmer shows
  const wall = box(g, 1.6, 3.1, 0.9, 5.8, y + 1.55, 1.45, 0x5a4c3e);
  const glyph = box(g, 0.9, 0.9, 0.02, 5.8, y + 1.5, 1.91, 0, { material: glow(0x5ac8ff, 2.2), cast: false });
  glyph.visible = false;
  st.falseWall = { mesh: wall, col: ctx.col.box('below', 5, 6.6, 1.0, 1.9) };
  ctx.glimmer.push(glyph);
  // the Keeper's Cradle: the mechanism that lifts light into the well
  const cr = group(g, 7.5, y, -3.2);
  cyl(cr, 0.6, 0.8, 0.9, 0, 0.45, 0, 0x6d665e, 10);
  const ring = new THREE.Mesh(new THREE.TorusGeometry(0.45, 0.06, 6, 18), mat(0xc9a646));
  ring.rotation.x = Math.PI / 2; ring.position.y = 1.0; cr.add(ring);
  for (let i = 0; i < 3; i++) cyl(cr, 0.06, 0.06, 2.4, Math.cos(i * 2.1) * 0.5, 2.1, Math.sin(i * 2.1) * 0.5, 0xb8902e, 6);
  const heartSlot = new THREE.Mesh(new THREE.OctahedronGeometry(0.35, 0), glow(0xffe08a, 2.5));
  heartSlot.position.y = 1.4; heartSlot.scale.y = 1.6; heartSlot.visible = false; cr.add(heartSlot);
  const splinter = new THREE.Mesh(new THREE.OctahedronGeometry(0.1, 0), glow(0x5a7a8a, 0.5));
  splinter.position.y = 1.1; cr.add(splinter);
  ctx.col.box('below', 7, 8, -3.7, -2.7);
  for (let i = 0; i < 5; i++) crate(g, 8.6, y, -4.4 + i * 0.75, 0.55, i * 0.4);
  ctx.col.box('below', 8.25, 9.5, -4.8, -0.9);
  box(g, 0.06, 0.25, 0.4, 5.2, y + 0.9, -4.6, 0xeee6d0, { cast: false });
  torch(g, 4.8, y + 1.8, -1.0, ctx, 3, 8);
  st.cradle = { heart: heartSlot, splinter };
  // crystal smuggled into the forge cellar, packed in coal
  const cmat = glow(0x5ac8ff, 1.4);
  for (let i = 0; i < 9; i++) {
    const c = new THREE.Mesh(new THREE.OctahedronGeometry(0.12 + rng() * 0.08, 0), cmat);
    c.position.set(23.4 + rng() * 2.2, y + 0.75 + rng() * 0.25, 5.7 + rng() * 1.7); c.scale.y = 1.8; g.add(c);
  }
  // cave: the root curtain, the passage, the hollow, the tablet, the heart crystal
  const cg = ctx.groups.cave, cy = ctx.caveY;
  const roots = new THREE.Group(); cg.add(roots);
  for (let i = 0; i < 16; i++) {
    const r = cyl(roots, 0.07, 0.12, 3.2, -65.8 + i * 0.18 + rng() * 0.1, cy + 1.6, 5.85 + (rng() - 0.5) * 0.3, 0x5a3d24, 5);
    r.rotation.z = (rng() - 0.5) * 0.4;
  }
  st.roots = { group: roots, col: ctx.col.box('cave', -66, -63, 5.5, 6.2) };
  const tab = group(cg, -64.5, cy, 16.9);
  box(tab, 2.2, 2.4, 0.4, 0, 1.2, 0, 0x6d6862);
  box(tab, 1.8, 1.8, 0.05, 0, 1.3, 0.21, 0x8a8478, { cast: false });
  for (let i = 0; i < 6; i++) box(tab, 1.3 - (i % 2) * 0.3, 0.06, 0.06, 0, 0.75 + i * 0.25, 0.25, 0x3a3630, { cast: false });
  ctx.col.box('cave', -65.7, -63.3, 16.6, 17.2);
  const ped = group(cg, -68.5, cy, 13.5);
  cyl(ped, 0.45, 0.6, 1.0, 0, 0.5, 0, 0x5e5953, 8);
  const heart = new THREE.Mesh(new THREE.OctahedronGeometry(0.4, 0), glow(0xffe08a, 2.2));
  heart.position.y = 1.45; heart.scale.y = 1.6; ped.add(heart);
  const hl = new THREE.PointLight(0xffd070, 4, 9, 1.5); hl.position.set(0, 1.8, 0); ped.add(hl);
  ctx.col.box('cave', -69.1, -67.9, 12.9, 14.1);
  st.heart = { mesh: heart, light: hl };
  // the beam that rises from the well when the light comes back
  const beam = new THREE.Mesh(new THREE.CylinderGeometry(0.7, 1.1, 60, 16, 1, true),
    new THREE.MeshBasicMaterial({ color: 0xffe2a0, transparent: true, opacity: 0.35, blending: THREE.AdditiveBlending, depthWrite: false, side: THREE.DoubleSide }));
  beam.position.set(PLAZA.x, terrainHeight(PLAZA.x, PLAZA.z) + 30, PLAZA.z);
  beam.visible = false;
  ctx.groups.above.add(beam);
  st.beam = beam;
}

// ---------------------------------------------------------------- entry point
export function buildWorld(scene, seed = 20261031) {
  const rng = mulberry32(seed);
  const groups = { above: new THREE.Group(), below: new THREE.Group(), cave: new THREE.Group() };
  for (const g of Object.values(groups)) scene.add(g);
  const ctx = {
    col: new Collision(), groups, fires: [], lamps: [], items: [],
    lampMat: glow(0xffc070, 0.2),
    caveY: terrainHeight(CAVE.enterX, -6),
    story: {}, glimmer: [],
  };
  const bx = riverX(-10.8);
  const bRect = [bx - 8.5, bx + 8.5, -12.6, -9.0];
  ctx.bridge = { rect: bRect, y: Math.max(terrainHeight(bRect[0], -10.8), terrainHeight(bRect[1], -10.8)) + 0.2 };
  ctx.col.space('outdoor');
  buildTerrain(ctx);
  for (const b of BUILDINGS) buildBuilding(b, ctx);
  furnishTavern(BUILDINGS[0], ctx, rng);
  furnishSmithy(BUILDINGS[1], ctx, rng);
  furnishWeaver(BUILDINGS[2], ctx, rng);
  furnishBakery(BUILDINGS[3], ctx, rng);
  furnishArchive(BUILDINGS[4], ctx, rng);
  furnishMill(BUILDINGS[5], ctx, rng);
  buildBelow(ctx, rng);
  buildCave(ctx, rng);
  buildOutdoorProps(ctx, rng);
  buildRiver(ctx);
  buildMoonring(ctx, rng);
  buildWillowbank(ctx);
  buildSecrets(ctx, rng);
  buildTrees(ctx, rng);
  return { ...ctx, buildings: BUILDINGS, rng };
}

export function roomName(space, x, z) {
  const list = space === 'below' ? BELOW_ROOMS : space === 'cave' ? CAVE_ROOMS : [];
  return list.find((r) => inRect(r.r, x, z))?.name ?? null;
}

// Lantern river simulation: pure 2D (x along the river, z across it), no three.js.
// The page renders it; selfTest runs it headless with different "breath" policies.
import { rng } from '../_shared/gc.js';

export const HW = 3.0;                       // half width of the river
export const center = (x) => 2.2 * Math.sin(x / 7);
export const slope = (x) => (2.2 / 7) * Math.cos(x / 7);
export const COVE = { x: 6, r: 2.3 };
COVE.z = center(COVE.x) - HW - 1.7;
export const OPEN = { x0: COVE.x - 1.7, x1: COVE.x + 1.7 };
export const ROCKS = [{ x: -9, z: center(-9) + 0.8, r: 0.55 }, { x: 0.5, z: center(0.5) - 1.2, r: 0.6 }, { x: 13, z: center(13) + 0.4, r: 0.7 }];
export const SPAWN_X = -22, END_X = 24;
export const GOAL = 12;
export const GUST_R = 2.6, GUST_F = 7.5;
const FLOW = 1.05;

export function createSim(seed, every = 2.6) {
  return { r: rng(seed), t: 0, next: 0.5, every, lanterns: [], id: 0, home: 0, passed: 0, spawned: 0, gust: null };
}
const inCove = (l) => Math.hypot(l.x - COVE.x, l.z - COVE.z) < COVE.r && l.z < center(l.x) - HW + 0.25;

export function step(s, dt) {
  s.t += dt;
  if (s.t >= s.next) {
    s.next = s.t + s.every * s.r.range(0.8, 1.25);
    s.lanterns.push({ id: s.id++, x: SPAWN_X, z: center(SPAWN_X) + s.r.range(-1.8, 1.8), vx: FLOW, vz: 0, home: false, gone: false, born: s.t, bob: s.r.range(0, 6) });
    s.spawned++;
  }
  for (const l of s.lanterns) {
    if (l.gone) continue;
    if (l.home) {
      // calm water: drift slowly toward a resting ring around the cove centre
      const a = (l.id * 2.39996) % (Math.PI * 2), rr = 0.5 + (l.id % 3) * 0.55;
      const tx = COVE.x + Math.cos(a) * rr, tz = COVE.z + Math.sin(a) * rr * 0.8;
      l.vx += ((tx - l.x) * 0.6 - l.vx) * Math.min(1, dt * 1.5);
      l.vz += ((tz - l.z) * 0.6 - l.vz) * Math.min(1, dt * 1.5);
      l.x += l.vx * dt; l.z += l.vz * dt;
      continue;
    }
    const c = center(l.x);
    // the current: along the meander, gently toward mid-stream
    const fx = FLOW, fz = slope(l.x) * FLOW + (c - l.z) * 0.12;
    l.vx += (fx - l.vx) * Math.min(1, dt * 0.9);
    l.vz += (fz - l.vz) * Math.min(1, dt * 0.9);
    if (s.gust) {
      const dx = l.x - s.gust.x, dz = l.z - s.gust.z, d = Math.hypot(dx, dz);
      if (d < GUST_R && d > 1e-3) { const f = GUST_F * (1 - d / GUST_R); l.vx += (dx / d) * f * dt; l.vz += (dz / d) * f * dt; }
    }
    const sp = Math.hypot(l.vx, l.vz); if (sp > 4) { l.vx *= 4 / sp; l.vz *= 4 / sp; }
    l.x += l.vx * dt; l.z += l.vz * dt;
    for (const rk of ROCKS) {
      const dx = l.x - rk.x, dz = l.z - rk.z, d = Math.hypot(dx, dz), min = rk.r + 0.3;
      if (d < min) { l.x = rk.x + (dx / d) * min; l.z = rk.z + (dz / d) * min; const vn = (l.vx * dx + l.vz * dz) / d; if (vn < 0) { l.vx -= 1.4 * vn * dx / d; l.vz -= 1.4 * vn * dz / d; } }
    }
    const cc = center(l.x), north = cc - HW + 0.3, south = cc + HW - 0.3;
    const opening = l.x > OPEN.x0 && l.x < OPEN.x1;
    if (l.z > south) { l.z = south; l.vz = Math.min(0, l.vz); }
    if (l.z < north && !opening) { l.z = north; l.vz = Math.max(0, l.vz); }
    if (l.z < north && opening) {
      if (inCove(l)) { l.home = true; s.home++; }
      else { const dx = l.x - COVE.x, dz = l.z - COVE.z, d = Math.hypot(dx, dz); if (d >= COVE.r) { l.x = COVE.x + dx / d * (COVE.r - 0.01); l.z = COVE.z + dz / d * (COVE.r - 0.01); } }
    }
    if (l.x > END_X) { l.gone = true; s.passed++; }
  }
  s.lanterns = s.lanterns.filter((l) => !l.gone);
}

// ------------------------------------------------------------------ policies
/** What a thoughtful player does: breathe on the lantern nearest the cove from the river side. */
export function greedyGust(s) {
  const cand = s.lanterns.filter((l) => !l.home && l.x > COVE.x - 4.5 && l.x < COVE.x + 0.8).sort((a, b) => b.x - a.x)[0];
  if (!cand) return null;
  return { x: cand.x - 0.5, z: cand.z + 1.1 };
}
export function randomGust(s) {
  if (s.r() < 0.5) return null;
  return { x: s.r.range(-15, 15), z: s.r.range(-3, 3) };
}

export function runPolicy(seed, policy, spawns) {
  const s = createSim(seed);
  const dt = 1 / 30;
  while (s.spawned < spawns || s.lanterns.some((l) => !l.home)) {
    s.gust = policy ? policy(s) : null;
    step(s, dt);
    if (s.t > 2000) break;
  }
  return { home: s.home, passed: s.passed, spawned: s.spawned };
}

export function selfTest() {
  const seeds = [1, 2, 3, 4, 5];
  const N = 30;
  const tally = { idle: 0, random: 0, greedy: 0 }, born = { idle: 0, random: 0, greedy: 0 };
  for (const sd of seeds) {
    for (const [k, pol] of [['idle', null], ['random', (s) => randomGust(s)], ['greedy', greedyGust]]) {
      const res = runPolicy(sd, pol, N); tally[k] += res.home; born[k] += res.spawned;
    }
  }
  const rate = (k) => tally[k] / born[k];
  const total = seeds.length * N;
  const checks = {
    current_alone_rarely_brings_lanterns_home: rate('idle') <= 0.1,
    random_breathing_is_not_enough: rate('random') < 0.5,
    careful_breathing_works: rate('greedy') >= 0.8,
    careful_beats_random: rate('greedy') > rate('random') * 1.5,
    nothing_is_ever_lost: true,
  };
  // conservation: every spawned lantern is either home or floated on
  const s = createSim(9); for (let i = 0; i < 6000; i++) { s.gust = greedyGust(s); step(s, 1 / 30); }
  checks.nothing_is_ever_lost = s.home + s.passed + s.lanterns.filter((l) => !l.home).length === s.spawned;
  return { pass: Object.values(checks).every(Boolean), checks, summary: `about ${total} lanterns per policy reached the shrine: idle ${Math.round(rate('idle') * 100)}%, random ${Math.round(rate('random') * 100)}%, careful ${Math.round(rate('greedy') * 100)}%` };
}

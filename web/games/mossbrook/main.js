// Mossbrook: top-down (angled) village RPG engine demo.
// Movement, companions, NPCs, cutaway walls, floors, cellars, cave, HUD,
// and window.__gamecraft: the deterministic self-play hook the verifier drives.

import * as THREE from 'three';
import * as W from './world.js';

const { BUILDINGS, STAIRS, CAVE, clamp, lerp, inRect, stairT } = W;
const params = new URLSearchParams(location.search);
const VERIFY = params.has('verify');
const DT = 1 / 60;
const byId = (id) => BUILDINGS.find((b) => b.id === id);

// ---------------------------------------------------------------- renderer, scene, light
const canvas = document.getElementById('view');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: !VERIFY, preserveDrawingBuffer: true });
renderer.setPixelRatio(VERIFY ? 1 : Math.min(window.devicePixelRatio, 2));
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFShadowMap;
const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(40, 1, 0.5, 400);
const sun = new THREE.DirectionalLight(0xffffff, 2);
sun.castShadow = true;
sun.shadow.mapSize.set(2048, 2048);
Object.assign(sun.shadow.camera, { left: -34, right: 34, top: 34, bottom: -34, near: 1, far: 170 });
sun.shadow.bias = -0.0006;
sun.shadow.normalBias = 0.03;
scene.add(sun, sun.target);
const hemi = new THREE.HemisphereLight(0xcfe3ff, 0x6b5a40, 1);
scene.add(hemi);
scene.background = new THREE.Color(0x9cc6ea);
scene.fog = new THREE.Fog(0x9cc6ea, 80, 170);

const world = W.buildWorld(scene);
for (const b of BUILDINGS) for (const key of ['S0', 'S1', 'roof']) for (const m of b.fades[key]) m.material.transparent = true;

const TIMES = {
  dawn: { label: 'Dawn', clock: 6 * 60 + 10, sun: [-0.7, 0.45, 0.5], sunColor: 0xffc49a, sunI: 1.7, sky: 0xe7b9a0, hemiSky: 0xb8c7e0, hemiGround: 0x5b4a3a, hemiI: 0.9, windows: 0.5, lamps: 0 },
  noon: { label: 'Noon', clock: 12 * 60 + 30, sun: [0.3, 1, 0.4], sunColor: 0xfff4e0, sunI: 2.6, sky: 0x9cc6ea, hemiSky: 0xcfe3ff, hemiGround: 0x6b5a40, hemiI: 1.1, windows: 0, lamps: 0 },
  dusk: { label: 'Dusk', clock: 18 * 60 + 40, sun: [0.8, 0.32, 0.45], sunColor: 0xff9a5a, sunI: 1.6, sky: 0xd98a6a, hemiSky: 0x9aa0c8, hemiGround: 0x4a3528, hemiI: 0.8, windows: 1.3, lamps: 6 },
  night: { label: 'Night', clock: 23 * 60 + 15, sun: [-0.5, 0.8, -0.3], sunColor: 0x8aa0ff, sunI: 0.35, sky: 0x0d1424, hemiSky: 0x2a3550, hemiGround: 0x15120f, hemiI: 0.45, windows: 2.0, lamps: 9 },
};
let timeKey = params.get('time') in TIMES ? params.get('time') : 'dusk';
let clockMinutes = TIMES[timeKey].clock;

// ---------------------------------------------------------------- characters
function makeCharacter(o) {
  const g = new THREE.Group();
  const body = new THREE.Group();
  g.add(body);
  const w = o.wide ?? 1;
  const add = (p, bw, bh, bd, c, x, y, z) => W.box(p, bw, bh, bd, x, y, z, c);
  const legL = new THREE.Group(), legR = new THREE.Group();
  legL.position.set(-0.13 * w, 0.78, 0);
  legR.position.set(0.13 * w, 0.78, 0);
  for (const leg of [legL, legR]) {
    add(leg, 0.19, 0.62, 0.21, o.legs, 0, -0.33, 0);
    add(leg, 0.21, 0.16, 0.28, o.boots ?? 0x3a2a1e, 0, -0.7, 0.03);
    body.add(leg);
  }
  add(body, 0.52 * w, 0.62, 0.3, o.body, 0, 1.08, 0);
  add(body, 0.54 * w, 0.08, 0.32, 0x3a2a1e, 0, 0.8, 0);
  if (o.apron) add(body, 0.44 * w, 0.62, 0.04, o.apron, 0, 0.88, 0.17);
  if (o.cloak) add(body, 0.56 * w, 0.95, 0.06, o.cloak, 0, 1.02, -0.19);
  if (o.armour) { add(body, 0.6 * w, 0.2, 0.36, o.armour, 0, 1.36, 0); add(body, 0.56 * w, 0.3, 0.34, o.armour, 0, 1.1, 0.01); }
  const armL = new THREE.Group(), armR = new THREE.Group();
  armL.position.set(-0.34 * w, 1.34, 0);
  armR.position.set(0.34 * w, 1.34, 0);
  for (const arm of [armL, armR]) {
    add(arm, 0.15, 0.58, 0.17, o.sleeves ?? o.body, 0, -0.28, 0);
    add(arm, 0.13, 0.13, 0.13, o.skin, 0, -0.62, 0);
    body.add(arm);
  }
  const head = new THREE.Group();
  head.position.y = 1.42;
  body.add(head);
  add(head, 0.36, 0.38, 0.36, o.skin, 0, 0.2, 0);
  for (const sx of [-0.08, 0.08]) add(head, 0.06, 0.06, 0.02, 0x1a1a1a, sx, 0.23, 0.185);
  add(head, 0.4, 0.12, 0.4, o.hair, 0, 0.42, 0);
  add(head, 0.4, 0.3, 0.1, o.hair, 0, 0.28, -0.16);
  if (o.longHair) add(head, 0.34, 0.55, 0.08, o.hair, 0, 0.05, -0.2);
  if (o.beard) add(head, 0.3, 0.22, 0.08, o.beard, 0, 0.06, 0.18);
  if (o.helmet) { add(head, 0.44, 0.22, 0.44, o.helmet, 0, 0.44, 0); add(head, 0.06, 0.3, 0.06, o.helmet, 0, 0.2, 0.2); }
  if (o.hood) add(head, 0.44, 0.44, 0.44, o.hood, 0, 0.3, -0.03);
  if (o.staff) {
    add(armR, 0.06, 1.8, 0.06, 0x6b4a2b, 0, -0.45, 0.1);
    const gem = W.box(armR, 0.14, 0.14, 0.14, 0, 0.5, 0.1, 0, { material: W.glow(0x7ad8ff, 2) });
    gem.rotation.set(0.6, 0.6, 0);
  }
  if (o.bow) { const bow = add(body, 0.06, 1.2, 0.06, 0x6b4a2b, 0.1, 1.15, -0.25); bow.rotation.z = 0.5; }
  if (o.hammer) { add(armR, 0.07, 0.8, 0.07, 0x5c3f27, 0, -0.6, 0.12); add(armR, 0.22, 0.2, 0.34, 0x8a9098, 0, -0.95, 0.12); }
  g.scale.setScalar(o.scale ?? 1);
  return { group: g, body, legL, legR, armL, armR, head, phase: 0 };
}
function animateWalk(ch, speed, dt) {
  const moving = speed > 0.2;
  if (moving) ch.phase += dt * (2.2 + speed * 1.6);
  const amp = moving ? Math.min(0.7, 0.25 + speed * 0.09) : 0;
  const s = Math.sin(ch.phase);
  const k = Math.min(1, dt * 14);
  ch.legL.rotation.x = lerp(ch.legL.rotation.x, s * amp, k);
  ch.legR.rotation.x = lerp(ch.legR.rotation.x, -s * amp, k);
  ch.armL.rotation.x = lerp(ch.armL.rotation.x, -s * amp * 0.8, k);
  ch.armR.rotation.x = lerp(ch.armR.rotation.x, s * amp * 0.8, k);
  ch.body.position.y = moving ? Math.abs(Math.cos(ch.phase)) * 0.06 : lerp(ch.body.position.y, 0, k);
}
function sitPose(ch) {
  ch.legL.rotation.x = -1.45; ch.legR.rotation.x = -1.45;
  ch.armL.rotation.x = -0.5; ch.armR.rotation.x = -0.5;
  ch.body.position.y = -0.33;
}

function entity(o) {
  const ch = makeCharacter(o.look);
  const e = { name: o.name, x: o.x, z: o.z, y: 0, space: o.space ?? 'outdoor', rot: o.rot ?? Math.PI, r: o.r ?? 0.32, speed: 0, ch, ...o.extra };
  e.y = W.floorY(world, e.space, e.x, e.z);
  return e;
}

// ---------------------------------------------------------------- movement
function resolveSpace(space, x, z) {
  for (const s of STAIRS) {
    if ((space === s.low || space === s.high) && inRect(s.rect, x, z)) return stairT(s, x, z) > 0.5 ? s.high : s.low;
  }
  if (space === 'outdoor' && x < CAVE.enterX && z > CAVE.z0 && z < CAVE.z1) return 'cave';
  if (space === 'cave' && x > CAVE.exitX) return 'outdoor';
  return space;
}
function heightAt(space, x, z) {
  for (const s of STAIRS) {
    if ((space === s.low || space === s.high) && inRect(s.rect, x, z)) return lerp(s.yLow, s.yHigh, stairT(s, x, z));
  }
  return W.floorY(world, space, x, z);
}
function canStandAt(e, x, z) {
  return world.col.canStand(resolveSpace(e.space, x, z), x, z, e.r);
}
// Try the full step, then each axis alone, so walls slide instead of stick.
function step(e, dx, dz, dist) {
  const nx = e.x + dx * dist, nz = e.z + dz * dist;
  let moved = true;
  if (canStandAt(e, nx, nz)) { e.x = nx; e.z = nz; }
  else if (Math.abs(dx) > 0.01 && canStandAt(e, nx, e.z)) e.x = nx;
  else if (Math.abs(dz) > 0.01 && canStandAt(e, e.x, nz)) e.z = nz;
  else moved = false;
  e.space = resolveSpace(e.space, e.x, e.z);
  e.y = heightAt(e.space, e.x, e.z);
  return moved;
}
function turnTowards(e, dx, dz, dt) {
  const target = Math.atan2(dx, dz);
  let d = target - e.rot;
  while (d > Math.PI) d -= Math.PI * 2;
  while (d < -Math.PI) d += Math.PI * 2;
  e.rot += d * Math.min(1, dt * 12);
}
function viewOf(space) {
  return space === 'below' ? 'below' : space === 'cave' ? 'cave' : 'above';
}

// ---------------------------------------------------------------- the party
const player = entity({ name: 'You', x: 2, z: 5, rot: Math.PI, look: { skin: 0xe8b894, body: 0x3a5a8a, legs: 0x4a3a2a, hair: 0x6a3a1a, cloak: 0x2a3a5a } });
const companions = [
  entity({ name: 'Tamsin', x: 2, z: 6.5, look: { skin: 0xf0c8a8, body: 0x3f6a3a, legs: 0x4a3a2a, hair: 0xd8b050, longHair: true, cloak: 0x2f4a2a, bow: true } }),
  entity({ name: 'Odo', x: 2, z: 8, look: { skin: 0xd8a07a, body: 0x6a2626, legs: 0x4a4a52, hair: 0x3a2a1a, beard: 0x3a2a1a, armour: 0x8a2d2d, hammer: true, wide: 1.15 } }),
];
companions[0].bark = { tavern: 'Smells like stew and spilled cider.', 'tavern-upper': 'Creaky boards. Good for hearing who comes up.', cellar: 'Cold down here. Cold and organised.', tunnel: 'Nobody digs a tunnel this straight for turnips.', cave: 'Quiet now. Something big is breathing.', troll: 'He is asleep. Let us keep it that way.' };
companions[1].bark = { tavern: 'First round is mine. Second round is also mine.', 'tavern-upper': 'Mind the low beam. Ow. That one.', cellar: 'Barrels. Good sign.', tunnel: 'I can hear the forge through the stone.', smithy: 'Bertil still owes me a buckle.', cave: 'Is that roast mutton? Is that our roast mutton?', troll: 'I could take him. I will not, but I could.' };
for (const e of [player, ...companions]) scene.add(e.ch.group);
const trail = [];
for (let i = 14; i >= 0; i--) trail.push({ x: 2, z: 5 + i * 0.3, y: player.y, space: 'outdoor' });

// ---------------------------------------------------------------- villagers
const NPC_DEFS = [
  { name: 'Mags', role: 'innkeeper', x: 1, z: -21, wander: [-1.7, 3.7, -21, -21], look: { skin: 0xe0b090, body: 0x7a4a2a, legs: 0x5a3a2a, hair: 0xa04020, apron: 0xe8e0d0, longHair: true },
    lines: ['Welcome to the Copper Kettle. Boots off the tables, coin on the counter.', 'Rooms upstairs are clean. The cellar is mine, so stay out of it.', 'Bertil? He comes in for one cider and leaves at closing. Every night.'],
    barks: ['Stew is on!', 'Mind the step.', 'Who left a mug by the fire?'] },
  { name: 'Corwin', role: 'regular', x: 5.6, z: -13, fixed: true, sit: true, rot: -Math.PI / 2, look: { skin: 0xd8b090, body: 0x5a5a3a, legs: 0x4a3a2a, hair: 0xc8c8c0, beard: 0xc8c8c0 },
    lines: ['Forty years I have sat in this chair. The chair has held up better than me.', 'They say the troll up in Knucklehollow only wakes for thunder and bad singing.', 'If you go up the crag, leave the cheese at home. Trust me on that.'],
    barks: ['Hmph.', 'Fire needs a log.', 'In my day the troll was smaller.'] },
  { name: 'Ysolde', role: 'cartographer', space: 'upper:tavern', x: 3, z: -18.5, wander: [1.2, 4.8, -20.2, -17.2], look: { skin: 0xc89070, body: 0x3a4a6a, legs: 0x2a2a3a, hair: 0x1a1a1a, longHair: true, cloak: 0x6a5a3a },
    lines: ['I am mapping the crag. The maps keep disagreeing with the crag.', 'There is a draught in this room that does not come from the window. Odd.', 'Your village has more cellar than village, if my measurements are right.'],
    barks: ['North is that way. Probably.', 'Where did I put my compass?'] },
  { name: 'Bertil', role: 'smith', x: 19.8, z: 6, wander: [18.6, 21, 5, 7], look: { skin: 0xc8906a, body: 0x5a4a3a, legs: 0x3a3a3a, hair: 0x2a1a10, beard: 0x2a1a10, apron: 0x3a2a1e, wide: 1.2, hammer: true },
    lines: ['Horseshoes, hinges, the odd sword. Mostly hinges.', 'The cellar? Coal. Just coal. Lots of coal. Why do you ask?', 'If you are going to the cave, take a torch and a fast pair of legs.'],
    barks: ['Clang.', 'Too hot. Too hot!', 'Where is my good tongs?'] },
  { name: 'Ada Thorne', role: 'weaver', x: -20, z: 5.5, wander: [-21, -19, 4.6, 6.4], look: { skin: 0xf0d0b0, body: 0x8a5aa0, legs: 0x4a3a4a, hair: 0x9a9a9a, longHair: true },
    lines: ['Every cloak in Mossbrook came off that loom. Including yours, by the look of it.', 'Upstairs is my bedroom, dear. Not a tavern.', 'The troll? I knitted him a scarf once. He ate it.'],
    barks: ['Clack, clack, clack.', 'Out of blue thread again.'] },
  { name: 'Pim', role: 'baker', x: -5, z: 28, wander: [-8, -3, 27.2, 29], look: { skin: 0xf0c0a0, body: 0xe8e0d0, legs: 0x6a5a4a, hair: 0xd89040, apron: 0xf8f4ec },
    lines: ['Rye, honey loaf, or the one that might be a pie. Your pick.', 'Up before dawn every day. Bread does not bake itself, more is the pity.', 'Odo owes me for eleven buns. Tell him I am counting.'],
    barks: ['Fresh loaves!', 'Still warm!', 'Mind the flour.'] },
  { name: 'Sister Wren', role: 'archivist', x: 24, z: -19, wander: [22.4, 25.6, -20.4, -17.6], look: { skin: 0xe8c0a0, body: 0x2a3a4a, legs: 0x2a3a4a, hair: 0x2a3a4a, hood: 0x2a3a4a },
    lines: ['The Lantern Archive holds every deed, debt and dispute since the founding.', 'There is a ledger here that lists one more cellar than Mossbrook admits to having.', 'Quietly, please. The books are asleep.'],
    barks: ['Shh.', 'Page forty, page forty.'] },
  { name: 'Lottie', role: 'child', x: 6, z: 4, wander: [-2, 10, -6, 8], speed: 3.4, look: { skin: 0xf0c8a8, body: 0xc8504a, legs: 0x4a3a2a, hair: 0x6a3a1a, scale: 0.7 },
    lines: ['Are you a hero? You do not look like a hero. You look like a person.', 'I went up to the cave once. Well, near it. Well, I looked at it.', 'Tamsin let me hold her bow and I did not even break it.'],
    barks: ['Tag!', 'Bet you cannot catch me!', 'Hee hee!'] },
  { name: 'Captain Osk', role: 'watch', x: -36, z: -1.5, fixed: true, rot: Math.PI / 2, look: { skin: 0xd0a080, body: 0x4a5a6a, legs: 0x3a3a3a, hair: 0x3a3a3a, helmet: 0x8a9098, armour: 0x6a7480, beard: 0x5a5a5a },
    lines: ['The cave mouth is just up the road. Old Knuckles sleeps in there.', 'Nobody goes in. That is the rule. You look like you are going in.', 'If you wake him, run downhill. Trolls are bad at downhill.'],
    barks: ['Halt. Or do not. I am not your mother.', 'Quiet road today.'] },
  { name: 'Hob', role: 'miller', x: 10, z: 32.5, wander: [6, 14, 31.6, 33.4], look: { skin: 0xd8b090, body: 0x8a7a5a, legs: 0x5a4a3a, hair: 0xe8e0d0, beard: 0xe8e0d0, apron: 0xd8d0b8 },
    lines: ['Flour in my hair, flour in my beard, flour in my tea.', 'The wheel is down by the stream. I just keep the sacks.'],
    barks: ['Sacks, sacks, sacks.'] },
];
const npcRng = W.mulberry32(77);
const npcs = NPC_DEFS.map((d) => {
  const e = entity({ name: d.name, x: d.x, z: d.z, space: d.space, rot: d.rot ?? 0, r: 0.28, look: d.look, extra: { def: d } });
  e.wait = npcRng() * 3;
  e.barkIn = 6 + npcRng() * 12;
  e.parent = e.space.startsWith('upper:') ? byId(e.space.slice(6)).upper : world.groups.above;
  e.parent.add(e.ch.group);
  if (d.sit) sitPose(e.ch);
  return e;
});

// the troll, asleep on his side
const troll = entity({ name: 'Old Knuckles', x: CAVE.troll[0], z: CAVE.troll[1], space: 'cave', look: { skin: 0x6f8f4a, body: 0x5a4a32, legs: 0x5a4a32, hair: 0x3a4a2a, scale: 2.5, wide: 1.4 } });
troll.ch.group.rotation.z = Math.PI / 2;
troll.ch.group.rotation.y = 0.15;
troll.ch.group.position.set(-64.6, world.caveY + 0.75, -11);
troll.ch.armL.rotation.x = -1.2;
troll.ch.legR.rotation.x = 0.5;
world.groups.cave.add(troll.ch.group);
troll.bubbleAnchor = { x: -68, z: -11, y: world.caveY + 3 };

const interactables = [
  ...npcs.map((n) => ({ kind: 'npc', ent: n, name: n.name, role: n.def.role, lines: n.def.lines, range: 2.4 })),
  ...world.items.map((it) => ({ kind: 'item', name: it.name, lines: it.lines, x: it.x, z: it.z, space: it.space, range: 1.8 })),
  { kind: 'troll', name: 'Old Knuckles', role: 'troll, asleep', x: -66, z: -11, space: 'cave', range: 6, lines: ['He snores like a rockslide in slow motion.', 'A smell of old cheese and wet stone. One eye twitches. Best not.', 'Next to him: a chest, a heap of coins, and a lot of very clean bones.'] },
];

// ---------------------------------------------------------------- input
const keys = new Set();
const stick = { x: 0, z: 0, active: false };
let zoom = 1;
addEventListener('keydown', (ev) => {
  if (ev.repeat && ev.code === 'KeyE') return;
  keys.add(ev.code);
  if (ev.code === 'KeyE' || ev.code === 'Enter') interact();
  if (ev.code === 'KeyH' || ev.code === 'Slash') toggleHelp();
  if (ev.code === 'Escape') { closeDialog(); setHelp(false); }
  const t = { Digit1: 'dawn', Digit2: 'noon', Digit3: 'dusk', Digit4: 'night' }[ev.code];
  if (t) setTime(t);
});
addEventListener('keyup', (ev) => keys.delete(ev.code));
addEventListener('blur', () => keys.clear());
addEventListener('wheel', (ev) => { zoom = clamp(zoom + Math.sign(ev.deltaY) * 0.08, 0.55, 1.6); }, { passive: true });

// ---------------------------------------------------------------- state + events
const state = {
  ready: false, steps: 0, time: 0, won: false, failed: null,
  checkpoint: null, checkpoints: [], log: [], seen: new Set(),
};
function mark(msg) {
  const line = `GameCraft: ${msg}`;
  state.log.push(line);
  console.log(line);
}
function context() {
  if (player.space === 'below') {
    const room = W.roomName('below', player.x, player.z) ?? '';
    return room.startsWith('Smugglers') ? 'tunnel' : 'cellar';
  }
  if (player.space === 'cave') return Math.hypot(player.x - CAVE.troll[0], player.z - CAVE.troll[1]) < 6 ? 'troll' : 'cave';
  if (player.space === 'upper:tavern') return 'tavern-upper';
  const b = W.buildingAt(player.x, player.z);
  if (b?.id === 'tavern') return 'tavern';
  if (b?.id === 'smithy') return 'smithy';
  return null;
}
let barkTurn = 0;
function onContext(ctx) {
  if (!ctx || state.seen.has(ctx)) return;
  state.seen.add(ctx);
  const order = barkTurn++ % 2 === 0 ? companions : [...companions].reverse();
  const c = order.find((cmp) => cmp.bark[ctx]);
  if (c) say(c, c.bark[ctx], 4.5);
}

// ---------------------------------------------------------------- autoplay (the verifier's driver)
const inside = (id) => (player.space === 'outdoor' || player.space === `upper:${id}`) && W.buildingAt(player.x, player.z)?.id === id;
const opacity = (id, key) => Math.max(...byId(id).fades[key].map((m) => m.material.opacity));
const trollDist = () => Math.hypot(player.x - CAVE.troll[0], player.z - CAVE.troll[1]);
const TOUR = [
  { to: [0, -8.5] },
  { to: [0, -11.5], checkpoint: 'tavern-ground', check: () => inside('tavern') && !byId('tavern').roofGroup.visible && !byId('tavern').upper.visible && opacity('tavern', 'S0') < 0.4 },
  { to: [-1, -18.5] }, { to: [-5.2, -19.9] }, { to: [-6.95, -20.9] },
  { to: [-6.95, -12.6], checkpoint: 'tavern-upper', check: () => player.space === 'upper:tavern' && byId('tavern').upper.visible && opacity('tavern', 'S1') < 0.4 && Math.abs(player.y - W.UPPER_Y) < 0.01 },
  { to: [3.5, -13.0] }, { to: [-6.95, -12.6] },
  { to: [-6.95, -20.9], check: () => player.space === 'outdoor' && Math.abs(player.y - W.FLOOR_Y) < 0.01 },
  { to: [-5.2, -19.9] }, { to: [5.4, -16.8] }, { to: [6.85, -16.8] },
  { to: [6.85, -23.0], checkpoint: 'cellar', check: () => player.space === 'below' && !world.groups.above.visible && world.groups.below.visible && Math.abs(player.y - W.BELOW_Y) < 0.01 },
  { to: [2.25, -22.5] },
  { to: [2.25, -8.6], checkpoint: 'tunnel', check: () => player.space === 'below' && (W.roomName('below', player.x, player.z) ?? '').startsWith('Smugglers') },
  { to: [2.25, 3.2] }, { to: [17.8, 3.2] }, { to: [19.5, 0.2] },
  { to: [25.8, 0.2], checkpoint: 'smithy', check: () => player.space === 'outdoor' && inside('smithy') && world.groups.above.visible },
  { to: [25.8, 4.0] }, { to: [22, 6.5] }, { to: [22, 9.6] },
  { to: [22, 12], checkpoint: 'village', check: () => !W.buildingAt(player.x, player.z) && byId('smithy').roofGroup.visible },
  { to: [14, 12] }, { to: [4.5, 3.5] }, { to: [-48.8, -6] },
  { to: [-56, -6], checkpoint: 'cave', check: () => player.space === 'cave' && world.groups.cave.visible && !world.groups.above.visible },
  { to: [-62.3, -10.5], checkpoint: 'troll', check: () => trollDist() < 5.0, win: true },
];
const auto = { active: false, i: 0, stepsHere: 0, budget: 0, settle: 0 };
function startAutoplay() {
  Object.assign(auto, { active: true, i: 0, stepsHere: 0, settle: 0 });
  auto.budget = budgetFor(0);
  mark('autoplay start');
}
function budgetFor(i) {
  const [tx, tz] = TOUR[i].to;
  return Math.ceil((Math.hypot(tx - player.x, tz - player.z) / 6) * 60 * 2.5) + 300;
}
function autoSteer() {
  const wp = TOUR[auto.i];
  if (auto.settle > 0) {
    auto.settle--;
    if (auto.settle === 0) {
      if (wp.check && !wp.check()) { fail(`check failed at ${wp.checkpoint ?? `waypoint ${auto.i}`}`); return [0, 0]; }
      if (wp.checkpoint) { state.checkpoint = wp.checkpoint; state.checkpoints.push({ name: wp.checkpoint, step: state.steps }); mark(`checkpoint ${wp.checkpoint}`); }
      if (wp.win) { state.won = true; auto.active = false; mark('tour complete, win'); return [0, 0]; }
      auto.i++; auto.stepsHere = 0; auto.budget = budgetFor(auto.i);
    }
    return [0, 0];
  }
  const dx = wp.to[0] - player.x, dz = wp.to[1] - player.z;
  const d = Math.hypot(dx, dz);
  if (d < 0.2) { auto.settle = wp.check || wp.checkpoint ? 45 : 1; return [0, 0]; }
  if (++auto.stepsHere > auto.budget) { fail(`autoplay stuck at waypoint ${auto.i} (${player.x.toFixed(2)}, ${player.z.toFixed(2)}) in ${player.space}`); return [0, 0]; }
  const k = Math.min(1, d / 0.6);
  return [(dx / d) * k, (dz / d) * k];
}
function fail(msg) {
  state.failed = msg;
  auto.active = false;
  mark(msg);
}

// ---------------------------------------------------------------- simulation step
const PLAYER_SPEED = 4.2, RUN_SPEED = 6.5, AUTO_SPEED = 6.0;
function update(dt) {
  state.steps++;
  state.time += dt;
  clockMinutes += dt * 0.5;
  // input
  let mx = 0, mz = 0, speed = PLAYER_SPEED;
  if (auto.active) { [mx, mz] = autoSteer(); speed = AUTO_SPEED; }
  else if (!dialog.open) {
    if (keys.has('KeyW') || keys.has('ArrowUp')) mz -= 1;
    if (keys.has('KeyS') || keys.has('ArrowDown')) mz += 1;
    if (keys.has('KeyA') || keys.has('ArrowLeft')) mx -= 1;
    if (keys.has('KeyD') || keys.has('ArrowRight')) mx += 1;
    if (stick.active) { mx = stick.x; mz = stick.z; }
    if (keys.has('ShiftLeft') || keys.has('ShiftRight')) speed = RUN_SPEED;
  }
  const len = Math.hypot(mx, mz);
  const prev = [player.x, player.z];
  if (len > 0.05) {
    const nx = mx / Math.max(1, len), nz = mz / Math.max(1, len);
    step(player, nx, nz, speed * Math.min(1, len) * dt);
    turnTowards(player, mx, mz, dt);
  }
  player.speed = Math.hypot(player.x - prev[0], player.z - prev[1]) / dt;
  animateWalk(player.ch, player.speed, dt);
  // trail for the companions
  const last = trail[trail.length - 1];
  if (Math.hypot(player.x - last.x, player.z - last.z) > 0.3 || player.space !== last.space) {
    trail.push({ x: player.x, z: player.z, y: player.y, space: player.space });
    if (trail.length > 240) trail.shift();
  }
  companions.forEach((c, i) => {
    const t = trail[Math.max(0, trail.length - 1 - (5 + 6 * i))];
    const ox = c.x, oz = c.z;
    const k = Math.min(1, dt * 7);
    c.x = lerp(c.x, t.x, k); c.z = lerp(c.z, t.z, k);
    if (Math.hypot(c.x - t.x, c.z - t.z) < 0.6 || c.space !== t.space) { c.space = t.space; }
    c.y = lerp(c.y, t.y, k);
    if (c.space !== t.space && Math.hypot(c.x - t.x, c.z - t.z) > 3) { c.x = t.x; c.z = t.z; c.y = t.y; c.space = t.space; }
    c.speed = Math.hypot(c.x - ox, c.z - oz) / dt;
    if (c.speed > 0.2) turnTowards(c, c.x - ox, c.z - oz, dt);
    animateWalk(c.ch, c.speed, dt);
  });
  // villagers
  for (const n of npcs) updateNpc(n, dt);
  // troll breathing
  troll.ch.body.scale.set(1, 1, 1 + Math.sin(state.time * 1.1) * 0.04);
  // world reactions
  onContext(context());
  updateView(dt);
  for (const [i, f] of world.fires.entries()) {
    const fl = 0.82 + 0.18 * Math.sin(state.time * 11 + i * 1.7) * Math.sin(state.time * 6.3 + i);
    f.light.intensity = f.base * fl;
    if (f.flame) f.flame.scale.set(1, 0.85 + 0.3 * fl, 1);
  }
  updateBarks(dt);
  if (dialog.open && dialog.target && dist2(dialog.target) > (dialog.target.range + 1.5) ** 2) closeDialog();
}
function updateNpc(n, dt) {
  const d = n.def;
  if (d.fixed) { if (!d.sit) animateWalk(n.ch, 0, dt); n.ch.group.position.set(n.x, n.y, n.z); n.ch.group.rotation.y = n.rot; return; }
  let speed = 0;
  if (n.wait > 0) n.wait -= dt;
  else {
    if (!n.target) {
      const [x0, x1, z0, z1] = d.wander;
      n.target = [lerp(x0, x1, npcRng()), lerp(z0, z1, npcRng())];
    }
    const dx = n.target[0] - n.x, dz = n.target[1] - n.z;
    const dd = Math.hypot(dx, dz);
    if (dd < 0.15) { n.target = null; n.wait = 1.5 + npcRng() * 3.5; }
    else {
      const ox = n.x, oz = n.z;
      if (!step(n, dx / dd, dz / dd, (d.speed ?? 1.6) * dt)) { n.target = null; n.wait = 0.6; }
      speed = Math.hypot(n.x - ox, n.z - oz) / dt;
      turnTowards(n, dx, dz, dt);
    }
  }
  animateWalk(n.ch, speed, dt);
  n.ch.group.position.set(n.x, n.y, n.z);
  n.ch.group.rotation.y = n.rot;
}

// ---------------------------------------------------------------- view: cutaway, floors, layers
const ray = new THREE.Ray();
const hitPoint = new THREE.Vector3();
const head = new THREE.Vector3();
const camTarget = new THREE.Vector3(player.x, player.y + 1, player.z);
const CAM_OFFSET = new THREE.Vector3(0, 15.5, 13.5);
camera.position.copy(camTarget).addScaledVector(CAM_OFFSET, zoom);
function setOpacity(m, target, k) {
  const o = m.material.opacity + (target - m.material.opacity) * k;
  m.material.opacity = o;
  const solid = o > 0.6;
  m.material.depthWrite = solid;
  m.castShadow = solid;
  for (const c of m.children) c.visible = solid;
}
function updateView(dt) {
  const view = viewOf(player.space);
  world.groups.above.visible = view === 'above';
  world.groups.below.visible = view === 'below';
  world.groups.cave.visible = view === 'cave';
  const onUpper = player.space.startsWith('upper:');
  const inB = onUpper ? byId(player.space.slice(6)) : player.space === 'outdoor' ? W.buildingAt(player.x, player.z) : null;
  // camera follow (also needed for the occlusion test)
  const k = 1 - Math.exp(-dt * 6);
  camTarget.lerp(head.set(player.x, player.y + 1, player.z), k);
  camera.position.lerp(head.copy(camTarget).addScaledVector(CAM_OFFSET, zoom), k);
  camera.lookAt(camTarget);
  head.set(player.x, player.y + 1.2, player.z);
  const fk = Math.min(1, dt * 8);
  for (const b of BUILDINGS) {
    let s0 = 1, s1 = 1, roof = 1, roofVis = true, upperVis = true;
    if (b === inB) {
      roofVis = false;
      if (onUpper) s1 = 0.12;
      else { upperVis = false; s0 = 0.12; }
    } else if (view === 'above') {
      ray.origin.copy(camera.position);
      ray.direction.subVectors(head, camera.position);
      const dist = ray.direction.length();
      ray.direction.normalize();
      if (ray.intersectBox(b.bounds, hitPoint) && hitPoint.distanceTo(camera.position) < dist - 0.5) { roof = 0.22; s0 = 0.22; s1 = 0.22; }
    }
    b.roofGroup.visible = roofVis;
    b.upper.visible = upperVis;
    for (const m of b.fades.S0) setOpacity(m, s0, fk);
    for (const m of b.fades.S1) setOpacity(m, s1, fk);
    for (const m of b.fades.roof) setOpacity(m, roof, fk);
  }
  // companions are only drawn on the layer the player is looking at
  for (const c of companions) {
    let vis = viewOf(c.space) === view;
    if (vis && c.space.startsWith('upper:') && !byId(c.space.slice(6)).upper.visible) vis = false;
    if (vis && c.space === 'outdoor' && onUpper && W.buildingAt(c.x, c.z) === inB) vis = true;
    c.ch.group.visible = vis;
    c.ch.group.position.set(c.x, c.y, c.z);
    c.ch.group.rotation.y = c.rot;
  }
  player.ch.group.position.set(player.x, player.y, player.z);
  player.ch.group.rotation.y = player.rot;
  applyLighting(view);
}

// ---------------------------------------------------------------- time of day + lighting
const sunDir = new THREE.Vector3();
const skyColor = new THREE.Color();
function setTime(key) {
  timeKey = key;
  clockMinutes = TIMES[key].clock;
  document.querySelectorAll('[data-time]').forEach((el) => el.classList.toggle('on', el.dataset.time === key));
}
function applyLighting(view) {
  const t = TIMES[timeKey];
  const under = view !== 'above';
  sunDir.set(...t.sun).normalize();
  sun.position.copy(camTarget).addScaledVector(sunDir, 70);
  sun.target.position.copy(camTarget);
  sun.color.setHex(t.sunColor);
  sun.intensity = under ? 0.12 : t.sunI;
  hemi.color.setHex(t.hemiSky);
  hemi.groundColor.setHex(t.hemiGround);
  hemi.intensity = under ? 0.22 : t.hemiI;
  skyColor.setHex(under ? 0x060505 : t.sky);
  scene.background.copy(skyColor);
  scene.fog.color.copy(skyColor);
  scene.fog.near = under ? 30 : 80;
  scene.fog.far = under ? 70 : 170;
  W.WINDOW_MAT.emissiveIntensity = t.windows;
  world.lampMat.emissiveIntensity = t.lamps > 0 ? 2.4 : 0.2;
  for (const l of world.lamps) l.intensity = t.lamps;
}

// ---------------------------------------------------------------- HUD
const $ = (id) => document.getElementById(id);
const dialog = { open: false, target: null, i: 0 };
function dist2(t) {
  const ex = t.ent ? t.ent.x : t.x, ez = t.ent ? t.ent.z : t.z;
  return (player.x - ex) ** 2 + (player.z - ez) ** 2;
}
function sameSpace(t) {
  const sp = t.ent ? t.ent.space : t.space;
  return sp === player.space;
}
function nearest() {
  let best = null, bd = Infinity;
  for (const t of interactables) {
    if (!sameSpace(t)) continue;
    const d = dist2(t);
    if (d < t.range * t.range && d < bd) { best = t; bd = d; }
  }
  return best;
}
function interact() {
  if (dialog.open) {
    dialog.i++;
    if (dialog.i >= dialog.target.lines.length) { closeDialog(); return; }
    $('dialog-text').textContent = dialog.target.lines[dialog.i];
    return;
  }
  const t = nearest();
  if (!t) return;
  Object.assign(dialog, { open: true, target: t, i: 0 });
  $('dialog-who').textContent = t.role ? `${t.name}, ${t.role}` : t.name;
  $('dialog-text').textContent = t.lines[0];
  $('dialog').hidden = false;
  if (t.ent) { const dx = player.x - t.ent.x, dz = player.z - t.ent.z; t.ent.rot = Math.atan2(dx, dz); t.ent.wait = 4; }
}
function closeDialog() { dialog.open = false; dialog.target = null; $('dialog').hidden = true; }
function setHelp(v) { $('help').hidden = !v; }
function toggleHelp() { setHelp($('help').hidden); }

// speech bubbles
const bubbles = new Map();
function say(ent, text, secs = 3.5) {
  let el = bubbles.get(ent);
  if (!el) { el = document.createElement('div'); el.className = 'bubble'; $('bubbles').appendChild(el); bubbles.set(ent, el); }
  el.textContent = text;
  ent.sayFor = secs;
}
function updateBarks(dt) {
  for (const n of npcs) {
    n.barkIn -= dt;
    if (n.barkIn <= 0) {
      n.barkIn = 9 + npcRng() * 14;
      if (n.space === player.space && dist2({ ent: n }) < 16 * 16) say(n, n.def.barks[Math.floor(npcRng() * n.def.barks.length)]);
    }
  }
  troll.sayFor = 99;
  for (const e of [...npcs, ...companions]) if (e.sayFor > 0) e.sayFor -= dt;
}
function shown(obj) {
  for (let o = obj; o; o = o.parent) if (!o.visible) return false;
  return true;
}
const proj = new THREE.Vector3();
function placeBubble(el, x, y, z, visible) {
  proj.set(x, y, z).project(camera);
  const onScreen = visible && proj.z < 1 && Math.abs(proj.x) < 1.1 && Math.abs(proj.y) < 1.1;
  el.style.display = onScreen ? 'block' : 'none';
  if (onScreen) el.style.transform = `translate(-50%, -100%) translate(${(proj.x * 0.5 + 0.5) * innerWidth}px, ${(-proj.y * 0.5 + 0.5) * innerHeight}px)`;
}
let zzz = null;
function drawBubbles() {
  for (const [ent, el] of bubbles) placeBubble(el, ent.x, ent.y + (ent === troll ? 0 : 2.35), ent.z, ent.sayFor > 0 && shown(ent.ch.group));
  if (!zzz) { zzz = document.createElement('div'); zzz.className = 'bubble zzz'; $('bubbles').appendChild(zzz); }
  zzz.textContent = ['z', 'zZ', 'zZz'][Math.floor(state.time * 1.5) % 3];
  placeBubble(zzz, troll.bubbleAnchor.x, troll.bubbleAnchor.y, troll.bubbleAnchor.z, world.groups.cave.visible);
}

function locationText() {
  if (player.space === 'below') return ['Underground', W.roomName('below', player.x, player.z) ?? 'Underground'];
  if (player.space === 'cave') return ['Knuckle Crag', W.roomName('cave', player.x, player.z) ?? 'Knucklehollow Cave'];
  if (player.space.startsWith('upper:')) { const b = byId(player.space.slice(6)); return [`${b.kind} - upstairs`, b.name]; }
  const b = W.buildingAt(player.x, player.z);
  if (b) return [b.kind, b.name];
  const dp = Math.hypot(player.x - W.PLAZA.x, player.z - W.PLAZA.z);
  if (player.x < -38) return ['Outskirts', 'Foot of Knuckle Crag'];
  return [dp < 14 ? 'Village green' : 'Mossbrook outskirts', 'Mossbrook'];
}
const mm = $('minimap').getContext('2d');
function drawMinimap() {
  const S = 150, R = 46, sc = S / (2 * R);
  const cx = player.x, cz = player.z;
  const tx = (x) => (x - cx) * sc + S / 2, tz = (z) => (z - cz) * sc + S / 2;
  mm.clearRect(0, 0, S, S);
  mm.save();
  mm.beginPath(); mm.arc(S / 2, S / 2, S / 2 - 2, 0, Math.PI * 2); mm.clip();
  const view = viewOf(player.space);
  mm.fillStyle = view === 'above' ? '#3d5a2a' : '#141210';
  mm.fillRect(0, 0, S, S);
  const rects = (list, fill) => { mm.fillStyle = fill; for (const r of list) mm.fillRect(tx(r[0]), tz(r[2]), (r[1] - r[0]) * sc, (r[3] - r[2]) * sc); };
  if (view === 'above') {
    mm.fillStyle = '#6d6862';
    mm.beginPath(); mm.arc(tx(W.MOUNTAIN.x), tz(W.MOUNTAIN.z), W.MOUNTAIN.block * sc, 0, Math.PI * 2); mm.fill();
    mm.strokeStyle = '#9b7a50'; mm.lineWidth = 2.2;
    for (const [a, b] of W.ROADS) { mm.beginPath(); mm.moveTo(tx(a[0]), tz(a[1])); mm.lineTo(tx(b[0]), tz(b[1])); mm.stroke(); }
    rects(BUILDINGS.map((b) => [b.x0, b.x1, b.z0, b.z1]), '#c8b48a');
    mm.fillStyle = '#111'; mm.beginPath(); mm.arc(tx(-50), tz(-6), 3, 0, Math.PI * 2); mm.fill();
  } else rects((view === 'below' ? W.BELOW_ROOMS : W.CAVE_ROOMS).map((r) => r.r), '#5e5952');
  const dot = (x, z, c, r = 3) => { mm.fillStyle = c; mm.beginPath(); mm.arc(tx(x), tz(z), r, 0, Math.PI * 2); mm.fill(); };
  for (const n of npcs) if (n.space === player.space) dot(n.x, n.z, '#f0d060', 2.2);
  dot(companions[0].x, companions[0].z, '#6ac06a');
  dot(companions[1].x, companions[1].z, '#d05050');
  dot(player.x, player.z, '#ffffff', 3.6);
  mm.restore();
  mm.strokeStyle = 'rgba(232,214,170,.7)'; mm.lineWidth = 2;
  mm.beginPath(); mm.arc(S / 2, S / 2, S / 2 - 2, 0, Math.PI * 2); mm.stroke();
  mm.fillStyle = '#e8d6aa'; mm.font = 'bold 11px Georgia, serif'; mm.textAlign = 'center'; mm.fillText('N', S / 2, 13);
}
let hudTick = 0;
function drawHud() {
  const [sub, place] = locationText();
  $('loc-place').textContent = place;
  $('loc-sub').textContent = sub;
  const m = Math.floor(clockMinutes) % (24 * 60);
  $('clock').textContent = `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
  const t = dialog.open ? null : nearest();
  $('prompt').hidden = !t;
  if (t) $('prompt-text').textContent = t.kind === 'item' ? `Read: ${t.name}` : `Talk to ${t.name}`;
  $('talk-btn').hidden = !t && !dialog.open;
  if (hudTick++ % 6 === 0) drawMinimap();
}

// ---------------------------------------------------------------- render + loop
function resize() {
  const w = innerWidth, h = innerHeight;
  renderer.setSize(w, h, false);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
}
addEventListener('resize', resize);
resize();
function render() {
  renderer.render(scene, camera);
  drawBubbles();
  drawHud();
}

// touch controls
const zone = $('stick-zone'), knob = $('stick-knob');
let stickId = null, stickOrigin = [0, 0];
zone.addEventListener('pointerdown', (ev) => { stickId = ev.pointerId; stickOrigin = [ev.clientX, ev.clientY]; zone.setPointerCapture(ev.pointerId); stick.active = true; knob.hidden = false; moveKnob(ev); });
zone.addEventListener('pointermove', (ev) => { if (ev.pointerId === stickId) moveKnob(ev); });
const endStick = (ev) => { if (ev.pointerId !== stickId) return; stickId = null; stick.active = false; stick.x = stick.z = 0; knob.hidden = true; };
zone.addEventListener('pointerup', endStick);
zone.addEventListener('pointercancel', endStick);
function moveKnob(ev) {
  const dx = ev.clientX - stickOrigin[0], dz = ev.clientY - stickOrigin[1];
  const d = Math.hypot(dx, dz), max = 50;
  const k = d > max ? max / d : 1;
  stick.x = (dx * k) / max; stick.z = (dz * k) / max;
  knob.style.left = `${stickOrigin[0] + dx * k}px`; knob.style.top = `${stickOrigin[1] + dz * k}px`;
}
$('talk-btn').addEventListener('click', interact);
$('help-btn').addEventListener('click', toggleHelp);
$('help-close').addEventListener('click', () => setHelp(false));
document.querySelectorAll('[data-time]').forEach((el) => el.addEventListener('click', () => setTime(el.dataset.time)));
setTime(timeKey);

window.__gamecraft = {
  version: 1,
  get ready() { return state.ready; },
  state: () => ({
    ready: state.ready, steps: state.steps, won: state.won, failed: state.failed,
    checkpoint: state.checkpoint, checkpoints: state.checkpoints, log: state.log,
    space: player.space, x: +player.x.toFixed(3), z: +player.z.toFixed(3), y: +player.y.toFixed(3),
    autoplay: auto.active, waypoint: auto.i,
  }),
  autoplay: () => { startAutoplay(); return true; },
  step: (n = 1) => {
    // Stop at the first new checkpoint so the verifier screenshots the frame it names.
    const seen = state.checkpoints.length;
    for (let i = 0; i < n && (auto.active || !VERIFY); i++) {
      update(DT);
      if (state.checkpoints.length !== seen) break;
    }
    return window.__gamecraft.state();
  },
  render: () => { render(); return true; },
  setTime: (k) => { if (k in TIMES) setTime(k); return timeKey; },
};

updateView(1);
state.ready = true;
mark('ready');
if (!VERIFY) {
  if (params.has('autoplay')) { setHelp(false); startAutoplay(); }
  else setHelp(true);
  let lastT = performance.now(), acc = 0;
  const frame = (now) => {
    acc += Math.min(0.1, (now - lastT) / 1000);
    lastT = now;
    let n = 0;
    while (acc >= DT && n < 6) { update(DT); acc -= DT; n++; }
    render();
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
} else {
  render();
}

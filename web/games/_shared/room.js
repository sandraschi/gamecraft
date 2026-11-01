// Procedural cosy-but-cluttered living room, shared by Spot the Difference and
// Tidy the Room. buildRoom() is deterministic, so two calls give identical rooms.
//
// Every interesting object is an "item": a Group with userData
//   { itemId, label, main: [meshes that carry its colour], muts: [allowed differences],
//     movable: bool (Tidy can carry it), base: y offset from its origin to the floor }
import * as THREE from 'three';
import { rng, std } from './gc.js';

const BOOK_COLORS = [0x8c2f2f, 0x2f4f8c, 0x2f7a4a, 0xc9a227, 0x6b3f8c, 0xd8d0c0, 0x2a2a2a, 0xb85a2a, 0x3f7f8c, 0x9c6b4a];

function box(w, h, d, color, x = 0, y = 0, z = 0) {
  const m = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), std(color));
  m.position.set(x, y, z);
  m.castShadow = true; m.receiveShadow = true;
  return m;
}
function cyl(rt, rb, h, color, x = 0, y = 0, z = 0, seg = 20) {
  const m = new THREE.Mesh(new THREE.CylinderGeometry(rt, rb, h, seg), std(color));
  m.position.set(x, y, z);
  m.castShadow = true; m.receiveShadow = true;
  return m;
}
function ball(r, color, x = 0, y = 0, z = 0) {
  const m = new THREE.Mesh(new THREE.SphereGeometry(r, 20, 14), std(color));
  m.position.set(x, y, z);
  m.castShadow = true;
  return m;
}

export function buildRoom() {
  const r = rng(20261031);
  const root = new THREE.Group();
  const items = new Map();
  const item = (id, label, x, y, z, { muts = [], movable = false } = {}) => {
    const g = new THREE.Group();
    g.position.set(x, y, z);
    g.userData = { itemId: id, label, main: [], muts, movable, base: 0 };
    root.add(g);
    items.set(id, g);
    return g;
  };
  const main = (g, m) => { g.add(m); g.userData.main.push(m); return m; };

  // ---- shell
  const floor = new THREE.Mesh(new THREE.PlaneGeometry(6, 5), std(0x9a6a42, { roughness: 0.7 }));
  floor.rotation.x = -Math.PI / 2; floor.receiveShadow = true; root.add(floor);
  const apron = new THREE.Mesh(new THREE.PlaneGeometry(14, 12), std(0x9a6a42, { roughness: 0.7 })); // floor beyond the room edge, so wide views never see the void
  apron.rotation.x = -Math.PI / 2; apron.position.set(2, -0.002, 3); root.add(apron);
  for (let i = 0; i < 12; i++) { // floorboard seams
    const s = new THREE.Mesh(new THREE.PlaneGeometry(0.012, 5), std(0x6e4a2c));
    s.rotation.x = -Math.PI / 2; s.position.set(-3 + i * 0.5, 0.002, 0); root.add(s);
  }
  const wallMat = std(0xefe2c8);
  const back = new THREE.Mesh(new THREE.PlaneGeometry(6, 2.8), wallMat);
  back.position.set(0, 1.4, -2.5); back.receiveShadow = true; root.add(back);
  const left = new THREE.Mesh(new THREE.PlaneGeometry(5, 2.8), wallMat);
  left.rotation.y = Math.PI / 2; left.position.set(-3, 1.4, 0); left.receiveShadow = true; root.add(left);
  root.add(box(6, 0.12, 0.03, 0xffffff, 0, 0.06, -2.485));
  root.add(box(0.03, 0.12, 5, 0xffffff, -2.985, 0.06, 0));

  // ---- window with curtains
  const win = new THREE.Group(); win.position.set(1.55, 1.55, -2.48); root.add(win);
  const glass = new THREE.Mesh(new THREE.PlaneGeometry(1.3, 1.2), new THREE.MeshBasicMaterial({ color: 0xbfe0f4 }));
  win.add(glass);
  const hill = new THREE.Mesh(new THREE.CircleGeometry(0.9, 24, 0, Math.PI), new THREE.MeshBasicMaterial({ color: 0x8fc48a }));
  hill.position.set(0.2, -0.6, 0.002); win.add(hill);
  for (const [w, h, x, y] of [[1.42, 0.06, 0, 0.63], [1.42, 0.06, 0, -0.63], [0.06, 1.3, 0.68, 0], [0.06, 1.3, -0.68, 0], [0.04, 1.2, 0, 0], [1.3, 0.04, 0, 0]]) win.add(box(w, h, 0.05, 0xf8f8f2, x, y, 0.02));
  root.add(box(1.6, 0.05, 0.22, 0xf8f8f2, 1.55, 0.92, -2.4)); // sill
  const curtL = item('curtain-l', 'left curtain', 0.72, 1.5, -2.42, { muts: [{ kind: 'color' }] });
  main(curtL, box(0.32, 1.6, 0.06, 0xa84a3a));
  const curtR = item('curtain-r', 'right curtain', 2.38, 1.5, -2.42, { muts: [{ kind: 'color' }] });
  main(curtR, box(0.32, 1.6, 0.06, 0xa84a3a));
  const rod = cyl(0.015, 0.015, 2.0, 0x5a3a22, 1.55, 2.32, -2.42);
  rod.rotation.z = Math.PI / 2; root.add(rod);

  // ---- sofa
  const sofaCol = 0x3f7f7a;
  root.add(box(2.3, 0.42, 0.9, sofaCol, -0.45, 0.25, -2.0));
  root.add(box(2.3, 0.55, 0.22, sofaCol, -0.45, 0.72, -2.38));
  root.add(box(0.22, 0.62, 0.9, sofaCol, -1.66, 0.33, -2.0));
  root.add(box(0.22, 0.62, 0.9, sofaCol, 0.76, 0.33, -2.0));
  for (const x of [-1.5, 0.6]) root.add(box(0.08, 0.06, 0.08, 0x3a2a1a, x, 0.02, -1.62));
  const cushColors = [0xd8a23a, 0xc76f7a, 0xe8e0cc];
  [-1.15, -0.45, 0.3].forEach((x, i) => {
    const c = item(`cushion-${i + 1}`, `${['mustard', 'rose', 'cream'][i]} cushion`, x, 0.66, -2.12, { muts: [{ kind: 'color' }, { kind: 'hide' }], movable: true });
    const m = main(c, box(0.46, 0.42, 0.14, cushColors[i]));
    m.rotation.x = -0.25; m.rotation.z = (i - 1) * 0.12;
    c.userData.base = 0.23;
  });
  const blanket = item('blanket', 'blanket', 0.76, 0.5, -1.85, { muts: [{ kind: 'color' }, { kind: 'hide' }] });
  main(blanket, box(0.26, 0.5, 0.55, 0x6a8fc4)).position.y = 0.1;

  // ---- rug, coffee table and what is on it
  const rug = new THREE.Mesh(new THREE.PlaneGeometry(2.8, 1.9), std(0xa8433a));
  rug.rotation.x = -Math.PI / 2; rug.position.set(-0.4, 0.006, -0.6); root.add(rug);
  const rugIn = new THREE.Mesh(new THREE.PlaneGeometry(2.4, 1.5), std(0xd9b48a));
  rugIn.rotation.x = -Math.PI / 2; rugIn.position.set(-0.4, 0.008, -0.6); root.add(rugIn);
  const tableCol = 0x6b4426;
  root.add(box(1.3, 0.06, 0.66, tableCol, -0.4, 0.42, -0.62));
  for (const [x, z] of [[-1.0, -0.88], [0.2, -0.88], [-1.0, -0.36], [0.2, -0.36]]) root.add(box(0.06, 0.4, 0.06, tableCol, x, 0.2, z));

  const mug = item('mug', 'mug', -0.82, 0.45, -0.5, { muts: [{ kind: 'color' }, { kind: 'hide' }, { kind: 'move', v: [0.22, 0, -0.18] }], movable: true });
  main(mug, cyl(0.055, 0.05, 0.11, 0x2f6fb0, 0, 0.055, 0));
  const handle = new THREE.Mesh(new THREE.TorusGeometry(0.035, 0.012, 8, 16), std(0x2f6fb0));
  handle.position.set(0.065, 0.06, 0); mug.add(handle); mug.userData.main.push(handle);

  const teapot = item('teapot', 'teapot', -0.42, 0.45, -0.72, { muts: [{ kind: 'color' }, { kind: 'rotate', axis: 'y', angle: Math.PI }], movable: true });
  const body = main(teapot, ball(0.13, 0xf2efe6, 0, 0.11, 0)); body.scale.y = 0.8;
  const spout = main(teapot, cyl(0.02, 0.035, 0.16, 0xf2efe6, 0.15, 0.13, 0)); spout.rotation.z = -0.9;
  const lid = main(teapot, cyl(0.06, 0.07, 0.03, 0xf2efe6, 0, 0.21, 0));
  teapot.add(ball(0.02, 0x2a2a2a, 0, 0.235, 0)); void lid;
  const th = new THREE.Mesh(new THREE.TorusGeometry(0.06, 0.014, 8, 16, Math.PI), std(0xf2efe6));
  th.position.set(-0.13, 0.11, 0); th.rotation.z = Math.PI / 2; teapot.add(th); teapot.userData.main.push(th);

  const vase = item('vase', 'vase of flowers', 0.02, 0.45, -0.5, { muts: [{ kind: 'color' }, { kind: 'scale', s: 1.45 }], movable: true });
  const vprof = [];
  for (let i = 0; i <= 10; i++) { const t = i / 10; vprof.push(new THREE.Vector2(0.05 + 0.05 * Math.sin(t * Math.PI) - 0.02 * t, t * 0.24)); }
  main(vase, new THREE.Mesh(new THREE.LatheGeometry(vprof, 20), std(0x4a8a6a, { roughness: 0.4 }))).castShadow = true;
  [[0, 0.42, 0, 0xe85a6a], [0.06, 0.38, 0.03, 0xf2c84a], [-0.05, 0.4, -0.02, 0xa86ad8]].forEach(([x, y, z, c]) => {
    const st = cyl(0.006, 0.006, y - 0.2, 0x4a7a3a, x * 0.6, 0.2 + (y - 0.2) / 2, z * 0.6);
    st.rotation.z = -x * 2; vase.add(st);
    vase.add(ball(0.04, c, x, y, z));
  });

  const books = item('book-stack', 'stack of books', -0.9, 0.45, -0.85, { muts: [{ kind: 'hide' }, { kind: 'rotate', axis: 'y', angle: 0.8 }], movable: true });
  [[0x7a2a2a, 0.0], [0x2a4a7a, 0.05], [0xc9a227, 0.1]].forEach(([c, y], i) => { const b = main(books, box(0.3 - i * 0.03, 0.05, 0.22 - i * 0.02, c, 0, y + 0.025, 0)); b.rotation.y = i * 0.25; });

  // ---- floor clutter
  const yarn = item('yarn', 'ball of yarn', 0.55, 0, -1.28, { muts: [{ kind: 'color' }, { kind: 'move', v: [-0.5, 0, 0.35] }], movable: true });
  main(yarn, ball(0.11, 0xd84a8a, 0, 0.11, 0));
  for (const a of [0.4, -0.3]) { const n = cyl(0.008, 0.008, 0.36, 0xd0c090, 0, 0.17, 0); n.rotation.z = a; n.rotation.x = 0.4; yarn.add(n); }

  const slippers = item('slippers', 'slippers', 1.3, 0, -1.25, { muts: [{ kind: 'color' }, { kind: 'hide' }], movable: true });
  main(slippers, box(0.11, 0.06, 0.26, 0x7a5aa8, -0.08, 0.03, 0)).rotation.y = 0.15;
  main(slippers, box(0.11, 0.06, 0.26, 0x7a5aa8, 0.08, 0.03, 0.02)).rotation.y = -0.1;

  const mags = item('magazines', 'stack of magazines', -1.95, 0, -1.4, { muts: [{ kind: 'hide' }, { kind: 'color' }], movable: true });
  [0x3a8ac4, 0xe0e0e0, 0xc43a3a, 0xe8c840].forEach((c, i) => { const m = main(mags, box(0.3, 0.025, 0.4, c, 0, 0.0125 + i * 0.026, 0)); m.rotation.y = r.range(-0.3, 0.3); });

  const cat = item('cat', 'sleeping cat', -0.35, 0, 0.55, { muts: [{ kind: 'hide' }, { kind: 'color' }] });
  const catBody = main(cat, ball(0.22, 0x3a3a3a, 0, 0.13, 0)); catBody.scale.set(1.25, 0.6, 0.9);
  main(cat, ball(0.1, 0x3a3a3a, 0.24, 0.13, 0.08));
  for (const s of [-1, 1]) { const ear = new THREE.Mesh(new THREE.ConeGeometry(0.035, 0.07, 8), std(0x3a3a3a)); ear.position.set(0.27, 0.23, 0.08 + s * 0.05); cat.add(ear); cat.userData.main.push(ear); }
  const tail = new THREE.Mesh(new THREE.TorusGeometry(0.18, 0.025, 8, 20, Math.PI * 1.1), std(0x3a3a3a));
  tail.rotation.x = Math.PI / 2; tail.position.set(-0.05, 0.04, 0.02); cat.add(tail); cat.userData.main.push(tail);

  // ---- floor lamp
  root.add(cyl(0.16, 0.18, 0.03, 0x2a2a2a, 1.15, 0.015, -2.05));
  root.add(cyl(0.015, 0.015, 1.5, 0x2a2a2a, 1.15, 0.77, -2.05));
  const shade = item('lamp-shade', 'lamp shade', 1.15, 1.5, -2.05, { muts: [{ kind: 'color' }] });
  main(shade, new THREE.Mesh(new THREE.CylinderGeometry(0.14, 0.24, 0.28, 24, 1, true), std(0xf2d9a0, { side: THREE.DoubleSide, emissive: 0x5a4020 })));
  const bulb = new THREE.PointLight(0xffd9a0, 1.6, 4, 1.5); bulb.position.set(1.15, 1.45, -2.05); root.add(bulb);

  // ---- big plant and windowsill succulent
  root.add(cyl(0.2, 0.15, 0.36, 0xb8643a, 2.35, 0.18, -1.95));
  const plant = item('plant', 'big plant', 2.35, 0.36, -1.95, { muts: [{ kind: 'scale', s: 1.4 }, { kind: 'color' }] });
  for (let i = 0; i < 9; i++) {
    const a = (i / 9) * Math.PI * 2;
    const leaf = main(plant, ball(0.16, 0x3f8a3f, Math.cos(a) * 0.16, 0.35 + (i % 3) * 0.18, Math.sin(a) * 0.16));
    leaf.scale.set(0.45, 1.3, 0.25); leaf.rotation.set(Math.sin(a) * 0.5, -a, Math.cos(a) * 0.5);
  }
  const succ = item('succulent', 'succulent', 1.05, 0.945, -2.38, { muts: [{ kind: 'hide' }, { kind: 'color' }] });
  succ.add(cyl(0.06, 0.05, 0.09, 0xe0d0b0, 0, 0.045, 0));
  for (let i = 0; i < 5; i++) { const l = main(succ, ball(0.035, 0x6aa87a, Math.cos(i * 1.26) * 0.03, 0.11, Math.sin(i * 1.26) * 0.03)); l.scale.y = 1.6; }

  // ---- picture frames on the back wall
  const frames = [
    { id: 'frame-sun', label: 'sunny picture', x: -1.25, w: 0.52, h: 0.4, art: 0x9ac8e8, extra: (g) => { const s = new THREE.Mesh(new THREE.CircleGeometry(0.07, 20), std(0xf2c84a, { emissive: 0x6a5010 })); s.position.set(0.12, 0.08, 0.02); g.add(s); g.userData.main.push(s); } },
    { id: 'frame-portrait', label: 'portrait', x: -0.5, w: 0.34, h: 0.46, art: 0xe8d8b8, extra: (g) => { const s = new THREE.Mesh(new THREE.CircleGeometry(0.07, 20), std(0x8a5a3a)); s.position.set(0, 0.06, 0.02); g.add(s); g.userData.main.push(s); const b = new THREE.Mesh(new THREE.CircleGeometry(0.11, 20, 0, Math.PI), std(0x3a5a8a)); b.position.set(0, -0.22, 0.02); g.add(b); } },
    { id: 'frame-stripes', label: 'striped print', x: 0.12, w: 0.44, h: 0.34, art: 0xf2efe6, extra: (g) => { [0xe85a4a, 0xf2a83a, 0x3a8ac4].forEach((c, i) => { const s = new THREE.Mesh(new THREE.PlaneGeometry(0.08, 0.26), std(c)); s.position.set(-0.12 + i * 0.12, 0, 0.02); g.add(s); g.userData.main.push(s); }); } },
  ];
  for (const f of frames) {
    const g = item(f.id, f.label, f.x, 1.65, -2.47, { muts: [{ kind: 'rotate', axis: 'z', angle: 0.22 }, { kind: 'color' }] });
    g.add(box(f.w + 0.06, f.h + 0.06, 0.03, 0x3a2a1a, 0, 0, 0));
    const art = new THREE.Mesh(new THREE.PlaneGeometry(f.w, f.h), std(f.art)); art.position.z = 0.017; g.add(art);
    if (f.id !== 'frame-stripes') g.userData.main.push(art);
    f.extra(g);
  }

  // ---- bookshelf on the left wall
  const shelfCol = 0x7a5232;
  root.add(box(0.4, 2.0, 0.04, shelfCol, -2.8, 1.0, -1.6));
  root.add(box(0.4, 2.0, 0.04, shelfCol, -2.8, 1.0, 0.2));
  root.add(box(0.02, 2.0, 1.84, 0x5a3a22, -2.99, 1.0, -0.7));
  const shelfYs = [0.05, 0.55, 1.05, 1.55, 1.98];
  for (const y of shelfYs) root.add(box(0.4, 0.04, 1.84, shelfCol, -2.8, y, -0.7));
  let bookN = 0;
  for (let row = 0; row < 3; row++) {
    let z = -1.55;
    const y0 = shelfYs[row + 1] + 0.02;
    while (z < 0.12) {
      const w = r.range(0.045, 0.085), h = r.range(0.26, 0.4);
      if (z + w > 0.16) break;
      if (r() < 0.08) { z += 0.12; continue; } // a gap
      const col = BOOK_COLORS[r.int(BOOK_COLORS.length)];
      const id = `book-${row}-${bookN++}`;
      const b = item(id, 'book on the shelf', -2.78, y0, z + w / 2, { muts: [{ kind: 'color' }, { kind: 'hide' }] });
      main(b, box(0.28, h, w, col, 0, h / 2, 0));
      z += w + 0.004;
    }
  }
  const globe = item('globe', 'globe', -2.8, 2.0, -1.2, { muts: [{ kind: 'hide' }, { kind: 'color' }] });
  globe.add(cyl(0.05, 0.07, 0.04, 0x3a2a1a, 0, 0.02, 0));
  main(globe, ball(0.12, 0x3a7ac4, 0, 0.16, 0));

  // ---- sideboard, radio, clock on the left wall
  root.add(box(0.45, 0.7, 1.3, 0x8a5a3a, -2.76, 0.35, 1.2));
  for (const z of [0.9, 1.5]) root.add(box(0.02, 0.5, 0.5, 0x6a4026, -2.53, 0.35, z));
  const radio = item('radio', 'radio', -2.75, 0.7, 1.0, { muts: [{ kind: 'color' }, { kind: 'rotate', axis: 'y', angle: 0.6 }], movable: true });
  main(radio, box(0.22, 0.2, 0.36, 0xc8a25a, 0, 0.1, 0));
  const spk = new THREE.Mesh(new THREE.CircleGeometry(0.06, 20), std(0x3a2a1a)); spk.rotation.y = Math.PI / 2; spk.position.set(0.112, 0.1, -0.07); radio.add(spk);
  const knob = cyl(0.02, 0.02, 0.02, 0x2a2a2a, 0.115, 0.1, 0.1); knob.rotation.z = Math.PI / 2; radio.add(knob);
  const clock = item('clock', 'wall clock', -2.98, 1.75, 1.2, { muts: [{ kind: 'clock' }, { kind: 'color' }] });
  const face = cyl(0.2, 0.2, 0.04, 0xf8f4ea, 0, 0, 0, 32); face.rotation.z = Math.PI / 2; face.position.x = 0.03; clock.add(face);
  const rim = new THREE.Mesh(new THREE.TorusGeometry(0.2, 0.02, 8, 32), std(0x8a2a2a)); rim.rotation.y = Math.PI / 2; rim.position.x = 0.04; clock.add(rim); clock.userData.main.push(rim);
  const hands = new THREE.Group(); hands.position.x = 0.06; clock.add(hands); clock.userData.hands = hands;
  const hh = box(0.01, 0.11, 0.02, 0x1a1a1a, 0, 0.05, 0); const mh = box(0.01, 0.16, 0.012, 0x1a1a1a, 0, 0.075, 0);
  const hp = new THREE.Group(); hp.add(hh); hp.rotation.x = -0.9; hands.add(hp);
  const mp = new THREE.Group(); mp.add(mh); mp.rotation.x = 1.4; hands.add(mp);

  // base offsets: how far each item's origin sits above its lowest point
  root.updateMatrixWorld(true);
  const bb = new THREE.Box3();
  for (const g of items.values()) { bb.setFromObject(g); g.userData.base = g.position.y - bb.min.y; }
  return { root, items };
}

export function roomLights(scene, { shadows = false } = {}) {
  scene.add(new THREE.HemisphereLight(0xfff4e0, 0x6a5040, 1.1));
  const sun = new THREE.DirectionalLight(0xfff0d8, 1.6);
  sun.position.set(3, 5, 4);
  if (shadows) { sun.castShadow = true; sun.shadow.mapSize.set(1024, 1024); Object.assign(sun.shadow.camera, { left: -4, right: 4, top: 4, bottom: -4 }); }
  scene.add(sun);
  return sun;
}

export function roomCamera(aspect) {
  const cam = new THREE.PerspectiveCamera(48, aspect, 0.1, 50);
  cam.position.set(2.7, 2.55, 3.6);
  cam.lookAt(-0.55, 0.75, -0.8);
  cam.updateMatrixWorld();
  return cam;
}

/** A property signature, used to prove a mutation really changed something. */
export function signature(g) {
  const cols = g.userData.main.map((m) => m.material.color.getHexString()).join(',');
  const hands = g.userData.hands ? g.userData.hands.children.map((c) => c.rotation.x.toFixed(2)).join(',') : '';
  return [g.visible, g.position.toArray().map((v) => v.toFixed(3)).join(','), g.rotation.toArray().slice(0, 3).map((v) => v.toFixed(3)).join(','), g.scale.x.toFixed(3), cols, hands].join('|');
}

/** Applies one difference to an item (in the second room copy). */
export function applyMut(g, mut, rr) {
  switch (mut.kind) {
    case 'hide': g.visible = false; break;
    case 'move': g.position.add(new THREE.Vector3(...mut.v)); break;
    case 'scale': g.scale.multiplyScalar(mut.s); break;
    case 'rotate': g.rotation[mut.axis] += mut.angle; break;
    case 'clock': g.userData.hands.children[0].rotation.x += 2.2; g.userData.hands.children[1].rotation.x -= 1.6; break;
    case 'color': {
      const shift = rr.range(0.3, 0.6);
      for (const m of g.userData.main) {
        m.material = m.material.clone();
        const hsl = {}; m.material.color.getHSL(hsl);
        const s = Math.max(0.55, hsl.s), l = Math.min(0.62, Math.max(0.35, hsl.l));
        m.material.color.setHSL((hsl.h + shift) % 1, s, l);
      }
      break;
    }
    default: break;
  }
}

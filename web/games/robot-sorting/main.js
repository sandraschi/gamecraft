// Boomy sorts parcels: write a few IF ... THEN rules, run a shift, see which
// parcels landed in the wrong bin, adjust, run again. First matching rule wins.
import * as THREE from 'three';
import { VERIFY, $, seedFromUrl, makeRenderer, harness, sfx, toast, wireHelp, load, save } from '../_shared/gc.js';
import { setupMusic } from '../_shared/music.js';
const music = setupMusic('factory', { sfx });
import * as R from './rules.js';

const SAVE_KEY = 'gamecraft.boomy.v1';
const COLOR_HEX = { red: 0xd8443a, blue: 0x3a6ad8, green: 0x4ab85a, yellow: 0xf2c43a };
const BELT_Z0 = -9, JUNCTION_Z = 0;

const canvas = $('view');
const renderer = makeRenderer(canvas, { shadows: !VERIFY, clear: 0xdfe6ee });
const scene = new THREE.Scene();
scene.background = new THREE.Color(0xdfe6ee);
const camera = new THREE.PerspectiveCamera(42, 1, 0.1, 100);
scene.add(new THREE.HemisphereLight(0xffffff, 0x8a8a90, 1.3));
const sun = new THREE.DirectionalLight(0xffffff, 1.8); sun.position.set(6, 12, 8);
if (!VERIFY) { sun.castShadow = true; sun.shadow.mapSize.set(2048, 2048); Object.assign(sun.shadow.camera, { left: -12, right: 12, top: 12, bottom: -12 }); }
scene.add(sun);
const M = (c, r = 0.6, m = 0) => new THREE.MeshStandardMaterial({ color: c, roughness: r, metalness: m });
function box(w, h, d, c, x, y, z, parent = scene, mat) { const m = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), mat ?? M(c)); m.position.set(x, y, z); m.castShadow = true; m.receiveShadow = true; parent.add(m); return m; }

// floor with hazard tape, conveyor, junction plate, bins
const floor = new THREE.Mesh(new THREE.PlaneGeometry(40, 40), M(0xb8bcc2, 0.9)); floor.rotation.x = -Math.PI / 2; floor.receiveShadow = true; scene.add(floor);
const beltTex = (() => {
  const c = document.createElement('canvas'); c.width = 64; c.height = 64;
  const x = c.getContext('2d'); x.fillStyle = '#2a2c30'; x.fillRect(0, 0, 64, 64); x.fillStyle = '#3a3c42'; for (let i = 0; i < 4; i++) x.fillRect(0, i * 16, 64, 6);
  const t = new THREE.CanvasTexture(c); t.wrapS = t.wrapT = THREE.RepeatWrapping; t.repeat.set(1, 8); return t;
})();
const belt = box(1.6, 0.5, 9.4, 0, 0, 0.25, -4.7, scene, new THREE.MeshStandardMaterial({ map: beltTex, roughness: 0.8 }));
box(1.8, 0.45, 9.4, 0x8a8e96, 0, 0.2, -4.7).scale.y = 0.95;
belt.position.y = 0.27;
const plate = new THREE.Mesh(new THREE.CylinderGeometry(1.1, 1.1, 0.5, 32), M(0x9aa0a8, 0.4, 0.5)); plate.position.set(0, 0.25, 0); plate.receiveShadow = true; scene.add(plate);
const outBelt = box(1.6, 0.5, 3.2, 0, 0, 0.27, 2.7, scene, new THREE.MeshStandardMaterial({ map: beltTex, roughness: 0.8 }));
void outBelt;
const BINS = {
  left: { x: -3.1, z: 0, label: 'LEFT', color: 0xe8a03a },
  right: { x: 3.1, z: 0, label: 'RIGHT', color: 0x3ab8c8 },
  pass: { x: 0, z: 5.2, label: 'ON', color: 0x8a8a96 },
};
for (const b of Object.values(BINS)) {
  const g = new THREE.Group(); g.position.set(b.x, 0, b.z); scene.add(g);
  const mat = M(b.color, 0.5);
  box(2.2, 0.1, 2.2, 0, 0, 0.05, 0, g, mat);
  box(2.2, 0.9, 0.1, 0, 0, 0.45, -1.05, g, mat); box(2.2, 0.9, 0.1, 0, 0, 0.45, 1.05, g, mat);
  box(0.1, 0.9, 2.2, 0, -1.05, 0.45, 0, g, mat); box(0.1, 0.9, 2.2, 0, 1.05, 0.45, 0, g, mat);
  b.group = g; b.count = 0; b.wrong = 0;
}

// Boomy: a little four-wheeled robot car with a pan-tilt camera head and a pusher arm
// he stands beside the junction plate, facing it
const BOOMY_YAW = Math.atan2(-1.6, 1.5);
const boomy = new THREE.Group(); boomy.position.set(1.6, 0, -1.5); boomy.rotation.y = BOOMY_YAW; scene.add(boomy);
box(0.9, 0.22, 0.62, 0x1e2024, 0, 0.2, 0, boomy);
box(0.86, 0.03, 0.58, 0x9ad8f2, 0, 0.34, 0, boomy, new THREE.MeshStandardMaterial({ color: 0x9ad8f2, transparent: true, opacity: 0.55, roughness: 0.2 }));
for (const [x, z] of [[-0.33, 0.36], [0.33, 0.36], [-0.33, -0.36], [0.33, -0.36]]) { const w = new THREE.Mesh(new THREE.CylinderGeometry(0.15, 0.15, 0.12, 16), M(0xf2c43a, 0.6)); w.rotation.x = Math.PI / 2; w.position.set(x, 0.15, z); w.castShadow = true; boomy.add(w); }
const head = new THREE.Group(); head.position.set(0, 0.55, 0.1); boomy.add(head);
box(0.06, 0.3, 0.06, 0x2a2a2e, 0, -0.12, 0, head);
box(0.3, 0.16, 0.18, 0x2a2a2e, 0, 0.06, 0, head);
const lens = new THREE.Mesh(new THREE.CylinderGeometry(0.05, 0.05, 0.06, 16), M(0x0a0a0a, 0.1, 0.6)); lens.rotation.x = Math.PI / 2; lens.position.set(0, 0.06, 0.11); head.add(lens);
const led = new THREE.Mesh(new THREE.SphereGeometry(0.035, 10, 8), new THREE.MeshStandardMaterial({ color: 0x3a8aff, emissive: 0x3a8aff, emissiveIntensity: 1.5 })); led.position.set(0.11, 0.15, 0.05); head.add(led);
const arm = new THREE.Group(); arm.position.set(0, 0.32, 0.32); boomy.add(arm);
box(0.08, 0.08, 1.7, 0xc8ccd2, 0, 0, 0.85, arm, M(0xc8ccd2, 0.3, 0.7));
box(0.5, 0.35, 0.06, 0xd8443a, 0, -0.05, 1.72, arm);

// parcels
const parcelGroup = new THREE.Group(); scene.add(parcelGroup);
const starTex = (() => {
  const c = document.createElement('canvas'); c.width = c.height = 64; const x = c.getContext('2d');
  x.fillStyle = '#ffe23a'; x.beginPath(); for (let i = 0; i < 10; i++) { const a = -Math.PI / 2 + i * Math.PI / 5, r = i % 2 ? 12 : 28; x.lineTo(32 + Math.cos(a) * r, 32 + Math.sin(a) * r); } x.fill();
  x.strokeStyle = '#8a6a00'; x.lineWidth = 2; x.stroke(); return new THREE.CanvasTexture(c);
})();
function parcelMesh(p) {
  const s = p.size === 'big' ? 0.85 : 0.5;
  const g = new THREE.Group();
  const b = box(s, s * 0.8, s, COLOR_HEX[p.color], 0, s * 0.4, 0, g, M(COLOR_HEX[p.color], 0.75));
  box(s + 0.01, 0.05, s * 0.18, 0xe8dcc0, 0, s * 0.8, 0, g, M(0xe8dcc0, 0.8)); // tape
  if (p.star) { const st = new THREE.Mesh(new THREE.PlaneGeometry(s * 0.6, s * 0.6), new THREE.MeshBasicMaterial({ map: starTex, transparent: true })); st.rotation.x = -Math.PI / 2; st.position.y = s * 0.8 + 0.03; g.add(st); }
  void b;
  return g;
}

// ------------------------------------------------------------------ rule editor (HTML)
const ruleRows = [];
function buildEditor(L) {
  const host = $('rules'); host.innerHTML = '';
  ruleRows.length = 0;
  for (let i = 0; i < L.slots; i++) {
    const row = document.createElement('div'); row.className = 'rule';
    const mk = (opts, cls) => { const s = document.createElement('select'); s.className = cls; for (const [v, t] of opts) { const o = document.createElement('option'); o.value = v; o.textContent = t; s.appendChild(o); } return s; };
    const a = mk([['', '(no rule)'], ...R.CONDS.map((c) => [c.id, c.text])], 'a');
    const b = mk([['', '...'], ...R.CONDS.filter((c) => c.id !== 'any').map((c) => [c.id, `and ${c.text}`])], 'b');
    const to = mk([['left', 'LEFT bin'], ['right', 'RIGHT bin'], ['pass', 'ride ON']], 'to');
    const n = document.createElement('span'); n.className = 'n'; n.textContent = `${i + 1}`;
    const ifs = document.createElement('span'); ifs.textContent = 'IF'; const then = document.createElement('span'); then.textContent = 'THEN';
    row.append(n, ifs, a, b, then, to);
    host.appendChild(row);
    const sync = () => { b.disabled = !a.value; to.disabled = !a.value; row.classList.toggle('off', !a.value); };
    a.addEventListener('change', sync); sync();
    ruleRows.push({ a, b, to, sync });
  }
}
function readProgram() { return ruleRows.map((r) => (r.a.value ? { a: r.a.value, b: r.b.value || undefined, to: r.to.value } : null)).filter(Boolean); }
function writeProgram(prog) {
  ruleRows.forEach((r, i) => { const p = prog[i]; r.a.value = p?.a ?? ''; r.b.value = p?.b ?? ''; r.to.value = p?.to ?? 'left'; r.sync(); });
}

// ------------------------------------------------------------------ level / run
let level = 0, seed = seedFromUrl(2026), run = null, runs = 0, lastScore = null, levelDone = false, doneT = 0;
const meta = load(SAVE_KEY, { levels: 0 });
function startLevel(i) {
  level = i;
  const L = R.LEVELS[level];
  buildEditor(L);
  $('goal-title').textContent = `${level + 1}. ${L.title}`;
  $('goal').textContent = L.goal;
  $('result').textContent = '';
  clearParcels();
  runs = 0; lastScore = null; levelDone = false; doneT = 0;
  $('done').hidden = true;
  setEditable(true);
}
function clearParcels() { for (const c of [...parcelGroup.children]) parcelGroup.remove(c); for (const b of Object.values(BINS)) { b.count = 0; b.wrong = 0; } }
function setEditable(on) { for (const r of ruleRows) { r.a.disabled = !on; r.sync(); if (!on) { r.b.disabled = true; r.to.disabled = true; } } $('run').disabled = !on; }
function startRun() {
  if (run || levelDone) return false;
  clearParcels();
  const program = readProgram();
  const parcels = R.shift(level, seed * 31 + level * 7 + runs * 0);   // the same shift each try, so changes are comparable
  run = { program, parcels, items: [], next: 0, spawnT: 0, correct: 0, finished: 0 };
  runs++;
  setEditable(false);
  $('result').textContent = 'Running...';
  return true;
}
$('run').addEventListener('click', startRun);
$('next').addEventListener('click', () => { if (level + 1 < R.LEVELS.length) startLevel(level + 1); else { seed = (seed * 7 + 3) % 2147483647; startLevel(0); } });

const SPEED = 4.2, GAP = 0.75;
let armTarget = 0;
function tick(dt) {
  beltTex.offset.y -= dt * SPEED / 9.4 * 8 * (run ? 1 : 0.15);
  if (run) {
    run.spawnT -= dt;
    const busy = run.items.some((it) => it.phase === 'decide');
    if (run.next < run.parcels.length && run.spawnT <= 0 && !busy && !run.items.some((it) => it.phase === 'belt' && it.mesh.position.z < BELT_Z0 + 1.6)) {
      const p = run.parcels[run.next++];
      const mesh = parcelMesh(p); mesh.position.set(0, 0.5, BELT_Z0); parcelGroup.add(mesh);
      run.items.push({ p, mesh, phase: 'belt', t: 0 });
      run.spawnT = GAP;
    }
    for (const it of run.items) {
      const m = it.mesh;
      if (it.phase === 'belt') {
        // queue behind whatever is ahead
        const ahead = run.items.filter((o) => o !== it && o.phase === 'belt' && o.mesh.position.z > m.position.z).map((o) => o.mesh.position.z);
        const limit = Math.min(JUNCTION_Z, ...ahead.map((z) => z - 1.05));
        m.position.z = Math.min(limit, m.position.z + SPEED * dt);
        if (m.position.z >= JUNCTION_Z - 1e-3 && !run.items.some((o) => o.phase === 'decide' || o.phase === 'push')) { it.phase = 'decide'; it.t = 0; it.dest = R.route(run.program, it.p); }
      } else if (it.phase === 'decide') {
        it.t += dt;
        head.rotation.x = -0.4; led.material.color.setHex(0xffd03a); led.material.emissive.setHex(0xffd03a);
        armTarget = it.dest === 'left' ? 0.9 : it.dest === 'right' ? -0.9 : 0;
        if (it.t > 0.28) { it.phase = 'push'; it.t = 0; it.from = m.position.clone(); const b = BINS[it.dest]; it.to = new THREE.Vector3(b.x + (Math.random() - 0.5) * 0.6, 0.12, b.z + (Math.random() - 0.5) * 0.6); }
      } else if (it.phase === 'push') {
        it.t = Math.min(1, it.t + dt * 1.8);
        m.position.lerpVectors(it.from, it.to, it.t);
        m.position.y = it.from.y + (it.to.y - it.from.y) * it.t + Math.sin(it.t * Math.PI) * 0.6;
        m.rotation.y += dt * (it.dest === 'pass' ? 0 : 2);
        if (it.t >= 1) {
          it.phase = 'binned';
          const ok = it.dest === R.LEVELS[level].want(it.p);
          const b = BINS[it.dest]; b.count++;
          if (ok) run.correct++; else { b.wrong++; markWrong(m); }
          run.finished++;
          sfx.tone(ok ? 660 : 220, 0.12, { type: ok ? 'sine' : 'square', vol: ok ? 0.05 : 0.03 });
          head.rotation.x = 0; led.material.color.setHex(0x3a8aff); led.material.emissive.setHex(0x3a8aff); armTarget = 0;
        }
      }
    }
    if (run.finished === run.parcels.length) {
      lastScore = run.correct;
      const total = run.parcels.length;
      if (run.correct === total) {
        levelDone = true; doneT = 0; meta.levels = Math.max(meta.levels, level + 1); save(SAVE_KEY, meta);
        $('result').textContent = `${total} of ${total} in the right bin.`;
        setTimeout(() => sfx.done(), 200);
      } else {
        $('result').textContent = `${run.correct} of ${total} in the right bin. The ones with a red flag went wrong.`;
        toast('Change a rule and run the shift again.', 2400);
        setEditable(true);
      }
      run = null;
    }
  }
  arm.rotation.y += (armTarget - arm.rotation.y) * Math.min(1, dt * 10);
  if (levelDone) {
    doneT += dt;
    boomy.rotation.y = BOOMY_YAW + Math.sin(doneT * 6) * 0.2;
    if (doneT > 1 && $('done').hidden) {
      $('done-text').textContent = `Sorted with ${readProgram().length} rule${readProgram().length === 1 ? '' : 's'}, ${runs} run${runs === 1 ? '' : 's'}.`;
      $('next').textContent = level + 1 < R.LEVELS.length ? 'Next job' : 'Start over';
      $('done').hidden = false;
    }
  } else boomy.rotation.y = BOOMY_YAW;
}
function markWrong(m) {
  const pole = box(0.03, 0.7, 0.03, 0x2a2a2a, 0, 0.9, 0, m);
  const flag = box(0.28, 0.18, 0.02, 0xe8302a, 0.14, 1.15, 0, m); void pole; void flag;
}
function render() { renderer.render(scene, camera); }
function resize() {
  renderer.setSize(innerWidth, innerHeight, false);
  camera.aspect = innerWidth / innerHeight; camera.fov = camera.aspect < 1 ? 66 : 42; camera.updateProjectionMatrix();
  // leave room for the rule panel on the left
  camera.position.set(-2.8, 11.5, 12.5); camera.lookAt(-1.5, 0, -0.2); camera.updateMatrixWorld();
}
addEventListener('resize', resize); resize();

// ------------------------------------------------------------------ tour
// Uses the real selects and the Run button. Runs the first job with no rules
// (most parcels go astray), fixes it, does job two, then on job three tries the
// rules in the wrong order before getting the order right.
const bot = { t: 0, step: 0, scores: [] };
const plan = [
  { level: 0, prog: [] },
  { level: 0, prog: R.LEVELS[0].answer },
  { level: 1, prog: R.LEVELS[1].answer },
  { level: 2, prog: R.LEVELS[2].naive },
  { level: 2, prog: R.LEVELS[2].answer },
];
function setSelect(sel, v) { sel.value = v; sel.dispatchEvent(new Event('change')); }
function botStep(dt) {
  if (run) return;
  bot.t += dt;
  if (bot.t < 0.4) return;
  bot.t = 0;
  if (levelDone) { if (!$('done').hidden && bot.step < plan.length) $('next').click(); return; }
  const p = plan[bot.step];
  if (!p || p.level !== level) return;
  if (lastScore !== null && bot.scores.length < bot.step) bot.scores.push(lastScore);
  ruleRows.forEach((r, i) => { const rule = p.prog[i]; setSelect(r.a, rule?.a ?? ''); setSelect(r.b, rule?.b ?? ''); setSelect(r.to, rule?.to ?? 'left'); });
  $('run').click();
  bot.step++;
}
const total = () => R.shift(level, seed * 31 + level * 7).length;
const checks = [
  { name: 'factory-floor', when: () => true, check: () => ruleRows.length === R.LEVELS[0].slots && readProgram().length === 0 },
  { name: 'no-rules', when: () => bot.step === 1 && !run && lastScore !== null, check: () => lastScore < total() && BINS.pass.count === total() && !levelDone },
  { name: 'red-left', when: () => levelDone && level === 0, check: () => BINS.left.count > 0 && BINS.left.wrong + BINS.right.wrong + BINS.pass.wrong === 0 && runs === 2 },
  { name: 'two-colours', when: () => levelDone && level === 1, check: () => runs === 1 && BINS.right.count > 0 },
  { name: 'order-matters', when: () => level === 2 && bot.step === 4 && !run && lastScore !== null, check: () => lastScore < total() && !levelDone },
  { name: 'big-ones', when: () => levelDone && level === 2 && doneT > 1.1, check: () => runs === 2 && !$('done').hidden },
];

startLevel(0);
const help = wireHelp();
const api = harness({
  checks, tick, render, selfTest: R.selfTest, bot: botStep,
  paused: () => help.isOpen(),
  extraState: () => ({ level, runs, lastScore, program: readProgram() }),
  winNote: () => 'three jobs sorted; an empty program and a wrong rule order were both caught by the bins',
  stuckSteps: 9000,
});
if (!VERIFY && meta.levels === 0 && !new URLSearchParams(location.search).has('autoplay')) help.open();
api.start();

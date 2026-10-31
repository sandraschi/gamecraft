// Kaffeehaus simulation: pure logic, no three.js, no DOM.
// Deterministic for a given seed, meta (save state) and sequence of actions.
// Time unit: game minutes. At 1x speed one real second is one game minute.

export const MIN_PER_SEC = 1;
export const TICK = 1 / 60; // game minutes per fixed step (one 60 Hz frame at 1x)
export const DAY_START = 7 * 60;
export const DOORS_CLOSE = 18 * 60 + 30;
export const DAY_END = 19 * 60 + 30;
export const WAITER_SPEED = 3.5; // metres per game minute
export const GUEST_SPEED = 1.6;

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

export const MENU = {
  melange: { name: 'Melange', price: 4.9, brew: 3, kind: 'coffee' },
  brauner: { name: 'Kleiner Brauner', price: 3.6, brew: 2, kind: 'coffee' },
  einspaenner: { name: 'Einspänner', price: 5.4, brew: 4, kind: 'coffee' },
  verlaengerter: { name: 'Verlängerter', price: 4.2, brew: 3, kind: 'coffee' },
  sacher: { name: 'Sachertorte', price: 5.9, kind: 'cake' },
  strudel: { name: 'Apfelstrudel', price: 5.2, kind: 'cake' },
  topfen: { name: 'Topfenstrudel', price: 5.2, kind: 'cake' },
};
export const CAKES = ['sacher', 'strudel', 'topfen'];

export const LAYOUT = {
  room: [-8, 8, -6, 6],
  outside: { x: 0, z: 7.5 },
  door: { x: 0, z: 5.7 },
  queue: [{ x: 0, z: 5.0 }, { x: -0.9, z: 5.25 }, { x: 0.9, z: 5.25 }, { x: -1.8, z: 5.45 }, { x: 1.8, z: 5.45 }, { x: 0, z: 5.6 }],
  pickup: { x: 1.2, z: -4.1 },
  rack: { x: -6.7, z: -1.4 },
  home: { x: 0.6, z: -3.4 },
  piccoloHome: { x: 3.2, z: -3.6 },
  tables: [
    { id: 1, x: -5.6, z: -3.4 },
    { id: 2, x: -5.6, z: 0.2 },
    { id: 3, x: -5.6, z: 3.9, window: true },
    { id: 4, x: -2.4, z: -1.4 },
    { id: 5, x: -2.6, z: 2.2 },
    { id: 6, x: 1.0, z: 0.6 },
    { id: 7, x: 4.4, z: -2.0 },
    { id: 8, x: 4.6, z: 1.6 },
    { id: 9, x: 5.8, z: 4.4, window: true },
    { id: 10, x: -3.0, z: 4.7, window: true },
  ],
  extraTable: { id: 11, x: 3.0, z: 4.5, window: true },
};
export const seatPos = (t) => ({ x: t.x - 0.65, z: t.z });
export const seat2Pos = (t) => ({ x: t.x + 0.65, z: t.z });
export const servePos = (t) => ({ x: t.x + 0.2, z: t.z + 0.8 });

const TYPES = {
  regular: { label: 'Stammgast', patience: { queue: 20, order: 15, serve: 28, bill: 18 }, mood: 75, tip: 1.3 },
  tourist: { label: 'Tourist', patience: { queue: 12, order: 12, serve: 20, bill: 12 }, mood: 65, tip: 1.0 },
  business: { label: 'Geschäftsmann', patience: { queue: 8, order: 8, serve: 16, bill: 8 }, mood: 60, tip: 1.1 },
  student: { label: 'Studentin', patience: { queue: 18, order: 20, serve: 30, bill: 22 }, mood: 65, tip: 0.4 },
};
export const TYPE_LABEL = Object.fromEntries(Object.entries(TYPES).map(([k, v]) => [k, v.label]));

export const REGULARS = [
  { key: 'weiss', name: 'Hofrat Weiss', usual: ['melange'], table: 4, arrive: 8 * 60 + 20, stay: 120, paper: true,
    happy: 'Ah, mein Tisch. Und die Presse. Sehr gut.', hurt: 'Die Rechnung? Ich bin doch kein Tourist!', taken: 'Da sitzt ja jemand an meinem Tisch!' },
  { key: 'lenz', name: 'Frau Professor Lenz', usual: ['einspaenner', 'strudel'], table: 2, arrive: 10 * 60, stay: 100, paper: true,
    happy: 'Wie immer, danke. Sie sind ein Schatz.', hurt: 'So eilig haben wir es heute?', taken: 'Mein Fensterblick... na gut.' },
  { key: 'ostrowski', name: 'Herr Ostrowski', usual: ['brauner'], table: 7, arrive: 13 * 60, stay: 150, paper: false,
    happy: 'Schach gegen mich selbst. Ich gewinne immer. Und verliere.', hurt: 'Mitten in der Partie!', taken: 'Mein Brett passt auf keinen anderen Tisch.' },
  { key: 'mitzi', name: 'Fräulein Mitzi', usual: ['verlaengerter'], table: 3, arrive: 15 * 60, stay: 130, paper: false,
    happy: 'Ein Gedicht auf der Serviette, für Sie.', hurt: 'Die Muse war gerade da...', taken: 'Ohne Fenster keine Verse.' },
  { key: 'poeschl', name: 'Dr. Pöschl', usual: ['melange', 'sacher'], table: 8, arrive: 16 * 60 + 40, stay: 80, paper: true,
    happy: 'Die beste Sacher der Stadt. Sagen Sie es nicht dem Hotel.', hurt: 'Ich wollte noch den Leitartikel lesen.', taken: 'Ich nehme, was frei ist. Seufz.' },
];

// Expected arrivals per hour, 7:00 .. 18:00.
const RATES = {
  business: [3, 3, 1, 0.5, 1, 0.5, 0.3, 0.3, 0.5, 0.5, 0.3, 0],
  tourist: [0, 0.5, 1.5, 2.5, 3, 3, 2.5, 2.5, 3, 2.5, 1.5, 0.5],
  student: [0, 0.3, 0.5, 0.5, 1, 1, 1, 1, 1, 1, 1.5, 1],
};

export const UPGRADES = [
  { key: 'machine', name: 'Second brass machine', desc: 'Two more brewing groups.', cost: 140 },
  { key: 'vitrine', name: 'Bigger vitrine', desc: 'Half again as much cake every morning.', cost: 70 },
  { key: 'piccolo', name: 'Hire a Piccolo', desc: 'An apprentice who carries ready orders to tables.', cost: 220 },
  { key: 'table', name: 'Extra window table', desc: 'Table 11, by the window. Tourists love it.', cost: 160 },
];
export const RENT = 90;

export function defaultMeta() {
  return {
    day: 1, cash: 150, reputation: 3,
    loyalty: Object.fromEntries(REGULARS.map((r) => [r.key, 3])),
    upgrades: { machine: false, vitrine: false, piccolo: false, table: false },
    history: [],
  };
}

const dist = (a, b) => Math.hypot(a.x - b.x, a.z - b.z);
const round2 = (v) => Math.round(v * 100) / 100;

// ------------------------------------------------------------------ the sim
export function createSim({ seed = 1, meta = defaultMeta(), policy = null } = {}) {
  const rng = mulberry32((seed * 7919 + meta.day * 104729) >>> 0);
  const tables = LAYOUT.tables.map((t) => ({ ...t, guest: null, reserved: null }));
  if (meta.upgrades.table) tables.push({ ...LAYOUT.extraTable, guest: null, reserved: null });
  const cakeBase = meta.upgrades.vitrine ? 12 : 8;
  const sim = {
    seed, meta, policy, rng,
    t: DAY_START, over: false,
    tables,
    guests: [], nextId: 1,
    arrivals: [],
    waiter: { x: LAYOUT.home.x, z: LAYOUT.home.z, task: null, leg: 0, legT: 0, carrying: null, busy: 0 },
    piccolo: meta.upgrades.piccolo ? { x: LAYOUT.piccoloHome.x, z: LAYOUT.piccoloHome.z, task: null, leg: 0, legT: 0, carrying: null } : null,
    queue: [],
    brew: { slots: meta.upgrades.machine ? 4 : 2, active: [], waiting: [] },
    pass: [],
    cakes: { sacher: cakeBase, strudel: cakeBase, topfen: Math.round(cakeBase * 0.75) },
    events: [],
    stats: { revenue: 0, tips: 0, paid: 0, walkouts: 0, angry: 0, moods: [], cakesSold: 0, regulars: {} },
  };
  scheduleArrivals(sim);
  Object.assign(sim, {
    tick: (dt) => tick(sim, dt),
    actionsFor: (g) => actionsFor(sim, g),
    enqueue: (task) => enqueue(sim, task),
    freeTables: () => sim.tables.filter((t) => !t.guest && !t.reserved),
    report: () => report(sim),
  });
  return sim;
}

function scheduleArrivals(sim) {
  const { meta, rng } = sim;
  const scale = 0.5 * (1 + 0.12 * (meta.day - 1));
  const touristScale = 0.6 + meta.reputation * 0.15;
  for (const [type, rates] of Object.entries(RATES)) {
    rates.forEach((r, h) => {
      const expected = r * scale * (type === 'tourist' ? touristScale : 1);
      let n = Math.floor(expected);
      if (rng() < expected - n) n++;
      for (let i = 0; i < n; i++) sim.arrivals.push({ t: DAY_START + h * 60 + rng() * 60, type });
    });
  }
  for (const r of REGULARS) {
    const loyal = meta.loyalty[r.key] ?? 3;
    if (loyal <= 0) continue;
    if (loyal === 1 && rng() < 0.5) continue;
    sim.arrivals.push({ t: r.arrive + (rng() - 0.5) * 20, type: 'regular', regular: r });
  }
  sim.arrivals = sim.arrivals.filter((a) => a.t < DOORS_CLOSE).sort((a, b) => a.t - b.t);
}

function log(sim, text, kind = 'info', guest = null) {
  sim.eventSeq = (sim.eventSeq ?? 0) + 1;
  sim.events.push({ seq: sim.eventSeq, t: sim.t, text, kind, guest: guest?.id ?? null });
  if (sim.events.length > 200) sim.events.shift();
}

function spawn(sim, a) {
  const { rng } = sim;
  const T = TYPES[a.type];
  const g = {
    id: sim.nextId++, type: a.type, regular: a.regular ?? null,
    name: a.regular ? a.regular.name : `${T.label} ${sim.nextId - 1}`,
    size: a.type === 'tourist' && rng() < 0.4 ? 2 : 1,
    state: 'queue', since: sim.t, mood: T.mood, patience: T.patience,
    x: LAYOUT.outside.x + (rng() - 0.5), z: LAYOUT.outside.z, tx: null, tz: null,
    table: null, order: [], billed: [], rounds: 0, ready: 0, readySince: null, stay: 0, left: 0,
    wantsPaper: a.regular ? a.regular.paper : a.type === 'student' ? rng() < 0.6 : a.type === 'business' ? rng() < 0.5 : false,
    hasPaper: false, hurried: false, paid: 0, look: Math.floor(rng() * 1000),
  };
  // what they will order
  const pick = (arr) => arr[Math.floor(rng() * arr.length)];
  const items = [];
  if (g.regular) items.push(...g.regular.usual);
  else if (g.type === 'tourist') for (let i = 0; i < g.size; i++) { items.push(pick(['melange', 'melange', 'einspaenner', 'brauner'])); if (rng() < 0.7) items.push(pick(CAKES)); }
  else if (g.type === 'business') items.push(pick(['brauner', 'verlaengerter', 'brauner']));
  else { items.push(rng() < 0.7 ? 'verlaengerter' : 'melange'); if (rng() < 0.15) items.push(pick(CAKES)); }
  g.wish = items;
  g.stay = g.regular ? g.regular.stay : g.type === 'tourist' ? 25 + rng() * 20 : g.type === 'business' ? 12 + rng() * 8 : 60 + rng() * 60;
  sim.guests.push(g);
  walkTo(g, queueSpot(sim, g));
  if (g.regular) sim.stats.regulars[g.regular.key] = { name: g.name, came: true, mood: null, hurried: false, angry: false };
  log(sim, g.regular ? `${g.name} comes in.` : `${g.size === 2 ? 'Two tourists' : TYPE_LABEL[g.type]} at the door.`, 'arrive', g);
}

function queueSpot(sim, g) {
  const waiting = sim.guests.filter((o) => o.state === 'queue');
  const i = Math.max(0, waiting.indexOf(g));
  return LAYOUT.queue[Math.min(i, LAYOUT.queue.length - 1)];
}
function walkTo(g, p) { g.tx = p.x; g.tz = p.z; }

// ------------------------------------------------------------------ actions
export function actionsFor(sim, g) {
  if (!g || g.state === 'gone' || g.state === 'leaving') return [];
  const queued = (kind) => sim.queue.some((t) => t.guest === g.id && t.kind === kind)
    || [sim.waiter.task, sim.piccolo?.task].some((t) => t && t.kind === kind && (t.guest === g.id || t.batch?.includes(g.id)));
  const primaryQueued = ['seat', 'order', 'serve', 'bill', 'hurry'].some(queued);
  const out = [];
  if (!primaryQueued) {
    if (g.state === 'queue' && sim.freeTables().length) out.push('seat');
    if (g.state === 'seated') out.push('order');
    if (g.state === 'ordered' && g.ready === g.order.length && g.order.length) out.push('serve');
    if (g.state === 'wantsBill') out.push('bill');
    if (g.state === 'eating' && !g.hurried) out.push('hurry');
  }
  if (g.wantsPaper && !g.hasPaper && g.table && ['seated', 'ordered', 'eating'].includes(g.state) && !queued('paper')) out.push('paper');
  return out;
}

export function enqueue(sim, task) {
  const g = sim.guests.find((o) => o.id === task.guest);
  if (!g || !actionsFor(sim, g).includes(task.kind)) return false;
  if (task.kind === 'seat') {
    const t = sim.tables.find((x) => x.id === task.table);
    if (!t || t.guest || t.reserved) return false;
    t.reserved = g.id;
  }
  sim.queue.push({ ...task, at: sim.t });
  return true;
}

// Legs: [{to}] walk, [{wait}] stand still, then the task's effect fires.
function planLegs(sim, task) {
  const g = sim.guests.find((o) => o.id === task.guest);
  if (!g) return null;
  const table = sim.tables.find((t) => t.id === (task.table ?? g.table));
  switch (task.kind) {
    case 'seat': return [{ toGuest: true }, { wait: 0.3 }, { to: servePos(table), lead: true }, { wait: 1.0 }];
    case 'order': return [{ to: servePos(table) }, { wait: 1.0 }];
    case 'serve': return [{ to: LAYOUT.pickup }, { wait: 0.3 }, { to: servePos(table) }, { wait: 0.5 }];
    case 'paper': return [{ to: LAYOUT.rack }, { wait: 0.3 }, { to: servePos(table) }, { wait: 0.3 }];
    case 'bill': case 'hurry': return [{ to: servePos(table) }, { wait: 1.4 }];
    default: return null;
  }
}

function valid(sim, task) {
  const g = sim.guests.find((o) => o.id === task.guest);
  if (!g || g.state === 'gone' || g.state === 'leaving') return false;
  const need = { seat: ['queue'], order: ['seated'], serve: ['ordered'], paper: ['seated', 'ordered', 'eating'], bill: ['wantsBill'], hurry: ['eating'] }[task.kind];
  return need.includes(g.state);
}

function readyForTray(sim, g) {
  return g.state === 'ordered' && g.order.length && g.ready === g.order.length;
}
function startTask(sim, worker, task) {
  worker.task = task;
  worker.legs = planLegs(sim, task);
  if (task.kind === 'serve' && worker.legs) {
    // One tray, up to three tables: every fully ready order nobody else is carrying.
    const other = worker === sim.waiter ? sim.piccolo : sim.waiter;
    const carriedElsewhere = new Set(other?.task?.batch ?? (other?.task ? [other.task.guest] : []));
    const first = sim.guests.find((o) => o.id === task.guest);
    const extra = sim.guests
      .filter((o) => o !== first && readyForTray(sim, o) && !carriedElsewhere.has(o.id))
      .sort((a, b) => dist(a, first) - dist(b, first))
      .slice(0, 2);
    const batch = [first, ...extra];
    sim.queue = sim.queue.filter((t) => !(t.kind === 'serve' && batch.some((b) => b.id === t.guest)));
    task.batch = batch.map((b) => b.id);
    worker.legs = [{ to: LAYOUT.pickup }, { wait: 0.3 }];
    let from = LAYOUT.pickup;
    const left = [...batch];
    while (left.length) {
      left.sort((a, b) => dist(servePos(tableOf(sim, a)), from) - dist(servePos(tableOf(sim, b)), from));
      const g = left.shift();
      const p = servePos(tableOf(sim, g));
      worker.legs.push({ to: p }, { wait: 0.4, deliver: g.id });
      from = p;
    }
  }
  worker.leg = 0;
  worker.legT = 0;
  if (task.kind === 'serve') worker.carrying = 'tray';
  if (task.kind === 'paper') worker.carrying = 'paper';
}
function abortTask(sim, worker) {
  const task = worker.task;
  if (task?.kind === 'seat') {
    const t = sim.tables.find((x) => x.id === task.table);
    if (t && t.reserved === task.guest) t.reserved = null;
  }
  worker.task = null;
  worker.carrying = null;
}

function finishTask(sim, worker) {
  const task = worker.task;
  const g = sim.guests.find((o) => o.id === task.guest);
  worker.task = null;
  worker.carrying = null;
  if (!g || !valid(sim, task)) return;
  const T = TYPES[g.type];
  const table = sim.tables.find((t) => t.id === (task.table ?? g.table));
  switch (task.kind) {
    case 'seat': {
      table.reserved = null;
      table.guest = g.id;
      g.table = table.id;
      g.state = 'seated';
      g.since = sim.t;
      const p = seatPos(table);
      g.x = p.x; g.z = p.z; walkTo(g, p);
      if (g.regular) {
        if (g.regular.table === table.id) { g.mood += 10; log(sim, `${g.name}: "${g.regular.happy}"`, 'happy', g); }
        else { g.mood -= 15; log(sim, `${g.name}: "${g.regular.taken}"`, 'upset', g); }
      } else if (table.window && g.type === 'tourist') { g.mood += 10; log(sim, 'Tourists get the window. Photos are taken.', 'happy', g); }
      // Menu with the chair: the order is taken in the same visit.
      g.mood = Math.max(0, Math.min(100, g.mood));
      takeOrder(sim, g);
      break;
    }
    case 'order': takeOrder(sim, g); break;
    case 'serve': break; // delivered leg by leg, see serveOne
    case 'paper': {
      g.hasPaper = true;
      g.mood += 12;
      if (g.regular) g.left += 15;
      break;
    }
    case 'hurry': {
      g.hurried = true;
      // The sting scales with how much of their visit you cut short.
      const cut = Math.max(0, Math.min(1, g.left / Math.max(1, g.stay)));
      const hit = { regular: -30, student: -12, tourist: -8, business: 0 }[g.type] - 30 * cut;
      g.mood += hit;
      if (g.regular) { log(sim, `${g.name}: "${g.regular.hurt}"`, 'upset', g); sim.stats.regulars[g.regular.key].hurried = true; }
      pay(sim, g, T);
      break;
    }
    case 'bill': {
      pay(sim, g, T);
      break;
    }
  }
  g.mood = Math.max(0, Math.min(100, g.mood));
}

function takeOrder(sim, g) {
  g.order = [];
  g.rounds++;
  for (const item of g.wish) {
    if (MENU[item].kind === 'cake') {
      if (sim.cakes[item] > 0) { sim.cakes[item]--; g.order.push(item); g.billed.push(item); sim.pass.push({ item, guest: g.id, at: sim.t }); }
      else { g.mood -= 8; log(sim, `No ${MENU[item].name} left. ${g.name} is disappointed.`, 'upset', g); }
    } else {
      g.order.push(item);
      g.billed.push(item);
      sim.brew.waiting.push({ item, guest: g.id });
    }
  }
  g.ready = g.order.filter((i) => MENU[i].kind === 'cake').length;
  if (g.ready === g.order.length) g.readySince = sim.t;
  g.state = 'ordered';
  g.since = sim.t;
  if (!g.order.length) { g.state = 'eating'; g.left = 5; }
}

function tableOf(sim, g) { return sim.tables.find((t) => t.id === g.table); }

function serveOne(sim, g) {
  sim.pass = sim.pass.filter((p) => p.guest !== g.id);
  if (g.readySince !== null && sim.t - g.readySince > 9) { g.mood -= 6; log(sim, `${g.name}: "Lauwarm..."`, 'upset', g); }
  if (sim.t - g.since < g.patience.serve * 0.4) g.mood += 5;
  g.mood = Math.max(0, Math.min(100, g.mood));
  g.state = 'eating';
  g.since = sim.t;
  g.left = g.rounds > 1 ? g.stay * 0.5 : g.stay;
}

function pay(sim, g, T, grudging = false) {
  const total = g.billed.reduce((s, i) => s + MENU[i].price, 0);
  const pct = grudging ? 0 : g.mood >= 80 ? 0.12 : g.mood >= 60 ? 0.08 : g.mood >= 40 ? 0.04 : 0;
  const tip = round2(total * pct * T.tip);
  sim.stats.revenue = round2(sim.stats.revenue + total);
  sim.stats.tips = round2(sim.stats.tips + tip);
  sim.stats.paid++;
  sim.stats.cakesSold += g.billed.filter((i) => MENU[i].kind === 'cake').length;
  g.paid = total + tip;
  if (tip >= 1) log(sim, `${g.name} pays ${total.toFixed(2)} and leaves ${tip.toFixed(2)} Trinkgeld.`, 'money', g);
  leave(sim, g);
}

function leave(sim, g, angry = false) {
  const table = sim.tables.find((t) => t.id === g.table);
  if (table) table.guest = null;
  for (const t of sim.tables) if (t.reserved === g.id) t.reserved = null;
  sim.queue = sim.queue.filter((t) => t.guest !== g.id);
  for (const w of [sim.waiter, sim.piccolo]) if (w?.task?.guest === g.id) abortTask(sim, w);
  sim.brew.waiting = sim.brew.waiting.filter((b) => b.guest !== g.id);
  sim.pass = sim.pass.filter((p) => p.guest !== g.id);
  g.state = 'leaving';
  g.since = sim.t;
  if (angry) g.mood = 0;
  sim.stats.moods.push(g.mood);
  if (g.regular) Object.assign(sim.stats.regulars[g.regular.key], { mood: g.mood, angry });
  walkTo(g, LAYOUT.outside);
}

// ------------------------------------------------------------------ tick
function moveTowards(o, p, speed, dt) {
  const dx = p.x - o.x, dz = p.z - o.z;
  const d = Math.hypot(dx, dz);
  const step = speed * dt;
  if (d <= step) { o.x = p.x; o.z = p.z; return true; }
  o.x += (dx / d) * step; o.z += (dz / d) * step;
  return false;
}

function runWorker(sim, w, dt, isPiccolo) {
  if (!w.task) {
    if (isPiccolo) {
      const next = sim.queue.findIndex((t) => t.kind === 'serve');
      const g = sim.guests.find((o) => o.state === 'ordered' && o.ready === o.order.length && o.order.length && !sim.queue.some((t) => t.guest === o.id) && sim.waiter.task?.guest !== o.id);
      if (next >= 0) startTask(sim, w, sim.queue.splice(next, 1)[0]);
      else if (g) startTask(sim, w, { kind: 'serve', guest: g.id, at: sim.t });
      else moveTowards(w, LAYOUT.piccoloHome, WAITER_SPEED, dt);
    } else {
      while (sim.queue.length && !w.task) {
        const task = sim.queue.shift();
        if (valid(sim, task)) startTask(sim, w, task);
        else if (task.kind === 'seat') { const t = sim.tables.find((x) => x.id === task.table); if (t?.reserved === task.guest) t.reserved = null; }
      }
      if (!w.task && sim.policy) {
        const task = sim.policy(sim);
        if (task && enqueue(sim, task)) { const q = sim.queue.pop(); startTask(sim, w, q); }
      }
    }
    if (!w.task) return;
  }
  const traying = w.task.kind === 'serve' && w.task.batch;
  if (traying ? !w.task.batch.some((id) => { const g = sim.guests.find((o) => o.id === id); return g && readyForTray(sim, g); }) : !valid(sim, w.task) || !w.legs) { abortTask(sim, w); return; }
  const leg = w.legs[w.leg];
  if (leg.to || leg.toGuest) {
    let target = leg.to;
    if (leg.toGuest) {
      // the guest may still be walking in; aim at their queue spot, not where they were
      const g = sim.guests.find((o) => o.id === w.task.guest);
      const spot = g.tx !== null ? { x: g.tx, z: g.tz } : g;
      target = { x: spot.x, z: Math.min(spot.z - 0.6, 4.8) };
    }
    let arrived = moveTowards(w, target, WAITER_SPEED, dt);
    if (arrived && leg.toGuest) {
      const g = sim.guests.find((o) => o.id === w.task.guest);
      arrived = g.tx === null || Math.hypot(g.x - g.tx, g.z - g.tz) < 0.15;
    }
    if (leg.lead) {
      const g = sim.guests.find((o) => o.id === w.task.guest);
      if (g) { g.x = w.x - 0.5; g.z = w.z + 0.5; g.tx = g.x; g.tz = g.z; }
    }
    if (arrived) { w.leg++; w.legT = 0; }
  } else {
    w.legT += dt;
    if (w.legT >= leg.wait) {
      if (leg.deliver) { const g = sim.guests.find((o) => o.id === leg.deliver); if (g && readyForTray(sim, g)) serveOne(sim, g); }
      w.leg++; w.legT = 0;
    }
  }
  if (w.leg >= w.legs.length) finishTask(sim, w);
}

function tick(sim, dt) {
  if (sim.over) return;
  sim.t += dt;
  while (sim.arrivals.length && sim.arrivals[0].t <= sim.t) spawn(sim, sim.arrivals.shift());
  // brewing
  const b = sim.brew;
  while (b.active.length < b.slots && b.waiting.length) {
    const job = b.waiting.shift();
    b.active.push({ ...job, left: MENU[job.item].brew });
  }
  for (const job of b.active) job.left -= dt;
  for (const job of b.active.filter((j) => j.left <= 0)) {
    sim.pass.push({ item: job.item, guest: job.guest, at: sim.t });
    const g = sim.guests.find((o) => o.id === job.guest);
    if (g) { g.ready++; if (g.ready === g.order.length) g.readySince = sim.t; }
  }
  b.active = b.active.filter((j) => j.left > 0);
  // workers
  runWorker(sim, sim.waiter, dt, false);
  if (sim.piccolo) runWorker(sim, sim.piccolo, dt, true);
  // guests
  for (const g of sim.guests) {
    if (g.state === 'gone') continue;
    if (g.state === 'queue') walkTo(g, queueSpot(sim, g));
    if (g.tx !== null) moveTowards(g, { x: g.tx, z: g.tz }, GUEST_SPEED, dt);
    if (g.state === 'leaving') {
      if (Math.hypot(g.x - LAYOUT.outside.x, g.z - LAYOUT.outside.z) < 0.05) g.state = 'gone';
      continue;
    }
    if (g.state === 'eating') {
      g.left -= dt;
      if (g.left <= 0) {
        // A guest left in peace, and happy, orders another round. Rushing them loses this.
        const p = { regular: 0.75, student: 0.5, tourist: 0.4, business: 0.1 }[g.type];
        if (g.rounds === 1 && !g.hurried && g.mood >= 60 && sim.t < DOORS_CLOSE && sim.rng() < p) {
          const second = g.regular ? [g.regular.usual[0]] : g.type === 'tourist' ? [sim.rng() < 0.5 ? 'melange' : CAKES[Math.floor(sim.rng() * 3)]] : ['verlaengerter'];
          g.wish = g.size === 2 && g.type === 'tourist' ? [...second, ...second] : second;
          g.state = 'seated';
          g.since = sim.t;
          log(sim, `${g.name}: "Noch ${MENU[g.wish[0]].kind === 'cake' ? 'ein Stück' : 'eine'} ${MENU[g.wish[0]].name}, bitte."`, 'order', g);
        } else { g.state = 'wantsBill'; g.since = sim.t; log(sim, `${g.name}: "Zahlen, bitte!"`, 'bill', g); }
      }
      continue;
    }
    const phase = { queue: 'queue', seated: 'order', ordered: 'serve', wantsBill: 'bill' }[g.state];
    if (!phase) continue;
    const limit = g.patience[phase];
    const waited = sim.t - g.since;
    if (waited > limit * 0.5) g.mood = Math.max(0, g.mood - (30 / limit) * dt);
    if (waited > limit) {
      if (phase === 'bill') {
        if (g.regular || g.type === 'business') { log(sim, `${g.name} puts the money on the table and leaves, unimpressed.`, 'upset', g); g.mood = Math.min(g.mood, 15); pay(sim, g, TYPES[g.type], true); }
        else { sim.stats.walkouts++; log(sim, `${g.name} left without paying!`, 'angry', g); leave(sim, g, true); }
      } else {
        sim.stats.angry++;
        log(sim, `${g.name} gives up and leaves.`, 'angry', g);
        leave(sim, g, true);
      }
    }
  }
  if (sim.guests.length > 400) sim.guests = sim.guests.filter((g) => g.state !== 'gone');
  // closing
  const present = sim.guests.some((g) => g.state !== 'gone');
  if (sim.t >= DOORS_CLOSE && !sim.arrivals.length && !present) sim.over = true;
  if (sim.t >= DAY_END) {
    for (const g of sim.guests) {
      if (['eating', 'wantsBill'].includes(g.state)) pay(sim, g, TYPES[g.type], true);
      else if (!['gone', 'leaving'].includes(g.state)) { g.mood = Math.min(g.mood, 30); leave(sim, g); }
    }
    log(sim, 'Sperrstunde. The chairs go up on the tables.', 'info');
    sim.over = true;
  }
}

// ------------------------------------------------------------------ end of day
export function report(sim) {
  const s = sim.stats;
  const avg = s.moods.length ? s.moods.reduce((a, b) => a + b, 0) / s.moods.length : 0;
  const stars = Math.round((avg / 20) * 10) / 10;
  return {
    day: sim.meta.day, revenue: s.revenue, tips: s.tips, rent: RENT, profit: round2(s.revenue + s.tips - RENT),
    guests: s.moods.length, paid: s.paid, angry: s.angry, walkouts: s.walkouts, stars, cakesSold: s.cakesSold,
    regulars: REGULARS.map((r) => s.regulars[r.key] ? { key: r.key, ...s.regulars[r.key] } : { key: r.key, name: r.name, came: false }),
  };
}

export function applyDayEnd(meta, rep) {
  const m = structuredClone(meta);
  m.cash = round2(m.cash + rep.profit);
  m.reputation = Math.round((m.reputation * 0.6 + rep.stars * 0.4) * 100) / 100;
  for (const r of rep.regulars) {
    if (!r.came) continue;
    let d = 0;
    if (r.angry) d = -2;
    else if (r.hurried) d = -1;
    else if (r.mood >= 70) d = 1;
    else if (r.mood < 40) d = -1;
    m.loyalty[r.key] = Math.max(0, Math.min(5, (m.loyalty[r.key] ?? 3) + d));
    r.loyaltyDelta = d;
  }
  m.history.push({ day: rep.day, profit: rep.profit, stars: rep.stars });
  m.day++;
  return m;
}

// ------------------------------------------------------------------ bots
// Used by the verifier to prove that decisions matter: greedy > random > idle.
export function bestTableFor(sim, g) {
  const free = sim.freeTables();
  if (!free.length) return null;
  if (g.regular) return free.find((t) => t.id === g.regular.table) ?? free[0];
  // cheapest for the waiter: close to the counter, where every tray starts
  const trips = (t) => dist(t, LAYOUT.pickup) * 3 + dist(t, LAYOUT.door);
  const byTrips = [...free].sort((a, b) => trips(a) - trips(b));
  if (g.type === 'tourist') return free.find((t) => t.window) ?? byTrips[0];
  if (g.type === 'business') return byTrips[0];
  // students stay for hours: give them the worst table, keep the good ones turning
  return byTrips[byTrips.length - 1];
}

// Earliest deadline first: do whatever lets the next guest down soonest, counting
// the walk. Papers only when nobody is close to their limit.
export function greedyPolicy(sim) {
  const w = sim.waiter;
  const waitingAtDoor = sim.guests.filter((g) => g.state === 'queue').length;
  const freeCount = sim.freeTables().length;
  let best = null, bestSlack = Infinity, paper = null, lost = null;
  for (const g of sim.guests) {
    const table = tableOf(sim, g);
    for (const kind of actionsFor(sim, g)) {
      if (kind === 'paper') { if (!paper || g.regular) paper = { kind, guest: g.id }; continue; }
      if (kind === 'hurry') {
        if (waitingAtDoor > 0 && freeCount === 0 && !g.regular && g.type !== 'student' && g.left < 10) {
          const slack = 4;
          if (slack < bestSlack) { bestSlack = slack; best = { kind, guest: g.id }; }
        }
        continue;
      }
      const phase = { seat: 'queue', order: 'order', serve: 'serve', bill: 'bill' }[kind];
      let walk = kind === 'seat' ? dist(w, g) + 5 : dist(w, servePos(table));
      if (kind === 'serve') walk = dist(w, LAYOUT.pickup) + dist(LAYOUT.pickup, servePos(table));
      const eta = walk / WAITER_SPEED;
      let slack = g.patience[phase] - (sim.t - g.since) - eta;
      if (kind === 'serve' && g.readySince !== null) slack = Math.min(slack, 9 - (sim.t - g.readySince) - eta);
      if (kind === 'seat' && g.regular) slack -= 2; // the Hofrat should not wait at the door
      if (kind === 'bill') slack -= 2 + g.billed.reduce((s2, i) => s2 + MENU[i].price, 0) / 5; // a walkout loses the whole bill
      if (slack < -1 && kind !== 'bill') { if (!lost) lost = { kind, guest: g.id }; continue; } // triage: cannot make it
      if (slack < bestSlack) { bestSlack = slack; best = { kind, guest: g.id }; }
    }
  }
  if ((!best || bestSlack > 8) && paper) best = paper;
  best ??= lost;
  if (!best) return null;
  if (best.kind === 'seat') {
    const g = sim.guests.find((o) => o.id === best.guest);
    best.table = bestTableFor(sim, g)?.id;
    if (!best.table) return null;
  }
  return best;
}

export function randomPolicy(sim) {
  const opts = [];
  for (const g of sim.guests) for (const kind of actionsFor(sim, g)) opts.push({ kind, guest: g.id });
  if (!opts.length) return null;
  const pick = opts[Math.floor(sim.rng() * opts.length)];
  if (pick.kind === 'seat') {
    const free = sim.freeTables();
    pick.table = free[Math.floor(sim.rng() * free.length)].id;
  }
  return pick;
}

export function runDay({ seed, meta = defaultMeta(), policy }) {
  const sim = createSim({ seed, meta, policy });
  let guard = 0;
  while (!sim.over && guard++ < 200000) sim.tick(TICK * 6);
  return sim.report();
}

// Does skill matter? Day 1 is demand-bound (nearly everyone gets served by any
// bot that acts), so there skill shows in stars and tips. On a busy day the
// waiter is the bottleneck, so there it must also show in takings.
export function selfTest() {
  const seeds = [1, 2, 3, 4, 5];
  const busyMeta = { ...defaultMeta(), day: 4 };
  const run = (policy, meta) => seeds.map((seed) => runDay({ seed, policy, meta }));
  const mean = (rs, k) => rs.reduce((s, r) => s + r[k], 0) / rs.length;
  const summarise = (res) => Object.fromEntries(Object.entries(res).map(([k, rs]) => [k, {
    takings: round2(mean(rs, 'revenue') + mean(rs, 'tips')), stars: round2(mean(rs, 'stars')),
    angry: round2(mean(rs, 'angry')), walkouts: round2(mean(rs, 'walkouts')), guests: round2(mean(rs, 'guests')),
  }]));
  const d1raw = { greedy: run(greedyPolicy), random: run(randomPolicy), idle: run(null) };
  const d4raw = { greedy: run(greedyPolicy, busyMeta), random: run(randomPolicy, busyMeta) };
  const d1 = summarise(d1raw), d4 = summarise(d4raw);
  const campaign = (policy) => {
    const runs = seeds.map((seed) => {
      let meta = defaultMeta();
      for (let d = 0; d < 5; d++) meta = applyDayEnd(meta, runDay({ seed: seed * 31 + d, policy, meta }));
      return { cash: meta.cash, reputation: meta.reputation, loyalty: Object.values(meta.loyalty).reduce((a, b) => a + b, 0) };
    });
    return { cash: round2(mean(runs, 'cash')), reputation: round2(mean(runs, 'reputation')), loyalty: round2(mean(runs, 'loyalty')) };
  };
  const camp = { greedy: campaign(greedyPolicy), random: campaign(randomPolicy) };
  const checks = {
    day1_greedy_takings_beat_random: d1.greedy.takings > d1.random.takings * 1.05,
    day1_random_beats_idle: d1.random.takings > d1.idle.takings,
    day1_idle_earns_nothing: d1.idle.takings === 0,
    day1_greedy_stars_at_least_3: d1.greedy.stars >= 3,
    day1_greedy_stars_beat_random: d1.greedy.stars > d1.random.stars + 0.3,
    // Overloaded day without upgrades: rushing buys raw throughput, so the takings
    // gap is modest by design; the stars gap and the campaign below carry the claim.
    busy_day_greedy_takings_beat_random: d4.greedy.takings > d4.random.takings * 1.05,
    busy_day_greedy_stars_beat_random: d4.greedy.stars > d4.random.stars + 0.3,
    campaign_greedy_cash_beats_random: camp.greedy.cash > camp.random.cash * 1.25,
    campaign_random_loses_regulars: camp.random.loyalty < camp.greedy.loyalty,
    every_day_ends: [...Object.values(d1raw), ...Object.values(d4raw)].flat().every((r) => r.guests > 0),
  };
  return { pass: Object.values(checks).every(Boolean), checks, summary: { day1: d1, day4: d4, campaign5: camp } };
}

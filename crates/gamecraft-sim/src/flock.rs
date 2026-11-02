//! Murmuration: a seeded, deterministic flocking sim.
//!
//! A flock of starlings (boids) is steered across a dusk arena by a lure the player
//! moves. Pillars block the way and hawks hunt stragglers. The goal is to get at least
//! 85 percent of the starting flock into the roost before time runs out.
//!
//! Boids are stored as parallel arrays (cheap to scan, cheap to hand to a renderer).
//! Steering reads the previous state and writes a new one, and neighbours come from a
//! counting-sort grid, so the result never depends on iteration order and replays
//! byte-identically from a seed. No dependencies, no unsafe.
#![allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use crate::DT;
use std::f32::consts::TAU;

/// Side of the square arena in world units (metres).
pub const ARENA: f32 = 400.0;
/// Where the flock starts.
pub const START: (f32, f32) = (50.0, 50.0);
/// The roost: be in here when the clock stops.
pub const ROOST: Disc = Disc {
    x: 320.0,
    y: 320.0,
    r: 70.0,
};
/// Fraction of the starting flock that must be alive and in the roost to win.
pub const WIN_FRACTION: f32 = 0.85;

const PERCEPTION: f32 = 10.0;
const GRID: usize = 40;
const SEPARATION: f32 = 2.0;
const NEIGHBOUR_CAP: f32 = 24.0;
const MIN_SPEED: f32 = 5.0;
const CRUISE: f32 = 14.0;
const PANIC: f32 = 19.0;
const MAX_ACCEL: f32 = 45.0;
const LURE_SPEED: f32 = 12.0;
/// Radius around the lure inside which boids are drawn to it.
pub const LURE_RANGE: f32 = 90.0;
const HAWK_SPEED: f32 = 22.0;
const HAWK_TURN: f32 = 60.0;
const HAWK_SIGHT: f32 = 140.0;
const FLEE_RANGE: f32 = 45.0;
const CATCH: f32 = 6.0;
const CATCH_COOLDOWN: u32 = 6;
/// Steps between flares (6 seconds).
pub const FLARE_COOLDOWN: u32 = 360;
/// Reach of a flare around the lure.
pub const FLARE_RADIUS: f32 = 90.0;
const FLARE_STUN: u32 = 180;
const PILLAR_MARGIN: f32 = 14.0;
const WALL_MARGIN: f32 = 20.0;
const SETTLE_SPEED: f32 = 7.0;
const SETTLE_ZONE: f32 = 1.3;

/// A circle on the ground: a pillar or the roost.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Disc {
    /// Centre x.
    pub x: f32,
    /// Centre y.
    pub y: f32,
    /// Radius.
    pub r: f32,
}

/// A predator. It chases the nearest boid it can see and patrols otherwise.
#[derive(Debug, Clone, PartialEq)]
pub struct Hawk {
    /// Position x.
    pub x: f32,
    /// Position y.
    pub y: f32,
    /// Velocity x.
    pub vx: f32,
    /// Velocity y.
    pub vy: f32,
    tx: f32,
    ty: f32,
    cooldown: u32,
    stun: u32,
}

/// Everything that defines a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlockConfig {
    /// Starting flock size.
    pub count: usize,
    /// Level seed (pillars, spawn, hawk patrols).
    pub seed: u64,
    /// Number of hawks.
    pub hawks: usize,
    /// Number of pillars.
    pub pillars: usize,
    /// Steps before the evening ends (60 per second).
    pub time_limit: u32,
}

impl FlockConfig {
    /// The shipped level: 20 000 starlings, three hawks, nine pillars, 150 seconds.
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            count: 20_000,
            seed: 7,
            hawks: 3,
            pillars: 9,
            time_limit: 60 * 150,
        }
    }

    /// The same level with a smaller flock, for fast tests.
    #[must_use]
    pub const fn small(count: usize) -> Self {
        Self {
            count,
            ..Self::standard()
        }
    }
}

/// Counters for the HUD, the gate and the tests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlockStats {
    /// Steps taken.
    pub tick: u32,
    /// Boids still flying.
    pub alive: usize,
    /// Boids caught by hawks.
    pub eaten: usize,
    /// Living boids inside the roost.
    pub in_roost: usize,
    /// Boids needed in the roost to win.
    pub needed: usize,
    /// Won.
    pub won: bool,
    /// Lost (time up, or too many eaten to ever win).
    pub lost: bool,
}

/// What the player asked for this step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Orders {
    /// Where the lure should head.
    pub target: (f32, f32),
    /// Fire a flare if one is ready.
    pub flare: bool,
}
/// The whole world.
#[derive(Debug, Clone)]
pub struct Flock {
    /// Level definition.
    pub cfg: FlockConfig,
    /// Boid x positions.
    pub px: Vec<f32>,
    /// Boid y positions.
    pub py: Vec<f32>,
    /// Boid x velocities.
    pub vx: Vec<f32>,
    /// Boid y velocities.
    pub vy: Vec<f32>,
    /// False once a hawk has caught the boid.
    pub alive: Vec<bool>,
    /// The lure the player steers.
    pub lure: (f32, f32),
    /// Hawks.
    pub hawks: Vec<Hawk>,
    /// Pillars.
    pub pillars: Vec<Disc>,
    /// Steps until the next flare is ready.
    pub flare_cooldown: u32,
    /// Steps since the last flare went off (large when none recent).
    pub flare_age: u32,
    tick: u32,
    eaten: usize,
    won: bool,
    lost: bool,
    rng: u64,
    live: usize,
    grid_start: Vec<u32>,
    grid_items: Vec<u32>,
    nvx: Vec<f32>,
    nvy: Vec<f32>,
}

fn next_rand(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 40) as f32) / 16_777_216.0
}

fn wobble(i: usize, tick: u32) -> (f32, f32) {
    let mut s = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ u64::from(tick).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ 0x94D0_49BB_1331_11EB;
    let a = next_rand(&mut s);
    let b = next_rand(&mut s);
    (a * 2.0 - 1.0, b * 2.0 - 1.0)
}

fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    (ax - bx).hypot(ay - by)
}

fn cell_of(x: f32, y: f32) -> usize {
    let cx = ((x / PERCEPTION) as usize).min(GRID - 1);
    let cy = ((y / PERCEPTION) as usize).min(GRID - 1);
    cy * GRID + cx
}

fn place_pillars(count: usize, rng: &mut u64) -> Vec<Disc> {
    let mut out: Vec<Disc> = Vec::with_capacity(count);
    let mut tries = 0;
    while out.len() < count && tries < 5000 {
        tries += 1;
        let r = 8.0 + next_rand(rng) * 10.0;
        let x = 40.0 + next_rand(rng) * 320.0;
        let y = 40.0 + next_rand(rng) * 320.0;
        if dist(x, y, START.0, START.1) < r + 60.0
            || dist(x, y, ROOST.x, ROOST.y) < r + ROOST.r + 40.0
            || out.iter().any(|o| dist(x, y, o.x, o.y) < r + o.r + 18.0)
        {
            continue;
        }
        out.push(Disc { x, y, r });
    }
    out
}

/// Push a point out of every pillar and back inside the arena.
fn resolve(x: &mut f32, y: &mut f32, pillars: &[Disc], pad: f32) {
    for p in pillars {
        let d = dist(*x, *y, p.x, p.y);
        let min = p.r + pad;
        if d < min {
            let (nx, ny) = if d > 1e-4 {
                ((*x - p.x) / d, (*y - p.y) / d)
            } else {
                (1.0, 0.0)
            };
            *x = p.x + nx * min;
            *y = p.y + ny * min;
        }
    }
    *x = x.clamp(1.0, ARENA - 1.0);
    *y = y.clamp(1.0, ARENA - 1.0);
}

#[derive(Default)]
struct Around {
    n: f32,
    vx: f32,
    vy: f32,
    ox: f32,
    oy: f32,
    sx: f32,
    sy: f32,
}

impl Flock {
    /// Build the level from its config. Same config, same world, every time.
    #[must_use]
    pub fn new(cfg: FlockConfig) -> Self {
        let mut rng = cfg.seed | 1;
        let pillars = place_pillars(cfg.pillars, &mut rng);
        let n = cfg.count;
        let (mut px, mut py) = (Vec::with_capacity(n), Vec::with_capacity(n));
        let (mut vx, mut vy) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for _ in 0..n {
            let a = next_rand(&mut rng) * TAU;
            let r = next_rand(&mut rng).sqrt() * 28.0;
            let h = next_rand(&mut rng) * TAU;
            px.push(START.0 + a.cos() * r);
            py.push(START.1 + a.sin() * r);
            vx.push(h.cos() * 8.0);
            vy.push(h.sin() * 8.0);
        }
        let posts = [(340.0, 80.0), (80.0, 340.0), (200.0, 200.0), (330.0, 200.0)];
        let hawks = (0..cfg.hawks)
            .map(|k| {
                let (x, y) = posts[k % posts.len()];
                Hawk {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                    tx: x,
                    ty: y,
                    cooldown: 0,
                    stun: 0,
                }
            })
            .collect();
        Self {
            cfg,
            px,
            py,
            vx,
            vy,
            alive: vec![true; n],
            lure: START,
            hawks,
            pillars,
            flare_cooldown: 0,
            flare_age: u32::MAX,
            tick: 0,
            eaten: 0,
            won: false,
            lost: false,
            rng,
            live: n,
            grid_start: Vec::new(),
            grid_items: Vec::new(),
            nvx: vec![0.0; n],
            nvy: vec![0.0; n],
        }
    }

    /// Current counters.
    #[must_use]
    pub fn stats(&self) -> FlockStats {
        FlockStats {
            tick: self.tick,
            alive: self.live,
            eaten: self.eaten,
            in_roost: self.in_roost(),
            needed: self.needed(),
            won: self.won,
            lost: self.lost,
        }
    }

    fn needed(&self) -> usize {
        (self.cfg.count as f32 * WIN_FRACTION).ceil() as usize
    }

    fn in_roost(&self) -> usize {
        (0..self.px.len())
            .filter(|&i| self.alive[i] && dist(self.px[i], self.py[i], ROOST.x, ROOST.y) < ROOST.r)
            .count()
    }

    /// Advance one fixed step.
    pub fn step(&mut self, orders: Orders) {
        if self.won || self.lost {
            return;
        }
        self.flare_cooldown = self.flare_cooldown.saturating_sub(1);
        self.flare_age = self.flare_age.saturating_add(1);
        if orders.flare {
            self.fire_flare();
        }
        self.move_lure(orders.target);
        self.rebuild_grid();
        self.move_boids();
        self.move_hawks();
        self.tick += 1;
        self.judge();
    }

    fn move_lure(&mut self, target: (f32, f32)) {
        let (dx, dy) = (target.0 - self.lure.0, target.1 - self.lure.1);
        let d = dx.hypot(dy);
        if d > 0.01 {
            let s = (LURE_SPEED * DT).min(d);
            self.lure.0 += dx / d * s;
            self.lure.1 += dy / d * s;
        }
        resolve(&mut self.lure.0, &mut self.lure.1, &self.pillars, 2.0);
    }

    fn rebuild_grid(&mut self) {
        let cells = GRID * GRID;
        self.grid_start.clear();
        self.grid_start.resize(cells + 1, 0);
        for i in 0..self.px.len() {
            if self.alive[i] {
                self.grid_start[cell_of(self.px[i], self.py[i]) + 1] += 1;
            }
        }
        for c in 0..cells {
            self.grid_start[c + 1] += self.grid_start[c];
        }
        let mut cursor: Vec<u32> = self.grid_start[..cells].to_vec();
        self.grid_items.clear();
        self.grid_items.resize(self.live, 0);
        for i in 0..self.px.len() {
            if self.alive[i] {
                let c = cell_of(self.px[i], self.py[i]);
                self.grid_items[cursor[c] as usize] = i as u32;
                cursor[c] += 1;
            }
        }
    }

    fn around(&self, i: usize) -> Around {
        let (x, y) = (self.px[i], self.py[i]);
        let c = cell_of(x, y);
        let (cx, cy) = (c % GRID, c / GRID);
        let mut a = Around::default();
        let mut cells = [0usize; 9];
        let mut m = 0;
        for gy in cy.saturating_sub(1)..=(cy + 1).min(GRID - 1) {
            for gx in cx.saturating_sub(1)..=(cx + 1).min(GRID - 1) {
                cells[m] = gy * GRID + gx;
                m += 1;
            }
        }
        let rot = (i + self.tick as usize) % m.max(1);
        'scan: for q in 0..m {
            let cell = cells[(q + rot) % m];
            let (lo, hi) = (self.grid_start[cell], self.grid_start[cell + 1]);
            for k in lo..hi {
                let j = self.grid_items[k as usize] as usize;
                if j == i {
                    continue;
                }
                let (dx, dy) = (self.px[j] - x, self.py[j] - y);
                let d2 = dx * dx + dy * dy;
                if d2 > PERCEPTION * PERCEPTION {
                    continue;
                }
                a.n += 1.0;
                a.vx += self.vx[j];
                a.vy += self.vy[j];
                a.ox += dx;
                a.oy += dy;
                if d2 < SEPARATION * SEPARATION && d2 > 1e-6 {
                    a.sx -= dx / d2;
                    a.sy -= dy / d2;
                }
                if a.n >= NEIGHBOUR_CAP {
                    break 'scan;
                }
            }
        }
        a
    }

    /// Flocking rules: stay apart, match heading, drift towards the group.
    fn flock_force(&self, i: usize) -> (f32, f32) {
        let a = self.around(i);
        if a.n < 0.5 {
            return (0.0, 0.0);
        }
        let (mvx, mvy) = (a.vx / a.n, a.vy / a.n);
        (
            a.sx * 55.0 + (mvx - self.vx[i]) * 2.2 + a.ox / a.n * 0.35,
            a.sy * 55.0 + (mvy - self.vy[i]) * 2.2 + a.oy / a.n * 0.35,
        )
    }

    /// The lure draws boids in and makes them swirl around it.
    fn lure_force(&self, i: usize) -> (f32, f32) {
        let (dx, dy) = (self.lure.0 - self.px[i], self.lure.1 - self.py[i]);
        let d = dx.hypot(dy).max(0.1);
        let (ux, uy) = (dx / d, dy / d);
        let parked = dist(self.lure.0, self.lure.1, ROOST.x, ROOST.y) < 20.0;
        let range = if parked { ROOST.r } else { LURE_RANGE };
        if d >= range {
            return (ux * 2.0, uy * 2.0);
        }
        let pull = 1.5 + 5.0 * d / range;
        let swirl = if parked { 0.0 } else { 7.0 * (1.0 - d / range) };
        (ux * pull - uy * swirl, uy * pull + ux * swirl)
    }

    /// Pillars, hawks and walls push back. Also reports whether a hawk is close.
    fn hazard_force(&self, i: usize) -> ((f32, f32), bool) {
        let (x, y) = (self.px[i], self.py[i]);
        let (mut ax, mut ay) = (0.0, 0.0);
        for p in &self.pillars {
            let d = dist(x, y, p.x, p.y);
            let gap = d - p.r;
            if gap < PILLAR_MARGIN && d > 1e-4 {
                let k = (PILLAR_MARGIN - gap.max(0.0)) / PILLAR_MARGIN * 60.0;
                ax += (x - p.x) / d * k;
                ay += (y - p.y) / d * k;
            }
        }
        let mut panic = false;
        for h in &self.hawks {
            let d = dist(x, y, h.x, h.y);
            if d < FLEE_RANGE && d > 1e-4 {
                panic = true;
                let k = (1.0 - d / FLEE_RANGE) * 90.0;
                ax += (x - h.x) / d * k;
                ay += (y - h.y) / d * k;
            }
        }
        ax += (WALL_MARGIN - x).max(0.0) * 3.0 - (x - (ARENA - WALL_MARGIN)).max(0.0) * 3.0;
        ay += (WALL_MARGIN - y).max(0.0) * 3.0 - (y - (ARENA - WALL_MARGIN)).max(0.0) * 3.0;
        ((ax, ay), panic)
    }

    /// Near the roost the flock settles: drawn to the centre and slowed down.
    fn roost_force(&self, i: usize) -> ((f32, f32), bool) {
        let (dx, dy) = (ROOST.x - self.px[i], ROOST.y - self.py[i]);
        let d = dx.hypot(dy).max(0.1);
        let zone = ROOST.r * SETTLE_ZONE;
        if d >= zone {
            return ((0.0, 0.0), false);
        }
        let q = d / zone;
        let k = 10.0 * q + 40.0 * q * q * q;
        ((dx / d * k, dy / d * k), true)
    }
    /// The velocity a boid wants next step, from the previous state only. Pure, so it
    /// can run on any thread in any order.
    fn steer_boid(&self, i: usize) -> (f32, f32) {
        let (fx, fy) = self.flock_force(i);
        let (lx, ly) = self.lure_force(i);
        let ((hx, hy), panic) = self.hazard_force(i);
        let ((rx, ry), settle) = self.roost_force(i);
        let (wx, wy) = wobble(i, self.tick);
        let (mut ax, mut ay) = (fx + lx + hx + rx + wx * 2.0, fy + ly + hy + ry + wy * 2.0);
        let a = ax.hypot(ay);
        if a > MAX_ACCEL {
            ax *= MAX_ACCEL / a;
            ay *= MAX_ACCEL / a;
        }
        let (mut vx, mut vy) = (self.vx[i] + ax * DT, self.vy[i] + ay * DT);
        let s = vx.hypot(vy).max(1e-4);
        let top = if panic {
            PANIC
        } else if settle {
            SETTLE_SPEED
        } else {
            CRUISE
        };
        let clamped = s.clamp(MIN_SPEED, top);
        vx *= clamped / s;
        vy *= clamped / s;
        (vx, vy)
    }

    /// Steering is spread over threads in fixed chunks. Every boid reads the old state
    /// and writes only its own slot, so the result is bit-identical for any thread count.
    fn move_boids(&mut self) {
        let n = self.px.len();
        let mut nvx = std::mem::take(&mut self.nvx);
        let mut nvy = std::mem::take(&mut self.nvy);
        let threads = if n < 4000 {
            1
        } else {
            std::thread::available_parallelism()
                .map_or(1, usize::from)
                .min(8)
        };
        let chunk = n.div_ceil(threads).max(1);
        let this: &Self = self;
        std::thread::scope(|scope| {
            for (c, (cx, cy)) in nvx.chunks_mut(chunk).zip(nvy.chunks_mut(chunk)).enumerate() {
                scope.spawn(move || {
                    for k in 0..cx.len() {
                        let i = c * chunk + k;
                        if this.alive[i] {
                            (cx[k], cy[k]) = this.steer_boid(i);
                        }
                    }
                });
            }
        });
        for i in 0..n {
            if !self.alive[i] {
                continue;
            }
            self.vx[i] = nvx[i];
            self.vy[i] = nvy[i];
            let (mut x, mut y) = (self.px[i] + nvx[i] * DT, self.py[i] + nvy[i] * DT);
            resolve(&mut x, &mut y, &self.pillars, 0.2);
            self.px[i] = x;
            self.py[i] = y;
        }
        self.nvx = nvx;
        self.nvy = nvy;
    }

    fn nearest_boid(&self, x: f32, y: f32) -> Option<(usize, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for i in 0..self.px.len() {
            if !self.alive[i] {
                continue;
            }
            let d = dist(x, y, self.px[i], self.py[i]);
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best
    }

    /// Scatter every hawk near the lure: they flee for a few seconds and cannot strike.
    fn fire_flare(&mut self) {
        if self.flare_cooldown > 0 {
            return;
        }
        self.flare_cooldown = FLARE_COOLDOWN;
        self.flare_age = 0;
        let (lx, ly) = self.lure;
        for h in &mut self.hawks {
            let d = dist(h.x, h.y, lx, ly);
            if d < FLARE_RADIUS {
                let (ux, uy) = if d > 1e-3 {
                    ((h.x - lx) / d, (h.y - ly) / d)
                } else {
                    (1.0, 0.0)
                };
                h.stun = FLARE_STUN;
                h.tx = (h.x + ux * 220.0).clamp(20.0, ARENA - 20.0);
                h.ty = (h.y + uy * 220.0).clamp(20.0, ARENA - 20.0);
            }
        }
    }

    /// Boids killed by one hawk strike: proportional to flock size so small test
    /// flocks behave like the full one.
    fn strike_size(&self) -> usize {
        self.cfg.count / 1500 + 1
    }

    fn strike(&mut self, hx: f32, hy: f32) {
        let mut left = self.strike_size();
        for i in 0..self.px.len() {
            if left == 0 {
                break;
            }
            if self.alive[i] && dist(hx, hy, self.px[i], self.py[i]) < CATCH {
                self.alive[i] = false;
                self.live -= 1;
                self.eaten += 1;
                left -= 1;
            }
        }
    }

    fn move_hawks(&mut self) {
        for k in 0..self.hawks.len() {
            let (hx, hy) = (self.hawks[k].x, self.hawks[k].y);
            let stunned = self.hawks[k].stun > 0;
            let prey = if stunned {
                None
            } else {
                self.nearest_boid(hx, hy).filter(|&(_, d)| d < HAWK_SIGHT)
            };
            if let Some((i, _)) = prey {
                self.hawks[k].tx = self.px[i] + self.vx[i] * 0.3;
                self.hawks[k].ty = self.py[i] + self.vy[i] * 0.3;
            } else if !stunned && dist(hx, hy, self.hawks[k].tx, self.hawks[k].ty) < 12.0 {
                self.hawks[k].tx = 30.0 + next_rand(&mut self.rng) * (ARENA - 60.0);
                self.hawks[k].ty = 30.0 + next_rand(&mut self.rng) * (ARENA - 60.0);
            }
            let h = &mut self.hawks[k];
            let (dx, dy) = (h.tx - h.x, h.ty - h.y);
            let d = dx.hypot(dy).max(1e-4);
            let (wx, wy) = (dx / d * HAWK_SPEED, dy / d * HAWK_SPEED);
            let (sx, sy) = (wx - h.vx, wy - h.vy);
            let s = sx.hypot(sy).max(1e-4);
            let turn = (HAWK_TURN * DT).min(s);
            h.vx += sx / s * turn;
            h.vy += sy / s * turn;
            h.x += h.vx * DT;
            h.y += h.vy * DT;
            resolve(&mut h.x, &mut h.y, &self.pillars, 1.0);
            h.stun = h.stun.saturating_sub(1);
            if h.cooldown > 0 {
                h.cooldown -= 1;
            } else if prey.is_some_and(|(_, pd)| pd < CATCH) {
                h.cooldown = CATCH_COOLDOWN;
                let (sx, sy) = (h.x, h.y);
                self.strike(sx, sy);
            }
        }
    }
    fn judge(&mut self) {
        let needed = self.needed();
        if self.in_roost() >= needed {
            self.won = true;
        } else if self.tick >= self.cfg.time_limit || self.live < needed {
            self.lost = true;
        }
    }

    /// Mean position of the living flock.
    #[must_use]
    pub fn centroid(&self) -> (f32, f32) {
        let (mut sx, mut sy, mut n) = (0.0, 0.0, 0.0);
        for i in 0..self.px.len() {
            if self.alive[i] {
                sx += self.px[i];
                sy += self.py[i];
                n += 1.0;
            }
        }
        if n < 0.5 {
            (START.0, START.1)
        } else {
            (sx / n, sy / n)
        }
    }

    /// Root-mean-square distance of the living boids from their centroid: how wide
    /// the cloud is.
    #[must_use]
    pub fn spread(&self) -> f32 {
        let (cx, cy) = self.centroid();
        let mut sum = 0.0;
        for i in 0..self.px.len() {
            if self.alive[i] {
                let (dx, dy) = (self.px[i] - cx, self.py[i] - cy);
                sum += dx * dx + dy * dy;
            }
        }
        (sum / self.live.max(1) as f32).sqrt()
    }
    /// Share of living boids within lure range of the lure.
    #[must_use]
    pub fn gathered(&self) -> f32 {
        let near = (0..self.px.len())
            .filter(|&i| {
                self.alive[i]
                    && dist(self.px[i], self.py[i], self.lure.0, self.lure.1) < LURE_RANGE * 0.7
            })
            .count();
        near as f32 / self.live.max(1) as f32
    }
}

/// A scripted player: head for the roost, wait for stragglers, shy away from hawks and
/// slide round pillars. Returns the lure target for this step.
#[must_use]
pub fn autopilot(f: &Flock) -> Orders {
    let (lx, ly) = f.lure;
    let (cx, cy) = f.centroid();
    let flare = f.flare_cooldown == 0
        && f.hawks
            .iter()
            .any(|h| h.stun == 0 && dist(h.x, h.y, f.lure.0, f.lure.1) < FLARE_RADIUS * 0.9);
    let hold = Orders {
        target: (lx, ly),
        flare,
    };
    if f.gathered() < 0.6 || dist(lx, ly, ROOST.x, ROOST.y) < 6.0 {
        return hold;
    }
    let (mut dx, mut dy) = (ROOST.x - lx, ROOST.y - ly);
    let d = dx.hypot(dy).max(1e-4);
    dx /= d;
    dy /= d;
    for h in &f.hawks {
        let hd = dist(cx, cy, h.x, h.y);
        if hd < 130.0 && hd > 1e-4 {
            let k = (130.0 - hd) / 130.0 * 1.6;
            dx += (cx - h.x) / hd * k;
            dy += (cy - h.y) / hd * k;
        }
    }
    // Dodging hawks must never cancel progress: keep the heading within ~70 degrees
    // of the roost, otherwise the lure ends up pinned in a corner.
    let (rx, ry) = (ROOST.x - lx, ROOST.y - ly);
    let rd = rx.hypot(ry).max(1e-4);
    let (rx, ry) = (rx / rd, ry / rd);
    let along = dx * rx + dy * ry;
    let len = dx.hypot(dy).max(1e-4);
    if along < 0.35 * len {
        let lift = 0.35 * len - along;
        dx += rx * lift;
        dy += ry * lift;
    }
    for p in &f.pillars {
        let pd = dist(lx, ly, p.x, p.y);
        if pd < p.r + 30.0 && pd > 1e-4 {
            let (nx, ny) = ((lx - p.x) / pd, (ly - p.y) / pd);
            let side = if nx * dy - ny * dx >= 0.0 { 1.0 } else { -1.0 };
            let k = (p.r + 30.0 - pd) / 30.0;
            dx += (nx - ny * side) * k;
            dy += (ny + nx * side) * k;
        }
    }
    let m = dx.hypot(dy).max(1e-4);
    Orders {
        target: (lx + dx / m * 60.0, ly + dy / m * 60.0),
        flare,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(cfg: FlockConfig, steps: u32) -> Flock {
        let mut f = Flock::new(cfg);
        for _ in 0..steps {
            let o = autopilot(&f);
            f.step(o);
        }
        f
    }

    #[test]
    fn same_seed_same_flock_and_other_seeds_differ() {
        let a = run(FlockConfig::small(1500), 300);
        let b = run(FlockConfig::small(1500), 300);
        assert!(a
            .px
            .iter()
            .zip(&b.px)
            .all(|(x, y)| x.to_bits() == y.to_bits()));
        assert_eq!(a.stats(), b.stats());
        let c = run(
            FlockConfig {
                seed: 8,
                ..FlockConfig::small(1500)
            },
            300,
        );
        assert_ne!(a.pillars, c.pillars);
    }

    #[test]
    fn threaded_steering_is_deterministic() {
        // 5000 boids takes the multi-threaded path.
        let a = run(FlockConfig::small(5000), 200);
        let b = run(FlockConfig::small(5000), 200);
        let same = |x: &[f32], y: &[f32]| x.iter().zip(y).all(|(p, q)| p.to_bits() == q.to_bits());
        assert!(same(&a.px, &b.px) && same(&a.py, &b.py) && same(&a.vx, &b.vx));
    }
    #[test]
    fn pillars_are_clear_of_start_and_roost() {
        let f = Flock::new(FlockConfig::standard());
        assert_eq!(f.pillars.len(), 9);
        for p in &f.pillars {
            assert!(dist(p.x, p.y, START.0, START.1) > p.r + 50.0);
            assert!(dist(p.x, p.y, ROOST.x, ROOST.y) > p.r + ROOST.r + 30.0);
        }
    }

    #[test]
    fn boids_stay_in_the_arena_and_out_of_pillars() {
        let f = run(FlockConfig::small(1500), 600);
        for i in 0..f.px.len() {
            assert!((0.0..=ARENA).contains(&f.px[i]) && (0.0..=ARENA).contains(&f.py[i]));
            for p in &f.pillars {
                assert!(dist(f.px[i], f.py[i], p.x, p.y) >= p.r - 0.01);
            }
        }
    }

    #[test]
    fn a_motionless_flock_gets_eaten() {
        let mut f = Flock::new(FlockConfig::small(1500));
        f.hawks[0].x = START.0 + 20.0;
        f.hawks[0].y = START.1;
        for _ in 0..600 {
            let target = f.lure;
            f.step(Orders {
                target,
                flare: false,
            });
        }
        assert!(f.stats().eaten > 0, "hawks caught nothing: {:?}", f.stats());
    }

    #[test]
    fn autopilot_wins_small_flocks_across_seeds() {
        for seed in [1_u64, 3, 5, 7] {
            let cfg = FlockConfig {
                seed,
                ..FlockConfig::small(2000)
            };
            let f = run(cfg, cfg.time_limit);
            let s = f.stats();
            println!("seed {seed}: {s:?}");
            assert!(s.won, "seed {seed} not won: {s:?}");
        }
    }

    #[test]
    fn flares_and_routing_beat_a_careless_player() {
        let cfg = FlockConfig {
            seed: 5,
            ..FlockConfig::small(5000)
        };
        let mut careless = Flock::new(cfg);
        for _ in 0..cfg.time_limit {
            careless.step(Orders {
                target: (ROOST.x, ROOST.y),
                flare: false,
            });
        }
        let skilled = run(cfg, cfg.time_limit);
        let (c, s) = (careless.stats(), skilled.stats());
        assert!(!c.won, "hawks should punish a careless player: {c:?}");
        assert!(s.won, "autopilot should win: {s:?}");
        assert!(
            s.eaten * 5 < c.eaten,
            "flares barely helped: {s:?} vs {c:?}"
        );
    }
    #[test]
    #[ignore = "diagnostic trace"]
    fn trace() {
        let mut f = Flock::new(FlockConfig {
            seed: 3,
            ..FlockConfig::small(5000)
        });
        for s in 0..9000 {
            let o = autopilot(&f);
            f.step(o);
            if s % 300 == 0 && s < 5400 {
                let c = f.centroid();
                println!(
                    "{s:5} lure ({:.0},{:.0}) cen ({:.0},{:.0}) spread {:.0} gathered {:.2} roost {} eaten {}",
                    f.lure.0,
                    f.lure.1,
                    c.0,
                    c.1,
                    f.spread(),
                    f.gathered(),
                    f.stats().in_roost,
                    f.stats().eaten
                );
            }
        }
    }

    #[test]
    #[ignore = "diagnostic: how hard do hawks bite a careless player"]
    fn naive_player() {
        for seed in [1_u64, 3, 5] {
            let cfg = FlockConfig {
                seed,
                ..FlockConfig::small(5000)
            };
            let mut f = Flock::new(cfg);
            for _ in 0..cfg.time_limit {
                f.step(Orders {
                    target: (ROOST.x, ROOST.y),
                    flare: false,
                });
            }
            println!("naive seed {seed}: {:?}", f.stats());
            let a = run(cfg, cfg.time_limit);
            println!("autopilot seed {seed}: {:?}", a.stats());
        }
    }
    /// Full size: `cargo test -p gamecraft-sim full_flock -- --ignored --nocapture`.
    #[test]
    #[ignore = "20 000 boids, about a minute"]
    fn full_flock_is_won_by_the_autopilot() {
        let cfg = FlockConfig::standard();
        let f = run(cfg, cfg.time_limit);
        println!("{:?}", f.stats());
        assert!(f.stats().won);
    }
}

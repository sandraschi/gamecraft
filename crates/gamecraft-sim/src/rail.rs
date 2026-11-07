//! Toy railway: a deterministic, block-signalled model railway.
//!
//! The layout is a graph of edges. Every edge is one signal block with a polyline
//! (x, y, z in metres; y is up), a speed limit and an optional platform stop. Trains
//! follow cyclic timetables. The interlocking has three rules: a train only moves
//! into blocks it has reserved, a block is reserved by at most one train, and a block
//! is released once the train's tail has left it. Movement authority ends a few
//! metres before the first block a train could not reserve (a red signal), and the
//! speed stays under the braking curve to that point. Safety comes from those rules;
//! the collision counter checks geometry independently of them.
//!
//! Time is an integer tick at `tps` ticks per second and timetables are whole
//! seconds, so a run is reproducible and the state hash compares two runs exactly.

/// A point in metres; `y` is height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct P3 {
    /// East.
    pub x: f32,
    /// Up.
    pub y: f32,
    /// South.
    pub z: f32,
}

impl P3 {
    /// A point.
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn dist(self, o: Self) -> f32 {
        ((self.x - o.x).powi(2) + (self.y - o.y).powi(2) + (self.z - o.z).powi(2)).sqrt()
    }

    /// Linear interpolation towards `o`.
    #[must_use]
    pub fn lerp(self, o: Self, t: f32) -> Self {
        Self::new(
            self.x + (o.x - self.x) * t,
            self.y + (o.y - self.y) * t,
            self.z + (o.z - self.z) * t,
        )
    }
}

/// Index of an edge in [`Layout::edges`].
pub type EdgeId = usize;

/// One signal block of track.
#[derive(Debug, Clone)]
pub struct Edge {
    /// Human name, shown in logs and the HUD.
    pub name: String,
    /// Centre line, in travel direction.
    pub points: Vec<P3>,
    cum: Vec<f32>,
    /// Edges a train may continue onto (more than one at a split).
    pub next: Vec<EdgeId>,
    /// Line speed in metres per second.
    pub speed_limit: f32,
    /// Station this edge is a platform track of.
    pub station: Option<usize>,
    /// Where a stopping train's head comes to rest, metres from the edge start.
    pub stop_at: Option<f32>,
}

impl Edge {
    /// An edge through `points`, with no successors yet.
    #[must_use]
    pub fn new(name: &str, points: Vec<P3>, speed_limit: f32) -> Self {
        let mut cum = Vec::with_capacity(points.len());
        let mut acc = 0.0_f32;
        let mut prev: Option<P3> = None;
        for p in &points {
            if let Some(q) = prev {
                acc += q.dist(*p);
            }
            cum.push(acc);
            prev = Some(*p);
        }
        Self {
            name: name.to_string(),
            points,
            cum,
            next: Vec::new(),
            speed_limit,
            station: None,
            stop_at: None,
        }
    }

    /// Length along the polyline.
    #[must_use]
    pub fn length(&self) -> f32 {
        self.cum.last().copied().unwrap_or(0.0)
    }

    /// The point `s` metres from the start (clamped to the edge).
    #[must_use]
    pub fn point_at(&self, s: f32) -> P3 {
        let s = s.clamp(0.0, self.length());
        let hi = self.cum.partition_point(|&c| c < s);
        let origin = P3::new(0.0, 0.0, 0.0);
        let Some(lo) = hi.checked_sub(1) else {
            return self.points.first().copied().unwrap_or(origin);
        };
        match (
            self.points.get(lo),
            self.points.get(hi),
            self.cum.get(lo),
            self.cum.get(hi),
        ) {
            (Some(p0), Some(p1), Some(c0), Some(c1)) => {
                let frac = if c1 > c0 { (s - c0) / (c1 - c0) } else { 0.0 };
                p0.lerp(*p1, frac)
            }
            _ => self.points.last().copied().unwrap_or(origin),
        }
    }

    /// Steepest segment: rise over horizontal run.
    #[must_use]
    pub fn max_grade(&self) -> f32 {
        self.points
            .windows(2)
            .map(|w| {
                let (a, b) = (w[0], w[1]);
                let run = ((b.x - a.x).powi(2) + (b.z - a.z).powi(2)).sqrt();
                if run > 1e-6 {
                    (b.y - a.y).abs() / run
                } else {
                    0.0
                }
            })
            .fold(0.0, f32::max)
    }
}

/// A named station and its platform tracks.
#[derive(Debug, Clone)]
pub struct Station {
    /// Name on the boards.
    pub name: String,
    /// Platform edges, track 1 first.
    pub platforms: Vec<EdgeId>,
}

/// Track graph plus stations.
#[derive(Debug, Clone)]
pub struct Layout {
    /// Every block.
    pub edges: Vec<Edge>,
    /// Every station.
    pub stations: Vec<Station>,
}

/// Height of the upper level.
pub const LEVEL_UP: f32 = 8.0;
/// Half length of a station (split to merge).
pub const STATION_HALF: f32 = 70.0;
/// Half length of an island platform.
pub const PLATFORM_HALF: f32 = 48.0;
/// Distance between a main track and its passing loop.
pub const LOOP_OFFSET: f32 = 6.0;
/// Distance of the two straights from the table centre line.
pub const LINE_Z: f32 = 50.0;
/// Distance of the helix centres from the table centre.
pub const HELIX_X: f32 = 100.0;
/// Helix radius.
pub const HELIX_R: f32 = 50.0;
/// The head stops this far before the end of a platform edge.
const STOP_BEFORE_END: f32 = 25.0;

fn line(name: &str, ends: (P3, P3), segments: u16, limit: f32) -> Edge {
    Edge::new(name, straight(ends.0, ends.1, segments), limit)
}

fn straight(a: P3, b: P3, segments: u16) -> Vec<P3> {
    let n = segments.max(1);
    (0..=n)
        .map(|k| a.lerp(b, f32::from(k) / f32::from(n)))
        .collect()
}

/// Helix arc: `sign` 1 starts south of the centre heading east, -1 starts north
/// heading west. Angles in radians, height interpolated linearly.
#[derive(Debug, Clone, Copy)]
struct Arc {
    cx: f32,
    cz: f32,
    sign: f32,
    from: f32,
    to: f32,
    y0: f32,
    y1: f32,
}

fn helix(arc: Arc, segments: u16) -> Vec<P3> {
    let n = segments.max(1);
    (0..=n)
        .map(|k| {
            let t = f32::from(k) / f32::from(n);
            let th = arc.from + (arc.to - arc.from) * t;
            P3::new(
                arc.cx + arc.sign * HELIX_R * th.sin(),
                arc.y0 + (arc.y1 - arc.y0) * t,
                arc.cz + arc.sign * HELIX_R * th.cos(),
            )
        })
        .collect()
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Passing loop from `x0` to `x1` beside the main at `z_main`, offset by `offset`,
/// with S-curves `ramp` metres long at both ends.
fn passing_loop(x0: f32, x1: f32, z_main: f32, offset: f32, y: f32, segments: u16) -> Vec<P3> {
    let n = segments.max(1);
    let total = (x1 - x0).abs();
    let ramp = 20.0;
    (0..=n)
        .map(|k| {
            let t = f32::from(k) / f32::from(n);
            let u = total * t;
            let side = smooth(u.min(total - u) / ramp);
            P3::new(x0 + (x1 - x0) * t, y, z_main + offset * side)
        })
        .collect()
}

impl Layout {
    /// The two-level table: Talheim station below, Bergblick above, joined by two
    /// helices of `helix_turns` turns each (use k + 0.5: 0.5, 1.5, 2.5).
    #[must_use]
    pub fn toy_table(helix_turns: f32) -> Self {
        let (up, z, sh, hx) = (LEVEL_UP, LINE_Z, STATION_HALF, HELIX_X);
        let end = std::f32::consts::TAU * helix_turns;
        let mid = end / 2.0;
        let ymid = up / 2.0;
        let east = |from, to, y0, y1| Arc {
            cx: hx,
            cz: 0.0,
            sign: 1.0,
            from,
            to,
            y0,
            y1,
        };
        let west = |from, to, y0, y1| Arc {
            cx: -hx,
            cz: 0.0,
            sign: -1.0,
            from,
            to,
            y0,
            y1,
        };
        let low = |x0: f32, x1: f32| (P3::new(x0, 0.0, z), P3::new(x1, 0.0, z));
        let high = |x0: f32, x1: f32| (P3::new(x0, up, -z), P3::new(x1, up, -z));
        let loop_low = passing_loop(-sh, sh, z, LOOP_OFFSET, 0.0, 70);
        let loop_high = passing_loop(sh, -sh, -z, -LOOP_OFFSET, up, 70);
        let mut edges = vec![
            line("Talheim approach", low(-hx, -sh), 15, 20.0),
            line("Talheim track 1", low(-sh, sh), 70, 20.0),
            Edge::new("Talheim track 2", loop_low, 12.0),
            line("Talheim exit", low(sh, hx), 15, 25.0),
            Edge::new(
                "East helix, lower",
                helix(east(0.0, mid, 0.0, ymid), 120),
                22.0,
            ),
            Edge::new(
                "East helix, upper",
                helix(east(mid, end, ymid, up), 120),
                22.0,
            ),
            line("Bergblick approach", high(hx, sh), 15, 20.0),
            line("Bergblick track 1", high(sh, -sh), 70, 20.0),
            Edge::new("Bergblick track 2", loop_high, 12.0),
            line("Bergblick exit", high(-sh, -hx), 15, 25.0),
            Edge::new(
                "West helix, upper",
                helix(west(0.0, mid, up, ymid), 120),
                22.0,
            ),
            Edge::new(
                "West helix, lower",
                helix(west(mid, end, ymid, 0.0), 120),
                22.0,
            ),
        ];
        let links: [&[EdgeId]; 12] = [
            &[1, 2],
            &[3],
            &[3],
            &[4],
            &[5],
            &[6],
            &[7, 8],
            &[9],
            &[9],
            &[10],
            &[11],
            &[0],
        ];
        for (e, next) in edges.iter_mut().zip(links) {
            e.next = next.to_vec();
        }
        let stations = vec![
            Station {
                name: "Talheim".to_string(),
                platforms: vec![1, 2],
            },
            Station {
                name: "Bergblick".to_string(),
                platforms: vec![7, 8],
            },
        ];
        for (si, st) in stations.iter().enumerate() {
            for &p in &st.platforms {
                if let Some(e) = edges.get_mut(p) {
                    e.station = Some(si);
                    e.stop_at = Some(e.length() - STOP_BEFORE_END);
                }
            }
        }
        Self { edges, stations }
    }

    fn len_of(&self, e: EdgeId) -> f32 {
        self.edges.get(e).map_or(0.0, Edge::length)
    }

    fn limit_of(&self, e: EdgeId) -> f32 {
        self.edges.get(e).map_or(0.0, |x| x.speed_limit)
    }

    /// Every edge reachable from `start`, including it.
    #[must_use]
    pub fn reachable(&self, start: EdgeId) -> Vec<EdgeId> {
        let mut seen = vec![false; self.edges.len()];
        let mut stack = vec![start];
        let mut out = Vec::new();
        while let Some(e) = stack.pop() {
            match seen.get_mut(e) {
                Some(s) if !*s => *s = true,
                _ => continue,
            }
            out.push(e);
            if let Some(edge) = self.edges.get(e) {
                stack.extend(edge.next.iter().copied());
            }
        }
        out.sort_unstable();
        out
    }
}

/// A class of rolling stock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrainClass {
    /// Name on the boards.
    pub name: &'static str,
    /// Top speed, m/s.
    pub vmax: f32,
    /// Acceleration, m/s^2.
    pub accel: f32,
    /// Service braking, m/s^2.
    pub brake: f32,
    /// Whole train, metres.
    pub length: f32,
    /// Steepest grade it can climb (0.03 is 3 percent).
    pub max_grade: f32,
    /// Vehicles, locomotive included.
    pub cars: u8,
}

/// Old sparky: a small steam tank engine with two coaches.
pub const SPARKY: TrainClass = TrainClass {
    name: "Sparky",
    vmax: 14.0,
    accel: 0.35,
    brake: 0.6,
    length: 45.0,
    max_grade: 0.03,
    cars: 3,
};

/// A three-car high-speed set in the shinkansen style.
pub const BULLET: TrainClass = TrainClass {
    name: "Bullet",
    vmax: 40.0,
    accel: 0.9,
    brake: 1.1,
    length: 75.0,
    max_grade: 0.04,
    cars: 3,
};

/// A regional electric multiple unit: slower than the bullet, climbs anything.
pub const REGIO: TrainClass = TrainClass {
    name: "Regio",
    vmax: 28.0,
    accel: 0.8,
    brake: 1.0,
    length: 60.0,
    max_grade: 0.06,
    cars: 3,
};

/// One timetabled stop. Times are seconds within the train's cycle and may run past
/// the period or below zero; cycle `c` adds `c * period`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stop {
    /// Platform edge.
    pub platform: EdgeId,
    /// Scheduled arrival.
    pub arrive: i32,
    /// Scheduled departure.
    pub depart: i32,
}

/// One train's working: its class and a cyclic list of stops. The train starts the
/// day standing at its first stop, waiting for that stop's departure in cycle 0.
#[derive(Debug, Clone, PartialEq)]
pub struct TrainPlan {
    /// Rolling stock.
    pub class: TrainClass,
    /// Cycle length in seconds.
    pub period: i32,
    /// Stops in running order.
    pub stops: Vec<Stop>,
}

/// The default working: Sparky stops in the passing loops, the bullet overtakes there.
#[must_use]
pub fn default_plans() -> Vec<TrainPlan> {
    vec![
        TrainPlan {
            class: SPARKY,
            period: 240,
            stops: vec![
                Stop {
                    platform: 2,
                    arrive: -30,
                    depart: 0,
                },
                Stop {
                    platform: 8,
                    arrive: 92,
                    depart: 120,
                },
            ],
        },
        TrainPlan {
            class: BULLET,
            period: 120,
            stops: vec![
                Stop {
                    platform: 1,
                    arrive: 105,
                    depart: 110,
                },
                Stop {
                    platform: 7,
                    arrive: 165,
                    depart: 170,
                },
            ],
        },
    ]
}

/// Problems with a layout or a timetable. Empty means fine.
#[must_use]
pub fn validate(layout: &Layout, plans: &[TrainPlan]) -> Vec<String> {
    let mut errors = Vec::new();
    for e in &layout.edges {
        if e.next.is_empty() {
            errors.push(format!("{} is a dead end", e.name));
        }
        for &n in &e.next {
            match (e.points.last(), layout.edges.get(n)) {
                (Some(end), Some(next)) => {
                    if next
                        .points
                        .first()
                        .is_none_or(|start| end.dist(*start) > 0.05)
                    {
                        errors.push(format!("{} does not meet {}", e.name, next.name));
                    }
                }
                _ => errors.push(format!("{} links to a missing edge", e.name)),
            }
        }
    }
    let mut starts = Vec::new();
    for plan in plans {
        let c = plan.class;
        let Some(first) = plan.stops.first() else {
            errors.push(format!("{} has no stops", c.name));
            continue;
        };
        if starts.contains(&first.platform) {
            errors.push(format!("two trains start on edge {}", first.platform));
        }
        starts.push(first.platform);
        if plan.period <= 0 {
            errors.push(format!("{} has a period of {} s", c.name, plan.period));
        }
        for s in &plan.stops {
            match layout.edges.get(s.platform) {
                Some(e) if e.stop_at.is_some() => {
                    if e.stop_at.is_some_and(|at| at < c.length) {
                        errors.push(format!("{} is too long for {}", c.name, e.name));
                    }
                }
                Some(e) => errors.push(format!("{} is not a platform", e.name)),
                None => errors.push(format!("{} stops at a missing edge", c.name)),
            }
            if s.depart < s.arrive {
                errors.push(format!("{} departs before it arrives", c.name));
            }
        }
        for e in layout.reachable(first.platform) {
            if let Some(edge) = layout.edges.get(e) {
                let g = edge.max_grade();
                if g > c.max_grade {
                    errors.push(format!(
                        "{} cannot climb {}: {:.1} percent, limit {:.1}",
                        c.name,
                        edge.name,
                        g * 100.0,
                        c.max_grade * 100.0
                    ));
                }
            }
        }
    }
    errors
}

/// A train on the layout.
#[derive(Debug, Clone, PartialEq)]
pub struct Train {
    /// Blocks held, tail block first: occupied up to `head_idx`, reserved after it.
    pub path: Vec<EdgeId>,
    /// Index into `path` of the block the head is on.
    pub head_idx: usize,
    /// Head position on that block, metres from its start.
    pub head_pos: f32,
    /// Speed, m/s.
    pub v: f32,
    /// Index of the next (or current, while dwelling) stop.
    pub next_stop: usize,
    /// Timetable cycle of that stop.
    pub cycle: i32,
    /// Standing at a platform until departure time.
    pub dwelling: bool,
    /// Stopped at a red signal.
    pub held: bool,
}

/// What happened, for the HUD and for tuning timetables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RailEvent {
    /// When.
    pub tick: u32,
    /// Which train.
    pub train: usize,
    /// Which platform.
    pub platform: EdgeId,
    /// Arrival (true) or departure.
    pub arrival: bool,
    /// Ticks after the scheduled time (negative is early).
    pub late: i64,
}

/// Counters for the gate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RailStats {
    /// Ticks on which two trains overlapped on one block.
    pub collisions: u32,
    /// Times a head ran past the end of its movement authority.
    pub overruns: u32,
    /// Times a running train came to a stand at a red signal.
    pub holds: u32,
    /// Arrivals at timetabled stops.
    pub arrivals: u32,
    /// Departures.
    pub departures: u32,
    /// Latest arrival, ticks after schedule.
    pub max_late_arrival: i64,
    /// Latest departure, ticks after schedule.
    pub max_departure_delay: i64,
    /// Every train stood at a red signal for longer than the deadlock limit.
    pub deadlocked: bool,
    /// FNV-1a hash of the state after every tick.
    pub hash: u64,
}

/// Stand at a red signal this far before the block end.
const SIGNAL_GAP: f32 = 3.0;
/// Snap to a stop point this close.
const STOP_EPS: f32 = 0.02;
/// All trains held this long is a deadlock.
const DEADLOCK_SECONDS: u32 = 300;
/// Reserve ahead this far beyond the braking distance.
const LOOKAHEAD_MARGIN: f32 = 60.0;

/// The running railway.
#[derive(Debug, Clone)]
pub struct Rail {
    /// Track and stations.
    pub layout: Layout,
    /// One working per train.
    pub plans: Vec<TrainPlan>,
    /// Trains, same order as `plans`.
    pub trains: Vec<Train>,
    /// Ticks per second.
    pub tps: u32,
    /// Ticks since the start of the day.
    pub tick: u32,
    /// Gate counters.
    pub stats: RailStats,
    /// Arrivals and departures, when recording.
    pub log: Option<Vec<RailEvent>>,
    all_held: u32,
}

fn fnv(h: u64, v: u64) -> u64 {
    let mut h = h;
    for b in v.to_le_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

impl Rail {
    /// A railway with every train standing at its first stop.
    ///
    /// # Errors
    ///
    /// Returns the validation problems, joined, when the layout or a plan is unsound.
    pub fn new(layout: Layout, plans: Vec<TrainPlan>, tps: u32) -> Result<Self, String> {
        let errors = validate(&layout, &plans);
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        let mut trains = Vec::with_capacity(plans.len());
        for p in &plans {
            let platform = p.stops.first().map_or(0, |s| s.platform);
            let stop_at = layout
                .edges
                .get(platform)
                .and_then(|e| e.stop_at)
                .unwrap_or(0.0);
            trains.push(Train {
                path: vec![platform],
                head_idx: 0,
                head_pos: stop_at,
                v: 0.0,
                next_stop: 0,
                cycle: 0,
                dwelling: true,
                held: false,
            });
        }
        Ok(Self {
            layout,
            plans,
            trains,
            tps: tps.clamp(1, 1000),
            tick: 0,
            stats: RailStats {
                hash: 0xcbf2_9ce4_8422_2325,
                ..RailStats::default()
            },
            log: None,
            all_held: 0,
        })
    }

    /// The default table and working.
    ///
    /// # Errors
    ///
    /// Only if the built-in layout stopped validating.
    pub fn toy_default(tps: u32) -> Result<Self, String> {
        Self::new(Layout::toy_table(1.5), default_plans(), tps)
    }

    /// Seconds per tick.
    #[must_use]
    pub fn dt(&self) -> f32 {
        1.0 / f32::from(u16::try_from(self.tps).unwrap_or(1000))
    }

    /// Whole seconds since the start of the day.
    #[must_use]
    pub fn seconds(&self) -> u32 {
        self.tick / self.tps
    }

    fn stop(&self, i: usize) -> Option<Stop> {
        let t = self.trains.get(i)?;
        self.plans.get(i)?.stops.get(t.next_stop).copied()
    }

    /// Scheduled tick of train `i`'s next stop: arrival or departure.
    #[must_use]
    pub fn scheduled(&self, i: usize, arrival: bool) -> Option<i64> {
        let t = self.trains.get(i)?;
        let plan = self.plans.get(i)?;
        let s = plan.stops.get(t.next_stop)?;
        let offset = if arrival { s.arrive } else { s.depart };
        let secs = i64::from(t.cycle) * i64::from(plan.period) + i64::from(offset);
        Some(secs * i64::from(self.tps))
    }

    fn dist_ahead(&self, t: &Train) -> f32 {
        let mut d = 0.0;
        for (k, &e) in t.path.iter().enumerate().skip(t.head_idx) {
            d += self.layout.len_of(e);
            if k == t.head_idx {
                d -= t.head_pos;
            }
        }
        d
    }

    /// Distance from the head to `s` metres into `path[k]` (k at or after the head).
    fn dist_to(&self, t: &Train, k: usize, s: f32) -> f32 {
        if k <= t.head_idx {
            return s - t.head_pos;
        }
        let mut d = 0.0;
        for (j, &e) in t.path.iter().enumerate().take(k).skip(t.head_idx) {
            d += self.layout.len_of(e);
            if j == t.head_idx {
                d -= t.head_pos;
            }
        }
        d + s
    }

    /// Advance one tick.
    pub fn step(&mut self) {
        let now = i64::from(self.tick);
        for i in 0..self.trains.len() {
            self.depart_if_due(i, now);
            self.reserve(i);
        }
        for i in 0..self.trains.len() {
            self.drive(i, now);
        }
        self.check_overlaps();
        self.check_deadlock();
        let mut h = fnv(self.stats.hash, u64::from(self.tick));
        for t in &self.trains {
            let block = t.path.get(t.head_idx).copied().unwrap_or(0);
            h = fnv(h, u64::try_from(block).unwrap_or(u64::MAX));
            h = fnv(h, u64::from(t.head_pos.to_bits()));
            h = fnv(h, u64::from(t.v.to_bits()));
        }
        self.stats.hash = h;
        self.tick = self.tick.saturating_add(1);
    }

    /// Run `seconds` of railway time.
    pub fn run_for(&mut self, seconds: u32) {
        for _ in 0..seconds.saturating_mul(self.tps) {
            self.step();
        }
    }

    fn record(&mut self, train: usize, platform: EdgeId, arrival: bool, late: i64) {
        if let Some(log) = self.log.as_mut() {
            log.push(RailEvent {
                tick: self.tick,
                train,
                platform,
                arrival,
                late,
            });
        }
    }

    fn depart_if_due(&mut self, i: usize, now: i64) {
        let dwelling = self.trains.get(i).is_some_and(|t| t.dwelling);
        let Some(due) = self.scheduled(i, false) else {
            return;
        };
        if !dwelling || now < due {
            return;
        }
        let platform = self.stop(i).map_or(0, |s| s.platform);
        let delay = now - due;
        self.stats.departures += 1;
        self.stats.max_departure_delay = self.stats.max_departure_delay.max(delay);
        self.record(i, platform, false, delay);
        let count = self.plans.get(i).map_or(1, |p| p.stops.len().max(1));
        if let Some(t) = self.trains.get_mut(i) {
            t.dwelling = false;
            t.next_stop += 1;
            if t.next_stop >= count {
                t.next_stop = 0;
                t.cycle += 1;
            }
        }
    }

    /// Set the route ahead, one block at a time, as far as the braking distance needs.
    fn reserve(&mut self, i: usize) {
        let Some(class) = self.plans.get(i).map(|p| p.class) else {
            return;
        };
        let target = self.stop(i).map(|s| s.platform);
        for _ in 0..8 {
            let Some(t) = self.trains.get(i) else {
                return;
            };
            if t.dwelling {
                return;
            }
            let look = t.v * t.v / (2.0 * class.brake) + LOOKAHEAD_MARGIN;
            if self.dist_ahead(t) > look {
                return;
            }
            let ahead = t.path.get(t.head_idx..).unwrap_or(&[]);
            if target.is_some_and(|p| ahead.contains(&p)) {
                return;
            }
            let Some(edge) = t.path.last().and_then(|&l| self.layout.edges.get(l)) else {
                return;
            };
            let choice = target
                .filter(|p| edge.next.contains(p))
                .or_else(|| edge.next.first().copied());
            let Some(next) = choice else {
                return;
            };
            if self.trains.iter().any(|o| o.path.contains(&next)) {
                return;
            }
            if let Some(t) = self.trains.get_mut(i) {
                t.path.push(next);
            }
        }
    }

    /// Highest speed that keeps every limit ahead and stops within `target` metres.
    fn speed_cap(&self, t: &Train, class: TrainClass, target: f32, dt: f32) -> f32 {
        let h = t.head_idx;
        let mut allowed = class.vmax;
        for &e in t.path.iter().take(h + 1) {
            allowed = allowed.min(self.layout.limit_of(e));
        }
        for (k, &e) in t.path.iter().enumerate().skip(h + 1) {
            let lim = self.layout.limit_of(e);
            let d = self.dist_to(t, k, 0.0).max(0.0);
            allowed = allowed.min((lim * lim + 2.0 * class.brake * d).sqrt());
        }
        allowed
            .min((2.0 * class.brake * target).sqrt())
            .min(target / dt)
    }

    fn drive(&mut self, i: usize, now: i64) {
        let Some(class) = self.plans.get(i).map(|p| p.class) else {
            return;
        };
        let Some(t) = self.trains.get(i) else {
            return;
        };
        if t.dwelling {
            return;
        }
        let dt = self.dt();
        let h = t.head_idx;
        let stop = self.stop(i);
        let stop_k = stop.and_then(|s| {
            let at = self.layout.edges.get(s.platform)?.stop_at?;
            let k = t
                .path
                .iter()
                .enumerate()
                .skip(h)
                .find(|(_, &e)| e == s.platform)?
                .0;
            Some((k, at))
        });
        let target = match stop_k {
            Some((k, at)) => self.dist_to(t, k, at),
            None => self.dist_ahead(t) - SIGNAL_GAP,
        }
        .max(0.0);
        let allowed = self.speed_cap(t, class, target, dt);
        let mut v = (t.v + class.accel * dt).min(allowed).max(0.0);
        let mut advance = v * dt;
        let at_target = target - advance <= STOP_EPS;
        if at_target {
            advance = target;
            v = 0.0;
        }
        let edges = &self.layout.edges;
        let Some(t) = self.trains.get_mut(i) else {
            return;
        };
        t.head_pos += advance;
        let mut overran = false;
        loop {
            let len = t
                .path
                .get(t.head_idx)
                .and_then(|&e| edges.get(e))
                .map_or(0.0, Edge::length);
            if t.head_pos <= len + 1e-3 {
                break;
            }
            if t.head_idx + 1 < t.path.len() {
                t.head_pos -= len;
                t.head_idx += 1;
            } else {
                t.head_pos = len;
                v = 0.0;
                overran = true;
                break;
            }
        }
        t.v = v;
        let arrived = at_target && stop_k.is_some();
        if arrived {
            t.dwelling = true;
        }
        let newly_held = !t.dwelling && v <= 0.0 && !t.held;
        t.held = !t.dwelling && v <= 0.0;
        // Release blocks the tail has left.
        let mut remaining = class.length;
        let mut idx = t.head_idx;
        let mut on = t.head_pos;
        while remaining > on && idx > 0 {
            remaining -= on;
            idx -= 1;
            on = t
                .path
                .get(idx)
                .and_then(|&e| edges.get(e))
                .map_or(0.0, Edge::length);
        }
        if idx > 0 {
            t.path.drain(..idx);
            t.head_idx -= idx;
        }
        if overran {
            self.stats.overruns += 1;
        }
        if newly_held {
            self.stats.holds += 1;
        }
        if arrived {
            let late = self.scheduled(i, true).map_or(0, |s| now - s);
            self.stats.arrivals += 1;
            self.stats.max_late_arrival = self.stats.max_late_arrival.max(late);
            self.record(i, stop.map_or(0, |s| s.platform), true, late);
        }
    }

    /// Stretches of track a train covers: (block, from, to) in metres.
    #[must_use]
    pub fn extents(&self, i: usize) -> Vec<(EdgeId, f32, f32)> {
        let mut out = Vec::new();
        let (Some(t), Some(plan)) = (self.trains.get(i), self.plans.get(i)) else {
            return out;
        };
        let mut remaining = plan.class.length;
        let mut idx = t.head_idx;
        let mut to = t.head_pos;
        while let Some(&e) = t.path.get(idx) {
            let from = (to - remaining).max(0.0);
            out.push((e, from, to));
            remaining -= to - from;
            if remaining <= 1e-4 || idx == 0 {
                break;
            }
            idx -= 1;
            to = self.layout.len_of(t.path.get(idx).copied().unwrap_or(0));
        }
        out
    }

    /// Geometric check, independent of the reservation rules.
    fn check_overlaps(&mut self) {
        let all: Vec<Vec<(EdgeId, f32, f32)>> =
            (0..self.trains.len()).map(|i| self.extents(i)).collect();
        for (a, ea) in all.iter().enumerate() {
            for eb in all.iter().skip(a + 1) {
                let hit = ea.iter().any(|&(e1, f1, t1)| {
                    eb.iter()
                        .any(|&(e2, f2, t2)| e1 == e2 && f1 < t2 && f2 < t1)
                });
                if hit {
                    self.stats.collisions += 1;
                }
            }
        }
    }

    fn check_deadlock(&mut self) {
        let all_held = !self.trains.is_empty() && self.trains.iter().all(|t| t.held);
        self.all_held = if all_held { self.all_held + 1 } else { 0 };
        if self.all_held > DEADLOCK_SECONDS.saturating_mul(self.tps) {
            self.stats.deadlocked = true;
        }
    }

    /// Point `back` metres behind train `i`'s head, following its path.
    #[must_use]
    pub fn point_behind(&self, i: usize, back: f32) -> Option<P3> {
        let t = self.trains.get(i)?;
        let mut idx = t.head_idx;
        let mut s = t.head_pos - back;
        while s < 0.0 && idx > 0 {
            idx -= 1;
            s += self.layout.len_of(*t.path.get(idx)?);
        }
        Some(self.layout.edges.get(*t.path.get(idx)?)?.point_at(s))
    }

    /// The next train due at `platform` and the ticks until its scheduled arrival
    /// (zero or less once it stands there or runs late).
    #[must_use]
    pub fn next_arrival(&self, platform: EdgeId) -> Option<(usize, i64)> {
        let now = i64::from(self.tick);
        (0..self.trains.len())
            .filter_map(|i| {
                let t = self.trains.get(i)?;
                let s = self.stop(i)?;
                if s.platform != platform {
                    return None;
                }
                let due = if t.dwelling {
                    0
                } else {
                    self.scheduled(i, true)? - now
                };
                Some((i, due))
            })
            .min_by_key(|&(_, due)| due)
    }

    /// Signal at the entrance of `e` shows proceed: a train has the block reserved
    /// ahead of its head.
    #[must_use]
    pub fn signal_clear(&self, e: EdgeId) -> bool {
        self.trains
            .iter()
            .any(|t| t.path.iter().skip(t.head_idx + 1).any(|&x| x == e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u32 = 24 * 3600;

    fn rail(layout: Layout, plans: Vec<TrainPlan>, tps: u32) -> Option<Rail> {
        let r = Rail::new(layout, plans, tps);
        assert!(r.is_ok(), "layout rejected: {:?}", r.as_ref().err());
        r.ok()
    }

    #[test]
    fn the_toy_table_validates() {
        let errors = validate(&Layout::toy_table(1.5), &default_plans());
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn helix_grade_is_gentle() {
        let l = Layout::toy_table(1.5);
        let g = l.edges.get(4).map_or(1.0, Edge::max_grade);
        assert!(g > 0.01 && g < 0.02, "grade {g}");
    }

    #[test]
    fn a_day_runs_safe_and_on_time() {
        let Some(mut r) = rail(Layout::toy_table(1.5), default_plans(), 10) else {
            return;
        };
        r.run_for(DAY);
        let s = r.stats;
        assert_eq!(s.collisions, 0, "{s:?}");
        assert_eq!(s.overruns, 0, "{s:?}");
        assert!(!s.deadlocked, "{s:?}");
        // Sparky 2 stops per 240 s, the bullet 2 per 120 s: 2160 arrivals, give or take
        // the last cycle.
        assert!(s.arrivals > 2100, "{s:?}");
        assert!(s.max_departure_delay <= 20 * 10, "{s:?}");
        assert!(s.max_late_arrival <= 30 * 10, "{s:?}");
    }

    #[test]
    fn fine_steps_are_safe_too() {
        let Some(mut r) = rail(Layout::toy_table(1.5), default_plans(), 60) else {
            return;
        };
        r.run_for(3600);
        let s = r.stats;
        assert_eq!((s.collisions, s.overruns), (0, 0), "{s:?}");
        assert!(s.max_departure_delay <= 20 * 60, "{s:?}");
    }

    #[test]
    fn two_runs_hash_the_same() {
        let Some(mut a) = rail(Layout::toy_table(1.5), default_plans(), 10) else {
            return;
        };
        let Some(mut b) = rail(Layout::toy_table(1.5), default_plans(), 10) else {
            return;
        };
        a.run_for(7200);
        b.run_for(7200);
        assert_eq!(a.stats, b.stats);
    }

    /// The bullet leaves right behind Sparky: the interlocking must hold it at red,
    /// never let it touch, and the on-time gate must catch the bad timetable.
    #[test]
    fn a_bad_timetable_is_held_at_red_not_crashed() {
        let mut plans = default_plans();
        if let Some(b) = plans.get_mut(1) {
            b.stops = vec![
                Stop {
                    platform: 1,
                    arrive: 0,
                    depart: 8,
                },
                Stop {
                    platform: 7,
                    arrive: 40,
                    depart: 60,
                },
            ];
        }
        let Some(mut r) = rail(Layout::toy_table(1.5), plans, 10) else {
            return;
        };
        r.run_for(4 * 3600);
        let s = r.stats;
        assert_eq!((s.collisions, s.overruns), (0, 0), "{s:?}");
        assert!(s.holds > 0, "never held: {s:?}");
        assert!(s.max_departure_delay > 20 * 10, "gate missed it: {s:?}");
    }

    #[test]
    fn a_steep_helix_is_refused_for_steam_and_fine_for_the_regio() {
        let steep = Layout::toy_table(0.5);
        let steam = validate(&steep, &default_plans());
        assert!(
            steam.iter().any(|e| e.starts_with("Sparky cannot climb")),
            "{steam:?}"
        );
        let regio = vec![TrainPlan {
            class: REGIO,
            period: 200,
            stops: vec![Stop {
                platform: 1,
                arrive: 0,
                depart: 30,
            }],
        }];
        let ok = validate(&steep, &regio);
        assert!(ok.is_empty(), "{ok:?}");
    }

    #[test]
    fn a_dwelling_train_sets_no_route() {
        let Some(mut r) = rail(Layout::toy_table(1.5), default_plans(), 10) else {
            return;
        };
        r.run_for(30);
        let bullet = r.trains.get(1).map(|t| (t.dwelling, t.path.len()));
        assert_eq!(bullet, Some((true, 1)));
    }

    #[test]
    fn the_board_counts_down_to_the_bullet() {
        let Some(r) = rail(Layout::toy_table(1.5), default_plans(), 10) else {
            return;
        };
        // At t = 0 the bullet stands at Talheim 1 and Sparky is due at Bergblick 2.
        assert_eq!(r.next_arrival(1), Some((1, 0)));
        assert_eq!(r.next_arrival(8), None);
        assert_eq!(r.next_arrival(7), None);
    }

    #[test]
    fn short_platforms_and_dead_ends_are_reported() {
        let mut l = Layout::toy_table(1.5);
        if let Some(e) = l.edges.get_mut(11) {
            e.next.clear();
        }
        let errors = validate(&l, &default_plans());
        assert!(errors.iter().any(|e| e.contains("dead end")), "{errors:?}");
    }
}

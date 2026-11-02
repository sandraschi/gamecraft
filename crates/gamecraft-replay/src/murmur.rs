//! Murmuration as a pure function of (config, order log).
//!
//! The order log is a list of changes: from `tick` on the lure heads for `(x, y)`.
//! A flare is a one-step pulse, so `flare: true` applies on its own tick only; the
//! target is held until the next entry. Before the first entry the lure holds still.

use crate::hash::Fnv;
use crate::{Outcome, ReplayError, Status, MAX_TICKS};
use gamecraft_sim::flock::{autopilot, Flock, FlockConfig, Orders};
use serde::{Deserialize, Serialize};

/// Largest flock a replay may ask for.
pub const MAX_COUNT: usize = 100_000;

/// Largest hawk or pillar count a replay may ask for.
pub const MAX_ACTORS: usize = 64;

/// Largest accepted coordinate magnitude in an order.
pub const MAX_COORD: f32 = 1.0e6;

/// The level: the serialisable twin of [`FlockConfig`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlockSpec {
    /// Starting flock size.
    pub count: usize,
    /// Level seed.
    pub seed: u64,
    /// Number of hawks.
    pub hawks: usize,
    /// Number of pillars.
    pub pillars: usize,
    /// Steps before the evening ends.
    pub time_limit: u32,
}

impl From<FlockConfig> for FlockSpec {
    fn from(c: FlockConfig) -> Self {
        Self {
            count: c.count,
            seed: c.seed,
            hawks: c.hawks,
            pillars: c.pillars,
            time_limit: c.time_limit,
        }
    }
}

impl From<FlockSpec> for FlockConfig {
    fn from(s: FlockSpec) -> Self {
        Self {
            count: s.count,
            seed: s.seed,
            hawks: s.hawks,
            pillars: s.pillars,
            time_limit: s.time_limit,
        }
    }
}

impl FlockSpec {
    /// The shipped level.
    #[must_use]
    pub fn standard() -> Self {
        FlockConfig::standard().into()
    }

    /// The shipped level with a smaller flock.
    #[must_use]
    pub fn small(count: usize) -> Self {
        FlockConfig::small(count).into()
    }

    /// Reject configs the sim was never meant to run.
    ///
    /// # Errors
    /// [`ReplayError::Invalid`] for a flock that is empty or over [`MAX_COUNT`], or too
    /// many hawks or pillars.
    pub fn validate(&self) -> Result<(), ReplayError> {
        if self.count == 0 || self.count > MAX_COUNT {
            return Err(ReplayError::Invalid(format!(
                "flock count {} must be in 1..={MAX_COUNT}",
                self.count
            )));
        }
        if self.hawks > MAX_ACTORS || self.pillars > MAX_ACTORS {
            return Err(ReplayError::Invalid(format!(
                "hawks and pillars are limited to {MAX_ACTORS}"
            )));
        }
        Ok(())
    }
}

/// From `tick` (0 is the first step) the lure heads for `(x, y)`; `flare` fires on this step.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OrderChange {
    /// Step the order starts on.
    pub tick: u32,
    /// Lure target x.
    pub x: f32,
    /// Lure target y.
    pub y: f32,
    /// Fire a flare on this step.
    pub flare: bool,
}

/// Step a fresh flock for up to `ticks` steps, asking `orders` what to do each step.
fn run(
    spec: &FlockSpec,
    ticks: u32,
    mut orders: impl FnMut(&Flock, u32) -> Orders,
) -> Result<Flock, ReplayError> {
    spec.validate()?;
    if u64::from(ticks) > MAX_TICKS {
        return Err(ReplayError::Invalid(format!(
            "ticks {ticks} exceeds the limit {MAX_TICKS}"
        )));
    }
    let mut flock = Flock::new((*spec).into());
    for t in 0..ticks {
        let s = flock.stats();
        if s.won || s.lost {
            break;
        }
        let o = orders(&flock, t);
        flock.step(o);
    }
    Ok(flock)
}

/// Fingerprint and summary of a finished (or cut-off) run. Public so the Bevy runtime's
/// tests can compare the real schedule against a replay with the very same function.
#[must_use]
pub fn outcome_of(flock: &Flock) -> Outcome {
    let s = flock.stats();
    let mut h = Fnv::new();
    h.u32(s.tick);
    h.usize(s.alive);
    h.usize(s.eaten);
    h.usize(s.in_roost);
    h.flag(s.won);
    h.flag(s.lost);
    h.f32(flock.lure.0);
    h.f32(flock.lure.1);
    for i in 0..flock.px.len() {
        h.flag(flock.alive[i]);
        h.f32(flock.px[i]);
        h.f32(flock.py[i]);
        h.f32(flock.vx[i]);
        h.f32(flock.vy[i]);
    }
    Outcome {
        status: if s.won {
            Status::Won
        } else if s.lost {
            Status::Lost
        } else {
            Status::Incomplete
        },
        ticks: u64::from(s.tick),
        state_hash: h.hex(),
        summary: serde_json::json!({
            "alive": s.alive,
            "eaten": s.eaten,
            "in_roost": s.in_roost,
            "needed": s.needed,
        }),
    }
}

/// Play an order log for up to `ticks` steps (stops early on a win or a loss).
///
/// # Errors
/// [`ReplayError::Invalid`] for a bad config, too many ticks, an order outside
/// [`MAX_COORD`] or not finite, or order ticks that are not strictly increasing.
pub fn simulate(spec: &FlockSpec, log: &[OrderChange], ticks: u32) -> Result<Outcome, ReplayError> {
    let mut prev: Option<u32> = None;
    for c in log {
        let sane =
            c.x.is_finite() && c.y.is_finite() && c.x.abs() <= MAX_COORD && c.y.abs() <= MAX_COORD;
        if !sane {
            return Err(ReplayError::Invalid(format!(
                "order at tick {} has a coordinate that is not finite or beyond {MAX_COORD}",
                c.tick
            )));
        }
        if prev.is_some_and(|p| c.tick <= p) {
            return Err(ReplayError::Invalid(format!(
                "order tick {} is not strictly increasing",
                c.tick
            )));
        }
        prev = Some(c.tick);
    }
    let mut next = 0usize;
    let mut target: Option<(f32, f32)> = None;
    let flock = run(spec, ticks, |f, t| {
        let mut flare = false;
        if let Some(c) = log.get(next).filter(|c| c.tick == t) {
            target = Some((c.x, c.y));
            flare = c.flare;
            next += 1;
        }
        Orders {
            target: target.unwrap_or(f.lure),
            flare,
        }
    })?;
    Ok(outcome_of(&flock))
}

/// Record the built-in autopilot for up to `max_ticks` steps.
/// Returns the order log and the number of steps actually run.
///
/// # Errors
/// [`ReplayError::Invalid`] for a bad config or `max_ticks` over [`MAX_TICKS`].
pub fn record_autopilot(
    spec: &FlockSpec,
    max_ticks: u32,
) -> Result<(Vec<OrderChange>, u32), ReplayError> {
    let mut log: Vec<OrderChange> = Vec::new();
    let mut last: Option<(u32, u32)> = None;
    let flock = run(spec, max_ticks, |f, t| {
        let o = autopilot(f);
        let bits = (o.target.0.to_bits(), o.target.1.to_bits());
        let changed = last != Some(bits);
        if changed || o.flare {
            log.push(OrderChange {
                tick: t,
                x: o.target.0,
                y: o.target.1,
                flare: o.flare,
            });
            last = Some(bits);
        }
        o
    })?;
    Ok((log, flock.stats().tick))
}

//! Toy railway on Bevy: the block-signalled sim from `gamecraft-sim::rail`, stepped
//! in `FixedUpdate` at 60 Hz.
//!
//! The logic plugin keeps the whole railway in one resource ([`RailWorld`]) and runs
//! [`Clockwork::speed`] sim ticks per fixed tick, so the timetable can be fast-forwarded
//! without changing the result: the sim never sees a variable timestep. It spawns no
//! entities, so tests drive it under `MinimalPlugins`. Drawing lives in
//! `render_railway` (feature `render`).

use bevy::prelude::*;
use gamecraft_sim::rail::{Rail, RailStats};

/// Printed once the 24-hour proof passes.
pub const WIN_MARKER: &str = "GameCraft: railway ran safe and on time";
/// Printed when it does not.
pub const LOSS_MARKER: &str = "GameCraft: railway failed its day";
/// Ticks per second, matching the fixed timestep.
pub const TPS: u32 = 60;
/// Latest acceptable arrival, seconds after schedule.
pub const GATE_LATE_S: i64 = 30;
/// Latest acceptable departure, seconds after schedule.
pub const GATE_DELAY_S: i64 = 20;
/// Seconds in a railway day.
pub const DAY_S: u32 = 24 * 3600;

/// The running railway.
#[derive(Resource, Debug, Clone)]
pub struct RailWorld(pub Rail);

/// How fast railway time runs.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clockwork {
    /// Sim ticks per fixed tick: 1 is real time.
    pub speed: u32,
    /// Frozen.
    pub paused: bool,
}

impl Default for Clockwork {
    fn default() -> Self {
        Self {
            speed: 1,
            paused: false,
        }
    }
}

/// Rules for the railway.
#[derive(Debug, Clone)]
pub struct RailwayLogicPlugin {
    /// The railway to run (validated already).
    pub rail: Rail,
    /// Starting speed.
    pub speed: u32,
}

impl Plugin for RailwayLogicPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(60.0))
            .insert_resource(RailWorld(self.rail.clone()))
            .insert_resource(Clockwork {
                speed: self.speed.max(1),
                paused: false,
            })
            .add_systems(FixedUpdate, step_rail);
    }
}

fn step_rail(mut world: ResMut<RailWorld>, clock: Res<Clockwork>) {
    if clock.paused {
        return;
    }
    for _ in 0..clock.speed {
        world.0.step();
    }
}

/// Judge a finished run: `Ok` with the summary line, or `Err` with what broke.
///
/// # Errors
///
/// Returns the failure summary when a train touched another, ran past its authority,
/// deadlocked, or ran outside the punctuality limits.
pub fn verdict(stats: &RailStats, tps: u32) -> Result<String, String> {
    let tps = i64::from(tps.max(1));
    let late = stats.max_late_arrival / tps;
    let delay = stats.max_departure_delay / tps;
    let summary = format!(
        "arrivals {}, collisions {}, overruns {}, held at red {}, worst arrival {late:+} s, worst departure {delay:+} s",
        stats.arrivals, stats.collisions, stats.overruns, stats.holds
    );
    let mut broke = Vec::new();
    if stats.collisions > 0 {
        broke.push("collision");
    }
    if stats.overruns > 0 {
        broke.push("overrun");
    }
    if stats.deadlocked {
        broke.push("deadlock");
    }
    if late > GATE_LATE_S {
        broke.push("late arrivals");
    }
    if delay > GATE_DELAY_S {
        broke.push("late departures");
    }
    if stats.arrivals == 0 {
        broke.push("nothing arrived");
    }
    if broke.is_empty() {
        Ok(summary)
    } else {
        Err(format!("{}: {summary}", broke.join(", ")))
    }
}

/// Run a copy of `rail` for a whole day and judge it.
///
/// # Errors
///
/// See [`verdict`].
pub fn prove_a_day(rail: &Rail) -> Result<String, String> {
    let mut copy = rail.clone();
    copy.run_for(DAY_S);
    verdict(&copy.stats, copy.tps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn headless(speed: u32) -> Option<App> {
        let rail = Rail::toy_default(TPS).ok()?;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_secs_f64(1.0 / 60.0),
            ))
            .add_plugins(RailwayLogicPlugin { rail, speed });
        Some(app)
    }

    #[test]
    fn the_schedule_runs_one_tick_per_fixed_step() {
        let Some(mut app) = headless(1) else {
            return;
        };
        for _ in 0..600 {
            app.update();
        }
        let tick = app.world().resource::<RailWorld>().0.tick;
        // The first update only starts the clock.
        assert!((598..=600).contains(&tick), "tick {tick}");
    }

    #[test]
    fn speed_multiplies_ticks_and_pause_stops_them() {
        let Some(mut app) = headless(8) else {
            return;
        };
        for _ in 0..61 {
            app.update();
        }
        let fast = app.world().resource::<RailWorld>().0.tick;
        assert!(fast >= 8 * 59, "tick {fast}");
        app.world_mut().resource_mut::<Clockwork>().paused = true;
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(app.world().resource::<RailWorld>().0.tick, fast);
    }

    #[test]
    fn bevy_and_the_bare_sim_agree() {
        let Some(mut app) = headless(4) else {
            return;
        };
        for _ in 0..1201 {
            app.update();
        }
        let bevy_side = app.world().resource::<RailWorld>().0.clone();
        let Ok(mut bare) = Rail::toy_default(TPS) else {
            return;
        };
        for _ in 0..bevy_side.tick {
            bare.step();
        }
        assert_eq!(bare.stats, bevy_side.stats);
        assert_eq!(bare.trains, bevy_side.trains);
    }

    #[test]
    fn the_verdict_rejects_a_crash() {
        let stats = RailStats {
            arrivals: 10,
            collisions: 1,
            ..RailStats::default()
        };
        let v = verdict(&stats, TPS);
        assert!(v.is_err_and(|e| e.starts_with("collision")));
    }

    #[test]
    fn an_hour_of_the_default_day_passes() {
        let Ok(mut rail) = Rail::toy_default(TPS) else {
            return;
        };
        rail.run_for(3600);
        let v = verdict(&rail.stats, rail.tps);
        assert!(v.is_ok(), "{v:?}");
    }
}

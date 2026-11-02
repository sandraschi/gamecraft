//! Murmuration on Bevy: the flock sim from `gamecraft-sim` driven by Bevy's fixed
//! timestep.
//!
//! The logic plugin holds the whole world in one resource ([`FlockWorld`]) and steps it
//! at 60 Hz in `FixedUpdate`. It spawns no entities at all, so tests run it under
//! `MinimalPlugins` with no window and no GPU. The render plugin (feature `render`)
//! mirrors the arrays into entities for drawing.
//!
//! Player input is the [`Steer`] resource: where the lure should head and whether to
//! fire a flare. With [`FlockAutoplay`] the sim's scripted pilot flies instead.

use bevy::prelude::*;
use gamecraft_sim::flock::{autopilot, Flock, FlockConfig, Orders};

/// Printed once when the flock settles in the roost.
pub const WIN_MARKER: &str = "GameCraft: murmuration settled, roost reached";
/// Printed once when the evening is lost.
pub const LOSS_MARKER: &str = "GameCraft: murmuration lost";

/// The whole simulated world.
#[derive(Resource, Debug, Clone)]
pub struct FlockWorld(pub Flock);

/// What the player is asking for. Written by input, read by the fixed step.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct Steer {
    /// Where the lure heads; `None` holds position.
    pub target: Option<(f32, f32)>,
    /// Fire a flare on the next step (cleared once consumed).
    pub flare: bool,
}

/// Marker resource: the scripted pilot flies instead of the keyboard.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct FlockAutoplay;

/// Whether the result has been announced already.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Announced(pub bool);

/// Rules for the murmuration.
#[derive(Debug, Clone, Copy)]
pub struct MurmurationLogicPlugin {
    /// The level.
    pub cfg: FlockConfig,
    /// Let the scripted pilot fly.
    pub autoplay: bool,
}

impl Plugin for MurmurationLogicPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(60.0))
            .insert_resource(FlockWorld(Flock::new(self.cfg)))
            .init_resource::<Steer>()
            .init_resource::<Announced>()
            .add_systems(FixedUpdate, (step_flock, announce).chain());
        if self.autoplay {
            app.insert_resource(FlockAutoplay);
        }
    }
}

fn step_flock(
    mut world: ResMut<FlockWorld>,
    mut steer: ResMut<Steer>,
    auto: Option<Res<FlockAutoplay>>,
) {
    let orders = if auto.is_some() {
        autopilot(&world.0)
    } else {
        let orders = Orders {
            target: steer.target.unwrap_or(world.0.lure),
            flare: steer.flare,
        };
        steer.flare = false;
        orders
    };
    world.0.step(orders);
}

fn announce(world: Res<FlockWorld>, mut announced: ResMut<Announced>) {
    let s = world.0.stats();
    if announced.0 || !(s.won || s.lost) {
        return;
    }
    announced.0 = true;
    let marker = if s.won { WIN_MARKER } else { LOSS_MARKER };
    println!(
        "{marker} (roost {} of {} needed, alive {} of {}, eaten {}, tick {})",
        s.in_roost, s.needed, s.alive, world.0.cfg.count, s.eaten, s.tick
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// A windowless app on a manual clock: each `update()` is one 60 Hz fixed tick.
    fn headless(cfg: FlockConfig, autoplay: bool) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_secs_f64(1.0 / 60.0),
            ))
            .add_plugins(MurmurationLogicPlugin { cfg, autoplay });
        app
    }

    fn run_until_over(app: &mut App, limit: u32) {
        for _ in 0..limit {
            app.update();
            let s = app.world().resource::<FlockWorld>().0.stats();
            if s.won || s.lost {
                break;
            }
        }
    }

    #[test]
    fn fixed_rate_matches_the_sim() {
        let app = headless(FlockConfig::small(100), false);
        let step = app.world().resource::<Time<Fixed>>().timestep();
        assert!((step.as_secs_f64() - 1.0 / 60.0).abs() < 1e-9);
    }

    #[test]
    fn autoplay_settles_the_flock_headless() {
        let cfg = FlockConfig {
            seed: 3,
            ..FlockConfig::small(2000)
        };
        let mut app = headless(cfg, true);
        run_until_over(&mut app, cfg.time_limit + 10);
        let s = app.world().resource::<FlockWorld>().0.stats();
        assert!(s.won, "autoplay did not settle the flock: {s:?}");
        assert!(app.world().resource::<Announced>().0);
    }

    #[test]
    fn a_flock_nobody_steers_is_lost() {
        let cfg = FlockConfig::small(1500);
        let mut app = headless(cfg, false);
        run_until_over(&mut app, cfg.time_limit + 10);
        let s = app.world().resource::<FlockWorld>().0.stats();
        assert!(s.lost && !s.won, "{s:?}");
    }

    #[test]
    fn a_flare_request_is_consumed_and_starts_the_cooldown() {
        let mut app = headless(FlockConfig::small(500), false);
        app.world_mut().resource_mut::<Steer>().flare = true;
        for _ in 0..3 {
            app.update();
        }
        assert!(!app.world().resource::<Steer>().flare);
        assert!(app.world().resource::<FlockWorld>().0.flare_cooldown > 0);
    }

    #[test]
    fn steering_moves_the_lure() {
        let mut app = headless(FlockConfig::small(500), false);
        let start = app.world().resource::<FlockWorld>().0.lure;
        app.world_mut().resource_mut::<Steer>().target = Some((start.0 + 100.0, start.1));
        for _ in 0..120 {
            app.update();
        }
        let now = app.world().resource::<FlockWorld>().0.lure;
        assert!(now.0 > start.0 + 10.0, "lure stayed at {now:?}");
    }
}

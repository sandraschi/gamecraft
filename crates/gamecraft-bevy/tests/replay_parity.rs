//! Parity between `.gcraft` replays and the real Bevy schedules.
//!
//! `gamecraft-replay` re-implements the maze rules as a pure function so a replay can be
//! checked without an engine. These tests feed a replay's inputs into the actual Bevy
//! systems, tick by tick, and require the very same result. If the rules in either place
//! change without the other, one of these fails.
#![allow(clippy::unwrap_used)]

use bevy::prelude::*;
use gamecraft_bevy::maze::{self, Intent, MazeLevel, MazeLogicPlugin, MazeRun};
use gamecraft_bevy::murmuration::{FlockWorld, MurmurationLogicPlugin, Steer};
use gamecraft_replay::maze::{MazeConfig, STEP_TICKS};
use gamecraft_replay::murmur::{self, FlockSpec};
use gamecraft_replay::{Replay, Session, Status};
use std::time::Duration;

fn headless(hz: f64) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).insert_resource(
        bevy::time::TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / hz)),
    );
    app
}

#[test]
fn maze_step_cadence_is_shared() {
    assert_eq!(u64::from(maze::STEP_TICKS), STEP_TICKS);
}

#[test]
fn maze_replay_inputs_drive_the_bevy_rules_to_the_same_result() {
    for cfg in [MazeConfig::depot(), MazeConfig::challenge()] {
        let replay = Replay::record_maze(cfg).unwrap();
        let Session::Maze { inputs, .. } = &replay.session else {
            unreachable!("recorded a maze");
        };
        let mut app = headless(64.0);
        app.add_plugins(MazeLogicPlugin {
            level: MazeLevel::generate(cfg.size, cfg.seed),
            autoplay: false,
        });
        let mut next = 0usize;
        for _ in 0..=replay.result.ticks {
            let tick = app.world().resource::<MazeRun>().tick + 1;
            if inputs.get(next).is_some_and(|i| i.tick == tick) {
                let i = inputs[next];
                app.world_mut().resource_mut::<Intent>().0 = Some((i.dx, i.dy));
                next += 1;
            }
            app.update();
            if app.world().resource::<MazeRun>().won {
                break;
            }
        }
        let run = app.world().resource::<MazeRun>().clone();
        assert_eq!(replay.result.status, Status::Won);
        assert!(run.won, "{cfg:?}: Bevy did not win from the replay inputs");
        assert_eq!(run.won_at_tick, Some(replay.result.ticks), "{cfg:?}");
        assert_eq!(replay.result.summary["moves"], run.moves, "{cfg:?}");
        assert_eq!(replay.result.summary["bumps"], run.bumps, "{cfg:?}");
    }
}

#[test]
fn maze_autoplay_and_recorded_replay_agree() {
    let cfg = MazeConfig::challenge();
    let replay = Replay::record_maze(cfg).unwrap();
    let mut app = headless(64.0);
    app.add_plugins(MazeLogicPlugin {
        level: MazeLevel::generate(cfg.size, cfg.seed),
        autoplay: true,
    });
    for _ in 0..20_000 {
        app.update();
        if app.world().resource::<MazeRun>().won {
            break;
        }
    }
    let run = app.world().resource::<MazeRun>().clone();
    assert_eq!(run.won_at_tick, Some(replay.result.ticks));
    assert_eq!(replay.result.summary["moves"], run.moves);
}

#[test]
fn murmuration_replay_orders_drive_the_bevy_schedule_to_the_same_state() {
    let spec = FlockSpec {
        seed: 3,
        ..FlockSpec::small(1500)
    };
    let replay = Replay::record_murmuration(spec, spec.time_limit).unwrap();
    let Session::Murmuration { orders, .. } = &replay.session else {
        unreachable!("recorded a murmuration");
    };
    let mut app = headless(60.0);
    app.add_plugins(MurmurationLogicPlugin {
        cfg: spec.into(),
        autoplay: false,
    });
    let mut next = 0usize;
    // The first `update()` can pass without a fixed step, so count the sim's own steps.
    let goal = u32::try_from(replay.result.ticks).unwrap();
    for _ in 0..=goal + 4 {
        let t = app.world().resource::<FlockWorld>().0.stats().tick;
        if t >= goal {
            break;
        }
        if orders.get(next).is_some_and(|o| o.tick == t) {
            let o = orders[next];
            let mut steer = app.world_mut().resource_mut::<Steer>();
            steer.target = Some((o.x, o.y));
            steer.flare = o.flare;
            next += 1;
        }
        app.update();
    }
    let actual = murmur::outcome_of(&app.world().resource::<FlockWorld>().0);
    assert_eq!(actual.status, replay.result.status);
    assert_eq!(actual.ticks, replay.result.ticks);
    assert_eq!(
        actual.state_hash, replay.result.state_hash,
        "Bevy drifted from the replay: {actual:?} vs {:?}",
        replay.result
    );
}

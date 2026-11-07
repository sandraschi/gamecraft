//! `gamecraft-bevy-railway`: a two-level toy railway with a working timetable.
//!
//! ```text
//! cargo run --profile play -p gamecraft-bevy --features render --bin gamecraft-bevy-railway
//!     --gate          run the whole day headless first; print the verdict, exit 1 on failure
//!     --tour DIR      scripted camera tour: save PNGs into DIR, then close
//!     --speed N       start N times faster than real time (default 1)
//! ```
//!
//! Sparky, a steam tank engine, stops in the passing loops at Talheim (below) and
//! Bergblick (above); the bullet overtakes it there. Two helices join the levels. The
//! interlocking keeps the trains apart; the timetable keeps them on time.
//!
//! Keys: Up/Down (or W/S) zoom from the whole table to a platform, Left/Right orbit or
//! look around, Tab next platform, F follow the next train in, +/- speed, Space pause,
//! N day/night, Home whole table.

#![allow(clippy::needless_pass_by_value)]

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use gamecraft_bevy::railway::{
    prove_a_day, Clockwork, RailWorld, RailwayLogicPlugin, LOSS_MARKER, TPS, WIN_MARKER,
};
use gamecraft_bevy::render_railway::{CamRig, RailwayRenderPlugin};
use gamecraft_sim::rail::Rail;

/// One scripted picture.
#[derive(Debug, Clone, Copy)]
struct Shot {
    name: &'static str,
    /// Railway second to take it at.
    at: u32,
    zoom: f32,
    platform: usize,
    yaw: f32,
}

/// The tour: the whole table, above Talheim, the bullet coming in (day and night).
const SHOTS: [Shot; 5] = [
    Shot {
        name: "1-table",
        at: 60,
        zoom: 1.0,
        platform: 0,
        yaw: 0.35,
    },
    Shot {
        name: "2-above-talheim",
        at: 150,
        zoom: 0.5,
        platform: 0,
        yaw: 0.35,
    },
    Shot {
        name: "3-platform-day",
        at: 216,
        zoom: 0.0,
        platform: 0,
        yaw: 0.35,
    },
    Shot {
        name: "4-platform-night",
        at: 456,
        zoom: 0.0,
        platform: 0,
        yaw: 0.35,
    },
    Shot {
        name: "5-table-night",
        at: 500,
        zoom: 1.0,
        platform: 0,
        yaw: -0.5,
    },
];

#[derive(Resource, Debug, Clone)]
struct Tour {
    dir: String,
    next: usize,
    frames: u32,
    /// Rendered frames and seconds while holding still for shots, for the fps line.
    held_frames: u32,
    held_secs: f32,
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rail = match Rail::toy_default(TPS) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("layout rejected: {e}");
            std::process::exit(2);
        }
    };
    if args.iter().any(|a| a == "--gate") {
        match prove_a_day(&rail) {
            Ok(summary) => println!("{WIN_MARKER} (24 h: {summary})"),
            Err(why) => {
                println!("{LOSS_MARKER} ({why})");
                std::process::exit(1);
            }
        }
    }
    let speed = arg_value(&args, "--speed")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1);
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Toy railway (Bevy)".into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(RailwayLogicPlugin { rail, speed })
    .add_plugins(RailwayRenderPlugin);
    if let Some(dir) = arg_value(&args, "--tour") {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("cannot create {dir}: {e}");
            std::process::exit(2);
        }
        app.insert_resource(Tour {
            dir,
            next: 0,
            frames: 0,
            held_frames: 0,
            held_secs: 0.0,
        })
        .add_systems(Update, tour);
    }
    app.run();
}

/// Fast-forward to each shot, freeze the railway, put the camera there, let the
/// post-processing settle, save the picture, move on; close after the last one.
fn tour(
    mut commands: Commands,
    time: Res<Time>,
    mut tour: ResMut<Tour>,
    world: Res<RailWorld>,
    mut clock: ResMut<Clockwork>,
    mut rig: ResMut<CamRig>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(shot) = SHOTS.get(tour.next).copied() else {
        tour.frames += 1;
        if tour.frames > 30 {
            let fps = f32::from(u16::try_from(tour.held_frames).unwrap_or(u16::MAX))
                / tour.held_secs.max(1e-3);
            println!("tour: {fps:.0} fps average while holding for shots");
            exit.write(AppExit::Success);
        }
        return;
    };
    let now = world.0.seconds();
    if now < shot.at && tour.frames == 0 {
        clock.paused = false;
        clock.speed = if shot.at - now > 4 { 32 } else { 1 };
        return;
    }
    clock.paused = true;
    rig.zoom = shot.zoom;
    rig.zoom_target = shot.zoom;
    rig.platform = shot.platform;
    rig.yaw = shot.yaw;
    rig.look = 0.0;
    rig.snap = true;
    tour.frames += 1;
    if tour.frames > 10 {
        tour.held_frames += 1;
        tour.held_secs += time.delta_secs();
    }
    if tour.frames == 40 {
        let path = format!("{}/{}.png", tour.dir, shot.name);
        println!("tour: {path} at railway second {now}");
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if tour.frames >= 55 {
        tour.frames = 0;
        tour.next += 1;
    }
}

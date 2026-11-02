//! `gamecraft-bevy-murmuration`: steer a flock of 20 000 starlings to the roost.
//!
//! ```text
//! cargo run --profile play -p gamecraft-bevy --features render --bin gamecraft-bevy-murmuration
//!     --autoplay        the scripted pilot flies (or GAMECRAFT_AUTOPLAY=1)
//!     --exit-on-win     close shortly after the evening is decided (for gates; also mutes)
//!     --mute            no sound (or GAMECRAFT_MUTE=1)
//!     --screenshot F    save a PNG to F
//!     --shot-tick T     with --screenshot: take it at sim tick T (default: at the end)
//!     --flock N         flock size (default 20000)
//!     --overview        start with the whole arena in view (C toggles)
//! ```
//!
//! Arrow keys or WASD steer the lure (hold), Space fires a flare that scatters hawks
//! near the lure. Get 85 percent of the flock into the gold roost before the time runs
//! out. The result prints `GameCraft: murmuration settled, roost reached` on a win.

#![allow(clippy::needless_pass_by_value)]

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use gamecraft_bevy::audio::{self, MurmurationAudioPlugin};
use gamecraft_bevy::murmuration::{FlockWorld, MurmurationLogicPlugin};
use gamecraft_bevy::render_murmuration::MurmurationRenderPlugin;
use gamecraft_sim::flock::FlockConfig;

/// Screenshot request from the command line.
#[derive(Resource, Debug, Clone)]
struct Shot {
    path: String,
    at_tick: Option<u32>,
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let autoplay = args.iter().any(|a| a == "--autoplay")
        || std::env::var("GAMECRAFT_AUTOPLAY").is_ok_and(|v| v == "1");
    let exit_on_win = args.iter().any(|a| a == "--exit-on-win");
    let mut cfg = FlockConfig::standard();
    if let Some(n) = arg_value(&args, "--flock").and_then(|v| v.parse::<usize>().ok()) {
        cfg.count = n;
    }
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Murmuration (Bevy)".into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(MurmurationLogicPlugin { cfg, autoplay })
    .add_plugins(MurmurationRenderPlugin {
        follow: !args.iter().any(|a| a == "--overview"),
    });
    if let Some(path) = arg_value(&args, "--screenshot") {
        app.insert_resource(Shot {
            path,
            at_tick: arg_value(&args, "--shot-tick").and_then(|v| v.parse().ok()),
        });
    }
    if !audio::muted(&args) {
        app.add_plugins(MurmurationAudioPlugin);
    }
    if exit_on_win {
        app.add_systems(Update, finish);
    }
    app.run();
}

/// Take the screenshot when asked, then close once the evening is decided and settled.
fn finish(
    mut commands: Commands,
    world: Res<FlockWorld>,
    time: Res<Time>,
    shot: Option<Res<Shot>>,
    mut over_since: Local<Option<f32>>,
    mut taken: Local<bool>,
    mut exit: MessageWriter<AppExit>,
) {
    let s = world.0.stats();
    let over = s.won || s.lost;
    let due = shot.as_ref().is_some_and(|sh| match sh.at_tick {
        Some(t) => s.tick >= t,
        None => over,
    });
    if let Some(sh) = shot.filter(|_| due && !*taken) {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(sh.path.clone()));
        *taken = true;
    }
    if !over {
        return;
    }
    let elapsed = time.elapsed_secs() - *over_since.get_or_insert(time.elapsed_secs());
    if elapsed > 1.5 {
        exit.write(AppExit::Success);
    }
}

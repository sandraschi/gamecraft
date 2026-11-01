//! `gamecraft-bevy-maze`: the depot maze, playable on Bevy.
//!
//! ```text
//! cargo run -p gamecraft-bevy --features render --bin gamecraft-bevy-maze
//!     --autoplay       let the built-in driver walk the shortest path
//!                      (or GAMECRAFT_AUTOPLAY=1, as for the Godot games)
//!     --exit-on-win    close shortly after the goal (for gates and demos)
//!     --screenshot F   with --exit-on-win: save the solved board to PNG file F
//! ```
//!
//! Arrow keys or WASD move. The win prints `GameCraft: maze solved, goal reached`.

#![allow(clippy::needless_pass_by_value)]

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use gamecraft_bevy::maze::{MazeLevel, MazeLogicPlugin, MazeRun};
use gamecraft_bevy::render::MazeRenderPlugin;

/// Where `--screenshot` saves, if asked.
#[derive(Resource, Debug, Clone)]
struct ShotPath(String);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let autoplay = args.iter().any(|a| a == "--autoplay")
        || std::env::var("GAMECRAFT_AUTOPLAY").is_ok_and(|v| v == "1");
    let exit_on_win = args.iter().any(|a| a == "--exit-on-win");
    let shot = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "GameCraft Maze (Bevy)".into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(MazeLogicPlugin {
        level: MazeLevel::challenge(),
        autoplay,
    })
    .add_plugins(MazeRenderPlugin);
    if let Some(path) = shot {
        app.insert_resource(ShotPath(path));
    }
    if exit_on_win {
        app.add_systems(Update, after_win);
    }
    app.run();
}

/// After the win: let the player settle, take the screenshot if asked, then close.
fn after_win(
    mut commands: Commands,
    run: Res<MazeRun>,
    time: Res<Time>,
    shot: Option<Res<ShotPath>>,
    mut since: Local<Option<f32>>,
    mut taken: Local<bool>,
    mut exit: MessageWriter<AppExit>,
) {
    if !run.won {
        return;
    }
    let elapsed = time.elapsed_secs() - *since.get_or_insert(time.elapsed_secs());
    if let Some(path) = shot.filter(|_| !*taken && elapsed > 0.6) {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.0.clone()));
        *taken = true;
    }
    if elapsed > 1.6 {
        exit.write(AppExit::Success);
    }
}

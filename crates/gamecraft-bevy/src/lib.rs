//! `gamecraft-bevy`: the games of this repo on Bevy 0.20, the third runtime next to
//! Godot (`depot/games`) and three.js (`web/games`).
//!
//! Each game is two plugins:
//!
//! - a **logic** plugin (components, resources, fixed-timestep systems, win
//!   check) that runs with `MinimalPlugins` and no renderer, so `cargo test`
//!   plays the game to its win headless and deterministically;
//! - a **render** plugin (meshes, camera, light, keyboard), compiled only with the
//!   `render` feature, used by the playable binary.
//!
//! Levels come from `gamecraft-sim`, the same generators and solvers the Godot
//! forge uses, so a Bevy game and its depot twin are the same level.
//!
//! How Bevy is organised, briefly: an *entity* is an id; *components* are plain
//! structs attached to entities (`GridPos`, `Player`); *resources* are singletons
//! (`MazeLevel`, `MazeRun`); *systems* are ordinary functions whose parameters
//! (`Query<...>`, `Res<...>`) tell Bevy what data they read and write, and Bevy
//! schedules them. `FixedUpdate` runs at a fixed rate (64 Hz by default), which is
//! where game rules live; `Update` runs once per rendered frame.

// Bevy systems receive `Res<T>` / `Query<..>` by value; that is the API, not a smell.
#![allow(clippy::needless_pass_by_value)]

pub mod analysis;
#[cfg(feature = "audio")]
pub mod audio;
pub mod cues;
pub mod maze;
pub mod murmuration;
pub mod railway;
#[cfg(feature = "render")]
pub mod render;
#[cfg(feature = "render")]
pub mod render_murmuration;
#[cfg(feature = "render")]
pub mod render_railway;
pub mod synth;

//! `gamecraft-bevy-nekomimi`: Nekomimi-chan VRM viewer on Bevy 0.20 + bevy_vrm 0.5.0.
//!
//! ```text
//! cargo run -p gamecraft-bevy --features render --bin gamecraft-bevy-nekomimi -- --vrm D:/Dev/repos/avatar-mcp/models/Nekomimi-chan.vrm
//! ```
//!
//! VRM source stays in `avatar-mcp/models/` (17.5 MB, do not copy into git).
//! Phase 1: orbit + ground + path plumbing. Phase 2 wires `bevy_vrm::VrmPlugin`.

#![allow(clippy::needless_pass_by_value)]

use bevy::prelude::*;

/// CLI `--vrm <path>` holder.
#[derive(Resource, Debug, Clone)]
struct VrmPath(String);

/// Extract `--vrm <path>` without `unwrap`/`expect` (workspace lints deny both).
#[must_use]
fn vrm_from_args() -> String {
    let args: Vec<String> = std::env::args().collect();
    let mut out = String::from("../../avatar-mcp/models/Nekomimi-chan.vrm");
    let mut i = 0_usize;
    while i < args.len() {
        if args[i] == "--vrm" {
            if let Some(next) = args.get(i + 1) {
                out.clone_from(next);
            }
            break;
        }
        i += 1;
    }
    out
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "GameCraft Nekomimi (Bevy VRM)".into(),
            ..default()
        }),
        ..default()
    }))
    .insert_resource(VrmPath(vrm_from_args()))
    .add_systems(Startup, setup);
    // Phase 2: .add_plugins(bevy_vrm::VrmPlugin) once API pinned against 0.20.1.
    app.run();
}

/// Camera, light, ground. VRM spawn lands here in phase 2.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    vrm_path: Res<VrmPath>,
) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 1.5, 3.0)
            .looking_at(Vec3::new(0.0, 1.1, 0.0), Vec3::Y),
        ..default()
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 4000.0,
            ..default()
        },
        transform: Transform::from_xyz(2.0, 4.0, 3.0),
        ..default()
    });
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(10.0, 10.0)),
        material: mats.add(Color::srgb(0.29, 0.20, 0.15)),
        ..default()
    });
    info!("VRM path: {}", vrm_path.0);
}

//! Maze rendering and input (feature `render`).
//!
//! Adds meshes to the entities the logic plugin spawned (walls, player, goal),
//! plus floor, camera and light. Grid y maps to world +z, so the camera looks
//! down from +z and "up" on the keyboard means y - 1. The player glides between
//! cells; the rules still move it one whole cell per fixed tick.

use crate::maze::{spawn_level, Autoplay, Goal, GridPos, Intent, MazeLevel, MazeRun, Player, Wall};
use bevy::prelude::*;

/// Meshes, camera, light, keyboard and the solved title for the maze.
#[derive(Debug, Clone, Copy, Default)]
pub struct MazeRenderPlugin;

impl Plugin for MazeRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.10)))
            .insert_resource(GlobalAmbientLight {
                color: Color::WHITE,
                brightness: 350.0,
                ..default()
            })
            .add_systems(Startup, dress.after(spawn_level))
            .add_systems(Update, (keyboard, glide, celebrate));
    }
}

/// Grid cell to world position, centred on the origin.
fn cell(level: &MazeLevel, x: i32, y: i32, lift: f32) -> Vec3 {
    let f = |v: i32| f32::from(i16::try_from(v).unwrap_or(0));
    Vec3::new(
        f(x) - f(level.width - 1) / 2.0,
        lift,
        f(y) - f(level.height - 1) / 2.0,
    )
}

#[allow(clippy::type_complexity)]
fn dress(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    level: Res<MazeLevel>,
    walls: Query<(Entity, &GridPos), With<Wall>>,
    player: Query<(Entity, &GridPos), With<Player>>,
    goal: Query<(Entity, &GridPos), With<Goal>>,
) {
    let wall_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.44, 0.52),
        perceptual_roughness: 0.9,
        ..default()
    });
    for (entity, p) in &walls {
        commands.entity(entity).insert((
            Mesh3d(wall_mesh.clone()),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_translation(cell(&level, p.0, p.1, 0.5)),
        ));
    }
    for (entity, p) in &goal {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Cuboid::new(0.8, 0.1, 0.8))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.75, 0.2),
                emissive: LinearRgba::rgb(2.0, 1.4, 0.3),
                ..default()
            })),
            Transform::from_translation(cell(&level, p.0, p.1, 0.05)),
        ));
    }
    for (entity, p) in &player {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Sphere::new(0.35))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.25, 0.55, 1.0),
                perceptual_roughness: 0.4,
                ..default()
            })),
            Transform::from_translation(cell(&level, p.0, p.1, 0.35)),
        ));
    }
    let w = f32::from(i16::try_from(level.width).unwrap_or(1));
    let h = f32::from(i16::try_from(level.height).unwrap_or(1));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(w, 0.2, h))),
        MeshMaterial3d(materials.add(Color::srgb(0.16, 0.18, 0.22))),
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));
    let span = w.max(h);
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, span * 1.25, span * 0.74).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 9_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-4.0, 10.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Arrow keys or WASD request one step; ignored while autoplay drives.
fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut intent: ResMut<Intent>,
    auto: Option<Res<Autoplay>>,
) {
    if auto.is_some() || intent.0.is_some() {
        return;
    }
    let pressed = |a: KeyCode, b: KeyCode| keys.just_pressed(a) || keys.just_pressed(b);
    let step = if pressed(KeyCode::ArrowUp, KeyCode::KeyW) {
        Some((0, -1))
    } else if pressed(KeyCode::ArrowDown, KeyCode::KeyS) {
        Some((0, 1))
    } else if pressed(KeyCode::ArrowLeft, KeyCode::KeyA) {
        Some((-1, 0))
    } else if pressed(KeyCode::ArrowRight, KeyCode::KeyD) {
        Some((1, 0))
    } else {
        None
    };
    if step.is_some() {
        intent.0 = step;
    }
}

/// Ease the player's transform toward its grid cell (frame-rate independent).
fn glide(
    time: Res<Time>,
    level: Res<MazeLevel>,
    mut player: Query<(&GridPos, &mut Transform), With<Player>>,
) {
    let k = 1.0 - (-18.0 * time.delta_secs()).exp();
    for (p, mut t) in &mut player {
        let target = cell(&level, p.0, p.1, 0.35);
        t.translation = t.translation.lerp(target, k);
    }
}

/// Put the result in the window title once the goal is reached.
fn celebrate(run: Res<MazeRun>, mut windows: Query<&mut Window>) {
    if run.is_changed() && run.won {
        for mut window in &mut windows {
            window.title = format!("GameCraft Maze (Bevy): solved in {} moves", run.moves);
        }
    }
}

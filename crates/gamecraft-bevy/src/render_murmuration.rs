//! Murmuration rendering and input (feature `render`).
//!
//! The sim keeps the flock in plain arrays; this plugin spawns one entity per boid and
//! copies position and heading into its `Transform` every frame. All boids of a colour
//! share one mesh and one unlit material, so Bevy batches them into a handful of
//! instanced draw calls. Sim (x, y) maps to world (x, z), centred on the origin, so the
//! camera looks down from +z like the maze.

use crate::murmuration::{FlockAutoplay, FlockWorld, Steer};
use bevy::prelude::*;
use gamecraft_sim::flock::{ARENA, FLARE_COOLDOWN, FLARE_RADIUS, ROOST};
use std::f32::consts::FRAC_PI_2;

const HALF: f32 = ARENA / 2.0;
const FLARE_RING_STEPS: u32 = 45;
const PILLAR_HEIGHT: f32 = 22.0;
/// Camera distance behind the flock in follow mode.
const FOLLOW_DISTANCE: f32 = 150.0;

/// Meshes, camera, light, keyboard and a live readout in the window title.
#[derive(Debug, Clone, Copy)]
pub struct MurmurationRenderPlugin {
    /// Start with the camera following the flock (C toggles to the whole arena).
    pub follow: bool,
}

impl Default for MurmurationRenderPlugin {
    fn default() -> Self {
        Self { follow: true }
    }
}

/// Camera mode: close behind the flock, or the whole arena.
#[derive(Resource, Debug, Clone, Copy)]
struct CamMode {
    follow: bool,
}

/// The one camera.
#[derive(Component, Debug)]
struct MainCam;

impl Plugin for MurmurationRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CamMode {
            follow: self.follow,
        })
        .insert_resource(ClearColor(Color::srgb(0.04, 0.035, 0.10)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.8, 0.8, 1.0),
            brightness: 420.0,
            ..default()
        })
        .add_systems(Startup, dress)
        .add_systems(
            Update,
            (
                keyboard, sync_boids, sync_hawks, sync_lure, sync_flare, camera_rig, hud,
            ),
        );
    }
}

/// Which boid of the sim this entity shows, plus its fixed flying height.
#[derive(Component, Debug)]
struct BoidSlot {
    index: usize,
    height: f32,
}

/// Which hawk of the sim this entity shows.
#[derive(Component, Debug)]
struct HawkSlot(usize);

/// The lure the player steers.
#[derive(Component, Debug)]
struct LureMarker;

/// The expanding ring a flare draws.
#[derive(Component, Debug)]
struct FlareRing;

fn to_world(x: f32, y: f32, height: f32) -> Vec3 {
    Vec3::new(x - HALF, height, y - HALF)
}

fn follow_offset() -> Vec3 {
    Vec3::new(0.0, FOLLOW_DISTANCE * 1.25, FOLLOW_DISTANCE * 0.74)
}

fn glow(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        cull_mode: None,
        ..default()
    }
}

fn dress(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<FlockWorld>,
) {
    spawn_flock(&mut commands, &mut meshes, &mut materials, &world);
    spawn_scenery(&mut commands, &mut meshes, &mut materials, &world);
    let (cx, cy) = world.0.centroid();
    let start = to_world(cx, cy, 0.0);
    commands.spawn((
        MainCam,
        Camera3d::default(),
        Transform::from_translation(start + follow_offset()).looking_at(start, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 5_000.0,
            ..default()
        },
        Transform::from_xyz(-120.0, 300.0, 160.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn spawn_flock(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    world: &FlockWorld,
) {
    let dash = meshes.add(Cuboid::new(2.6, 0.5, 0.9));
    let tints = [
        materials.add(glow(Color::srgb(1.0, 0.92, 0.80))),
        materials.add(glow(Color::srgb(0.95, 0.80, 0.55))),
        materials.add(glow(Color::srgb(0.75, 0.85, 1.0))),
    ];
    let n = world.0.px.len();
    for i in 0..n {
        let band = u8::try_from(i % 7).unwrap_or(0);
        commands.spawn((
            BoidSlot {
                index: i,
                height: 1.5 + f32::from(band) * 0.35,
            },
            Mesh3d(dash.clone()),
            MeshMaterial3d(tints[i % tints.len()].clone()),
            Transform::from_translation(to_world(world.0.px[i], world.0.py[i], 1.5)),
        ));
    }
}

fn spawn_scenery(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    world: &FlockWorld,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(ARENA, 0.2, ARENA))),
        MeshMaterial3d(materials.add(Color::srgb(0.05, 0.06, 0.11))),
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));
    let flat = Quat::from_rotation_x(-FRAC_PI_2);
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(ROOST.r))),
        MeshMaterial3d(materials.add(glow(Color::srgb(0.22, 0.17, 0.07)))),
        Transform::from_translation(to_world(ROOST.x, ROOST.y, 0.08)).with_rotation(flat),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Annulus::new(ROOST.r - 1.2, ROOST.r))),
        MeshMaterial3d(materials.add(glow(Color::srgb(1.0, 0.75, 0.25)))),
        Transform::from_translation(to_world(ROOST.x, ROOST.y, 0.12)).with_rotation(flat),
    ));
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.40, 0.50),
        perceptual_roughness: 0.9,
        ..default()
    });
    for p in &world.0.pillars {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(p.r, PILLAR_HEIGHT))),
            MeshMaterial3d(stone.clone()),
            Transform::from_translation(to_world(p.x, p.y, PILLAR_HEIGHT / 2.0)),
        ));
    }
    let hawk = meshes.add(Cuboid::new(6.0, 1.2, 2.2));
    let hawk_mat = materials.add(glow(Color::srgb(0.95, 0.12, 0.10)));
    for k in 0..world.0.hawks.len() {
        commands.spawn((
            HawkSlot(k),
            Mesh3d(hawk.clone()),
            MeshMaterial3d(hawk_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, 4.0, 0.0)),
        ));
    }
    commands.spawn((
        LureMarker,
        Mesh3d(meshes.add(Sphere::new(2.2))),
        MeshMaterial3d(materials.add(glow(Color::srgb(0.3, 1.0, 0.9)))),
        Transform::from_translation(Vec3::new(0.0, 3.0, 0.0)),
    ));
    commands.spawn((
        FlareRing,
        Mesh3d(meshes.add(Annulus::new(0.96, 1.0))),
        MeshMaterial3d(materials.add(glow(Color::srgb(0.6, 1.0, 1.0)))),
        Transform::from_translation(Vec3::new(0.0, 0.6, 0.0))
            .with_rotation(flat)
            .with_scale(Vec3::ZERO),
    ));
}

/// Arrow keys or WASD steer the lure while held; Space fires a flare. Ignored while the
/// scripted pilot flies.
fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<FlockWorld>,
    mut steer: ResMut<Steer>,
    mut mode: ResMut<CamMode>,
    auto: Option<Res<FlockAutoplay>>,
) {
    if keys.just_pressed(KeyCode::KeyC) {
        mode.follow = !mode.follow;
    }
    if auto.is_some() {
        return;
    }
    let held = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
    let mut dir = Vec2::ZERO;
    if held(KeyCode::ArrowUp, KeyCode::KeyW) {
        dir.y -= 1.0;
    }
    if held(KeyCode::ArrowDown, KeyCode::KeyS) {
        dir.y += 1.0;
    }
    if held(KeyCode::ArrowLeft, KeyCode::KeyA) {
        dir.x -= 1.0;
    }
    if held(KeyCode::ArrowRight, KeyCode::KeyD) {
        dir.x += 1.0;
    }
    let (lx, ly) = world.0.lure;
    steer.target = if dir == Vec2::ZERO {
        None
    } else {
        let d = dir.normalize() * 60.0;
        Some((lx + d.x, ly + d.y))
    };
    if keys.just_pressed(KeyCode::Space) {
        steer.flare = true;
    }
}

/// Copy boid positions and headings into transforms; caught boids vanish.
fn sync_boids(world: Res<FlockWorld>, mut boids: Query<(&BoidSlot, &mut Transform)>) {
    let f = &world.0;
    for (slot, mut t) in &mut boids {
        let i = slot.index;
        if !f.alive[i] {
            t.scale = Vec3::ZERO;
            continue;
        }
        t.translation = to_world(f.px[i], f.py[i], slot.height);
        t.rotation = Quat::from_rotation_y((-f.vy[i]).atan2(f.vx[i]));
        t.scale = Vec3::ONE;
    }
}

fn sync_hawks(world: Res<FlockWorld>, mut hawks: Query<(&HawkSlot, &mut Transform)>) {
    for (slot, mut t) in &mut hawks {
        if let Some(h) = world.0.hawks.get(slot.0) {
            t.translation = to_world(h.x, h.y, 4.0);
            t.rotation = Quat::from_rotation_y((-h.vy).atan2(h.vx));
        }
    }
}

fn sync_lure(world: Res<FlockWorld>, mut lure: Query<&mut Transform, With<LureMarker>>) {
    for mut t in &mut lure {
        t.translation = to_world(world.0.lure.0, world.0.lure.1, 3.0);
    }
}

/// The flare ring grows from the lure out to the flare radius, then disappears.
fn sync_flare(world: Res<FlockWorld>, mut ring: Query<&mut Transform, With<FlareRing>>) {
    let age = world.0.flare_age;
    for mut t in &mut ring {
        t.translation = to_world(world.0.lure.0, world.0.lure.1, 0.6);
        t.scale = if age < FLARE_RING_STEPS {
            let k = f32::from(u8::try_from(age).unwrap_or(u8::MAX))
                / f32::from(u8::try_from(FLARE_RING_STEPS).unwrap_or(1));
            Vec3::splat(FLARE_RADIUS * k.max(0.05))
        } else {
            Vec3::ZERO
        };
    }
}

/// Follow mode glides above and behind the flock; overview frames the whole arena.
fn camera_rig(
    time: Res<Time>,
    world: Res<FlockWorld>,
    mode: Res<CamMode>,
    mut cam: Query<&mut Transform, With<MainCam>>,
) {
    let (focus, offset) = if mode.follow {
        let (cx, cy) = world.0.centroid();
        (to_world(cx, cy, 0.0), follow_offset())
    } else {
        (Vec3::ZERO, Vec3::new(0.0, ARENA * 1.25, ARENA * 0.74))
    };
    let k = 1.0 - (-2.5 * time.delta_secs()).exp();
    for mut t in &mut cam {
        t.translation = t.translation.lerp(focus + offset, k);
        let look = t.translation - offset;
        t.look_at(look, Vec3::Y);
    }
}

/// A live readout in the window title (no UI feature needed).
fn hud(
    world: Res<FlockWorld>,
    time: Res<Time>,
    mut windows: Query<&mut Window>,
    mut last: Local<u32>,
    mut fps: Local<f32>,
) {
    let dt = time.delta_secs().max(1e-4);
    *fps = if *fps == 0.0 {
        1.0 / dt
    } else {
        *fps * 0.95 + (1.0 / dt) * 0.05
    };
    let s = world.0.stats();
    if s.tick / 15 == *last && !(s.won || s.lost) {
        return;
    }
    *last = s.tick / 15;
    let flare = if world.0.flare_cooldown == 0 {
        "ready".to_string()
    } else {
        format!(
            "{}s",
            world.0.flare_cooldown.div_ceil(60).min(FLARE_COOLDOWN)
        )
    };
    let state = if s.won {
        " | SETTLED"
    } else if s.lost {
        " | LOST"
    } else {
        ""
    };
    for mut window in &mut windows {
        window.title = format!(
            "Murmuration (Bevy) | roost {}/{} | flock {} | eaten {} | flare {flare} | t {} | {:.0} fps{state}",
            s.in_roost, s.needed, s.alive, s.eaten, s.tick, *fps
        );
    }
}

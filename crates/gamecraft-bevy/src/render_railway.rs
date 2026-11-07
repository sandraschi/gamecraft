//! Toy railway rendering and input (feature `render`).
//!
//! The table is built once from the layout: ballast, sleepers and rails sampled along
//! every block, viaduct piers wherever the line is raised, island platforms with
//! canopies, signals, departure boards and some scenery. Each car is a parent entity
//! whose transform follows `Rail::point_behind` every frame, so cars bend through the
//! helices on their own.
//!
//! One camera rides a single zoom value. At 1 it is a telephoto view over the whole
//! table with shallow depth of field, which reads as a miniature; at 0 it stands on a
//! platform at eye height with a normal lens. The model never changes, only the lens.

use crate::railway::{Clockwork, RailWorld};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::post_process::bloom::Bloom;
use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};
use bevy::prelude::*;
use gamecraft_sim::rail::{EdgeId, Rail, LEVEL_UP, LINE_Z, LOOP_OFFSET, P3, PLATFORM_HALF};
use std::f32::consts::{FRAC_PI_2, TAU};

/// Railway seconds in one turn of the light: a toy day.
pub const TOY_DAY_S: u32 = 720;
/// The first day starts at 07:00.
const DAY_START_S: u32 = 210;
/// Platform top above the track base.
const PLATFORM_TOP: f32 = 1.1;
/// Eye height above the platform.
const EYE: f32 = 1.7;
/// Rail head above the track base; cars stand here.
const RAIL_TOP: f32 = 0.58;
/// Width of an island platform.
const ISLAND_W: f32 = 2.8;

/// Meshes, camera, lights, keyboard and a live readout in the window title.
#[derive(Debug, Clone, Copy, Default)]
pub struct RailwayRenderPlugin;

impl Plugin for RailwayRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.55, 0.72, 0.92)))
            .insert_resource(GlobalAmbientLight {
                color: Color::WHITE,
                brightness: 400.0,
                ..default()
            })
            .init_resource::<CamRig>()
            .init_resource::<DayClock>()
            .add_systems(Startup, (build_table, spawn_trains, spawn_camera).chain())
            .add_systems(
                Update,
                (
                    keyboard,
                    place_trains,
                    signals,
                    boards,
                    daylight,
                    camera_rig.after(keyboard),
                    hud,
                ),
            );
    }
}

/// Where the camera is and where it is going.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct CamRig {
    /// 1 is the whole table, 0 is eye height on a platform.
    pub zoom: f32,
    /// Where `zoom` is easing to.
    pub zoom_target: f32,
    /// Orbit angle of the overview, radians.
    pub yaw: f32,
    /// Look-around on the platform, radians.
    pub look: f32,
    /// Index into the platform list (station order).
    pub platform: usize,
    /// Jump straight to the target this frame (for scripted shots).
    pub snap: bool,
}

impl Default for CamRig {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            zoom_target: 1.0,
            yaw: 0.35,
            look: 0.0,
            platform: 0,
            snap: false,
        }
    }
}

/// Shift of the light cycle against railway time (N flips day and night).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DayClock {
    /// Seconds added to railway time.
    pub offset: u32,
}

#[derive(Component, Debug)]
struct MainCam;

#[derive(Component, Debug)]
struct Sun;

#[derive(Component, Debug)]
struct StationLamp;

#[derive(Component, Debug)]
struct Headlamp;

/// One vehicle: which train, and its centre and length in metres behind the head.
#[derive(Component, Debug, Clone, Copy)]
struct TrainCar {
    train: usize,
    center: f32,
    length: f32,
}

#[derive(Component, Debug, Clone, Copy)]
struct SignalLamp(EdgeId);

/// One segment of one digit on a departure board.
#[derive(Component, Debug, Clone, Copy)]
struct BoardSeg {
    platform: EdgeId,
    slot: u8,
    seg: u8,
}

#[derive(Resource, Debug, Clone)]
struct Lamps {
    red: Handle<StandardMaterial>,
    green: Handle<StandardMaterial>,
    windows: Handle<StandardMaterial>,
}

fn v3(p: P3) -> Vec3 {
    Vec3::new(p.x, p.y, p.z)
}

fn fu(n: u32) -> f32 {
    f32::from(u16::try_from(n).unwrap_or(u16::MAX))
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Deterministic scatter for trees and people.
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f32::from(u16::try_from(self.0 >> 16).unwrap_or(0)) / 65_536.0
    }

    fn pick(&mut self, n: usize) -> usize {
        self.next();
        usize::try_from(self.0 >> 16).unwrap_or(0) % n.max(1)
    }
}

fn mat(color: Color, rough: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        perceptual_roughness: rough,
        ..default()
    }
}

fn glow(color: Color, strength: f32) -> StandardMaterial {
    let l = color.to_linear();
    StandardMaterial {
        base_color: color,
        emissive: LinearRgba::rgb(l.red * strength, l.green * strength, l.blue * strength),
        ..default()
    }
}

/// Platforms in station order: Talheim 1, Talheim 2, Bergblick 1, Bergblick 2.
fn platforms(rail: &Rail) -> Vec<EdgeId> {
    rail.layout
        .stations
        .iter()
        .flat_map(|s| s.platforms.iter().copied())
        .collect()
}

/// Level, side (+1 south, -1 north) and travel direction of a station.
fn station_frame(station: usize) -> (f32, f32) {
    if station == 0 {
        (0.0, 1.0)
    } else {
        (LEVEL_UP, -1.0)
    }
}

fn island_z(side: f32) -> f32 {
    side * (LINE_Z + LOOP_OFFSET / 2.0)
}

/// Camera poses for one platform: eye height, and high above the station.
struct PlatformView {
    eye: Vec3,
    eye_look: Vec3,
    high: Vec3,
    high_look: Vec3,
}

fn platform_view(rail: &Rail, edge: EdgeId) -> Option<PlatformView> {
    let e = rail.layout.edges.get(edge)?;
    let (level, side) = station_frame(e.station?);
    // Trains run east through Talheim (south) and west through Bergblick (north).
    let dir = side;
    let track = e.point_at(e.stop_at?);
    let iz = island_z(side);
    Some(PlatformView {
        // Near the main-track edge, out of line with the canopy posts.
        eye: Vec3::new(dir * 30.0, level + PLATFORM_TOP + EYE, iz - side * 0.9),
        eye_look: Vec3::new(-dir * 90.0, level + 2.0, track.z),
        high: Vec3::new(dir * 10.0, level + 55.0, iz + side * 95.0),
        high_look: Vec3::new(0.0, level, iz),
    })
}

/// Seconds into the toy day, with the fraction of the current second.
fn day_seconds(rail: &Rail, offset: u32) -> f32 {
    let secs = (rail.seconds() + offset + DAY_START_S) % TOY_DAY_S;
    let frac = fu(rail.tick % rail.tps) / fu(rail.tps.max(1));
    fu(secs) + frac
}

#[allow(clippy::too_many_lines)]
fn build_table(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<RailWorld>,
) {
    let rail = &world.0;
    // Room and table.
    let wood = materials.add(mat(Color::srgb(0.36, 0.22, 0.12), 0.7));
    let grass = materials.add(mat(Color::srgb(0.30, 0.47, 0.22), 0.95));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3000.0, 1.0, 3000.0))),
        MeshMaterial3d(materials.add(mat(Color::srgb(0.25, 0.17, 0.11), 0.8))),
        Transform::from_xyz(0.0, -31.0, 0.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(400.0, 1.0, 260.0))),
        MeshMaterial3d(grass.clone()),
        Transform::from_xyz(0.0, -0.52, 0.0),
    ));
    for (x, z, w, d) in [
        (0.0, 131.0, 404.0, 2.0),
        (0.0, -131.0, 404.0, 2.0),
        (201.0, 0.0, 2.0, 260.0),
        (-201.0, 0.0, 2.0, 260.0),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(w, 2.0, d))),
            MeshMaterial3d(wood.clone()),
            Transform::from_xyz(x, -0.6, z),
        ));
    }
    for (x, z) in [
        (185.0, 115.0),
        (-185.0, 115.0),
        (185.0, -115.0),
        (-185.0, -115.0),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(3.0, 30.0))),
            MeshMaterial3d(wood.clone()),
            Transform::from_xyz(x, -16.0, z),
        ));
    }
    // A low hill inside the loop.
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(grass.clone()),
        Transform::from_xyz(0.0, -4.0, 0.0).with_scale(Vec3::new(55.0, 10.0, 22.0)),
    ));
    spawn_track(&mut commands, &mut meshes, &mut materials, rail);
    spawn_stations(&mut commands, &mut meshes, &mut materials, rail);
    spawn_signals(&mut commands, &mut meshes, &mut materials, rail);
    spawn_trees(&mut commands, &mut meshes, &mut materials, rail);
}

fn spawn_track(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    rail: &Rail,
) {
    let ballast = materials.add(mat(Color::srgb(0.48, 0.45, 0.42), 1.0));
    let sleeper = materials.add(mat(Color::srgb(0.30, 0.22, 0.16), 0.9));
    let steel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.62, 0.66),
        metallic: 0.9,
        perceptual_roughness: 0.35,
        ..default()
    });
    let stone = materials.add(mat(Color::srgb(0.58, 0.55, 0.50), 0.9));
    let sleeper_mesh = meshes.add(Cuboid::new(2.6, 0.12, 0.25));
    let pier_mesh = meshes.add(Cylinder::new(0.55, 1.0));
    let step = 2.0_f32;
    for e in &rail.layout.edges {
        let len = e.length();
        let mut s = 0.0_f32;
        while s < len - 1e-3 {
            let s1 = (s + step).min(len);
            let (p0, p1) = (v3(e.point_at(s)), v3(e.point_at(s1)));
            let dir = p1 - p0;
            let seg = dir.length();
            if seg < 1e-3 {
                break;
            }
            let side = dir.cross(Vec3::Y).normalize_or_zero();
            let mid = (p0 + p1) / 2.0;
            let frame = Transform::from_translation(mid).looking_to(dir, Vec3::Y);
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(3.2, 0.3, seg + 0.05))),
                MeshMaterial3d(ballast.clone()),
                frame.with_translation(mid + Vec3::Y * 0.15),
            ));
            for k in [-0.72_f32, 0.72] {
                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.09, 0.16, seg + 0.02))),
                    MeshMaterial3d(steel.clone()),
                    frame.with_translation(mid + side * k + Vec3::Y * 0.5),
                ));
            }
            for frac in [0.0_f32, 0.33, 0.66] {
                let at = p0 + dir * frac;
                commands.spawn((
                    Mesh3d(sleeper_mesh.clone()),
                    MeshMaterial3d(sleeper.clone()),
                    frame.with_translation(at + Vec3::Y * 0.36),
                ));
            }
            // Viaduct piers under raised track, outside the Bergblick deck.
            let on_deck = mid.x.abs() < 72.0 && mid.z < -44.0;
            let pier_here = (s / 10.0).fract() < (step / 10.0) * 0.999;
            if mid.y > 0.8 && !on_deck && pier_here {
                commands.spawn((
                    Mesh3d(pier_mesh.clone()),
                    MeshMaterial3d(stone.clone()),
                    Transform::from_xyz(mid.x, mid.y / 2.0, mid.z)
                        .with_scale(Vec3::new(1.0, mid.y, 1.0)),
                ));
            }
            s = s1;
        }
    }
}

#[allow(clippy::too_many_lines)]
fn spawn_stations(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    rail: &Rail,
) {
    let concrete = materials.add(mat(Color::srgb(0.78, 0.76, 0.72), 0.9));
    let edge_line = materials.add(glow(Color::srgb(0.95, 0.80, 0.15), 0.2));
    let canopy = materials.add(mat(Color::srgb(0.20, 0.32, 0.30), 0.6));
    let walls = materials.add(mat(Color::srgb(0.93, 0.86, 0.70), 0.8));
    let roof = materials.add(mat(Color::srgb(0.62, 0.22, 0.16), 0.7));
    let deck = materials.add(mat(Color::srgb(0.25, 0.40, 0.19), 0.95));
    let board = materials.add(mat(Color::srgb(0.06, 0.07, 0.10), 0.5));
    let amber = materials.add(glow(Color::srgb(1.0, 0.62, 0.10), 6.0));
    let post = meshes.add(Cylinder::new(0.12, 1.0));
    let mut rng = Lcg(7);
    let coats = [
        Color::srgb(0.75, 0.20, 0.18),
        Color::srgb(0.18, 0.32, 0.65),
        Color::srgb(0.85, 0.72, 0.25),
        Color::srgb(0.25, 0.25, 0.28),
        Color::srgb(0.35, 0.55, 0.30),
    ];
    let coat_mats: Vec<Handle<StandardMaterial>> =
        coats.iter().map(|c| materials.add(mat(*c, 0.8))).collect();
    let person = meshes.add(Capsule3d::new(0.24, 1.2));
    for (si, st) in rail.layout.stations.iter().enumerate() {
        let (level, side) = station_frame(si);
        let iz = island_z(side);
        let top = level + PLATFORM_TOP;
        // Island: Bergblick's runs down to the ground so it stands on its deck.
        let base = if level > 0.0 { 0.0 } else { level };
        let h = top - base;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(PLATFORM_HALF * 2.0, h, ISLAND_W))),
            MeshMaterial3d(concrete.clone()),
            Transform::from_xyz(0.0, base + h / 2.0, iz),
        ));
        for edge_z in [iz - ISLAND_W / 2.0 + 0.2, iz + ISLAND_W / 2.0 - 0.2] {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(PLATFORM_HALF * 2.0, 0.02, 0.15))),
                MeshMaterial3d(edge_line.clone()),
                Transform::from_xyz(0.0, top + 0.01, edge_z),
            ));
        }
        if level > 0.0 {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(146.0, level - 0.02, 28.0))),
                MeshMaterial3d(deck.clone()),
                Transform::from_xyz(0.0, (level - 0.02) / 2.0, -59.0),
            ));
        }
        // Canopy on posts.
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(PLATFORM_HALF * 1.4, 0.25, 4.2))),
            MeshMaterial3d(canopy.clone()),
            Transform::from_xyz(0.0, top + 3.7, iz),
        ));
        let mut x = -PLATFORM_HALF * 0.65;
        while x <= PLATFORM_HALF * 0.65 + 0.1 {
            commands.spawn((
                Mesh3d(post.clone()),
                MeshMaterial3d(canopy.clone()),
                Transform::from_xyz(x, top + 1.85, iz).with_scale(Vec3::new(1.0, 3.7, 1.0)),
            ));
            x += 11.0;
        }
        for k in 0..3 {
            let lx = (fu(k) - 1.0) * 20.0;
            commands.spawn((
                StationLamp,
                PointLight {
                    color: Color::srgb(1.0, 0.85, 0.6),
                    intensity: 0.0,
                    range: 30.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(lx, top + 3.3, iz),
            ));
        }
        // Station building on the outer side.
        let bz = side * (LINE_Z + LOOP_OFFSET + 10.0);
        let bz = if si == 0 { side * (LINE_Z - 10.0) } else { bz };
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(34.0, 6.0, 8.0))),
            MeshMaterial3d(walls.clone()),
            Transform::from_xyz(0.0, level + 3.0, bz),
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(36.0, 0.8, 9.5))),
            MeshMaterial3d(roof.clone()),
            Transform::from_xyz(0.0, level + 6.4, bz),
        ));
        // Departure boards, one per track, facing the arrival end.
        for (slot_track, &platform) in st.platforms.iter().enumerate() {
            let dir = side;
            let bx = dir * 14.0;
            let by = top + 2.25 + 0.85 * fu(u32::try_from(slot_track).unwrap_or(0));
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.12, 0.78, 2.5))),
                MeshMaterial3d(board.clone()),
                Transform::from_xyz(bx, by, iz),
            ));
            let face = bx + dir * 0.075;
            let track_no = u8::try_from(slot_track + 1).unwrap_or(1);
            spawn_digits(
                commands,
                meshes,
                &amber,
                (face, by, iz, dir),
                platform,
                track_no,
            );
        }
        // A few people waiting.
        for _ in 0..9 {
            let px = (rng.next() - 0.5) * PLATFORM_HALF * 1.6;
            let pz = iz + (rng.next() - 0.5) * 1.2;
            // Keep the view from the eye point clear.
            if (5.0..34.0).contains(&(px * side)) {
                continue;
            }
            if let Some(m) = coat_mats.get(rng.pick(coat_mats.len())) {
                commands.spawn((
                    Mesh3d(person.clone()),
                    MeshMaterial3d(m.clone()),
                    Transform::from_xyz(px, top + 0.84, pz),
                ));
            }
        }
    }
}

/// Seven segments (a b c d e f g) per digit, for 0 to 9.
const SEGMENTS: [u8; 10] = [
    0b111_1110, 0b011_0000, 0b110_1101, 0b111_1001, 0b011_0011, 0b101_1011, 0b101_1111, 0b111_0000,
    0b111_1111, 0b111_1011,
];

/// Segment centre (u right, v up) and size, digit cell 0.32 by 0.56.
fn segment_box(seg: u8) -> (f32, f32, f32, f32) {
    let (w, h, t) = (0.32, 0.56, 0.06);
    match seg {
        0 => (0.0, h / 2.0, w, t),
        1 => (w / 2.0, h / 4.0, t, h / 2.0),
        2 => (w / 2.0, -h / 4.0, t, h / 2.0),
        3 => (0.0, -h / 2.0, w, t),
        4 => (-w / 2.0, -h / 4.0, t, h / 2.0),
        5 => (-w / 2.0, h / 4.0, t, h / 2.0),
        _ => (0.0, 0.0, w, t),
    }
}

/// Board digits: slot 0 is the track number, 1 to 4 are mm:ss.
fn spawn_digits(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    lit: &Handle<StandardMaterial>,
    at: (f32, f32, f32, f32),
    platform: EdgeId,
    track_no: u8,
) {
    let (face, by, iz, dir) = at;
    // Seen from the arrival end, "right" on the board is -dir along z.
    let slots = [-0.95_f32, -0.40, -0.02, 0.52, 0.90];
    for (slot, &u0) in slots.iter().enumerate() {
        let slot = u8::try_from(slot).unwrap_or(0);
        for seg in 0..7_u8 {
            let (u, v, su, sv) = segment_box(seg);
            let visible = slot == 0 && digit_has(track_no, seg);
            commands.spawn((
                BoardSeg {
                    platform,
                    slot,
                    seg,
                },
                Mesh3d(meshes.add(Cuboid::new(0.02, sv, su))),
                MeshMaterial3d(lit.clone()),
                Transform::from_xyz(face, by + v, iz - dir * (u0 + u)),
                if visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
            ));
        }
    }
    for dv in [0.12_f32, -0.12] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.02, 0.06, 0.06))),
            MeshMaterial3d(lit.clone()),
            Transform::from_xyz(face, by + dv, iz - dir * 0.25),
        ));
    }
}

fn digit_has(d: u8, seg: u8) -> bool {
    SEGMENTS
        .get(usize::from(d))
        .is_some_and(|bits| bits & (1 << (6 - seg.min(6))) != 0)
}

fn spawn_signals(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    rail: &Rail,
) {
    let red = materials.add(glow(Color::srgb(1.0, 0.08, 0.05), 8.0));
    let green = materials.add(glow(Color::srgb(0.1, 1.0, 0.35), 8.0));
    let windows = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.18, 0.22),
        perceptual_roughness: 0.2,
        emissive: LinearRgba::BLACK,
        ..default()
    });
    let mast = materials.add(mat(Color::srgb(0.15, 0.15, 0.17), 0.6));
    let mast_mesh = meshes.add(Cylinder::new(0.12, 4.0));
    let head_mesh = meshes.add(Cuboid::new(0.5, 0.9, 0.35));
    let lamp_mesh = meshes.add(Sphere::new(0.2));
    let edges = &rail.layout.edges;
    for (id, e) in edges.iter().enumerate() {
        let start = e.points.first().copied().map_or(Vec3::ZERO, v3);
        let siblings = edges
            .iter()
            .take(id)
            .filter(|o| {
                o.points
                    .first()
                    .is_some_and(|p| v3(*p).distance(start) < 0.1)
            })
            .count();
        let s = 4.0 + 10.0 * fu(u32::try_from(siblings).unwrap_or(0));
        let p = v3(e.point_at(s));
        let ahead = v3(e.point_at(s + 1.0)) - p;
        let left = -ahead.cross(Vec3::Y).normalize_or_zero();
        let base = p + left * 2.4;
        commands.spawn((
            Mesh3d(mast_mesh.clone()),
            MeshMaterial3d(mast.clone()),
            Transform::from_translation(base + Vec3::Y * 2.0),
        ));
        let facing = Transform::from_translation(base + Vec3::Y * 4.2).looking_to(-ahead, Vec3::Y);
        commands.spawn((
            Mesh3d(head_mesh.clone()),
            MeshMaterial3d(mast.clone()),
            facing,
        ));
        commands.spawn((
            SignalLamp(id),
            Mesh3d(lamp_mesh.clone()),
            MeshMaterial3d(red.clone()),
            Transform::from_translation(base + Vec3::Y * 4.2 - ahead.normalize_or_zero() * 0.2),
        ));
    }
    commands.insert_resource(Lamps {
        red,
        green,
        windows,
    });
}

fn spawn_trees(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    rail: &Rail,
) {
    let mut samples = Vec::new();
    for e in &rail.layout.edges {
        let mut s = 0.0;
        while s < e.length() {
            samples.push(v3(e.point_at(s)));
            s += 4.0;
        }
    }
    let trunk = materials.add(mat(Color::srgb(0.32, 0.22, 0.14), 0.9));
    let leaves = [
        materials.add(mat(Color::srgb(0.13, 0.33, 0.16), 0.9)),
        materials.add(mat(Color::srgb(0.20, 0.40, 0.18), 0.9)),
        materials.add(mat(Color::srgb(0.10, 0.27, 0.20), 0.9)),
    ];
    let trunk_mesh = meshes.add(Cylinder::new(0.3, 1.0));
    let cone = meshes.add(Cone {
        radius: 1.0,
        height: 1.0,
    });
    let mut rng = Lcg(42);
    let mut placed = 0_u32;
    for _ in 0..900 {
        if placed >= 170 {
            break;
        }
        let x = (rng.next() - 0.5) * 380.0;
        let z = (rng.next() - 0.5) * 240.0;
        let near_track = samples
            .iter()
            .any(|p| (p.x - x).powi(2) + (p.z - z).powi(2) < 100.0);
        let in_station = x.abs() < 75.0 && (z.abs() - 55.0).abs() < 22.0;
        if near_track || in_station {
            continue;
        }
        let hill = 1.0 - (x / 55.0).powi(2) - (z / 22.0).powi(2);
        let ground = if hill > 0.0 {
            (-4.0 + 10.0 * hill.sqrt()).max(0.0)
        } else {
            0.0
        };
        let size = 0.7 + rng.next() * 0.8;
        let tone = leaves
            .get(usize::from(placed.is_multiple_of(3)) + usize::from(placed.is_multiple_of(5)));
        commands.spawn((
            Mesh3d(trunk_mesh.clone()),
            MeshMaterial3d(trunk.clone()),
            Transform::from_xyz(x, ground + 1.0 * size, z).with_scale(Vec3::new(
                size,
                2.0 * size,
                size,
            )),
        ));
        if let Some(t) = tone {
            commands.spawn((
                Mesh3d(cone.clone()),
                MeshMaterial3d(t.clone()),
                Transform::from_xyz(x, ground + 4.6 * size, z).with_scale(Vec3::new(
                    2.4 * size,
                    6.0 * size,
                    2.4 * size,
                )),
            ));
        }
        placed += 1;
    }
}

/// What one vehicle looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Car {
    Tank,
    Coach,
    BulletNose,
    BulletMid,
    BulletTail,
}

fn consist(name: &str) -> Vec<(Car, f32)> {
    match name {
        "Sparky" => vec![(Car::Tank, 15.0), (Car::Coach, 14.0), (Car::Coach, 14.0)],
        "Bullet" => vec![
            (Car::BulletNose, 25.0),
            (Car::BulletMid, 24.0),
            (Car::BulletTail, 25.0),
        ],
        _ => vec![(Car::Coach, 19.0), (Car::Coach, 19.0), (Car::Coach, 19.0)],
    }
}

struct Paint {
    green: Handle<StandardMaterial>,
    black: Handle<StandardMaterial>,
    brass: Handle<StandardMaterial>,
    red: Handle<StandardMaterial>,
    maroon: Handle<StandardMaterial>,
    grey: Handle<StandardMaterial>,
    white: Handle<StandardMaterial>,
    blue: Handle<StandardMaterial>,
    glass: Handle<StandardMaterial>,
    windows: Handle<StandardMaterial>,
    lamp: Handle<StandardMaterial>,
}

#[allow(clippy::needless_pass_by_value)]
fn spawn_trains(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<RailWorld>,
    lamps: Option<Res<Lamps>>,
) {
    let windows = lamps.map_or_else(
        || materials.add(mat(Color::srgb(0.16, 0.18, 0.22), 0.2)),
        |l| l.windows.clone(),
    );
    let paint = Paint {
        green: materials.add(mat(Color::srgb(0.10, 0.35, 0.18), 0.45)),
        black: materials.add(mat(Color::srgb(0.06, 0.06, 0.07), 0.6)),
        brass: materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.65, 0.25),
            metallic: 0.9,
            perceptual_roughness: 0.3,
            ..default()
        }),
        red: materials.add(mat(Color::srgb(0.70, 0.08, 0.06), 0.5)),
        maroon: materials.add(mat(Color::srgb(0.42, 0.10, 0.12), 0.5)),
        grey: materials.add(mat(Color::srgb(0.45, 0.45, 0.48), 0.6)),
        white: materials.add(mat(Color::srgb(0.94, 0.95, 0.96), 0.25)),
        blue: materials.add(mat(Color::srgb(0.10, 0.25, 0.70), 0.35)),
        glass: materials.add(mat(Color::srgb(0.04, 0.05, 0.08), 0.1)),
        windows,
        lamp: materials.add(glow(Color::srgb(1.0, 0.95, 0.8), 10.0)),
    };
    for (train, plan) in world.0.plans.iter().enumerate() {
        let mut back = 0.0_f32;
        for (k, (car, length)) in consist(plan.class.name).into_iter().enumerate() {
            let parent = commands
                .spawn((
                    TrainCar {
                        train,
                        center: back + length / 2.0,
                        length,
                    },
                    Transform::default(),
                    Visibility::default(),
                ))
                .id();
            dress_car(&mut commands, &mut meshes, &paint, parent, car, length);
            if k == 0 {
                commands.spawn((
                    Headlamp,
                    SpotLight {
                        color: Color::srgb(1.0, 0.92, 0.75),
                        intensity: 0.0,
                        range: 160.0,
                        inner_angle: 0.18,
                        outer_angle: 0.42,
                        shadow_maps_enabled: false,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 1.6, -length / 2.0 - 0.5)
                        .looking_to(Vec3::new(0.0, -0.06, -1.0), Vec3::Y),
                    ChildOf(parent),
                ));
            }
            back += length + 1.0;
        }
    }
}

/// Add the meshes of one vehicle under `parent`. Local frame: forward is -z, the
/// origin sits on the rail head.
#[allow(clippy::too_many_lines)]
fn dress_car(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    paint: &Paint,
    parent: Entity,
    car: Car,
    length: f32,
) {
    let mut part = |mesh: Mesh, m: &Handle<StandardMaterial>, t: Transform| {
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(m.clone()),
            t,
            ChildOf(parent),
        ));
    };
    let half = length / 2.0;
    let along = Quat::from_rotation_x(FRAC_PI_2);
    for z in [-half + 2.5, half - 2.5] {
        part(
            Cuboid::new(2.3, 0.7, 2.6).into(),
            &paint.black,
            Transform::from_xyz(0.0, 0.35, z),
        );
    }
    match car {
        Car::Tank => {
            part(
                Cuboid::new(2.7, 0.6, length - 0.4).into(),
                &paint.black,
                Transform::from_xyz(0.0, 0.9, 0.0),
            );
            part(
                Cylinder::new(1.05, 9.5).into(),
                &paint.green,
                Transform::from_xyz(0.0, 2.35, -2.0).with_rotation(along),
            );
            part(
                Cylinder::new(1.08, 1.0).into(),
                &paint.black,
                Transform::from_xyz(0.0, 2.35, -6.6).with_rotation(along),
            );
            part(
                Cuboid::new(2.9, 1.5, 6.5).into(),
                &paint.green,
                Transform::from_xyz(0.0, 1.95, -1.6),
            );
            part(
                Cylinder::new(0.32, 1.5).into(),
                &paint.black,
                Transform::from_xyz(0.0, 3.9, -5.8),
            );
            part(
                Sphere::new(0.5).into(),
                &paint.brass,
                Transform::from_xyz(0.0, 3.4, -1.8),
            );
            part(
                Cuboid::new(2.9, 3.0, 4.0).into(),
                &paint.green,
                Transform::from_xyz(0.0, 2.7, 5.2),
            );
            part(
                Cuboid::new(3.1, 0.2, 4.5).into(),
                &paint.black,
                Transform::from_xyz(0.0, 4.3, 5.2),
            );
            part(
                Cuboid::new(2.95, 0.7, 3.0).into(),
                &paint.windows,
                Transform::from_xyz(0.0, 3.4, 5.2),
            );
            part(
                Cuboid::new(2.8, 0.5, 0.3).into(),
                &paint.red,
                Transform::from_xyz(0.0, 0.9, -half + 0.1),
            );
            part(
                Sphere::new(0.22).into(),
                &paint.lamp,
                Transform::from_xyz(0.0, 1.6, -half),
            );
        }
        Car::Coach => {
            part(
                Cuboid::new(2.8, 3.0, length - 0.4).into(),
                &paint.maroon,
                Transform::from_xyz(0.0, 2.25, 0.0),
            );
            part(
                Cuboid::new(2.6, 0.35, length - 0.6).into(),
                &paint.grey,
                Transform::from_xyz(0.0, 3.9, 0.0),
            );
            part(
                Cuboid::new(2.84, 0.8, length - 2.4).into(),
                &paint.windows,
                Transform::from_xyz(0.0, 2.7, 0.0),
            );
        }
        Car::BulletNose | Car::BulletMid | Car::BulletTail => {
            let nose = 7.5;
            let (body_len, body_z) = match car {
                Car::BulletNose => (length - nose, nose / 2.0),
                Car::BulletTail => (length - nose, -nose / 2.0),
                _ => (length - 0.4, 0.0),
            };
            part(
                Cuboid::new(3.2, 3.4, body_len).into(),
                &paint.white,
                Transform::from_xyz(0.0, 2.3, body_z),
            );
            part(
                Cuboid::new(3.24, 0.35, body_len).into(),
                &paint.blue,
                Transform::from_xyz(0.0, 1.35, body_z),
            );
            part(
                Cuboid::new(3.24, 0.7, body_len - 1.5).into(),
                &paint.windows,
                Transform::from_xyz(0.0, 2.75, body_z),
            );
            let ends: &[f32] = match car {
                Car::BulletNose => &[-1.0],
                Car::BulletTail => &[1.0],
                _ => &[],
            };
            for &end in ends {
                let z = end * (half - nose);
                part(
                    Sphere::new(1.0).into(),
                    &paint.white,
                    Transform::from_xyz(0.0, 1.85, z).with_scale(Vec3::new(1.6, 1.3, nose)),
                );
                part(
                    Sphere::new(1.0).into(),
                    &paint.glass,
                    Transform::from_xyz(0.0, 2.75, z + end * 2.4)
                        .with_scale(Vec3::new(1.05, 0.45, 2.4)),
                );
                part(
                    Sphere::new(0.2).into(),
                    &paint.lamp,
                    Transform::from_xyz(0.9, 1.4, end * (half - 0.6)),
                );
                part(
                    Sphere::new(0.2).into(),
                    &paint.lamp,
                    Transform::from_xyz(-0.9, 1.4, end * (half - 0.6)),
                );
            }
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        MainCam,
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 0.35,
            near: 0.1,
            far: 5000.0,
            ..default()
        }),
        Transform::from_xyz(0.0, 330.0, 280.0).looking_at(Vec3::ZERO, Vec3::Y),
        DepthOfField {
            mode: DepthOfFieldMode::Gaussian,
            focal_distance: 430.0,
            sensor_height: 0.3,
            aperture_f_stops: 1.0,
            max_circle_of_confusion_diameter: 40.0,
            max_depth: 4000.0,
        },
        Bloom::NATURAL,
        DistanceFog {
            color: Color::srgb(0.55, 0.72, 0.92),
            directional_light_color: Color::srgba(1.0, 0.92, 0.75, 0.4),
            directional_light_exponent: 24.0,
            falloff: FogFalloff::Linear {
                start: 700.0,
                end: 3000.0,
            },
        },
    ));
    commands.spawn((
        Sun,
        DirectionalLight {
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-200.0, 300.0, 150.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Arrows zoom and orbit, Tab picks a platform, F follows the next train in, +/- speed,
/// Space pauses, N flips day and night, Home goes back to the whole table.
fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    world: Res<RailWorld>,
    mut rig: ResMut<CamRig>,
    mut clock: ResMut<Clockwork>,
    mut day: ResMut<DayClock>,
) {
    let dt = time.delta_secs();
    let held = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
    if held(KeyCode::ArrowUp, KeyCode::KeyW) {
        rig.zoom_target = (rig.zoom_target - 0.35 * dt).clamp(0.0, 1.0);
    }
    if held(KeyCode::ArrowDown, KeyCode::KeyS) {
        rig.zoom_target = (rig.zoom_target + 0.35 * dt).clamp(0.0, 1.0);
    }
    let turn = f32::from(i8::from(held(KeyCode::ArrowRight, KeyCode::KeyD)))
        - f32::from(i8::from(held(KeyCode::ArrowLeft, KeyCode::KeyA)));
    if rig.zoom > 0.6 {
        rig.yaw += turn * 0.8 * dt;
    } else {
        rig.look = (rig.look + turn * 0.9 * dt).clamp(-2.6, 2.6);
    }
    let count = platforms(&world.0).len().max(1);
    if keys.just_pressed(KeyCode::Tab) {
        rig.platform = (rig.platform + 1) % count;
        rig.look = 0.0;
    }
    if keys.just_pressed(KeyCode::KeyF) {
        let plats = platforms(&world.0);
        let soonest = plats
            .iter()
            .enumerate()
            .filter_map(|(k, &e)| world.0.next_arrival(e).map(|(_, due)| (k, due)))
            .filter(|&(_, due)| due > 0)
            .min_by_key(|&(_, due)| due);
        if let Some((k, _)) = soonest {
            rig.platform = k;
            rig.zoom_target = 0.0;
            rig.look = 0.0;
        }
    }
    if keys.just_pressed(KeyCode::Home) {
        rig.zoom_target = 1.0;
    }
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        clock.speed = (clock.speed * 2).min(64);
    }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        clock.speed = (clock.speed / 2).max(1);
    }
    if keys.just_pressed(KeyCode::Space) {
        clock.paused = !clock.paused;
    }
    if keys.just_pressed(KeyCode::KeyN) {
        day.offset = (day.offset + TOY_DAY_S / 2) % TOY_DAY_S;
    }
}

fn place_trains(world: Res<RailWorld>, mut cars: Query<(&TrainCar, &mut Transform)>) {
    let rail = &world.0;
    for (car, mut t) in &mut cars {
        let reach = car.length * 0.4;
        let (Some(front), Some(back)) = (
            rail.point_behind(car.train, car.center - reach),
            rail.point_behind(car.train, car.center + reach),
        ) else {
            continue;
        };
        let (f, b) = (v3(front), v3(back));
        let dir = f - b;
        if dir.length_squared() < 1e-6 {
            continue;
        }
        *t = Transform::from_translation((f + b) / 2.0 + Vec3::Y * RAIL_TOP)
            .looking_to(dir, Vec3::Y);
    }
}

fn signals(
    world: Res<RailWorld>,
    lamps: Option<Res<Lamps>>,
    mut q: Query<(&SignalLamp, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let Some(lamps) = lamps else {
        return;
    };
    for (lamp, mut m) in &mut q {
        let want = if world.0.signal_clear(lamp.0) {
            &lamps.green
        } else {
            &lamps.red
        };
        if m.0 != *want {
            m.0 = want.clone();
        }
    }
}

fn boards(world: Res<RailWorld>, mut q: Query<(&BoardSeg, &mut Visibility)>) {
    let rail = &world.0;
    let tps = i64::from(rail.tps.max(1));
    for (seg, mut vis) in &mut q {
        if seg.slot == 0 {
            continue;
        }
        let shown = match rail.next_arrival(seg.platform) {
            Some((_, due)) => {
                let secs = (due.max(0) + tps - 1) / tps;
                let (mm, ss) = ((secs / 60).min(99), secs % 60);
                let digit = match seg.slot {
                    1 => mm / 10,
                    2 => mm % 10,
                    3 => ss / 10,
                    _ => ss % 10,
                };
                digit_has(u8::try_from(digit).unwrap_or(0), seg.seg)
            }
            None => seg.seg == 6,
        };
        let want = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn daylight(
    world: Res<RailWorld>,
    day_clock: Res<DayClock>,
    lamps: Option<Res<Lamps>>,
    mut sun: Query<(&mut DirectionalLight, &mut Transform), With<Sun>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut clear: ResMut<ClearColor>,
    mut fog: Query<&mut DistanceFog>,
    mut lights: ParamSet<(
        Query<&mut PointLight, With<StationLamp>>,
        Query<&mut SpotLight, With<Headlamp>>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let phase = day_seconds(&world.0, day_clock.offset) / fu(TOY_DAY_S);
    let elev = (TAU * (phase - 0.25)).sin();
    let day = smooth((elev + 0.12) / 0.45);
    let night = 1.0 - day;
    let az = TAU * phase;
    let e = elev.max(0.08);
    for (mut light, mut t) in &mut sun {
        // Sun by day, a pale moon by night.
        light.illuminance = 11_000.0 * day + 900.0 * night;
        light.color = Color::srgb(
            lerp(0.55, 1.0, day),
            lerp(0.62, 0.97, day),
            lerp(0.95, 0.92, day),
        );
        *t = Transform::from_xyz(az.cos() * 400.0, e * 500.0, az.sin() * 400.0)
            .looking_at(Vec3::ZERO, Vec3::Y);
    }
    ambient.brightness = 160.0 + 430.0 * day;
    ambient.color = Color::srgb(lerp(0.45, 1.0, day), lerp(0.55, 1.0, day), 1.0);
    let dusk = (-(elev / 0.14).powi(2)).exp() * 0.65;
    let sky = |n: f32, d: f32, k: f32| lerp(lerp(n, d, day), k, dusk);
    let sky = Color::srgb(
        sky(0.02, 0.55, 0.92),
        sky(0.03, 0.72, 0.50),
        sky(0.08, 0.92, 0.30),
    );
    clear.0 = sky;
    for mut f in &mut fog {
        f.color = sky;
    }
    for mut l in &mut lights.p0() {
        l.intensity = 60_000.0 * night;
    }
    for mut l in &mut lights.p1() {
        l.intensity = 900_000.0 * night;
    }
    if let Some(h) = lamps.as_ref().map(|l| l.windows.clone()) {
        if let Some(mut m) = materials.get_mut(&h) {
            m.emissive = LinearRgba::rgb(2.6 * night, 1.9 * night, 0.9 * night);
        }
    }
}

#[allow(clippy::type_complexity)]
fn camera_rig(
    time: Res<Time>,
    world: Res<RailWorld>,
    mut rig: ResMut<CamRig>,
    mut cam: Query<(&mut Transform, &mut Projection, &mut DepthOfField), With<MainCam>>,
) {
    let ease = if rig.snap {
        1.0
    } else {
        1.0 - (-2.2 * time.delta_secs()).exp()
    };
    rig.zoom += (rig.zoom_target - rig.zoom) * ease;
    rig.snap = false;
    let plats = platforms(&world.0);
    let edge = plats
        .get(rig.platform % plats.len().max(1))
        .copied()
        .unwrap_or(0);
    let Some(pv) = platform_view(&world.0, edge) else {
        return;
    };
    let (dist, pitch) = (430.0_f32, 0.9_f32);
    let over_look = Vec3::new(0.0, 3.0, 0.0);
    let over = over_look
        + Vec3::new(
            dist * pitch.cos() * rig.yaw.sin(),
            dist * pitch.sin(),
            dist * pitch.cos() * rig.yaw.cos(),
        );
    let eye_look = pv.eye + Quat::from_rotation_y(rig.look) * (pv.eye_look - pv.eye);
    // (position, look at, fov, sensor height, f-stops) at eye, high and overview.
    let poses = [
        (pv.eye, eye_look, 1.05_f32, 0.024_f32, 5.6_f32),
        (pv.high, pv.high_look, 0.62, 0.8, 1.4),
        (over, over_look, 0.34, 6.0, 1.0),
    ];
    let zoom = rig.zoom.clamp(0.0, 1.0);
    let (from, to, blend) = if zoom < 0.5 {
        (poses[0], poses[1], smooth(zoom * 2.0))
    } else {
        (poses[1], poses[2], smooth((zoom - 0.5) * 2.0))
    };
    let pos = from.0.lerp(to.0, blend);
    let look = from.1.lerp(to.1, blend);
    for (mut frame, mut proj, mut dof) in &mut cam {
        *frame = Transform::from_translation(pos).looking_at(look, Vec3::Y);
        if let Projection::Perspective(persp) = &mut *proj {
            persp.fov = lerp(from.2, to.2, blend);
        }
        dof.sensor_height = lerp(from.3, to.3, blend);
        dof.aperture_f_stops = lerp(from.4, to.4, blend);
        dof.focal_distance = pos.distance(look).max(5.0);
    }
}

fn hud(
    world: Res<RailWorld>,
    clock: Res<Clockwork>,
    day_clock: Res<DayClock>,
    rig: Res<CamRig>,
    time: Res<Time>,
    mut windows: Query<&mut Window>,
    mut fps: Local<f32>,
) {
    let rail = &world.0;
    let dt = time.delta_secs().max(1e-4);
    *fps = if *fps <= 0.0 {
        1.0 / dt
    } else {
        *fps * 0.95 + (1.0 / dt) * 0.05
    };
    let minutes = (rail.seconds() + day_clock.offset + DAY_START_S) % TOY_DAY_S * 2;
    let plats = platforms(rail);
    let edge = plats
        .get(rig.platform % plats.len().max(1))
        .copied()
        .unwrap_or(0);
    let name = rail.layout.edges.get(edge).map_or("?", |e| e.name.as_str());
    let next = match rail.next_arrival(edge) {
        Some((i, due)) => {
            let who = rail.plans.get(i).map_or("?", |p| p.class.name);
            let secs = due.max(0) / i64::from(rail.tps.max(1));
            format!("{who} in {}:{:02}", secs / 60, secs % 60)
        }
        None => "no train due".to_string(),
    };
    let s = rail.stats;
    let state = if clock.paused { " | PAUSED" } else { "" };
    for mut window in &mut windows {
        window.title = format!(
            "Toy railway (Bevy) | {:02}:{:02} | x{} | {name}: {next} | held {} | collisions {} | {:.0} fps{state}",
            minutes / 60,
            minutes % 60,
            clock.speed,
            s.holds,
            s.collisions,
            *fps
        );
    }
}

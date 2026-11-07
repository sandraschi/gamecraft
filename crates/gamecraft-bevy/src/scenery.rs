//! Scenery for the toy railway (feature `render`): the landscape mesh, trees, a
//! village with a church, a vineyard on the hill, fields, and cars on the main street.
//!
//! Everything is generated from the layout and a fixed seed, so every run builds the
//! same table. Shapes are deliberately low-poly, the way model scenery is: the camera's
//! depth of field and the light do the rest.

use crate::railway::Clockwork;
use crate::terrain::{
    natural, roll, water_distance, Terrain, BRIDGE_X, LINE_Z_LOWER, TABLE_X, TABLE_Z, VILLAGE,
    WATER_Y,
};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

/// Grid spacing of the landscape mesh, metres.
const STEP: f32 = 2.0;
/// Main street centre line (z) and its ends (x).
const STREET_Z: f32 = 96.0;
const STREET_END: f32 = 82.0;

/// Deterministic scatter.
pub struct Lcg(pub u32);

impl Lcg {
    /// Next value in [0, 1).
    pub fn unit(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f32::from(u16::try_from(self.0 >> 16).unwrap_or(0)) / 65_536.0
    }

    /// Next index below `n`.
    pub fn pick(&mut self, n: usize) -> usize {
        self.unit();
        usize::try_from(self.0 >> 16).unwrap_or(0) % n.max(1)
    }

    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unit()
    }

    fn next_bool(&mut self) -> bool {
        self.unit() < 0.5
    }
}

fn fi(n: usize) -> f32 {
    f32::from(u16::try_from(n).unwrap_or(u16::MAX))
}

fn mat(color: Color, rough: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        perceptual_roughness: rough,
        ..default()
    }
}

fn lin(c: Vec3) -> [f32; 4] {
    let l = Color::srgb(c.x, c.y, c.z).to_linear();
    [l.red, l.green, l.blue, 1.0]
}

/// Colour of the ground: meadow tones, patchwork fields away from the line, rock on
/// steep slopes.
fn ground_color(x: f32, z: f32, height: f32, steep: f32) -> [f32; 4] {
    let wave = roll(x * 1.7 + 13.0, z * 1.3 - 7.0);
    let mut col = Vec3::new(0.30, 0.47, 0.21) + Vec3::new(0.035, 0.045, 0.01) * wave;
    if height > 5.5 {
        col = col.lerp(Vec3::new(0.36, 0.48, 0.22), 0.5);
    }
    let fields = (x.abs() > 152.0 || z < -98.0) && steep < 0.35;
    if fields {
        let wa = (x * 0.027 + z * 0.006).sin();
        let wb = (z * 0.033 - x * 0.007 + 1.0).sin();
        let furrow = 0.04 * (x * 1.3 + z * 0.2).sin();
        let field = match (wa > 0.0, wb > 0.0) {
            (true, true) => Vec3::new(0.78, 0.66, 0.30),
            (true, false) => Vec3::new(0.42, 0.29, 0.18),
            (false, true) => Vec3::new(0.40, 0.56, 0.24),
            (false, false) => Vec3::new(0.82, 0.76, 0.22),
        };
        col = field + Vec3::splat(furrow);
    }
    let in_village = x > VILLAGE.0 && x < VILLAGE.1 && z > VILLAGE.2 && z < VILLAGE.3;
    if in_village {
        col = col.lerp(Vec3::new(0.38, 0.52, 0.27), 0.5);
    }
    let rock = ((steep - 0.45) / 0.35).clamp(0.0, 1.0);
    lin(col.lerp(Vec3::new(0.47, 0.44, 0.40), rock))
}

/// The landscape as one mesh with per-vertex colour.
#[must_use]
pub fn terrain_mesh(t: &Terrain) -> Mesh {
    let mut nx = 0_usize;
    while -TABLE_X + STEP * fi(nx) <= TABLE_X + 1e-3 {
        nx += 1;
    }
    let mut nz = 0_usize;
    while -TABLE_Z + STEP * fi(nz) <= TABLE_Z + 1e-3 {
        nz += 1;
    }
    let at = |ix: usize, iz: usize| (-TABLE_X + STEP * fi(ix), -TABLE_Z + STEP * fi(iz));
    let mut heights = Vec::with_capacity(nx * nz);
    for iz in 0..nz {
        for ix in 0..nx {
            let (x, z) = at(ix, iz);
            heights.push(t.height(x, z));
        }
    }
    let height_at = |ix: usize, iz: usize| {
        heights
            .get(iz.min(nz - 1) * nx + ix.min(nx - 1))
            .copied()
            .unwrap_or(0.0)
    };
    let mut positions = Vec::with_capacity(nx * nz);
    let mut normals = Vec::with_capacity(nx * nz);
    let mut colors = Vec::with_capacity(nx * nz);
    for iz in 0..nz {
        for ix in 0..nx {
            let (x, z) = at(ix, iz);
            let height = height_at(ix, iz);
            let dx = height_at(ix + 1, iz) - height_at(ix.saturating_sub(1), iz);
            let dz = height_at(ix, iz + 1) - height_at(ix, iz.saturating_sub(1));
            let normal = Vec3::new(-dx, 2.0 * STEP, -dz).normalize_or_zero();
            positions.push([x, height, z]);
            normals.push(normal.to_array());
            colors.push(ground_color(x, z, height, 1.0 - normal.y));
        }
    }
    let mut indices = Vec::with_capacity((nx - 1) * (nz - 1) * 6);
    let id = |ix: usize, iz: usize| u32::try_from(iz * nx + ix).unwrap_or(0);
    for iz in 0..nz - 1 {
        for ix in 0..nx - 1 {
            let (i00, i10, i01, i11) = (
                id(ix, iz),
                id(ix + 1, iz),
                id(ix, iz + 1),
                id(ix + 1, iz + 1),
            );
            indices.extend_from_slice(&[i00, i01, i10, i10, i01, i11]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

/// A unit gable roof: base 1 x 1 (x along the ridge, z across), height 1, flat-shaded.
#[must_use]
pub fn gable_mesh() -> Mesh {
    let (l, r, f, b) = (-0.5_f32, 0.5_f32, -0.5_f32, 0.5_f32);
    let top_f = [l, 1.0, 0.0];
    let top_r = [r, 1.0, 0.0];
    let mut pos: Vec<[f32; 3]> = Vec::new();
    let mut nor: Vec<[f32; 3]> = Vec::new();
    // Counter-clockwise seen from outside, so the faces survive back-face culling.
    let mut quad = |p: [[f32; 3]; 4], n: Vec3| {
        for k in [0, 2, 1, 0, 3, 2] {
            pos.push(p[k]);
            nor.push(n.to_array());
        }
    };
    // Front slope (towards -z) and back slope.
    quad(
        [[l, 0.0, f], [r, 0.0, f], top_r, top_f],
        Vec3::new(0.0, 0.5, -1.0).normalize(),
    );
    quad(
        [[r, 0.0, b], [l, 0.0, b], top_f, top_r],
        Vec3::new(0.0, 0.5, 1.0).normalize(),
    );
    // Gable ends.
    for (x, nx) in [(l, -1.0_f32), (r, 1.0)] {
        let tri = if nx < 0.0 {
            [[x, 0.0, f], [x, 0.0, b], [x, 1.0, 0.0]]
        } else {
            [[x, 0.0, b], [x, 0.0, f], [x, 1.0, 0.0]]
        };
        for p in tri {
            pos.push(p);
            nor.push([nx, 0.0, 0.0]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nor)
}

/// Shared handles for the scenery.
pub struct Kit {
    /// Lit at night.
    pub windows: Handle<StandardMaterial>,
    /// Street lamps and car headlights, lit at night.
    pub lamps: Handle<StandardMaterial>,
    /// Tail lights.
    pub tail: Handle<StandardMaterial>,
}

/// Trees, village, vineyard and cars.
pub fn spawn_scenery(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain: &Terrain,
    kit: &Kit,
) {
    spawn_trees(commands, meshes, materials, terrain);
    spawn_village(commands, meshes, materials, terrain, kit);
    spawn_vineyard(commands, meshes, materials, terrain);
    spawn_cars(commands, meshes, materials, kit);
    spawn_water(commands, meshes, materials);
    spawn_bridge(commands, meshes, materials);
    spawn_castle(commands, meshes, materials, terrain);
    spawn_pasture(commands, meshes, materials, terrain);
}

/// The castle ruin on the hilltop.
const CASTLE: (f32, f32) = (0.0, -6.0);
/// The cow pasture inside the west helix: x0, x1, z0, z1.
const PASTURE: (f32, f32, f32, f32) = (-126.0, -80.0, -20.0, 22.0);

fn in_pasture(x: f32, z: f32, margin: f32) -> bool {
    x > PASTURE.0 - margin
        && x < PASTURE.1 + margin
        && z > PASTURE.2 - margin
        && z < PASTURE.3 + margin
}

/// One water plane under the whole table: the land hides it except where the river
/// and the lake are carved.
fn spawn_water(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0 * TABLE_X, 0.05, 2.0 * TABLE_Z))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.27, 0.30),
            perceptual_roughness: 0.05,
            reflectance: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, WATER_Y, 0.0),
    ));
}

/// Plate-girder bridge where the lower line crosses the river, on stone abutments.
fn spawn_bridge(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let steel = materials.add(mat(Color::srgb(0.22, 0.28, 0.25), 0.55));
    let stone = materials.add(mat(Color::srgb(0.55, 0.52, 0.47), 0.95));
    let span = 15.0;
    let z = LINE_Z_LOWER;
    for side in [-1.0_f32, 1.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(span, 1.7, 0.35))),
            MeshMaterial3d(steel.clone()),
            Transform::from_xyz(BRIDGE_X, 0.25, z + side * 2.05),
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(3.0, 3.6, 7.0))),
            MeshMaterial3d(stone.clone()),
            Transform::from_xyz(BRIDGE_X + side * (span / 2.0 + 1.0), -1.6, z),
        ));
    }
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(span, 0.5, 4.4))),
        MeshMaterial3d(steel.clone()),
        Transform::from_xyz(BRIDGE_X, -0.25, z),
    ));
    // Stiffeners along the girders.
    let rib = meshes.add(Cuboid::new(0.15, 1.6, 0.5));
    let mut x = BRIDGE_X - span / 2.0 + 1.0;
    while x < BRIDGE_X + span / 2.0 {
        for side in [-1.0_f32, 1.0] {
            commands.spawn((
                Mesh3d(rib.clone()),
                MeshMaterial3d(steel.clone()),
                Transform::from_xyz(x, 0.25, z + side * 2.3),
            ));
        }
        x += 1.6;
    }
}

/// A ruined castle: a keep, broken corner towers, walls with gaps.
fn spawn_castle(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain: &Terrain,
) {
    let stone = materials.add(mat(Color::srgb(0.62, 0.59, 0.52), 0.95));
    let dark = materials.add(mat(Color::srgb(0.12, 0.11, 0.10), 0.9));
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let tower = meshes.add(Cylinder::new(1.0, 1.0));
    let (cx, cz) = CASTLE;
    let base = terrain.height(cx, cz) - 0.6;
    let (hw, hd) = (10.0_f32, 7.0_f32);
    let block = |commands: &mut Commands,
                 m: &Handle<Mesh>,
                 mt: &Handle<StandardMaterial>,
                 at: Vec3,
                 size: Vec3| {
        commands.spawn((
            Mesh3d(m.clone()),
            MeshMaterial3d(mt.clone()),
            Transform::from_translation(at).with_scale(size),
        ));
    };
    // Walls, each with a ruined gap: (centre x, centre z, length, along x).
    for (wx, wz, len, along_x, height) in [
        (cx - 3.0, cz - hd, 2.0 * hw - 8.0, true, 5.0),
        (cx + 2.0, cz + hd, 2.0 * hw - 6.0, true, 3.2),
        (cx - hw, cz, 2.0 * hd, false, 4.4),
        (cx + hw, cz + 1.5, 2.0 * hd - 5.0, false, 3.6),
    ] {
        let size = if along_x {
            Vec3::new(len, height, 1.2)
        } else {
            Vec3::new(1.2, height, len)
        };
        block(
            commands,
            &cube,
            &stone,
            Vec3::new(wx, base + height / 2.0, wz),
            size,
        );
    }
    for (k, (tx, tz)) in [(-hw, -hd), (hw, -hd), (-hw, hd), (hw, hd)]
        .iter()
        .enumerate()
    {
        let height = [10.0_f32, 6.5, 8.5, 4.0].get(k).copied().unwrap_or(6.0);
        block(
            commands,
            &tower,
            &stone,
            Vec3::new(cx + tx, base + height / 2.0, cz + tz),
            Vec3::new(2.4, height, 2.4),
        );
    }
    // The keep with crenellations and a dark doorway.
    let (kx, kz, kh) = (cx - 4.0, cz - 1.0, 14.0);
    block(
        commands,
        &cube,
        &stone,
        Vec3::new(kx, base + kh / 2.0, kz),
        Vec3::new(6.0, kh, 6.0),
    );
    for (mx, mz) in [
        (-2.5, -2.5),
        (0.0, -2.5),
        (2.5, -2.5),
        (-2.5, 2.5),
        (0.0, 2.5),
        (2.5, 2.5),
        (-2.5, 0.0),
        (2.5, 0.0),
    ] {
        block(
            commands,
            &cube,
            &stone,
            Vec3::new(kx + mx, base + kh + 0.5, kz + mz),
            Vec3::new(1.0, 1.0, 1.0),
        );
    }
    block(
        commands,
        &cube,
        &dark,
        Vec3::new(kx, base + 1.9, kz + 3.02),
        Vec3::new(1.4, 2.4, 0.06),
    );
}

/// A fenced pasture with a red barn and a herd of Fleckvieh cows.
#[allow(clippy::too_many_lines)]
fn spawn_pasture(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain: &Terrain,
) {
    let wood = materials.add(mat(Color::srgb(0.42, 0.30, 0.20), 0.9));
    let white = materials.add(mat(Color::srgb(0.95, 0.93, 0.88), 0.85));
    let brown = materials.add(mat(Color::srgb(0.55, 0.28, 0.14), 0.85));
    let hoof = materials.add(mat(Color::srgb(0.15, 0.12, 0.10), 0.8));
    let barn_red = materials.add(mat(Color::srgb(0.55, 0.16, 0.12), 0.85));
    let roof_grey = materials.add(mat(Color::srgb(0.32, 0.32, 0.34), 0.7));
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let post = meshes.add(Cuboid::new(0.18, 1.4, 0.18));
    // Fence: posts every 3 m, two rails between neighbours.
    let (x0, x1, z0, z1) = PASTURE;
    let mut corners = Vec::new();
    let mut x = x0;
    while x < x1 {
        corners.push((x, z0));
        x += 3.0;
    }
    let mut z = z0;
    while z < z1 {
        corners.push((x1, z));
        z += 3.0;
    }
    let mut x = x1;
    while x > x0 {
        corners.push((x, z1));
        x -= 3.0;
    }
    let mut z = z1;
    while z > z0 {
        corners.push((x0, z));
        z -= 3.0;
    }
    let fence: Vec<Vec3> = corners
        .iter()
        .map(|&(px, pz)| Vec3::new(px, terrain.height(px, pz), pz))
        .collect();
    for (k, &p) in fence.iter().enumerate() {
        commands.spawn((
            Mesh3d(post.clone()),
            MeshMaterial3d(wood.clone()),
            Transform::from_translation(p + Vec3::Y * 0.7),
        ));
        let Some(&q) = fence.get((k + 1) % fence.len()) else {
            continue;
        };
        let mid = (p + q) / 2.0;
        let len = p.distance(q);
        if len < 0.1 || len > 5.0 {
            continue;
        }
        for h in [0.55_f32, 1.1] {
            commands.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(wood.clone()),
                Transform::from_translation(mid + Vec3::Y * h)
                    .looking_to(q - p, Vec3::Y)
                    .with_scale(Vec3::new(0.08, 0.12, len)),
            ));
        }
    }
    // Barn in the north-west corner.
    let (bx, bz) = (x0 + 9.0, z1 - 7.0);
    let bg = terrain.height(bx, bz);
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(barn_red.clone()),
        Transform::from_xyz(bx, bg + 2.6, bz).with_scale(Vec3::new(12.0, 5.6, 8.0)),
    ));
    commands.spawn((
        Mesh3d(meshes.add(gable_mesh())),
        MeshMaterial3d(roof_grey),
        Transform::from_xyz(bx, bg + 5.4, bz).with_scale(Vec3::new(12.8, 3.6, 9.0)),
    ));
    // Cows.
    let mut rng = Lcg(1907);
    for _ in 0..16 {
        let cx = rng.range(x0 + 4.0, x1 - 4.0);
        let cz = rng.range(z0 + 3.0, z1 - 3.0);
        if (cx - bx).abs() < 8.0 && (cz - bz).abs() < 6.0 {
            continue;
        }
        let g = terrain.height(cx, cz);
        let yaw = rng.range(0.0, std::f32::consts::TAU);
        let parent = commands
            .spawn((
                Transform::from_xyz(cx, g, cz).with_rotation(Quat::from_rotation_y(yaw)),
                Visibility::default(),
            ))
            .id();
        let grazing = rng.next_bool();
        let parts = [
            (
                &white,
                Vec3::new(0.0, 1.15, 0.0),
                Vec3::new(0.95, 0.95, 2.1),
            ),
            (
                &brown,
                Vec3::new(0.0, 1.25, 0.45),
                Vec3::new(0.98, 0.7, 0.9),
            ),
            (
                &brown,
                Vec3::new(0.0, 1.35, -0.55),
                Vec3::new(0.98, 0.5, 0.6),
            ),
            (
                &white,
                if grazing {
                    Vec3::new(0.0, 0.75, -1.35)
                } else {
                    Vec3::new(0.0, 1.45, -1.3)
                },
                Vec3::new(0.5, 0.55, 0.75),
            ),
            (
                &hoof,
                Vec3::new(0.32, 0.35, -0.75),
                Vec3::new(0.2, 0.7, 0.2),
            ),
            (
                &hoof,
                Vec3::new(-0.32, 0.35, -0.75),
                Vec3::new(0.2, 0.7, 0.2),
            ),
            (&hoof, Vec3::new(0.32, 0.35, 0.75), Vec3::new(0.2, 0.7, 0.2)),
            (
                &hoof,
                Vec3::new(-0.32, 0.35, 0.75),
                Vec3::new(0.2, 0.7, 0.2),
            ),
        ];
        for (m, at, size) in parts {
            commands.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(m.clone()),
                Transform::from_translation(at).with_scale(size),
                ChildOf(parent),
            ));
        }
    }
}

fn forest(x: f32, z: f32) -> bool {
    roll(x * 0.6 + 31.0, z * 0.6 - 17.0) > 0.45 || natural(x, z) > 3.5
}

#[allow(clippy::too_many_lines)]
fn spawn_trees(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain: &Terrain,
) {
    let bark = materials.add(mat(Color::srgb(0.30, 0.21, 0.14), 0.95));
    let birch = materials.add(mat(Color::srgb(0.85, 0.83, 0.78), 0.9));
    let leaves: Vec<Handle<StandardMaterial>> = [
        (0.20, 0.38, 0.15),
        (0.26, 0.44, 0.17),
        (0.32, 0.47, 0.18),
        (0.22, 0.34, 0.13),
        (0.36, 0.42, 0.16),
        (0.30, 0.50, 0.22),
    ]
    .iter()
    .map(|&(r, g, b)| materials.add(mat(Color::srgb(r, g, b), 0.85)))
    .collect();
    let needles: Vec<Handle<StandardMaterial>> =
        [(0.10, 0.25, 0.15), (0.13, 0.29, 0.16), (0.09, 0.22, 0.13)]
            .iter()
            .map(|&(r, g, b)| materials.add(mat(Color::srgb(r, g, b), 0.9)))
            .collect();
    let trunk = meshes.add(Cylinder::new(0.22, 1.0));
    let blob_mesh = Sphere::new(1.0)
        .mesh()
        .ico(1)
        .unwrap_or_else(|_| Mesh::from(Sphere::new(1.0)));
    let blob = meshes.add(blob_mesh);
    let cone = meshes.add(Cone {
        radius: 1.0,
        height: 1.0,
    });
    let mut rng = Lcg(2026);
    let mut placed = 0_u32;
    for _ in 0..9000 {
        if placed >= 1500 {
            break;
        }
        let x = rng.range(-TABLE_X + 6.0, TABLE_X - 6.0);
        let z = rng.range(-TABLE_Z + 6.0, TABLE_Z - 6.0);
        let keep = if forest(x, z) { 0.85 } else { 0.07 };
        if rng.unit() > keep {
            continue;
        }
        let village = x > VILLAGE.0 - 4.0
            && x < VILLAGE.1 + 4.0
            && z > VILLAGE.2 - 4.0
            && z < VILLAGE.3 + 4.0;
        let station = x.abs() < 76.0 && (z.abs() - 56.0).abs() < 20.0;
        let fields = x.abs() > 152.0 || z < -98.0;
        let vineyard = x.abs() < 34.0 && z > 4.0 && z < 23.0;
        let castle = (x - CASTLE.0).powi(2) + (z - CASTLE.1).powi(2) < 17.0 * 17.0;
        let wet = water_distance(x, z) < 3.0;
        if village
            || station
            || vineyard
            || castle
            || wet
            || in_pasture(x, z, 3.0)
            || (fields && rng.unit() > 0.15)
        {
            continue;
        }
        if terrain.track_distance(x, z) < 9.0 {
            continue;
        }
        let ground = terrain.height(x, z);
        let s = rng.range(0.75, 1.35);
        let tilt = Quat::from_rotation_z(rng.range(-0.06, 0.06))
            * Quat::from_rotation_x(rng.range(-0.06, 0.06));
        let parent = commands
            .spawn((
                Transform::from_xyz(x, ground, z).with_rotation(tilt),
                Visibility::default(),
            ))
            .id();
        let conifer = rng.unit() < if ground > 3.5 { 0.7 } else { 0.3 };
        if conifer {
            let tone = needles
                .get(rng.pick(needles.len()))
                .cloned()
                .unwrap_or_default();
            commands.spawn((
                Mesh3d(trunk.clone()),
                MeshMaterial3d(bark.clone()),
                Transform::from_xyz(0.0, 0.8 * s, 0.0).with_scale(Vec3::new(s, 1.6 * s, s)),
                ChildOf(parent),
            ));
            for (k, (r, h)) in [(2.1_f32, 3.4_f32), (1.6, 3.0), (1.05, 2.6)]
                .iter()
                .enumerate()
            {
                let y = (1.4 + 1.55 * fi(k) + h / 2.0) * s;
                commands.spawn((
                    Mesh3d(cone.clone()),
                    MeshMaterial3d(tone.clone()),
                    Transform::from_xyz(0.0, y, 0.0).with_scale(Vec3::new(r * s, h * s, r * s)),
                    ChildOf(parent),
                ));
            }
        } else {
            let is_birch = rng.unit() < 0.15;
            let trunk_h = rng.range(2.0, 3.0) * s;
            commands.spawn((
                Mesh3d(trunk.clone()),
                MeshMaterial3d(if is_birch {
                    birch.clone()
                } else {
                    bark.clone()
                }),
                Transform::from_xyz(0.0, trunk_h / 2.0, 0.0).with_scale(Vec3::new(
                    s,
                    trunk_h + 1.0,
                    s,
                )),
                ChildOf(parent),
            ));
            let tone = leaves
                .get(rng.pick(leaves.len()))
                .cloned()
                .unwrap_or_default();
            let crown_y = trunk_h + 1.4 * s;
            let blobs = 3 + rng.pick(3);
            for _ in 0..blobs {
                let r = rng.range(1.2, 1.9) * s;
                let offset = Vec3::new(
                    rng.range(-1.0, 1.0) * s,
                    rng.range(-0.4, 0.9) * s,
                    rng.range(-1.0, 1.0) * s,
                );
                commands.spawn((
                    Mesh3d(blob.clone()),
                    MeshMaterial3d(tone.clone()),
                    Transform::from_translation(Vec3::new(0.0, crown_y, 0.0) + offset)
                        .with_scale(Vec3::new(r, r * 0.9, r)),
                    ChildOf(parent),
                ));
            }
        }
        placed += 1;
    }
}

/// One house: `at` is the footprint centre on the ground, `facing` the street side.
struct House {
    at: Vec3,
    facing: Vec3,
    width: f32,
    depth: f32,
    floors: u8,
}

struct VillageKit {
    walls: Vec<Handle<StandardMaterial>>,
    roofs: Vec<Handle<StandardMaterial>>,
    door: Handle<StandardMaterial>,
    cube: Handle<Mesh>,
    gable: Handle<Mesh>,
}

fn spawn_house(commands: &mut Commands, vk: &VillageKit, kit: &Kit, house: &House, rng: &mut Lcg) {
    let yaw = (-house.facing.x).atan2(-house.facing.z);
    let parent = commands
        .spawn((
            Transform::from_translation(house.at).with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
        ))
        .id();
    let (width, depth) = (house.width, house.depth);
    let height = 3.2 * f32::from(house.floors) + 0.5;
    let wall = vk
        .walls
        .get(rng.pick(vk.walls.len()))
        .cloned()
        .unwrap_or_default();
    let roof = vk
        .roofs
        .get(rng.pick(vk.roofs.len()))
        .cloned()
        .unwrap_or_default();
    let mut part = |m: &Handle<Mesh>, material: &Handle<StandardMaterial>, t: Transform| {
        commands.spawn((
            Mesh3d(m.clone()),
            MeshMaterial3d(material.clone()),
            t,
            ChildOf(parent),
        ));
    };
    part(
        &vk.cube,
        &wall,
        Transform::from_xyz(0.0, height / 2.0 - 0.3, 0.0).with_scale(Vec3::new(
            width,
            height + 0.6,
            depth,
        )),
    );
    part(
        &vk.gable,
        &roof,
        Transform::from_xyz(0.0, height - 0.3, 0.0).with_scale(Vec3::new(
            width + 0.8,
            rng.range(2.4, 3.6),
            depth + 0.9,
        )),
    );
    // Windows on the street side and the back, a door at the street.
    let mut n = 1_usize;
    while fi(n + 1) * 2.6 < width {
        n += 1;
    }
    for floor in 0..house.floors {
        let y = 3.2 * f32::from(floor) + 1.9;
        for k in 0..n {
            let x = -width / 2.0 + width * (fi(k) + 0.5) / fi(n);
            if floor == 0 && k == n / 2 {
                part(
                    &vk.cube,
                    &vk.door,
                    Transform::from_xyz(x, 1.1, -depth / 2.0 - 0.04)
                        .with_scale(Vec3::new(1.1, 2.2, 0.08)),
                );
                continue;
            }
            for side in [-1.0_f32, 1.0] {
                part(
                    &vk.cube,
                    &kit.windows,
                    Transform::from_xyz(x, y, side * (depth / 2.0 + 0.04))
                        .with_scale(Vec3::new(0.95, 1.15, 0.08)),
                );
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
fn spawn_village(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain: &Terrain,
    kit: &Kit,
) {
    let vk = VillageKit {
        walls: [
            (0.93, 0.86, 0.66),
            (0.95, 0.92, 0.84),
            (0.92, 0.78, 0.72),
            (0.88, 0.80, 0.55),
            (0.82, 0.86, 0.78),
            (0.96, 0.95, 0.93),
        ]
        .iter()
        .map(|&(r, g, b)| materials.add(mat(Color::srgb(r, g, b), 0.85)))
        .collect(),
        roofs: [
            (0.58, 0.20, 0.14),
            (0.45, 0.17, 0.12),
            (0.30, 0.30, 0.33),
            (0.52, 0.30, 0.18),
        ]
        .iter()
        .map(|&(r, g, b)| materials.add(mat(Color::srgb(r, g, b), 0.7)))
        .collect(),
        door: materials.add(mat(Color::srgb(0.30, 0.18, 0.10), 0.6)),
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        gable: meshes.add(gable_mesh()),
    };
    let ground = terrain.height(0.0, STREET_Z);
    let asphalt = materials.add(mat(Color::srgb(0.22, 0.22, 0.24), 0.95));
    commands.spawn((
        Mesh3d(vk.cube.clone()),
        MeshMaterial3d(asphalt.clone()),
        Transform::from_xyz(0.0, ground + 0.06, STREET_Z).with_scale(Vec3::new(
            2.0 * STREET_END,
            0.12,
            6.5,
        )),
    ));
    commands.spawn((
        Mesh3d(vk.cube.clone()),
        MeshMaterial3d(asphalt.clone()),
        Transform::from_xyz(0.0, ground + 0.07, STREET_Z).with_scale(Vec3::new(6.5, 0.12, 52.0)),
    ));
    let mut rng = Lcg(1683);
    // Along the main street, both sides; the church plaza is north-east of the crossing.
    for side in [-1.0_f32, 1.0] {
        let mut x = -76.0_f32;
        while x < 74.0 {
            let w = rng.range(6.0, 10.0);
            let d = rng.range(7.0, 9.0);
            let cx = x + w / 2.0;
            let plaza = side > 0.0 && cx > 22.0 && cx < 60.0;
            if cx.abs() < 8.0 + w / 2.0 || plaza {
                x += w + 1.5;
                continue;
            }
            let z = STREET_Z + side * (3.25 + 1.4 + d / 2.0);
            let house = House {
                at: Vec3::new(cx, terrain.height(cx, z), z),
                facing: Vec3::new(0.0, 0.0, -side),
                width: w,
                depth: d,
                floors: if rng.unit() < 0.45 { 2 } else { 1 },
            };
            spawn_house(commands, &vk, kit, &house, &mut rng);
            x += w + rng.range(1.0, 3.0);
        }
    }
    // Along the cross street.
    for side in [-1.0_f32, 1.0] {
        let mut z = 72.0_f32;
        while z < 120.0 {
            let w = rng.range(6.0, 9.0);
            let d = rng.range(7.0, 8.5);
            let cz = z + w / 2.0;
            if (cz - STREET_Z).abs() < 8.0 + d / 2.0 + w / 2.0 {
                z += w + 1.5;
                continue;
            }
            let x = side * (3.25 + 1.4 + d / 2.0);
            let house = House {
                at: Vec3::new(x, terrain.height(x, cz), cz),
                facing: Vec3::new(-side, 0.0, 0.0),
                width: w,
                depth: d,
                floors: if rng.unit() < 0.5 { 2 } else { 1 },
            };
            spawn_house(commands, &vk, kit, &house, &mut rng);
            z += w + rng.range(1.0, 2.5);
        }
    }
    // The church: white nave, red roof, tower with a green onion dome.
    let white = materials.add(mat(Color::srgb(0.96, 0.95, 0.92), 0.8));
    let copper = materials.add(mat(Color::srgb(0.32, 0.56, 0.46), 0.5));
    let church_roof = materials.add(mat(Color::srgb(0.50, 0.18, 0.13), 0.7));
    let (cx, cz) = (41.0_f32, 112.0_f32);
    let cg = terrain.height(cx, cz);
    let piece = |commands: &mut Commands,
                 m: Handle<Mesh>,
                 material: &Handle<StandardMaterial>,
                 t: Transform| {
        commands.spawn((Mesh3d(m), MeshMaterial3d(material.clone()), t));
    };
    piece(
        commands,
        vk.cube.clone(),
        &white,
        Transform::from_xyz(cx, cg + 4.5, cz).with_scale(Vec3::new(10.0, 9.0, 20.0)),
    );
    piece(
        commands,
        vk.gable.clone(),
        &church_roof,
        Transform::from_xyz(cx, cg + 9.0, cz)
            .with_rotation(Quat::from_rotation_y(FRAC_PI_2))
            .with_scale(Vec3::new(21.0, 5.0, 11.0)),
    );
    let tower_z = cz - 12.0;
    piece(
        commands,
        vk.cube.clone(),
        &white,
        Transform::from_xyz(cx, cg + 10.0, tower_z).with_scale(Vec3::new(5.0, 20.0, 5.0)),
    );
    piece(
        commands,
        meshes.add(Sphere::new(1.0)),
        &copper,
        Transform::from_xyz(cx, cg + 21.6, tower_z).with_scale(Vec3::new(2.8, 3.2, 2.8)),
    );
    piece(
        commands,
        meshes.add(Cone {
            radius: 1.0,
            height: 1.0,
        }),
        &copper,
        Transform::from_xyz(cx, cg + 25.5, tower_z).with_scale(Vec3::new(1.0, 3.6, 1.0)),
    );
    for side in [-1.0_f32, 1.0] {
        piece(
            commands,
            vk.cube.clone(),
            &kit.windows,
            Transform::from_xyz(cx + side * 5.05, cg + 5.0, cz)
                .with_scale(Vec3::new(0.08, 4.0, 1.4)),
        );
    }
    // Street lamps.
    let post = meshes.add(Cylinder::new(0.09, 4.5));
    let bulb = meshes.add(Sphere::new(0.32));
    let dark = materials.add(mat(Color::srgb(0.12, 0.12, 0.13), 0.6));
    let mut x = -STREET_END + 6.0;
    while x < STREET_END {
        for side in [-1.0_f32, 1.0] {
            let z = STREET_Z + side * 3.9;
            commands.spawn((
                Mesh3d(post.clone()),
                MeshMaterial3d(dark.clone()),
                Transform::from_xyz(x, ground + 2.25, z),
            ));
            commands.spawn((
                Mesh3d(bulb.clone()),
                MeshMaterial3d(kit.lamps.clone()),
                Transform::from_xyz(x, ground + 4.6, z),
            ));
        }
        x += 16.0;
    }
}

fn spawn_vineyard(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain: &Terrain,
) {
    let vine = meshes.add(Cuboid::new(0.45, 1.1, 0.45));
    let green = materials.add(mat(Color::srgb(0.22, 0.40, 0.14), 0.85));
    let mut z = 6.0_f32;
    while z < 21.0 {
        let mut x = -30.0_f32;
        while x <= 30.0 {
            let y = terrain.height(x, z);
            commands.spawn((
                Mesh3d(vine.clone()),
                MeshMaterial3d(green.clone()),
                Transform::from_xyz(x, y + 0.55, z),
            ));
            x += 1.15;
        }
        z += 1.7;
    }
}

/// A car on the main street: lane (1 eastbound, -1 westbound) and where it starts.
#[derive(Component, Debug, Clone, Copy)]
pub struct StreetCar {
    lane: f32,
    start: f32,
    speed: f32,
}

/// Seconds of street time; stops while the railway is paused.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct StreetClock(pub f32);

fn spawn_cars(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    kit: &Kit,
) {
    let body = meshes.add(Cuboid::new(1.9, 0.9, 4.2));
    let cabin = meshes.add(Cuboid::new(1.7, 0.75, 2.2));
    let light = meshes.add(Cuboid::new(0.35, 0.2, 0.06));
    let glass = materials.add(mat(Color::srgb(0.10, 0.12, 0.16), 0.15));
    let paints = [
        (0.75, 0.10, 0.10),
        (0.12, 0.25, 0.60),
        (0.92, 0.92, 0.90),
        (0.15, 0.15, 0.16),
        (0.85, 0.65, 0.15),
        (0.20, 0.45, 0.30),
        (0.55, 0.55, 0.58),
    ];
    let mut rng = Lcg(77);
    for (k, &(r, g, b)) in paints.iter().enumerate() {
        let paint = materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            metallic: 0.4,
            perceptual_roughness: 0.35,
            ..default()
        });
        let car = StreetCar {
            lane: if k % 2 == 0 { 1.0 } else { -1.0 },
            start: rng.range(0.0, 2.0 * STREET_END),
            speed: rng.range(7.0, 11.0),
        };
        let parent = commands
            .spawn((car, Transform::default(), Visibility::default()))
            .id();
        for (m, material, t) in [
            (&body, &paint, Transform::from_xyz(0.0, 0.62, 0.0)),
            (&cabin, &glass, Transform::from_xyz(0.0, 1.42, 0.3)),
            (&light, &kit.lamps, Transform::from_xyz(0.6, 0.72, -2.12)),
            (&light, &kit.lamps, Transform::from_xyz(-0.6, 0.72, -2.12)),
            (&light, &kit.tail, Transform::from_xyz(0.6, 0.72, 2.12)),
            (&light, &kit.tail, Transform::from_xyz(-0.6, 0.72, 2.12)),
        ] {
            commands.spawn((
                Mesh3d(m.clone()),
                MeshMaterial3d(material.clone()),
                t,
                ChildOf(parent),
            ));
        }
    }
}

/// Drive the cars along the main street, wrapping at the ends.
pub fn drive_cars(
    time: Res<Time>,
    clock: Res<Clockwork>,
    mut street: ResMut<StreetClock>,
    mut cars: Query<(&StreetCar, &mut Transform)>,
) {
    if !clock.paused {
        let rate = f32::from(u16::try_from(clock.speed.min(64)).unwrap_or(1));
        street.0 = (street.0 + time.delta_secs() * rate) % 100_000.0;
    }
    let span = 2.0 * STREET_END;
    for (car, mut t) in &mut cars {
        let along = (car.start + street.0 * car.speed) % span;
        let x = if car.lane > 0.0 {
            -STREET_END + along
        } else {
            STREET_END - along
        };
        let z = STREET_Z + car.lane * 1.6;
        *t = Transform::from_xyz(x, 0.37, z).looking_to(Vec3::new(car.lane, 0.0, 0.0), Vec3::Y);
    }
}

//! Parametric props: buildings and creatures built from a spec, at explicit LOD.
//!
//! The point is that a "church" and an "igloo" are the same code path with
//! different parameters, and level of detail is a node-count decision, not an
//! art decision:
//!
//! | LOD | Nodes per building | Use |
//! |---|---|---|
//! | 0 | 1 (one box) | far / background |
//! | 1 | 2 (+ roof) | mid distance |
//! | 2 | 4-6 (+ door, chimney or tower) | near / flown past |
//!
//! Nothing here is art. Every part is a primitive with a flat colour, so the
//! whole village is still deterministic text.

use crate::model::Forge;
use gamecraft_core::NodeId;

/// Which building to instantiate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingKind {
    /// Cream walls, red pitched roof, chimney.
    Farmhouse,
    /// Dark red walls, steep roof.
    Barn,
    /// Pale stone, tower and spire at LOD 2.
    Church,
    /// Squashed dome with a low entrance.
    Igloo,
    /// Cylindrical tower with four sails at LOD 2.
    Windmill,
    /// Tapered tower with a lantern room at LOD 2.
    Lighthouse,
    /// Keep with corner turrets and crenellations at LOD 2.
    Castle,
}

/// One building instance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuildingSpec {
    /// Which kind.
    pub kind: BuildingKind,
    /// World x.
    pub x: f32,
    /// World z.
    pub z: f32,
    /// Footprint width.
    pub width: f32,
    /// Footprint depth.
    pub depth: f32,
    /// Wall height.
    pub height: f32,
    /// 0 silhouette, 1 + roof, 2 + detail.
    pub lod: u8,
    /// Optional generated-texture ext-resource id for the walls.
    pub tex: Option<&'static str>,
}

impl BuildingSpec {
    /// A spec with sensible defaults for the kind.
    #[must_use]
    pub fn new(kind: BuildingKind, x: f32, z: f32, lod: u8) -> Self {
        let (width, depth, height) = match kind {
            BuildingKind::Farmhouse => (3.0, 2.4, 1.8),
            BuildingKind::Barn => (4.0, 3.0, 2.2),
            BuildingKind::Church => (2.6, 4.0, 2.6),
            BuildingKind::Igloo => (2.2, 2.2, 1.2),
            BuildingKind::Windmill => (2.4, 2.4, 3.2),
            BuildingKind::Lighthouse => (2.0, 2.0, 4.5),
            BuildingKind::Castle => (5.0, 5.0, 3.0),
        };
        Self {
            kind,
            x,
            z,
            width,
            depth,
            height,
            lod,
            tex: None,
        }
    }

    /// Wall colour for the kind.
    fn wall_color(self) -> (f32, f32, f32) {
        match self.kind {
            BuildingKind::Farmhouse => (0.87, 0.81, 0.69),
            BuildingKind::Barn => (0.55, 0.24, 0.22),
            BuildingKind::Church => (0.82, 0.80, 0.76),
            BuildingKind::Igloo => (0.92, 0.94, 0.97),
            BuildingKind::Windmill => (0.92, 0.90, 0.84),
            BuildingKind::Lighthouse => (0.95, 0.95, 0.96),
            BuildingKind::Castle => (0.62, 0.62, 0.60),
        }
    }

    /// Roof colour for the kind.
    fn roof_color(self) -> (f32, f32, f32) {
        match self.kind {
            BuildingKind::Farmhouse => (0.62, 0.26, 0.20),
            BuildingKind::Barn => (0.30, 0.28, 0.30),
            BuildingKind::Church => (0.38, 0.42, 0.52),
            BuildingKind::Igloo => (0.86, 0.89, 0.94),
            BuildingKind::Windmill => (0.35, 0.30, 0.28),
            BuildingKind::Lighthouse => (0.82, 0.20, 0.20),
            BuildingKind::Castle => (0.38, 0.40, 0.45),
        }
    }
}

/// Instantiate one building. Returns the number of nodes created.
///
/// Long but linear: one node per part. Splitting it per-part would scatter a
/// single readable construction sequence across helpers for no gain.
///
/// # Errors
///
/// Returns a message when a node cannot be created.
#[allow(clippy::too_many_lines)]
pub fn spawn_building(
    forge: &mut Forge,
    root: NodeId,
    serial: usize,
    spec: &BuildingSpec,
) -> Result<usize, String> {
    let base = format!("{}_{serial}", kind_name(spec.kind));
    let (red, green, blue) = spec.wall_color();
    let (roof_r, roof_g, roof_b) = spec.roof_color();
    let mut made = 0_usize;

    // Igloos are a dome, not a box: same code path, different primitive.
    if spec.kind == BuildingKind::Igloo {
        let shell = forge
            .solid_box(
                base.as_str(),
                "StaticBody3D",
                Some(root),
                [spec.x, spec.height * 0.5, spec.z],
                [spec.width, spec.height, spec.depth],
                red,
                green,
                blue,
            )
            .map_err(|err| err.to_string())?;
        made += 1;
        if let Some(node) = forge.graph.get_mut(shell) {
            node.properties.insert(
                "name".to_string(),
                gamecraft_core::PropertyValue::Str(base.clone()),
            );
        }
        // Re-shape the visual: replace the box mesh child with a squashed sphere.
        let dome = forge.sphere_mesh(base.as_str(), spec.width * 0.5, spec.height);
        let mat = forge.material(base.as_str(), red, green, blue);
        if let Some(first_child) = forge
            .graph
            .get(shell)
            .and_then(|node| node.children.first().copied())
            .and_then(|child| forge.graph.get_mut(child))
        {
            first_child
                .properties
                .insert("mesh".to_string(), Forge::sub_ref(dome.as_str()));
            first_child.properties.insert(
                "material_override".to_string(),
                Forge::sub_ref(mat.as_str()),
            );
        }
        if spec.lod >= 2 {
            forge
                .deco_box(
                    format!("{base}_door").as_str(),
                    Some(root),
                    [spec.x + spec.width * 0.4, 0.4, spec.z],
                    [0.8, 0.8, 0.5],
                    0.35,
                    0.40,
                    0.48,
                )
                .map_err(|err| err.to_string())?;
            made += 1;
        }
        return Ok(made);
    }

    // Walls: the single part every LOD has.
    let walls = forge
        .solid_box(
            base.as_str(),
            "StaticBody3D",
            Some(root),
            [spec.x, spec.height * 0.5, spec.z],
            [spec.width, spec.height, spec.depth],
            red,
            green,
            blue,
        )
        .map_err(|err| err.to_string())?;
    made += 1;
    if let Some(tex) = spec.tex {
        forge
            .apply_texture(walls, tex, red, green, blue)
            .map_err(|err| err.to_string())?;
    }

    if spec.lod >= 1 {
        // Pitched roof: a prism reads as a gable from any side the player sees.
        let roof_mesh = forge.prism_mesh(
            format!("{base}_roof").as_str(),
            [spec.width * 1.15, spec.height * 0.55, spec.depth * 1.15],
        );
        let roof_mat = forge.material(format!("{base}_roof").as_str(), roof_r, roof_g, roof_b);
        forge
            .mesh_child("Roof", walls, roof_mesh.as_str(), roof_mat.as_str())
            .map_err(|err| err.to_string())?;
        if let Some(roof_node) = forge
            .graph
            .get(walls)
            .and_then(|node| node.children.get(1).copied())
            .and_then(|child| forge.graph.get_mut(child))
        {
            roof_node.transform.position =
                glam::Vec3::new(0.0, spec.height * 0.5 + spec.height * 0.25, 0.0);
        }
        made += 1;
    }

    if spec.lod >= 2 {
        // Detail that only survives close inspection.
        forge
            .deco_box(
                format!("{base}_door").as_str(),
                Some(root),
                [spec.x, spec.height * 0.35, spec.z + spec.depth * 0.5],
                [spec.width * 0.22, spec.height * 0.7, 0.12],
                0.32,
                0.22,
                0.16,
            )
            .map_err(|err| err.to_string())?;
        made += 1;
        match spec.kind {
            BuildingKind::Church => {
                // Tower plus spire: the silhouette that says "church".
                forge
                    .deco_box(
                        format!("{base}_tower").as_str(),
                        Some(root),
                        [spec.x, spec.height * 0.9, spec.z - spec.depth * 0.35],
                        [spec.width * 0.45, spec.height * 1.8, spec.width * 0.45],
                        red,
                        green,
                        blue,
                    )
                    .map_err(|err| err.to_string())?;
                let spire = forge.prism_mesh(
                    format!("{base}_spire").as_str(),
                    [spec.width * 0.5, spec.height * 0.9, spec.width * 0.5],
                );
                let spire_mat =
                    forge.material(format!("{base}_spire").as_str(), roof_r, roof_g, roof_b);
                let spire_node = forge
                    .place(
                        format!("{base}_spire_node").as_str(),
                        "MeshInstance3D",
                        Some(root),
                        [
                            spec.x,
                            spec.height * 1.8 + spec.height * 0.45,
                            spec.z - spec.depth * 0.35,
                        ],
                    )
                    .map_err(|err| err.to_string())?;
                forge
                    .set_prop(spire_node, "mesh", Forge::sub_ref(spire.as_str()))
                    .map_err(|err| err.to_string())?;
                forge
                    .set_prop(
                        spire_node,
                        "material_override",
                        Forge::sub_ref(spire_mat.as_str()),
                    )
                    .map_err(|err| err.to_string())?;
                made += 2;
            }
            BuildingKind::Farmhouse | BuildingKind::Barn => {
                forge
                    .deco_box(
                        format!("{base}_chimney").as_str(),
                        Some(root),
                        [spec.x - spec.width * 0.3, spec.height * 1.2, spec.z],
                        [0.35, 0.9, 0.35],
                        0.45,
                        0.38,
                        0.34,
                    )
                    .map_err(|err| err.to_string())?;
                made += 1;
            }
            BuildingKind::Windmill => {
                // Hub plus cross-blades: reads as a windmill from the side.
                forge
                    .deco_box(
                        format!("{base}_hub").as_str(),
                        Some(root),
                        [spec.x, spec.height * 1.15, spec.z + spec.depth * 0.55],
                        [0.4, 0.4, 0.4],
                        0.30,
                        0.26,
                        0.24,
                    )
                    .map_err(|err| err.to_string())?;
                for (index, size) in [[0.25_f32, 2.4_f32, 0.12_f32], [2.4, 0.25, 0.12]]
                    .iter()
                    .enumerate()
                {
                    forge
                        .deco_box(
                            format!("{base}_sail{index}").as_str(),
                            Some(root),
                            [spec.x, spec.height * 1.15, spec.z + spec.depth * 0.6],
                            *size,
                            0.96,
                            0.94,
                            0.88,
                        )
                        .map_err(|err| err.to_string())?;
                }
                made += 3;
            }
            BuildingKind::Lighthouse => {
                // Lantern room and a red band; the tower itself is the wall box.
                forge
                    .deco_box(
                        format!("{base}_lantern").as_str(),
                        Some(root),
                        [spec.x, spec.height * 1.05, spec.z],
                        [1.0, 0.9, 1.0],
                        1.0,
                        0.88,
                        0.42,
                    )
                    .map_err(|err| err.to_string())?;
                forge
                    .deco_box(
                        format!("{base}_band").as_str(),
                        Some(root),
                        [spec.x, spec.height * 0.55, spec.z],
                        [spec.width * 1.02, 0.35, spec.depth * 1.02],
                        0.82,
                        0.20,
                        0.20,
                    )
                    .map_err(|err| err.to_string())?;
                made += 2;
            }
            BuildingKind::Castle => {
                // Four corner turrets plus a crenellated parapet.
                for (index, (dx, dz)) in
                    [(-0.5_f32, -0.5_f32), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)]
                        .iter()
                        .enumerate()
                {
                    forge
                        .deco_box(
                            format!("{base}_turret{index}").as_str(),
                            Some(root),
                            [
                                spec.x + dx * spec.width,
                                spec.height * 0.75,
                                spec.z + dz * spec.depth,
                            ],
                            [0.9, spec.height * 1.5, 0.9],
                            red,
                            green,
                            blue,
                        )
                        .map_err(|err| err.to_string())?;
                }
                for index in 0..5_u16 {
                    let span = f32::from(index) - 2.0;
                    forge
                        .deco_box(
                            format!("{base}_crenel{index}").as_str(),
                            Some(root),
                            [
                                spec.x + span * spec.width * 0.4,
                                spec.height * 1.12,
                                spec.z + spec.depth * 0.5,
                            ],
                            [0.5, 0.6, 0.4],
                            red,
                            green,
                            blue,
                        )
                        .map_err(|err| err.to_string())?;
                }
                made += 9;
            }
            BuildingKind::Igloo => {}
        }
    }
    Ok(made)
}

/// Which creature to instantiate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureKind {
    /// Two-segment humanoid: capsule body, sphere head.
    Person,
    /// Four-legged silhouette: box body, sphere head, four leg boxes.
    Cow,
}

/// Instantiate one creature. Returns the number of nodes created.
///
/// # Errors
///
/// Returns a message when a node cannot be created.
pub fn spawn_creature(
    forge: &mut Forge,
    root: NodeId,
    serial: usize,
    kind: CreatureKind,
    x: f32,
    z: f32,
) -> Result<usize, String> {
    let base = format!("{}_{serial}", creature_name(kind));
    let mut made = 0_usize;
    match kind {
        CreatureKind::Person => {
            let body = forge.capsule_mesh(format!("{base}_body").as_str(), 0.22, 1.1);
            let skin = forge.material(format!("{base}_body").as_str(), 0.36, 0.60, 1.0);
            let torso = forge
                .place(base.as_str(), "MeshInstance3D", Some(root), [x, 0.75, z])
                .map_err(|err| err.to_string())?;
            forge
                .set_prop(torso, "mesh", Forge::sub_ref(body.as_str()))
                .map_err(|err| err.to_string())?;
            forge
                .set_prop(torso, "material_override", Forge::sub_ref(skin.as_str()))
                .map_err(|err| err.to_string())?;
            made += 1;
            let head_mesh = forge.sphere_mesh(format!("{base}_head").as_str(), 0.18, 0.36);
            let head_mat = forge.material(format!("{base}_head").as_str(), 0.93, 0.78, 0.64);
            let head = forge
                .place(
                    format!("{base}_head_node").as_str(),
                    "MeshInstance3D",
                    Some(root),
                    [x, 1.45, z],
                )
                .map_err(|err| err.to_string())?;
            forge
                .set_prop(head, "mesh", Forge::sub_ref(head_mesh.as_str()))
                .map_err(|err| err.to_string())?;
            forge
                .set_prop(head, "material_override", Forge::sub_ref(head_mat.as_str()))
                .map_err(|err| err.to_string())?;
            made += 1;
        }
        CreatureKind::Cow => {
            let hide = forge.material(format!("{base}_hide").as_str(), 0.95, 0.95, 0.93);
            let spots = forge.material(format!("{base}_spots").as_str(), 0.18, 0.18, 0.20);
            let body_mesh = forge.box_mesh(format!("{base}_torso").as_str(), [1.3, 0.7, 0.7]);
            let torso = forge
                .place(base.as_str(), "MeshInstance3D", Some(root), [x, 0.85, z])
                .map_err(|err| err.to_string())?;
            forge
                .set_prop(torso, "mesh", Forge::sub_ref(body_mesh.as_str()))
                .map_err(|err| err.to_string())?;
            forge
                .set_prop(torso, "material_override", Forge::sub_ref(hide.as_str()))
                .map_err(|err| err.to_string())?;
            made += 1;
            let head_mesh = forge.box_mesh(format!("{base}_headm").as_str(), [0.45, 0.45, 0.45]);
            let head = forge
                .place(
                    format!("{base}_head_node").as_str(),
                    "MeshInstance3D",
                    Some(root),
                    [x + 0.85, 0.95, z],
                )
                .map_err(|err| err.to_string())?;
            forge
                .set_prop(head, "mesh", Forge::sub_ref(head_mesh.as_str()))
                .map_err(|err| err.to_string())?;
            forge
                .set_prop(head, "material_override", Forge::sub_ref(spots.as_str()))
                .map_err(|err| err.to_string())?;
            made += 1;
            for (leg_index, offset) in [-0.45_f32, 0.45].iter().enumerate() {
                for (side_index, side) in [-0.25_f32, 0.25].iter().enumerate() {
                    forge
                        .deco_box(
                            format!("{base}_leg{leg_index}{side_index}").as_str(),
                            Some(root),
                            [x + offset, 0.35, z + side],
                            [0.18, 0.7, 0.18],
                            0.18,
                            0.18,
                            0.20,
                        )
                        .map_err(|err| err.to_string())?;
                    made += 1;
                }
            }
        }
    }
    Ok(made)
}

/// Slug fragment for a building kind.
fn kind_name(kind: BuildingKind) -> &'static str {
    match kind {
        BuildingKind::Farmhouse => "farmhouse",
        BuildingKind::Barn => "barn",
        BuildingKind::Church => "church",
        BuildingKind::Igloo => "igloo",
        BuildingKind::Windmill => "windmill",
        BuildingKind::Lighthouse => "lighthouse",
        BuildingKind::Castle => "castle",
    }
}

/// Slug fragment for a creature kind.
fn creature_name(kind: CreatureKind) -> &'static str {
    match kind {
        CreatureKind::Person => "person",
        CreatureKind::Cow => "cow",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_spawns_nodes() {
        let mut forge = Forge::new();
        let root = forge.add_node("World", "Node3D", None).unwrap_or(0);
        let kinds = [
            BuildingKind::Farmhouse,
            BuildingKind::Barn,
            BuildingKind::Church,
            BuildingKind::Igloo,
            BuildingKind::Windmill,
            BuildingKind::Lighthouse,
            BuildingKind::Castle,
        ];
        for (serial, kind) in kinds.iter().enumerate() {
            let spec = BuildingSpec::new(*kind, 0.0, 0.0, 2);
            let made = spawn_building(&mut forge, root, serial, &spec).unwrap_or(0);
            assert!(made > 0, "{kind:?} produced no nodes");
        }
    }
    #[test]
    fn lod_controls_node_count() {
        let mut forge = Forge::new();
        let root = forge.add_node("World", "Node3D", None).unwrap_or(0);
        let mut counts = Vec::new();
        for lod in 0..=2 {
            let spec = BuildingSpec::new(BuildingKind::Farmhouse, 0.0, 0.0, lod);
            let made = spawn_building(&mut forge, root, usize::from(lod), &spec).unwrap_or(0);
            counts.push(made);
        }
        assert!(counts[0] < counts[1], "LOD1 should add the roof");
        assert!(counts[1] < counts[2], "LOD2 should add detail");
    }

    #[test]
    fn kinds_differ_in_footprint() {
        let church = BuildingSpec::new(BuildingKind::Church, 0.0, 0.0, 2);
        let igloo = BuildingSpec::new(BuildingKind::Igloo, 0.0, 0.0, 2);
        assert!(church.height > igloo.height);
    }
}

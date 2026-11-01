//! Software mesh rendering for the desktop viewport.
//!
//! The scene file already tells us what each `MeshInstance3D` draws: its
//! `mesh = SubResource("id")` points at a primitive declaration with the real
//! dimensions, and `material_override` points at the albedo. So the viewer can
//! build the same geometry the engine does, without loading Godot meshes.
//!
//! Primitive tessellation lives in `gamecraft_gltf::geometry`, shared with the
//! glTF exporter, so the viewer and an exported file contain the same triangles.
//!
//! Rendering is CPU-side: triangles are transformed, back-face culled, flat
//! shaded against one fixed light, sorted far-to-near, and handed to egui as
//! filled polygons. No depth buffer exists in egui's render pass, so the painter's
//! algorithm is doing the depth work.

use egui::Color32;
use gamecraft_core::{NodeId, PropertyValue, SceneGraph};
use gamecraft_godot::SceneFile;
use glam::{Mat4, Vec3};

/// One renderable triangle in world space.
pub struct Tri {
    /// Corner positions.
    pub points: [Vec3; 3],
    /// Shaded fill colour (the tint when the triangle is textured).
    pub color: Color32,
    /// Texture coordinates when the material has a triplanar albedo texture.
    pub uvs: Option<[[f32; 2]; 3]>,
    /// `ExtResource` id of that albedo texture.
    pub texture: Option<String>,
    /// Normal and ORM maps, when the material has any (needs the software rasterizer).
    pub pbr: Option<Pbr>,
}

/// Maps and colours the per-pixel path needs beyond the albedo texture.
#[derive(Debug, Clone)]
pub struct Pbr {
    /// `ExtResource` id of the tangent-space normal map.
    pub normal: Option<String>,
    /// `ExtResource` id of the packed occlusion/roughness/metallic map.
    pub orm: Option<String>,
    /// The material's `albedo_color`, or the node fallback colour (`0..=1`).
    pub base: Vec3,
    /// Per-triangle lighting factor of a white surface, for albedo-only fallback.
    pub lit: Vec3,
}

/// Textured triangles longer than this (scene units) are split, because egui
/// interpolates texture coordinates in screen space, not with perspective.
const MAX_TEXTURED_EDGE: f32 = 1.5;
/// Bisection depth cap: 2^8 triangles from one source triangle at most.
const MAX_SPLIT_DEPTH: u32 = 8;

/// Bisect the longest edge until every edge is short enough (winding is kept).
fn subdivide(points: [Vec3; 3], depth: u32, out: &mut Vec<[Vec3; 3]>) {
    let lengths = [
        points[0].distance(points[1]),
        points[1].distance(points[2]),
        points[2].distance(points[0]),
    ];
    let (longest, length) =
        lengths
            .iter()
            .copied()
            .enumerate()
            .fold(
                (0, 0.0_f32),
                |best, (i, l)| if l > best.1 { (i, l) } else { best },
            );
    if depth == 0 || length <= MAX_TEXTURED_EDGE {
        out.push(points);
        return;
    }
    let (p0, p1, p2) = (
        points[longest],
        points[(longest + 1) % 3],
        points[(longest + 2) % 3],
    );
    let mid = (p0 + p1) * 0.5;
    subdivide([p0, mid, p2], depth - 1, out);
    subdivide([mid, p1, p2], depth - 1, out);
}

/// Fallback key direction for scenes that declare no lights, so a lightless
/// scene is shaded rather than flat.
const FALLBACK_LIGHT: Vec3 = Vec3::new(-0.4, 0.85, 0.35);

/// One light read from the scene.
#[derive(Debug, Clone, Copy)]
pub enum SceneLight {
    /// Directional light (sun): `direction` points away from the light.
    Directional {
        /// Normalised direction of travel.
        direction: Vec3,
        /// Linear colour.
        color: Vec3,
        /// Intensity.
        energy: f32,
    },
    /// Omni light with distance falloff.
    Point {
        /// World position.
        position: Vec3,
        /// Linear colour.
        color: Vec3,
        /// Intensity.
        energy: f32,
        /// Falloff radius.
        range: f32,
    },
}

/// Everything needed to shade one scene.
#[derive(Debug, Clone)]
pub struct Lighting {
    /// Ambient colour (from the environment, or a dim default).
    pub ambient: Vec3,
    /// Lights declared in the scene.
    pub lights: Vec<SceneLight>,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            ambient: Vec3::splat(0.30),
            lights: Vec::new(),
        }
    }
}

/// Normalised colour from a parsed byte colour.
fn linear(color: Color32) -> Vec3 {
    Vec3::new(
        f32::from(color.r()) / 255.0,
        f32::from(color.g()) / 255.0,
        f32::from(color.b()) / 255.0,
    )
}

/// Collect the scene's lights and ambient.
#[must_use]
pub fn collect_lighting(graph: &SceneGraph, source: Option<&SceneFile>) -> Lighting {
    let mut lighting = Lighting::default();
    if let Some(file) = source {
        if let Some(environment) = file
            .sub_resources
            .iter()
            .find(|resource| resource.resource_type == "Environment")
        {
            if let Some(color) = environment
                .properties
                .iter()
                .find(|(key, _)| key == "ambient_light_color")
                .and_then(|(_, value)| parse_color(value))
            {
                lighting.ambient = linear(color);
            }
        }
    }
    let matrices: Vec<(NodeId, Mat4)> = gamecraft_core::spatial::world_matrices(graph);
    for (id, world) in matrices {
        let Some(node) = graph.get(id) else {
            continue;
        };
        let energy = node
            .properties
            .get("light_energy")
            .and_then(PropertyValue::as_str)
            .and_then(|text| text.trim().parse::<f32>().ok())
            .unwrap_or(1.0);
        let color = node
            .properties
            .get("light_color")
            .and_then(PropertyValue::as_str)
            .and_then(parse_color)
            .map_or(Vec3::ONE, linear);
        match node.node_type.as_str() {
            "DirectionalLight3D" => {
                lighting.lights.push(SceneLight::Directional {
                    direction: world.transform_vector3(Vec3::NEG_Z).normalize_or_zero(),
                    color,
                    energy,
                });
            }
            "OmniLight3D" | "SpotLight3D" => {
                let range = node
                    .properties
                    .get("omni_range")
                    .and_then(PropertyValue::as_str)
                    .and_then(|text| text.trim().parse::<f32>().ok())
                    .unwrap_or(8.0);
                lighting.lights.push(SceneLight::Point {
                    position: world.w_axis.truncate(),
                    color,
                    energy,
                    range,
                });
            }
            _ => {}
        }
    }
    lighting
}

/// Fixed colour used when a mesh has no material.
fn fallback_color(node_type: &str) -> Color32 {
    match node_type {
        "CollisionShape3D" => Color32::from_rgb(90, 120, 190),
        "MeshInstance3D" => Color32::from_rgb(150, 155, 165),
        _ => Color32::from_rgb(120, 124, 132),
    }
}

/// Build every triangle the scene will draw.
#[must_use]
pub fn build_scene(graph: &SceneGraph, source: Option<&SceneFile>) -> Vec<Tri> {
    let lighting = collect_lighting(graph, source);
    let mut out: Vec<Tri> = Vec::new();
    let matrices: Vec<(NodeId, Mat4)> = gamecraft_core::spatial::world_matrices(graph);
    for (id, world) in matrices {
        let Some(node) = graph.get(id) else {
            continue;
        };
        let Some(mesh_ref) = node.properties.get("mesh").and_then(PropertyValue::as_str) else {
            continue;
        };
        let Some(mesh_id) = sub_id(mesh_ref) else {
            continue;
        };
        let Some(sub) = source.and_then(|file| {
            file.sub_resources
                .iter()
                .find(|resource| resource.id == mesh_id)
        }) else {
            continue;
        };
        let maps = material_sub(node, source)
            .and_then(|material| gamecraft_gltf::triplanar_maps(&material.properties));
        let textured = maps.as_ref().map(|m| (m.albedo.clone(), m.uv));
        let has_albedo = maps.as_ref().is_some_and(|m| m.albedo.is_some());
        let color = material_color(node, source).unwrap_or_else(|| {
            if has_albedo {
                Color32::WHITE
            } else {
                fallback_color(node.node_type.as_str())
            }
        });
        let local =
            gamecraft_gltf::geometry::shape_triangles(sub.resource_type.as_str(), &sub.properties);
        for (points, normal) in local.iter().filter_map(gamecraft_gltf::geometry::oriented) {
            let mut pieces: Vec<[Vec3; 3]> = Vec::new();
            if textured.is_some() {
                subdivide(points, MAX_SPLIT_DEPTH, &mut pieces);
            } else {
                pieces.push(points);
            }
            // Transform the normal by the rotation part only.
            let world_normal = world.transform_vector3(normal).normalize_or_zero();
            for piece in pieces {
                let world_points = [
                    world.transform_point3(piece[0]),
                    world.transform_point3(piece[1]),
                    world.transform_point3(piece[2]),
                ];
                let centre = (world_points[0] + world_points[1] + world_points[2]) / 3.0;
                let pbr = maps
                    .as_ref()
                    .filter(|m| m.normal.is_some() || m.orm.is_some())
                    .map(|m| Pbr {
                        normal: m.normal.clone(),
                        orm: m.orm.clone(),
                        base: linear(color),
                        lit: linear(shade(Color32::WHITE, world_normal, centre, &lighting)),
                    });
                out.push(Tri {
                    points: world_points,
                    color: shade(color, world_normal, centre, &lighting),
                    uvs: textured
                        .as_ref()
                        .map(|(_, uv)| gamecraft_gltf::triplanar_uvs(&piece, normal, *uv)),
                    texture: textured.as_ref().and_then(|(id, _)| id.clone()),
                    pbr,
                });
            }
        }
    }
    out
}

/// A surface point to shade per pixel.
#[derive(Debug, Clone, Copy)]
pub struct Surface {
    /// Albedo (`0..=1`).
    pub albedo: Vec3,
    /// Unit shading normal.
    pub normal: Vec3,
    /// World position.
    pub position: Vec3,
    /// Unit direction from the point to the eye.
    pub view: Vec3,
    /// Ambient occlusion (`1` = open).
    pub ao: f32,
    /// Roughness (`0` mirror, `1` matte).
    pub roughness: f32,
    /// Metallic (`0` dielectric, `1` metal).
    pub metallic: f32,
}

/// Per-pixel colour (`0..=1`) of a surface under the scene lights.
///
/// Lambert diffuse scaled by `(1 - metallic)`, Blinn-Phong specular whose exponent comes
/// from the roughness and whose colour blends a 4% dielectric reflectance toward the
/// albedo with the metallic value, a broad `roughness^2` lobe of that same colour (a rough
/// metal is about as bright as a diffuse surface in three.js, measured headless, so it must
/// not go black), and ambient scaled by the occlusion. A preview approximation, not
/// Godot's PBR.
#[must_use]
pub fn shade_surface(s: &Surface, lighting: &Lighting) -> Vec3 {
    let diffuse = s.albedo * (1.0 - s.metallic);
    let spec_color = Vec3::splat(0.04).lerp(s.albedo, s.metallic);
    let roughness = s.roughness.clamp(0.05, 1.0);
    let alpha = roughness * roughness;
    let shininess = (2.0 / (alpha * alpha + 1e-4) - 2.0).clamp(2.0, 512.0);
    let norm = (shininess + 8.0) / (8.0 * std::f32::consts::PI);
    let broad = alpha;
    let mut lit = s.albedo * lighting.ambient * s.ao;
    if lighting.lights.is_empty() {
        lit += s.albedo * 0.15 * s.ao;
    }
    let mut add = |to_light: Vec3, radiance: Vec3| {
        let lambert = s.normal.dot(to_light).max(0.0);
        if lambert <= 0.0 {
            return;
        }
        let half = (to_light + s.view).normalize_or_zero();
        let spec = s.normal.dot(half).max(0.0).powf(shininess) * norm;
        lit += (diffuse + spec_color * (spec + broad)) * radiance * lambert;
    };
    if lighting.lights.is_empty() {
        add(FALLBACK_LIGHT.normalize(), Vec3::splat(0.75));
    }
    for light in &lighting.lights {
        match light {
            SceneLight::Directional {
                direction,
                color,
                energy,
            } => add(-*direction, *color * *energy),
            SceneLight::Point {
                position,
                color,
                energy,
                range,
            } => {
                let to_light = *position - s.position;
                let distance = to_light.length();
                if distance > *range || distance < f32::EPSILON {
                    continue;
                }
                let attenuation = (1.0 - distance / *range).max(0.0).powi(2);
                add(to_light / distance, *color * (*energy * attenuation));
            }
        }
    }
    lit.min(Vec3::ONE)
}

/// Extract `id` from `SubResource("id")` / `ExtResource("id")`.
fn sub_id(reference: &str) -> Option<&str> {
    let start = reference.find('"')? + 1;
    let end = reference.rfind('"')?;
    if end <= start {
        return None;
    }
    Some(&reference[start..end])
}

/// The material sub-resource a node's `material_override` points at.
fn material_sub<'a>(
    node: &gamecraft_core::SceneNode,
    source: Option<&'a SceneFile>,
) -> Option<&'a gamecraft_godot::SubResource> {
    let material_ref = node
        .properties
        .get("material_override")
        .and_then(PropertyValue::as_str)?;
    let material_id = sub_id(material_ref)?;
    source?
        .sub_resources
        .iter()
        .find(|resource| resource.id == material_id)
}

/// Resolve a node's material colour from its `material_override`.
fn material_color(node: &gamecraft_core::SceneNode, source: Option<&SceneFile>) -> Option<Color32> {
    let material = material_sub(node, source)?;
    let albedo = material
        .properties
        .iter()
        .find(|(key, _)| key == "albedo_color")?
        .1
        .clone();
    parse_color(albedo.as_str())
}

/// Parse `Color(r, g, b, a)`.
fn parse_color(text: &str) -> Option<Color32> {
    let inner = text.strip_prefix("Color(")?.strip_suffix(')')?;
    let mut parts = inner.split(',');
    let red = parts.next()?.trim().parse::<f32>().ok()?;
    let green = parts.next()?.trim().parse::<f32>().ok()?;
    let blue = parts.next()?.trim().parse::<f32>().ok()?;
    Some(Color32::from_rgb(
        to_byte(red),
        to_byte(green),
        to_byte(blue),
    ))
}

/// `0.0..=1.0` to `0..=255`.
///
/// The value is clamped before narrowing, so the cast cannot truncate anything
/// outside the byte range; `try_from(f32)` does not exist.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn shade(base: Color32, normal: Vec3, position: Vec3, lighting: &Lighting) -> Color32 {
    let albedo = linear(base);
    let mut lit = albedo * lighting.ambient;
    if lighting.lights.is_empty() {
        let lambert = normal.dot(FALLBACK_LIGHT.normalize()).max(0.0);
        lit += albedo * (0.15 + 0.75 * lambert);
    }
    for light in &lighting.lights {
        match light {
            SceneLight::Directional {
                direction,
                color,
                energy,
            } => {
                let lambert = normal.dot(-*direction).max(0.0);
                lit += albedo * *color * (lambert * energy);
            }
            SceneLight::Point {
                position: light_position,
                color,
                energy,
                range,
            } => {
                let to_light = *light_position - position;
                let distance = to_light.length();
                if distance > *range || distance < f32::EPSILON {
                    continue;
                }
                let attenuation = (1.0 - distance / *range).max(0.0).powi(2);
                let lambert = normal.dot(to_light / distance).max(0.0);
                lit += albedo * *color * (lambert * energy * attenuation);
            }
        }
    }
    Color32::from_rgb(
        to_byte(lit.x.min(1.0)),
        to_byte(lit.y.min(1.0)),
        to_byte(lit.z.min(1.0)),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn color_parsing_reads_godot_literals() {
        let parsed = parse_color("Color(0.55, 0.36, 0.18, 1)");
        assert_eq!(parsed, Some(Color32::from_rgb(140, 92, 46)));
    }

    #[test]
    fn references_resolve_to_ids() {
        assert_eq!(sub_id("SubResource(\"box_shape\")"), Some("box_shape"));
        assert_eq!(sub_id("ExtResource(\"1_a\")"), Some("1_a"));
        assert_eq!(sub_id("nonsense"), None);
    }

    const GROUND: &str = r#"[gd_scene load_steps=4 format=3]

[ext_resource type="Texture2D" path="res://t/a.png" id="1_a"]

[sub_resource type="BoxMesh" id="slab"]
size = Vector3(20, 0.2, 20)

[sub_resource type="StandardMaterial3D" id="flat"]
albedo_color = Color(0.5, 0.5, 0.5, 1)

[sub_resource type="ORMMaterial3D" id="stone"]
uv1_scale = Vector3(0.25, 0.25, 0.25)
uv1_triplanar = true
albedo_texture = ExtResource("1_a")

[node name="World" type="Node3D"]

[node name="Ground" type="MeshInstance3D" parent="."]
mesh = SubResource("slab")
material_override = SubResource("MATERIAL")
"#;

    fn build(material: &str) -> Vec<Tri> {
        let text = GROUND.replace("MATERIAL", material);
        let file = gamecraft_godot::parse_tscn(&text).expect("parse");
        let graph = gamecraft_godot::build_graph(&file).expect("graph");
        build_scene(&graph, Some(&file))
    }

    #[test]
    fn triplanar_material_gives_textured_subdivided_triangles() {
        let flat = build("flat");
        let textured = build("stone");
        assert!(flat
            .iter()
            .all(|tri| tri.uvs.is_none() && tri.texture.is_none()));
        assert!(!textured.is_empty());
        assert!(
            textured.len() > flat.len() * 8,
            "{} vs {}",
            textured.len(),
            flat.len()
        );
        for tri in &textured {
            assert_eq!(tri.texture.as_deref(), Some("1_a"));
            assert!(tri.uvs.is_some());
        }
    }

    #[test]
    fn subdivision_keeps_edges_short_and_area_unchanged() {
        let big = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(20.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 20.0),
        ];
        let mut pieces = Vec::new();
        subdivide(big, MAX_SPLIT_DEPTH, &mut pieces);
        let area = |p: &[Vec3; 3]| (p[1] - p[0]).cross(p[2] - p[0]).length() * 0.5;
        let total: f32 = pieces.iter().map(area).sum();
        assert!((total - 200.0).abs() < 0.01, "{total}");
        for piece in &pieces {
            // winding preserved: still facing +Y or -Y consistently with the source
            let source = (big[1] - big[0]).cross(big[2] - big[0]);
            let normal = (piece[1] - piece[0]).cross(piece[2] - piece[0]);
            assert!(normal.dot(source) > 0.0);
        }
        assert!(pieces.len() <= 1 << MAX_SPLIT_DEPTH);
    }

    #[test]
    fn small_triangles_are_not_split() {
        let mut pieces = Vec::new();
        subdivide(
            [
                Vec3::ZERO,
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            MAX_SPLIT_DEPTH,
            &mut pieces,
        );
        assert_eq!(pieces.len(), 1);
    }
}

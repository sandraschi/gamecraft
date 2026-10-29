//! Software mesh rendering for the desktop viewport.
//!
//! The scene file already tells us what each `MeshInstance3D` draws: its
//! `mesh = SubResource("id")` points at a primitive declaration with the real
//! dimensions, and `material_override` points at the albedo. So the viewer can
//! build the same geometry the engine does, without loading Godot meshes.
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
    /// Shaded fill colour.
    pub color: Color32,
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
        let color =
            material_color(node, source).unwrap_or_else(|| fallback_color(node.node_type.as_str()));
        let local = shape_triangles(sub.resource_type.as_str(), &sub.properties);
        for (points, normal) in local {
            let world_points = [
                world.transform_point3(points[0]),
                world.transform_point3(points[1]),
                world.transform_point3(points[2]),
            ];
            // Transform the normal by the rotation part only.
            let world_normal = world.transform_vector3(normal).normalize_or_zero();
            out.push(Tri {
                points: world_points,
                color: shade(
                    color,
                    world_normal,
                    (world_points[0] + world_points[1] + world_points[2]) / 3.0,
                    &lighting,
                ),
            });
        }
    }
    out
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

/// Resolve a node's material colour from its `material_override`.
fn material_color(node: &gamecraft_core::SceneNode, source: Option<&SceneFile>) -> Option<Color32> {
    let material_ref = node
        .properties
        .get("material_override")
        .and_then(PropertyValue::as_str)?;
    let material_id = sub_id(material_ref)?;
    let file = source?;
    let material = file
        .sub_resources
        .iter()
        .find(|resource| resource.id == material_id)?;
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

/// Read a `Vector3(...)` property.
fn vector3(properties: &[(String, String)], key: &str) -> Option<Vec3> {
    let text = properties.iter().find(|(name, _)| name == key)?.1.as_str();
    let inner = text.strip_prefix("Vector3(")?.strip_suffix(')')?;
    let mut parts = inner.split(',');
    let x = parts.next()?.trim().parse::<f32>().ok()?;
    let y = parts.next()?.trim().parse::<f32>().ok()?;
    let z = parts.next()?.trim().parse::<f32>().ok()?;
    Some(Vec3::new(x, y, z))
}

/// Read a scalar property.
fn scalar(properties: &[(String, String)], key: &str, fallback: f32) -> f32 {
    properties
        .iter()
        .find(|(name, _)| name == key)
        .and_then(|(_, value)| value.trim().parse::<f32>().ok())
        .unwrap_or(fallback)
}

/// Small counts to float without a lossy cast.
fn fi(value: i32) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(0))
}

/// Local-space triangles `(points, normal)` for one primitive type.
fn shape_triangles(kind: &str, properties: &[(String, String)]) -> Vec<([Vec3; 3], Vec3)> {
    match kind {
        "SphereMesh" => sphere(
            scalar(properties, "radius", 0.5),
            scalar(properties, "height", 1.0),
        ),
        "CapsuleMesh" => capsule(
            scalar(properties, "radius", 0.3),
            scalar(properties, "height", 1.0),
        ),
        "CylinderMesh" => cylinder(
            scalar(properties, "top_radius", 0.5),
            scalar(properties, "bottom_radius", 0.5),
            scalar(properties, "height", 1.0),
        ),
        "PrismMesh" => prism(vector3(properties, "size").unwrap_or(Vec3::ONE)),
        "TorusMesh" => torus(
            scalar(properties, "inner_radius", 0.5),
            scalar(properties, "outer_radius", 1.0),
        ),
        _ => cuboid(vector3(properties, "size").unwrap_or(Vec3::ONE)),
    }
}

/// Axis-aligned box.
fn cuboid(size: Vec3) -> Vec<([Vec3; 3], Vec3)> {
    let half = size * 0.5;
    let corner = |x: f32, y: f32, z: f32| Vec3::new(x * half.x, y * half.y, z * half.z);
    let c = [
        corner(-1.0, -1.0, -1.0),
        corner(1.0, -1.0, -1.0),
        corner(-1.0, 1.0, -1.0),
        corner(1.0, 1.0, -1.0),
        corner(-1.0, -1.0, 1.0),
        corner(1.0, -1.0, 1.0),
        corner(-1.0, 1.0, 1.0),
        corner(1.0, 1.0, 1.0),
    ];
    let faces = [
        ([0, 1, 3, 2], Vec3::NEG_Z),
        ([4, 6, 7, 5], Vec3::Z),
        ([0, 4, 5, 1], Vec3::NEG_Y),
        ([2, 3, 7, 6], Vec3::Y),
        ([0, 2, 6, 4], Vec3::NEG_X),
        ([1, 5, 7, 3], Vec3::X),
    ];
    let mut out = Vec::with_capacity(12);
    for (quad, normal) in faces {
        out.push(([c[quad[0]], c[quad[1]], c[quad[2]]], normal));
        out.push(([c[quad[0]], c[quad[2]], c[quad[3]]], normal));
    }
    out
}

/// Latitude/longitude sphere, low tessellation on purpose.
#[allow(clippy::many_single_char_names)]
fn sphere(radius: f32, height: f32) -> Vec<([Vec3; 3], Vec3)> {
    let sy = height * 0.5;
    let mut out: Vec<([Vec3; 3], Vec3)> = Vec::new();
    let rings = 8_i32;
    let segments = 14_i32;
    for ring in 0..rings {
        let phi0 = std::f32::consts::PI * fi(ring) / fi(rings);
        let phi1 = std::f32::consts::PI * (fi(ring) + 1.0) / fi(rings);
        for segment in 0..segments {
            let theta0 = std::f32::consts::TAU * fi(segment) / fi(segments);
            let theta1 = std::f32::consts::TAU * (fi(segment) + 1.0) / fi(segments);
            let p = |phi: f32, theta: f32| {
                Vec3::new(
                    phi.sin() * theta.cos(),
                    phi.cos() * height / 2.0,
                    phi.sin() * theta.sin(),
                ) * Vec3::new(radius, 1.0, radius)
            };
            let a = p(phi0, theta0);
            let b = p(phi0, theta1);
            let c = p(phi1, theta0);
            let d = p(phi1, theta1);
            out.push(([a, b, d], a.normalize_or_zero()));
            out.push(([a, d, c], a.normalize_or_zero()));
        }
    }
    let _ = sy;
    out
}

/// Cylinder with caps.
fn cylinder(top: f32, bottom: f32, height: f32) -> Vec<([Vec3; 3], Vec3)> {
    let segments = 14_i32;
    let half = height * 0.5;
    let mut out: Vec<([Vec3; 3], Vec3)> = Vec::new();
    for segment in 0..segments {
        let a0 = std::f32::consts::TAU * fi(segment) / fi(segments);
        let a1 = std::f32::consts::TAU * (fi(segment) + 1.0) / fi(segments);
        let bottom0 = Vec3::new(bottom * a0.cos(), -half, bottom * a0.sin());
        let bottom1 = Vec3::new(bottom * a1.cos(), -half, bottom * a1.sin());
        let top0 = Vec3::new(top * a0.cos(), half, top * a0.sin());
        let top1 = Vec3::new(top * a1.cos(), half, top * a1.sin());
        let normal = Vec3::new(a0.cos(), 0.0, a0.sin());
        out.push(([bottom0, bottom1, top1], normal));
        out.push(([bottom0, top1, top0], normal));
        out.push(([Vec3::new(0.0, half, 0.0), top0, top1], Vec3::Y));
        out.push(([Vec3::new(0.0, -half, 0.0), bottom1, bottom0], Vec3::NEG_Y));
    }
    out
}

/// Capsule approximated as a cylinder with rounded ends.
fn capsule(radius: f32, height: f32) -> Vec<([Vec3; 3], Vec3)> {
    let mut out = cylinder(radius, radius, (height - radius * 2.0).max(0.05));
    let cap = sphere(radius, radius * 2.0);
    for (points, normal) in cap {
        let lifted = points.map(|point| point + Vec3::new(0.0, height * 0.5 - radius, 0.0));
        out.push((lifted, normal));
    }
    out
}

/// Triangular prism (Godot's `PrismMesh`).
///
/// `a`..`f` are the six corners; naming each after its coordinates would be
/// longer and no clearer.
#[allow(clippy::many_single_char_names)]
fn prism(size: Vec3) -> Vec<([Vec3; 3], Vec3)> {
    let (x, y, z) = (size.x * 0.5, size.y * 0.5, size.z * 0.5);
    let a = Vec3::new(-x, -y, -z);
    let b = Vec3::new(x, -y, -z);
    let c = Vec3::new(-x, y, -z);
    let d = Vec3::new(-x, -y, z);
    let e = Vec3::new(x, -y, z);
    let f = Vec3::new(-x, y, z);
    vec![
        ([a, b, c], Vec3::NEG_Z),
        ([d, f, e], Vec3::Z),
        ([a, c, f], Vec3::NEG_X),
        ([a, f, d], Vec3::NEG_X),
        ([b, e, f], Vec3::new(0.6, 0.5, 0.0)),
        ([b, f, c], Vec3::new(0.6, 0.5, 0.0)),
        ([a, d, e], Vec3::NEG_Y),
        ([a, e, b], Vec3::NEG_Y),
    ]
}

/// Torus in the XZ plane.
fn torus(inner: f32, outer: f32) -> Vec<([Vec3; 3], Vec3)> {
    let tube = (outer - inner) * 0.5;
    let ring_radius = inner + tube;
    let rings = 18_i32;
    let sides = 8_i32;
    let mut out: Vec<([Vec3; 3], Vec3)> = Vec::new();
    let point = |ring: f32, side: f32| {
        let outer_angle = std::f32::consts::TAU * ring / fi(rings);
        let tube_angle = std::f32::consts::TAU * side / fi(sides);
        let r = ring_radius + tube * tube_angle.cos();
        Vec3::new(
            r * outer_angle.cos(),
            tube * tube_angle.sin(),
            r * outer_angle.sin(),
        )
    };
    for ring in 0..rings {
        for side in 0..sides {
            let a = point(fi(ring), fi(side));
            let b = point(fi(ring) + 1.0, fi(side));
            let c = point(fi(ring), fi(side) + 1.0);
            let d = point(fi(ring) + 1.0, fi(side) + 1.0);
            out.push(([a, b, d], (a + b + d).normalize_or_zero()));
            out.push(([a, d, c], (a + d + c).normalize_or_zero()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_produce_triangles() {
        assert_eq!(cuboid(Vec3::ONE).len(), 12);
        assert_ne!(sphere(0.5, 1.0).len(), 0);
        assert_ne!(cylinder(0.5, 0.5, 1.0).len(), 0);
        assert_ne!(prism(Vec3::ONE).len(), 0);
        assert_ne!(torus(1.0, 2.0).len(), 0);
    }

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
}

//! Godot primitive meshes as triangles: `BoxMesh`, `SphereMesh`, `CapsuleMesh`,
//! `CylinderMesh`, `PrismMesh`, `TorusMesh`.
//!
//! One source for the desktop viewport and the glTF exporter, so what the viewer
//! shows and what a file contains are the same triangles. Each triangle carries an
//! approximate outward normal; [`oriented`] turns that into consistent
//! counter-clockwise winding and an exact face normal.

use glam::Vec3;

/// One local-space triangle and its approximate outward normal.
pub type Tri = ([Vec3; 3], Vec3);

/// Read a `Vector3(...)` property.
#[must_use]
pub fn vector3(properties: &[(String, String)], key: &str) -> Option<Vec3> {
    let text = properties.iter().find(|(name, _)| name == key)?.1.as_str();
    let inner = text.strip_prefix("Vector3(")?.strip_suffix(')')?;
    let mut parts = inner.split(',');
    let x = parts.next()?.trim().parse::<f32>().ok()?;
    let y = parts.next()?.trim().parse::<f32>().ok()?;
    let z = parts.next()?.trim().parse::<f32>().ok()?;
    Some(Vec3::new(x, y, z))
}

/// Read a scalar property, or `fallback`.
#[must_use]
pub fn scalar(properties: &[(String, String)], key: &str, fallback: f32) -> f32 {
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

/// Is this sub-resource type a primitive mesh we can tessellate?
#[must_use]
pub fn is_primitive_mesh(kind: &str) -> bool {
    matches!(
        kind,
        "BoxMesh" | "SphereMesh" | "CapsuleMesh" | "CylinderMesh" | "PrismMesh" | "TorusMesh"
    )
}

/// Local-space triangles for one primitive type (unknown types become a unit box).
#[must_use]
pub fn shape_triangles(kind: &str, properties: &[(String, String)]) -> Vec<Tri> {
    match kind {
        "SphereMesh" => sphere(
            scalar(properties, "radius", 0.5),
            scalar(properties, "height", 1.0),
        ),
        "CapsuleMesh" => capsule(
            scalar(properties, "radius", 0.5),
            scalar(properties, "height", 2.0),
        ),
        "CylinderMesh" => cylinder(
            scalar(properties, "top_radius", 0.5),
            scalar(properties, "bottom_radius", 0.5),
            scalar(properties, "height", 2.0),
        ),
        "PrismMesh" => prism(
            vector3(properties, "size").unwrap_or(Vec3::ONE),
            scalar(properties, "left_to_right", 0.5),
        ),
        "TorusMesh" => torus(
            scalar(properties, "inner_radius", 0.5),
            scalar(properties, "outer_radius", 1.0),
        ),
        _ => cuboid(vector3(properties, "size").unwrap_or(Vec3::ONE)),
    }
}

/// Counter-clockwise (seen from outside) corners and the exact face normal, or
/// `None` for a degenerate triangle (the poles of a lat/long sphere make some).
#[must_use]
pub fn oriented(tri: &Tri) -> Option<([Vec3; 3], Vec3)> {
    let ([a, b, c], hint) = *tri;
    let cross = (b - a).cross(c - a);
    if cross.length_squared() < 1e-12 {
        return None;
    }
    let normal = cross.normalize();
    if normal.dot(hint) < 0.0 {
        Some(([a, c, b], -normal))
    } else {
        Some(([a, b, c], normal))
    }
}

/// Axis-aligned box centred on the origin.
#[must_use]
pub fn cuboid(size: Vec3) -> Vec<Tri> {
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

/// Latitude/longitude ellipsoid, low tessellation on purpose.
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn sphere(radius: f32, height: f32) -> Vec<Tri> {
    let mut out: Vec<Tri> = Vec::new();
    let rings = 8_i32;
    let segments = 14_i32;
    let p = |phi: f32, theta: f32| {
        Vec3::new(
            radius * phi.sin() * theta.cos(),
            phi.cos() * height / 2.0,
            radius * phi.sin() * theta.sin(),
        )
    };
    for ring in 0..rings {
        let phi0 = std::f32::consts::PI * fi(ring) / fi(rings);
        let phi1 = std::f32::consts::PI * (fi(ring) + 1.0) / fi(rings);
        for segment in 0..segments {
            let theta0 = std::f32::consts::TAU * fi(segment) / fi(segments);
            let theta1 = std::f32::consts::TAU * (fi(segment) + 1.0) / fi(segments);
            let a = p(phi0, theta0);
            let b = p(phi0, theta1);
            let c = p(phi1, theta0);
            let d = p(phi1, theta1);
            let hint = (a + b + c + d).normalize_or_zero();
            out.push(([a, b, d], hint));
            out.push(([a, d, c], hint));
        }
    }
    out
}

/// Cylinder (or cone) with caps.
#[must_use]
pub fn cylinder(top: f32, bottom: f32, height: f32) -> Vec<Tri> {
    let segments = 14_i32;
    let half = height * 0.5;
    let mut out: Vec<Tri> = Vec::new();
    for segment in 0..segments {
        let a0 = std::f32::consts::TAU * fi(segment) / fi(segments);
        let a1 = std::f32::consts::TAU * (fi(segment) + 1.0) / fi(segments);
        let bottom0 = Vec3::new(bottom * a0.cos(), -half, bottom * a0.sin());
        let bottom1 = Vec3::new(bottom * a1.cos(), -half, bottom * a1.sin());
        let top0 = Vec3::new(top * a0.cos(), half, top * a0.sin());
        let top1 = Vec3::new(top * a1.cos(), half, top * a1.sin());
        let mid = f32::midpoint(a0, a1);
        let normal = Vec3::new(mid.cos(), 0.0, mid.sin());
        out.push(([bottom0, bottom1, top1], normal));
        out.push(([bottom0, top1, top0], normal));
        out.push(([Vec3::new(0.0, half, 0.0), top0, top1], Vec3::Y));
        out.push(([Vec3::new(0.0, -half, 0.0), bottom1, bottom0], Vec3::NEG_Y));
    }
    out
}

/// Capsule: a cylinder with a hemisphere-ish cap at each end. `height` is the
/// full height including the caps, as in Godot 4.
#[must_use]
pub fn capsule(radius: f32, height: f32) -> Vec<Tri> {
    let body = (height - radius * 2.0).max(0.05);
    let mut out = cylinder(radius, radius, body);
    let lift = body * 0.5;
    for (points, normal) in sphere(radius, radius * 2.0) {
        let centre = (points[0] + points[1] + points[2]) / 3.0;
        // Upper half of the sphere goes on top, lower half on the bottom.
        let offset = if centre.y >= 0.0 { lift } else { -lift };
        out.push((points.map(|p| p + Vec3::new(0.0, offset, 0.0)), normal));
    }
    out
}

/// Triangular prism (Godot's `PrismMesh`). `left_to_right` places the apex:
/// 0 = over the left edge, 0.5 = centred (Godot's default), 1 = right edge.
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn prism(size: Vec3, left_to_right: f32) -> Vec<Tri> {
    let (x, y, z) = (size.x * 0.5, size.y * 0.5, size.z * 0.5);
    let apex = -x + size.x * left_to_right.clamp(0.0, 1.0);
    let a = Vec3::new(-x, -y, -z);
    let b = Vec3::new(x, -y, -z);
    let c = Vec3::new(apex, y, -z);
    let d = Vec3::new(-x, -y, z);
    let e = Vec3::new(x, -y, z);
    let f = Vec3::new(apex, y, z);
    // Outward normals of the two sloped faces, perpendicular to base-to-apex edges.
    let left = Vec3::new(-2.0 * y, apex + x, 0.0).normalize_or_zero();
    let right = Vec3::new(2.0 * y, x - apex, 0.0).normalize_or_zero();
    vec![
        ([a, b, c], Vec3::NEG_Z),
        ([d, f, e], Vec3::Z),
        ([a, c, f], left),
        ([a, f, d], left),
        ([b, e, f], right),
        ([b, f, c], right),
        ([a, d, e], Vec3::NEG_Y),
        ([a, e, b], Vec3::NEG_Y),
    ]
}

/// Torus in the XZ plane. Normals point away from the tube's centre line, so the
/// inner surface faces the hole (pointing away from the origin would be wrong
/// there).
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn torus(inner: f32, outer: f32) -> Vec<Tri> {
    let tube = (outer - inner) * 0.5;
    let ring_radius = inner + tube;
    let rings = 18_i32;
    let sides = 8_i32;
    let mut out: Vec<Tri> = Vec::new();
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
    let centre_line = |ring: f32| {
        let angle = std::f32::consts::TAU * (ring + 0.5) / fi(rings);
        Vec3::new(ring_radius * angle.cos(), 0.0, ring_radius * angle.sin())
    };
    for ring in 0..rings {
        for side in 0..sides {
            let a = point(fi(ring), fi(side));
            let b = point(fi(ring) + 1.0, fi(side));
            let c = point(fi(ring), fi(side) + 1.0);
            let d = point(fi(ring) + 1.0, fi(side) + 1.0);
            let hint = ((a + b + c + d) / 4.0 - centre_line(fi(ring))).normalize_or_zero();
            out.push(([a, b, d], hint));
            out.push(([a, d, c], hint));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every oriented triangle of a closed convex-ish primitive faces away from
    /// its centre (for the torus: away from the tube centre line).
    fn faces_out(tris: &[Tri]) -> bool {
        tris.iter()
            .filter_map(oriented)
            .all(|(points, normal)| normal.dot((points[0] + points[1] + points[2]) / 3.0) > -1e-4)
    }

    #[test]
    fn primitives_produce_triangles() {
        assert_eq!(cuboid(Vec3::ONE).len(), 12);
        assert_ne!(sphere(0.5, 1.0).len(), 0);
        assert_ne!(cylinder(0.5, 0.5, 1.0).len(), 0);
        assert_ne!(prism(Vec3::ONE, 0.5).len(), 0);
        assert_ne!(torus(1.0, 2.0).len(), 0);
    }

    #[test]
    fn convex_primitives_wind_outward() {
        assert!(faces_out(&cuboid(Vec3::new(2.0, 1.0, 3.0))));
        assert!(faces_out(&sphere(0.5, 1.0)));
        assert!(faces_out(&cylinder(0.5, 0.5, 2.0)));
        assert!(faces_out(&capsule(0.5, 2.0)));
        assert!(faces_out(&prism(Vec3::ONE, 0.5)));
        assert!(faces_out(&prism(Vec3::new(2.0, 1.0, 1.0), 0.0)));
    }

    #[test]
    fn torus_inner_surface_faces_the_hole() {
        let inner: Vec<([Vec3; 3], Vec3)> = torus(1.0, 2.0)
            .iter()
            .filter_map(oriented)
            .filter(|(points, _)| {
                let c = (points[0] + points[1] + points[2]) / 3.0;
                c.y.abs() < 0.25 && Vec3::new(c.x, 0.0, c.z).length() < 1.2
            })
            .collect();
        assert_ne!(inner.len(), 0);
        assert!(inner.iter().all(|(points, normal)| {
            let c = (points[0] + points[1] + points[2]) / 3.0;
            normal.dot(Vec3::new(c.x, 0.0, c.z)) < 0.0
        }));
    }

    #[test]
    fn capsule_has_both_caps() {
        let tris = capsule(0.5, 2.0);
        let top = tris.iter().any(|(p, _)| p.iter().any(|v| v.y > 0.95));
        let bottom = tris.iter().any(|(p, _)| p.iter().any(|v| v.y < -0.95));
        assert!(top && bottom);
    }

    #[test]
    fn degenerate_triangles_are_dropped() {
        let flat: Tri = ([Vec3::ZERO, Vec3::X, Vec3::X * 2.0], Vec3::Y);
        assert!(oriented(&flat).is_none());
    }
}

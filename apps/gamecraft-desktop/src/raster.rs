//! Software rasterizer for surfaces the egui painter cannot shade.
//!
//! egui draws triangles with no depth buffer, no perspective-correct texture
//! coordinates and no per-pixel lighting, so a normal map or an ORM map cannot show
//! up through it. When the scene contains such a material the viewport renders its
//! mesh triangles here instead: a depth-buffered, perspective-correct rasterizer
//! that shades each pixel from the albedo, normal and ORM maps with the same lights
//! as the flat path (`mesh::shade_surface`). The result is one image per frame.
//!
//! This is a preview, not Godot's renderer: Blinn-Phong specular driven by the ORM
//! roughness and metallic channels, no shadows, no aliasing smoothing, one mip level
//! picked per pixel.
#![allow(
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::sync::Arc;

use glam::Vec3;
use image::{imageops, RgbaImage};

use crate::mesh::{shade_surface, Lighting, Surface};

/// A texture with its mip chain.
pub struct Mips {
    levels: Vec<RgbaImage>,
}

impl Mips {
    /// Build the chain by halving with a triangle filter.
    #[must_use]
    pub fn new(base: RgbaImage) -> Self {
        let mut levels = vec![base];
        while levels.len() < 12 {
            let Some(last) = levels.last() else { break };
            let (w, h) = last.dimensions();
            if w <= 1 && h <= 1 {
                break;
            }
            let next = imageops::resize(
                last,
                (w / 2).max(1),
                (h / 2).max(1),
                imageops::FilterType::Triangle,
            );
            levels.push(next);
        }
        Self { levels }
    }

    /// Base level width in texels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.levels.first().map_or(1, RgbaImage::width)
    }

    /// Bilinear sample with repeat wrapping at a mip level; channels in `0..=1`.
    #[must_use]
    pub fn sample(&self, u: f32, v: f32, lod: f32) -> [f32; 4] {
        let level = (lod.max(0.0).round() as usize).min(self.levels.len() - 1);
        let image = &self.levels[level];
        let (w, h) = image.dimensions();
        let x = u * w as f32 - 0.5;
        let y = v * h as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let wrap = |value: f32, size: u32| (value as i64).rem_euclid(i64::from(size)) as u32;
        let texel = |dx: f32, dy: f32| image.get_pixel(wrap(x0 + dx, w), wrap(y0 + dy, h)).0;
        let corners = [
            texel(0.0, 0.0),
            texel(1.0, 0.0),
            texel(0.0, 1.0),
            texel(1.0, 1.0),
        ];
        let mut out = [0.0_f32; 4];
        for (channel, slot) in out.iter_mut().enumerate() {
            let c = |i: usize| f32::from(corners[i][channel]) / 255.0;
            let top = c(0) * (1.0 - fx) + c(1) * fx;
            let bottom = c(2) * (1.0 - fx) + c(3) * fx;
            *slot = top * (1.0 - fy) + bottom * fy;
        }
        out
    }
}

/// Maps and base colour of one material.
pub struct Material {
    /// Albedo map.
    pub albedo: Option<Arc<Mips>>,
    /// Tangent-space normal map (green up, as Godot expects).
    pub normal: Option<Arc<Mips>>,
    /// Packed occlusion (R), roughness (G), metallic (B).
    pub orm: Option<Arc<Mips>>,
    /// `albedo_color`, multiplied into the albedo map.
    pub base: Vec3,
}

/// One triangle to draw.
pub struct RasterTri {
    /// World-space corners.
    pub points: [Vec3; 3],
    /// Per-corner texture coordinates, when textured.
    pub uvs: Option<[[f32; 2]; 3]>,
    /// Index into the material list; `None` draws `flat` as is.
    pub material: Option<usize>,
    /// Flat lit colour (`0..=1`): the whole colour without a material, the lighting
    /// factor when a material has an albedo map but no normal or ORM map.
    pub flat: Vec3,
}

/// Camera for one render.
pub struct View {
    /// Eye position.
    pub eye: Vec3,
    /// View direction.
    pub fwd: Vec3,
    /// Right vector.
    pub right: Vec3,
    /// Up vector.
    pub up: Vec3,
    /// Focal length in render pixels.
    pub focal: f32,
    /// Render width in pixels.
    pub width: usize,
    /// Render height in pixels.
    pub height: usize,
}

/// A triangle projected into render pixels.
struct Screen {
    x: [f32; 3],
    y: [f32; 3],
    z: [f32; 3],
    area: f32,
}

impl Screen {
    fn new(view: &View, points: &[Vec3; 3]) -> Option<Self> {
        let mut screen = Self {
            x: [0.0; 3],
            y: [0.0; 3],
            z: [0.0; 3],
            area: 0.0,
        };
        for (i, point) in points.iter().enumerate() {
            let rel = *point - view.eye;
            let depth = rel.dot(view.fwd);
            if depth < 0.1 {
                return None;
            }
            screen.z[i] = depth;
            screen.x[i] = view.width as f32 * 0.5 + view.focal * rel.dot(view.right) / depth;
            screen.y[i] = view.height as f32 * 0.5 - view.focal * rel.dot(view.up) / depth;
        }
        screen.area = edge(&screen, 0, 1, screen.x[2], screen.y[2]);
        (screen.area.abs() > 1e-6).then_some(screen)
    }

    /// Perspective-correct weights `(w0, w1, w2)` at a pixel position and `1/z`.
    fn weights(&self, px: f32, py: f32) -> ([f32; 3], f32) {
        let w = [
            edge(self, 1, 2, px, py) / self.area / self.z[0],
            edge(self, 2, 0, px, py) / self.area / self.z[1],
            edge(self, 0, 1, px, py) / self.area / self.z[2],
        ];
        let inv_z = w[0] + w[1] + w[2];
        (w, inv_z)
    }
}

/// Signed area of `(a, b, p)`.
fn edge(s: &Screen, a: usize, b: usize, px: f32, py: f32) -> f32 {
    (px - s.x[a]) * (s.y[b] - s.y[a]) - (py - s.y[a]) * (s.x[b] - s.x[a])
}

/// Interpolate per-corner values with perspective-correct weights.
fn mix<T>(
    w: &[f32; 3],
    inv_z: f32,
    a: [T; 3],
    scale: impl Fn(T, f32) -> T,
    add: impl Fn(T, T) -> T,
) -> T
where
    T: Copy,
{
    let t0 = scale(a[0], w[0] / inv_z);
    let t1 = scale(a[1], w[1] / inv_z);
    let t2 = scale(a[2], w[2] / inv_z);
    add(add(t0, t1), t2)
}

/// Tangent frame `(tangent, bitangent_green_up, normal)` of a textured triangle.
fn frame(points: &[Vec3; 3], uvs: &[[f32; 2]; 3], eye: Vec3) -> (Vec3, Vec3, Vec3) {
    let e1 = points[1] - points[0];
    let e2 = points[2] - points[0];
    let mut normal = e1.cross(e2).normalize_or_zero();
    if normal.dot(eye - points[0]) < 0.0 {
        normal = -normal;
    }
    let (du1, dv1) = (uvs[1][0] - uvs[0][0], uvs[1][1] - uvs[0][1]);
    let (du2, dv2) = (uvs[2][0] - uvs[0][0], uvs[2][1] - uvs[0][1]);
    let det = du1 * dv2 - du2 * dv1;
    if det.abs() < 1e-9 {
        let t = normal.any_orthonormal_vector();
        return (t, normal.cross(t), normal);
    }
    let inv = 1.0 / det;
    let tangent = (e1 * dv2 - e2 * dv1) * inv;
    let toward_v = (e2 * du1 - e1 * du2) * inv;
    // Image rows grow with v, a green-up normal map points the other way.
    let green_up = -toward_v;
    let t = (tangent - normal * normal.dot(tangent)).normalize_or_zero();
    let b = (green_up - normal * normal.dot(green_up) - t * t.dot(green_up)).normalize_or_zero();
    (t, b, normal)
}

/// Rasterize into an RGBA image; pixels no triangle covers stay transparent.
#[must_use]
pub fn render(
    view: &View,
    tris: &[RasterTri],
    materials: &[Material],
    lighting: &Lighting,
) -> egui::ColorImage {
    let (w, h) = (view.width, view.height);
    let mut depth = vec![0.0_f32; w * h];
    let mut rgba = vec![0_u8; w * h * 4];
    for tri in tris {
        let Some(screen) = Screen::new(view, &tri.points) else {
            continue;
        };
        let facing = (tri.points[1] - tri.points[0])
            .cross(tri.points[2] - tri.points[0])
            .dot(view.eye - tri.points[0]);
        if facing <= 0.0 {
            continue;
        }
        let material = tri.material.and_then(|i| materials.get(i));
        let tangent = match (material, tri.uvs) {
            (Some(_), Some(uvs)) => Some(frame(&tri.points, &uvs, view.eye)),
            _ => None,
        };
        let min_x = screen
            .x
            .iter()
            .copied()
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as usize;
        let max_x = (screen
            .x
            .iter()
            .copied()
            .fold(f32::MIN, f32::max)
            .ceil()
            .max(0.0) as usize)
            .min(w);
        let min_y = screen
            .y
            .iter()
            .copied()
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as usize;
        let max_y = (screen
            .y
            .iter()
            .copied()
            .fold(f32::MIN, f32::max)
            .ceil()
            .max(0.0) as usize)
            .min(h);
        for py in min_y..max_y {
            for px in min_x..max_x {
                let (cx, cy) = (px as f32 + 0.5, py as f32 + 0.5);
                let (weights, inv_z) = screen.weights(cx, cy);
                let inside = weights.iter().all(|&v| v * inv_z.signum() >= 0.0) && inv_z > 0.0;
                if !inside || inv_z <= depth[py * w + px] {
                    continue;
                }
                depth[py * w + px] = inv_z;
                let position = mix(&weights, inv_z, tri.points, |p, s| p * s, |a, b| a + b);
                let color = shade_pixel(
                    view,
                    tri,
                    material,
                    tangent,
                    &screen,
                    (cx, cy),
                    position,
                    lighting,
                );
                let at = (py * w + px) * 4;
                rgba[at] = byte(color.x);
                rgba[at + 1] = byte(color.y);
                rgba[at + 2] = byte(color.z);
                rgba[at + 3] = 255;
            }
        }
    }
    egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba)
}

/// `0..=1` to a byte.
fn byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Colour of one pixel.
#[allow(clippy::too_many_arguments)]
fn shade_pixel(
    view: &View,
    tri: &RasterTri,
    material: Option<&Material>,
    tangent: Option<(Vec3, Vec3, Vec3)>,
    screen: &Screen,
    (cx, cy): (f32, f32),
    position: Vec3,
    lighting: &Lighting,
) -> Vec3 {
    let (Some(material), Some(uvs), Some((t, b, n))) = (material, tri.uvs, tangent) else {
        return tri.flat;
    };
    let uv_at = |x: f32, y: f32| {
        let (w, inv_z) = screen.weights(x, y);
        let u = mix(&w, inv_z, uvs.map(|c| c[0]), |a, s| a * s, |a, c| a + c);
        let v = mix(&w, inv_z, uvs.map(|c| c[1]), |a, s| a * s, |a, c| a + c);
        (u, v)
    };
    let (u, v) = uv_at(cx, cy);
    let (ux, vx) = uv_at(cx + 1.0, cy);
    let (uy, vy) = uv_at(cx, cy + 1.0);
    let texels = material.albedo.as_ref().map_or(1, |m| m.width()) as f32;
    let footprint = ((ux - u).abs().max((vx - v).abs())).max((uy - u).abs().max((vy - v).abs()));
    let lod = (footprint * texels).max(1e-6).log2();
    let texel = material.albedo.as_ref().map_or(Vec3::ONE, |m| {
        let s = m.sample(u, v, lod);
        Vec3::new(s[0], s[1], s[2])
    });
    let albedo = texel * material.base;
    if material.normal.is_none() && material.orm.is_none() {
        return albedo * tri.flat;
    }
    let normal = material.normal.as_ref().map_or(n, |m| {
        let s = m.sample(u, v, lod);
        let ts = Vec3::new(s[0] * 2.0 - 1.0, s[1] * 2.0 - 1.0, s[2] * 2.0 - 1.0);
        (t * ts.x + b * ts.y + n * ts.z).normalize_or(n)
    });
    let (ao, roughness, metallic) = material.orm.as_ref().map_or((1.0, 0.8, 0.0), |m| {
        let s = m.sample(u, v, lod);
        (s[0], s[1], s[2])
    });
    shade_surface(
        &Surface {
            albedo,
            normal,
            position,
            view: (view.eye - position).normalize_or_zero(),
            ao,
            roughness,
            metallic,
        },
        lighting,
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::mesh::SceneLight;
    use egui::Color32;

    fn view() -> View {
        View {
            eye: Vec3::new(0.0, 4.0, 0.0),
            fwd: Vec3::NEG_Y,
            right: Vec3::X,
            up: Vec3::NEG_Z,
            focal: 16.0,
            width: 16,
            height: 16,
        }
    }

    fn solid(rgb: [u8; 3]) -> Arc<Mips> {
        Arc::new(Mips::new(RgbaImage::from_pixel(
            4,
            4,
            image::Rgba([rgb[0], rgb[1], rgb[2], 255]),
        )))
    }

    fn sun_from_plus_x(ambient: f32) -> Lighting {
        Lighting {
            ambient: Vec3::splat(ambient),
            lights: vec![SceneLight::Directional {
                direction: Vec3::new(-1.0, -1.0, 0.0).normalize(),
                color: Vec3::ONE,
                energy: 1.0,
            }],
        }
    }

    /// Two triangles of a 2x2 ground quad, u along x and v along z.
    fn ground(material: Option<usize>) -> Vec<RasterTri> {
        let a = Vec3::new(-1.0, 0.0, -1.0);
        let b = Vec3::new(-1.0, 0.0, 1.0);
        let c = Vec3::new(1.0, 0.0, -1.0);
        let d = Vec3::new(1.0, 0.0, 1.0);
        let uv = |p: Vec3| [f32::midpoint(p.x, 1.0), f32::midpoint(p.z, 1.0)];
        [[a, b, c], [d, c, b]]
            .into_iter()
            .map(|points| RasterTri {
                points,
                uvs: Some(points.map(uv)),
                material,
                flat: Vec3::ONE,
            })
            .collect()
    }

    fn centre(image: &egui::ColorImage) -> Color32 {
        image.pixels[8 * image.width() + 8]
    }

    fn material(normal: Option<[u8; 3]>, orm: Option<[u8; 3]>) -> Material {
        Material {
            albedo: Some(solid([255, 255, 255])),
            normal: normal.map(solid),
            orm: orm.map(solid),
            base: Vec3::ONE,
        }
    }

    #[test]
    fn nearer_triangle_wins_whatever_the_draw_order() {
        let big = |y: f32| {
            [
                Vec3::new(-3.0, y, -3.0),
                Vec3::new(-3.0, y, 6.0),
                Vec3::new(6.0, y, -3.0),
            ]
        };
        let tri = |y: f32, flat: Vec3| RasterTri {
            points: big(y),
            uvs: None,
            material: None,
            flat,
        };
        let red = Vec3::new(1.0, 0.0, 0.0);
        let blue = Vec3::new(0.0, 0.0, 1.0);
        // the far one is drawn last: a painter's algorithm in submission order would show it
        let image = render(
            &view(),
            &[tri(1.0, red), tri(0.0, blue)],
            &[],
            &Lighting::default(),
        );
        assert_eq!(centre(&image), Color32::from_rgb(255, 0, 0));
    }

    #[test]
    fn uvs_are_perspective_correct() {
        let view = View {
            eye: Vec3::ZERO,
            fwd: Vec3::Z,
            right: Vec3::X,
            up: Vec3::Y,
            focal: 50.0,
            width: 100,
            height: 100,
        };
        let points = [
            Vec3::new(-0.5, 0.0, 1.0),
            Vec3::new(1.5, 0.0, 3.0),
            Vec3::new(-0.5, 1.0, 1.0),
        ];
        let screen = Screen::new(&view, &points).expect("on screen");
        // halfway along the screen edge between a near (z=1, u=0) and far (z=3, u=1) corner
        let (w, inv_z) = screen.weights(50.0, 50.0);
        let u = mix(&w, inv_z, [0.0_f32, 1.0, 0.0], |a, s| a * s, |a, b| a + b);
        assert!((u - 0.25).abs() < 1e-3, "got {u}, linear would be 0.5");
    }

    #[test]
    fn normal_map_tilt_toward_the_light_brightens() {
        // tangent-space (x, y, z) = (0.7, 0, 0.714) encoded as bytes
        let toward = [217, 128, 218];
        let away = [38, 128, 218];
        let flat = [128, 128, 255];
        let light = sun_from_plus_x(0.0);
        let shade = |normal: [u8; 3]| {
            let mats = [material(Some(normal), None)];
            let image = render(&view(), &ground(Some(0)), &mats, &light);
            let c = centre(&image);
            u32::from(c.r()) + u32::from(c.g()) + u32::from(c.b())
        };
        let (bright, plain, dark) = (shade(toward), shade(flat), shade(away));
        assert!(bright > plain, "{bright} vs {plain}");
        assert!(plain > dark, "{plain} vs {dark}");
    }

    #[test]
    fn orm_channels_drive_metal_and_occlusion() {
        let light = sun_from_plus_x(0.0);
        let shade = |orm: [u8; 3], lighting: &Lighting| {
            let mats = [material(None, Some(orm))];
            centre(&render(&view(), &ground(Some(0)), &mats, lighting)).r()
        };
        // R = occlusion, G = roughness, B = metallic
        let dielectric = shade([255, 255, 0], &light);
        let metal = shade([255, 255, 255], &light);
        assert!(
            metal < dielectric,
            "rough metal has no diffuse: {metal} vs {dielectric}"
        );
        // light that grazes the surface: only ambient is left, and occlusion scales it
        let grazing = Lighting {
            ambient: Vec3::splat(0.5),
            lights: vec![SceneLight::Directional {
                direction: Vec3::Y,
                color: Vec3::ONE,
                energy: 1.0,
            }],
        };
        let open = shade([255, 255, 0], &grazing);
        let occluded = shade([128, 255, 0], &grazing);
        assert!(
            open > occluded + 40,
            "ambient must follow R: {open} vs {occluded}"
        );
    }

    #[test]
    fn mip_sampling_wraps_and_filters() {
        let mut img = RgbaImage::new(2, 2);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, image::Rgba([0, 0, 0, 255]));
        img.put_pixel(0, 1, image::Rgba([0, 0, 0, 255]));
        img.put_pixel(1, 1, image::Rgba([0, 0, 0, 255]));
        let mips = Mips::new(img);
        let at = mips.sample(0.25, 0.25, 0.0)[0];
        let wrapped = mips.sample(1.25, 2.25, 0.0)[0];
        assert!((at - wrapped).abs() < 1e-4, "repeat wrapping");
        assert!(
            mips.sample(0.5, 0.25, 0.0)[0] < at,
            "bilinear blends toward the neighbour"
        );
        assert!(
            (mips.sample(0.5, 0.5, 5.0)[0] - 0.25).abs() < 0.02,
            "coarsest mip is the mean"
        );
    }
}

//! Orbit camera plus `wgpu`-viewport overlays: AABB boxes, collision colors,
//! frustum wireframe, selection ghost from the trace scrubber.

use crate::state::{DesktopState, ACCENT};
use gamecraft_core::{spatial, NodeId};
use glam::Vec3;

/// Opaque viewport background.
const BG: egui::Color32 = egui::Color32::from_rgb(10, 12, 20);
/// Missing-collision-shape bodies.
const BAD_RED: egui::Color32 = egui::Color32::from_rgb(255, 90, 90);
/// Healthy physics bodies.
const BODY_GREEN: egui::Color32 = egui::Color32::from_rgb(90, 220, 130);
/// Mesh instances.
const MESH_BLUE: egui::Color32 = egui::Color32::from_rgb(110, 170, 255);
/// Plain nodes.
const NODE_GRAY: egui::Color32 = egui::Color32::from_rgb(150, 150, 160);
/// Frustum wireframe.
const FRUSTUM: egui::Color32 = egui::Color32::from_rgb(120, 220, 255);
/// Trace ghost marker.
const GHOST_WHITE: egui::Color32 = egui::Color32::from_rgb(240, 240, 245);
/// Axis colors.
const AXIS_X: egui::Color32 = egui::Color32::from_rgb(220, 90, 90);
const AXIS_Y: egui::Color32 = egui::Color32::from_rgb(100, 220, 100);
const AXIS_Z: egui::Color32 = egui::Color32::from_rgb(90, 140, 255);

/// Pinhole camera derived from the orbit state.
pub(crate) struct Cam {
    /// Camera position.
    eye: Vec3,
    /// Normalized view direction.
    fwd: Vec3,
    /// Normalized right vector.
    right: Vec3,
    /// Normalized up vector.
    up: Vec3,
    /// Focal length in pixels.
    focal: f32,
    /// Screen center.
    center: egui::Pos2,
}

/// Build the camera for this frame.
pub(crate) fn build_cam(
    eye: Vec3,
    target: Vec3,
    fov_deg: f32,
    center: egui::Pos2,
    height: f32,
) -> Cam {
    let to_target = target - eye;
    let dist_sq = to_target.length_squared();
    let fwd = if dist_sq > 1e-8 {
        to_target / dist_sq.sqrt()
    } else {
        Vec3::NEG_Z
    };
    let right_raw = fwd.cross(Vec3::Y);
    let right_sq = right_raw.length_squared();
    let right = if right_sq > 1e-8 {
        right_raw / right_sq.sqrt()
    } else {
        Vec3::X
    };
    let up = right.cross(fwd);
    let half_rad = fov_deg * std::f32::consts::PI / 360.0;
    let focal = (height * 0.5) / half_rad.tan().max(1e-4);
    Cam {
        eye,
        fwd,
        right,
        up,
        focal,
        center,
    }
}

/// Project a world point; `None` when behind the near plane.
pub(crate) fn project(cam: &Cam, point: Vec3) -> Option<egui::Pos2> {
    let rel = point - cam.eye;
    let depth = rel.dot(cam.fwd);
    if depth < 0.1 {
        return None;
    }
    let right = rel.dot(cam.right);
    let up = rel.dot(cam.up);
    Some(egui::Pos2::new(
        cam.center.x + cam.focal * right / depth,
        cam.center.y - cam.focal * up / depth,
    ))
}

/// Draw the whole viewport: background, axes, boxes, frustum, ghost.
pub(crate) fn draw_viewport(state: &mut DesktopState, ui: &mut egui::Ui) {
    let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());
    let rect = response.rect;
    painter.rect_filled(rect, 0.0, BG);
    let drag = response.drag_delta();
    state.cam_yaw -= drag.x * 0.005;
    state.cam_pitch = (state.cam_pitch + drag.y * 0.005).clamp(-1.4, 1.4);
    let zoom = ui.input(|input| input.smooth_scroll_delta.y);
    state.cam_dist = (state.cam_dist * (1.0 - zoom * 0.001)).max(0.5);
    let dir = Vec3::new(
        state.cam_yaw.cos() * state.cam_pitch.cos(),
        state.cam_pitch.sin(),
        state.cam_yaw.sin() * state.cam_pitch.cos(),
    );
    let eye = state.cam_target + dir * state.cam_dist;
    let cam = build_cam(eye, state.cam_target, 60.0, rect.center(), rect.height());
    if state.show_meshes {
        draw_meshes(&painter, state, &cam);
    }
    if state.show_axes {
        draw_axes(&painter, &cam);
    }
    if state.show_boxes {
        draw_boxes(&painter, state, &cam);
    }
    if state.show_frustum {
        draw_frustum(&painter, &cam, rect);
    }
    draw_ghost(&painter, state, &cam);
    painter.text(
        rect.left_top() + egui::Vec2::new(8.0, 6.0),
        egui::Align2::LEFT_TOP,
        state.adapter_label.clone(),
        egui::FontId::monospace(11.0),
        NODE_GRAY,
    );
}

/// Ground axes through the origin plus the 20m ground square.
fn draw_axes(painter: &egui::Painter, cam: &Cam) {
    let ends = [
        (
            Vec3::new(-10.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            AXIS_X,
        ),
        (
            Vec3::new(0.0, -10.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
            AXIS_Y,
        ),
        (
            Vec3::new(0.0, 0.0, -10.0),
            Vec3::new(0.0, 0.0, 10.0),
            AXIS_Z,
        ),
    ];
    for (start, end, color) in ends {
        if let (Some(a), Some(b)) = (project(cam, start), project(cam, end)) {
            painter.line_segment([a, b], egui::Stroke::new(1.0, color));
        }
    }
}

/// Shaded triangles built from the scene's own mesh primitives.
///
/// egui's render pass has no depth buffer, so triangles are back-face culled and
/// sorted far-to-near before painting (painter's algorithm).
fn draw_meshes(painter: &egui::Painter, state: &DesktopState, cam: &Cam) {
    let triangles = crate::mesh::build_scene(&state.graph, state.source.as_ref());
    let mut visible: Vec<(f32, [egui::Pos2; 3], egui::Color32)> = Vec::new();
    for tri in triangles {
        let edge_a = tri.points[1] - tri.points[0];
        let edge_b = tri.points[2] - tri.points[0];
        let normal = edge_a.cross(edge_b).normalize_or_zero();
        if normal.dot(cam.eye - tri.points[0]) <= 0.0 {
            continue;
        }
        let mut screen = [egui::Pos2::ZERO; 3];
        let mut complete = true;
        for (slot, point) in screen.iter_mut().zip(tri.points.iter()) {
            if let Some(projected) = project(cam, *point) {
                *slot = projected;
            } else {
                complete = false;
                break;
            }
        }
        if !complete {
            continue;
        }
        let depth = tri
            .points
            .iter()
            .map(|point| (*point - cam.eye).dot(cam.fwd))
            .sum::<f32>()
            / 3.0;
        visible.push((depth, screen, tri.color));
    }
    visible.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (_, points, color) in visible {
        painter.add(egui::Shape::convex_polygon(
            points.to_vec(),
            color,
            egui::Stroke::NONE,
        ));
    }
}

/// Per-node wireframe with collision colours and selection highlight.
///
/// Draws the box's 12 edges in projected space rather than one screen-aligned
/// rectangle: a rect hides rotation and depth, which is why the viewport read as
/// a flat diagram. Nodes are drawn far-to-near so nearer ones overlap correctly.
fn draw_boxes(painter: &egui::Painter, state: &DesktopState, cam: &Cam) {
    struct Item {
        depth: f32,
        id: NodeId,
        corners: [Vec3; 8],
        path: String,
        kind: String,
        body: bool,
    }
    let mut items: Vec<Item> = Vec::new();
    for id in state.graph.nodes.keys().copied() {
        let Some(node) = state.graph.get(id) else {
            continue;
        };
        let bounds = spatial::subtree_aabb(&state.graph, id);
        if bounds.is_empty() {
            continue;
        }
        items.push(Item {
            depth: (bounds.min + bounds.max)
                .mul_add(Vec3::splat(0.5), -cam.eye)
                .dot(cam.fwd),
            id,
            corners: box_corners(&bounds),
            path: node_path_or(state, id),
            kind: node.node_type.clone(),
            body: node.is_physics_body(),
        });
    }
    // Far first.
    items.sort_by(|left, right| {
        right
            .depth
            .partial_cmp(&left.depth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for item in items {
        // All eight corners must be in front of the camera to draw real edges;
        // a partial box would produce wrong lines through the near plane.
        let mut screen: [egui::Pos2; 8] = [egui::Pos2::ZERO; 8];
        let mut complete = true;
        for (slot, corner) in screen.iter_mut().zip(item.corners.iter()) {
            if let Some(point) = project(cam, *corner) {
                *slot = point;
            } else {
                complete = false;
                break;
            }
        }
        if !complete {
            continue;
        }
        let selected = state.selected == Some(item.id);
        let color = if selected {
            ACCENT
        } else if state.show_collision && state.bare_paths.contains(&item.path) {
            BAD_RED
        } else if state.show_collision && item.body {
            BODY_GREEN
        } else if item.kind == "MeshInstance3D" {
            MESH_BLUE
        } else {
            NODE_GRAY
        };
        let stroke = egui::Stroke::new(if selected { 2.2 } else { 1.2 }, color);
        // 12 edges of a cube: corner indices share one bit pattern per axis.
        for (a, b) in [
            (0, 1),
            (2, 3),
            (4, 5),
            (6, 7),
            (0, 2),
            (1, 3),
            (4, 6),
            (5, 7),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ] {
            if let (Some(from), Some(to)) = (screen.get(a), screen.get(b)) {
                painter.line_segment([*from, *to], stroke);
            }
        }
        if selected {
            if let Some(center) = project(cam, (item.corners[0] + item.corners[7]) * 0.5) {
                painter.circle_filled(center, 3.0, ACCENT);
            }
        }
    }
}

/// Eight world-space corners of a bounding box.
fn box_corners(bounds: &gamecraft_core::Aabb) -> [Vec3; 8] {
    let (lo, hi) = (bounds.min, bounds.max);
    [
        Vec3::new(lo.x, lo.y, lo.z),
        Vec3::new(hi.x, lo.y, lo.z),
        Vec3::new(lo.x, hi.y, lo.z),
        Vec3::new(hi.x, hi.y, lo.z),
        Vec3::new(lo.x, lo.y, hi.z),
        Vec3::new(hi.x, lo.y, hi.z),
        Vec3::new(lo.x, hi.y, hi.z),
        Vec3::new(hi.x, hi.y, hi.z),
    ]
}

/// Screen-space extent of projected points.
/// Scene path of a node for badge and color lookups.
fn node_path_or(state: &DesktopState, id: NodeId) -> String {
    state.graph.node_path(id).unwrap_or_else(|| "?".to_string())
}

/// Wireframe of the view frustum (near rect, far rect, four edges, eye dot).
fn draw_frustum(painter: &egui::Painter, cam: &Cam, rect: egui::Rect) {
    let aspect = (rect.width() / rect.height().max(1.0)).max(0.1);
    let tan_half = (30.0 * std::f32::consts::PI / 180.0).tan();
    let near = 0.5_f32;
    let far = 24.0_f32;
    let mut near_pts: Vec<egui::Pos2> = Vec::new();
    let mut far_pts: Vec<egui::Pos2> = Vec::new();
    for side_y in [-1.0_f32, 1.0] {
        for side_x in [-1.0_f32, 1.0] {
            let dir = cam.fwd * near
                + cam.right * (side_x * tan_half * aspect * near)
                + cam.up * (side_y * tan_half * near);
            if let Some(point) = project(cam, cam.eye + dir) {
                near_pts.push(point);
            }
            let long = cam.fwd * far
                + cam.right * (side_x * tan_half * aspect * far)
                + cam.up * (side_y * tan_half * far);
            if let Some(point) = project(cam, cam.eye + long) {
                far_pts.push(point);
            }
        }
    }
    let stroke = egui::Stroke::new(1.0, FRUSTUM);
    for quad in [&near_pts, &far_pts] {
        for (a, b) in [(0, 1), (1, 3), (3, 2), (2, 0)] {
            if let (Some(p), Some(q)) = (quad.get(a), quad.get(b)) {
                painter.line_segment([*p, *q], stroke);
            }
        }
    }
    for (a, b) in [(0, 0), (1, 1), (2, 2), (3, 3)] {
        if let (Some(p), Some(q)) = (near_pts.get(a), far_pts.get(b)) {
            painter.line_segment([*p, *q], stroke);
        }
    }
    if let Some(eye) = project(cam, cam.eye + cam.fwd * 0.6) {
        painter.circle_filled(eye, 3.0, FRUSTUM);
    }
}

/// Ghost marker for the scrubbed (non-latest) trace entry.
fn draw_ghost(painter: &egui::Painter, state: &DesktopState, cam: &Cam) {
    let latest = state.trace.len().saturating_sub(1);
    let scrubbed_index = usize::try_from(state.scrub_step).unwrap_or(0);
    if scrubbed_index >= latest {
        return;
    }
    let ghost = state.scrubbed().and_then(|entry| entry.ghost);
    if let Some(position) = ghost {
        if let Some(point) = project(cam, position) {
            let stroke = egui::Stroke::new(1.5, GHOST_WHITE);
            painter.circle_stroke(point, 7.0, stroke);
            painter.line_segment(
                [
                    egui::Pos2::new(point.x - 10.0, point.y),
                    egui::Pos2::new(point.x + 10.0, point.y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::Pos2::new(point.x, point.y - 10.0),
                    egui::Pos2::new(point.x, point.y + 10.0),
                ],
                stroke,
            );
        }
    }
}

//! Driving: a top-down car on a closed circuit, one lap to win.
//!
//! Replaces the original straight-line `driving-box` (a car sliding down a strip).
//! This one has a real car model: constant speed, bounded turn rate, and a
//! pure-pursuit autopilot that steers toward the next checkpoint.

use crate::model::{Forge, GameParts};
use crate::render::{Glyph, View};
use crate::scripts;
use gamecraft_core::{NodeId, PropertyValue, Transform};
use gamecraft_sim as sim;
use glam::Vec3;

/// Circuit radius (units).
const RADIUS: f32 = 12.0;
/// Waypoints around the loop.
const POINTS: usize = 8;
/// Cruise speed (units/s).
const SPEED: f32 = 10.0;
/// Steering rate (rad/s).
const TURN: f32 = 3.0;
/// Distance at which a checkpoint counts as reached.
const ADVANCE: f32 = 3.5;

/// Build the driving archetype.
///
/// # Errors
///
/// Returns a message when the trace cannot complete a lap or a node is invalid.
pub fn build_driving() -> Result<GameParts, String> {
    build_driving_with(RADIUS, POINTS, "driving", "GameCraft Driving")
}

/// Driving with a caller-supplied loop radius and checkpoint count.
///
/// # Errors
///
/// Returns a message when the trace cannot complete a lap or a node is invalid.
pub fn build_driving_with(
    radius: f32,
    points: usize,
    slug: &str,
    title: &str,
) -> Result<GameParts, String> {
    let course = waypoints(radius, points);
    let heading = start_heading(&course);
    let run = sim::run_drive(&course, heading, SPEED, TURN, ADVANCE, 1, 600);
    if !run.won {
        return Err(format!(
            "driving trace did not complete a lap: {} laps in {} frames",
            run.laps, run.frames
        ));
    }

    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(
            root,
            "checkpoints_to_win",
            PropertyValue::Int(i64::try_from(points).unwrap_or(8)),
        )
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/player_controller.gd", "2_player");
    forge.ext_resource("Script", "res://scripts/autoplay.gd", "3_autoplay");
    forge
        .top_camera("Camera3D", Some(root), [0.0, 0.0], 34.0)
        .map_err(|err| err.to_string())?;

    circuit(&mut forge, root, radius)?;
    checkpoints(&mut forge, root, &course)?;
    car(&mut forge, root, heading, radius)?;
    forge
        .autoplay_drive(root, "3_autoplay", "CP")
        .map_err(|err| err.to_string())?;

    let frames = run
        .path
        .iter()
        .map(|(x, z)| {
            vec![Glyph {
                x: *x,
                y: *z,
                w: 1.2,
                h: 0.6,
                color: 2,
                circle: false,
            }]
        })
        .collect::<Vec<Vec<Glyph>>>();
    let commands_json = forge
        .log
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>()
        .join("\n");
    Ok(GameParts {
        slug: Box::leak(slug.to_string().into_boxed_str()),
        title: Box::leak(title.to_string().into_boxed_str()),
        controls: "Left/Right steer. Constant speed, one lap to win.",
        win_text: Box::leak(
            format!("Pass all {points} checkpoints and cross the line.").into_boxed_str(),
        ),
        seed: 14,
        scene_text: export_scene(&forge)?,
        scripts: vec![
            ("gamecraft_runtime.gd", scripts::RUNTIME),
            ("player_controller.gd", scripts::PLAYER),
            ("autoplay.gd", scripts::AUTOPLAY),
        ],
        trace_json: format!(
            "{{\"game\":\"driving\",\"win\":true,\"frames\":{frames},\"laps\":{laps},\"speed\":{SPEED},\"turn_rate\":{TURN},\"note\":\"pure-pursuit steering, verified in Rust then replayed in-engine\"}}",
            frames = run.frames,
            laps = run.laps
        ),
        commands_json,
        statics: statics(&course),
        frames,
        view: View {
            x: -16.0,
            y: -16.0,
            w: 32.0,
            h: 32.0,
        },
        readme_body: format!(
            "Top-down circuit: {points} checkpoints on a {radius}-unit-radius loop.\nThe car has constant speed and a bounded turn rate, so the lap is a real\nsteering problem - a tighter loop would be un-drivable and the simulator\nrejects one (see gamecraft-sim drive_tests).\n"
        ),
        summary: format!(
            "driving: 1 lap, {points} checkpoints in {frames} frames, lint clean",
            frames = run.frames
        ),
    })
}

/// The eight circuit waypoints, evenly spaced on a circle.
fn waypoints(radius: f32, points: usize) -> Vec<(f32, f32)> {
    (0..points)
        .map(|index| {
            let step = u16::try_from(index).unwrap_or(0);
            let angle = std::f32::consts::TAU * f32::from(step)
                / f32::from(u16::try_from(points).unwrap_or(8));
            (radius * angle.cos(), radius * angle.sin())
        })
        .collect()
}

/// Initial heading, aimed at the second waypoint.
fn start_heading(course: &[(f32, f32)]) -> f32 {
    let first = course.first().copied().unwrap_or((RADIUS, 0.0));
    let second = course.get(1).copied().unwrap_or(first);
    (-(second.1 - first.1)).atan2(second.0 - first.0)
}

/// Ring road plus a darker infield.
fn circuit(forge: &mut Forge, root: NodeId, radius: f32) -> Result<(), String> {
    forge
        .deco_box(
            "Infield",
            Some(root),
            [0.0, -0.35, 0.0],
            [radius * 2.6, 0.3, radius * 2.6],
            0.24,
            0.34,
            0.24,
        )
        .map_err(|err| err.to_string())?;
    let ring = forge.torus_mesh("road", radius - 2.2, radius + 2.2);
    let asphalt = forge.material("road", 0.26, 0.27, 0.31);
    let road = forge
        .place("Road", "MeshInstance3D", Some(root), [0.0, -0.15, 0.0])
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(road, "mesh", Forge::sub_ref(ring.as_str()))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(road, "material_override", Forge::sub_ref(asphalt.as_str()))
        .map_err(|err| err.to_string())?;
    Ok(())
}

/// One trigger plus a marker post at each waypoint.
fn checkpoints(forge: &mut Forge, root: NodeId, course: &[(f32, f32)]) -> Result<(), String> {
    for (index, (x, z)) in course.iter().enumerate() {
        let name = format!("CP{index}");
        forge
            .area_trigger(name.as_str(), Some(root), [*x, 0.8, *z], [3.0, 2.0, 3.0])
            .map_err(|err| err.to_string())?;
        forge.connect("body_entered", name.as_str(), ".", "_on_checkpoint");
        forge
            .deco_box(
                format!("{name}_post").as_str(),
                Some(root),
                [*x, 1.2, *z],
                [0.3, 2.4, 0.3],
                0.95,
                0.78,
                0.24,
            )
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

/// The car: body plus four wheels, in driving mode.
fn car(forge: &mut Forge, root: NodeId, heading: f32, radius: f32) -> Result<NodeId, String> {
    let start = (radius, 0.0_f32);
    let id = forge
        .place(
            "Player",
            "CharacterBody3D",
            Some(root),
            [start.0, 0.5, start.1],
        )
        .map_err(|err| err.to_string())?;
    if let Some(node) = forge.graph.get_mut(id) {
        node.unique_name = true;
    }
    forge
        .set_prop(id, "script", Forge::ext_ref("2_player"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "mode", PropertyValue::Int(6))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "speed", PropertyValue::Float(f64::from(SPEED)))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "turn_rate", PropertyValue::Float(f64::from(TURN)))
        .map_err(|err| err.to_string())?;
    forge
        .set_transform(
            id,
            Transform::from_position_euler_scale(
                Vec3::new(start.0, 0.5, start.1),
                Vec3::new(0.0, heading, 0.0),
                Vec3::ONE,
            ),
        )
        .map_err(|err| err.to_string())?;
    let body = forge.box_mesh("car_body", [1.6, 0.5, 0.8]);
    let paint = forge.material("car_body", 0.92, 0.30, 0.22);
    forge
        .mesh_child("Body", id, body.as_str(), paint.as_str())
        .map_err(|err| err.to_string())?;
    for (index, (dx, dz)) in [
        (0.5_f32, 0.45_f32),
        (0.5, -0.45),
        (-0.5, 0.45),
        (-0.5, -0.45),
    ]
    .iter()
    .enumerate()
    {
        forge
            .deco_box(
                format!("Wheel{index}").as_str(),
                Some(id),
                [*dx, -0.1, *dz],
                [0.35, 0.35, 0.2],
                0.12,
                0.12,
                0.14,
            )
            .map_err(|err| err.to_string())?;
    }
    let shape = forge.box_shape("car", [1.6, 0.6, 0.9]);
    forge
        .shape_child("Shape", id, shape.as_str())
        .map_err(|err| err.to_string())?;
    Ok(id)
}

/// Export the forged graph to text.
fn export_scene(forge: &Forge) -> Result<String, String> {
    let request = gamecraft_godot::ExportRequest {
        graph: &forge.graph,
        connections: forge.conns.as_slice(),
        uid: gamecraft_godot::deterministic_uid("res://depot/games/driving/main.tscn"),
        format: 3,
        ext_resources: forge.ext.clone(),
        sub_resources: forge.sub.clone(),
    };
    gamecraft_godot::export_tscn(&request)
}

/// Static geometry for the trace render: the loop and its checkpoints.
fn statics(course: &[(f32, f32)]) -> Vec<Glyph> {
    let mut out: Vec<Glyph> = Vec::new();
    let mut previous = course.last().copied().unwrap_or((0.0, 0.0));
    for point in course {
        let mid = (
            f32::midpoint(previous.0, point.0),
            f32::midpoint(previous.1, point.1),
        );
        out.push(Glyph {
            x: mid.0,
            y: mid.1,
            w: 4.0,
            h: 4.0,
            color: 11,
            circle: false,
        });
        previous = *point;
    }
    for (x, z) in course {
        out.push(Glyph {
            x: *x,
            y: *z,
            w: 0.4,
            h: 2.0,
            color: 3,
            circle: false,
        });
    }
    out
}

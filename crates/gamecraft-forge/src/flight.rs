//! Flight sim: fly a gate course over a parametric village.
//!
//! Third archetype family after platformer/arcade and the top-down set. The
//! environment is where the parametric props live: four buildings at different
//! LODs and four creatures, all generated from specs rather than modelled.

use crate::model::{Forge, GameParts};
use crate::props::{spawn_building, spawn_creature, BuildingKind, BuildingSpec, CreatureKind};
use crate::render::{Glyph, View};
use crate::scripts;
use gamecraft_core::{NodeId, PropertyValue, Transform};
use gamecraft_sim as sim;
use glam::Vec3;

/// Cruise speed (units/s).
const SPEED: f32 = 12.0;
/// Climb rate (units/s).
const CLIMB: f32 = 6.0;
/// Gate tolerance in altitude (units).
const TOLERANCE: f32 = 1.6;

/// Gate positions: `(x, altitude)`.
const GATES: [(f32, f32); 5] = [
    (20.0, 4.0),
    (40.0, 6.5),
    (60.0, 4.5),
    (80.0, 8.0),
    (100.0, 5.5),
];

/// Build the flight archetype.
///
/// # Errors
///
/// Returns a message when the trace cannot clear the gates or a node is invalid.
pub fn build_flight() -> Result<GameParts, String> {
    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "rings_to_win", PropertyValue::Int(5))
        .map_err(|err| err.to_string())?;
    forge
        .sky(root, [0.24, 0.42, 0.75], [0.78, 0.87, 0.96])
        .map_err(|err| err.to_string())?;
    forge
        .sun(root, -0.95, -0.55, 1.15)
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/player_controller.gd", "2_player");
    forge.ext_resource("Script", "res://scripts/camera_follow.gd", "3_camera");
    forge.ext_resource("Script", "res://scripts/autoplay.gd", "4_autoplay");
    forge.ext_resource("Texture2D", "res://textures/plaster.png", "tex_plaster");
    forge.ext_resource("Texture2D", "res://textures/brick.png", "tex_brick");

    terrain(&mut forge, root)?;
    village(&mut forge, root)?;
    gates(&mut forge, root)?;
    aircraft(&mut forge, root)?;
    forge
        .follow_camera("Camera3D", Some(root), [0.0, 3.6, 16.0], 42.0, "3_camera")
        .map_err(|err| err.to_string())?;
    forge
        .autoplay_flight(root, "4_autoplay")
        .map_err(|err| err.to_string())?;

    let run = sim::run_flight(SPEED, CLIMB, &GATES, TOLERANCE, 600);
    if !run.won {
        return Err(format!(
            "flight trace missed a gate: {} of {} cleared in {} frames",
            run.rings,
            GATES.len(),
            run.frames
        ));
    }
    let frames = run
        .path
        .iter()
        .map(|(x, y)| {
            vec![Glyph {
                x: *x,
                y: *y,
                w: 1.0,
                h: 0.5,
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
    let statics = statics();
    Ok(GameParts {
        slug: "flight",
        title: "GameCraft Flight",
        controls: "Up/Down pitch to climb or dive. Speed is fixed; fly through all 5 gates.",
        win_text: "Pass all 5 gates.",
        seed: 13,
        scene_text: export_scene(&forge)?,
        scripts: vec![
            ("gamecraft_runtime.gd", scripts::RUNTIME),
            ("player_controller.gd", scripts::PLAYER),
            ("camera_follow.gd", scripts::CAMERA_FOLLOW),
            ("autoplay.gd", scripts::AUTOPLAY),
        ],
        trace_json: format!(
            "{{\"game\":\"flight\",\"win\":true,\"frames\":{frames},\"gates\":{gates},\"speed\":{SPEED},\"climb\":{CLIMB},\"note\":\"autopilot tracks the next gate altitude; verified in Rust then replayed in-engine\"}}",
            frames = run.frames,
            gates = run.rings
        ),
        commands_json,
        statics,
        frames,
        view: View {
            x: -6.0,
            y: -2.0,
            w: 118.0,
            h: 15.0,
        },
        readme_body: "Gate course over a small settlement. Buildings are generated\nfrom a spec (kind, footprint, height, LOD), not modelled: farmhouse and\nigloo at LOD 2, barn at LOD 1, church at LOD 2 with tower and spire.\nCreatures are two-primitive silhouettes (person = capsule + head).\nPitch with Up/Down; the autopilot replay tracks each gate's altitude.\n".to_string(),
        summary: format!(
            "flight: {} gates in {} frames, lint clean",
            run.rings, run.frames
        ),
    })
}

/// Ground strip plus a runway marking.
fn terrain(forge: &mut Forge, root: NodeId) -> Result<(), String> {
    forge
        .deco_box(
            "Terrain",
            Some(root),
            [60.0, -0.6, 0.0],
            [150.0, 1.2, 60.0],
            0.42,
            0.55,
            0.32,
        )
        .map_err(|err| err.to_string())?;
    forge
        .deco_box(
            "Runway",
            Some(root),
            [8.0, -0.02, 0.0],
            [34.0, 0.06, 5.0],
            0.30,
            0.31,
            0.34,
        )
        .map_err(|err| err.to_string())?;
    Ok(())
}

/// Settlement: four building kinds at three LODs, plus creatures.
fn village(forge: &mut Forge, root: NodeId) -> Result<(), String> {
    let plan = [
        (BuildingKind::Farmhouse, 16.0_f32, -7.0_f32, 2_u8),
        (BuildingKind::Barn, 34.0, -9.0, 1),
        (BuildingKind::Igloo, 54.0, -6.0, 2),
        (BuildingKind::Church, 74.0, -8.0, 2),
        (BuildingKind::Windmill, 26.0, -14.0, 2),
        (BuildingKind::Lighthouse, 48.0, -16.0, 2),
        (BuildingKind::Castle, 92.0, -11.0, 2),
    ];
    for (serial, (kind, x, z, lod)) in plan.iter().enumerate() {
        let mut spec = BuildingSpec::new(*kind, *x, *z, *lod);
        spec.tex = Some(match kind {
            BuildingKind::Barn | BuildingKind::Church | BuildingKind::Castle => "tex_brick",
            BuildingKind::Farmhouse
            | BuildingKind::Igloo
            | BuildingKind::Windmill
            | BuildingKind::Lighthouse => "tex_plaster",
        });
        spawn_building(forge, root, serial, &spec)?;
    }
    let folk = [
        (CreatureKind::Person, 17.0_f32, -4.0_f32),
        (CreatureKind::Cow, 35.0, -5.5),
        (CreatureKind::Person, 55.0, -3.5),
        (CreatureKind::Cow, 75.0, -5.0),
    ];
    for (serial, (kind, x, z)) in folk.iter().enumerate() {
        spawn_creature(forge, root, serial, *kind, *x, *z)?;
    }
    Ok(())
}

/// Five torus gates, each an `Area3D` trigger wired to the runtime.
fn gates(forge: &mut Forge, root: NodeId) -> Result<(), String> {
    for (index, (x, y)) in GATES.iter().enumerate() {
        let name = format!("Ring{index}");
        let ring = forge
            .place(name.as_str(), "Area3D", Some(root), [*x, *y, 0.0])
            .map_err(|err| err.to_string())?;
        let mesh = forge.torus_mesh(name.as_str(), 1.5, 2.1);
        let material = forge.material(name.as_str(), 0.98, 0.78, 0.24);
        forge
            .mesh_child("Mesh", ring, mesh.as_str(), material.as_str())
            .map_err(|err| err.to_string())?;
        // TorusMesh lies flat; stand it up so the hole faces the flight path.
        if let Some(mesh_node) = forge
            .graph
            .get(ring)
            .and_then(|node| node.children.first().copied())
            .and_then(|child| forge.graph.get_mut(child))
        {
            mesh_node.transform = Transform::from_position_euler_scale(
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, std::f32::consts::FRAC_PI_2),
                Vec3::ONE,
            );
        }
        let shape = forge.box_shape(name.as_str(), [2.4, 3.0, 1.2]);
        forge
            .shape_child("Shape", ring, shape.as_str())
            .map_err(|err| err.to_string())?;
        forge.connect("body_entered", name.as_str(), ".", "_on_ring");
    }
    Ok(())
}

/// The aircraft: a `CharacterBody3D` in flight mode with a fuselage and wings.
fn aircraft(forge: &mut Forge, root: NodeId) -> Result<NodeId, String> {
    let id = forge
        .place("Player", "CharacterBody3D", Some(root), [0.0, 3.0, 0.0])
        .map_err(|err| err.to_string())?;
    if let Some(node) = forge.graph.get_mut(id) {
        node.unique_name = true;
    }
    forge
        .set_prop(id, "script", Forge::ext_ref("2_player"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "mode", PropertyValue::Int(5))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "speed", PropertyValue::Float(f64::from(SPEED)))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "climb_speed", PropertyValue::Float(f64::from(CLIMB)))
        .map_err(|err| err.to_string())?;
    let body = forge.box_mesh("plane_body", [1.4, 0.5, 0.5]);
    let paint = forge.material("plane_body", 0.88, 0.30, 0.24);
    forge
        .mesh_child("Fuselage", id, body.as_str(), paint.as_str())
        .map_err(|err| err.to_string())?;
    forge
        .deco_box(
            "Wing",
            Some(id),
            [0.0, 0.0, 0.0],
            [0.5, 0.12, 2.6],
            0.92,
            0.92,
            0.95,
        )
        .map_err(|err| err.to_string())?;
    let shape = forge.box_shape("plane", [1.4, 0.5, 2.6]);
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
        uid: gamecraft_godot::deterministic_uid("res://depot/games/flight/main.tscn"),
        format: 3,
        ext_resources: forge.ext.clone(),
        sub_resources: forge.sub.clone(),
    };
    gamecraft_godot::export_tscn(&request)
}

/// Static geometry for the trace render.
fn statics() -> Vec<Glyph> {
    let mut out = vec![Glyph {
        x: 60.0,
        y: -0.6,
        w: 150.0,
        h: 1.2,
        color: 1,
        circle: false,
    }];
    for (x, y) in GATES {
        out.push(Glyph {
            x,
            y,
            w: 0.3,
            h: 3.2,
            color: 3,
            circle: false,
        });
    }
    for (x, height) in [(16.0_f32, 1.8_f32), (34.0, 2.2), (54.0, 1.2), (74.0, 2.6)] {
        out.push(Glyph {
            x,
            y: height * 0.5,
            w: 3.0,
            h: height,
            color: 11,
            circle: false,
        });
    }
    out
}

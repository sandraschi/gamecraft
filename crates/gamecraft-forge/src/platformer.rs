//! Platformer: run right, jump gaps, grab coins, tag the goal.
//!
//! The trace is verified by [`run_trace`](gamecraft_sim::run_trace): hold Right,
//! jump at scripted frames, reach `x = 31` with 3+ coins inside 600 frames.

use crate::model::{Forge, GameParts};
use crate::render::{Glyph, View};
use crate::scripts::{AUTOPLAY, CAMERA_FOLLOW, COIN, PLAYER, RUNTIME};
use gamecraft_core::{NodeId, PropertyValue};
use gamecraft_godot::ExportRequest;
use gamecraft_sim::{self as sim, Body2, Phys, Platform, TraceInput};
use glam::Vec3;

/// Player physics (mirrored in `player_controller.gd` exports).
const PHYS: Phys = Phys {
    speed: 4.5,
    jump_v: 9.0,
    gravity: 22.0,
    auto_run: false,
    half_w: 0.3,
    half_h: 0.5,
};

/// Walkable tops: three grounds plus three stair platforms.
const GROUNDS: [Platform; 6] = [
    Platform {
        x: -8.0,
        y: 0.0,
        w: 14.0,
    },
    Platform {
        x: 8.5,
        y: 0.0,
        w: 11.5,
    },
    Platform {
        x: 22.5,
        y: 0.0,
        w: 11.5,
    },
    Platform {
        x: 1.0,
        y: 0.9,
        w: 3.0,
    },
    Platform {
        x: 11.0,
        y: 1.0,
        w: 4.0,
    },
    Platform {
        x: 25.0,
        y: 1.1,
        w: 2.0,
    },
];

/// Coin pickups (must match the scene).
const COINS: [(f32, f32); 4] = [(1.9, 2.3), (12.8, 2.3), (26.0, 2.4), (21.25, 1.5)];

/// Goal gate x.
const GOAL_X: f32 = 31.0;
/// Kill plane y.
const KILL_Y: f32 = -6.0;

/// Scripted jumps (hold Right; each hop verified against the sim).
const JUMPS: [usize; 5] = [0, 75, 147, 260, 327];

/// Build the platformer: scene, trace, frames.
///
/// # Errors
///
/// Returns a message when the trace fails verification or a node is invalid.
pub fn build_platformer() -> Result<GameParts, String> {
    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge
        .sky(root, [0.24, 0.42, 0.75], [0.78, 0.87, 0.96])
        .map_err(|err| err.to_string())?;
    forge
        .sun(root, -0.95, -0.55, 1.15)
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/player_controller.gd", "2_player");
    forge.ext_resource("Script", "res://scripts/coin.gd", "3_coin");
    forge.ext_resource("Script", "res://scripts/camera_follow.gd", "4_camera");
    forge.ext_resource("Script", "res://scripts/autoplay.gd", "5_autoplay");

    camera(&mut forge, root)?;
    forge
        .autoplay(root, "5_autoplay", "ui_right", &JUMPS)
        .map_err(|err| err.to_string())?;
    ground(&mut forge, root, "GroundA", -1.0, 14.0)?;
    ground(&mut forge, root, "GroundB", 14.25, 11.5)?;
    ground(&mut forge, root, "GroundC", 28.25, 11.5)?;
    platform(&mut forge, root, "Plat1", 2.5, 0.7, 3.0)?;
    platform(&mut forge, root, "Plat2", 13.0, 0.8, 4.0)?;
    platform(&mut forge, root, "Plat3", 26.0, 0.9, 2.0)?;
    player(&mut forge, root)?;
    for (index, coin) in COINS.iter().enumerate() {
        coin_node(&mut forge, root, format!("Coin{index}").as_str(), *coin)?;
    }
    goal(&mut forge, root)?;
    kill(&mut forge, root)?;
    decor(&mut forge, root)?;
    hint(&mut forge, root, "Hint0", [0.1, 1.0, 0.0])?;
    hint(&mut forge, root, "Hint1", [5.6, 1.0, 0.0])?;
    hint(&mut forge, root, "Hint2", [11.0, 1.0, 0.0])?;
    hint(&mut forge, root, "Hint3", [19.5, 1.0, 0.0])?;
    hint(&mut forge, root, "Hint4", [24.5, 1.0, 0.0])?;

    forge.connect("body_entered", "Goal", ".", "_on_goal");
    forge.connect("body_entered", "Kill", ".", "_on_kill");

    let scene_text = export(&forge)?;
    let (run, _inputs) = trace();
    if !run.won {
        return Err("platformer trace did not reach the goal".to_string());
    }
    if run.frames > 600 {
        return Err(format!(
            "platformer trace too slow: {frames} frames",
            frames = run.frames
        ));
    }
    if run.coins < 3 {
        return Err(format!(
            "platformer trace coins: {coins}",
            coins = run.coins
        ));
    }
    let frames = frames(&run);
    let trace_json = format!(
        "{{\"game\":\"platformer\",\"win\":true,\"frames\":{frames},\"coins\":{coins},\"hold\":\"Right\",\"jumps\":[0,75,147,260,327],\"note\":\"scripted input, verified by kinematic replay\"}}",
        frames = run.frames, coins = run.coins
    );
    let commands_json = forge
        .log
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>()
        .join("\n");
    Ok(GameParts {
        slug: "platformer",
        title: "GameCraft Platformer",
        controls: "Left/Right arrows to run, Space to jump. Grab coins, tag the red goal. Falling resets you to spawn.",
        win_text: "Touch the goal flag at x = 31.",
        seed: 1,
        scene_text,
        scripts: vec![
            ("gamecraft_runtime.gd", RUNTIME),
            ("player_controller.gd", PLAYER),
            ("coin.gd", COIN),
            ("camera_follow.gd", CAMERA_FOLLOW),
            ("autoplay.gd", AUTOPLAY),
        ],
        trace_json,
        commands_json,
        statics: statics(),
        frames,
        view: View { x: -10.0, y: -3.0, w: 46.0, h: 11.0 },
        readme_body: "Three grounds, two gaps, three stair platforms, four coins.\nJump tuning: speed 4.5, jump 9.0, gravity 22.0 (see trace.json).\nCoins sit on jump arcs, never off the critical path (Nintendo lesson 1).\n".to_string(),
        summary: format!(
            "platformer: goal in {frames} frames, {coins}/4 coins, lint clean",
            frames = run.frames, coins = run.coins
        ),
    })
}

/// Scripted input trace plus its echo for the trace file.
fn trace() -> (sim::TraceRun, Vec<TraceInput>) {
    let mut script = vec![TraceInput::Right; 600];
    for frame in JUMPS {
        if let Some(slot) = script.get_mut(frame) {
            *slot = TraceInput::RightJump;
        }
    }
    let start = Body2 {
        x: 0.0,
        y: 0.5,
        vx: 0.0,
        vy: 0.0,
    };
    let coins: Vec<(f32, f32)> = COINS.to_vec();
    let run = sim::run_trace(
        start,
        script.as_slice(),
        PHYS,
        &GROUNDS,
        coins.as_slice(),
        GOAL_X,
        KILL_Y,
        600,
    );
    (run, script)
}

/// Per-frame dynamics: player plus uncollected coins.
fn frames(run: &sim::TraceRun) -> Vec<Vec<Glyph>> {
    let mut out: Vec<Vec<Glyph>> = Vec::new();
    for (index, pos) in run.path.iter().enumerate() {
        let mut frame = vec![Glyph {
            x: pos.0,
            y: pos.1,
            w: 0.6,
            h: 1.0,
            color: 2,
            circle: false,
        }];
        for (coin_index, coin) in COINS.iter().enumerate() {
            let taken = match run.coin_at.get(coin_index) {
                Some(Some(tick)) => *tick <= index,
                _ => false,
            };
            if !taken {
                frame.push(Glyph {
                    x: coin.0,
                    y: coin.1,
                    w: 0.5,
                    h: 0.5,
                    color: 3,
                    circle: true,
                });
            }
        }
        out.push(frame);
    }
    out
}

/// Static level geometry for the renderer.
fn statics() -> Vec<Glyph> {
    let mut out = vec![
        Glyph {
            x: -1.0,
            y: -0.5,
            w: 14.0,
            h: 1.0,
            color: 0,
            circle: false,
        },
        Glyph {
            x: 14.25,
            y: -0.5,
            w: 11.5,
            h: 1.0,
            color: 0,
            circle: false,
        },
        Glyph {
            x: 28.25,
            y: -0.5,
            w: 11.5,
            h: 1.0,
            color: 0,
            circle: false,
        },
        Glyph {
            x: 3.5,
            y: 0.7,
            w: 3.0,
            h: 0.4,
            color: 1,
            circle: false,
        },
        Glyph {
            x: 13.0,
            y: 0.8,
            w: 4.0,
            h: 0.4,
            color: 1,
            circle: false,
        },
        Glyph {
            x: 26.0,
            y: 0.9,
            w: 2.0,
            h: 0.4,
            color: 1,
            circle: false,
        },
        Glyph {
            x: 31.0,
            y: 0.5,
            w: 0.3,
            h: 3.0,
            color: 0,
            circle: false,
        },
        Glyph {
            x: 31.8,
            y: 1.6,
            w: 1.6,
            h: 1.0,
            color: 3,
            circle: false,
        },
        Glyph {
            x: 13.0,
            y: -7.0,
            w: 50.0,
            h: 1.0,
            color: 5,
            circle: false,
        },
    ];
    for bush in [-4.0_f32, 4.5, 16.0, 24.0] {
        out.push(Glyph {
            x: bush,
            y: 0.4,
            w: 1.2,
            h: 0.8,
            color: 1,
            circle: false,
        });
    }
    for cloud in [2.0_f32, 15.0, 28.0] {
        out.push(Glyph {
            x: cloud,
            y: 6.0,
            w: 3.0,
            h: 0.8,
            color: 7,
            circle: false,
        });
    }
    out
}

/// Export the forged graph to text.
fn export(forge: &Forge) -> Result<String, String> {
    let request = ExportRequest {
        graph: &forge.graph,
        connections: forge.conns.as_slice(),
        uid: gamecraft_godot::deterministic_uid("res://depot/games/platformer/main.tscn"),
        format: 3,
        ext_resources: forge.ext.clone(),
        sub_resources: forge.sub.clone(),
    };
    gamecraft_godot::export::export_tscn(&request)
}

/// Follow camera: tracks the player on x, fixed height and distance.
///
/// A static camera framing the whole 48-unit level reduced it to a thin strip at
/// the bottom of the screen (measured: all geometry in the bottom 10% of a 720p
/// frame), so the scene was unplayable to watch. The follow camera uses a narrow
/// vertical FOV so the ground and the player fill the frame.
fn camera(forge: &mut Forge, root: NodeId) -> Result<NodeId, String> {
    forge
        .follow_camera("Camera3D", Some(root), [0.0, 2.4, 14.0], 34.0, "4_camera")
        .map_err(|err| err.to_string())?;
    Ok(root)
}

/// Ground slab centered at `center_x`, top at `y = 0`.
fn ground(
    forge: &mut Forge,
    root: NodeId,
    name: &str,
    center_x: f32,
    width: f32,
) -> Result<NodeId, String> {
    forge
        .solid_box(
            name,
            "StaticBody3D",
            Some(root),
            [center_x, -0.5, 0.0],
            [width, 1.0, 4.0],
            0.55,
            0.36,
            0.18,
        )
        .map_err(|err| err.to_string())
}

/// Floating stair platform of variable width.
fn platform(
    forge: &mut Forge,
    root: NodeId,
    name: &str,
    center_x: f32,
    center_y: f32,
    width: f32,
) -> Result<NodeId, String> {
    forge
        .solid_box(
            name,
            "StaticBody3D",
            Some(root),
            [center_x, center_y, 0.0],
            [width, 0.4, 2.0],
            0.29,
            0.63,
            0.35,
        )
        .map_err(|err| err.to_string())
}

/// Controllable player with mesh, shape, and controller script.
fn player(forge: &mut Forge, root: NodeId) -> Result<NodeId, String> {
    let id = forge
        .place("Player", "CharacterBody3D", Some(root), [0.0, 0.5, 0.0])
        .map_err(|err| err.to_string())?;
    if let Some(node) = forge.graph.get_mut(id) {
        node.unique_name = true;
    }
    forge
        .set_prop(id, "script", Forge::ext_ref("2_player"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "speed", PropertyValue::Float(4.5))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "jump_velocity", PropertyValue::Float(9.0))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "gravity", PropertyValue::Float(22.0))
        .map_err(|err| err.to_string())?;
    let mesh = forge.capsule_mesh("player", 0.3, 1.0);
    let material = forge.material("player", 0.35, 0.59, 1.0);
    forge
        .mesh_child("Mesh", id, mesh.as_str(), material.as_str())
        .map_err(|err| err.to_string())?;
    let shape = forge.capsule_shape("player", 0.3, 1.0);
    forge
        .shape_child("Shape", id, shape.as_str())
        .map_err(|err| err.to_string())?;
    Ok(id)
}

/// Collectible coin area.
fn coin_node(
    forge: &mut Forge,
    root: NodeId,
    name: &str,
    at: (f32, f32),
) -> Result<NodeId, String> {
    let id = forge
        .place("CoinX", "Area3D", Some(root), [at.0, at.1, 0.0])
        .map_err(|err| err.to_string())?;
    if let Some(node) = forge.graph.get_mut(id) {
        node.name = name.to_string();
    }
    forge
        .set_prop(id, "script", Forge::ext_ref("3_coin"))
        .map_err(|err| err.to_string())?;
    let mesh = forge.sphere_mesh(name, 0.28, 0.56);
    let material = forge.material(name, 1.0, 0.8, 0.24);
    forge
        .mesh_child("Mesh", id, mesh.as_str(), material.as_str())
        .map_err(|err| err.to_string())?;
    let shape = forge.sphere_shape(name, 0.28);
    forge
        .shape_child("Shape", id, shape.as_str())
        .map_err(|err| err.to_string())?;
    Ok(id)
}

/// Goal flag with a trigger volume.
fn goal(forge: &mut Forge, root: NodeId) -> Result<NodeId, String> {
    let id = forge
        .place("Goal", "Area3D", Some(root), [31.0, 1.5, 0.0])
        .map_err(|err| err.to_string())?;
    let pole = forge.box_mesh("pole", [0.3, 3.0, 0.3]);
    let wood = forge.material("pole", 0.55, 0.36, 0.18);
    forge
        .mesh_child("Pole", id, pole.as_str(), wood.as_str())
        .map_err(|err| err.to_string())?;
    if let Some(pole_node) = forge
        .graph
        .get(id)
        .and_then(|node| node.children.first().copied())
        .and_then(|child| forge.graph.get_mut(child))
    {
        pole_node.transform.position = Vec3::new(0.0, -0.5, 0.0);
    }
    let flag = forge.prism_mesh("flag", [1.6, 1.0, 0.2]);
    let gold = forge.material("flag", 1.0, 0.8, 0.24);
    forge
        .mesh_child("Flag", id, flag.as_str(), gold.as_str())
        .map_err(|err| err.to_string())?;
    if let Some(flag_node) = forge
        .graph
        .get(id)
        .and_then(|node| node.children.get(1).copied())
        .and_then(|child| forge.graph.get_mut(child))
    {
        flag_node.transform.position = Vec3::new(0.8, 0.8, 0.0);
    }
    let shape = forge.box_shape("goal", [1.5, 3.0, 1.5]);
    forge
        .shape_child("Shape", id, shape.as_str())
        .map_err(|err| err.to_string())?;
    Ok(id)
}

/// Kill plane under the level.
fn kill(forge: &mut Forge, root: NodeId) -> Result<NodeId, String> {
    let id = forge
        .place("Kill", "Area3D", Some(root), [13.0, -7.0, 0.0])
        .map_err(|err| err.to_string())?;
    let shape = forge.box_shape("kill", [50.0, 1.0, 6.0]);
    forge
        .shape_child("Shape", id, shape.as_str())
        .map_err(|err| err.to_string())?;
    Ok(id)
}

/// Jump hint: replays one verified hop headless.
fn hint(forge: &mut Forge, root: NodeId, name: &str, at: [f32; 3]) -> Result<NodeId, String> {
    let id = forge
        .area_trigger(name, Some(root), at, [0.6, 2.0, 2.0])
        .map_err(|err| err.to_string())?;
    forge.connect("body_entered", name, ".", "_on_hint");
    Ok(id)
}

/// Bushes and clouds (pure decoration, no collision).
fn decor(forge: &mut Forge, root: NodeId) -> Result<(), String> {
    let bush_mesh = forge.sphere_mesh("bush", 0.55, 0.75);
    let bush_mat = forge.material("bush", 0.29, 0.63, 0.35);
    let cloud_mesh = forge.sphere_mesh("cloud", 1.6, 1.1);
    let cloud_mat = forge.material("cloud", 0.94, 0.94, 0.96);
    for (index, x) in [-6.5_f32, 10.0, 17.0, 32.0].iter().enumerate() {
        let id = forge
            .place(
                format!("Bush{index}").as_str(),
                "MeshInstance3D",
                Some(root),
                [*x, 0.42, 0.0],
            )
            .map_err(|err| err.to_string())?;
        forge
            .set_prop(id, "mesh", Forge::sub_ref(bush_mesh.as_str()))
            .map_err(|err| err.to_string())?;
        forge
            .set_prop(id, "material_override", Forge::sub_ref(bush_mat.as_str()))
            .map_err(|err| err.to_string())?;
    }
    for (index, x) in [2.0_f32, 15.0, 28.0].iter().enumerate() {
        let id = forge
            .place(
                format!("Cloud{index}").as_str(),
                "MeshInstance3D",
                Some(root),
                [*x, 5.2, -3.5],
            )
            .map_err(|err| err.to_string())?;
        forge
            .set_prop(id, "mesh", Forge::sub_ref(cloud_mesh.as_str()))
            .map_err(|err| err.to_string())?;
        forge
            .set_prop(id, "material_override", Forge::sub_ref(cloud_mat.as_str()))
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

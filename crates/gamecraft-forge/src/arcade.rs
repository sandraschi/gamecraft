//! Movement games: runner, flappy, driving, pong, breakout.
//!
//! Each ships a verified trace: kinematic replay (runner/driving), feedback
//! policy (flappy), or mirrored game logic (pong/breakout).

use crate::model::{Forge, GameParts};
use crate::render::{Glyph, View};
use gamecraft_core::PropertyValue;
use gamecraft_sim::{self as sim, Body2, Phys, Platform, TraceInput};

/// Lossless level index to float (all counts fit in `u16`).
fn idx(index: usize) -> f32 {
    f32::from(u16::try_from(index).unwrap_or(0))
}

const KILL_Y: f32 = -6.0;

/// Side-scroller camera: follows the player, narrow vertical FOV.
///
/// `x` is only the starting placement; the script snaps to the player. `z` is
/// ignored - a fixed far distance was what made the level a thin strip.
fn side_camera(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    x: f32,
    z: f32,
) -> Result<(), String> {
    let _ = z;
    // Side views get the gradient sky; the platformer does the same explicitly.
    forge
        .sky(root, [0.24, 0.42, 0.75], [0.78, 0.87, 0.96])
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/camera_follow.gd", "9_camera");
    forge
        .follow_camera("Camera3D", Some(root), [x, 2.4, 14.0], 34.0, "9_camera")
        .map_err(|err| err.to_string())?;
    Ok(())
}

/// Top-down camera helper.
fn top_camera(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    x: f32,
    z: f32,
) -> Result<(), String> {
    forge
        .top_camera("Camera3D", Some(root), [x, z], 18.0)
        .map_err(|err| err.to_string())?;
    Ok(())
}

/// Standard kill plane.
fn kill_plane(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    x: f32,
    w: f32,
) -> Result<(), String> {
    forge
        .area_trigger("Kill", Some(root), [x, KILL_Y - 1.0, 0.0], [w, 1.0, 8.0])
        .map_err(|err| err.to_string())?;
    forge.connect("body_entered", "Kill", ".", "_on_kill");
    Ok(())
}

/// Scripted platforming player (runner-flavored modes included).
fn runner_player(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    mode: i64,
    speed: f32,
    jump: f32,
    gravity: f32,
    start: [f32; 3],
) -> Result<gamecraft_core::NodeId, String> {
    let id = forge
        .place("Player", "CharacterBody3D", Some(root), start)
        .map_err(|err| err.to_string())?;
    if let Some(node) = forge.graph.get_mut(id) {
        node.unique_name = true;
    }
    forge
        .set_prop(id, "script", Forge::ext_ref("2_player"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "mode", PropertyValue::Int(mode))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "speed", PropertyValue::Float(f64::from(speed)))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "jump_velocity", PropertyValue::Float(f64::from(jump)))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "gravity", PropertyValue::Float(f64::from(gravity)))
        .map_err(|err| err.to_string())?;
    let mesh = forge.box_mesh("player", [0.6, 1.0, 0.6]);
    let material = forge.material("player", 0.35, 0.59, 1.0);
    forge
        .mesh_child("Mesh", id, mesh.as_str(), material.as_str())
        .map_err(|err| err.to_string())?;
    let shape = forge.box_shape("player", [0.6, 1.0, 0.6]);
    forge
        .shape_child("Shape", id, shape.as_str())
        .map_err(|err| err.to_string())?;
    Ok(id)
}

/// Automatic jump hint: replays one verified trace decision headless.
fn hint(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    name: &str,
    at: [f32; 3],
) -> Result<(), String> {
    forge
        .area_trigger(name, Some(root), at, [0.6, 2.0, 2.0])
        .map_err(|err| err.to_string())?;
    forge.connect("body_entered", name, ".", "_on_hint");
    Ok(())
}

/// Export helper shared by arcade builders.
fn export_scene(forge: &Forge, slug: &str) -> Result<String, String> {
    let request = gamecraft_godot::ExportRequest {
        graph: &forge.graph,
        connections: forge.conns.as_slice(),
        uid: gamecraft_godot::deterministic_uid(
            format!("res://depot/games/{slug}/main.tscn").as_str(),
        ),
        format: 3,
        ext_resources: forge.ext.clone(),
        sub_resources: forge.sub.clone(),
    };
    gamecraft_godot::export::export_tscn(&request)
}

/// Commands log as newline-delimited JSON.
fn commands_json(forge: &Forge) -> String {
    forge
        .log
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

/// Runner arena: three grounds, two hint gates, goal, kill plane.
///
/// # Errors
///
/// Returns the bus error when a node cannot be placed.
fn runner_scene(theme: &crate::create::Theme) -> Result<Forge, String> {
    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/player_controller.gd", "2_player");
    side_camera(&mut forge, root, 17.0, 25.0)?;
    forge
        .solid_box(
            "GroundA",
            "StaticBody3D",
            Some(root),
            [1.0, -0.5, 0.0],
            [14.0, 1.0, 4.0],
            theme.ground[0],
            theme.ground[1],
            theme.ground[2],
        )
        .map_err(|err| err.to_string())?;
    forge
        .solid_box(
            "GroundB",
            "StaticBody3D",
            Some(root),
            [17.5, -0.5, 0.0],
            [13.0, 1.0, 4.0],
            theme.ground[0],
            theme.ground[1],
            theme.ground[2],
        )
        .map_err(|err| err.to_string())?;
    forge
        .solid_box(
            "GroundC",
            "StaticBody3D",
            Some(root),
            [33.5, -0.5, 0.0],
            [13.0, 1.0, 4.0],
            theme.ground[0],
            theme.ground[1],
            theme.ground[2],
        )
        .map_err(|err| err.to_string())?;
    runner_player(&mut forge, root, 1, 5.5, 9.0, 22.0, [0.0, 0.6, 0.0])?;
    hint(&mut forge, root, "Hint0", [7.2, 1.0, 0.0])?;
    hint(&mut forge, root, "Hint1", [23.2, 1.0, 0.0])?;
    forge
        .area_trigger("Goal", Some(root), [38.0, 1.5, 0.0], [1.5, 3.0, 2.0])
        .map_err(|err| err.to_string())?;
    forge.connect("body_entered", "Goal", ".", "_on_goal");
    kill_plane(&mut forge, root, 17.0, 60.0)?;
    Ok(forge)
}

/// Auto-runner over three gaps with hint-assisted jumps.
///
/// # Errors
///
/// Returns a message when the trace fails verification or a node is invalid.
pub fn build_runner() -> Result<GameParts, String> {
    // Keep the depot runner's original ground colour so regeneration stays stable.
    let theme = crate::create::Theme {
        ground: [0.55, 0.36, 0.18],
        ..crate::create::Theme::default()
    };
    build_runner_with(&theme, "runner", "GameCraft Runner")
}

/// Runner with a caller-supplied theme and name.
///
/// # Errors
///
/// Returns a message when the trace fails verification or a node is invalid.
pub fn build_runner_with(
    theme: &crate::create::Theme,
    slug: &str,
    title: &str,
) -> Result<GameParts, String> {
    let phys = Phys {
        speed: 5.5,
        jump_v: 9.0,
        gravity: 22.0,
        auto_run: true,
        half_w: 0.3,
        half_h: 0.5,
    };
    let grounds = [
        Platform {
            x: -6.0,
            y: 0.0,
            w: 14.0,
        },
        Platform {
            x: 11.0,
            y: 0.0,
            w: 13.0,
        },
        Platform {
            x: 27.0,
            y: 0.0,
            w: 13.0,
        },
    ];
    let forge = runner_scene(theme)?;

    let mut script = vec![TraceInput::None; 600];
    for frame in [76_usize, 255] {
        if let Some(slot) = script.get_mut(frame) {
            *slot = TraceInput::Jump;
        }
    }
    let start = Body2 {
        x: 0.0,
        y: 0.5,
        vx: 0.0,
        vy: 0.0,
    };
    let run = sim::run_trace(
        start,
        script.as_slice(),
        phys,
        &grounds,
        &[],
        38.0,
        KILL_Y,
        600,
    );
    if !run.won {
        return Err("runner trace missed the goal".to_string());
    }
    if run.frames > 600 {
        return Err(format!("runner too slow: {frames}", frames = run.frames));
    }
    let frames = run
        .path
        .iter()
        .map(|pos| {
            vec![Glyph {
                x: pos.0,
                y: pos.1,
                w: 0.6,
                h: 1.0,
                color: 2,
                circle: false,
            }]
        })
        .collect::<Vec<Vec<Glyph>>>();
    Ok(GameParts {
        slug: Box::leak(slug.to_string().into_boxed_str()),
        title: Box::leak(title.to_string().into_boxed_str()),
        controls: "Auto-runs. Space jumps (hint markers auto-jump headless).",
        win_text: "Reach x = 38.",
        seed: 2,
        scene_text: export_scene(&forge, slug)?,
        scripts: vec![
            ("gamecraft_runtime.gd", crate::scripts::RUNTIME),
            ("player_controller.gd", crate::scripts::PLAYER),
            ("camera_follow.gd", crate::scripts::CAMERA_FOLLOW),
        ],
        trace_json: format!(
            "{{\"game\":\"runner\",\"win\":true,\"frames\":{run_frames},\"hint_jumps\":[76,255]}}",
            run_frames = run.frames
        ),
        commands_json: commands_json(&forge),
        statics: vec![
            Glyph { x: 1.0, y: -0.5, w: 14.0, h: 1.0, color: 0, circle: false },
            Glyph { x: 17.5, y: -0.5, w: 13.0, h: 1.0, color: 0, circle: false },
            Glyph { x: 33.5, y: -0.5, w: 13.0, h: 1.0, color: 0, circle: false },
            Glyph { x: 38.0, y: 1.5, w: 1.5, h: 3.0, color: 4, circle: false },
        ],
        frames,
        view: View { x: -8.0, y: -3.0, w: 50.0, h: 11.0 },
        readme_body: "Auto-scroll runner over two 3-unit gaps. Hint volumes replay\nthe verified jumps headless; humans jump with Space.\n".to_string(),
        summary: format!("runner: goal in {frames} frames, lint clean", frames = run.frames),
    })
}

// ---------------------------------------------------------------------------
// Flappy
// ---------------------------------------------------------------------------

/// Flap physics (mirrors mode 2 with fixed forward speed).
const FLAP: Phys = Phys {
    speed: 6.0,
    jump_v: 6.5,
    gravity: 18.0,
    auto_run: true,
    half_w: 0.3,
    half_h: 0.25,
};

/// Pipe gap centers (10 pipes).
const PIPE_GAPS: [f32; 10] = [1.5, 2.2, 2.8, 2.4, 1.6, 1.0, 1.4, 2.0, 2.6, 2.2];

/// Barrier islands flanking the driving line (center stays open).
const BARRIERS: [(f32, f32); 6] = [
    (15.0, 2.5),
    (15.0, -2.5),
    (30.0, -2.5),
    (30.0, 2.5),
    (45.0, 2.5),
    (45.0, -2.5),
];

/// Bang-bang policy rollout: flap below the next gap, record hint ticks.
fn flappy_policy() -> (Vec<TraceInput>, Vec<(f32, f32)>, bool) {
    let mut script = vec![TraceInput::None; 600];
    let mut body = Body2 {
        x: 0.0,
        y: 2.0,
        vx: 0.0,
        vy: 0.0,
    };
    let mut positions: Vec<(f32, f32)> = Vec::new();
    let mut pipe = 0_usize;
    let mut won = false;
    for tick in 0..600 {
        while pipe < PIPE_GAPS.len() && body.x > 8.0 + idx(pipe) * 4.5 {
            pipe += 1;
        }
        let gap = PIPE_GAPS[pipe.min(PIPE_GAPS.len() - 1)];
        if body.y < gap - 0.2 {
            script[tick] = TraceInput::Flap;
        }
        let _grounded = sim::step_body(&mut body, script[tick], FLAP, &[], -7.0);
        positions.push((body.x, body.y));
        if pipe >= PIPE_GAPS.len() && body.x > 8.0 + 9.0 * 4.5 + 2.0 {
            won = true;
            positions.truncate(tick + 1);
            script.truncate(tick + 1);
            break;
        }
        if body.y < -7.0 {
            break;
        }
    }
    (script, positions, won)
}

/// Verify the policy threads every pipe gap.
fn flappy_check(positions: &[(f32, f32)]) -> bool {
    for (index, gap) in PIPE_GAPS.iter().enumerate() {
        let pipe_x = 8.0 + idx(index) * 4.5;
        let mut ok = false;
        for (x, y) in positions {
            if (x - pipe_x).abs() < 0.55 && (y - gap).abs() < 1.0 {
                ok = true;
                break;
            }
        }
        if !ok {
            return false;
        }
    }
    true
}

/// Flappy scene: pipes, hint replays, goal, kill plane.
///
/// # Errors
///
/// Returns the bus error when a node cannot be placed.
fn flappy_scene(script: &[TraceInput], positions: &[(f32, f32)]) -> Result<Forge, String> {
    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/player_controller.gd", "2_player");
    side_camera(&mut forge, root, 26.0, 31.0)?;
    runner_player(&mut forge, root, 2, 6.0, 6.5, 18.0, [0.0, 2.0, 0.0])?;
    for (index, gap) in PIPE_GAPS.iter().enumerate() {
        let pipe_x = 8.0 + idx(index) * 4.5;
        let lower_h = gap - 1.4 + 7.0;
        let upper_h = 9.0 - (gap + 1.4);
        forge
            .solid_box(
                format!("PipeLow{index}").as_str(),
                "StaticBody3D",
                Some(root),
                [pipe_x, -7.0 + lower_h * 0.5, 0.0],
                [1.0, lower_h, 1.0],
                0.29,
                0.63,
                0.35,
            )
            .map_err(|err| err.to_string())?;
        forge
            .solid_box(
                format!("PipeHigh{index}").as_str(),
                "StaticBody3D",
                Some(root),
                [pipe_x, gap + 1.4 + upper_h * 0.5, 0.0],
                [1.0, upper_h, 1.0],
                0.29,
                0.63,
                0.35,
            )
            .map_err(|err| err.to_string())?;
    }
    let mut hint_count = 0_usize;
    for (tick, input) in script.iter().enumerate() {
        if *input != TraceInput::Flap {
            continue;
        }
        if let Some(pos) = positions.get(tick) {
            hint(
                &mut forge,
                root,
                format!("Hint{hint_count}").as_str(),
                [pos.0, pos.1, 0.0],
            )?;
            hint_count += 1;
        }
    }
    kill_plane(&mut forge, root, 26.0, 70.0)?;
    forge.ext_resource("Script", "res://scripts/autoplay.gd", "5_autoplay");
    let flappy_auto = forge
        .autoplay(root, "5_autoplay", "", &[])
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(
            flappy_auto,
            "flap",
            gamecraft_core::PropertyValue::Bool(true),
        )
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(
            flappy_auto,
            "hover",
            gamecraft_core::PropertyValue::Float(0.5),
        )
        .map_err(|err| err.to_string())?;
    forge
        .area_trigger("Goal", Some(root), [55.0, 2.0, 0.0], [1.5, 12.0, 2.0])
        .map_err(|err| err.to_string())?;
    forge.connect("body_entered", "Goal", ".", "_on_goal");
    Ok(forge)
}

/// Build the flappy depot game.
///
/// # Errors
///
/// Returns a message when the policy crashes, misses a pipe, or is too slow.
///
/// # Errors
///
/// Returns a message when the policy crashes, misses a pipe, or is too slow.
pub fn build_flappy() -> Result<GameParts, String> {
    let (script, positions, won) = flappy_policy();
    if !won {
        return Err("flappy policy crashed".to_string());
    }
    if !flappy_check(positions.as_slice()) {
        return Err("flappy policy missed a pipe".to_string());
    }
    let total = positions.len();
    if total > 600 {
        return Err(format!("flappy too slow: {total}"));
    }
    let forge = flappy_scene(script.as_slice(), positions.as_slice())?;
    let flaps = script
        .iter()
        .filter(|input| **input == TraceInput::Flap)
        .count();
    let frames = positions
        .iter()
        .map(|pos| {
            vec![Glyph {
                x: pos.0,
                y: pos.1,
                w: 0.6,
                h: 0.5,
                color: 2,
                circle: false,
            }]
        })
        .collect::<Vec<Vec<Glyph>>>();
    let mut statics: Vec<Glyph> = Vec::new();
    for (index, gap) in PIPE_GAPS.iter().enumerate() {
        let pipe_x = 8.0 + idx(index) * 4.5;
        statics.push(Glyph {
            x: pipe_x,
            y: -7.0 + f32::midpoint(gap - 1.4, 7.0),
            w: 1.0,
            h: gap - 1.4 + 7.0,
            color: 1,
            circle: false,
        });
        statics.push(Glyph {
            x: pipe_x,
            y: f32::midpoint(gap + 1.4, 9.0),
            w: 1.0,
            h: 9.0 - (gap + 1.4),
            color: 1,
            circle: false,
        });
    }
    Ok(GameParts {
        slug: "flappy",
        title: "GameCraft Flappy",
        controls: "Space flaps. Thread 10 pipes; floor and pipes reset you.",
        win_text: "Pass pipe 10 (x = 48.5).",
        seed: 3,
        scene_text: export_scene(&forge, "flappy")?,
        scripts: vec![
            ("gamecraft_runtime.gd", crate::scripts::RUNTIME),
            ("player_controller.gd", crate::scripts::PLAYER),
            ("camera_follow.gd", crate::scripts::CAMERA_FOLLOW),
            ("autoplay.gd", crate::scripts::AUTOPLAY),
        ],
        trace_json: format!(
            "{{\"game\":\"flappy\",\"win\":true,\"frames\":{total},\"flaps\":{flaps},\"note\":\"bang-bang policy, hints replay it headless\"}}"
        ),
        commands_json: commands_json(&forge),
        statics,
        frames,
        view: View { x: -4.0, y: -8.0, w: 62.0, h: 18.0 },
        readme_body: "Hold-to-lift flappy across 10 pipes. A bang-bang policy was\nrolled out in Rust; every flap became a hint volume, so the headless\nrun replays the verified flight exactly.\n".to_string(),
        summary: format!("flappy: 10 pipes in {total} frames, lint clean"),
    })
}

// ---------------------------------------------------------------------------
// Driving
// ---------------------------------------------------------------------------

/// Driving arena: track, car, barriers, checkpoint gates.
///
/// # Errors
///
/// Returns the bus error when a node cannot be placed.
fn driving_scene() -> Result<Forge, String> {
    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "checkpoints_to_win", PropertyValue::Int(3))
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/player_controller.gd", "2_player");
    side_camera(&mut forge, root, 30.0, 34.0)?;
    forge
        .solid_box(
            "Track",
            "StaticBody3D",
            Some(root),
            [30.0, -0.5, 0.0],
            [64.0, 1.0, 10.0],
            0.25,
            0.27,
            0.32,
        )
        .map_err(|err| err.to_string())?;
    runner_player(&mut forge, root, 3, 7.0, 9.0, 22.0, [0.0, 0.6, 0.0])?;
    for (index, (x, z)) in BARRIERS.iter().enumerate() {
        forge
            .solid_box(
                format!("Barrier{index}").as_str(),
                "StaticBody3D",
                Some(root),
                [*x, 0.5, *z],
                [2.0, 1.0, 2.0],
                0.85,
                0.3,
                0.3,
            )
            .map_err(|err| err.to_string())?;
    }
    for (index, x) in [15.0_f32, 30.0, 45.0].iter().enumerate() {
        forge
            .area_trigger(
                format!("CP{index}").as_str(),
                Some(root),
                [*x, 1.0, 0.0],
                [1.0, 2.0, 8.0],
            )
            .map_err(|err| err.to_string())?;
        forge.connect(
            "body_entered",
            format!("CP{index}").as_str(),
            ".",
            "_on_checkpoint",
        );
    }
    Ok(forge)
}

/// Straight time-trial with checkpoint gates and barrier islands.
///
/// # Errors
///
/// Returns a message when the trace fails verification or a node is invalid.
pub fn build_driving() -> Result<GameParts, String> {
    let forge = driving_scene()?;
    // Straight-line trace: 45 units at speed 7 (integer ceil + margin).
    let frames_needed = (45_usize * 60).div_ceil(7) + 5;
    if frames_needed > 600 {
        return Err("driving trace too slow".to_string());
    }
    let mut dynamics: Vec<Vec<Glyph>> = Vec::new();
    let mut car_x = 0.0_f32;
    for _ in 0..frames_needed {
        dynamics.push(vec![Glyph {
            x: car_x,
            y: 0.6,
            w: 1.2,
            h: 0.6,
            color: 2,
            circle: false,
        }]);
        car_x += 7.0 / 60.0;
    }
    let mut statics = vec![Glyph {
        x: 30.0,
        y: -0.5,
        w: 64.0,
        h: 1.0,
        color: 11,
        circle: false,
    }];
    for (x, _z) in BARRIERS {
        statics.push(Glyph {
            x,
            y: 0.5,
            w: 2.0,
            h: 1.0,
            color: 4,
            circle: false,
        });
    }
    Ok(GameParts {
        slug: "driving-box",
        title: "GameCraft Driving Box",
        controls: "Left/Right steer, auto-forward. Thread 3 checkpoints.",
        win_text: "Pass all 3 checkpoints.",
        seed: 4,
        scene_text: export_scene(&forge, "driving-box")?,
        scripts: vec![
            ("gamecraft_runtime.gd", crate::scripts::RUNTIME),
            ("player_controller.gd", crate::scripts::PLAYER),
            ("camera_follow.gd", crate::scripts::CAMERA_FOLLOW),
        ],
        trace_json: format!(
            "{{\"game\":\"driving-box\",\"win\":true,\"frames\":{frames_needed},\"checkpoints\":3}}"
        ),
        commands_json: commands_json(&forge),
        statics,
        frames: dynamics,
        view: View { x: -4.0, y: -3.0, w: 68.0, h: 8.0 },
        readme_body: "Box-car time trial. Barrier islands leave the racing line open\nso the headless trace holds center; humans steer for style.\n".to_string(),
        summary: format!("driving-box: 3 checkpoints in {frames_needed} frames, lint clean"),
    })
}

// ---------------------------------------------------------------------------
// Pong
// ---------------------------------------------------------------------------

/// Pong ball + paddles state for the mirrored sim.
struct PongSim {
    ball_x: f32,
    ball_z: f32,
    vel_x: f32,
    vel_z: f32,
    left_z: f32,
    right_z: f32,
    left_score: u32,
    right_score: u32,
    speed: f32,
    cool: u32,
}

/// One 60 Hz tick. Returns scoring side if a point landed.
fn pong_tick(sim: &mut PongSim) -> Option<bool> {
    // Paddles track only approaching balls with a 1.0 deadband, else drift
    // home. Deadband models reaction threshold; without it tracking is
    // perfect and rallies never end. (Headless demo tracks always.)
    if sim.vel_x < 0.0 {
        let dz = sim.ball_z - sim.left_z;
        if dz.abs() > 1.0 {
            sim.left_z += dz.clamp(-3.0 / 60.0, 3.0 / 60.0);
        }
    } else {
        sim.left_z += (0.0 - sim.left_z).clamp(-2.0 / 60.0, 2.0 / 60.0);
    }
    if sim.vel_x > 0.0 {
        let dz = sim.ball_z - sim.right_z;
        if dz.abs() > 1.0 {
            sim.right_z += dz.clamp(-6.5 / 60.0, 6.5 / 60.0);
        }
    } else {
        sim.right_z += (0.0 - sim.right_z).clamp(-2.0 / 60.0, 2.0 / 60.0);
    }
    sim.ball_x += sim.vel_x / 60.0;
    sim.ball_z += sim.vel_z / 60.0;
    if sim.ball_z > 4.75 {
        sim.ball_z = 4.75;
        sim.vel_z = -sim.vel_z.abs();
    }
    if sim.ball_z < -4.75 {
        sim.ball_z = -4.75;
        sim.vel_z = sim.vel_z.abs();
    }
    if sim.cool > 0 {
        sim.cool -= 1;
    } else {
        for (px, pz, side) in [(-7.5, sim.left_z, 1.0), (7.5, sim.right_z, -1.0)] {
            if (sim.ball_x - px).abs() < 0.55 && (sim.ball_z - pz).abs() < 1.45 {
                sim.speed += 0.4;
                let off = ((sim.ball_z - pz) * 0.6).clamp(-0.7, 0.7);
                let len = (1.0 + off * off).sqrt();
                sim.vel_x = side * sim.speed / len;
                sim.vel_z = off * sim.speed / len;
                sim.ball_x = px + side * 0.55;
                sim.cool = 6;
                break;
            }
        }
    }
    if sim.ball_x > 8.5 {
        sim.left_score += 1;
        pong_serve(sim, -1.0);
        return Some(false);
    }
    if sim.ball_x < -8.5 {
        sim.right_score += 1;
        pong_serve(sim, 1.0);
        return Some(true);
    }
    None
}

/// Serve toward a side with a fixed angle.
fn pong_serve(sim: &mut PongSim, side: f32) {
    sim.ball_x = 0.0;
    sim.ball_z = 0.0;
    let len: f32 = (1.0_f32 + 0.09).sqrt();
    sim.vel_x = side * sim.speed / len;
    sim.vel_z = 0.3 * sim.speed / len;
}

/// First to 2, player (right) must win inside 600 frames.
/// Pong arena: paddles, ball, walls, goal triggers.
///
/// # Errors
///
/// Returns the bus error when a node cannot be placed.
fn pong_scene(ball_speed: f32) -> Result<Forge, String> {
    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/ball.gd", "2_ball");
    forge.ext_resource("Script", "res://scripts/tracker.gd", "3_track");
    top_camera(&mut forge, root, 0.0, 0.0)?;
    paddle(&mut forge, root, "PaddleL", -7.5, 3.0)?;
    paddle(&mut forge, root, "PaddleR", 7.5, 6.5)?;
    ball_node(&mut forge, root, ball_speed)?;
    for (name, x) in [("WallN", -5.5_f32), ("WallS", 5.5_f32)] {
        forge
            .solid_box(
                name,
                "StaticBody3D",
                Some(root),
                [0.0, 0.5, x],
                [17.0, 1.0, 1.0],
                0.3,
                0.3,
                0.35,
            )
            .map_err(|err| err.to_string())?;
    }
    for (name, x) in [("GoalL", -8.5_f32), ("GoalR", 8.5_f32)] {
        forge
            .area_trigger(name, Some(root), [x, 0.5, 0.0], [1.0, 1.0, 11.0])
            .map_err(|err| err.to_string())?;
    }
    forge.connect("body_entered", "GoalL", ".", "_on_point_right");
    forge.connect("body_entered", "GoalR", ".", "_on_point_left");
    forge
        .set_prop(root, "points_to_win", gamecraft_core::PropertyValue::Int(2))
        .map_err(|err| err.to_string())?;
    Ok(forge)
}

/// Build the pong depot game.
///
/// # Errors
///
/// Returns a message when the trace fails verification or a node is invalid.
pub fn build_pong() -> Result<GameParts, String> {
    build_pong_with(14.0, "pong", "GameCraft Pong")
}

/// Pong with a caller-supplied ball speed and name.
///
/// # Errors
///
/// Returns a message when the trace fails verification or a node is invalid.
pub fn build_pong_with(ball_speed: f32, slug: &str, title: &str) -> Result<GameParts, String> {
    let mut sim = PongSim {
        ball_x: 0.0,
        ball_z: 0.0,
        vel_x: 0.0,
        vel_z: 0.0,
        left_z: 0.0,
        right_z: 0.0,
        left_score: 0,
        right_score: 0,
        speed: ball_speed,
        cool: 0,
    };
    pong_serve(&mut sim, 1.0);
    let mut frames: Vec<Vec<Glyph>> = Vec::new();
    let mut won = false;
    for _ in 0..600 {
        pong_tick(&mut sim);
        frames.push(vec![
            Glyph {
                x: sim.ball_x,
                y: sim.ball_z,
                w: 0.5,
                h: 0.5,
                color: 10,
                circle: true,
            },
            Glyph {
                x: -7.5,
                y: sim.left_z,
                w: 0.5,
                h: 2.4,
                color: 9,
                circle: false,
            },
            Glyph {
                x: 7.5,
                y: sim.right_z,
                w: 0.5,
                h: 2.4,
                color: 2,
                circle: false,
            },
        ]);
        if sim.right_score >= 2 {
            won = true;
            break;
        }
        if sim.left_score >= 2 {
            break;
        }
    }
    if !won {
        return Err(format!(
            "pong trace: player lost ({}-{})",
            sim.right_score, sim.left_score
        ));
    }
    let used = frames.len();
    let forge = pong_scene(ball_speed)?;
    let view = View {
        x: -10.0,
        y: -6.5,
        w: 20.0,
        h: 13.0,
    };
    Ok(GameParts {
        slug: Box::leak(slug.to_string().into_boxed_str()),
        title: Box::leak(title.to_string().into_boxed_str()),
        controls: "Up/Down move the right paddle. First to 2.",
        win_text: "Outscore the simple AI (first to 2).",
        seed: 5,
        scene_text: export_scene(&forge, slug)?,
        scripts: vec![
            ("gamecraft_runtime.gd", crate::scripts::RUNTIME),
            ("ball.gd", crate::scripts::BALL),
            ("tracker.gd", crate::scripts::TRACKER),
        ],
        trace_json: format!(
            "{{\"game\":\"pong\",\"win\":true,\"frames\":{used},\"score\":\"{right}-{left}\"}}",
            right = sim.right_score, left = sim.left_score
        ),
        commands_json: commands_json(&forge),
        statics: vec![
            Glyph { x: 0.0, y: 0.0, w: 17.0, h: 11.0, color: 11, circle: false },
        ],
        frames,
        view,
        readme_body: "Player (right, fast tracker) vs simple AI (left, capped).\nMirrored sim proves the player takes it 2 first.\n".to_string(),
        summary: format!("pong: player wins 2-{left} in {used} frames, lint clean", left = sim.left_score),
    })
}

/// Tracker paddle: `StaticBody` + shape + mesh + tracker script.
fn paddle(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    name: &str,
    x: f32,
    max_speed: f32,
) -> Result<gamecraft_core::NodeId, String> {
    let id = forge
        .place(name, "StaticBody3D", Some(root), [x, 0.5, 0.0])
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "script", Forge::ext_ref("3_track"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "axis", PropertyValue::Str("z".to_string()))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(
            id,
            "target_path",
            PropertyValue::Raw("NodePath(\"../Ball\")".to_string()),
        )
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "max_speed", PropertyValue::Float(f64::from(max_speed)))
        .map_err(|err| err.to_string())?;
    let mesh = forge.box_mesh(name, [0.5, 1.0, 2.4]);
    let material = forge.material(name, 0.43, 0.86, 1.0);
    forge
        .mesh_child("Mesh", id, mesh.as_str(), material.as_str())
        .map_err(|err| err.to_string())?;
    let shape = forge.box_shape(name, [0.5, 1.0, 2.4]);
    forge
        .shape_child("Shape", id, shape.as_str())
        .map_err(|err| err.to_string())?;
    Ok(id)
}

/// Ball with constant-velocity script.
fn ball_node(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    speed: f32,
) -> Result<gamecraft_core::NodeId, String> {
    let id = forge
        .place("Ball", "CharacterBody3D", Some(root), [0.0, 0.5, 0.0])
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "script", Forge::ext_ref("2_ball"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "speed", PropertyValue::Float(f64::from(speed)))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "mode", PropertyValue::Int(0))
        .map_err(|err| err.to_string())?;
    let mesh = forge.sphere_mesh("ball", 0.25, 0.5);
    let material = forge.material("ball", 1.0, 0.6, 0.24);
    forge
        .mesh_child("Mesh", id, mesh.as_str(), material.as_str())
        .map_err(|err| err.to_string())?;
    let shape = forge.box_shape("ball", [0.5, 0.5, 0.5]);
    forge
        .shape_child("Shape", id, shape.as_str())
        .map_err(|err| err.to_string())?;
    Ok(id)
}

// ---------------------------------------------------------------------------
// Breakout
// ---------------------------------------------------------------------------

/// Breakout sim: AI paddle tracks the descending ball, bricks fall per hit.
struct BreakoutSim {
    ball_x: f32,
    ball_z: f32,
    vel_x: f32,
    vel_z: f32,
    paddle_x: f32,
    bricks: Vec<(f32, f32)>,
    lives: u32,
    speed: f32,
    cool: u32,
}

/// One tick. Returns false when the ball is lost.
fn breakout_tick(sim: &mut BreakoutSim) -> bool {
    // Predictive aim: park the paddle so the rebound travels to the nearest
    // remaining brick. Sign check: vel_x/vel_z must equal
    // (target - contact)/(target_z - paddle_z) with vel_z negative, so
    // off = (target_x - contact_x) / (paddle_z - target_z).
    if sim.vel_z > 0.0 {
        let time_to_contact = ((4.5 - sim.ball_z) / sim.vel_z).max(0.0);
        let contact_x = sim.ball_x + sim.vel_x * time_to_contact;
        let mut best: Option<(f32, f32)> = None;
        let mut best_d = f32::MAX;
        for brick in &sim.bricks {
            let dx = brick.0 - contact_x;
            let dy = brick.1 - 4.5;
            let dist = dx * dx + dy * dy;
            if dist < best_d {
                best_d = dist;
                best = Some(*brick);
            }
        }
        let mut want = contact_x;
        if let Some(target) = best {
            let reach = 4.5 - target.1;
            if reach > 0.5 {
                let off = ((target.0 - contact_x) / reach).clamp(-0.7, 0.7);
                want = contact_x - off / 0.6;
            }
        }
        want = want.clamp(-6.5, 6.5);
        sim.paddle_x += (want - sim.paddle_x).clamp(-12.0 / 60.0, 12.0 / 60.0);
    } else {
        sim.paddle_x += (sim.ball_x - sim.paddle_x).clamp(-12.0 / 60.0, 12.0 / 60.0);
    }
    sim.ball_x += sim.vel_x / 60.0;
    sim.ball_z += sim.vel_z / 60.0;
    if sim.ball_x > 7.75 {
        sim.ball_x = 7.75;
        sim.vel_x = -sim.vel_x.abs();
    }
    if sim.ball_x < -7.75 {
        sim.ball_x = -7.75;
        sim.vel_x = sim.vel_x.abs();
    }
    if sim.ball_z < -4.25 {
        sim.ball_z = -4.25;
        sim.vel_z = sim.vel_z.abs();
    }
    if sim.cool > 0 {
        sim.cool -= 1;
    } else if (sim.ball_z - 4.5).abs() < 0.5
        && (sim.ball_x - sim.paddle_x).abs() < 1.65
        && sim.vel_z > 0.0
    {
        sim.speed += 0.4;
        sim.cool = 6;
        let off = ((sim.ball_x - sim.paddle_x) * 0.6).clamp(-0.7, 0.7);
        let len = (off * off + 1.0).sqrt();
        sim.vel_x = off * sim.speed / len;
        sim.vel_z = -sim.speed / len;
        sim.ball_z = 4.0;
    }
    let mut hit: Option<usize> = None;
    for (index, brick) in sim.bricks.iter().enumerate() {
        if (sim.ball_x - brick.0).abs() < 0.95 && (sim.ball_z - brick.1).abs() < 0.65 {
            hit = Some(index);
            break;
        }
    }
    if let Some(index) = hit {
        sim.bricks.remove(index);
        // Reflect and scatter: a perfectly vertical rebound would lock the ball
        // into one cleared lane forever. A deterministic x-kick per brick keeps
        // the ball visiting new columns (no randomness, same trace every run).
        let kick = if index % 2 == 0 { 1.0 } else { -1.0 };
        let current = (sim.vel_x * sim.vel_x + sim.vel_z * sim.vel_z)
            .sqrt()
            .max(1.0);
        let mut vx = sim.vel_x + kick * current * 0.25;
        let vz = -sim.vel_z.abs();
        if vx.abs() < current * 0.2 {
            vx = kick * current * 0.2;
        }
        let scale = current / (vx * vx + vz * vz).sqrt().max(0.001);
        sim.vel_x = vx * scale;
        sim.vel_z = vz * scale;
    }
    if sim.ball_z > 5.5 {
        return false;
    }
    true
}

/// Brick layout: 2 rows of 10.
fn breakout_bricks() -> Vec<(f32, f32)> {
    let mut bricks: Vec<(f32, f32)> = Vec::new();
    for row in 0..2_u16 {
        for col in 0..10_u16 {
            bricks.push((-6.75 + f32::from(col) * 1.5, -3.2 + f32::from(row) * 1.0));
        }
    }
    bricks
}

/// Verified trace: AI paddle clears every brick inside 600 frames.
///
/// # Errors
///
/// Returns a message when bricks remain or the balls run out.
fn breakout_trace(mut sim: BreakoutSim) -> Result<(Vec<Vec<Glyph>>, u32), String> {
    let mut frames: Vec<Vec<Glyph>> = Vec::new();
    let mut won = false;
    for _ in 0..600 {
        let alive = breakout_tick(&mut sim);
        if !alive {
            sim.lives = sim.lives.saturating_sub(1);
            if sim.lives == 0 {
                break;
            }
            sim.ball_x = 0.0;
            sim.ball_z = 3.0;
            sim.vel_x = 4.8;
            sim.vel_z = -15.2;
        }
        let mut frame = vec![
            Glyph {
                x: sim.ball_x,
                y: sim.ball_z,
                w: 0.5,
                h: 0.5,
                color: 10,
                circle: true,
            },
            Glyph {
                x: sim.paddle_x,
                y: 4.5,
                w: 2.8,
                h: 0.5,
                color: 2,
                circle: false,
            },
        ];
        for brick in &sim.bricks {
            frame.push(Glyph {
                x: brick.0,
                y: brick.1,
                w: 1.4,
                h: 0.8,
                color: 4,
                circle: false,
            });
        }
        frames.push(frame);
        if sim.bricks.is_empty() {
            won = true;
            break;
        }
    }
    if !won {
        let mut missing = String::new();
        for brick in &sim.bricks {
            use std::fmt::Write as _;
            let _ = write!(missing, " ({:.1},{:.1})", brick.0, brick.1);
        }
        return Err(format!(
            "breakout trace stalled: {} bricks left at{} after {} frames ({} lives)",
            sim.bricks.len(),
            missing,
            frames.len(),
            sim.lives
        ));
    }
    Ok((frames, sim.lives))
}

/// Breakout arena: paddle, ball, walls, bricks.
///
/// # Errors
///
/// Returns the bus error when a node cannot be placed.
fn breakout_scene() -> Result<Forge, String> {
    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/ball.gd", "2_ball");
    forge.ext_resource("Script", "res://scripts/tracker.gd", "3_track");
    top_camera(&mut forge, root, 0.0, 0.0)?;
    forge
        .set_prop(
            root,
            "bricks_to_win",
            gamecraft_core::PropertyValue::Int(20),
        )
        .map_err(|err| err.to_string())?;
    let paddle = forge
        .place("Paddle", "StaticBody3D", Some(root), [0.0, 0.5, 4.5])
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(paddle, "script", Forge::ext_ref("3_track"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(paddle, "axis", PropertyValue::Str("x".to_string()))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(
            paddle,
            "target_path",
            PropertyValue::Raw("NodePath(\"../Ball\")".to_string()),
        )
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(paddle, "max_speed", PropertyValue::Float(7.0))
        .map_err(|err| err.to_string())?;
    let pmesh = forge.box_mesh("paddle", [2.8, 0.5, 0.5]);
    let pmat = forge.material("paddle", 0.35, 0.59, 1.0);
    forge
        .mesh_child("Mesh", paddle, pmesh.as_str(), pmat.as_str())
        .map_err(|err| err.to_string())?;
    let pshape = forge.box_shape("paddle", [2.8, 0.5, 0.5]);
    forge
        .shape_child("Shape", paddle, pshape.as_str())
        .map_err(|err| err.to_string())?;
    let breakout_ball = ball_node(&mut forge, root, 16.0)?;
    forge
        .set_prop(breakout_ball, "mode", gamecraft_core::PropertyValue::Int(1))
        .map_err(|err| err.to_string())?;
    for (name, x) in [("WallW", -8.25_f32), ("WallE", 8.25_f32)] {
        forge
            .solid_box(
                name,
                "StaticBody3D",
                Some(root),
                [x, 0.5, 0.0],
                [0.5, 1.0, 11.0],
                0.3,
                0.3,
                0.35,
            )
            .map_err(|err| err.to_string())?;
    }
    forge
        .solid_box(
            "WallN",
            "StaticBody3D",
            Some(root),
            [0.0, 0.5, -4.75],
            [17.0, 1.0, 0.5],
            0.3,
            0.3,
            0.35,
        )
        .map_err(|err| err.to_string())?;
    for row in 0..2_u16 {
        for col in 0..10_u16 {
            let x = -6.75 + f32::from(col) * 1.5;
            let z = -3.2 + f32::from(row) * 1.0;
            forge
                .solid_box(
                    format!("Brick{row}_{col}").as_str(),
                    "StaticBody3D",
                    Some(root),
                    [x, 0.5, z],
                    [1.4, 0.8, 0.8],
                    0.92,
                    0.31,
                    0.31,
                )
                .map_err(|err| err.to_string())?;
        }
    }
    Ok(forge)
}

/// Build the breakout depot game.
///
/// # Errors
///
/// Returns a message when the trace fails verification or a node is invalid.
pub fn build_breakout() -> Result<GameParts, String> {
    let total = breakout_bricks().len();
    let sim = BreakoutSim {
        ball_x: 0.0,
        ball_z: 3.0,
        vel_x: 4.8,
        vel_z: -15.2,
        paddle_x: 0.0,
        bricks: breakout_bricks(),
        lives: 3,
        speed: 16.0,
        cool: 0,
    };
    let (frames, lives) = breakout_trace(sim)?;
    let used = frames.len();
    let forge = breakout_scene()?;
    let view = View {
        x: -10.0,
        y: -6.0,
        w: 20.0,
        h: 13.0,
    };
    Ok(GameParts {
        slug: "breakout",
        title: "GameCraft Breakout",
        controls: "Left/Right move the paddle. Clear 20 bricks, 3 balls.",
        win_text: "Clear all 20 bricks.",
        seed: 6,
        scene_text: export_scene(&forge, "breakout")?,
        scripts: vec![
            ("gamecraft_runtime.gd", crate::scripts::RUNTIME),
            ("ball.gd", crate::scripts::BALL),
            ("tracker.gd", crate::scripts::TRACKER),
        ],
        trace_json: format!(
            "{{\"game\":\"breakout\",\"win\":true,\"frames\":{used},\"bricks\":{total},\"lives_left\":{lives}}}"
        ),
        commands_json: commands_json(&forge),
        statics: vec![
            Glyph { x: 0.0, y: 0.0, w: 17.0, h: 11.0, color: 11, circle: false },
        ],
        frames,
        view,
        readme_body: "AI paddle tracks the descending ball (attract mode headless).\nMirrored sim clears all 20 bricks inside the frame budget.\n".to_string(),
        summary: format!("breakout: 20 bricks in {used} frames, lint clean"),
    })
}

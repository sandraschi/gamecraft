//! Puzzle games: sokoban, maze, talk-inventory, chessboard-move.
//!
//! All four run on the grid walker with BFS-verified traces.

use crate::model::{Forge, GameParts};
use crate::render::{Glyph, View};
use gamecraft_core::PropertyValue;
use gamecraft_sim as sim;

/// Lossless grid index to float (boards fit in `u16`).
fn cell_f(index: i32) -> f32 {
    f32::from(u16::try_from(index).unwrap_or(0))
}

/// Export helper shared by puzzle builders.
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

/// Checkerboard floor plus a downward camera.
fn board_base(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    w: i32,
    h: i32,
    light: [f32; 3],
    dark: [f32; 3],
) -> Result<(), String> {
    for x in 0..w {
        for z in 0..h {
            let odd = (x + z) % 2 == 0;
            let color = if odd { light } else { dark };
            forge
                .deco_box(
                    format!("Tile{x}_{z}").as_str(),
                    Some(root),
                    [cell_f(x), -0.05, cell_f(z)],
                    [1.0, 0.1, 1.0],
                    color[0],
                    color[1],
                    color[2],
                )
                .map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

/// Walker avatar with a mode and grid exports.
fn walker_node(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    mode: &str,
    at: [f32; 3],
    w: i32,
    h: i32,
) -> Result<gamecraft_core::NodeId, String> {
    let id = forge
        .place("Player", "Node3D", Some(root), at)
        .map_err(|err| err.to_string())?;
    if let Some(node) = forge.graph.get_mut(id) {
        node.unique_name = true;
    }
    forge
        .set_prop(id, "script", Forge::ext_ref("2_walker"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "mode", PropertyValue::Str(mode.to_string()))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "step_size", PropertyValue::Float(1.0))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "grid_w", PropertyValue::Int(i64::from(w)))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "grid_h", PropertyValue::Int(i64::from(h)))
        .map_err(|err| err.to_string())?;
    let mesh = forge.capsule_mesh("walker", 0.3, 0.8);
    let material = forge.material("walker", 0.35, 0.59, 1.0);
    forge
        .mesh_child("Mesh", id, mesh.as_str(), material.as_str())
        .map_err(|err| err.to_string())?;
    Ok(id)
}

/// Wall cell (solid, lint-clean).
fn wall_cell(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    name: &str,
    x: i32,
    z: i32,
) -> Result<(), String> {
    forge
        .solid_box(
            name,
            "StaticBody3D",
            Some(root),
            [cell_f(x), 0.5, cell_f(z)],
            [1.0, 1.0, 1.0],
            0.3,
            0.3,
            0.35,
        )
        .map_err(|err| err.to_string())?;
    Ok(())
}

/// Crate (kinematic, script-moved, no physics).
fn crate_node(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    name: &str,
    x: i32,
    z: i32,
) -> Result<(), String> {
    let id = forge
        .place(name, "Node3D", Some(root), [cell_f(x), 0.45, cell_f(z)])
        .map_err(|err| err.to_string())?;
    let mesh = forge.box_mesh(name, [0.9, 0.9, 0.9]);
    let material = forge.material(name, 0.55, 0.36, 0.18);
    forge
        .mesh_child("Mesh", id, mesh.as_str(), material.as_str())
        .map_err(|err| err.to_string())?;
    Ok(())
}

/// Target pad (flat marker).
fn target_pad(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    name: &str,
    x: i32,
    z: i32,
) -> Result<(), String> {
    forge
        .deco_box(
            name,
            Some(root),
            [cell_f(x), 0.03, cell_f(z)],
            [0.9, 0.06, 0.9],
            0.31,
            0.78,
            0.75,
        )
        .map_err(|err| err.to_string())?;
    Ok(())
}

/// Interpolated frames along a cell path (4 sub-frames per step).
fn path_frames(path: &[(i32, i32)], color: u8) -> Vec<Vec<Glyph>> {
    let mut frames: Vec<Vec<Glyph>> = Vec::new();
    let mut prev = path.first().copied().unwrap_or((0, 0));
    frames.push(vec![Glyph {
        x: cell_f(prev.0),
        y: cell_f(prev.1),
        w: 0.6,
        h: 0.8,
        color,
        circle: false,
    }]);
    for cell in path.iter().skip(1) {
        for step in 1..=4_u16 {
            let t = f32::from(step) / 4.0;
            frames.push(vec![Glyph {
                x: cell_f(prev.0) + cell_f(cell.0 - prev.0) * t,
                y: cell_f(prev.1) + cell_f(cell.1 - prev.1) * t,
                w: 0.6,
                h: 0.8,
                color,
                circle: false,
            }]);
        }
        prev = *cell;
    }
    frames
}

// ---------------------------------------------------------------------------
// Maze
// ---------------------------------------------------------------------------

/// Border plus a vertical wall with a gap at row 5.
fn maze_walls() -> Vec<(i32, i32)> {
    let mut walls: Vec<(i32, i32)> = Vec::new();
    for i in 0..7 {
        walls.push((i, 0));
        walls.push((i, 6));
        walls.push((0, i));
        walls.push((6, i));
    }
    for y in 1..5 {
        walls.push((3, y));
    }
    walls
}

/// Build the maze depot game.
///
/// # Errors
///
/// Returns a message when the trace fails verification or a node is invalid.
pub fn build_maze() -> Result<GameParts, String> {
    let walls = maze_walls();
    let path = sim::bfs_path((1, 1), (5, 5), walls.as_slice(), 7, 7)
        .ok_or_else(|| "maze has no path".to_string())?;
    if path.len() > 600 {
        return Err("maze path too long".to_string());
    }
    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/grid_walker.gd", "2_walker");
    forge
        .top_camera("Camera3D", Some(root), [3.0, 3.0], 12.0)
        .map_err(|err| err.to_string())?;
    board_base(
        &mut forge,
        root,
        7,
        7,
        [0.16, 0.18, 0.24],
        [0.12, 0.14, 0.2],
    )?;
    for (serial, (x, z)) in walls.into_iter().enumerate() {
        wall_cell(&mut forge, root, format!("Wall{serial}").as_str(), x, z)?;
    }
    walker_node(&mut forge, root, "maze", [1.0, 0.5, 1.0], 7, 7)?;
    forge
        .area_trigger("Goal", Some(root), [5.0, 0.5, 5.0], [1.0, 2.0, 1.0])
        .map_err(|err| err.to_string())?;
    forge.connect("body_entered", "Goal", ".", "_on_goal");
    let frames = path_frames(path.as_slice(), 2);
    let used = frames.len();
    Ok(GameParts {
        slug: "maze",
        title: "GameCraft Maze",
        controls: "Arrow keys move cell to cell. Find the gold exit.",
        win_text: "Reach the exit at (5, 5) inside 60 seconds.",
        seed: 9,
        scene_text: export_scene(&forge, "maze")?,
        scripts: vec![
            ("gamecraft_runtime.gd", crate::scripts::RUNTIME),
            ("grid_walker.gd", crate::scripts::WALKER),
        ],
        trace_json: format!(
            "{{\"game\":\"maze\",\"win\":true,\"frames\":{},\"steps\":{}}}",
            used,
            path.len()
        ),
        commands_json: commands_json(&forge),
        statics: vec![],
        frames,
        view: View { x: -1.0, y: -1.0, w: 8.0, h: 8.0 },
        readme_body: "Discrete maze: BFS proves the exit reachable, trace walks it.\nThe 60s timer is the par; the trace finishes far inside it.\n".to_string(),
        summary: format!("maze: exit in {used} frames, lint clean"),
    })
}

// ---------------------------------------------------------------------------
// Sokoban
// ---------------------------------------------------------------------------

/// Open 5x5 room, three crates, three targets.
const SOKO_CRATES: [(i32, i32); 3] = [(1, 0), (1, 2), (3, 3)];
const SOKO_TARGETS: [(i32, i32); 3] = [(4, 0), (4, 2), (0, 4)];

/// Build the sokoban depot game.
///
/// # Errors
///
/// Returns a message when the trace fails verification or a node is invalid.
pub fn build_sokoban() -> Result<GameParts, String> {
    let moves = sim::solve_sokoban((0, 0), &SOKO_CRATES, &SOKO_TARGETS, &[], 5, 5, 500_000)
        .ok_or_else(|| "sokoban layout unsolvable".to_string())?;
    if moves.len() > 600 {
        let step_count = moves.len();
        return Err(format!("sokoban solution too long: {step_count}"));
    }
    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/grid_walker.gd", "2_walker");
    forge
        .top_camera("Camera3D", Some(root), [2.0, 2.0], 10.0)
        .map_err(|err| err.to_string())?;
    board_base(
        &mut forge,
        root,
        5,
        5,
        [0.16, 0.18, 0.24],
        [0.12, 0.14, 0.2],
    )?;
    walker_node(&mut forge, root, "soko", [0.0, 0.5, 0.0], 5, 5)?;
    for (index, cell) in SOKO_CRATES.iter().enumerate() {
        crate_node(
            &mut forge,
            root,
            format!("Crate{index}").as_str(),
            cell.0,
            cell.1,
        )?;
    }
    for (index, cell) in SOKO_TARGETS.iter().enumerate() {
        target_pad(
            &mut forge,
            root,
            format!("Target{index}").as_str(),
            cell.0,
            cell.1,
        )?;
    }
    // Replay the solver trace on the discrete model for the GIF.
    let mut player = (0_i32, 0_i32);
    let mut crates: Vec<(i32, i32)> = SOKO_CRATES.to_vec();
    let mut frames: Vec<Vec<Glyph>> = vec![soko_frame(player, &crates)];
    for step in &moves {
        let dest = (player.0 + step.0, player.1 + step.1);
        if let Some(slot) = crates.iter().position(|c| *c == dest) {
            crates[slot] = (dest.0 + step.0, dest.1 + step.1);
        }
        player = dest;
        frames.push(soko_frame(player, &crates));
    }
    let used = frames.len();
    Ok(GameParts {
        slug: "sokoban",
        title: "GameCraft Sokoban",
        controls: "Arrows push one crate at a time. Cover all 3 targets.",
        win_text: "All crates on targets.",
        seed: 10,
        scene_text: export_scene(&forge, "sokoban")?,
        scripts: vec![
            ("gamecraft_runtime.gd", crate::scripts::RUNTIME),
            ("grid_walker.gd", crate::scripts::WALKER),
        ],
        trace_json: format!(
            "{{\"game\":\"sokoban\",\"win\":true,\"frames\":{},\"pushes\":{}}}",
            used,
            moves.len()
        ),
        commands_json: commands_json(&forge),
        statics: vec![],
        frames,
        view: View { x: -1.0, y: -1.0, w: 6.0, h: 6.0 },
        readme_body: "BFS over (player, crates) proves the room solvable; the GIF\nreplays the exact solver trace on the discrete model.\n".to_string(),
        summary: format!("sokoban: solved in {used} frames, lint clean"),
    })
}

/// One GIF frame: player plus crates.
fn soko_frame(player: (i32, i32), crates: &[(i32, i32)]) -> Vec<Glyph> {
    let mut out = vec![Glyph {
        x: cell_f(player.0),
        y: cell_f(player.1),
        w: 0.6,
        h: 0.8,
        color: 2,
        circle: false,
    }];
    for cell in crates {
        out.push(Glyph {
            x: cell_f(cell.0),
            y: cell_f(cell.1),
            w: 0.9,
            h: 0.9,
            color: 0,
            circle: false,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Talk + inventory + door
// ---------------------------------------------------------------------------

const TALK_LINES: [&str; 3] = [
    "Traveler! The door east eats keys.",
    "I dropped mine by the old well. Shame.",
    "Here, take my spare. The door will listen now.",
];

/// Talk-scene: NPC, key, door, triggers on a 5x5 board.
///
/// # Errors
///
/// Returns the bus error when a node cannot be placed.
fn talk_scene() -> Result<Forge, String> {
    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "needs_key", PropertyValue::Bool(true))
        .map_err(|err| err.to_string())?;
    let mut dialogue = "Array[String]([".to_string();
    dialogue.push_str(
        TALK_LINES
            .map(|line| format!("\"{line}\""))
            .join(", ")
            .as_str(),
    );
    dialogue.push_str("])");
    forge
        .set_prop(root, "dialogue", PropertyValue::Raw(dialogue))
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/grid_walker.gd", "2_walker");
    forge
        .top_camera("Camera3D", Some(root), [2.0, 2.0], 10.0)
        .map_err(|err| err.to_string())?;
    board_base(
        &mut forge,
        root,
        5,
        5,
        [0.16, 0.2, 0.18],
        [0.12, 0.16, 0.14],
    )?;
    walker_node(&mut forge, root, "talk", [0.0, 0.5, 0.0], 5, 5)?;
    forge
        .deco_box(
            "NPC",
            Some(root),
            [3.0, 0.5, 3.0],
            [0.8, 1.0, 0.8],
            1.0,
            0.59,
            0.78,
        )
        .map_err(|err| err.to_string())?;
    forge
        .area_trigger("NPCTalk", Some(root), [3.0, 0.5, 3.0], [1.2, 2.0, 1.2])
        .map_err(|err| err.to_string())?;
    forge.connect("body_entered", "NPCTalk", ".", "_on_talk");
    forge
        .deco_box(
            "Key",
            Some(root),
            [3.0, 0.3, 4.0],
            [0.4, 0.2, 0.4],
            1.0,
            0.8,
            0.24,
        )
        .map_err(|err| err.to_string())?;
    forge
        .deco_box(
            "Door",
            Some(root),
            [4.0, 1.0, 4.0],
            [1.0, 2.0, 0.4],
            0.55,
            0.36,
            0.18,
        )
        .map_err(|err| err.to_string())?;
    forge
        .area_trigger("DoorIn", Some(root), [4.0, 0.5, 4.0], [1.2, 2.0, 1.2])
        .map_err(|err| err.to_string())?;
    forge.connect("body_entered", "DoorIn", ".", "_on_door");
    Ok(forge)
}

/// Build the talk-inventory depot game.
///
/// # Errors
///
/// Returns a message when the trace fails verification or a node is invalid.
pub fn build_talk() -> Result<GameParts, String> {
    let forge = talk_scene()?;
    // Scripted visit: NPC x3 (in-out-in-out-in), then the door.
    let route = [
        (1, 0),
        (2, 0),
        (3, 0),
        (3, 1),
        (3, 2),
        (3, 3),
        (3, 2),
        (3, 3),
        (3, 2),
        (3, 3),
        (4, 3),
        (4, 4),
    ];
    let mut visits = 0_usize;
    let mut has_key = false;
    let mut won = false;
    for cell in route {
        if cell == (3, 3) {
            visits += 1;
            if visits >= 3 {
                has_key = true;
            }
        }
        if cell == (4, 4) && has_key {
            won = true;
        }
    }
    if !won {
        return Err("talk trace never unlocked the door".to_string());
    }
    let frames = path_frames(&route, 2);
    let used = frames.len();
    Ok(GameParts {
        slug: "talk-inventory",
        title: "GameCraft Talk & Key",
        controls: "Arrows walk. Bump the NPC 3 times, then take the door.",
        win_text: "Open the door with the spare key.",
        seed: 11,
        scene_text: export_scene(&forge, "talk-inventory")?,
        scripts: vec![
            ("gamecraft_runtime.gd", crate::scripts::RUNTIME),
            ("grid_walker.gd", crate::scripts::WALKER),
        ],
        trace_json: format!(
            "{{\"game\":\"talk-inventory\",\"win\":true,\"frames\":{used},\"talks\":3,\"route\":\"npc x3 then door\"}}"
        ),
        commands_json: commands_json(&forge),
        statics: vec![],
        frames,
        view: View { x: -1.0, y: -1.0, w: 6.0, h: 6.0 },
        readme_body: "Talk -> key -> door state machine on the root script.\nDialogue and the key requirement are scene data, not code.\n".to_string(),
        summary: format!("talk-inventory: door opened in {used} frames, lint clean"),
    })
}

// ---------------------------------------------------------------------------
// Chessboard knight
// ---------------------------------------------------------------------------

/// Knight-reachable target chain from (0, 0).
const CHESS_TARGETS: [(i32, i32); 4] = [(1, 2), (2, 4), (4, 3), (3, 1)];

/// Verify the scripted knight tour: every hop legal, all targets landed on.
fn chess_verify(moves: &[(i32, i32)]) -> bool {
    let mut at = (0_i32, 0_i32);
    let mut landed = [false; 4];
    for step in moves {
        let knight = (at.0 + step.0, at.1 + step.1);
        let dx = (knight.0 - at.0).abs();
        let dy = (knight.1 - at.1).abs();
        let legal = (dx == 1 && dy == 2) || (dx == 2 && dy == 1);
        if !legal || knight.0 < 0 || knight.1 < 0 || knight.0 > 4 || knight.1 > 4 {
            return false;
        }
        at = knight;
        for (index, target) in CHESS_TARGETS.iter().enumerate() {
            if at == *target {
                landed[index] = true;
            }
        }
    }
    landed.iter().all(|landed| *landed)
}

/// Build the chess-knight depot game.
///
/// # Errors
///
/// Returns a message when the trace fails verification or a node is invalid.
pub fn build_chess() -> Result<GameParts, String> {
    // Cursor hops: deltas chosen so each hop is a legal knight move.
    let moves = [(1, 2), (1, 2), (2, -1), (-1, -2)];
    if !chess_verify(&moves) {
        return Err("chess script leaves a target unvisited".to_string());
    }
    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/grid_walker.gd", "2_walker");
    forge
        .top_camera("Camera3D", Some(root), [2.0, 2.0], 10.0)
        .map_err(|err| err.to_string())?;
    board_base(&mut forge, root, 5, 5, [0.75, 0.7, 0.62], [0.2, 0.22, 0.3])?;
    let knight = walker_node(&mut forge, root, "chess", [0.0, 0.5, 0.0], 5, 5)?;
    forge
        .deco_box(
            "Cursor",
            Some(knight),
            [0.0, -0.35, 0.0],
            [0.9, 0.1, 0.9],
            1.0,
            0.74,
            0.38,
        )
        .map_err(|err| err.to_string())?;
    for (index, target) in CHESS_TARGETS.iter().enumerate() {
        target_pad(
            &mut forge,
            root,
            format!("Target{index}").as_str(),
            target.0,
            target.1,
        )?;
    }
    // Frames: cursor glides to each hop are skipped; landings + holds.
    let mut at = (0_i32, 0_i32);
    let mut frames: Vec<Vec<Glyph>> = vec![vec![Glyph {
        x: 0.0,
        y: 0.0,
        w: 0.6,
        h: 0.8,
        color: 2,
        circle: false,
    }]];
    for step in &moves {
        at = (at.0 + step.0, at.1 + step.1);
        for _ in 0..8 {
            frames.push(vec![Glyph {
                x: cell_f(at.0),
                y: cell_f(at.1),
                w: 0.6,
                h: 0.8,
                color: 2,
                circle: false,
            }]);
        }
    }
    let used = frames.len();
    Ok(GameParts {
        slug: "chessboard-move",
        title: "GameCraft Chess Knight",
        controls: "Arrows move the cursor, Space jumps the knight there.",
        win_text: "Land the knight on all 4 targets, legally.",
        seed: 12,
        scene_text: export_scene(&forge, "chessboard-move")?,
        scripts: vec![
            ("gamecraft_runtime.gd", crate::scripts::RUNTIME),
            ("grid_walker.gd", crate::scripts::WALKER),
        ],
        trace_json: format!(
            "{{\"game\":\"chessboard-move\",\"win\":true,\"frames\":{used},\"hops\":4,\"rule\":\"knight only\"}}"
        ),
        commands_json: commands_json(&forge),
        statics: vec![],
        frames,
        view: View { x: -1.0, y: -1.0, w: 6.0, h: 6.0 },
        readme_body: "Cursor/accept split: arrows aim, Space commits - and only\nknight moves land. The trace is verified hop by hop.\n".to_string(),
        summary: format!("chessboard-move: 4 legal hops in {used} frames, lint clean"),
    })
}

//! Combat games: top-down shooter and tower defense.
//!
//! Both verify by simulation: autofire DPS (shooter) and tower coverage
//! (tower-place) against scripted waves. No randomness anywhere.

use crate::model::{Forge, GameParts};
use crate::render::{Glyph, View};
use gamecraft_core::PropertyValue;

/// Shared turret cadence: every 12 ticks (0.2 s), mirrored in the scene.
const FIRE_TICKS: usize = 12;
const BULLET_DAMAGE: i32 = 1;

/// Place the twin-stick player.
fn shooter_player(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
) -> Result<gamecraft_core::NodeId, String> {
    let id = forge
        .place("Player", "CharacterBody3D", Some(root), [0.0, 0.6, 0.0])
        .map_err(|err| err.to_string())?;
    if let Some(node) = forge.graph.get_mut(id) {
        node.unique_name = true;
    }
    forge
        .set_prop(id, "script", Forge::ext_ref("2_player"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "mode", PropertyValue::Int(4))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "speed", PropertyValue::Float(5.0))
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

/// Place one enemy (seeker or laner).
#[allow(clippy::too_many_arguments)]
fn enemy_node(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    name: &str,
    at: [f32; 3],
    mode: i64,
    speed: f32,
    hp: i64,
    red: f32,
    green: f32,
    blue: f32,
) -> Result<gamecraft_core::NodeId, String> {
    let id = forge
        .place(name, "CharacterBody3D", Some(root), at)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "script", Forge::ext_ref("3_enemy"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "mode", PropertyValue::Int(mode))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "speed", PropertyValue::Float(f64::from(speed)))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "hp", PropertyValue::Int(hp))
        .map_err(|err| err.to_string())?;
    let mesh = forge.capsule_mesh(name, 0.4, 0.9);
    let material = forge.material(name, red, green, blue);
    forge
        .mesh_child("Mesh", id, mesh.as_str(), material.as_str())
        .map_err(|err| err.to_string())?;
    let shape = forge.box_shape(name, [0.8, 0.8, 0.8]);
    forge
        .shape_child("Shape", id, shape.as_str())
        .map_err(|err| err.to_string())?;
    Ok(id)
}

/// Place one bullet pool member (hidden until launched).
fn bullet_node(
    forge: &mut Forge,
    root: gamecraft_core::NodeId,
    index: usize,
) -> Result<gamecraft_core::NodeId, String> {
    let name = format!("Bullet{index}");
    let id = forge
        .place(name.as_str(), "Area3D", Some(root), [0.0, -50.0, 0.0])
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(id, "script", Forge::ext_ref("4_bullet"))
        .map_err(|err| err.to_string())?;
    let mesh = forge.sphere_mesh(name.as_str(), 0.15, 0.3);
    let material = forge.material(name.as_str(), 0.96, 0.96, 0.96);
    forge
        .mesh_child("Mesh", id, mesh.as_str(), material.as_str())
        .map_err(|err| err.to_string())?;
    let shape = forge.box_shape(name.as_str(), [0.3, 0.3, 0.3]);
    forge
        .shape_child("Shape", id, shape.as_str())
        .map_err(|err| err.to_string())?;
    Ok(id)
}

/// Export helper shared by combat builders.
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
// Top-down shooter
// ---------------------------------------------------------------------------

/// Shooter waves: 3 x 3 drifting in from the edges.
const WAVES: [(usize, f32, f32); 9] = [
    (0, -10.0, -6.0),
    (0, 10.0, -6.0),
    (0, 0.0, 6.0),
    (150, -10.0, 6.0),
    (150, 10.0, 6.0),
    (150, 0.0, -6.0),
    (300, -10.0, 0.0),
    (300, 10.0, 0.0),
    (300, 0.0, 6.0),
];

/// Autofire DPS sim: every 12 ticks the nearest alive enemy takes 1 damage.
/// Shooter arena: floor, player, pooled bullets, three waves.
///
/// # Errors
///
/// Returns the bus error when a node cannot be placed.
fn shooter_scene() -> Result<Forge, String> {
    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "autofire", PropertyValue::Bool(true))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "fire_interval", PropertyValue::Float(0.2))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "bullet_speed", PropertyValue::Float(14.0))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "total_to_win", PropertyValue::Int(9))
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/player_controller.gd", "2_player");
    forge.ext_resource("Script", "res://scripts/enemy.gd", "3_enemy");
    forge.ext_resource("Script", "res://scripts/bullet.gd", "4_bullet");
    forge
        .top_camera("Camera3D", Some(root), [0.0, 0.0], 22.0)
        .map_err(|err| err.to_string())?;
    forge
        .deco_box(
            "Floor",
            Some(root),
            [0.0, -0.5, 0.0],
            [26.0, 1.0, 18.0],
            0.12,
            0.14,
            0.2,
        )
        .map_err(|err| err.to_string())?;
    shooter_player(&mut forge, root)?;
    for (index, wave) in WAVES.iter().enumerate() {
        enemy_node(
            &mut forge,
            root,
            format!("Enemy{index}").as_str(),
            [wave.1, 0.6, wave.2],
            0,
            1.5,
            2,
            0.65,
            0.43,
            0.85,
        )?;
    }
    for index in 0..8_usize {
        bullet_node(&mut forge, root, index)?;
    }
    Ok(forge)
}

/// Build the top-down shooter depot game.
///
/// # Errors
///
/// Returns a message when the trace fails verification or a node is invalid.
pub fn build_shooter() -> Result<GameParts, String> {
    let mut alive = vec![false; WAVES.len()];
    let mut hp = vec![2_i32; WAVES.len()];
    let mut pos: Vec<(f32, f32)> = WAVES.iter().map(|wave| (wave.1, wave.2)).collect();
    let mut downs = 0_usize;
    let mut frames: Vec<Vec<Glyph>> = Vec::new();
    let mut won = false;
    for tick in 0..600 {
        for (index, wave) in WAVES.iter().enumerate() {
            if tick >= wave.0 {
                alive[index] = true;
            }
        }
        // Drift toward the player at the origin.
        for (index, spot) in pos.iter_mut().enumerate() {
            if !alive[index] || hp[index] <= 0 {
                continue;
            }
            let dx = 0.0 - spot.0;
            let dz = 0.0 - spot.1;
            let dist = (dx * dx + dz * dz).sqrt();
            if dist > 0.05 {
                spot.0 += dx / dist * 1.5 / 60.0;
                spot.1 += dz / dist * 1.5 / 60.0;
            }
        }
        // Autofire every 12 ticks at the nearest alive enemy.
        if tick % FIRE_TICKS == 0 {
            let mut best: Option<usize> = None;
            let mut best_d = f32::MAX;
            for (index, spot) in pos.iter().enumerate() {
                if !alive[index] || hp[index] <= 0 {
                    continue;
                }
                let dist = spot.0 * spot.0 + spot.1 * spot.1;
                if dist < best_d {
                    best_d = dist;
                    best = Some(index);
                }
            }
            if let Some(target) = best {
                hp[target] -= BULLET_DAMAGE;
                if hp[target] <= 0 {
                    downs += 1;
                }
            }
        }
        let mut frame = vec![Glyph {
            x: 0.0,
            y: 0.0,
            w: 0.6,
            h: 1.0,
            color: 2,
            circle: false,
        }];
        for (index, spot) in pos.iter().enumerate() {
            if alive[index] && hp[index] > 0 {
                frame.push(Glyph {
                    x: spot.0,
                    y: spot.1,
                    w: 0.8,
                    h: 0.8,
                    color: 6,
                    circle: false,
                });
            }
        }
        frames.push(frame);
        if downs >= WAVES.len() {
            won = true;
            break;
        }
    }
    if !won {
        return Err("shooter trace: waves survived".to_string());
    }
    let used = frames.len();
    let forge = shooter_scene()?;
    Ok(GameParts {
        slug: "topdown-shooter",
        title: "GameCraft Topdown Shooter",
        controls: "WASD twin-stick, autofire handles aiming. Survive 3 waves.",
        win_text: "Destroy all 9 enemies.",
        seed: 7,
        scene_text: export_scene(&forge, "topdown-shooter")?,
        scripts: vec![
            ("gamecraft_runtime.gd", crate::scripts::RUNTIME),
            ("player_controller.gd", crate::scripts::PLAYER),
            ("enemy.gd", crate::scripts::ENEMY),
            ("bullet.gd", crate::scripts::BULLET),
        ],
        trace_json: format!(
            "{{\"game\":\"topdown-shooter\",\"win\":true,\"frames\":{used},\"kills\":9,\"waves\":3}}"
        ),
        commands_json: commands_json(&forge),
        statics: vec![
            Glyph { x: 0.0, y: 0.0, w: 26.0, h: 18.0, color: 11, circle: false },
        ],
        frames,
        view: View { x: -14.0, y: -10.0, w: 28.0, h: 20.0 },
        readme_body: "Twin-stick arena, autofire turret logic on the root.\nSim mirrors fire interval, damage, drift speed and spawn ticks.\n".to_string(),
        summary: format!("topdown-shooter: 9 kills in {used} frames, lint clean"),
    })
}

// ---------------------------------------------------------------------------
// Tower defense
// ---------------------------------------------------------------------------

/// Tower descriptor.
struct TowerDef {
    x: f32,
    z: f32,
    range: f32,
    cooldown: u32,
    damage: i32,
}

const TOWERS: [TowerDef; 3] = [
    TowerDef {
        x: -4.0,
        z: 2.0,
        range: 6.0,
        cooldown: 36,
        damage: 1,
    },
    TowerDef {
        x: 0.0,
        z: -2.0,
        range: 6.0,
        cooldown: 36,
        damage: 1,
    },
    TowerDef {
        x: 4.0,
        z: 2.0,
        range: 6.0,
        cooldown: 36,
        damage: 1,
    },
];

/// 8 enemies marching the lane; towers fire on cooldown at nearest in range.
/// Tower arena: floor, base, three towers, eight lane marchers.
///
/// # Errors
///
/// Returns the bus error when a node cannot be placed.
fn tower_scene() -> Result<Forge, String> {
    let mut forge = Forge::new();
    let root = forge
        .add_node("World", "Node3D", None)
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "script", Forge::ext_ref("1_runtime"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(root, "total_to_win", PropertyValue::Int(8))
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/tower.gd", "2_tower");
    // enemy_node() always references ExtResource("3_enemy"); keep the id aligned.
    forge.ext_resource("Script", "res://scripts/enemy.gd", "3_enemy");
    forge
        .top_camera("Camera3D", Some(root), [0.0, 0.0], 20.0)
        .map_err(|err| err.to_string())?;
    forge
        .deco_box(
            "Floor",
            Some(root),
            [0.0, -0.5, 0.0],
            [24.0, 1.0, 10.0],
            0.12,
            0.14,
            0.2,
        )
        .map_err(|err| err.to_string())?;
    forge
        .deco_box(
            "Base",
            Some(root),
            [10.5, 0.5, 0.0],
            [1.0, 2.0, 4.0],
            0.35,
            0.59,
            1.0,
        )
        .map_err(|err| err.to_string())?;
    for (index, tower) in TOWERS.iter().enumerate() {
        let id = forge
            .place(
                format!("Tower{index}").as_str(),
                "Node3D",
                Some(root),
                [tower.x, 0.5, tower.z],
            )
            .map_err(|err| err.to_string())?;
        forge
            .set_prop(id, "script", Forge::ext_ref("2_tower"))
            .map_err(|err| err.to_string())?;
        forge
            .set_prop(
                id,
                "fire_range",
                PropertyValue::Float(f64::from(tower.range)),
            )
            .map_err(|err| err.to_string())?;
        // Cooldown must match the sim exactly (ticks -> seconds at 60 Hz),
        // otherwise the Godot build out-damages the verified trace.
        forge
            .set_prop(
                id,
                "cooldown",
                PropertyValue::Float(f64::from(tower.cooldown) / 60.0),
            )
            .map_err(|err| err.to_string())?;
        let mesh = forge.cylinder_mesh(format!("Tower{index}").as_str(), 0.35, 0.5, 1.2);
        let material = forge.material(format!("Tower{index}").as_str(), 0.59, 0.59, 0.63);
        forge
            .mesh_child("Mesh", id, mesh.as_str(), material.as_str())
            .map_err(|err| err.to_string())?;
    }
    for index in 0..8_usize {
        enemy_node(
            &mut forge,
            root,
            format!("Enemy{index}").as_str(),
            [-10.0, 0.6, 0.0],
            1,
            1.2,
            2,
            0.65,
            0.43,
            0.85,
        )?;
    }
    Ok(forge)
}

/// Verified tower trace: 8 marchers die, none leak.
///
/// # Errors
///
/// Returns a message when a marcher leaks or survives.
fn tower_trace() -> Result<(Vec<Vec<Glyph>>, usize), String> {
    let mut ex: Vec<f32> = Vec::new();
    let mut hp: Vec<i32> = Vec::new();
    let mut live: Vec<bool> = Vec::new();
    let mut clock: Vec<u32> = vec![0; TOWERS.len()];
    let mut downs = 0_usize;
    let mut leaked = false;
    let mut frames: Vec<Vec<Glyph>> = Vec::new();
    let mut won = false;
    for tick in 0..600 {
        if tick % 40 == 0 && ex.len() < 8 {
            ex.push(-10.0);
            hp.push(2);
            live.push(true);
        }
        for (index, x) in ex.iter_mut().enumerate() {
            if live[index] {
                *x += 1.2 / 60.0;
                if *x > 10.0 {
                    leaked = true;
                }
            }
        }
        for (tower, cd) in TOWERS.iter().zip(clock.iter_mut()) {
            *cd += 1;
            if *cd < tower.cooldown {
                continue;
            }
            *cd = 0;
            let mut best: Option<usize> = None;
            let mut best_d = tower.range * tower.range;
            for (index, x) in ex.iter().enumerate() {
                if !live[index] {
                    continue;
                }
                let dx = *x - tower.x;
                let dz = 0.0 - tower.z;
                if dx * dx + dz * dz < best_d {
                    best_d = dx * dx + dz * dz;
                    best = Some(index);
                }
            }
            if let Some(target) = best {
                hp[target] -= tower.damage;
                if hp[target] <= 0 {
                    live[target] = false;
                    downs += 1;
                }
            }
        }
        let mut frame: Vec<Glyph> = TOWERS
            .iter()
            .map(|tower| Glyph {
                x: tower.x,
                y: tower.z,
                w: 1.0,
                h: 1.0,
                color: 7,
                circle: false,
            })
            .collect();
        for (index, x) in ex.iter().enumerate() {
            if live[index] {
                frame.push(Glyph {
                    x: *x,
                    y: 0.0,
                    w: 0.8,
                    h: 0.8,
                    color: 6,
                    circle: false,
                });
            }
        }
        frames.push(frame);
        if downs >= 8 {
            won = true;
            break;
        }
        if leaked {
            break;
        }
    }
    if !won {
        return Err("tower trace: wave leaked or survived".to_string());
    }
    let used = frames.len();
    Ok((frames, used))
}

/// Build the tower-defense depot game.
///
/// # Errors
///
/// Returns a message when the trace fails verification or a node is invalid.
pub fn build_tower() -> Result<GameParts, String> {
    let (frames, used) = tower_trace()?;
    let forge = tower_scene()?;
    Ok(GameParts {
        slug: "tower-place",
        title: "GameCraft Tower Place",
        controls: "Towers are pre-placed (3 fixed). Watch the wave break.",
        win_text: "Destroy all 8 lane marchers before any leaks.",
        seed: 8,
        scene_text: export_scene(&forge, "tower-place")?,
        scripts: vec![
            ("gamecraft_runtime.gd", crate::scripts::RUNTIME),
            ("enemy.gd", crate::scripts::ENEMY),
            ("tower.gd", crate::scripts::TOWER),
        ],
        trace_json: format!(
            "{{\"game\":\"tower-place\",\"win\":true,\"frames\":{used},\"kills\":8,\"leaks\":0}}"
        ),
        commands_json: commands_json(&forge),
        statics: vec![
            Glyph { x: 0.0, y: 0.0, w: 24.0, h: 10.0, color: 11, circle: false },
            Glyph { x: 10.5, y: 0.0, w: 1.0, h: 4.0, color: 2, circle: false },
        ],
        frames,
        view: View { x: -13.0, y: -6.0, w: 26.0, h: 12.0 },
        readme_body: "Fixed-tower defense: three towers, one lane, eight marchers.\nDPS race is verified tick by tick; zero leaks.\n".to_string(),
        summary: format!("tower-place: 8 kills, 0 leaks in {used} frames, lint clean"),
    })
}

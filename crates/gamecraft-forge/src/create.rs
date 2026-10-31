//! "Make me a new game": an LLM designs a spec, `GameCraft` compiles and verifies it.
//!
//! The model never writes `GDScript`. It writes a [`GameSpec`] - a declarative level
//! and feel description - which this module compiles through the same builders,
//! lint rules, and trace simulation as the hand-made depot. That is the whole
//! reason the feature can claim the output is playable: the compiler is
//! deterministic and the gates still run.
//!
//! Scope, stated honestly: v1 supports the `platformer` archetype. "New game"
//! means a new level, theme and physics feel - not a new mechanic. Unknown
//! archetypes are rejected with the supported list rather than silently mapped.

use crate::model::{Forge, GameParts};
use crate::render::{Glyph, View};
use crate::scripts;
use crate::textures;
use gamecraft_core::PropertyValue;
use gamecraft_sim as sim;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Colour triple, `0.0..=1.0`.
pub type Rgb = [f32; 3];

/// Visual theme.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Theme {
    /// Sky top colour.
    pub sky_top: Rgb,
    /// Sky horizon colour.
    pub sky_horizon: Rgb,
    /// Ground body colour.
    pub ground: Rgb,
    /// Platform colour.
    pub platform: Rgb,
    /// Player colour.
    pub player: Rgb,
    /// Coin colour.
    pub coin: Rgb,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            sky_top: [0.24, 0.42, 0.75],
            sky_horizon: [0.78, 0.87, 0.96],
            ground: [0.55, 0.36, 0.18],
            platform: [0.29, 0.63, 0.35],
            player: [0.35, 0.59, 1.0],
            coin: [1.0, 0.8, 0.24],
        }
    }
}

/// Movement feel.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Physics {
    /// Horizontal speed (units/s).
    pub speed: f32,
    /// Jump impulse (units/s).
    pub jump_velocity: f32,
    /// Gravity (units/s^2).
    pub gravity: f32,
}

impl Default for Physics {
    fn default() -> Self {
        Self {
            speed: 4.5,
            jump_velocity: 10.5,
            gravity: 24.0,
        }
    }
}

/// One ground slab: left edge and width, top at `y = 0`.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Ground {
    /// Left edge.
    pub x: f32,
    /// Width.
    pub w: f32,
}

/// One floating platform.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct PlatformSpec {
    /// Centre x.
    pub x: f32,
    /// Centre y.
    pub y: f32,
    /// Width.
    pub w: f32,
}

/// Level layout.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Level {
    /// Ground slabs in order.
    pub grounds: Vec<Ground>,
    /// Floating platforms.
    pub platforms: Vec<PlatformSpec>,
    /// Coin positions `[x, y]`.
    pub coins: Vec<[f32; 2]>,
    /// x of the goal flag.
    pub goal_x: f32,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            grounds: vec![Ground { x: -6.0, w: 12.0 }, Ground { x: 9.0, w: 12.0 }],
            platforms: vec![PlatformSpec {
                x: 2.0,
                y: 1.6,
                w: 2.5,
            }],
            coins: vec![[2.0, 2.2], [14.0, 1.5]],
            goal_x: 18.0,
        }
    }
}

/// Maze generation parameters (only read when `archetype` is `"maze"`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct MazeParams {
    /// Board width/height in cells; must be odd, 7 through 25.
    pub size: i32,
    /// Deterministic generator seed.
    pub seed: u64,
}

impl Default for MazeParams {
    fn default() -> Self {
        Self { size: 15, seed: 9 }
    }
}

/// Flight course parameters (only read when `archetype` is `"flight"`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FlightParams {
    /// Number of gates to fly through (2-12).
    pub gates: usize,
    /// Distance between gates along +x.
    pub spacing: f32,
    /// Base altitude the gates alternate around.
    pub altitude: f32,
}

impl Default for FlightParams {
    fn default() -> Self {
        Self {
            gates: 5,
            spacing: 20.0,
            altitude: 4.0,
        }
    }
}

/// Driving course parameters (only read when `archetype` is `"driving"`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct DriveParams {
    /// Loop radius in world units.
    pub radius: f32,
    /// Checkpoints around the loop (4-16).
    pub checkpoints: usize,
}

impl Default for DriveParams {
    fn default() -> Self {
        Self {
            radius: 12.0,
            checkpoints: 8,
        }
    }
}

/// Chess course parameters (only read when `archetype` is `"chess"`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ChessParams {
    /// Board size in cells (5-9).
    pub board: i32,
    /// Knight hops to make (2-8). Targets are wherever the walk lands.
    pub hops: usize,
    /// Deterministic walk seed.
    pub seed: u64,
}

impl Default for ChessParams {
    fn default() -> Self {
        Self {
            board: 5,
            hops: 4,
            seed: 12,
        }
    }
}

/// A complete game design.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct GameSpec {
    /// Folder/url slug.
    pub slug: String,
    /// Display title.
    pub title: String,
    /// Archetype id: `platformer` | `runner` | `pong` | `maze`.
    pub archetype: String,
    /// Visual theme.
    pub theme: Theme,
    /// Movement feel.
    pub physics: Physics,
    /// Level layout.
    pub level: Level,
    /// Maze generation parameters (only used when `archetype` is `"maze"`).
    pub maze: MazeParams,
    /// Flight course parameters (only used when `archetype` is `"flight"`).
    pub flight: FlightParams,
    /// Driving course parameters (only used when `archetype` is `"driving"`).
    pub drive: DriveParams,
    /// Chess parameters (only used when `archetype` is `"chess"`).
    pub chess: ChessParams,
}

impl Default for GameSpec {
    fn default() -> Self {
        Self {
            slug: "generated".to_string(),
            title: "Generated Game".to_string(),
            archetype: "platformer".to_string(),
            theme: Theme::default(),
            physics: Physics::default(),
            level: Level::default(),
            maze: MazeParams::default(),
            flight: FlightParams::default(),
            drive: DriveParams::default(),
            chess: ChessParams::default(),
        }
    }
}

/// Structural problems a spec can have that the compiler cannot fix.
///
/// # Errors
///
/// Returns a human-readable reason listing every problem found.
pub fn validate(spec: &GameSpec) -> Result<(), String> {
    let mut problems: Vec<String> = Vec::new();
    if spec.archetype != "platformer"
        && spec.archetype != "runner"
        && spec.archetype != "pong"
        && spec.archetype != "maze"
        && spec.archetype != "flight"
        && spec.archetype != "driving"
        && spec.archetype != "chess"
    {
        problems.push(format!(
            "archetype '{}' is not supported (supported: platformer, runner, pong, maze, flight, driving, chess)",
            spec.archetype
        ));
    }
    if spec.slug.is_empty()
        || !spec
            .slug
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        problems.push("slug must be non-empty ascii [a-z0-9-]".to_string());
    }
    if spec.level.grounds.is_empty() {
        problems.push("level.grounds must contain at least one slab".to_string());
    }
    // The player is 1.0 tall standing at y=0.5, so a platform whose underside is
    // below 1.1 is a wall, not a step. The autopilot only detects gaps, so such a
    // platform stops the run dead (measured: stuck at x=0.4 in a generated level).
    if spec
        .level
        .platforms
        .iter()
        .any(|platform| platform.y - 0.2 < 1.1)
    {
        problems.push(
            "platform.y must be >= 1.3 so it clears the 1.0-tall player; lower ones block the run"
                .to_string(),
        );
    }
    if spec.level.goal_x < 2.0 {
        problems.push("level.goal_x must be >= 2".to_string());
    }
    if spec.physics.gravity < 5.0 {
        problems.push(
            "physics.gravity below 5 makes jumps floaty to the point of unplayable".to_string(),
        );
    }
    if spec.physics.speed <= 0.0 || spec.physics.jump_velocity <= 0.0 {
        problems.push("physics speed and jump_velocity must be positive".to_string());
    }
    if spec.level.grounds.iter().any(|ground| ground.w <= 0.0) {
        problems.push("every ground slab needs a positive width".to_string());
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// Walkable tops derived from the grounds, for simulation.
fn walkable(level: &Level) -> Vec<sim::Platform> {
    level
        .grounds
        .iter()
        .map(|ground| sim::Platform {
            x: ground.x,
            y: 0.0,
            w: ground.w,
        })
        .collect()
}

/// Gap-detecting policy: hold right, jump when the floor ahead runs out.
///
/// The same rule the in-engine autoplay uses (a downward probe in front of the
/// player), so a pass here predicts a pass there.
fn auto_script(level: &Level, physics: &Physics, frames: usize) -> Vec<sim::TraceInput> {
    let grounds = walkable(level);
    let phys = sim::Phys {
        speed: physics.speed,
        jump_v: physics.jump_velocity,
        gravity: physics.gravity,
        auto_run: false,
        half_w: 0.3,
        half_h: 0.5,
    };
    let mut body = sim::Body2 {
        x: level.grounds.first().map_or(0.0, |ground| ground.x + 1.0),
        y: 0.5,
        vx: 0.0,
        vy: 0.0,
    };
    let mut script = Vec::with_capacity(frames);
    for _ in 0..frames {
        let ahead = body.x + 1.1;
        let has_floor = grounds.iter().any(|ground| {
            ahead > ground.x && ahead < ground.x + ground.w && (body.y - 0.5 - ground.y).abs() < 0.6
        });
        let grounded = grounds.iter().any(|ground| {
            (body.y - 0.5 - ground.y).abs() < 0.06
                && body.x + 0.3 > ground.x
                && body.x - 0.3 < ground.x + ground.w
        });
        let input = if !has_floor && grounded {
            sim::TraceInput::RightJump
        } else {
            sim::TraceInput::Right
        };
        script.push(input);
        let _ = sim::step_body(&mut body, input, phys, &grounds, -6.0);
    }
    script
}

/// Compile a spec into a playable project.
///
/// # Errors
///
/// Returns a reason when the spec is structurally invalid or the level cannot be
/// completed by the built-in policy (which is what a model most often gets wrong).
/// Long but linear: one node per scene element, in reading order. Splitting it
/// would scatter a single construction sequence across helpers for no gain.
#[allow(clippy::too_many_lines)]
pub fn compile(spec: &GameSpec) -> Result<GameParts, String> {
    if spec.archetype == "runner" {
        return crate::arcade::build_runner_with(&spec.theme, &spec.slug, &spec.title);
    }
    if spec.archetype == "pong" {
        let ball_speed = spec.physics.speed;
        if !(8.0..=22.0).contains(&ball_speed) {
            return Err(format!("pong ball speed {ball_speed} outside 8..22"));
        }
        return crate::arcade::build_pong_with(ball_speed, &spec.slug, &spec.title);
    }
    if spec.archetype == "maze" {
        let size = spec.maze.size;
        if !(7..=25).contains(&size) || size % 2 == 0 {
            return Err(format!("maze size {size} must be an odd number 7..=25"));
        }
        return crate::puzzle::build_maze_with(size, size, spec.maze.seed, &spec.slug, &spec.title);
    }
    if spec.archetype == "flight" {
        let gates = spec.flight.gates;
        let spacing = spec.flight.spacing;
        let altitude = spec.flight.altitude;
        if !(2..=12).contains(&gates) {
            return Err(format!("flight gates {gates} must be 2..=12"));
        }
        if !(8.0..=60.0).contains(&spacing) {
            return Err(format!("flight spacing {spacing} must be 8..=60"));
        }
        if !(1.0..=12.0).contains(&altitude) {
            return Err(format!("flight altitude {altitude} must be 1..=12"));
        }
        return crate::flight::build_flight_with(
            gates,
            spacing,
            altitude,
            &spec.slug,
            &spec.title,
            false,
        );
    }
    if spec.archetype == "driving" {
        let radius = spec.drive.radius;
        let checkpoints = spec.drive.checkpoints;
        if !(5.0..=40.0).contains(&radius) {
            return Err(format!("driving radius {radius} must be 5..=40"));
        }
        if !(4..=16).contains(&checkpoints) {
            return Err(format!("driving checkpoints {checkpoints} must be 4..=16"));
        }
        return crate::driving::build_driving_with(radius, checkpoints, &spec.slug, &spec.title);
    }
    if spec.archetype == "chess" {
        let board = spec.chess.board;
        let hops = spec.chess.hops;
        if !(5..=9).contains(&board) {
            return Err(format!("chess board {board} must be 5..=9"));
        }
        if !(2..=8).contains(&hops) {
            return Err(format!("chess hops {hops} must be 2..=8"));
        }
        return crate::puzzle::build_chess_with(
            board,
            hops,
            spec.chess.seed,
            &spec.slug,
            &spec.title,
        );
    }
    validate(spec)?;
    let physics = &spec.physics;
    let level = &spec.level;
    let phys = sim::Phys {
        speed: physics.speed,
        jump_v: physics.jump_velocity,
        gravity: physics.gravity,
        auto_run: false,
        half_w: 0.3,
        half_h: 0.5,
    };
    let grounds = walkable(level);
    let script = auto_script(level, physics, 900);
    let start = sim::Body2 {
        x: level.grounds.first().map_or(0.0, |ground| ground.x + 1.0),
        y: 0.5,
        vx: 0.0,
        vy: 0.0,
    };
    let coins: Vec<(f32, f32)> = level.coins.iter().map(|coin| (coin[0], coin[1])).collect();
    let run = sim::run_trace(
        start,
        script.as_slice(),
        phys,
        &grounds,
        coins.as_slice(),
        level.goal_x,
        -6.0,
        900,
    );
    if !run.won {
        return Err(format!(
            "the level is not completable: the player stalls or falls before x={} (reached x={:.1} after {} frames). Widen gaps, lower the goal, or raise jump_velocity.",
            level.goal_x,
            run.path.last().map_or(start.x, |point| point.0),
            run.frames
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
        .sky(root, spec.theme.sky_top, spec.theme.sky_horizon)
        .map_err(|err| err.to_string())?;
    forge
        .sun(root, -0.95, -0.55, 1.15)
        .map_err(|err| err.to_string())?;
    forge.ext_resource("Script", "res://scripts/gamecraft_runtime.gd", "1_runtime");
    forge.ext_resource("Script", "res://scripts/player_controller.gd", "2_player");
    forge.ext_resource("Script", "res://scripts/coin.gd", "3_coin");
    forge.ext_resource("Script", "res://scripts/camera_follow.gd", "4_camera");
    forge.ext_resource("Script", "res://scripts/autoplay.gd", "5_autoplay");

    for (index, ground) in level.grounds.iter().enumerate() {
        forge
            .solid_box(
                format!("Ground{index}").as_str(),
                "StaticBody3D",
                Some(root),
                [ground.x + ground.w * 0.5, -0.5, 0.0],
                [ground.w, 1.0, 4.0],
                spec.theme.ground[0],
                spec.theme.ground[1],
                spec.theme.ground[2],
            )
            .map_err(|err| err.to_string())?;
    }
    for (index, platform) in level.platforms.iter().enumerate() {
        forge
            .solid_box(
                format!("Platform{index}").as_str(),
                "StaticBody3D",
                Some(root),
                [platform.x, platform.y - 0.2, 0.0],
                [platform.w, 0.4, 2.0],
                spec.theme.platform[0],
                spec.theme.platform[1],
                spec.theme.platform[2],
            )
            .map_err(|err| err.to_string())?;
    }
    let start_x = level.grounds.first().map_or(0.0, |ground| ground.x + 1.0);
    let player = forge
        .place("Player", "CharacterBody3D", Some(root), [start_x, 0.5, 0.0])
        .map_err(|err| err.to_string())?;
    if let Some(node) = forge.graph.get_mut(player) {
        node.unique_name = true;
    }
    forge
        .set_prop(player, "script", Forge::ext_ref("2_player"))
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(
            player,
            "speed",
            PropertyValue::Float(f64::from(physics.speed)),
        )
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(
            player,
            "jump_velocity",
            PropertyValue::Float(f64::from(physics.jump_velocity)),
        )
        .map_err(|err| err.to_string())?;
    forge
        .set_prop(
            player,
            "gravity",
            PropertyValue::Float(f64::from(physics.gravity)),
        )
        .map_err(|err| err.to_string())?;
    let body_mesh = forge.capsule_mesh("player", 0.3, 1.0);
    let body_mat = forge.material(
        "player",
        spec.theme.player[0],
        spec.theme.player[1],
        spec.theme.player[2],
    );
    forge
        .mesh_child("Mesh", player, body_mesh.as_str(), body_mat.as_str())
        .map_err(|err| err.to_string())?;
    let body_shape = forge.capsule_shape("player", 0.3, 1.0);
    forge
        .shape_child("Shape", player, body_shape.as_str())
        .map_err(|err| err.to_string())?;

    for (index, coin) in level.coins.iter().enumerate() {
        let id = forge
            .area_trigger(
                format!("Coin{index}").as_str(),
                Some(root),
                [coin[0], coin[1], 0.0],
                [0.6, 0.6, 0.6],
            )
            .map_err(|err| err.to_string())?;
        forge
            .set_prop(id, "script", Forge::ext_ref("3_coin"))
            .map_err(|err| err.to_string())?;
        let mesh = forge.sphere_mesh(format!("Coin{index}").as_str(), 0.28, 0.56);
        let material = forge.material(
            format!("Coin{index}").as_str(),
            spec.theme.coin[0],
            spec.theme.coin[1],
            spec.theme.coin[2],
        );
        forge
            .mesh_child("Mesh", id, mesh.as_str(), material.as_str())
            .map_err(|err| err.to_string())?;
    }

    let goal = forge
        .area_trigger(
            "Goal",
            Some(root),
            [level.goal_x, 1.5, 0.0],
            [1.5, 3.0, 2.0],
        )
        .map_err(|err| err.to_string())?;
    let pole = forge.box_mesh("goal_pole", [0.25, 3.0, 0.25]);
    let pole_mat = forge.material(
        "goal_pole",
        spec.theme.coin[0],
        spec.theme.coin[1],
        spec.theme.coin[2],
    );
    forge
        .mesh_child("Pole", goal, pole.as_str(), pole_mat.as_str())
        .map_err(|err| err.to_string())?;
    forge.connect("body_entered", "Goal", ".", "_on_goal");
    forge
        .area_trigger(
            "Kill",
            Some(root),
            [level.goal_x * 0.5, -7.0, 0.0],
            [level.goal_x * 2.0 + 40.0, 1.0, 6.0],
        )
        .map_err(|err| err.to_string())?;
    forge.connect("body_entered", "Kill", ".", "_on_kill");
    forge
        .follow_camera(
            "Camera3D",
            Some(root),
            [start_x, 2.4, 14.0],
            34.0,
            "4_camera",
        )
        .map_err(|err| err.to_string())?;
    forge
        .autoplay(root, "5_autoplay", "ui_right", &[])
        .map_err(|err| err.to_string())?;

    let request = gamecraft_godot::ExportRequest {
        graph: &forge.graph,
        connections: forge.conns.as_slice(),
        uid: gamecraft_godot::deterministic_uid(
            format!("res://depot/games/{}/main.tscn", spec.slug).as_str(),
        ),
        format: 3,
        ext_resources: forge.ext.clone(),
        sub_resources: forge.sub.clone(),
    };
    let scene_text = gamecraft_godot::export_tscn(&request)?;

    let frames = run
        .path
        .iter()
        .map(|(x, y)| {
            vec![Glyph {
                x: *x,
                y: *y,
                w: 0.6,
                h: 1.0,
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
        slug: leak_slug(&spec.slug),
        title: leak_title(&spec.title),
        controls: "Left/Right run, Space jumps. Autoplay drives the verified route.",
        win_text: "Touch the goal flag at the far end.",
        seed: 0,
        scene_text,
        scripts: vec![
            ("gamecraft_runtime.gd", scripts::RUNTIME),
            ("player_controller.gd", scripts::PLAYER),
            ("coin.gd", scripts::COIN),
            ("camera_follow.gd", scripts::CAMERA_FOLLOW),
            ("autoplay.gd", scripts::AUTOPLAY),
        ],
        trace_json: format!(
            "{{\"game\":\"{}\",\"generated\":true,\"win\":true,\"frames\":{},\"coins\":{},\"goal_x\":{}}}",
            spec.slug, run.frames, run.coins, level.goal_x
        ),
        commands_json,
        statics: Vec::new(),
        frames,
        view: View {
            x: -8.0,
            y: -4.0,
            w: level.goal_x + 18.0,
            h: 14.0,
        },
        readme_body: format!(
            "Generated from a GameSpec by the local/cloud model path.\nArchetype: {}. Goal at x={:.1}. Physics: speed {:.1}, jump {:.1}, gravity {:.1}.\nThe spec is saved next to this folder as `spec.json` - the level is reproducible from it.\n",
            spec.archetype, level.goal_x, physics.speed, physics.jump_velocity, physics.gravity
        ),
        summary: format!(
            "{}: generated platformer, goal in {} frames, lint clean",
            spec.slug, run.frames
        ),
    })
}

// `GameParts` uses `&'static str` for slug/title, but a generated name is runtime
// data. Leaking the two small strings once per generated game is the smallest
// change that keeps one type instead of splitting it in two.
fn leak_slug(slug: &str) -> &'static str {
    Box::leak(slug.to_string().into_boxed_str())
}

/// See [`leak_slug`].
fn leak_title(title: &str) -> &'static str {
    Box::leak(title.to_string().into_boxed_str())
}

/// The texture set a generated game ships (reuses the generated depot).
#[must_use]
pub fn textures_for_generated() -> Vec<textures::TextureFile> {
    textures::depot()
}

/// Compile a spec, gate it, and write the project under `<root>/depot/games/<slug>`.
///
/// # Errors
///
/// Returns a reason when the spec is invalid, the level cannot be completed, the
/// generated scene fails lint, or a file cannot be written.
pub fn create_game(spec: &GameSpec, root: &Path) -> Result<String, String> {
    let parts = compile(spec)?;
    let diags = gamecraft_godot::scene_lint(parts.scene_text.as_str());
    if !diags.is_empty() {
        return Err(format!("generated scene failed lint: {diags:?}"));
    }
    let dir = root.join("depot").join("games").join(spec.slug.as_str());
    crate::write_game(&dir, &parts)?;
    let spec_json = serde_json::to_string_pretty(spec).map_err(|err| err.to_string())?;
    crate::write_text(&dir.join("spec.json"), spec_json.as_str())?;
    Ok(parts.summary)
}

/// Ask a model for a spec, then compile it.
///
/// One repair round: if the reply does not parse or the level is not completable,
/// the error is sent back with the same schema and the model gets a second try.
/// More rounds mostly buy longer failures.
///
/// # Errors
///
/// Returns a message when both attempts fail, or when the provider call fails.
pub fn generate(idea: &str, config: &crate::llm::LlmConfig, root: &Path) -> Result<String, String> {
    let prompt = crate::llm::spec_prompt(idea);
    let mut last_error = String::new();
    for attempt in 0..2 {
        let instruction = if attempt == 0 {
            prompt.clone()
        } else {
            format!(
                "{prompt}\n\nYour previous reply was rejected: {last_error}\nReply with corrected JSON only."
            )
        };
        let reply = crate::llm::complete(config, instruction.as_str())?;
        let Some(json) = crate::llm::extract_json(reply.as_str()) else {
            last_error = "no JSON object found in the reply".to_string();
            continue;
        };
        let spec: GameSpec = match serde_json::from_str(json) {
            Ok(spec) => spec,
            Err(err) => {
                last_error = format!("spec did not parse: {err}");
                continue;
            }
        };
        match create_game(&spec, root) {
            Ok(summary) => return Ok(summary),
            Err(reason) => last_error = reason,
        }
    }
    Err(format!("generation failed after 2 attempts: {last_error}"))
}

#[cfg(test)]
mod create_tests {
    use super::*;

    #[test]
    fn generated_game_writes_a_project() {
        let root = std::env::temp_dir().join("gamecraft-create-test");
        let _ = std::fs::remove_dir_all(&root);
        let spec = GameSpec::default();
        let outcome = create_game(&spec, root.as_path());
        assert!(outcome.is_ok(), "{:?}", outcome.err());
        let scene = root.join("depot/games/generated/main.tscn");
        assert!(scene.is_file(), "scene was not written");
        let spec_file = root.join("depot/games/generated/spec.json");
        assert!(spec_file.is_file(), "spec.json was not written");
        let _ = std::fs::remove_dir_all(&root);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_spec_compiles_and_wins() {
        let spec = GameSpec::default();
        let parts = compile(&spec);
        assert!(parts.is_ok(), "{:?}", parts.err());
        if let Ok(parts) = parts {
            assert!(parts.scene_text.contains("Goal"));
        }
    }

    #[test]
    fn unknown_archetype_is_rejected() {
        let spec = GameSpec {
            archetype: "shooter".to_string(),
            ..GameSpec::default()
        };
        assert!(validate(&spec).is_err());
    }

    #[test]
    fn impossible_level_is_rejected_with_a_reason() {
        let spec = GameSpec {
            level: Level {
                // One short slab and a goal far past its end: nothing to land on.
                grounds: vec![Ground { x: -4.0, w: 8.0 }],
                platforms: Vec::new(),
                coins: Vec::new(),
                goal_x: 60.0,
            },
            ..GameSpec::default()
        };
        let outcome = compile(&spec);
        assert!(outcome.is_err());
        if let Err(reason) = outcome {
            assert!(reason.contains("not completable"), "{reason}");
        }
    }

    #[test]
    fn slug_must_be_safe() {
        let spec = GameSpec {
            slug: "../etc".to_string(),
            ..GameSpec::default()
        };
        assert!(validate(&spec).is_err());
    }
}

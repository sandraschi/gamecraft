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

use crate::model::{Forge, GameParts, SurfaceIds};
use crate::render::{Glyph, View};
use crate::scripts;
use crate::textures;
use gamecraft_core::PropertyValue;
use gamecraft_sim as sim;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    /// Optional PBR texture sets by role (`ground`, `platform`). Platformer only.
    /// Roles without an entry keep their flat colour.
    pub surfaces: BTreeMap<String, SurfaceTexture>,
}

/// Roles a theme can texture.
const SURFACE_ROLES: [&str; 2] = ["ground", "platform"];

/// A PBR texture set for one surface, as image files on disk (for example a
/// `TextureCraft` "Godot 4 (ORM)" export). The files are copied into the game.
///
/// Tiling matters: the maps repeat every `1 / scale` units, and most procedural
/// materials have visible seams at the repeat (only brick, tile and weave kinds
/// were measured seamless).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SurfaceTexture {
    /// Name of a texture set in a catalog folder (see [`texture_catalog`]). Resolved
    /// into the three file fields by [`resolve_sets`].
    pub set: Option<String>,
    /// Albedo image (`png`, `jpg`, `jpeg`).
    pub albedo: String,
    /// Packed ORM image (R occlusion, G roughness, B metallic).
    pub orm: Option<String>,
    /// Normal map image.
    pub normal: Option<String>,
    /// Repeats per scene unit; `0.25` repeats every 4 units.
    pub scale: f32,
    /// Lay each map out as a 2x2 mirrored block (see [`crate::tiling::mirror_tile`]) so
    /// that textures with a visible seam tile cleanly. The images are written as PNG at
    /// twice the size and `scale` is halved internally, so the texel density is unchanged.
    pub mirror: bool,
}

impl Default for SurfaceTexture {
    fn default() -> Self {
        Self {
            set: None,
            albedo: String::new(),
            orm: None,
            normal: None,
            scale: 0.25,
            mirror: false,
        }
    }
}

/// Image files of one catalog texture set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetFiles {
    /// Albedo image.
    pub albedo: String,
    /// Packed ORM image, if present.
    pub orm: Option<String>,
    /// Normal map image, if present.
    pub normal: Option<String>,
}

/// Texture sets in a catalog folder: every subfolder holding an image whose name
/// contains `albedo` is a set named after the folder. `orm` and `normal` images are
/// matched the same way (the names the `TextureCraft` Godot preset writes).
#[must_use]
pub fn texture_catalog(dir: &Path) -> BTreeMap<String, SetFiles> {
    let mut sets = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return sets;
    };
    for entry in entries.flatten() {
        let folder = entry.path();
        let Some(name) = folder
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let Ok(files) = std::fs::read_dir(&folder) else {
            continue;
        };
        let mut images: Vec<String> = files
            .flatten()
            .map(|f| f.path())
            .filter(|p| {
                p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                    ["png", "jpg", "jpeg"]
                        .iter()
                        .any(|k| e.eq_ignore_ascii_case(k))
                })
            })
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        images.sort();
        let find = |key: &str| {
            images
                .iter()
                .find(|p| {
                    // Whole tokens only: "normal" must not count as "orm".
                    Path::new(p)
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .is_some_and(|stem| {
                            stem.to_ascii_lowercase()
                                .split(|c: char| !c.is_ascii_alphanumeric())
                                .any(|token| token == key)
                        })
                })
                .cloned()
        };
        if let Some(albedo) = find("albedo") {
            sets.insert(
                name,
                SetFiles {
                    albedo,
                    orm: find("orm"),
                    normal: find("normal"),
                },
            );
        }
    }
    sets
}

/// Fill the file fields of every surface that names a catalog `set`.
///
/// # Errors
///
/// Returns a reason naming the unknown set and the sets that do exist.
pub fn resolve_sets(spec: &mut GameSpec, catalog: &Path) -> Result<(), String> {
    let sets = texture_catalog(catalog);
    for (role, surface) in &mut spec.theme.surfaces {
        let Some(name) = surface.set.clone() else {
            continue;
        };
        let Some(files) = sets.get(&name) else {
            let known: Vec<&str> = sets.keys().map(String::as_str).collect();
            return Err(format!(
                "theme.surfaces.{role}.set '{name}' is not in the texture catalog (available: {})",
                known.join(", ")
            ));
        };
        surface.albedo.clone_from(&files.albedo);
        surface.orm.clone_from(&files.orm);
        surface.normal.clone_from(&files.normal);
    }
    Ok(())
}

/// Every image of a theme as `(role, map, source file, res:// path)`.
fn surface_files(theme: &Theme) -> Vec<(String, &'static str, String, String)> {
    let mut out = Vec::new();
    for (role, set) in &theme.surfaces {
        let maps = [
            ("albedo", Some(set.albedo.as_str())),
            ("orm", set.orm.as_deref()),
            ("normal", set.normal.as_deref()),
        ];
        for (map, source) in maps {
            let Some(source) = source else { continue };
            // mirrored maps are re-encoded, always as PNG
            let ext = if set.mirror {
                "png".to_string()
            } else {
                Path::new(source)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map_or_else(|| "png".to_string(), str::to_ascii_lowercase)
            };
            out.push((
                role.clone(),
                map,
                source.to_string(),
                format!("res://surfaces/{role}_{map}.{ext}"),
            ));
        }
    }
    out
}

/// Problems with `theme.surfaces`, appended to a validation list.
fn check_surfaces(spec: &GameSpec, problems: &mut Vec<String>) {
    if spec.theme.surfaces.is_empty() {
        return;
    }
    if spec.archetype != "platformer" {
        problems.push(format!(
            "theme.surfaces is only supported for the platformer archetype, not '{}'",
            spec.archetype
        ));
    }
    for (role, set) in &spec.theme.surfaces {
        if !SURFACE_ROLES.contains(&role.as_str()) {
            problems.push(format!(
                "theme.surfaces role '{role}' is unknown (supported: ground, platform)"
            ));
        }
        if !(0.01..=8.0).contains(&set.scale) {
            problems.push(format!("theme.surfaces.{role}.scale must be 0.01..=8"));
        }
        if set.albedo.is_empty() {
            problems.push(match &set.set {
                Some(name) => {
                    format!("theme.surfaces.{role}.set '{name}' needs a texture catalog (pass one)")
                }
                None => format!("theme.surfaces.{role}.albedo is required"),
            });
        }
    }
    for (role, map, source, _) in surface_files(&spec.theme) {
        let image = Path::new(&source)
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| {
                ["png", "jpg", "jpeg"]
                    .iter()
                    .any(|k| e.eq_ignore_ascii_case(k))
            });
        if !image {
            problems.push(format!(
                "theme.surfaces.{role}.{map} must be a png or jpg file: {source}"
            ));
        } else if !Path::new(&source).is_file() {
            problems.push(format!("theme.surfaces.{role}.{map} not found: {source}"));
        }
    }
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
            surfaces: BTreeMap::new(),
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

/// Talk & key parameters (only read when `archetype` is `"talk"`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct TalkParams {
    /// NPC conversations required before the door opens (2-6).
    pub visits: usize,
}

impl Default for TalkParams {
    fn default() -> Self {
        Self { visits: 3 }
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
    /// Talk & key parameters (only used when `archetype` is `"talk"`).
    pub talk: TalkParams,
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
            talk: TalkParams::default(),
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
    check_surfaces(spec, &mut problems);
    if spec.archetype != "platformer"
        && spec.archetype != "runner"
        && spec.archetype != "pong"
        && spec.archetype != "maze"
        && spec.archetype != "flight"
        && spec.archetype != "driving"
        && spec.archetype != "chess"
        && spec.archetype != "talk"
    {
        problems.push(format!(
            "archetype '{}' is not supported (supported: platformer, runner, pong, maze, flight, driving, chess, talk)",
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
    if spec.archetype == "talk" {
        let visits = spec.talk.visits;
        if !(2..=6).contains(&visits) {
            return Err(format!("talk visits {visits} must be 2..=6"));
        }
        return crate::puzzle::build_talk_with(visits, &spec.slug, &spec.title);
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
    let mut surface_ids: BTreeMap<String, SurfaceIds> = BTreeMap::new();
    for (role, set) in &spec.theme.surfaces {
        let res = |map: &str| {
            surface_files(&spec.theme)
                .into_iter()
                .find(|(r, m, _, _)| r == role && *m == map)
                .map(|(_, _, _, path)| path)
        };
        let mut register = |map: &str| {
            res(map).map(|path| {
                forge.ext_resource(
                    "Texture2D",
                    path.as_str(),
                    format!("surf_{role}_{map}").as_str(),
                )
            })
        };
        let albedo = register("albedo").ok_or_else(|| format!("surface {role} has no albedo"))?;
        let orm = register("orm");
        let normal = register("normal");
        surface_ids.insert(
            role.clone(),
            SurfaceIds {
                albedo,
                orm,
                normal,
                scale: if set.mirror {
                    set.scale * 0.5
                } else {
                    set.scale
                },
            },
        );
    }

    for (index, ground) in level.grounds.iter().enumerate() {
        let body = forge
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
        if let Some(ids) = surface_ids.get("ground") {
            forge
                .apply_surface(body, ids)
                .map_err(|err| err.to_string())?;
        }
    }
    for (index, platform) in level.platforms.iter().enumerate() {
        let body = forge
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
        if let Some(ids) = surface_ids.get("platform") {
            forge
                .apply_surface(body, ids)
                .map_err(|err| err.to_string())?;
        }
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
            "Generated from a GameSpec by the local/cloud model path.\nArchetype: {}. Goal at x={:.1}. Physics: speed {:.1}, jump {:.1}, gravity {:.1}.\nThe spec is saved next to this folder as `spec.json` - the level is reproducible from it.\n{}",
            spec.archetype, level.goal_x, physics.speed,
            physics.jump_velocity,
            physics.gravity,
            if spec.theme.surfaces.is_empty() {
                ""
            } else {
                "\nTextured surfaces (`surfaces/`) must be imported once before Godot can load \
                 them: run `godot --headless --path . --import` (or open the project in the \
                 editor). Until then the scene fails to load.\n"
            }
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
    for (role, map, source, res) in surface_files(&spec.theme) {
        let target = dir.join(res.trim_start_matches("res://"));
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        if spec.theme.surfaces.get(&role).is_some_and(|set| set.mirror) {
            write_mirrored(&source, &target, map == "normal")?;
        } else {
            std::fs::copy(&source, &target).map_err(|err| format!("copy {source}: {err}"))?;
        }
    }
    let spec_json = serde_json::to_string_pretty(spec).map_err(|err| err.to_string())?;
    crate::write_text(&dir.join("spec.json"), spec_json.as_str())?;
    Ok(parts.summary)
}

/// Write `source` as a 2x2 mirrored PNG block at `target`.
fn write_mirrored(source: &str, target: &Path, normal_map: bool) -> Result<(), String> {
    let image = image::open(source)
        .map_err(|err| format!("mirror {source}: {err}"))?
        .to_rgba8();
    crate::tiling::mirror_tile(&image, normal_map)
        .save_with_format(target, image::ImageFormat::Png)
        .map_err(|err| format!("write {}: {err}", target.display()))
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
    generate_with_textures(idea, config, root, None)
}

/// [`generate`] with an optional texture catalog folder. When given, the prompt lists
/// the available sets, the model may pick one per surface role, and the choice is
/// resolved to files before the game is compiled. A set name the catalog does not
/// have is sent back as a rejection, like any other bad spec.
///
/// # Errors
///
/// Returns a message when both attempts fail, or when the provider call fails.
pub fn generate_with_textures(
    idea: &str,
    config: &crate::llm::LlmConfig,
    root: &Path,
    catalog: Option<&Path>,
) -> Result<String, String> {
    let names: Vec<String> = catalog
        .map(|dir| texture_catalog(dir).into_keys().collect())
        .unwrap_or_default();
    let prompt = crate::llm::spec_prompt_with(idea, names.as_slice());
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
        let mut spec: GameSpec = match serde_json::from_str(json) {
            Ok(spec) => spec,
            Err(err) => {
                last_error = format!("spec did not parse: {err}");
                continue;
            }
        };
        if let Some(dir) = catalog {
            if let Err(reason) = resolve_sets(&mut spec, dir) {
                last_error = reason;
                continue;
            }
        }
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
#[allow(clippy::expect_used)]
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

    fn tiny_png(name: &str) -> String {
        let path = std::env::temp_dir().join(format!("gc-surf-{}-{name}.png", std::process::id()));
        image::RgbaImage::from_pixel(4, 4, image::Rgba([180, 60, 40, 255]))
            .save(&path)
            .expect("png");
        path.to_string_lossy().into_owned()
    }

    fn textured_spec() -> GameSpec {
        let mut spec = GameSpec::default();
        spec.theme.surfaces.insert(
            "ground".to_string(),
            SurfaceTexture {
                set: None,
                albedo: tiny_png("a"),
                orm: Some(tiny_png("o")),
                normal: Some(tiny_png("n")),
                scale: 0.5,
                mirror: false,
            },
        );
        spec
    }

    #[test]
    fn surfaces_become_triplanar_materials_on_ground_only() {
        let spec = textured_spec();
        let parts = compile(&spec).expect("compile");
        let scene = parts.scene_text;
        assert!(scene.contains("ORMMaterial3D"), "{scene}");
        assert!(scene.contains("uv1_triplanar = true"));
        assert!(scene.contains("res://surfaces/ground_albedo.png"));
        assert!(scene.contains("res://surfaces/ground_orm.png"));
        assert!(scene.contains("res://surfaces/ground_normal.png"));
        assert!(!scene.contains("res://surfaces/platform_"));
        let findings = gamecraft_godot::scene_lint(scene.as_str());
        assert_eq!(findings.len(), 0, "{findings:?}");
    }

    fn catalog_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("gc-cat-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for set in ["brick_red", "granite"] {
            std::fs::create_dir_all(dir.join(set)).expect("dir");
            for map in ["albedo", "orm", "normal"] {
                image::RgbaImage::from_pixel(2, 2, image::Rgba([90, 90, 90, 255]))
                    .save(dir.join(set).join(format!("plane_Plane_{map}.png")))
                    .expect("png");
            }
        }
        std::fs::create_dir_all(dir.join("empty")).expect("dir");
        dir
    }

    #[test]
    fn catalog_lists_folders_that_hold_an_albedo() {
        let dir = catalog_dir("list");
        let sets = texture_catalog(&dir);
        assert_eq!(
            sets.keys().map(String::as_str).collect::<Vec<_>>(),
            ["brick_red", "granite"]
        );
        let brick = &sets["brick_red"];
        assert!(brick.albedo.ends_with("plane_Plane_albedo.png"));
        assert!(brick
            .orm
            .as_deref()
            .is_some_and(|p| p.ends_with("_orm.png")));
        assert!(brick.normal.is_some());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn named_sets_resolve_and_unknown_ones_are_refused() {
        let dir = catalog_dir("resolve");
        let mut spec = GameSpec::default();
        spec.theme.surfaces.insert(
            "ground".to_string(),
            SurfaceTexture {
                set: Some("granite".to_string()),
                ..SurfaceTexture::default()
            },
        );
        let unresolved = validate(&spec).expect_err("needs a catalog");
        assert!(
            unresolved.contains("needs a texture catalog"),
            "{unresolved}"
        );
        resolve_sets(&mut spec, &dir).expect("resolve");
        validate(&spec).expect("valid after resolve");
        assert!(spec.theme.surfaces["ground"].albedo.contains("granite"));
        spec.theme.surfaces.get_mut("ground").expect("ground").set = Some("lava".to_string());
        let reason = resolve_sets(&mut spec, &dir).expect_err("unknown set");
        assert!(reason.contains("available: brick_red, granite"), "{reason}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_model_style_spec_with_a_set_name_builds_a_textured_game() {
        // Exactly what the prompt asks the model to write: a set name, no paths.
        let reply = r#"{"slug":"brick-roofs","title":"Brick Roofs","archetype":"platformer",
            "theme":{"surfaces":{"ground":{"set":"brick_red"},"platform":{"set":"granite"}}}}"#;
        let mut spec: GameSpec = serde_json::from_str(reply).expect("model json");
        let dir = catalog_dir("model");
        resolve_sets(&mut spec, &dir).expect("resolve");
        let root = std::env::temp_dir().join(format!("gc-model-game-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        create_game(&spec, &root).expect("create");
        let game = root.join("depot/games/brick-roofs");
        assert!(game.join("surfaces/ground_albedo.png").is_file());
        assert!(game.join("surfaces/platform_normal.png").is_file());
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn surface_files_are_copied_into_the_game() {
        let root = std::env::temp_dir().join(format!("gc-surf-game-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut spec = textured_spec();
        spec.slug = "surf-test".to_string();
        create_game(&spec, root.as_path()).expect("create");
        let dir = root.join("depot/games/surf-test/surfaces");
        for name in ["ground_albedo.png", "ground_orm.png", "ground_normal.png"] {
            assert!(dir.join(name).is_file(), "{name} missing");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn mirrored_surfaces_are_written_as_2x2_blocks_at_half_the_scale() {
        let root = std::env::temp_dir().join(format!("gc-mirror-game-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        // an asymmetric source so a mirror cannot be mistaken for a copy, saved as JPEG
        // to prove the output is re-encoded as PNG
        let source = std::env::temp_dir().join(format!("gc-mirror-src-{}.jpg", std::process::id()));
        image::RgbImage::from_fn(6, 4, |x, y| {
            image::Rgb([
                u8::try_from(x * 40).unwrap_or(0),
                u8::try_from(y * 60).unwrap_or(0),
                90,
            ])
        })
        .save(&source)
        .expect("jpg");
        let mut spec = GameSpec {
            slug: "mirror-test".to_string(),
            ..GameSpec::default()
        };
        spec.theme.surfaces.insert(
            "ground".to_string(),
            SurfaceTexture {
                albedo: source.to_string_lossy().into_owned(),
                scale: 0.5,
                mirror: true,
                ..SurfaceTexture::default()
            },
        );
        create_game(&spec, root.as_path()).expect("create");
        let game = root.join("depot/games/mirror-test");
        let written = image::open(game.join("surfaces/ground_albedo.png")).expect("png written");
        assert_eq!((written.width(), written.height()), (12, 8));
        let scene = std::fs::read_to_string(game.join("main.tscn")).expect("scene");
        assert!(
            scene.contains("res://surfaces/ground_albedo.png"),
            "{scene}"
        );
        assert!(
            scene.contains("uv1_scale = Vector3(0.25, 0.25, 0.25)"),
            "scale halves: {scene}"
        );
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(source);
    }

    #[test]
    fn bad_surfaces_are_rejected_with_reasons() {
        let mut spec = GameSpec::default();
        spec.theme.surfaces.insert(
            "sky".to_string(),
            SurfaceTexture {
                albedo: "C:/nope/missing.png".to_string(),
                ..SurfaceTexture::default()
            },
        );
        let reason = validate(&spec).expect_err("must fail");
        assert!(reason.contains("role 'sky' is unknown"), "{reason}");
        assert!(reason.contains("not found"), "{reason}");
        let mut runner = textured_spec();
        runner.archetype = "runner".to_string();
        let reason = validate(&runner).expect_err("must fail");
        assert!(
            reason.contains("only supported for the platformer"),
            "{reason}"
        );
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

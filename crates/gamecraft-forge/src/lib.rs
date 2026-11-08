//! Depot forge: builds all 12 games, verifies traces, writes the depot.
//!
//! Every game is generated in-process: nodes go through `gamecraft-commands`,
//! scenes serialize with deterministic `export_tscn`, traces replay in Rust,
//! and the same frames rasterize `thumb.png` / `play.gif`.

pub mod arcade;
pub mod combat;
pub mod create;
pub mod driving;
pub mod economy;
pub mod flight;
pub mod llm;
pub mod model;
pub mod platformer;
pub mod props;
pub mod puzzle;
pub mod render;
pub mod scripts;
pub mod textures;
pub mod tiling;

use gamecraft_godot::scene_lint;
use gamecraft_godot::{build_graph, parse_tscn};
use model::GameParts;
use std::fs;
use std::path::Path;

/// One depot game: slug plus its builder.
pub struct GameDef {
    /// URL/folder slug.
    pub slug: &'static str,
    /// Builder entry point.
    pub build: fn() -> Result<GameParts, String>,
}

/// All twelve depot games in plan order.
pub const GAMES: [GameDef; 14] = [
    GameDef {
        slug: "platformer",
        build: platformer::build_platformer,
    },
    GameDef {
        slug: "runner",
        build: arcade::build_runner,
    },
    GameDef {
        slug: "flappy",
        build: arcade::build_flappy,
    },
    GameDef {
        slug: "driving",
        build: driving::build_driving,
    },
    GameDef {
        slug: "pong",
        build: arcade::build_pong,
    },
    GameDef {
        slug: "breakout",
        build: arcade::build_breakout,
    },
    GameDef {
        slug: "topdown-shooter",
        build: combat::build_shooter,
    },
    GameDef {
        slug: "tower-place",
        build: combat::build_tower,
    },
    GameDef {
        slug: "sokoban",
        build: puzzle::build_sokoban,
    },
    GameDef {
        slug: "sokoban-vault",
        build: puzzle::build_sokoban_9x9,
    },
    GameDef {
        slug: "maze",
        build: puzzle::build_maze,
    },
    GameDef {
        slug: "talk-inventory",
        build: puzzle::build_talk,
    },
    GameDef {
        slug: "chessboard-move",
        build: puzzle::build_chess,
    },
    GameDef {
        slug: "flight",
        build: flight::build_flight,
    },
];

/// Build one game by slug.
///
/// # Errors
///
/// Returns a message when the slug is unknown or the build fails.
pub fn forge_one(slug: &str, out: &Path) -> Result<String, String> {
    for game in &GAMES {
        if game.slug == slug {
            return forge_game(game, out);
        }
    }
    Err(format!("unknown game slug: {slug}"))
}

/// Build the full depot.
///
/// # Errors
///
/// Returns on the first game that fails verification.
pub fn forge_all(out: &Path) -> Result<Vec<String>, String> {
    let mut summaries: Vec<String> = Vec::new();
    for game in &GAMES {
        summaries.push(forge_game(game, out)?);
    }
    Ok(summaries)
}

/// Build, verify, and write one game.
fn forge_game(game: &GameDef, out: &Path) -> Result<String, String> {
    let parts = (game.build)()?;
    // Determinism: building twice must yield byte-identical scenes.
    let again = (game.build)()?;
    if again.scene_text != parts.scene_text {
        return Err(format!("{}: non-deterministic export", game.slug));
    }
    // Lint gate: zero findings.
    let diags = scene_lint(parts.scene_text.as_str());
    if !diags.is_empty() {
        return Err(format!("{}: lint failed: {:?}", game.slug, diags));
    }
    // Structural gates: res:// targets exist, a current camera exists.
    let dir = out.join("depot").join("games").join(game.slug);
    write_game(&dir, &parts)?;
    check_resources(&dir, parts.scene_text.as_str())?;
    check_camera(parts.scene_text.as_str())?;
    // Frames: middle frame thumb, downsampled GIF (<= 150 frames).
    let statics = parts.statics.clone();
    let mid = parts.frames.len() / 2;
    if let Some(shot) = parts.frames.get(mid) {
        let png = render::render_frame(parts.view, statics.as_slice(), shot.as_slice());
        render::write_png(
            dir.join("thumb.png").to_string_lossy().as_ref(),
            png.as_slice(),
        )?;
    }
    let stride = parts.frames.len().div_ceil(150);
    let stride = stride.max(1);
    let mut gif_frames: Vec<Vec<u8>> = Vec::new();
    for (index, frame) in parts.frames.iter().enumerate() {
        if index % stride == 0 {
            gif_frames.push(render::render_frame(
                parts.view,
                statics.as_slice(),
                frame.as_slice(),
            ));
        }
    }
    render::write_gif(
        dir.join("play.gif").to_string_lossy().as_ref(),
        gif_frames.as_slice(),
        3,
    )?;
    write_text(&dir.join("trace.json"), parts.trace_json.as_str())?;
    write_text(&dir.join("commands.jsonl"), parts.commands_json.as_str())?;
    let lint_json = serde_json::to_string_pretty(&diags).map_err(|err| err.to_string())?;
    write_text(&dir.join("lint.json"), lint_json.as_str())?;
    write_text(&dir.join("README.md"), readme(&parts).as_str())?;
    Ok(parts.summary.clone())
}

/// Write scene, scripts, project, and icon.
/// Write a game's scene, scripts, project file and textures.
///
/// # Errors
///
/// Returns a message when any file cannot be written.
pub fn write_game(dir: &Path, parts: &GameParts) -> Result<(), String> {
    fs::create_dir_all(dir.join("scripts")).map_err(|err| err.to_string())?;
    // Generated textures (the flight village). Written before the resource
    // check below, which requires every res:// target to exist. Keyed on what
    // the scene actually references, not on the slug: a generated flight game
    // has a user-chosen slug and used to ship with missing textures.
    if parts.scene_text.contains("res://textures/") {
        let tex_dir = dir.join("textures");
        fs::create_dir_all(&tex_dir).map_err(|err| err.to_string())?;
        for (name, bytes) in textures::depot() {
            fs::write(tex_dir.join(name), bytes).map_err(|err| err.to_string())?;
        }
    }
    write_text(&dir.join("main.tscn"), parts.scene_text.as_str())?;
    for (name, content) in &parts.scripts {
        write_text(&dir.join("scripts").join(name), content)?;
    }
    write_text(&dir.join("project.godot"), project_godot(parts).as_str())?;
    write_text(&dir.join("icon.svg"), ICON_SVG)?;
    write_text(&dir.join("CHEATSHEET.md"), cheatsheet(parts).as_str())?;
    Ok(())
}

/// Write one text file (creates parents).
/// Write one text file, creating parent directories.
///
/// # Errors
///
/// Returns a message when the file cannot be written.
pub fn write_text(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(path, content).map_err(|err| err.to_string())
}

/// Minimal Godot 4 project file.
fn project_godot(parts: &GameParts) -> String {
    format!(
        "; Engine configuration file.\n; GameCraft depot game: {}\nconfig_version=5\n\n[application]\n\nconfig/name=\"{}\"\nrun/main_scene=\"res://main.tscn\"\nconfig/features=PackedStringArray(\"4.4\", \"GL Compatibility\")\nconfig/icon=\"res://icon.svg\"\n",
        parts.slug, parts.title
    )
}

/// `GameCraft` diamond icon.
const ICON_SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"128\" height=\"128\"><rect x=\"24\" y=\"24\" width=\"80\" height=\"80\" rx=\"8\" transform=\"rotate(45 64 64)\" fill=\"#ffbe3c\"/><rect x=\"44\" y=\"44\" width=\"40\" height=\"40\" rx=\"4\" transform=\"rotate(45 64 64)\" fill=\"#10141c\"/></svg>\n";

/// Per-game README with controls, win condition, and trace summary.
fn readme(parts: &GameParts) -> String {
    format!(
        "# {}\n\nGenerated by GameCraft forge (phase 5 depot). Do not hand-edit;\nregenerate with `gamecraft-cli forge --game {}`.\n\n## Controls\n\n{}\n\n## Win condition\n\n{}\n\n## Seed\n\n{}\n\n## Trace\n\n{}\n\n## Verification\n\n- `scene_lint`: zero findings (see `lint.json`).\n- `trace.json`: scripted inputs replayed in Rust; win inside frame budget.\n- `commands.jsonl`: every scene mutation as an undoable command.\n- `play.gif`: the verified trace, rasterized frame by frame.\n\n{}\n",
        parts.title,
        parts.slug,
        parts.controls,
        parts.win_text,
        parts.seed,
        parts.trace_json,
        parts.readme_body
    )
}

/// Collapse a value into one table cell: single line, no pipes.
fn cell(text: &str) -> String {
    text.replace('|', "/")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// One-screen cheat sheet in the same table shape the web and Bevy games use,
/// so `gamecraft help <slug>` reads every runtime the same way.
fn cheatsheet(parts: &GameParts) -> String {
    format!(
        "# {}: cheat sheet\n\nGenerated by GameCraft forge. Do not hand-edit.\n\n| What | Value |\n|---|---|\n| Controls | {} |\n| Goal | {} |\n| Seed | {} |\n| Gate | Headless Godot boot to a win marker; the Rust trace replays the verified inputs |\n",
        cell(parts.title),
        cell(parts.controls),
        cell(parts.win_text),
        parts.seed
    )
}

/// Verify every `ext_resource` path resolves to a written file.
fn check_resources(dir: &Path, scene_text: &str) -> Result<(), String> {
    let scene = parse_tscn(scene_text).map_err(|err| err.to_string())?;
    for resource in &scene.ext_resources {
        let mut rel = resource.path.as_str();
        if let Some(stripped) = rel.strip_prefix("res://") {
            rel = stripped;
        } else {
            return Err(format!("non-local resource path: {}", resource.path));
        }
        if !dir.join(rel).is_file() {
            return Err(format!("missing resource file: {}", resource.path));
        }
    }
    Ok(())
}

/// Verify the scene carries a current camera.
fn check_camera(scene_text: &str) -> Result<(), String> {
    let scene = parse_tscn(scene_text).map_err(|err| err.to_string())?;
    let graph = build_graph(&scene).map_err(|err| err.to_string())?;
    for node in graph.nodes.values() {
        if node.node_type == "Camera3D"
            && node.properties.get("current").is_some_and(|prop| {
                matches!(
                    prop,
                    gamecraft_core::PropertyValue::Bool(true)
                        | gamecraft_core::PropertyValue::Raw(_)
                )
            })
        {
            return Ok(());
        }
    }
    Err("scene has no current camera".to_string())
}

/// Depot thumbnail helper for external tools: middle frame as RGBA.
#[must_use]
pub fn thumb_frame(parts: &GameParts) -> Vec<u8> {
    let mid = parts.frames.len() / 2;
    if let Some(shot) = parts.frames.get(mid) {
        render::render_frame(parts.view, parts.statics.as_slice(), shot.as_slice())
    } else {
        render::render_frame(parts.view, parts.statics.as_slice(), &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_is_one_line_without_pipes() {
        assert_eq!(cell("a | b\n  c"), "a / b c");
    }

    #[test]
    fn forge_writes_a_cheat_sheet() {
        let out =
            std::env::temp_dir().join(format!("gamecraft-forge-cheat-{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        let done = forge_one("breakout", &out);
        let sheet =
            fs::read_to_string(out.join("depot/games/breakout/CHEATSHEET.md")).unwrap_or_default();
        let _ = fs::remove_dir_all(&out);
        assert!(done.is_ok(), "{done:?}");
        assert!(
            sheet.contains("| Controls | Left/Right move the paddle."),
            "{sheet}"
        );
        assert!(sheet.contains("| Goal |"), "{sheet}");
    }
}

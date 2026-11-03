//! Tool bodies shared by every front end.
//!
//! These started life inside the MCP server binary. They live in the library now so the
//! command registry, the CLI and the MCP server all run the same code and cannot disagree.

#![allow(clippy::missing_errors_doc)]

use crate::catalog::{self, GameKind};
use crate::{scene_lint, spatial_query};
use serde_json::{json, Value};
use std::fmt::Write as _;

/// A tool failure the model can act on.
pub struct ToolError {
    /// Stable machine-readable error code.
    pub code: &'static str,
    /// What went wrong.
    pub message: String,
    /// What to change.
    pub suggestion: String,
    /// A concrete call that would work.
    pub remediation: String,
}

/// Shared by the CLI, the MCP server and the command registry: `fail`.
pub fn fail(
    code: &'static str,
    message: impl Into<String>,
    suggestion: impl Into<String>,
    remediation: impl Into<String>,
) -> ToolError {
    ToolError {
        code,
        message: message.into(),
        suggestion: suggestion.into(),
        remediation: remediation.into(),
    }
}

/// What a tool body returns: a one-sentence summary and the data, or a failure.
pub type ToolOutcome = Result<(String, Value), ToolError>;

/// Shared by the CLI, the MCP server and the command registry: `scene_source`.
pub fn scene_source(args: &Value) -> Result<String, ToolError> {
    let text = args.get("scene_text").and_then(Value::as_str);
    let file = args.get("scene_file").and_then(Value::as_str);
    let root = args.get("root").and_then(Value::as_str);
    if text.is_none() && file.is_none() {
        return Err(fail(
            "missing_scene",
            "No scene given.",
            "Pass the .tscn text as scene_text, or a path as scene_file.",
            "scene_lint {\"scene_file\": \"D:/project/main.tscn\"}",
        ));
    }
    crate::resolve_scene_text(text, file, root).map_err(|reason| {
        fail(
            "scene_unreadable",
            format!("Could not read the scene: {reason}"),
            "Check the path; res:// paths need root set to the project folder.",
            "game_list to find a project, then game_report for its folder",
        )
    })
}

/// Shared by the CLI, the MCP server and the command registry: `plural`.
#[must_use]
pub fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Shared by the CLI, the MCP server and the command registry: `tool_scene_lint`.
pub fn tool_scene_lint(args: &Value) -> ToolOutcome {
    let text = scene_source(args)?;
    let findings = scene_lint(text.as_str());
    let summary = match findings.first() {
        None => "No findings: the scene is clean.".to_string(),
        Some(first) => format!(
            "{}; first: {} at {} ({}).",
            plural(findings.len(), "finding", "findings"),
            first.code,
            first.node,
            first.message
        ),
    };
    let data = json!({"count": findings.len(), "findings": findings});
    Ok((summary, data))
}

/// Shared by the CLI, the MCP server and the command registry: `usize_arg`.
pub fn usize_arg(args: &Value, key: &str, fallback: usize) -> usize {
    args.get(key)
        .and_then(Value::as_u64)
        .map_or(fallback, |value| usize::try_from(value).unwrap_or(fallback))
}

/// Shared by the CLI, the MCP server and the command registry: `tool_scene_inspect`.
pub fn tool_scene_inspect(args: &Value) -> ToolOutcome {
    let text = scene_source(args)?;
    let operation = args
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or(crate::OP_TREE);
    let options = crate::InspectOptions {
        operation,
        node_path: args.get("node_path").and_then(Value::as_str),
        query: args.get("query").and_then(Value::as_str),
        depth: usize_arg(args, "depth", crate::DEFAULT_DEPTH),
        limit: usize_arg(args, "limit", crate::DEFAULT_LIMIT),
        offset: usize_arg(args, "offset", 0),
    };
    let value = crate::scene_inspect(text.as_str(), &options).map_err(|reason| {
        fail(
            "inspect_failed",
            format!("scene_inspect {operation} failed: {reason}"),
            "operation 'property' needs node_path, 'find' needs query.",
            "scene_inspect {\"operation\": \"tree\"}",
        )
    })?;
    let total = value.get("total").and_then(Value::as_u64).unwrap_or(0);
    let summary = match operation {
        "signals" => format!(
            "{} connections, {} unresolved.",
            total,
            value.get("unresolved").and_then(Value::as_u64).unwrap_or(0)
        ),
        "property" => format!(
            "Properties of {}.",
            value
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("the node")
        ),
        "find" => format!("{} nodes match '{}'.", total, options.query.unwrap_or("")),
        _ => match value.get("next_offset").and_then(Value::as_u64) {
            Some(next) => {
                format!("{total} nodes; this page ends before {next}, pass offset={next} for more.")
            }
            None => format!("{total} nodes, all on this page."),
        },
    };
    Ok((summary, value))
}

/// Shared by the CLI, the MCP server and the command registry: `repo_root`.
pub fn repo_root(args: &Value) -> Result<std::path::PathBuf, ToolError> {
    catalog::find_root(args.get("root").and_then(Value::as_str)).ok_or_else(|| {
        fail("repo_not_found", "Could not find the GameCraft repo (a folder with depot/games or web/games).", "Pass root, or start the server with GAMECRAFT_ROOT set, or run it from inside the repo.", "game_list {\"root\": \"D:/Dev/repos/gamecraft\"}")
    })
}
/// Shared by the CLI, the MCP server and the command registry: `kind_arg`.
pub fn kind_arg(args: &Value) -> Option<GameKind> {
    args.get("kind")
        .and_then(Value::as_str)
        .and_then(GameKind::parse)
}
/// Shared by the CLI, the MCP server and the command registry: `find_game`.
pub fn find_game(args: &Value) -> Result<(std::path::PathBuf, catalog::GameEntry), ToolError> {
    let root = repo_root(args)?;
    let Some(slug) = args.get("slug").and_then(Value::as_str) else {
        return Err(fail(
            "missing_slug",
            "slug is required.",
            "Use a folder name from game_list.",
            "game_list",
        ));
    };
    let games = catalog::discover(root.as_path());
    if let Some(game) = catalog::find(&games, slug, kind_arg(args)) {
        return Ok((root, game.clone()));
    }
    let close = catalog::suggest(&games, slug);
    Err(fail(
        "unknown_game",
        format!("There is no game '{slug}'."),
        if close.is_empty() {
            "List the games and use one of their slugs.".to_string()
        } else {
            format!("Did you mean: {}?", close.join(", "))
        },
        close.first().map_or_else(
            || "game_list".to_string(),
            |s| format!("game_report {{\"slug\": \"{s}\"}}"),
        ),
    ))
}

/// Shared by the CLI, the MCP server and the command registry: `tool_game_list`.
pub fn tool_game_list(args: &Value) -> ToolOutcome {
    let root = repo_root(args)?;
    let filter = args.get("filter").and_then(Value::as_str).unwrap_or("");
    let kind = kind_arg(args);
    let games: Vec<catalog::GameEntry> = catalog::discover(root.as_path())
        .into_iter()
        .filter(|g| kind.is_none_or(|k| g.kind == k) && catalog::matches(g, filter))
        .collect();
    let failing: Vec<&str> = games
        .iter()
        .filter(|g| g.verify.as_ref().is_some_and(|v| !v.pass))
        .map(|g| g.slug.as_str())
        .collect();
    let unverified = games.iter().filter(|g| g.verify.is_none()).count();
    let count = |kind: GameKind| games.iter().filter(|g| g.kind == kind).count();
    let mut summary = format!(
        "{} ({} Godot, {} web, {} Bevy)",
        plural(games.len(), "game", "games"),
        count(GameKind::Godot),
        count(GameKind::Web),
        count(GameKind::Bevy)
    );
    if failing.is_empty() {
        summary.push_str(if unverified == 0 {
            "; all passed their last gate."
        } else {
            "; none failing."
        });
    } else {
        let _ = write!(summary, "; failing last gate: {}.", failing.join(", "));
    }
    if unverified > 0 {
        let _ = write!(
            summary,
            " {} never verified.",
            plural(unverified, "game", "games")
        );
    }
    if games.is_empty() && !filter.is_empty() {
        return Err(fail(
            "no_match",
            format!("No game matches '{filter}'."),
            "Try a shorter filter, or none.",
            "game_list",
        ));
    }
    Ok((summary, json!({"count": games.len(), "games": games})))
}

/// Shared by the CLI, the MCP server and the command registry: `tool_game_report`.
pub fn tool_game_report(args: &Value) -> ToolOutcome {
    let (_, game) = find_game(args)?;
    let status = match &game.verify {
        Some(v) if v.pass => format!("passed its last gate ({})", v.progress),
        Some(v) => format!(
            "FAILED its last gate: {}",
            v.failed.clone().unwrap_or_default()
        ),
        None => "has never been verified".to_string(),
    };
    let summary = format!(
        "{} ({} game, {}) {status}.",
        game.title,
        game.kind.as_str(),
        game.group
    );
    Ok((summary, catalog::report(&game)))
}

/// Shared by the CLI, the MCP server and the command registry: `tool_spatial_query`.
pub fn tool_spatial_query(args: &Value) -> ToolOutcome {
    let text = scene_source(args)?;
    let Some(path) = args.get("node_path").and_then(Value::as_str) else {
        return Err(fail(
            "missing_node_path",
            "node_path is required.",
            "Use '.' for the whole scene or a path like 'Level/Platform3'.",
            "scene_inspect {\"operation\": \"tree\"} to see the node paths",
        ));
    };
    let mut value = spatial_query(text.as_str(), path).map_err(|reason| {
        fail(
            "node_not_found",
            format!("Cannot measure '{path}': {reason}"),
            "Node paths are relative to the scene root, separated by '/'.",
            format!(
                "scene_inspect {{\"operation\": \"find\", \"query\": \"{}\"}}",
                path.rsplit('/').next().unwrap_or(path)
            ),
        )
    })?;
    let triple = |key: &str| -> Vec<f64> {
        value
            .get(key)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_f64).collect())
            .unwrap_or_default()
    };
    let (min, max) = (triple("min"), triple("max"));
    let size: Vec<f64> = min
        .iter()
        .zip(max.iter())
        .map(|(a, b)| ((b - a) * 1000.0).round() / 1000.0)
        .collect();
    let empty = value.get("empty").and_then(Value::as_bool).unwrap_or(false);
    let summary = if empty || size.len() != 3 {
        format!("'{path}' has no geometry, so its box is empty.")
    } else {
        format!(
            "'{path}' spans {:.2} x {:.2} x {:.2} m, from {:?} to {:?}.",
            size[0],
            size[1],
            size[2],
            round3(&min),
            round3(&max)
        )
    };
    if let Some(map) = value.as_object_mut() {
        map.insert("size".to_string(), json!(size));
    }
    Ok((summary, value))
}
fn round3(v: &[f64]) -> Vec<f64> {
    v.iter().map(|x| (x * 1000.0).round() / 1000.0).collect()
}

/// Shared by the CLI, the MCP server and the command registry: `tool_export_scene`.
pub fn tool_export_scene(args: &Value) -> ToolOutcome {
    let format = args
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or(crate::spec::FORMAT_GODOT4);
    if !crate::spec::FORMATS.contains(&format) {
        return Err(fail(
            "unsupported_format",
            format!("There is no '{format}' export."),
            format!(
                "Use one of: {}. glTF loads in Blender, Unity, three.js and most other engines.",
                crate::spec::FORMATS.join(", ")
            ),
            "export_scene {\"format\": \"gltf\", \"scene\": <same scene>}",
        ));
    }
    let Some(scene) = args.get("scene") else {
        return Err(fail(
            "missing_scene",
            "scene is required.",
            "Pass a scene spec object with at least nodes[].",
            "export_scene {\"scene\": {\"nodes\": [{\"name\": \"World\", \"type\": \"Node3D\"}]}}",
        ));
    };
    let spec: crate::SceneSpec = serde_json::from_value(scene.clone()).map_err(|err| {
        fail(
            "invalid_spec",
            format!("The scene spec does not parse: {err}"),
            "Check field names and types against the scene description in tools/list.",
            "export_scene with a minimal spec, then add nodes",
        )
    })?;
    let outcome = crate::export_spec(&spec, format).map_err(|reason| {
        fail(
            "export_failed",
            format!("Export failed: {reason}"),
            "Every parent must name an earlier node; resources must be declared before use.",
            "scene_lint on the exported text once it exports",
        )
    })?;
    let summary = format!(
        "Exported {} as {format}: {} bytes, uid {}.",
        plural(outcome.nodes, "node", "nodes"),
        outcome.text.len(),
        outcome.uid
    );
    Ok((
        summary,
        json!({"format": format, "uid": outcome.uid, "nodes": outcome.nodes, "bytes": outcome.text.len(), "text": outcome.text}),
    ))
}

/// Shared by the CLI, the MCP server and the command registry: `tool_game_verify`.
pub fn tool_game_verify(args: &Value) -> ToolOutcome {
    let (root, game) = find_game(args)?;
    let timeout = args.get("timeout_s").and_then(Value::as_u64).unwrap_or(300);
    match game.kind {
        GameKind::Web => {
            if std::env::var("GAMECRAFT_MCP_ALLOW_WEB").ok().as_deref() != Some("1") {
                return Err(fail("web_gate_disabled", "Running the web gate is turned off on this server.", "It launches node, bun and headless Chromium, so it is opt-in. Restart the server with GAMECRAFT_MCP_ALLOW_WEB=1, or read the last result instead.", format!("game_report {{\"slug\": \"{}\"}}", game.slug)));
            }
            let run = catalog::web_verify(root.as_path(), game.slug.as_str(), timeout)
                .map_err(|reason| fail("web_gate_failed_to_run", reason, "Check that node and bun are installed and web/node_modules exists (bun install in web/).", "game_report for the previous result"))?;
            let pass = run
                .pointer("/verify/pass")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let failed = run
                .pointer("/verify/failed")
                .and_then(Value::as_str)
                .unwrap_or("");
            let summary = if pass {
                format!("{} passes its gate.", game.title)
            } else {
                format!("{} FAILS its gate: {failed}.", game.title)
            };
            let mut data = run;
            if let Some(map) = data.as_object_mut() {
                map.insert("pass".to_string(), Value::Bool(pass));
                map.insert("kind".to_string(), json!("web"));
            }
            Ok((summary, data))
        }
        GameKind::Bevy => {
            if std::env::var("GAMECRAFT_MCP_ALLOW_BEVY").ok().as_deref() != Some("1") {
                return Err(fail("bevy_gate_disabled", "Running the Bevy gate is turned off on this server.", "It runs cargo (a cold optimised build takes about 15 minutes) and opens a game window, so it is opt-in. Restart the server with GAMECRAFT_MCP_ALLOW_BEVY=1, or read the last result instead.", format!("game_report {{\"slug\": \"{}\", \"kind\": \"bevy\"}}", game.slug)));
            }
            let run = catalog::bevy_verify(root.as_path(), game.slug.as_str(), timeout)
                .map_err(|reason| fail("bevy_gate_failed_to_run", reason, "Check that cargo is on PATH and the repo builds (cargo build -p gamecraft-bevy).", "game_report for the previous result"))?;
            let pass = run
                .pointer("/verify/pass")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let failed = run
                .pointer("/verify/failed")
                .and_then(Value::as_str)
                .unwrap_or("");
            let summary = if pass {
                format!("{} passes its gate.", game.title)
            } else {
                format!("{} FAILS its gate: {failed}.", game.title)
            };
            let mut data = run;
            if let Some(map) = data.as_object_mut() {
                map.insert("pass".to_string(), Value::Bool(pass));
                map.insert("kind".to_string(), json!("bevy"));
            }
            Ok((summary, data))
        }
        GameKind::Godot => {
            if std::env::var("GAMECRAFT_MCP_ALLOW_GODOT").ok().as_deref() != Some("1") {
                return Err(fail("godot_gate_disabled", "Running Godot is turned off on this server.", "It launches a local Godot process, so it is opt-in. Restart the server with GAMECRAFT_MCP_ALLOW_GODOT=1, or read the last recorded trace instead.", format!("game_report {{\"slug\": \"{}\", \"kind\": \"godot\"}}", game.slug)));
            }
            let run = catalog::godot_verify(game.dir.as_path(), 3600, true, Some(timeout))
                .map_err(|reason| {
                    fail(
                        "godot_failed_to_run",
                        reason,
                        "Set GAMECRAFT_GODOT to the Godot 4 executable if it is not on PATH.",
                        "game_report for the recorded trace",
                    )
                })?;
            let won = run.get("won").and_then(Value::as_bool).unwrap_or(false);
            let summary = if won {
                format!("{} reached its win marker in headless Godot.", game.title)
            } else {
                format!("{} did NOT reach a win marker.", game.title)
            };
            let mut data = run;
            if let Some(map) = data.as_object_mut() {
                map.insert("pass".to_string(), Value::Bool(won));
                map.insert("kind".to_string(), json!("godot"));
            }
            Ok((summary, data))
        }
    }
}

/// Shared by the CLI, the MCP server and the command registry: `tool_game_create`.
pub fn tool_game_create(args: &Value) -> ToolOutcome {
    if args
        .get("explain")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || args.get("spec").is_none()
    {
        let guide = gamecraft_forge::llm::spec_prompt("<your idea>");
        if args.get("spec").is_none()
            && !args
                .get("explain")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            return Err(fail(
                "missing_spec",
                "No spec given.",
                "Read the spec format first, then call again with spec.",
                "game_create {\"explain\": true}",
            ));
        }
        return Ok(("The GameSpec format and design rules are in guide; design a spec and call game_create with it.".to_string(), json!({"guide": guide})));
    }
    let root = repo_root(args)?;
    let spec: gamecraft_forge::create::GameSpec = serde_json::from_value(
        args.get("spec").cloned().unwrap_or(Value::Null),
    )
    .map_err(|err| {
        fail(
            "invalid_spec",
            format!("The spec does not parse: {err}"),
            "Field names and types must match the GameSpec format.",
            "game_create {\"explain\": true}",
        )
    })?;
    if let Err(reason) = gamecraft_forge::create::validate(&spec) {
        return Err(fail(
            "spec_rejected",
            format!("The spec was rejected: {reason}"),
            "Fix every listed problem; the rules are in the guide.",
            "game_create {\"explain\": true}",
        ));
    }
    let summary = gamecraft_forge::create::create_game(&spec, root.as_path())
        .map_err(|reason| fail("not_completable", format!("The game was not written: {reason}"), "Usually a gap too wide to jump or a platform too low; the Rust replay must reach the goal.", "game_create {\"explain\": true} for the physics limits"))?;
    let dir = root.join("depot").join("games").join(spec.slug.as_str());
    Ok((
        format!(
            "Created '{}' ({}) in {}: {summary}",
            spec.title,
            spec.archetype,
            dir.display()
        ),
        json!({"slug": spec.slug, "dir": dir, "summary": summary}),
    ))
}

/// Shared by the CLI, the MCP server and the command registry: `tool_trellis_asset`.
///
/// Plan-only: returns the deterministic sidecar invocation plus the
/// licence-ledger row. Execution happens in `gamecraft-cli trellis --execute`,
/// never from this tool (no child processes behind MCP).
pub fn tool_trellis_asset(args: &Value) -> ToolOutcome {
    let Some(slug) = args.get("slug").and_then(Value::as_str) else {
        return Err(fail(
            "missing_slug",
            "slug is required.",
            "Use an asset slug like 'farmhouse-hero'.",
            "asset.trellis {\"slug\": \"farmhouse-hero\", \"image\": \"shots/farmhouse.png\"}",
        ));
    };
    let Some(image) = args.get("image").and_then(Value::as_str) else {
        return Err(fail(
            "missing_image",
            "image is required: TRELLIS.2 reconstructs from an image.",
            "Pass the source PNG path; prompt_ref is ledger context only.",
            "asset.trellis {\"slug\": \"farmhouse-hero\", \"image\": \"shots/farmhouse.png\"}",
        ));
    };
    let mut config = gamecraft_trellis::TrellisConfig::from_env();
    if let Some(out) = args.get("out_dir").and_then(Value::as_str) {
        config.out_dir = std::path::PathBuf::from(out);
    }
    if let Some(endpoint) = args.get("endpoint").and_then(Value::as_str) {
        config.backend = gamecraft_trellis::Backend::Http;
        config.endpoint = endpoint.to_string();
    }
    let request = gamecraft_trellis::TrellisRequest {
        slug: slug.to_string(),
        image: std::path::PathBuf::from(image),
        prompt_ref: args
            .get("prompt_ref")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        poly_budget: args
            .get("poly_budget")
            .and_then(Value::as_u64)
            .unwrap_or(20_000),
        extent_limit_m: args
            .get("extent_limit_m")
            .and_then(Value::as_f64)
            .unwrap_or(8.0),
    };
    let plan = gamecraft_trellis::plan(&config, &request).map_err(|reason| {
        fail(
            "trellis_plan_rejected",
            format!("The request was rejected: {reason}"),
            "Fix the slug, image path or budget.",
            "asset.trellis with a safe slug and an image path",
        )
    })?;
    if let Err(reason) = gamecraft_trellis::ledger_complete(&plan.ledger) {
        return Err(fail(
            "trellis_ledger_incomplete",
            format!("Ledger incomplete: {reason}"),
            "Fill every ledger field before human approval.",
            "gamecraft-cli trellis --help",
        ));
    }
    let sidecar = gamecraft_trellis::probe(&config).unwrap_or_else(|reason| reason);
    Ok((
        format!(
            "Plan for '{}': {} -> {}. {}.",
            plan.ledger.slug,
            plan.argv.join(" "),
            plan.out_glb.display(),
            plan.ledger.license,
        ),
        json!({
            "slug": plan.ledger.slug,
            "argv": plan.argv,
            "out_glb": plan.out_glb,
            "ledger": plan.ledger,
            "sidecar": sidecar,
            "execute": "gamecraft-cli trellis --execute",
        }),
    ))
}

/// Shared by the CLI, the MCP server and the command registry: `tool_scene_verify`.
pub fn tool_scene_verify(args: &Value) -> ToolOutcome {
    if std::env::var("GAMECRAFT_MCP_ALLOW_GODOT").ok().as_deref() != Some("1") {
        return Err(fail("godot_gate_disabled", "scene_verify is turned off on this server.", "It launches a local Godot process, so it is opt-in. Restart the server with GAMECRAFT_MCP_ALLOW_GODOT=1.", "scene_lint on the project's main.tscn needs no engine"));
    }
    let Some(game) = args.get("game_path").and_then(Value::as_str) else {
        return Err(fail(
            "missing_game_path",
            "game_path is required.",
            "Pass the folder holding project.godot.",
            "game_list to find catalog games",
        ));
    };
    let frames = args.get("frames").and_then(Value::as_u64).unwrap_or(3600);
    let autoplay = args
        .get("autoplay")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let run = catalog::godot_verify(
        std::path::Path::new(game),
        frames,
        autoplay,
        args.get("timeout_s").and_then(Value::as_u64),
    )
    .map_err(|reason| {
        fail(
            "godot_failed_to_run",
            reason,
            "Set GAMECRAFT_GODOT to the Godot 4 executable if it is not on PATH.",
            "scene_lint on main.tscn",
        )
    })?;
    let won = run.get("won").and_then(Value::as_bool).unwrap_or(false);
    let timed_out = run
        .get("timed_out")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let summary = if won {
        "Win marker reached.".to_string()
    } else if timed_out {
        "No win: the engine hit the time limit.".to_string()
    } else {
        "No win marker within the frame budget.".to_string()
    };
    Ok((summary, run))
}

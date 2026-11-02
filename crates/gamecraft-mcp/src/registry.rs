//! The command registry: the whole `GameCraft` API as a list of named commands.
//!
//! This is the Craft app contract (`commands --json`, `describe <id>`, `run --cmd
//! 'id={json}'`). Every capability has a stable dotted id, a menu path and a typed
//! parameter list, and runs through [`run`]. The CLI and the MCP server both call
//! [`run`], so they cannot disagree. MCP tool names are the ids with `.` turned into `_`
//! (see [`tool_name`]), because several MCP hosts reject dots in tool names.
//!
//! Ids are API: they are never renamed, only added.

use crate::replay_cmds::{replay_info, replay_record, replay_verify};
use crate::tools::{
    fail, tool_export_scene, tool_game_create, tool_game_list, tool_game_report, tool_game_verify,
    tool_scene_inspect, tool_scene_lint, tool_scene_verify, tool_spatial_query, ToolError,
    ToolOutcome,
};
use serde_json::{json, Value};
use std::fmt::Write as _;

/// One parameter of a command.
#[derive(Debug, Clone, Copy)]
pub struct ParamSpec {
    /// Key in the params object.
    pub name: &'static str,
    /// `string`, `integer` or `boolean`.
    pub kind: &'static str,
    /// Must be present.
    pub required: bool,
    /// What it does.
    pub help: &'static str,
    /// Allowed values, if the set is closed (empty means free).
    pub choices: &'static [&'static str],
}

/// One command, as `commands --json` lists it.
#[derive(Debug, Clone, Copy)]
pub struct CommandSpec {
    /// Stable dotted id, `<area>.<verb>`.
    pub id: &'static str,
    /// Menu label.
    pub label: &'static str,
    /// Menu path, `Area/Item`.
    pub menu: &'static str,
    /// Keyboard shortcut in the desktop app, if bound.
    pub shortcut: Option<&'static str>,
    /// One sentence for `describe`.
    pub summary: &'static str,
    /// Changes nothing outside its own output file.
    pub readonly: bool,
    /// Parameters.
    pub params: &'static [ParamSpec],
    body: fn(&Value) -> ToolOutcome,
}

const fn p(
    name: &'static str,
    kind: &'static str,
    required: bool,
    help: &'static str,
) -> ParamSpec {
    ParamSpec {
        name,
        kind,
        required,
        help,
        choices: &[],
    }
}

const SCENE_SRC: [ParamSpec; 3] = [
    p(
        "scene_file",
        "string",
        false,
        "Path to a .tscn (res:// needs root).",
    ),
    p(
        "scene_text",
        "string",
        false,
        "The .tscn text itself, instead of a path.",
    ),
    p(
        "root",
        "string",
        false,
        "Project folder that res:// paths resolve against.",
    ),
];

const INSPECT: [ParamSpec; 8] = [
    SCENE_SRC[0],
    SCENE_SRC[1],
    SCENE_SRC[2],
    ParamSpec {
        name: "operation",
        kind: "string",
        required: false,
        help: "What to read; tree by default.",
        choices: &["tree", "property", "signals", "find"],
    },
    p("node_path", "string", false, "Node for operation property."),
    p("query", "string", false, "Substring for operation find."),
    p("depth", "integer", false, "Tree depth."),
    p("limit", "integer", false, "Page size."),
];

const LIST: [ParamSpec; 3] = [
    p(
        "filter",
        "string",
        false,
        "Substring of slug, title or group.",
    ),
    ParamSpec {
        name: "kind",
        kind: "string",
        required: false,
        help: "Engine family.",
        choices: &["godot", "web", "bevy"],
    },
    p(
        "root",
        "string",
        false,
        "Repo folder; found automatically when omitted.",
    ),
];

const REPORT: [ParamSpec; 3] = [
    p("slug", "string", true, "Folder name from game.list."),
    LIST[1],
    LIST[2],
];

const RECORD: [ParamSpec; 8] = [
    ParamSpec {
        name: "game",
        kind: "string",
        required: true,
        help: "Which game to run.",
        choices: &["maze", "murmuration"],
    },
    ParamSpec {
        name: "level",
        kind: "string",
        required: false,
        help: "Maze preset; challenge by default.",
        choices: &["depot", "challenge"],
    },
    p("size", "integer", false, "Maze size, odd, 5 to 151."),
    p("seed", "integer", false, "Level seed."),
    p("count", "integer", false, "Murmuration flock size."),
    p("ticks", "integer", false, "Murmuration step limit."),
    p(
        "out",
        "string",
        false,
        "Write the document here (must end in .gcraft).",
    ),
    p("overwrite", "boolean", false, "Replace an existing file."),
];

const SPATIAL: [ParamSpec; 4] = [
    SCENE_SRC[0],
    SCENE_SRC[1],
    SCENE_SRC[2],
    p(
        "node_path",
        "string",
        true,
        "Scene path from the root, or . for the whole scene.",
    ),
];

const EXPORT: [ParamSpec; 2] = [
    p(
        "scene",
        "object",
        true,
        "Scene spec: nodes[] with name, type, parent, transform.",
    ),
    p(
        "format",
        "string",
        false,
        "godot4 (default) or gltf; see describe output of tools/list.",
    ),
];

const GAME_VERIFY: [ParamSpec; 4] = [
    p("slug", "string", true, "Folder name from game.list."),
    LIST[1],
    LIST[2],
    p(
        "timeout_s",
        "integer",
        false,
        "Give up after this many seconds.",
    ),
];

const GAME_CREATE: [ParamSpec; 2] = [
    p(
        "spec",
        "object",
        false,
        "GameSpec; omit (or explain true) to get the format guide.",
    ),
    p(
        "explain",
        "boolean",
        false,
        "Return the spec guide instead of building.",
    ),
];

const SCENE_VERIFY: [ParamSpec; 4] = [
    p("game_path", "string", true, "Folder holding project.godot."),
    p(
        "frames",
        "integer",
        false,
        "Frames to run; 3600 by default.",
    ),
    p(
        "autoplay",
        "boolean",
        false,
        "Let the scripted player play; true by default.",
    ),
    p(
        "timeout_s",
        "integer",
        false,
        "Give up after this many seconds.",
    ),
];

const PATH: [ParamSpec; 1] = [p("path", "string", true, "A .gcraft file.")];

/// Every command, in menu order.
pub const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: "game.list",
        label: "List Games",
        menu: "Library/List Games",
        shortcut: None,
        summary: "List the depot, web and Bevy games with their last gate result.",
        readonly: true,
        params: &LIST,
        body: tool_game_list,
    },
    CommandSpec {
        id: "game.report",
        label: "Game Report",
        menu: "Library/Game Report",
        shortcut: None,
        summary: "Everything known about one game: files, gate result, how to run it.",
        readonly: true,
        params: &REPORT,
        body: tool_game_report,
    },
    CommandSpec {
        id: "scene.lint",
        label: "Lint Scene",
        menu: "Scene/Lint",
        shortcut: Some("Ctrl+L"),
        summary: "Static checks on a .tscn: missing collision shapes, dead signal targets, unresolved resources.",
        readonly: true,
        params: &SCENE_SRC,
        body: tool_scene_lint,
    },
    CommandSpec {
        id: "scene.inspect",
        label: "Inspect Scene",
        menu: "Scene/Inspect",
        shortcut: Some("Ctrl+I"),
        summary: "Read a scene: node tree, one node's properties, signal connections, or a name search.",
        readonly: true,
        params: &INSPECT,
        body: tool_scene_inspect,
    },
    CommandSpec {
        id: "scene.spatial_query",
        label: "Bounding Box",
        menu: "Scene/Bounding Box",
        shortcut: None,
        summary: "World-space bounding box of one node and everything under it.",
        readonly: true,
        params: &SPATIAL,
        body: tool_spatial_query,
    },
    CommandSpec {
        id: "scene.export",
        label: "Export Scene",
        menu: "Scene/Export",
        shortcut: None,
        summary: "Turn a scene spec into engine text (Godot 4 .tscn) or glTF.",
        readonly: true,
        params: &EXPORT,
        body: tool_export_scene,
    },
    CommandSpec {
        id: "scene.verify",
        label: "Verify Scene",
        menu: "Scene/Verify in Godot",
        shortcut: None,
        summary: "Boot a Godot project headless and play it. Opt-in: needs GAMECRAFT_MCP_ALLOW_GODOT=1.",
        readonly: false,
        params: &SCENE_VERIFY,
        body: tool_scene_verify,
    },
    CommandSpec {
        id: "game.verify",
        label: "Verify Game",
        menu: "Library/Verify Game",
        shortcut: None,
        summary: "Run a game's gate (Godot, web or Bevy) and record the result in its verify.json.",
        readonly: false,
        params: &GAME_VERIFY,
        body: tool_game_verify,
    },
    CommandSpec {
        id: "game.create",
        label: "Create Game",
        menu: "Library/Create Game",
        shortcut: None,
        summary: "Build a game from a GameSpec, or return the spec guide when none is given.",
        readonly: false,
        params: &GAME_CREATE,
        body: tool_game_create,
    },
    CommandSpec {
        id: "replay.record",
        label: "Record Replay",
        menu: "Replay/Record",
        shortcut: Some("Ctrl+R"),
        summary: "Run a game's built-in autoplay and keep its inputs as a .gcraft document.",
        readonly: false,
        params: &RECORD,
        body: replay_record,
    },
    CommandSpec {
        id: "replay.verify",
        label: "Verify Replay",
        menu: "Replay/Verify",
        shortcut: None,
        summary: "Re-simulate a .gcraft from its seed and inputs and compare with the recorded result.",
        readonly: true,
        params: &PATH,
        body: replay_verify,
    },
    CommandSpec {
        id: "replay.info",
        label: "Replay Info",
        menu: "Replay/Info",
        shortcut: None,
        summary: "Read a .gcraft header: game, config, input count and recorded result. Does not simulate.",
        readonly: true,
        params: &PATH,
        body: replay_info,
    },
];

/// Look a command up by id.
#[must_use]
pub fn find(id: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|c| c.id == id)
}

/// MCP tool name for a command id (dots are not portable in tool names).
#[must_use]
pub fn tool_name(id: &str) -> String {
    id.replace('.', "_")
}

/// Command id for an MCP tool name, when it names a registered command.
#[must_use]
pub fn id_for_tool(name: &str) -> Option<&'static str> {
    COMMANDS
        .iter()
        .find(|c| tool_name(c.id) == name || c.id == name)
        .map(|c| c.id)
}

fn spec_json(c: &CommandSpec) -> Value {
    let params: Vec<Value> = c
        .params
        .iter()
        .map(|a| {
            let mut v = json!({
                "name": a.name,
                "type": a.kind,
                "required": a.required,
                "help": a.help,
            });
            if !a.choices.is_empty() {
                v["choices"] = json!(a.choices);
            }
            v
        })
        .collect();
    json!({
        "id": c.id,
        "label": c.label,
        "menu": c.menu,
        "shortcut": c.shortcut,
        "summary": c.summary,
        "readonly": c.readonly,
        "enabled": true,
        "params": params,
        "tool": tool_name(c.id),
    })
}

/// `commands --json`: every command with its menu, shortcut and parameters.
#[must_use]
pub fn list_json() -> Value {
    json!({"count": COMMANDS.len(), "commands": COMMANDS.iter().map(spec_json).collect::<Vec<_>>()})
}

/// `describe <id>`: one command, or `None` for an unknown id.
#[must_use]
pub fn describe(id: &str) -> Option<Value> {
    find(id).map(spec_json)
}

/// The ids closest to `typo`, for "did you mean".
#[must_use]
pub fn suggest(typo: &str) -> Vec<&'static str> {
    let t = typo.to_lowercase();
    let area = t.split(['.', '_']).next().unwrap_or("");
    let mut hits: Vec<&'static str> = COMMANDS
        .iter()
        .filter(|c| c.id.contains(&t) || (!area.is_empty() && c.id.starts_with(area)))
        .map(|c| c.id)
        .collect();
    hits.truncate(4);
    hits
}

/// Check required parameters and that the value is an object.
fn check(spec: &CommandSpec, params: &Value) -> Result<(), ToolError> {
    if !params.is_object() {
        return Err(fail(
            "params_not_object",
            format!("Parameters for {} must be a JSON object.", spec.id),
            "Wrap them in braces.",
            format!("run --cmd '{}={{}}'", spec.id),
        ));
    }
    for a in spec.params.iter().filter(|a| a.required) {
        if params.get(a.name).is_none_or(Value::is_null) {
            return Err(fail(
                "missing_param",
                format!("{} needs {}.", spec.id, a.name),
                format!("{}: {}", a.name, a.help),
                format!("describe {}", spec.id),
            ));
        }
    }
    for a in spec.params.iter().filter(|a| !a.choices.is_empty()) {
        if let Some(v) = params.get(a.name).and_then(Value::as_str) {
            if !a.choices.contains(&v) {
                return Err(fail(
                    "bad_choice",
                    format!("{} is not a valid {} for {}.", v, a.name, spec.id),
                    format!("Use one of: {}.", a.choices.join(", ")),
                    format!("describe {}", spec.id),
                ));
            }
        }
    }
    Ok(())
}

/// Run a command. Never panics; an unknown id or bad parameters come back as a
/// [`ToolError`] with a suggestion and a concrete next call.
#[allow(clippy::missing_errors_doc)] // the error is the documented ToolError: code, suggestion, remediation
pub fn run(id: &str, params: &Value) -> ToolOutcome {
    let Some(spec) = find(id) else {
        let close = suggest(id);
        return Err(fail(
            "unknown_command",
            format!("There is no command '{id}'."),
            if close.is_empty() {
                "List the commands and use one of their ids.".to_string()
            } else {
                format!("Did you mean: {}?", close.join(", "))
            },
            "commands --json",
        ));
    };
    check(spec, params)?;
    (spec.body)(params)
}

/// Largest params file `id=@file` will read.
const MAX_PARAMS_FILE: u64 = 1024 * 1024;

/// Split a `--cmd` argument: `id`, `id={json}` or `id=@params.json`.
///
/// # Errors
/// A message when the JSON after `=` does not parse.
pub fn parse_cmd(arg: &str) -> Result<(&str, Value), String> {
    match arg.split_once('=') {
        None => Ok((arg.trim(), json!({}))),
        Some((id, raw)) => {
            let raw = raw.trim();
            let id = id.trim();
            // `id=@file.json` reads the params from a file: Windows PowerShell 5 strips the
            // double quotes out of arguments handed to a native program, so inline JSON
            // there needs `\"` escapes, and a file avoids the problem.
            let owned;
            let raw = if let Some(file) = raw.strip_prefix('@') {
                let meta = std::fs::metadata(file)
                    .map_err(|e| format!("params file {file} for {id}: {e}"))?;
                if meta.len() > MAX_PARAMS_FILE {
                    return Err(format!(
                        "params file {file} is over {MAX_PARAMS_FILE} bytes"
                    ));
                }
                owned = std::fs::read_to_string(file)
                    .map_err(|e| format!("params file {file} for {id}: {e}"))?;
                owned.as_str()
            } else {
                raw
            };
            let params = if raw.trim().is_empty() {
                json!({})
            } else {
                serde_json::from_str(raw)
                    .map_err(|e| format!("params for {id} are not valid JSON: {e}"))?
            };
            Ok((id, params))
        }
    }
}

/// Render a [`ToolOutcome`] as the flat JSON object `run --json` prints.
#[must_use]
pub fn outcome_json(outcome: &ToolOutcome) -> Value {
    match outcome {
        Ok((summary, data)) => {
            let mut v = if data.is_object() {
                data.clone()
            } else {
                json!({"data": data})
            };
            v["success"] = json!(true);
            v["message"] = json!(summary);
            v
        }
        Err(e) => json!({
            "success": false,
            "error": e.code,
            "message": e.message,
            "dialogic": {"suggestion": e.suggestion, "remediation": e.remediation},
        }),
    }
}

// ------------------------------------------------------------------ command line

/// What a command line invocation produced; the caller prints it and exits with `code`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    /// For standard output.
    pub stdout: String,
    /// For standard error.
    pub stderr: String,
    /// Process exit code: 0 done, 1 the command failed, 2 bad usage.
    pub code: i32,
}

impl CliOutput {
    fn out(text: String) -> Self {
        Self {
            stdout: text,
            stderr: String::new(),
            code: 0,
        }
    }
    fn usage(text: impl Into<String>) -> Self {
        Self {
            stdout: String::new(),
            stderr: text.into(),
            code: 2,
        }
    }
}

/// Usage text for the registry subcommands.
pub const CLI_USAGE: &str = "registry commands:
  commands [--json]              list every command (id, menu, shortcut, params)
  describe <id> [--json]         one command in full
  run --cmd 'id={json}' [--json]  run a command; params are a JSON object, 'id' alone means {}
exit codes: 0 done, 1 the command failed, 2 bad usage";

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

/// `commands [--json]`.
#[must_use]
pub fn cli_commands(json: bool) -> CliOutput {
    if json {
        return CliOutput::out(pretty(&list_json()));
    }
    let mut s = String::new();
    for c in COMMANDS {
        let keys = c.shortcut.map_or(String::new(), |k| format!("  [{k}]"));
        let _ = write!(s, "{:<14} {:<22}{keys}\n    {}\n", c.id, c.menu, c.summary);
    }
    CliOutput::out(s)
}

/// `describe <id> [--json]`.
#[must_use]
pub fn cli_describe(id: &str, json: bool) -> CliOutput {
    let Some(d) = describe(id) else {
        let close = suggest(id);
        let hint = if close.is_empty() {
            String::new()
        } else {
            format!(" Did you mean: {}?", close.join(", "))
        };
        return CliOutput::usage(format!("no command '{id}'.{hint} Try: commands"));
    };
    if json {
        return CliOutput::out(pretty(&d));
    }
    let mut s = format!(
        "{}  ({})\n{}\nmenu: {}   shortcut: {}   read-only: {}\nparams:\n",
        d["id"].as_str().unwrap_or(id),
        d["label"].as_str().unwrap_or(""),
        d["summary"].as_str().unwrap_or(""),
        d["menu"].as_str().unwrap_or(""),
        d["shortcut"].as_str().unwrap_or("none"),
        d["readonly"],
    );
    for a in d["params"].as_array().map_or(&[][..], Vec::as_slice) {
        let choices = a
            .get("choices")
            .and_then(Value::as_array)
            .map(|c| {
                let names: Vec<&str> = c.iter().filter_map(Value::as_str).collect();
                format!(" one of {}", names.join("|"))
            })
            .unwrap_or_default();
        let _ = writeln!(
            s,
            "  {}{} ({}{choices}): {}",
            a["name"].as_str().unwrap_or(""),
            if a["required"] == true { "*" } else { "" },
            a["type"].as_str().unwrap_or(""),
            a["help"].as_str().unwrap_or(""),
        );
    }
    CliOutput::out(s)
}

/// `run --cmd 'id={json}' [--json]`.
#[must_use]
pub fn cli_run(cmd: &str, json: bool) -> CliOutput {
    let (id, params) = match parse_cmd(cmd) {
        Ok(parsed) => parsed,
        Err(reason) => return CliOutput::usage(reason),
    };
    let outcome = run(id, &params);
    let failed = outcome.is_err();
    let stdout = if json {
        pretty(&outcome_json(&outcome))
    } else {
        match &outcome {
            Ok((summary, _)) => summary.clone(),
            Err(e) => format!(
                "{}\nWhat to do: {}\nNext: {}",
                e.message, e.suggestion, e.remediation
            ),
        }
    };
    // A failed command still prints its JSON on stdout under --json, so scripts can parse it.
    if failed && !json {
        return CliOutput {
            stdout: String::new(),
            stderr: stdout,
            code: 1,
        };
    }
    CliOutput {
        stdout,
        stderr: String::new(),
        code: i32::from(failed),
    }
}

/// Dispatch raw arguments (without the program name) for the registry subcommands.
/// `None` means the first argument is not a registry subcommand.
#[must_use]
pub fn cli_args(args: &[String]) -> Option<CliOutput> {
    let first = args.first()?.as_str();
    let json = args.iter().any(|a| a == "--json");
    let rest: Vec<&str> = args[1..]
        .iter()
        .map(String::as_str)
        .filter(|a| *a != "--json")
        .collect();
    match first {
        "commands" => Some(cli_commands(json)),
        "describe" => Some(match rest.as_slice() {
            [id] => cli_describe(id, json),
            _ => CliOutput::usage("usage: describe <id> [--json]"),
        }),
        "run" => Some(match rest.as_slice() {
            ["--cmd", cmd] => cli_run(cmd, json),
            [one] if one.starts_with("--cmd=") => cli_run(&one["--cmd=".len()..], json),
            _ => CliOutput::usage("usage: run --cmd 'id={json}' [--json]"),
        }),
        "help" | "--help" | "-h" => Some(CliOutput::out(CLI_USAGE.to_string())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("gcraft-reg-{}-{name}", std::process::id()))
    }

    #[test]
    fn ids_are_unique_dotted_and_have_tool_names() {
        let mut seen = std::collections::HashSet::new();
        for c in COMMANDS {
            assert!(seen.insert(c.id), "duplicate id {}", c.id);
            assert_eq!(c.id.matches('.').count(), 1, "{} is not area.verb", c.id);
            assert!(!tool_name(c.id).contains('.'));
            assert!(c.menu.contains('/'), "{} has no menu path", c.id);
            assert_ne!(c.summary, "");
            assert_eq!(id_for_tool(&tool_name(c.id)), Some(c.id));
        }
    }

    #[test]
    fn list_has_the_contract_fields() {
        let v = list_json();
        assert_eq!(v["count"], COMMANDS.len());
        for c in v["commands"].as_array().unwrap_or(&vec![]) {
            for key in ["id", "label", "menu", "shortcut", "params", "enabled"] {
                assert!(c.get(key).is_some(), "{} lacks {key}", c["id"]);
            }
        }
    }

    #[test]
    fn describe_known_and_unknown() {
        assert_eq!(
            describe("replay.verify").map(|d| d["readonly"].clone()),
            Some(json!(true))
        );
        assert!(describe("replay.nope").is_none());
    }

    #[test]
    fn unknown_command_suggests_siblings() {
        let Err(e) = run("replay.rec", &json!({})) else {
            unreachable!("must fail");
        };
        assert_eq!(e.code, "unknown_command");
        assert!(e.suggestion.contains("replay.record"), "{}", e.suggestion);
    }

    #[test]
    fn missing_and_bad_params_are_tool_errors() {
        let Err(e) = run("replay.verify", &json!({})) else {
            unreachable!("must fail");
        };
        assert_eq!(e.code, "missing_param");
        let Err(e) = run("replay.record", &json!({"game": "chess"})) else {
            unreachable!("must fail");
        };
        assert_eq!(e.code, "bad_choice");
        let Err(e) = run("replay.info", &json!([1])) else {
            unreachable!("must fail");
        };
        assert_eq!(e.code, "params_not_object");
    }

    #[test]
    fn parse_cmd_forms() {
        assert_eq!(parse_cmd("game.list").ok(), Some(("game.list", json!({}))));
        assert_eq!(
            parse_cmd("game.list={\"filter\":\"maze\"}").ok(),
            Some(("game.list", json!({"filter": "maze"})))
        );
        assert!(parse_cmd("game.list={oops").is_err());
    }

    #[test]
    fn record_verify_info_round_trip_through_the_registry() {
        let dir = tmp("rt");
        let file = dir.join("depot.gcraft");
        let out = file.to_string_lossy().to_string();
        let rec = run(
            "replay.record",
            &json!({"game": "maze", "level": "depot", "out": out}),
        );
        let rec = outcome_json(&rec);
        assert_eq!(rec["success"], true, "{rec}");
        assert_eq!(rec["result"]["status"], "won");
        let again = outcome_json(&run(
            "replay.record",
            &json!({"game": "maze", "level": "depot", "out": out}),
        ));
        assert_eq!(again["error"], "exists", "{again}");
        let ver = outcome_json(&run("replay.verify", &json!({"path": out})));
        assert_eq!(ver["verified"], true, "{ver}");
        let info = outcome_json(&run("replay.info", &json!({"path": out})));
        assert_eq!(info["game"], "maze");
        // Tamper: flip one step in the file and the registry must say so.
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let edited = text.replacen("\"dx\":1", "\"dx\":-1", 1);
        assert_ne!(text, edited);
        std::fs::write(&file, edited).unwrap_or_default();
        let bad = outcome_json(&run("replay.verify", &json!({"path": out})));
        assert_eq!(bad["success"], false, "{bad}");
        assert_eq!(bad["error"], "replay_mismatch", "{bad}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_cmd_reads_params_from_a_file() {
        let dir = tmp("params");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("p.json");
        std::fs::write(&file, "{\"filter\": \"maze\"}").unwrap_or_default();
        let arg = format!("game.list=@{}", file.display());
        assert_eq!(
            parse_cmd(&arg).ok(),
            Some(("game.list", json!({"filter": "maze"})))
        );
        assert!(parse_cmd("game.list=@no/such/file.json").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn cli_forms_and_exit_codes() {
        let list =
            cli_args(&args(&["commands", "--json"])).unwrap_or_else(|| CliOutput::usage("none"));
        assert_eq!(list.code, 0);
        let parsed: Value = serde_json::from_str(&list.stdout).unwrap_or_default();
        assert_eq!(parsed["count"], COMMANDS.len());
        let text = cli_args(&args(&["commands"]))
            .map(|o| o.stdout)
            .unwrap_or_default();
        assert!(text.contains("replay.verify"));
        let d = cli_args(&args(&["describe", "replay.record"]))
            .map(|o| o.stdout)
            .unwrap_or_default();
        assert!(d.contains("game*"), "{d}");
        assert_eq!(
            cli_args(&args(&["describe", "nope.x"])).map(|o| o.code),
            Some(2)
        );
        assert_eq!(cli_args(&args(&["describe"])).map(|o| o.code), Some(2));
        assert_eq!(
            cli_args(&args(&["run", "--cmd", "replay.info"])).map(|o| o.code),
            Some(1)
        );
        assert_eq!(
            cli_args(&args(&["run", "--cmd=replay.info={bad"])).map(|o| o.code),
            Some(2)
        );
        let ok = cli_args(&args(&[
            "run",
            "--cmd",
            "game.list={\"filter\":\"zzz-nothing\"}",
            "--json",
        ]));
        assert_eq!(ok.as_ref().map(|o| o.code), Some(1));
        assert!(ok
            .map(|o| o.stdout)
            .unwrap_or_default()
            .contains("\"success\": false"));
        assert!(cli_args(&args(&["lint"])).is_none());
        assert!(cli_args(&[]).is_none());
    }

    #[test]
    fn output_must_be_a_gcraft_file() {
        let bad = outcome_json(&run(
            "replay.record",
            &json!({"game": "maze", "level": "depot", "out": "D:/x/evil.exe"}),
        ));
        assert_eq!(bad["error"], "wrong_extension", "{bad}");
        let bad = outcome_json(&run("replay.info", &json!({"path": "C:/Windows/win.ini"})));
        assert_eq!(bad["error"], "wrong_extension", "{bad}");
    }
}

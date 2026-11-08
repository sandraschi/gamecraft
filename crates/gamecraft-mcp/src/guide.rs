//! Words about games: `describe`, `help` and `doctor`, for the CLI and the MCP server.
//!
//! Everything here is generated from what is on disk (`game.json`, `CHEATSHEET.md`,
//! `README.md`, the gate records), so it cannot drift from the games. Nothing is
//! hand-written per game.

use crate::catalog::{self, GameEntry, GameKind};
use crate::launch;
use serde::Serialize;
use serde_json::{json, Value};
use std::fmt::Write as _;
use std::path::Path;

/// `| label | value |` rows of a cheat sheet table, header and separator rows removed.
#[must_use]
pub fn sheet_rows(sheet: &str) -> Vec<(String, String)> {
    sheet
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('|'))
        .filter_map(|l| {
            let cells: Vec<&str> = l.trim_matches('|').split('|').map(str::trim).collect();
            let (label, value) = (cells.first()?, cells.get(1)?);
            let separator = label.chars().all(|c| c == '-' || c == ':' || c == ' ');
            (!separator && !label.eq_ignore_ascii_case("what"))
                .then(|| ((*label).to_string(), (*value).to_string()))
        })
        .collect()
}

fn sheet_of(game: &GameEntry) -> Option<String> {
    std::fs::read_to_string(game.dir.join("CHEATSHEET.md")).ok()
}

/// The rows whose label starts with any of `prefixes` (case-insensitive).
fn rows_like(rows: &[(String, String)], prefixes: &[&str]) -> Vec<(String, String)> {
    rows.iter()
        .filter(|(label, _)| {
            let l = label.to_lowercase();
            prefixes.iter().any(|p| l.starts_with(p))
        })
        .cloned()
        .collect()
}

fn gate_line(game: &GameEntry) -> String {
    match &game.verify {
        Some(v) if v.pass => format!("passed its last gate ({}, {})", v.progress, v.when),
        Some(v) => format!(
            "FAILED its last gate: {}",
            v.failed.clone().unwrap_or_default()
        ),
        None => "never verified".to_string(),
    }
}

/// The three ways to start a game, as CLI and MCP calls.
fn run_lines(game: &GameEntry) -> Value {
    let s = game.slug.as_str();
    json!({
        "start": format!("gamecraft-cli start {s}"),
        "play": format!("gamecraft-cli play {s}"),
        "demo": format!("gamecraft-cli demo {s}"),
        "mcp": format!("game_play {{\"slug\": \"{s}\"}} or game_demo {{\"slug\": \"{s}\"}} (needs GAMECRAFT_MCP_ALLOW_LAUNCH=1)"),
    })
}

/// One game, described from its files.
#[must_use]
pub fn describe(game: &GameEntry) -> Value {
    let sheet = sheet_of(game);
    let rows = sheet.as_deref().map(sheet_rows).unwrap_or_default();
    let goal = rows_like(&rows, &["goal", "objective"]);
    let controls = rows_like(&rows, &["controls", "play", "move", "reveal", "click"]);
    let playable = catalog::playable(game);
    json!({
        "slug": game.slug,
        "title": game.title,
        "kind": game.kind.as_str(),
        "group": game.group,
        "description": game.description,
        "playable": playable.is_ok(),
        "not_playable_reason": playable.err(),
        "goal": goal.iter().map(|(l, v)| format!("{l}: {v}")).collect::<Vec<_>>(),
        "controls": controls.iter().map(|(l, v)| format!("{l}: {v}")).collect::<Vec<_>>(),
        "has_cheatsheet": sheet.is_some(),
        "gate": gate_line(game),
        "run": run_lines(game),
        "dir": game.dir,
    })
}

/// The description as readable text.
#[must_use]
pub fn describe_text(game: &GameEntry) -> String {
    let d = describe(game);
    let text = |k: &str| d.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let list = |k: &str| -> Vec<String> {
        d.get(k)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut out = format!("{} ({}, {})\n", text("title"), text("kind"), text("group"));
    if !game.description.is_empty() {
        let _ = write!(out, "\n{}\n", game.description);
    }
    if let Some(reason) = d.get("not_playable_reason").and_then(Value::as_str) {
        let _ = write!(out, "\nNOT PLAYABLE: {reason}\n");
    }
    for (title, key) in [("Goal", "goal"), ("Controls", "controls")] {
        let lines = list(key);
        if !lines.is_empty() {
            let _ = write!(out, "\n{title}\n");
            for line in lines {
                let _ = writeln!(out, "  {line}");
            }
        }
    }
    let _ = write!(out, "\nLast gate: {}\n", text("gate"));
    if d.get("playable").and_then(Value::as_bool) == Some(true) {
        let _ = write!(
            out,
            "\nStart:  gamecraft-cli play {s}\nWatch:  gamecraft-cli demo {s}\nMore:   gamecraft-cli help {s}\n",
            s = game.slug
        );
    }
    out
}

const OVERVIEW: &str =
    "GameCraft: Godot 4, three.js and Bevy games, one catalog, one CLI, one MCP server.

Find a game
  gamecraft-cli list [--filter calm]      every game with its last gate result
  gamecraft-cli search <words>            same, as a plain search
  gamecraft-cli random [--filter web]     pick one for me
  gamecraft-cli describe <slug>           what it is, goal, controls, how to start it
  gamecraft-cli help <slug>               describe plus the full cheat sheet

Run a game
  gamecraft-cli play <slug>               interactive: browser tab (web) or its own window
  gamecraft-cli demo <slug>               the game plays itself (autoplay)
  gamecraft-cli start <slug>              bring it up and print where (web: URL, no browser)
  gamecraft-cli running                   games and the arcade server started from here
  gamecraft-cli stop [slug] [--server]    stop them

Check
  gamecraft-cli verify <slug>             run the gate now
  gamecraft-cli doctor [--strict]         catalog and tool health, report only

Help topics: overview, games, start, play, demo, running, stop, doctor, mcp, or any game slug.";

const TOPIC_START: &str = "start / play / demo

  play   interactive. Web games open in your default browser; Godot and Bevy games open
         their own window.
  demo   the game plays itself: web gets ?autoplay=1, Godot gets GAMECRAFT_AUTOPLAY=1,
         Bevy gets --autoplay.
  start  brings the game up and prints where it is. For a web game that means the arcade
         server is running and the URL is printed, with no browser opened. A native game
         has no such middle step, so start and play are the same for it.

Web games are served by `bun run serve.ts` on 127.0.0.1:11258. If nothing answers there it
is started for you, and `gamecraft-cli stop --server` stops it again. A browser tab is not a
process this tool owns: close the tab to end a web game.

A Bevy game must be built first; when it is not, the command prints the exact cargo line
(or pass --build to run it). Godot needs `godot` on PATH or GAMECRAFT_GODOT.";

const TOPIC_RUNNING: &str = "running / stop

`running` lists native games (Godot, Bevy) started by this tool that are still alive, and
whether the web arcade answers. `stop <slug>` stops one native game, `stop` stops all of
them, `stop --server` also stops an arcade server this tool started. The record lives in
target/gamecraft/running.json and dead processes are dropped when it is read.";

const TOPIC_DOCTOR: &str = "doctor

Report-only health check: games that cannot be started (data or plans without code), games
without a cheat sheet or description, games that never passed a gate or fail it now, plans
with no game yet, and whether bun, node, cargo and godot are installed. It changes nothing.
Exit code is 0 unless --strict is given and anything beyond plans and info was found.";

const TOPIC_MCP: &str = "MCP server (gamecraft-mcp)

Read: game_list, game_describe, game_help, game_report, game_running, game_doctor.
Run:  game_start, game_play, game_demo, game_stop. These start processes, so they are off
      until the server is started with GAMECRAFT_MCP_ALLOW_LAUNCH=1.
Check: game_verify (needs GAMECRAFT_MCP_ALLOW_WEB / _GODOT / _BEVY = 1).
Games started by the MCP server end when the server exits.
Every cheat sheet is also served as the resource gamecraft://games/<kind>/<slug>/cheatsheet.";

fn games_topic(games: &[GameEntry]) -> String {
    let mut out = String::new();
    for kind in GameKind::ALL {
        let mine: Vec<&GameEntry> = games.iter().filter(|g| g.kind == kind).collect();
        if mine.is_empty() {
            continue;
        }
        let _ = writeln!(out, "{} ({})", kind.label(), mine.len());
        for g in mine {
            let first = g
                .description
                .split(". ")
                .next()
                .unwrap_or("")
                .trim_end_matches('.');
            let short: String = first.chars().take(90).collect();
            let _ = writeln!(out, "  {:<20} {}", g.slug, short);
        }
    }
    out
}

/// Help text for a topic or a game slug. `None` is the overview.
///
/// # Errors
///
/// Returns a message with the topics and close slugs when nothing matches.
pub fn help_text(games: &[GameEntry], topic: Option<&str>) -> Result<String, String> {
    let Some(raw) = topic.map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(OVERVIEW.to_string());
    };
    match raw.to_lowercase().as_str() {
        "overview" | "help" => Ok(OVERVIEW.to_string()),
        "games" | "list" => Ok(games_topic(games)),
        "start" | "play" | "demo" => Ok(TOPIC_START.to_string()),
        "running" | "stop" => Ok(TOPIC_RUNNING.to_string()),
        "doctor" => Ok(TOPIC_DOCTOR.to_string()),
        "mcp" => Ok(TOPIC_MCP.to_string()),
        _ => {
            if let Some(game) = catalog::find(games, raw, None) {
                let mut out = describe_text(game);
                if let Some(sheet) = sheet_of(game) {
                    let _ = write!(out, "\nCheat sheet\n{}\n", sheet.trim_end());
                } else {
                    out.push_str("\n(no cheat sheet for this game)\n");
                }
                Ok(out)
            } else {
                let close = catalog::suggest(games, raw);
                let hint = if close.is_empty() {
                    String::new()
                } else {
                    format!(" Did you mean: {}?", close.join(", "))
                };
                Err(format!(
                    "no help topic or game '{raw}'.{hint} Topics: overview, games, start, play, demo, running, stop, doctor, mcp, or a game slug."
                ))
            }
        }
    }
}

/// One thing `doctor` noticed.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Finding {
    /// Game slug, or `(env)` for a tool.
    pub slug: String,
    /// `fail`, `warn`, `plan` or `info`.
    pub level: &'static str,
    /// What is wrong, or the version found.
    pub issue: String,
}

fn finding(slug: &str, level: &'static str, issue: impl Into<String>) -> Finding {
    Finding {
        slug: slug.to_string(),
        level,
        issue: issue.into(),
    }
}

/// First line of `<program> --version`, or `None` when it cannot be run.
fn tool_version(program: &str) -> Option<String> {
    let out = std::process::Command::new(program)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next().unwrap_or("").trim();
    (out.status.success() && !line.is_empty()).then(|| line.to_string())
}

/// Check the catalog and the tools. Reads files and runs `--version` on four programs;
/// changes nothing.
#[must_use]
pub fn doctor(root: &Path, games: &[GameEntry]) -> Vec<Finding> {
    let mut out = Vec::new();
    for g in games {
        let slug = g.slug.as_str();
        if let Err(reason) = catalog::playable(g) {
            out.push(finding(
                slug,
                "fail",
                format!("cannot be started: {reason}"),
            ));
            continue;
        }
        match &g.verify {
            None => out.push(finding(slug, "warn", "never verified (no gate record)")),
            Some(v) if !v.pass => out.push(finding(
                slug,
                "fail",
                format!(
                    "fails its last gate: {}",
                    v.failed.clone().unwrap_or_default()
                ),
            )),
            Some(_) => {}
        }
        if !g.dir.join("CHEATSHEET.md").is_file() {
            out.push(finding(slug, "warn", "no CHEATSHEET.md"));
        }
        if g.description.trim().is_empty() {
            out.push(finding(
                slug,
                "warn",
                "no description (game.json experiment or README)",
            ));
        }
        if g.kind == GameKind::Web && !g.dir.join("thumb.png").is_file() {
            out.push(finding(slug, "warn", "no thumb.png"));
        }
    }
    for slug in catalog::plan_only(root) {
        out.push(finding(
            slug.as_str(),
            "plan",
            "planned, not built (only game.plan.json)",
        ));
    }
    let godot = std::env::var("GAMECRAFT_GODOT").unwrap_or_else(|_| "godot".to_string());
    for (label, program) in [
        ("bun", launch::bun_binary()),
        ("node", "node".to_string()),
        ("cargo", "cargo".to_string()),
        ("godot", godot),
    ] {
        match tool_version(program.as_str()) {
            Some(version) => out.push(finding("(env)", "info", format!("{label}: {version}"))),
            None => out.push(finding(
                "(env)",
                "warn",
                format!("{label} not found ({program}); games that need it cannot start or verify"),
            )),
        }
    }
    out
}

/// Counts of `(fail, warn, plan)` findings.
#[must_use]
pub fn doctor_counts(findings: &[Finding]) -> (usize, usize, usize) {
    let n = |level: &str| findings.iter().filter(|f| f.level == level).count();
    (n("fail"), n("warn"), n("plan"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("gamecraft-guide-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn fixture(root: &Path) {
        let ok = root.join("web").join("games").join("tiles");
        let data = root.join("web").join("games").join("case");
        let plan = root.join("web").join("games").join("jenga");
        for d in [&ok, &data, &plan] {
            assert!(std::fs::create_dir_all(d).is_ok());
        }
        assert!(std::fs::write(
            ok.join("game.json"),
            r#"{"title":"Tiles","category":"Calm puzzles","experiment":"Slide tiles. Win by order."}"#
        )
        .is_ok());
        assert!(std::fs::write(ok.join("index.html"), "<html></html>").is_ok());
        assert!(std::fs::write(
            ok.join("CHEATSHEET.md"),
            "# Tiles\n\n| What | Value |\n|---|---|\n| Goal | Order the tiles |\n| Controls | Click a tile |\n| Gate | headless |\n"
        )
        .is_ok());
        assert!(std::fs::write(data.join("game.json"), r#"{"title":"Case"}"#).is_ok());
        assert!(std::fs::write(plan.join("game.plan.json"), "{}").is_ok());
    }

    #[test]
    fn sheet_rows_skip_header_and_separator() {
        let rows =
            sheet_rows("# T\n\n| What | Value |\n|---|---|\n| Goal | Win |\n| Gate | x | y |\n");
        assert_eq!(
            rows,
            vec![
                ("Goal".to_string(), "Win".to_string()),
                ("Gate".to_string(), "x".to_string())
            ]
        );
    }

    #[test]
    fn describe_takes_goal_and_controls_from_the_sheet() {
        let root = temp_root("describe");
        fixture(root.as_path());
        let games = catalog::discover(root.as_path());
        let tiles = catalog::find(&games, "tiles", None);
        let text = tiles.map(describe_text).unwrap_or_default();
        assert!(text.contains("Goal: Order the tiles"), "{text}");
        assert!(text.contains("Controls: Click a tile"), "{text}");
        assert!(text.contains("gamecraft-cli demo tiles"), "{text}");
        let case = catalog::find(&games, "case", None)
            .map(describe_text)
            .unwrap_or_default();
        assert!(case.contains("NOT PLAYABLE"), "{case}");
        assert!(!case.contains("gamecraft-cli play case"), "{case}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn help_knows_topics_games_and_close_misses() {
        let root = temp_root("help");
        fixture(root.as_path());
        let games = catalog::discover(root.as_path());
        assert!(help_text(&games, None).is_ok_and(|t| t.contains("gamecraft-cli doctor")));
        assert!(help_text(&games, Some("games"))
            .is_ok_and(|t| t.contains("tiles") && t.contains("Slide tiles")));
        assert!(help_text(&games, Some("tiles"))
            .is_ok_and(|t| t.contains("Cheat sheet") && t.contains("Order the tiles")));
        let miss = help_text(&games, Some("tile")).err().unwrap_or_default();
        assert!(miss.contains("Did you mean: tiles"), "{miss}");
        let bad = help_text(&games, Some("zzz")).err().unwrap_or_default();
        assert!(bad.contains("Topics:"), "{bad}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn doctor_separates_broken_from_planned() {
        let root = temp_root("doctor");
        fixture(root.as_path());
        let games = catalog::discover(root.as_path());
        let found = doctor(root.as_path(), &games);
        let has =
            |slug: &str, level: &str| found.iter().any(|f| f.slug == slug && f.level == level);
        assert!(has("case", "fail"), "{found:?}");
        assert!(has("jenga", "plan"), "{found:?}");
        assert!(
            has("tiles", "warn"),
            "never verified and no thumb: {found:?}"
        );
        assert!(!found
            .iter()
            .any(|f| f.slug == "tiles" && f.issue.contains("CHEATSHEET")));
        assert!(found.iter().any(|f| f.slug == "(env)"));
        let (fail, _, plan) = doctor_counts(&found);
        assert!(fail >= 1 && plan == 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}

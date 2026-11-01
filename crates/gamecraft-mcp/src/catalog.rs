//! The game catalog: every playable project in one list, shared by the MCP
//! server, the CLI and the desktop app so all three say the same thing.
//!
//! Two runtimes live side by side: forge-generated Godot projects under
//! `depot/games/<slug>/` (marker `main.tscn`) and model-written three.js pages
//! under `web/games/<slug>/` (marker `game.json`). Each entry carries its group
//! for the sidebar, a one-line description, and the last verification result
//! read from the files the gates write (`trace.json` + `lint.json`, `verify.json`).

use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Runtime of a catalog entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GameKind {
    /// Godot 4 project under `depot/games/`.
    Godot,
    /// three.js page under `web/games/`.
    Web,
}

impl GameKind {
    /// Lowercase name used in JSON and filters (`godot`, `web`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Godot => "godot",
            Self::Web => "web",
        }
    }
}

/// Sidebar groups for Godot games, in display order.
pub const GODOT_GROUPS: [&str; 4] = ["Arcade", "Action", "Puzzles", "Worlds"];
/// Sidebar groups for web games, in display order (from `category` in `game.json`).
pub const WEB_GROUPS: [&str; 3] = ["Seek and find", "Calm puzzles", "Worlds and sims"];

/// Last verification result, as recorded on disk by the gates.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VerifySummary {
    /// Did the game pass its gate?
    pub pass: bool,
    /// What failed, when it did not.
    pub failed: Option<String>,
    /// Checkpoints reached on the tour (web) or frames to the win (Godot).
    pub progress: String,
    /// When the gate ran: ISO date from `verify.json` (web) or unix seconds of `trace.json` (Godot).
    pub when: String,
    /// Self-test verdict and summary, for web games that have one.
    pub self_test: Option<String>,
}

/// One playable project.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GameEntry {
    /// Runtime.
    pub kind: GameKind,
    /// Folder name.
    pub slug: String,
    /// Display title.
    pub title: String,
    /// Sidebar group.
    pub group: String,
    /// One-line description (`experiment` in `game.json`, or the README's first paragraph).
    pub description: String,
    /// Project directory.
    pub dir: PathBuf,
    /// Last gate result, if the gate has ever run.
    pub verify: Option<VerifySummary>,
}

/// The depot is forge-generated and never hand-edited, so Godot groups live here.
#[must_use]
pub fn godot_group(slug: &str) -> &'static str {
    match slug {
        "breakout" | "flappy" | "pong" | "runner" | "topdown-shooter" => "Arcade",
        "platformer" | "driving" | "flight" => "Action",
        "maze" | "sokoban" | "chessboard-move" | "tower-place" => "Puzzles",
        "night-forest" | "snowy-mountain-pass" | "talk-inventory" => "Worlds",
        _ => "Other",
    }
}

/// Find the repository root: an explicit path, then `GAMECRAFT_ROOT`, then walking
/// up from the working directory and from the executable until a folder holds
/// `depot/games` or `web/games`.
#[must_use]
pub fn find_root(explicit: Option<&str>) -> Option<PathBuf> {
    let has_games = |dir: &Path| {
        dir.join("depot").join("games").is_dir() || dir.join("web").join("games").is_dir()
    };
    if let Some(path) = explicit.filter(|p| !p.trim().is_empty()) {
        let dir = PathBuf::from(path);
        return has_games(dir.as_path()).then_some(dir);
    }
    if let Ok(env) = std::env::var("GAMECRAFT_ROOT") {
        let dir = PathBuf::from(env);
        if has_games(dir.as_path()) {
            return Some(dir);
        }
    }
    let starts = [
        std::env::current_dir().ok(),
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf)),
    ];
    for start in starts.into_iter().flatten() {
        let mut cursor = start;
        for _ in 0..7 {
            if has_games(cursor.as_path()) {
                return Some(cursor);
            }
            if !cursor.pop() {
                break;
            }
        }
    }
    None
}

fn read_json(path: &Path) -> Option<Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(text.as_str()).ok())
}
fn str_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

/// First paragraph of a README that is not a heading, trimmed to one line.
fn readme_line(dir: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(dir.join("README.md")) else {
        return String::new();
    };
    let mut para = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.starts_with('|') || line.starts_with("```") {
            if para.is_empty() {
                continue;
            }
            break;
        }
        if line.is_empty() {
            if para.is_empty() {
                continue;
            }
            break;
        }
        if !para.is_empty() {
            para.push(' ');
        }
        para.push_str(line);
    }
    para
}

fn web_verify_summary(dir: &Path) -> Option<VerifySummary> {
    let v = read_json(dir.join("verify.json").as_path())?;
    let checkpoints = v
        .get("checkpoints")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let self_test = v.get("self_test").filter(|s| !s.is_null()).map(|s| {
        let pass = s.get("pass").and_then(Value::as_bool).unwrap_or(false);
        let summary = str_field(s, "summary").unwrap_or_default();
        format!(
            "{}{}",
            if pass { "pass" } else { "FAIL" },
            if summary.is_empty() {
                String::new()
            } else {
                format!(": {summary}")
            }
        )
    });
    Some(VerifySummary {
        pass: v.get("pass").and_then(Value::as_bool).unwrap_or(false),
        failed: str_field(&v, "failed"),
        progress: format!("{checkpoints} checkpoints"),
        when: str_field(&v, "date").unwrap_or_default(),
        self_test,
    })
}

fn godot_verify_summary(dir: &Path) -> Option<VerifySummary> {
    let trace_path = dir.join("trace.json");
    let trace = read_json(trace_path.as_path())?;
    let won = trace.get("win").and_then(Value::as_bool).unwrap_or(false);
    let findings = read_json(dir.join("lint.json").as_path())
        .and_then(|l| l.as_array().map(Vec::len))
        .unwrap_or(0);
    let frames = trace.get("frames").and_then(Value::as_u64).unwrap_or(0);
    let when = std::fs::metadata(&trace_path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or_else(String::new, |d| d.as_secs().to_string());
    let failed = if !won {
        Some("Rust trace replay did not reach the goal".to_string())
    } else if findings > 0 {
        Some(format!("{findings} lint findings"))
    } else {
        None
    };
    Some(VerifySummary {
        pass: won && findings == 0,
        failed,
        progress: format!("won in {frames} frames"),
        when,
        self_test: None,
    })
}

fn collect(root: &Path, kind: GameKind, out: &mut Vec<GameEntry>) {
    let (base, marker) = match kind {
        GameKind::Godot => (root.join("depot").join("games"), "main.tscn"),
        GameKind::Web => (root.join("web").join("games"), "game.json"),
    };
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.join(marker).is_file() {
            continue;
        }
        let Some(slug) = dir.file_name().and_then(|n| n.to_str()).map(str::to_string) else {
            continue;
        };
        let pretty = slug.replace('-', " ");
        let game = match kind {
            GameKind::Godot => GameEntry {
                kind,
                title: pretty,
                group: godot_group(slug.as_str()).to_string(),
                description: readme_line(dir.as_path()),
                verify: godot_verify_summary(dir.as_path()),
                slug,
                dir,
            },
            GameKind::Web => {
                let meta = read_json(dir.join("game.json").as_path()).unwrap_or(Value::Null);
                GameEntry {
                    kind,
                    title: str_field(&meta, "title").unwrap_or(pretty),
                    group: str_field(&meta, "category")
                        .unwrap_or_else(|| "More three.js".to_string()),
                    description: str_field(&meta, "experiment")
                        .unwrap_or_else(|| readme_line(dir.as_path())),
                    verify: web_verify_summary(dir.as_path()),
                    slug,
                    dir,
                }
            }
        };
        out.push(game);
    }
}

/// Every game under `root`, sorted by title.
#[must_use]
pub fn discover(root: &Path) -> Vec<GameEntry> {
    let mut games = Vec::new();
    collect(root, GameKind::Godot, &mut games);
    collect(root, GameKind::Web, &mut games);
    games.sort_by_key(|g| g.title.to_lowercase());
    games
}

/// Does a game match a free-text filter (title, slug, group or kind; any case)?
#[must_use]
pub fn matches(game: &GameEntry, filter: &str) -> bool {
    let filter = filter.trim().to_lowercase();
    filter.is_empty()
        || [
            game.title.as_str(),
            game.slug.as_str(),
            game.group.as_str(),
            game.kind.as_str(),
        ]
        .iter()
        .any(|text| text.to_lowercase().contains(filter.as_str()))
}

/// Find one game by slug. With the same slug in both runtimes, `kind` decides
/// (default: the web game, which is the newer family).
#[must_use]
pub fn find<'a>(
    games: &'a [GameEntry],
    slug: &str,
    kind: Option<GameKind>,
) -> Option<&'a GameEntry> {
    let hits: Vec<&GameEntry> = games
        .iter()
        .filter(|g| g.slug == slug && kind.is_none_or(|k| g.kind == k))
        .collect();
    hits.iter()
        .find(|g| g.kind == GameKind::Web)
        .or_else(|| hits.first())
        .copied()
}

/// Close slugs, for "did you mean" answers.
#[must_use]
pub fn suggest(games: &[GameEntry], slug: &str) -> Vec<String> {
    let needle = slug.to_lowercase();
    let mut out: Vec<String> = games
        .iter()
        .filter(|g| {
            let s = g.slug.to_lowercase();
            s.contains(needle.as_str())
                || needle.contains(s.as_str())
                || g.title.to_lowercase().contains(needle.as_str())
        })
        .map(|g| g.slug.clone())
        .collect();
    out.truncate(5);
    out
}

/// Everything known about one game: the entry, its player cheat sheet, its README
/// and the full last gate record.
#[must_use]
pub fn report(game: &GameEntry) -> Value {
    let text = |name: &str| std::fs::read_to_string(game.dir.join(name)).ok();
    let gate_record = match game.kind {
        GameKind::Web => read_json(game.dir.join("verify.json").as_path()),
        GameKind::Godot => Some(json!({
            "trace": read_json(game.dir.join("trace.json").as_path()),
            "lint": read_json(game.dir.join("lint.json").as_path()),
        })),
    };
    let shots: Vec<String> = std::fs::read_dir(game.dir.join("shots"))
        .map(|d| {
            d.flatten()
                .filter_map(|e| e.file_name().to_str().map(str::to_string))
                .filter(|n| {
                    Path::new(n)
                        .extension()
                        .is_some_and(|x| x.eq_ignore_ascii_case("png"))
                })
                .collect()
        })
        .unwrap_or_default();
    json!({
        "game": game,
        "cheatsheet": text("CHEATSHEET.md"),
        "readme": text("README.md"),
        "gate": gate_record,
        "shots": shots,
        "thumb": game.dir.join("thumb.png").is_file().then(|| game.dir.join("thumb.png")),
    })
}

/// Win markers any Godot runtime here prints.
pub const WIN_MARKERS: [&str; 4] = [
    "goal reached",
    "puzzle solved",
    "door opened",
    "all hostiles down",
];

/// Boot a Godot project headless and look for a win marker.
///
/// Launches the `GAMECRAFT_GODOT` binary (default `godot`); the engine is killed
/// after `timeout_s` and whenever the calling process exits.
///
/// # Errors
///
/// Returns a reason when the engine cannot be launched.
pub fn godot_verify(
    game_path: &Path,
    frames: u64,
    autoplay: bool,
    timeout_s: Option<u64>,
) -> Result<Value, String> {
    let timeout_s = timeout_s.unwrap_or((frames / 60) * 2 + 60);
    let binary = std::env::var("GAMECRAFT_GODOT").unwrap_or_else(|_| "godot".to_string());
    let mut command = std::process::Command::new(binary.as_str());
    command
        .arg("--headless")
        .arg("--path")
        .arg(game_path)
        .arg("--fixed-fps")
        .arg("60")
        .arg("--quit-after")
        .arg(frames.to_string());
    if autoplay {
        command.env("GAMECRAFT_AUTOPLAY", "1");
    }
    let output = crate::proc_guard::run_with_timeout(
        &mut command,
        std::time::Duration::from_secs(timeout_s),
    )
    .map_err(|err| format!("cannot launch {binary}: {err}"))?;
    let found: Vec<&str> = WIN_MARKERS
        .iter()
        .copied()
        .filter(|m| output.stdout.contains(m) || output.stderr.contains(m))
        .collect();
    let errors: Vec<&str> = output
        .stderr
        .lines()
        .filter(|l| l.contains("ERROR"))
        .take(5)
        .collect();
    let tail: String = output
        .stdout
        .chars()
        .rev()
        .take(2000)
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect();
    Ok(json!({
        "game_path": game_path,
        "won": !found.is_empty(),
        "markers": found,
        "frames_budget": frames,
        "elapsed_ms": output.elapsed.as_millis(),
        "exit_code": output.status.and_then(|s| s.code()),
        "timed_out": output.timed_out,
        "timeout_s": timeout_s,
        "engine_errors": errors,
        "tail": tail,
    }))
}

/// Run the web gate for one game (`node scripts/verify.ts <slug>` in `web/`) and
/// return its fresh `verify.json`. Bun must be installed (the gate serves the
/// pages with it); `%USERPROFILE%\.bun\bin` is put on PATH for the child.
///
/// # Errors
///
/// Returns a reason when node cannot be launched, the run times out, or no
/// `verify.json` appears.
pub fn web_verify(root: &Path, slug: &str, timeout_s: u64) -> Result<Value, String> {
    let web = root.join("web");
    let mut command = std::process::Command::new(if cfg!(windows) { "node.exe" } else { "node" });
    command
        .current_dir(&web)
        .arg("--no-deprecation")
        .arg("scripts/verify.ts")
        .arg(slug);
    if let Ok(home) = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) {
        let bun = PathBuf::from(home).join(".bun").join("bin");
        let path = std::env::var_os("PATH").unwrap_or_default();
        let mut parts = vec![bun];
        parts.extend(std::env::split_paths(&path));
        if let Ok(joined) = std::env::join_paths(parts) {
            command.env("PATH", joined);
        }
    }
    let output = crate::proc_guard::run_with_timeout(
        &mut command,
        std::time::Duration::from_secs(timeout_s),
    )
    .map_err(|err| format!("cannot launch node: {err}"))?;
    if output.timed_out {
        return Err(format!(
            "the web gate for {slug} did not finish within {timeout_s} s"
        ));
    }
    let verify =
        read_json(web.join("games").join(slug).join("verify.json").as_path()).ok_or_else(|| {
            format!(
                "no verify.json after the run; gate output: {}",
                output.stdout.lines().last().unwrap_or("")
            )
        })?;
    Ok(
        json!({ "slug": slug, "elapsed_ms": output.elapsed.as_millis(), "verify": verify, "gate_output": output.stdout.lines().rfind(|l| !l.trim().is_empty()) }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("gamecraft-catalog-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn discovers_both_runtimes_with_groups_and_status() {
        let root = temp_root("both");
        let g = root.join("depot").join("games").join("maze");
        let w = root.join("web").join("games").join("kintsugi");
        assert!(std::fs::create_dir_all(&g).is_ok() && std::fs::create_dir_all(&w).is_ok());
        assert!(std::fs::write(g.join("main.tscn"), "[gd_scene]").is_ok());
        assert!(std::fs::write(g.join("trace.json"), r#"{"win":true,"frames":129}"#).is_ok());
        assert!(std::fs::write(g.join("lint.json"), "[]").is_ok());
        assert!(std::fs::write(
            w.join("game.json"),
            r#"{"title":"Kintsugi","category":"Calm puzzles","experiment":"Mend bowls."}"#
        )
        .is_ok());
        assert!(std::fs::write(w.join("verify.json"), r#"{"pass":true,"date":"2026-11-01","checkpoints":[{},{}],"self_test":{"pass":true,"summary":"ok"}}"#).is_ok());
        let games = discover(root.as_path());
        assert_eq!(games.len(), 2);
        let k = find(&games, "kintsugi", None);
        assert!(k.is_some_and(|k| k.group == "Calm puzzles"
            && k.title == "Kintsugi"
            && k.verify
                .as_ref()
                .is_some_and(|v| v.pass && v.progress == "2 checkpoints")));
        let m = find(&games, "maze", Some(GameKind::Godot));
        assert!(
            m.is_some_and(|m| m.group == "Puzzles" && m.verify.as_ref().is_some_and(|v| v.pass))
        );
        assert_eq!(games.iter().filter(|g| matches(g, "calm")).count(), 1);
        assert_eq!(suggest(&games, "kints"), vec!["kintsugi".to_string()]);
        assert!(find_root(Some(root.to_string_lossy().as_ref())).is_some());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_gate_files_mean_no_status() {
        let root = temp_root("bare");
        let w = root.join("web").join("games").join("bare");
        assert!(std::fs::create_dir_all(&w).is_ok());
        assert!(std::fs::write(w.join("game.json"), "{}").is_ok());
        let games = discover(root.as_path());
        assert!(games.first().is_some_and(|g| g.verify.is_none()
            && g.title == "bare"
            && g.group == "More three.js"));
        let _ = std::fs::remove_dir_all(&root);
    }
}

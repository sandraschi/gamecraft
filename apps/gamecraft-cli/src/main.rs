//! `gamecraft-cli`: scene tools (`lint`, `query`, `uid`, `export`), game making
//! (`forge`, `new`) and the game catalog (`list`, `report`, `verify`), the same
//! catalog the MCP server and the desktop app read.

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

/// `GameCraft` scene verification CLI.
#[derive(Debug, Parser)]
#[command(
    name = "gamecraft-cli",
    version,
    about = "Make, list, verify and inspect GameCraft games (Godot 4 and three.js)"
)]
struct Cli {
    /// Subcommand to run.
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands.
#[derive(Debug, Subcommand)]
enum Commands {
    /// Lint a scene file; prints diagnostics as JSON.
    Lint {
        /// Scene file to lint.
        #[arg(long)]
        scene: PathBuf,
    },
    /// World-space AABB of one node subtree; prints JSON.
    Query {
        /// Scene file to query.
        #[arg(long)]
        scene: PathBuf,
        /// Scene path from the root (`.` = root).
        #[arg(long)]
        node: String,
    },
    /// Deterministic `uid://...` for a relative scene path.
    Uid {
        /// Relative scene path (e.g. `res://scenes/world.tscn`).
        #[arg(long)]
        path: String,
    },
    /// Forge depot games (deterministic scenes + verified traces).
    Forge {
        /// Single game slug (omit with `--all`).
        #[arg(long)]
        game: Option<String>,
        /// Output root (`depot/` lands here).
        #[arg(long, default_value = ".")]
        out: PathBuf,
        /// Forge every game.
        #[arg(long)]
        all: bool,
    },
    /// Make a NEW game: an `LLM` designs a spec, `GameCraft` compiles and verifies it.
    New {
        /// What the game should feel like (e.g. "night forest, hard jumps").
        #[arg(long)]
        prompt: Option<String>,
        /// Skip the model: compile this `GameSpec` JSON file directly.
        #[arg(long = "from-spec")]
        from_spec: Option<PathBuf>,
        /// Provider: auto | local | openai | anthropic. uto probes local servers.
        #[arg(long, default_value = "auto")]
        provider: String,
        /// Model id (provider default when omitted).
        #[arg(long)]
        model: Option<String>,
        /// Base URL override (LM Studio, a proxy, a remote Ollama).
        #[arg(long)]
        endpoint: Option<String>,
        /// Output root; the game lands in <out>/depot/games/<slug>/.
        #[arg(long, default_value = ".")]
        out: PathBuf,
    },
    /// List every game (Godot and three.js) with group and last gate result.
    List {
        /// Free-text filter over title, slug, group and kind.
        #[arg(long, default_value = "")]
        filter: String,
        /// Print JSON instead of a table.
        #[arg(long)]
        json: bool,
    },
    /// One game in detail: description, gate record, cheat sheet.
    Report {
        /// Game slug (folder name), e.g. `where-is-benny`.
        slug: String,
        /// Print JSON instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Run one game's gate now (web: headless self-play; Godot: headless boot to a win marker).
    Verify {
        /// Game slug (folder name).
        slug: String,
        /// Wall-clock limit in seconds.
        #[arg(long, default_value_t = 300)]
        timeout: u64,
        /// Print JSON instead of one line.
        #[arg(long)]
        json: bool,
    },
    /// Convert a Godot scene to glTF 2.0 (`.glb`, or `.gltf` with the buffer embedded).
    Gltf {
        /// Scene file, e.g. `depot/games/platformer/main.tscn`. Omit with `--depot-out`.
        input: Option<PathBuf>,
        /// Output file; `.gltf` writes text, anything else binary. Default: input with `.glb`.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Export every depot game's `main.tscn` into this folder as `<slug>.glb`.
        #[arg(long)]
        depot_out: Option<PathBuf>,
    },
    /// Export a JSON scene spec to engine text (same engine as the MCP tool).
    Export {
        /// Input scene spec (JSON).
        #[arg(long = "in")]
        input: PathBuf,
        /// Output file; prints to stdout when omitted.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Target format.
        #[arg(long, default_value = "godot4")]
        format: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Lint { scene } => cmd_lint(scene.as_path()),
        Commands::Query { scene, node } => cmd_query(scene.as_path(), node.as_str()),
        Commands::Uid { path } => {
            println!("{}", gamecraft_godot::deterministic_uid(path.as_str()));
        }
        Commands::Forge { game, out, all } => cmd_forge(game, out.as_path(), all),
        Commands::New {
            prompt,
            from_spec,
            provider,
            model,
            endpoint,
            out,
        } => cmd_new(
            prompt,
            from_spec,
            provider.as_str(),
            model.as_deref(),
            endpoint.as_deref(),
            out.as_path(),
        ),
        Commands::Export { input, out, format } => {
            cmd_export(input.as_path(), out.as_deref(), format.as_str());
        }
        Commands::Gltf {
            input,
            out,
            depot_out,
        } => match (input, depot_out) {
            (_, Some(dir)) => cmd_gltf_depot(dir.as_path()),
            (Some(input), None) => {
                let target = out.unwrap_or_else(|| input.with_extension("glb"));
                if let Err(reason) = write_gltf(input.as_path(), target.as_path()) {
                    eprintln!("{reason}");
                    std::process::exit(1);
                }
            }
            (None, None) => {
                eprintln!("give a scene file, or --depot-out <dir> for every depot game");
                std::process::exit(2);
            }
        },
        Commands::List { filter, json } => cmd_list(filter.as_str(), json),
        Commands::Report { slug, json } => cmd_report(slug.as_str(), json),
        Commands::Verify {
            slug,
            timeout,
            json,
        } => cmd_verify(slug.as_str(), timeout, json),
    }
}

use gamecraft_mcp::catalog::{self, GameEntry, GameKind};

/// Convert one scene file and print what went in.
fn write_gltf(input: &Path, target: &Path) -> Result<(), String> {
    let gltf =
        gamecraft_gltf::from_tscn_file(input).map_err(|e| format!("{}: {e}", input.display()))?;
    let text_form = target
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gltf"));
    let bytes = if text_form {
        gltf.to_gltf_embedded().into_bytes()
    } else {
        gltf.to_glb()?
    };
    std::fs::write(target, &bytes)
        .map_err(|e| format!("cannot write {}: {e}", target.display()))?;
    let s = gltf.stats;
    println!(
        "{}: {} nodes, {} meshes, {} materials, {} triangles, {} lights, {} cameras, {} bytes",
        target.display(),
        s.nodes,
        s.meshes,
        s.materials,
        s.triangles,
        s.lights,
        s.cameras,
        bytes.len()
    );
    if s.textures > 0 || s.textures_skipped > 0 {
        let hint = if s.textures_skipped > 0 {
            " (only triplanar materials export textures)"
        } else {
            ""
        };
        println!(
            "  textures: {} embedded, {} not exported{hint}",
            s.textures, s.textures_skipped
        );
    }
    Ok(())
}

/// Every Godot depot game to `<dir>/<slug>.glb`; exits 1 if any fails.
fn cmd_gltf_depot(dir: &Path) {
    let root = root_or_exit();
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("cannot create {}: {e}", dir.display());
        std::process::exit(1);
    }
    let mut failed = 0_usize;
    for game in catalog::discover(root.as_path())
        .iter()
        .filter(|g| g.kind == GameKind::Godot)
    {
        let target = dir.join(format!("{}.glb", game.slug));
        if let Err(reason) = write_gltf(game.dir.join("main.tscn").as_path(), target.as_path()) {
            eprintln!("{reason}");
            failed += 1;
        }
    }
    if failed > 0 {
        eprintln!("{failed} scene(s) failed");
        std::process::exit(1);
    }
}

/// The repo root or a clear exit.
fn root_or_exit() -> PathBuf {
    catalog::find_root(None).unwrap_or_else(|| {
        eprintln!("no GameCraft repo here: run inside the repo or set GAMECRAFT_ROOT");
        std::process::exit(1);
    })
}

/// One game by slug, or exit with suggestions.
fn game_or_exit(slug: &str) -> (PathBuf, GameEntry) {
    let root = root_or_exit();
    let games = catalog::discover(root.as_path());
    if let Some(game) = catalog::find(&games, slug, None) {
        return (root, game.clone());
    }
    let close = catalog::suggest(&games, slug);
    if close.is_empty() {
        eprintln!("no game '{slug}'; `gamecraft-cli list` shows them all");
    } else {
        eprintln!("no game '{slug}'; did you mean: {}?", close.join(", "));
    }
    std::process::exit(1);
}

fn status_word(game: &GameEntry) -> String {
    match &game.verify {
        Some(v) if v.pass => format!("pass  {}", v.progress),
        Some(v) => format!("FAIL  {}", v.failed.clone().unwrap_or_default()),
        None => "never verified".to_string(),
    }
}

/// List entry: a grouped table, or JSON.
fn cmd_list(filter: &str, json: bool) {
    let root = root_or_exit();
    let games: Vec<GameEntry> = catalog::discover(root.as_path())
        .into_iter()
        .filter(|g| catalog::matches(g, filter))
        .collect();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&games).unwrap_or_else(|_| "[]".to_string())
        );
        return;
    }
    for (kind, label, groups) in [
        (GameKind::Godot, "Godot", &catalog::GODOT_GROUPS[..]),
        (GameKind::Web, "three.js", &catalog::WEB_GROUPS[..]),
    ] {
        let mine: Vec<&GameEntry> = games.iter().filter(|g| g.kind == kind).collect();
        if mine.is_empty() {
            continue;
        }
        println!("{label} ({})", mine.len());
        let mut order: Vec<String> = groups.iter().map(|g| (*g).to_string()).collect();
        for g in &mine {
            if !order.contains(&g.group) {
                order.push(g.group.clone());
            }
        }
        for group in order {
            let members: Vec<&&GameEntry> = mine.iter().filter(|g| g.group == group).collect();
            if members.is_empty() {
                continue;
            }
            println!("  {group}");
            for g in members {
                println!("    {:<22} {:<26} {}", g.slug, g.title, status_word(g));
            }
        }
    }
}

/// Report entry: readable summary, or the full JSON report.
fn cmd_report(slug: &str, json: bool) {
    let (_, game) = game_or_exit(slug);
    let report = catalog::report(&game);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
        return;
    }
    println!("{} ({}, {})", game.title, game.kind.as_str(), game.group);
    println!("{}", game.dir.display());
    if !game.description.is_empty() {
        println!("\n{}", game.description);
    }
    println!("\nlast gate: {}", status_word(&game));
    if let Some(v) = &game.verify {
        if !v.when.is_empty() {
            println!("when:      {}", v.when);
        }
        if let Some(st) = &v.self_test {
            println!("self-test: {st}");
        }
    }
    if let Some(sheet) = report.get("cheatsheet").and_then(serde_json::Value::as_str) {
        println!("\n{sheet}");
    }
}

/// Verify entry: runs the gate, prints one line (or JSON), exits 1 on failure.
fn cmd_verify(slug: &str, timeout: u64, json: bool) {
    let (root, game) = game_or_exit(slug);
    let run = match game.kind {
        GameKind::Web => catalog::web_verify(root.as_path(), game.slug.as_str(), timeout),
        GameKind::Godot => catalog::godot_verify(game.dir.as_path(), 3600, true, Some(timeout)),
    };
    match run {
        Ok(value) => {
            let pass = value
                .pointer("/verify/pass")
                .or_else(|| value.get("won"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if json {
                println!("{value}");
            } else {
                let why = value
                    .pointer("/verify/failed")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("no win marker");
                println!(
                    "{}: {}",
                    game.slug,
                    if pass {
                        "PASS".to_string()
                    } else {
                        format!("FAIL ({why})")
                    }
                );
            }
            if !pass {
                std::process::exit(1);
            }
        }
        Err(reason) => {
            eprintln!("{}: could not run the gate: {reason}", game.slug);
            std::process::exit(1);
        }
    }
}

/// Read a scene file; `None` after reporting the IO error.
fn read_scene(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(err) => {
            eprintln!("cannot read {}: {err}", path.display());
            None
        }
    }
}

/// Lint entry: JSON diagnostics on stdout, nothing on stderr unless IO fails.
fn cmd_lint(path: &Path) {
    if let Some(text) = read_scene(path) {
        let diags = gamecraft_mcp::scene_lint(text.as_str());
        println!(
            "{}",
            serde_json::to_string(&diags).unwrap_or_else(|_| "[]".to_string())
        );
    } else {
        std::process::exit(1);
    }
}

/// Export entry: JSON scene spec in, engine text out.
fn cmd_export(input: &Path, out: Option<&Path>, format: &str) {
    let text = match std::fs::read_to_string(input) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("cannot read {}: {err}", input.display());
            std::process::exit(1);
        }
    };
    let spec: gamecraft_mcp::SceneSpec = match serde_json::from_str(text.as_str()) {
        Ok(spec) => spec,
        Err(err) => {
            eprintln!("invalid scene spec in {}: {err}", input.display());
            std::process::exit(1);
        }
    };
    match gamecraft_mcp::export_spec(&spec, format) {
        Ok(outcome) => match out {
            Some(path) => {
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        if let Err(err) = std::fs::create_dir_all(parent) {
                            eprintln!("cannot create {}: {err}", parent.display());
                            std::process::exit(1);
                        }
                    }
                }
                if let Err(err) = std::fs::write(path, outcome.text.as_str()) {
                    eprintln!("cannot write {}: {err}", path.display());
                    std::process::exit(1);
                }
                println!(
                    "wrote {} ({} nodes, {} bytes, uid {})",
                    path.display(),
                    outcome.nodes,
                    outcome.text.len(),
                    outcome.uid
                );
            }
            None => println!("{}", outcome.text),
        },
        Err(reason) => {
            eprintln!("export failed: {reason}");
            std::process::exit(1);
        }
    }
}

/// New-game entry: spec file, or prompt -> model -> spec -> compile.
fn cmd_new(
    prompt: Option<String>,
    from_spec: Option<PathBuf>,
    provider: &str,
    model: Option<&str>,
    endpoint: Option<&str>,
    out: &Path,
) {
    if let Some(path) = from_spec {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("cannot read {}: {err}", path.display());
                std::process::exit(1);
            }
        };
        let spec: gamecraft_forge::create::GameSpec = match serde_json::from_str(text.as_str()) {
            Ok(spec) => spec,
            Err(err) => {
                eprintln!("invalid spec in {}: {err}", path.display());
                std::process::exit(1);
            }
        };
        match gamecraft_forge::create::create_game(&spec, out) {
            Ok(summary) => println!("{summary}"),
            Err(reason) => {
                eprintln!("generation failed: {reason}");
                std::process::exit(1);
            }
        }
        return;
    }
    let Some(idea) = prompt else {
        eprintln!("pass --prompt \"...\" or --from-spec <file.json>");
        std::process::exit(1);
    };
    // Flags win; the config file fills what the flags leave out, so a key entered
    // once in the viewer Settings works for `new` without retyping.
    let file_config = gamecraft_core::config::AppConfig::load();
    let provider_name = if provider == "auto" && file_config.provider != "auto" {
        file_config.provider.clone()
    } else {
        provider.to_string()
    };
    let model_name: Option<String> = model
        .map(str::to_string)
        .or_else(|| file_config.effective_model().map(str::to_string));
    let endpoint_name: Option<String> = endpoint.map(str::to_string).or_else(|| {
        (!file_config.ollama_endpoint.trim().is_empty())
            .then(|| file_config.ollama_endpoint.clone())
    });
    let discovered = if provider_name == "auto" {
        gamecraft_forge::llm::auto(model_name.as_deref())
    } else {
        gamecraft_forge::llm::LlmConfig::new(provider_name.as_str(), model_name.as_deref())
    };
    let mut config = match discovered {
        Ok(config) => config,
        Err(reason) => {
            eprintln!("{reason}");
            std::process::exit(1);
        }
    };
    if let Some(url) = endpoint_name.as_deref() {
        config = config.at(url);
    }
    match gamecraft_forge::create::generate(idea.as_str(), &config, out) {
        Ok(summary) => println!("{summary}"),
        Err(reason) => {
            eprintln!("generation failed: {reason}");
            std::process::exit(1);
        }
    }
}

/// Forge entry: builds games, prints one summary line each.
fn cmd_forge(game: Option<String>, out: &Path, all: bool) {
    if all || game.is_none() {
        match gamecraft_forge::forge_all(out) {
            Ok(summaries) => {
                for summary in summaries {
                    println!("{summary}");
                }
            }
            Err(reason) => {
                eprintln!("forge failed: {reason}");
                std::process::exit(1);
            }
        }
        return;
    }
    if let Some(slug) = game {
        match gamecraft_forge::forge_one(slug.as_str(), out) {
            Ok(summary) => println!("{summary}"),
            Err(reason) => {
                eprintln!("forge failed: {reason}");
                std::process::exit(1);
            }
        }
    }
}

/// Query entry: JSON box on stdout, or the reason on stderr + exit 1.
fn cmd_query(path: &Path, node: &str) {
    if let Some(text) = read_scene(path) {
        match gamecraft_mcp::spatial_query(text.as_str(), node) {
            Ok(value) => println!("{value}"),
            Err(reason) => {
                eprintln!("query failed: {reason}");
                std::process::exit(1);
            }
        }
    } else {
        std::process::exit(1);
    }
}

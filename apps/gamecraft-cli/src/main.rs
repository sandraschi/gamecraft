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
    about = "Find, play, demo, verify and make GameCraft games (Godot 4, three.js, Bevy)",
    disable_help_subcommand = true,
    after_help = "`gamecraft-cli help` explains the game commands; `gamecraft-cli help <slug>` shows one game's cheat sheet."
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
        /// Provider: auto | local | openai | anthropic. auto probes local servers.
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
        /// Texture catalog folder: one subfolder per set holding `*albedo*`, `*orm*`
        /// and `*normal*` images (a `TextureCraft` Godot export per folder). Specs may
        /// then name `theme.surfaces.<role>.set`.
        #[arg(long)]
        textures: Option<PathBuf>,
    },
    /// List every game (Godot, three.js and Bevy) with group and last gate result.
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
    /// Plan (default) or run a TRELLIS.2 image-to-3D sidecar request; writes the licence ledger.
    Trellis {
        /// Source image the model reconstructs from.
        #[arg(long)]
        image: PathBuf,
        /// Asset slug (ASCII, `-`/`_` ok); GLB lands as `<out>/<slug>.glb`.
        #[arg(long)]
        slug: String,
        /// Style reference for the ledger only (TRELLIS.2 takes an image, not text).
        #[arg(long, default_value = "")]
        prompt_ref: String,
        /// Triangle budget the import gate enforces.
        #[arg(long, default_value_t = 20_000)]
        poly_budget: u64,
        /// Really invoke the sidecar (default is plan-only with a 60 s timeout).
        #[arg(long)]
        execute: bool,
        /// Output root for the GLB + `.ledger.json`.
        #[arg(long, default_value = "assets/trellis")]
        out: PathBuf,
    },
    /// Provision the TRELLIS.2 sidecar (clone, conda env, weights, driver).
    /// Plan-only by default; `--execute` runs each missing step once.
    TrellisSetup {
        /// Really run the setup steps (default is plan-only).
        #[arg(long)]
        execute: bool,
        /// Skip the ~8 GB weight download (env + driver only).
        #[arg(long)]
        skip_weights: bool,
        /// Re-run steps whose markers already exist.
        #[arg(long)]
        force: bool,
    },
    /// List every command in the registry (the whole API; same list the MCP server serves).
    #[command(name = "commands")]
    CommandList {
        /// Print JSON: id, label, menu, shortcut, params, enabled.
        #[arg(long)]
        json: bool,
    },
    /// Describe a game (goal, controls, how to start it) or, for an id like
    /// `replay.verify`, one registry command and its parameters.
    Describe {
        /// Game slug, or a registry command id.
        id: String,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Help for the game commands; `help <slug>` is one game's description and cheat sheet.
    Help {
        /// A topic (overview, games, start, play, demo, running, stop, doctor, mcp) or a game slug.
        topic: Option<String>,
    },
    /// Search games by words in the title, slug, group or kind.
    Search {
        /// Words to look for.
        #[arg(required = true)]
        words: Vec<String>,
        /// Print JSON instead of a table.
        #[arg(long)]
        json: bool,
    },
    /// Pick a playable game at random.
    Random {
        /// Free-text filter, as for `list`.
        #[arg(long, default_value = "")]
        filter: String,
        /// Start it as well (play mode).
        #[arg(long)]
        play: bool,
    },
    /// Bring a game up and say where it is (web: the arcade server and the URL, no browser).
    Start {
        /// Game slug.
        slug: String,
        /// Let the game play itself.
        #[arg(long)]
        demo: bool,
        /// Bevy only: build the game first when it is not built.
        #[arg(long)]
        build: bool,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Play a game: web games open in your browser, others in their own window.
    Play {
        /// Game slug.
        slug: String,
        /// Bevy only: build the game first when it is not built.
        #[arg(long)]
        build: bool,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Watch a game play itself (autoplay).
    Demo {
        /// Game slug.
        slug: String,
        /// Bevy only: build the game first when it is not built.
        #[arg(long)]
        build: bool,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Native games started from here that are still running, and the web arcade state.
    Running {
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Stop native games started from here (one slug, or all).
    Stop {
        /// Game to stop; omit for all.
        slug: Option<String>,
        /// Also stop the web arcade server when this tool started it.
        #[arg(long)]
        server: bool,
    },
    /// Check the catalog and the tools (report only). Exit 1 with `--strict` on any failure or warning.
    Doctor {
        /// Exit non-zero when anything is failing or warned about.
        #[arg(long)]
        strict: bool,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Run a registry command: `run --cmd 'replay.verify={"path":"maze.gcraft"}'`.
    Run {
        /// `id` or `id={json params}`.
        #[arg(long)]
        cmd: String,
        /// Print the result as JSON (failures too, with a non-zero exit code).
        #[arg(long)]
        json: bool,
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
            textures,
        } => cmd_new(
            prompt,
            from_spec,
            provider.as_str(),
            model.as_deref(),
            endpoint.as_deref(),
            out.as_path(),
            textures.as_deref(),
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
        Commands::Trellis {
            image,
            slug,
            prompt_ref,
            poly_budget,
            execute,
            out,
        } => cmd_trellis(
            image.as_path(),
            slug.as_str(),
            prompt_ref.as_str(),
            poly_budget,
            execute,
            out.as_path(),
        ),
        Commands::TrellisSetup {
            execute,
            skip_weights,
            force,
        } => cmd_trellis_setup(execute, !skip_weights, force),
        Commands::CommandList { json } => emit(&gamecraft_mcp::registry::cli_commands(json)),
        Commands::Describe { id, json } => cmd_describe(id.as_str(), json),
        Commands::Help { topic } => cmd_help(topic.as_deref()),
        Commands::Search { words, json } => cmd_list(words.join(" ").as_str(), json),
        Commands::Random { filter, play } => cmd_random(filter.as_str(), play),
        Commands::Start {
            slug,
            demo,
            build,
            json,
        } => cmd_start(slug.as_str(), demo, build, json),
        Commands::Play { slug, build, json } => {
            cmd_launch(slug.as_str(), Mode::Play, true, build, json);
        }
        Commands::Demo { slug, build, json } => {
            cmd_launch(slug.as_str(), Mode::Demo, true, build, json);
        }
        Commands::Running { json } => cmd_running(json),
        Commands::Stop { slug, server } => cmd_stop(slug.as_deref(), server),
        Commands::Doctor { strict, json } => cmd_doctor(strict, json),
        Commands::Run { cmd, json } => emit(&gamecraft_mcp::registry::cli_run(&cmd, json)),
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
use gamecraft_mcp::launch::{self, Mode};

/// `describe <slug>` describes a game; anything else is looked up as a registry command id.
fn cmd_describe(id: &str, json: bool) {
    let games = catalog::find_root(None)
        .map(|root| catalog::discover(root.as_path()))
        .unwrap_or_default();
    if let Some(game) = catalog::find(&games, id, None) {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&gamecraft_mcp::guide::describe(game))
                    .unwrap_or_default()
            );
        } else {
            print!("{}", gamecraft_mcp::guide::describe_text(game));
        }
        return;
    }
    emit(&gamecraft_mcp::registry::cli_describe(id, json));
}

/// `help [topic]`: generated from the repo; works without a repo for the overview.
fn cmd_help(topic: Option<&str>) {
    let games = catalog::find_root(None)
        .map(|root| catalog::discover(root.as_path()))
        .unwrap_or_default();
    match gamecraft_mcp::guide::help_text(&games, topic) {
        Ok(text) => println!("{}", text.trim_end()),
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    }
}

/// A playable game picked by the clock (no random-number crate needed for this).
fn cmd_random(filter: &str, play: bool) {
    let root = root_or_exit();
    let pool: Vec<GameEntry> = catalog::discover(root.as_path())
        .into_iter()
        .filter(|g| catalog::matches(g, filter) && catalog::playable(g).is_ok())
        .collect();
    if pool.is_empty() {
        eprintln!("no playable game matches '{filter}'");
        std::process::exit(1);
    }
    let tick = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() as usize);
    let game = &pool[tick % pool.len()];
    print!("{}", gamecraft_mcp::guide::describe_text(game));
    if play {
        println!();
        cmd_launch(game.slug.as_str(), Mode::Play, true, false, false);
    }
}

/// start: bring the game up without opening a browser.
fn cmd_start(slug: &str, demo: bool, build: bool, json: bool) {
    let mode = if demo { Mode::Demo } else { Mode::Play };
    cmd_launch(slug, mode, false, build, json);
}

/// start / play / demo.
fn cmd_launch(slug: &str, mode: Mode, open: bool, build: bool, json: bool) {
    let (root, game) = game_or_exit(slug);
    if build && game.kind == GameKind::Bevy && catalog::bevy_launch(&game, root.as_path()).is_err()
    {
        eprintln!(
            "building {} (a cold optimised build can take 15 minutes)...",
            game.slug
        );
        if let Err(reason) = catalog::bevy_build(&game, root.as_path()) {
            eprintln!("{}: {reason}", game.slug);
            std::process::exit(1);
        }
    }
    let godot = gamecraft_core::config::AppConfig::load().godot_binary;
    let godot = (!godot.trim().is_empty()).then_some(godot);
    match launch::launch(&game, root.as_path(), mode, open, godot.as_deref()) {
        Ok(done) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&done).unwrap_or_default()
                );
            } else {
                println!("{}: {}", game.slug, done.note);
                if done.server_started.is_some() {
                    println!("(started the web arcade; `gamecraft-cli stop --server` stops it)");
                }
            }
        }
        Err(reason) => {
            eprintln!("{}: {reason}", game.slug);
            std::process::exit(1);
        }
    }
}

/// running: native games started from here, and whether the arcade answers.
fn cmd_running(json: bool) {
    let root = root_or_exit();
    let now = launch::running(root.as_path());
    if json {
        println!("{}", serde_json::to_string_pretty(&now).unwrap_or_default());
        return;
    }
    if now.games.is_empty() {
        println!("no native game started from here is running");
    }
    for p in &now.games {
        println!("{:<22} {:<6} {:<5} pid {}", p.slug, p.kind, p.mode, p.pid);
    }
    println!(
        "web arcade: {}{}",
        if now.arcade_up {
            "up on port 11258"
        } else {
            "not running"
        },
        now.arcade_pid.map_or_else(String::new, |pid| format!(
            " (started from here, pid {pid})"
        ))
    );
}

/// stop: one native game, or all.
fn cmd_stop(slug: Option<&str>, server: bool) {
    let root = root_or_exit();
    let stopped = launch::stop(root.as_path(), slug, server);
    if stopped.is_empty() {
        eprintln!(
            "{}",
            slug.map_or_else(
                || "nothing to stop: no native game started from here is running".to_string(),
                |s| format!("'{s}' was not started from here, or has already ended (web games: close the browser tab)"),
            )
        );
        std::process::exit(1);
    }
    let mut failed = false;
    for s in stopped {
        println!(
            "{} {}",
            if s.ok { "stopped" } else { "could not stop" },
            s.what
        );
        failed |= !s.ok;
    }
    if failed {
        std::process::exit(1);
    }
}

/// doctor: report only; `--strict` turns failures and warnings into exit code 1.
fn cmd_doctor(strict: bool, json: bool) {
    let root = root_or_exit();
    let games = catalog::discover(root.as_path());
    let findings = gamecraft_mcp::guide::doctor(root.as_path(), &games);
    let (fail, warn, plan) = gamecraft_mcp::guide::doctor_counts(&findings);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "games": games.len(), "failing": fail, "warnings": warn, "planned": plan, "findings": findings
            }))
            .unwrap_or_default()
        );
    } else {
        for (level, title) in [
            ("fail", "FAILING"),
            ("warn", "WARNINGS"),
            ("plan", "PLANNED, NOT BUILT"),
            ("info", "TOOLS"),
        ] {
            let mine: Vec<_> = findings.iter().filter(|f| f.level == level).collect();
            if mine.is_empty() {
                continue;
            }
            println!("{title}");
            for f in mine {
                println!("  {:<22} {}", f.slug, f.issue);
            }
        }
        println!(
            "\n{} games: {fail} failing, {warn} warnings, {plan} planned only",
            games.len()
        );
    }
    if strict && fail + warn > 0 {
        std::process::exit(1);
    }
}

/// Print a registry result and exit with its code.
fn emit(out: &gamecraft_mcp::registry::CliOutput) {
    if !out.stdout.is_empty() {
        println!("{}", out.stdout.trim_end());
    }
    if !out.stderr.is_empty() {
        eprintln!("{}", out.stderr.trim_end());
    }
    if out.code != 0 {
        std::process::exit(out.code);
    }
}

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
    for kind in GameKind::ALL {
        let (label, groups) = (kind.label(), kind.groups());
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
        GameKind::Bevy => catalog::bevy_verify(root.as_path(), game.slug.as_str(), timeout),
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
    textures: Option<&Path>,
) {
    if let Some(path) = from_spec {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("cannot read {}: {err}", path.display());
                std::process::exit(1);
            }
        };
        let mut spec: gamecraft_forge::create::GameSpec = match serde_json::from_str(text.as_str())
        {
            Ok(spec) => spec,
            Err(err) => {
                eprintln!("invalid spec in {}: {err}", path.display());
                std::process::exit(1);
            }
        };
        if let Some(catalog) = textures {
            if let Err(reason) = gamecraft_forge::create::resolve_sets(&mut spec, catalog) {
                eprintln!("generation failed: {reason}");
                std::process::exit(1);
            }
        }
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
    match gamecraft_forge::create::generate_with_textures(idea.as_str(), &config, out, textures) {
        Ok(summary) => println!("{summary}"),
        Err(reason) => {
            eprintln!("generation failed: {reason}");
            std::process::exit(1);
        }
    }
}

/// TRELLIS.2 entry: plan-only by default; `--execute` runs the sidecar once.
///
/// The sidecar is operator-owned (a TRELLIS.2 checkout or HTTP server). This
/// never bundles weights: it writes the ledger row first, then — only with
/// `--execute` — spawns one bounded process and kills it on timeout.
fn cmd_trellis(
    image: &Path,
    slug: &str,
    prompt_ref: &str,
    poly_budget: u64,
    execute: bool,
    out: &Path,
) {
    let mut config = gamecraft_trellis::TrellisConfig::from_env();
    config.out_dir = out.to_path_buf();
    let request = gamecraft_trellis::TrellisRequest {
        slug: slug.to_string(),
        image: image.to_path_buf(),
        prompt_ref: prompt_ref.to_string(),
        poly_budget,
        extent_limit_m: 8.0,
    };
    let plan = match gamecraft_trellis::plan(&config, &request) {
        Ok(plan) => plan,
        Err(reason) => {
            eprintln!("trellis plan failed: {reason}");
            std::process::exit(1);
        }
    };
    match gamecraft_trellis::probe(&config) {
        Ok(note) => println!("{note}"),
        Err(reason) => {
            eprintln!("trellis sidecar missing: {reason}");
            std::process::exit(1);
        }
    }
    println!(
        "plan: {} -> {}",
        plan.argv.join(" "),
        plan.out_glb.display()
    );
    println!("licence: {}", plan.ledger.license);
    if !execute {
        println!("dry run: pass --execute to invoke the sidecar (60 s timeout)");
        return;
    }
    if config.backend != gamecraft_trellis::Backend::Cli {
        eprintln!("--execute currently supports the CLI backend only; HTTP stays plan-only");
        std::process::exit(2);
    }
    let mut child = match std::process::Command::new(&plan.argv[0])
        .args(&plan.argv[1..])
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            eprintln!("could not spawn sidecar: {err}");
            std::process::exit(1);
        }
    };
    let deadline = std::time::Duration::from_secs(60);
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    eprintln!("sidecar exited with {status}");
                    std::process::exit(1);
                }
                break;
            }
            Ok(None) => {
                if start.elapsed() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    eprintln!("sidecar timed out after 60 s and was killed");
                    std::process::exit(1);
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            Err(err) => {
                eprintln!("sidecar wait failed: {err}");
                std::process::exit(1);
            }
        }
    }
    match gamecraft_trellis::write_ledger(&plan.out_glb, &plan.ledger) {
        Ok(path) => println!("ledger: {}", path.display()),
        Err(reason) => {
            eprintln!("could not write ledger: {reason}");
            std::process::exit(1);
        }
    }
}

/// TRELLIS.2 setup entry: plan-only by default; `--execute` provisions.
///
/// Long steps (conda env, weight download) run one at a time with a 30 min
/// cap each and are killed on expiry. Steps whose marker exists are skipped
/// unless `--force`. Never leaves a half-cloned repo behind silently: a
/// failed clone removes the partial dir so the next run retries cleanly.
fn cmd_trellis_setup(execute: bool, with_weights: bool, force: bool) {
    let config = gamecraft_trellis::TrellisConfig::from_env();
    let steps = match gamecraft_trellis::plan_setup(&config, with_weights) {
        Ok(steps) => steps,
        Err(reason) => {
            eprintln!("trellis setup plan failed: {reason}");
            std::process::exit(1);
        }
    };
    println!("sidecar root: {}", config.repo_dir.display());
    for step in &steps {
        let skipped = !force && step.skip_if_exists.as_ref().is_some_and(|m| m.exists());
        println!(
            "{} {}: {}",
            if skipped {
                "skip "
            } else if execute {
                "run  "
            } else {
                "plan "
            },
            step.label,
            step.argv.join(" ")
        );
    }
    if !execute {
        println!("dry run: pass --execute to provision (long steps, 30 min cap each)");
        return;
    }
    for step in &steps {
        if !force && step.skip_if_exists.as_ref().is_some_and(|m| m.exists()) {
            continue;
        }
        if step.argv == ["write-driver"] {
            match gamecraft_trellis::write_driver(&config) {
                Ok(path) => println!("wrote {}", path.display()),
                Err(reason) => {
                    eprintln!("could not write driver: {reason}");
                    std::process::exit(1);
                }
            }
            continue;
        }
        let mut child = match std::process::Command::new(&step.argv[0])
            .args(&step.argv[1..])
            .current_dir(step.cwd.clone().unwrap_or_else(|| config.repo_dir.clone()))
            .spawn()
        {
            Ok(child) => child,
            Err(err) => {
                eprintln!("could not spawn '{}': {err}", step.label);
                std::process::exit(1);
            }
        };
        let deadline = std::time::Duration::from_mins(30);
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        if step.label.starts_with("clone") {
                            let _ = std::fs::remove_dir_all(&config.repo_dir);
                        }
                        eprintln!("step '{}' exited with {status}", step.label);
                        std::process::exit(1);
                    }
                    break;
                }
                Ok(None) => {
                    if start.elapsed() > deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        eprintln!(
                            "step '{}' timed out after 30 min and was killed",
                            step.label
                        );
                        std::process::exit(1);
                    }
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
                Err(err) => {
                    eprintln!("step '{}' wait failed: {err}", step.label);
                    std::process::exit(1);
                }
            }
        }
    }
    println!(
        "trellis setup complete; driver at {}",
        config.repo_dir.join("trellis_driver.py").display()
    );
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

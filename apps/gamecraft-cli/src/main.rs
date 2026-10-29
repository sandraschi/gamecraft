//! `gamecraft-cli`: `lint` / `query` / `uid` over Godot scene files.

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

/// `GameCraft` scene verification CLI.
#[derive(Debug, Parser)]
#[command(
    name = "gamecraft-cli",
    version,
    about = "Lint and query Godot 4 scenes"
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
    let discovered = if provider == "auto" {
        gamecraft_forge::llm::auto(model)
    } else {
        gamecraft_forge::llm::LlmConfig::new(provider, model)
    };
    let mut config = match discovered {
        Ok(config) => config,
        Err(reason) => {
            eprintln!("{reason}");
            std::process::exit(1);
        }
    };
    if let Some(url) = endpoint {
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

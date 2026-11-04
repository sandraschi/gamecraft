//! Desktop shell state: live graph, command stack, camera, trace ring.
//!
//! Every scene mutation flows through `gamecraft-commands`; camera, trace,
//! selection, and palette text are view state and never touch the graph.

use gamecraft_commands::{AddNodeCommand, CommandStack, SetTransformCommand};
use gamecraft_core::{spatial, NodeId, SceneGraph, SignalRegistry, Transform};
use gamecraft_godot::{
    build_graph, build_registry, deterministic_uid, export_tscn, parse_tscn, ConnectionDecl,
    ExportRequest, SceneFile,
};
use gamecraft_mcp::{lint_live, scene_lint, LintDiagnostic, ERR_MISSING_COLLISION_SHAPE};
use glam::Vec3;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// Fixture loaded when the shell starts.
const FIXTURE_PATH: &str = "depot/games/platformer/main.tscn";

/// Minimal scene when the fixture file is missing.
const FALLBACK_SCENE: &str =
    "[gd_scene load_steps=2 format=3]\n\n[node name=\"Root\" type=\"Node3D\"]\n";

/// Frames kept in the trace ring.
const TRACE_CAP: usize = 60;

/// Amber accent for the on-metacraft theme.
pub(crate) const ACCENT: egui::Color32 = egui::Color32::from_rgb(255, 190, 60);

/// One recorded frame: camera pose plus what the shell showed.
#[derive(Debug, Clone)]
pub(crate) struct TraceFrame {
    /// Camera yaw at record time.
    pub(crate) yaw: f32,
    /// Camera pitch at record time.
    pub(crate) pitch: f32,
    /// Selected node path at record time (`-` = none).
    pub(crate) selected: String,
    /// Finding count at record time.
    pub(crate) findings: usize,
    /// World position of the selection at record time, if any.
    pub(crate) ghost: Option<Vec3>,
}

/// Full shell state.
#[derive(Debug)]
// Independent view toggles (overlays, trace recording) - not a state machine,
// which is what this lint is actually looking for. Splitting them into a struct
// would just move the same four bools one level down.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct DesktopState {
    /// Live scene graph (mutated only via [`CommandStack`]).
    pub(crate) graph: SceneGraph,
    /// Signal methods, fixed at load (scripts are static).
    pub(crate) registry: SignalRegistry,
    /// Undo/redo over every scene mutation.
    pub(crate) stack: CommandStack,
    /// Live [`lint_live`] output, refreshed after each mutation.
    pub(crate) diags: Vec<LintDiagnostic>,
    /// Node paths currently failing the collision-shape check.
    pub(crate) bare_paths: HashSet<String>,
    /// Inspector selection.
    pub(crate) selected: Option<NodeId>,
    /// Command-palette filter text.
    pub(crate) palette_query: String,
    /// Highlighted palette match (Up/Down cycle, Enter runs).
    pub(crate) palette_index: usize,
    /// Orbit camera yaw.
    pub(crate) cam_yaw: f32,
    /// Orbit camera pitch.
    pub(crate) cam_pitch: f32,
    /// Orbit camera distance.
    pub(crate) cam_dist: f32,
    /// Orbit camera target.
    pub(crate) cam_target: Vec3,
    /// Rolling trace ring (max [`TRACE_CAP`] frames).
    pub(crate) trace: VecDeque<TraceFrame>,
    /// Whether each frame is recorded.
    pub(crate) recording: bool,
    /// Whether the scrubber follows the newest frame.
    pub(crate) follow: bool,
    /// Scrub position into [`trace`](DesktopState::trace).
    pub(crate) scrub_step: u32,
    /// Counter for unique palette-created node names.
    pub(crate) counter: u32,
    /// Last action status for the top bar.
    pub(crate) status: String,
    /// The Godot game this viewer started last. Starting another game closes it first,
    /// so repeated Start presses (or a test driver mashing the pad's A button) cannot
    /// pile up engine windows; all of them also die with the viewer (Job Object, see
    /// `gamecraft_mcp::proc_guard`).
    pub(crate) game_child: Option<std::process::Child>,
    /// First `wgpu` adapter label, probed once at startup.
    pub(crate) adapter_label: String,
    /// Scene-root name, used to relativize lint paths.
    pub(crate) root_name: String,
    /// Scene `uid` (header tag, else deterministic hash) shown in the top bar.
    pub(crate) scene_uid: String,
    /// Path the scene was loaded from (`None` = bundled fixture).
    pub(crate) scene_path: Option<PathBuf>,
    /// Parsed source, kept so a save can restore ext/sub resources verbatim.
    pub(crate) source: Option<SceneFile>,
    /// Albedo textures for the loaded scene.
    pub(crate) tex_cache: crate::textures::TexCache,
    /// Draw ground axes.
    pub(crate) show_axes: bool,
    /// Draw shaded mesh geometry built from the scene's own primitives.
    pub(crate) show_meshes: bool,
    /// Draw per-node AABB boxes.
    pub(crate) show_boxes: bool,
    /// Tint boxes by collision state.
    pub(crate) show_collision: bool,
    /// Draw the view frustum wireframe.
    pub(crate) show_frustum: bool,
    /// Playable projects found under `depot/games/`.
    pub(crate) depot_games: Vec<DepotGame>,
    /// Gamepad context; polled once per frame by the app shell.
    pub(crate) gamepad: crate::gamepad::Gamepad,
    /// Highlighted depot entry.
    pub(crate) selected_game: Option<usize>,
    /// Open modal, if any.
    pub(crate) dialog: Option<Dialog>,
    /// LLM provider / key configuration, edited through App settings.
    pub(crate) app_cfg: gamecraft_core::config::AppConfig,
    /// Play overlay for the selected depot game, edited through Game settings.
    pub(crate) overlay: gamecraft_core::config::GameOverlay,
    /// Per-game preview renders, loaded once at startup (`thumb.png`).
    pub(crate) thumbs: HashMap<String, Thumb>,
    /// Text filter over the depot list (title, slug or group); empty shows all.
    pub(crate) depot_filter: String,
    /// Library (card grid plus detail card) instead of the scene editor.
    pub(crate) library: bool,
    /// Bumped by View > Reset layout; folded into panel ids so dragged sizes are forgotten.
    pub(crate) layout_gen: u32,
    /// Command palette (the registry in the GUI).
    pub(crate) palette: crate::palette::Palette,
    /// Light theme instead of dark.
    pub(crate) light: bool,
    /// Gate running in the background, if any.
    pub(crate) verify_job: Option<VerifyJob>,
    /// Whether authenticated via Basic Auth.
    pub(crate) is_authenticated: bool,
    /// Operator / username for Basic Auth.
    pub(crate) auth_username: String,
    /// Working input buffer for Basic Auth password.
    pub(crate) auth_password_input: String,
    /// Cheat sheet of the selected game: (slug, text), read once per selection.
    pub(crate) sheet: Option<(String, String)>,
    /// Scroll the selected card into view on the next frame (keyboard navigation).
    pub(crate) scroll_to_selected: bool,
}

/// Modal dialogs owned by the menu bar.
///
/// Open and Save use native file dialogs, so only About needs a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dialog {
    /// About / controls.
    About,
    /// Provider, model, keys, binaries.
    AppSettings,
    /// Per-game overlay: speed, gravity, jump, autoplay.
    GameOverlay,
    /// Basic Auth login dialog.
    BasicAuth,
}

pub(crate) use gamecraft_mcp::catalog::GameKind;

/// One playable project. This is the shared catalog entry, so the app, the CLI
/// (`gamecraft-cli list`) and the MCP server (`game_list`) agree on titles,
/// groups, descriptions and the last gate result.
pub(crate) type DepotGame = gamecraft_mcp::catalog::GameEntry;

/// A gate run on a background thread so the shell stays responsive.
#[derive(Debug)]
pub(crate) struct VerifyJob {
    /// Game being verified.
    pub(crate) slug: String,
    /// Delivers the gate result once.
    pub(crate) rx: std::sync::mpsc::Receiver<Result<serde_json::Value, String>>,
}

/// Darker amber for text and strokes on the light theme (the bright one fails contrast there).
pub(crate) const ACCENT_DARK: egui::Color32 = egui::Color32::from_rgb(168, 104, 0);

/// Apply the dark or light theme. Both use hairline (0.5 pt) low-contrast borders
/// so panels separate without boxing everything in.
pub(crate) fn apply_theme(ctx: &egui::Context, light: bool) {
    ctx.set_theme(if light {
        egui::Theme::Light
    } else {
        egui::Theme::Dark
    });
    let mut visuals = if light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    let line = if light {
        visuals.override_text_color = Some(egui::Color32::from_rgb(26, 28, 34));
        visuals.panel_fill = egui::Color32::from_rgb(246, 244, 239);
        visuals.window_fill = egui::Color32::from_rgb(252, 251, 248);
        visuals.extreme_bg_color = egui::Color32::from_rgb(255, 255, 255);
        visuals.faint_bg_color = egui::Color32::from_rgb(236, 233, 226);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, ACCENT_DARK);
        visuals.hyperlink_color = ACCENT_DARK;
        visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(214, 140, 0, 60);
        visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT_DARK);
        egui::Color32::from_rgb(214, 209, 198)
    } else {
        // Contrast: egui's defaults are mid-greys on near-black, which is hard to
        // read on a dark panel. Force a bright text colour and give each widget
        // state a clearly different stroke.
        visuals.override_text_color = Some(egui::Color32::from_rgb(234, 237, 244));
        visuals.panel_fill = egui::Color32::from_rgb(16, 18, 24);
        visuals.window_fill = egui::Color32::from_rgb(22, 25, 33);
        visuals.extreme_bg_color = egui::Color32::from_rgb(9, 11, 16);
        visuals.faint_bg_color = egui::Color32::from_rgb(30, 33, 42);
        visuals.widgets.noninteractive.fg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(214, 219, 229));
        visuals.widgets.inactive.fg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(226, 230, 238));
        visuals.widgets.hovered.fg_stroke =
            egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 255, 255));
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, ACCENT);
        visuals.hyperlink_color = ACCENT;
        visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(255, 190, 60, 45);
        visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);
        egui::Color32::from_rgb(46, 50, 62)
    };
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, line);
    visuals.window_stroke = egui::Stroke::new(0.5, line);
    ctx.set_visuals(visuals);
}

/// A loaded preview render. The GPU handle has no `Debug`, so this wrapper
/// reports dimensions instead - enough to diagnose a corrupt thumbnail.
#[derive(Clone)]
pub(crate) struct Thumb {
    /// GPU texture shown in the depot row.
    pub(crate) texture: egui::TextureHandle,
}

impl std::fmt::Debug for Thumb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Thumb")
            .field("size", &self.texture.size())
            .finish()
    }
}

/// Load each game's `thumb.png` once at startup. Games without one (the two
/// spec-generated entries) simply get a text-only row; a corrupt file never
/// blocks the shell.
fn load_thumbs(ctx: &egui::Context, games: &[DepotGame]) -> HashMap<String, Thumb> {
    let mut thumbs: HashMap<String, Thumb> = HashMap::new();
    for game in games {
        let Ok(bytes) = std::fs::read(game.dir.join("thumb.png")) else {
            continue;
        };
        let Ok(decoded) = image::load_from_memory(bytes.as_slice()) else {
            continue;
        };
        let rgba = decoded.to_rgba8();
        let size = [
            usize::try_from(rgba.width()).unwrap_or(0),
            usize::try_from(rgba.height()).unwrap_or(0),
        ];
        if size[0] == 0 || size[1] == 0 {
            continue;
        }
        let raw = rgba.into_raw();
        let texture = ctx.load_texture(
            format!("thumb-{}", game.slug),
            egui::ColorImage::from_rgba_unmultiplied(size, raw.as_slice()),
            egui::TextureOptions::LINEAR,
        );
        thumbs.insert(game.slug.clone(), Thumb { texture });
    }
    thumbs
}

impl DesktopState {
    /// Load the fixture (or the fallback scene) and probe the adapter.
    #[must_use]
    pub(crate) fn load(ctx: &eframe::CreationContext<'_>) -> Self {
        apply_theme(&ctx.egui_ctx, false);
        ctx.egui_ctx.all_styles_mut(|style| {
            use egui::{FontFamily, FontId, TextStyle};
            style
                .text_styles
                .insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
            style.text_styles.insert(
                TextStyle::Button,
                FontId::new(15.0, FontFamily::Proportional),
            );
            style.text_styles.insert(
                TextStyle::Small,
                FontId::new(12.5, FontFamily::Proportional),
            );
            style.text_styles.insert(
                TextStyle::Monospace,
                FontId::new(14.5, FontFamily::Monospace),
            );
        });
        // Resolve the bundled fixture by walking up from the working directory:
        // launched from Explorer the cwd is `target\debug`, not the repo root.
        let text = find_up(FIXTURE_PATH)
            .and_then(|path| std::fs::read_to_string(path).ok())
            .unwrap_or_else(|| FALLBACK_SCENE.to_string());
        let mut state = Self::from_text(text.as_str(), deterministic_uid(FIXTURE_PATH).as_str());
        state.thumbs = load_thumbs(&ctx.egui_ctx, state.depot_games.as_slice());
        if let Some(path) = find_up(FIXTURE_PATH) {
            state.scene_path = Some(path);
            state.status = format!("loaded {FIXTURE_PATH}");
        }
        state
    }

    /// Build shell state from scene text (test seam; no window needed).
    #[must_use]
    pub(crate) fn from_text(scene_text: &str, fallback_uid: &str) -> Self {
        let scene = parse_tscn(scene_text).ok();
        let graph = scene
            .as_ref()
            .and_then(|parsed| build_graph(parsed).ok())
            .unwrap_or_default();
        let registry = scene
            .as_ref()
            .map_or_else(SignalRegistry::new, |parsed| build_registry(parsed, &graph));
        let root_name = graph
            .root
            .and_then(|root| graph.get(root))
            .map_or_else(|| "Root".to_string(), |node| node.name.clone());
        let scene_uid = scene
            .as_ref()
            .and_then(|parsed| parsed.header.uid.clone())
            .unwrap_or_else(|| fallback_uid.to_string());
        let mut state = Self {
            graph,
            registry,
            stack: CommandStack::new(),
            diags: Vec::new(),
            bare_paths: HashSet::new(),
            selected: None,
            palette_query: String::new(),
            palette_index: 0,
            cam_yaw: 0.6,
            cam_pitch: 0.35,
            cam_dist: 12.0,
            cam_target: Vec3::new(0.0, 1.0, 0.0),
            trace: VecDeque::new(),
            recording: true,
            follow: true,
            scrub_step: 0,
            counter: 0,
            status: "loaded".to_string(),
            game_child: None,
            adapter_label: wgpu_adapter_label(),
            root_name,
            scene_uid,
            scene_path: None,
            source: scene,
            tex_cache: crate::textures::TexCache::default(),
            show_axes: true,
            show_meshes: true,
            show_boxes: false,
            show_collision: true,
            show_frustum: false,
            depot_games: discover_depot(),
            selected_game: None,
            gamepad: crate::gamepad::Gamepad::new(),
            dialog: None,
            app_cfg: gamecraft_core::config::AppConfig::load(),
            overlay: gamecraft_core::config::GameOverlay::default(),
            thumbs: HashMap::new(),
            depot_filter: String::new(),
            library: true,
            layout_gen: 0,
            palette: crate::palette::Palette::default(),
            light: false,
            verify_job: None,
            is_authenticated: false,
            auth_username: "gamecrafter".to_string(),
            auth_password_input: String::new(),
            sheet: None,
            scroll_to_selected: false,
        };
        state.selected = state.graph.root;
        state.refresh();
        state.frame_scene();
        state
    }

    /// Load a scene from disk, replacing graph, stack, and trace.
    pub(crate) fn load_from_path(&mut self, path: &Path) {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) => {
                self.status = format!("cannot read {}: {err}", path.display());
                return;
            }
        };
        let Ok(source) = parse_tscn(text.as_str()) else {
            self.status = format!("{} is not a parseable scene", path.display());
            return;
        };
        let graph = build_graph(&source).unwrap_or_default();
        let registry = build_registry(&source, &graph);
        let root_name = graph
            .root
            .and_then(|root| graph.get(root))
            .map_or_else(|| "Root".to_string(), |node| node.name.clone());
        let scene_uid = source
            .header
            .uid
            .clone()
            .unwrap_or_else(|| deterministic_uid(path.to_string_lossy().as_ref()));
        self.graph = graph;
        self.registry = registry;
        self.source = Some(source);
        self.stack = CommandStack::new();
        self.root_name = root_name;
        self.scene_uid = scene_uid;
        self.scene_path = Some(path.to_path_buf());
        self.diags.clear();
        self.bare_paths.clear();
        self.trace.clear();
        self.scrub_step = 0;
        self.selected = self.graph.root;
        self.status = format!("loaded {}", path.display());
        self.refresh();
        self.frame_scene();
    }

    /// Write the current graph to `path`.
    ///
    /// Refuses to write a scene that would not lint clean, so the desktop can
    /// never replace a working file with a broken one.
    pub(crate) fn save_to_path(&mut self, path: &Path) {
        let (ext, sub) = self.source.as_ref().map_or_else(
            || (Vec::new(), Vec::new()),
            |source| {
                let ext = source
                    .ext_resources
                    .iter()
                    .map(|res| (res.resource_type.clone(), res.path.clone(), res.id.clone()))
                    .collect::<Vec<_>>();
                let sub = source
                    .sub_resources
                    .iter()
                    .map(|res| {
                        (
                            res.resource_type.clone(),
                            res.id.clone(),
                            res.properties.clone(),
                        )
                    })
                    .collect::<Vec<_>>();
                (ext, sub)
            },
        );
        let connections: Vec<ConnectionDecl> = self
            .registry
            .connections
            .iter()
            .map(|conn| ConnectionDecl {
                signal: conn.signal.clone(),
                from: conn.from.clone(),
                to: conn.to.clone(),
                method: conn.method.clone(),
            })
            .collect();
        let request = ExportRequest {
            graph: &self.graph,
            connections: connections.as_slice(),
            uid: self.scene_uid.clone(),
            format: 3,
            ext_resources: ext,
            sub_resources: sub,
        };
        let text = match export_tscn(&request) {
            Ok(text) => text,
            Err(reason) => {
                self.status = format!("export failed: {reason}");
                return;
            }
        };
        let findings = scene_lint(text.as_str());
        if !findings.is_empty() {
            self.status = format!(
                "refused to write: {} lint finding(s) - fix them first",
                findings.len()
            );
            return;
        }
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        if path.exists() {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_secs());
            let backup = path.with_extension(format!("tscn.{stamp}.bak"));
            let _ = std::fs::copy(path, backup);
        }
        match std::fs::write(path, text.as_str()) {
            Ok(()) => {
                self.scene_path = Some(path.to_path_buf());
                self.status = format!("wrote {} ({} bytes)", path.display(), text.len());
            }
            Err(err) => self.status = format!("cannot write {}: {err}", path.display()),
        }
    }

    /// Load a depot project's scene into the viewer.
    pub(crate) fn load_depot(&mut self, index: usize) {
        let Some(game) = self.depot_games.get(index) else {
            return;
        };
        let dir = game.dir.clone();
        let kind = game.kind;
        self.selected_game = Some(index);
        self.overlay = gamecraft_core::config::load_overlay(dir.as_path());
        if kind != GameKind::Godot {
            // No Godot scene to parse; keep the current viewport and say so.
            let how = if kind == GameKind::Web {
                "three.js, opens in the browser"
            } else {
                "Bevy, runs as its own window"
            };
            self.status = format!("selected {} ({how})", game.slug);
            return;
        }
        self.load_from_path(dir.join("main.tscn").as_path());
    }

    /// Directory of the selected depot game, if any.
    pub(crate) fn selected_game_dir(&self) -> Option<std::path::PathBuf> {
        self.selected_game
            .and_then(|index| self.depot_games.get(index))
            .map(|game| game.dir.clone())
    }

    /// Open a three.js game in the browser, starting the static server when the
    /// port is closed. `demo` maps to `?autoplay=1`, the visible tour.
    fn open_web(&mut self, game: &DepotGame, demo: bool) {
        let mut url = format!("http://127.0.0.1:11258/games/{}/", game.slug);
        if demo {
            url.push_str("?autoplay=1");
        }
        let port_open = std::net::TcpStream::connect(("127.0.0.1", 11_258)).is_ok();
        if port_open && !probe_web_arcade() {
            self.status =
                "port 11258 is held by something that is not the GameCraft web server".to_string();
            return;
        }
        if !port_open {
            let web_root = game
                .dir
                .parent()
                .and_then(|p| p.parent())
                .map_or_else(|| game.dir.clone(), std::path::Path::to_path_buf);
            let bun = bun_binary();
            match std::process::Command::new(bun.as_str())
                .arg("run")
                .arg("serve.ts")
                .current_dir(web_root)
                .spawn()
            {
                Ok(_) => {
                    for _ in 0..30 {
                        if probe_web_arcade() {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
                Err(err) => {
                    self.status = format!("cannot start web server ({bun}): {err}");
                    return;
                }
            }
        }
        match std::process::Command::new("cmd")
            .args(["/C", "start", "", url.as_str()])
            .spawn()
        {
            Ok(_) => self.status = format!("opened {} in the browser", game.slug),
            Err(err) => self.status = format!("cannot open browser: {err}"),
        }
    }

    /// Persist the app config and say where it went.
    pub(crate) fn save_app_config(&mut self) {
        match self.app_cfg.save() {
            Ok(()) => self.status = "settings saved".to_string(),
            Err(err) => self.status = format!("cannot save settings: {err}"),
        }
    }

    /// Persist the overlay for the selected depot game.
    pub(crate) fn save_overlay(&mut self) {
        let Some(dir) = self.selected_game_dir() else {
            self.status = "pick a game first".to_string();
            return;
        };
        match gamecraft_core::config::save_overlay(dir.as_path(), &self.overlay) {
            Ok(()) => self.status = "overlay saved (applies on next launch)".to_string(),
            Err(err) => self.status = format!("cannot save overlay: {err}"),
        }
    }

    /// Boot the highlighted depot project in Godot.
    ///
    /// Spawns a new engine process rather than attaching to a running one: the
    /// case this serves is "show me this game now", and spawning works whether or
    /// not an editor is already open. `GAMECRAFT_GODOT` overrides the binary.
    pub(crate) fn start_game(&mut self) {
        self.spawn_game(false);
    }

    /// Same, but lets the built-in autoplay play the level.
    pub(crate) fn start_game_demo(&mut self) {
        self.spawn_game(true);
    }

    fn spawn_game(&mut self, demo: bool) {
        let Some(index) = self.selected_game else {
            self.status = "pick a game first".to_string();
            return;
        };
        let Some(game) = self.depot_games.get(index).cloned() else {
            return;
        };
        if game.kind == GameKind::Web {
            self.open_web(&game, demo);
            return;
        }
        if game.kind == GameKind::Bevy {
            self.spawn_bevy(&game, demo);
            return;
        }
        let binary = if self.app_cfg.godot_binary.trim().is_empty() {
            std::env::var("GAMECRAFT_GODOT").unwrap_or_else(|_| "godot".to_string())
        } else {
            self.app_cfg.godot_binary.clone()
        };
        let replaced = self.stop_game();
        let mut command = std::process::Command::new(binary.as_str());
        command.arg("--path").arg(&game.dir);
        if demo {
            command.env("GAMECRAFT_AUTOPLAY", "1");
        }
        match command.spawn() {
            Ok(child) => {
                let mode = if demo { "demo" } else { "play" };
                let note = if replaced {
                    ", previous game closed"
                } else {
                    ""
                };
                self.status = format!("started {} [{mode}] (pid {}{note})", game.slug, child.id());
                self.game_child = Some(child);
            }
            Err(err) => self.status = format!("cannot start godot ({binary}): {err}"),
        }
    }

    /// Start a built Bevy game in its own window; `demo` lets it play itself.
    /// An unbuilt game reports the exact build command instead of compiling in the UI.
    fn spawn_bevy(&mut self, game: &DepotGame, demo: bool) {
        let Some(root) = gamecraft_mcp::catalog::find_root(None) else {
            self.status = "no GameCraft repo found; set GAMECRAFT_ROOT".to_string();
            return;
        };
        let exe = match gamecraft_mcp::catalog::bevy_launch(game, root.as_path()) {
            Ok(exe) => exe,
            Err(hint) => {
                self.status = format!("{}: {hint}", game.slug);
                return;
            }
        };
        let replaced = self.stop_game();
        let mut command = std::process::Command::new(exe);
        command.current_dir(root);
        if demo {
            command.arg("--autoplay");
        }
        match command.spawn() {
            Ok(child) => {
                let mode = if demo { "demo" } else { "play" };
                let note = if replaced {
                    ", previous game closed"
                } else {
                    ""
                };
                self.status = format!("started {} [{mode}] (pid {}{note})", game.slug, child.id());
                self.game_child = Some(child);
            }
            Err(err) => self.status = format!("cannot start {}: {err}", game.slug),
        }
    }

    /// Close the game this viewer started, if it is still running. Returns whether one
    /// was closed.
    pub(crate) fn stop_game(&mut self) -> bool {
        let Some(mut child) = self.game_child.take() else {
            return false;
        };
        if matches!(child.try_wait(), Ok(None)) {
            let _ = child.kill();
            let _ = child.wait();
            return true;
        }
        false
    }

    /// Re-read the file this scene came from.
    pub(crate) fn reload(&mut self) {
        match self.scene_path.clone() {
            Some(path) => self.load_from_path(path.as_path()),
            None => self.status = "no file to reload".to_string(),
        }
    }

    /// Recompute live findings plus the bare-body path set.
    pub(crate) fn refresh(&mut self) {
        self.diags = lint_live(&self.graph, &self.registry);
        let mut bare = HashSet::new();
        for diag in &self.diags {
            if diag.code == ERR_MISSING_COLLISION_SHAPE {
                bare.insert(diag.node.clone());
            }
        }
        self.bare_paths = bare;
    }

    /// Undo one step, reporting failures in the top bar.
    pub(crate) fn undo(&mut self) {
        if let Err(err) = self.stack.undo(&mut self.graph) {
            self.status = format!("undo: {err}");
        } else {
            self.status = "undone".to_string();
            self.refresh();
        }
    }

    /// Redo one step, reporting failures in the top bar.
    pub(crate) fn redo(&mut self) {
        if let Err(err) = self.stack.redo(&mut self.graph) {
            self.status = format!("redo: {err}");
        } else {
            self.status = "redone".to_string();
            self.refresh();
        }
    }

    /// Add a node of `node_type` under the selection (or the root).
    pub(crate) fn add_node(&mut self, node_type: &str) {
        let name = format!("Node{}", self.counter);
        self.counter = self.counter.saturating_add(1);
        let parent = self.selected.or(self.graph.root);
        let cmd = AddNodeCommand::new(name, node_type.to_string(), parent);
        if let Err(err) = self.stack.execute(Box::new(cmd), &mut self.graph) {
            self.status = format!("add failed: {err}");
        } else {
            self.status = format!("added {node_type}");
            self.refresh();
        }
    }

    /// Write a new transform through the undoable command bus.
    pub(crate) fn apply_transform(&mut self, node: NodeId, next: Transform) {
        if let Some(current) = self.graph.get(node) {
            let cmd = SetTransformCommand::new(node, current.transform, next);
            if let Err(err) = self.stack.execute(Box::new(cmd), &mut self.graph) {
                self.status = format!("transform failed: {err}");
            } else {
                self.refresh();
            }
        }
    }

    /// Select a lint-style path (`Root/Body`, `.`, or bare relative).
    pub(crate) fn select_path(&mut self, path: &str) {
        if let Some(root) = self.graph.root {
            let target = if path == "." {
                Some(root)
            } else {
                let prefix = format!("{}/", self.root_name);
                let short = path.strip_prefix(prefix.as_str()).unwrap_or(path);
                self.graph.resolve_path(root, short)
            };
            if let Some(id) = target {
                self.selected = Some(id);
            }
        }
    }

    /// Point the camera at the loaded scene and back off far enough to see it.
    pub(crate) fn frame_scene(&mut self) {
        let Some(root) = self.graph.root else {
            return;
        };
        // Frame on visible geometry only: the root's subtree bounds include the
        // kill plane and trigger volumes, which drag the centre below the floor.
        let mut bounds = gamecraft_core::Aabb::empty();
        for (id, matrix) in spatial::world_matrices(&self.graph) {
            let Some(node) = self.graph.get(id) else {
                continue;
            };
            if !node.properties.contains_key("mesh") {
                continue;
            }
            let local = spatial::local_aabb_guess(node);
            if !local.is_empty() {
                bounds = bounds.union(local.transformed(matrix));
            }
        }
        if bounds.is_empty() {
            bounds = spatial::subtree_aabb(&self.graph, root);
        }
        if bounds.is_empty() {
            return;
        }
        let centre = (bounds.min + bounds.max) * 0.5;
        // The level's kill plane and goal pole inflate the bounds, so fit on a
        // fraction of the diagonal and clamp: fitting the raw diagonal backs the
        // camera off until the scene is a speck.
        let size = (bounds.max - bounds.min).length().max(4.0);
        self.cam_target = centre;
        self.cam_dist = (size * 0.55).clamp(6.0, 26.0);
        self.cam_pitch = 0.32;
    }

    /// Reset the orbit camera to the default pose.
    pub(crate) fn reset_camera(&mut self) {
        self.cam_yaw = 0.6;
        self.cam_pitch = 0.35;
        self.cam_dist = 12.0;
        self.cam_target = Vec3::new(0.0, 1.0, 0.0);
    }

    /// Append one trace entry; follow mode tracks the newest frame.
    pub(crate) fn record_frame(&mut self) {
        if !self.recording {
            return;
        }
        let selected = match self.selected {
            Some(id) => self.graph.node_path(id).unwrap_or_else(|| "-".to_string()),
            None => "-".to_string(),
        };
        let ghost = match self.selected {
            Some(id) => world_pos_of(&self.graph, id),
            None => None,
        };
        self.trace.push_back(TraceFrame {
            yaw: self.cam_yaw,
            pitch: self.cam_pitch,
            selected,
            findings: self.diags.len(),
            ghost,
        });
        while self.trace.len() > TRACE_CAP {
            self.trace.pop_front();
        }
        if self.follow {
            self.scrub_step = u32::try_from(self.trace.len())
                .unwrap_or(0)
                .saturating_sub(1);
        }
    }

    /// Currently scrubbed trace entry, if any.
    #[must_use]
    pub(crate) fn scrubbed(&self) -> Option<&TraceFrame> {
        let index = usize::try_from(self.scrub_step).unwrap_or(0);
        self.trace.get(index)
    }

    /// Scrubber upper bound for the step control.
    #[must_use]
    pub(crate) fn scrub_max(&self) -> u32 {
        u32::try_from(self.trace.len())
            .unwrap_or(0)
            .saturating_sub(1)
    }

    /// Path badges: scene path to finding count.
    #[must_use]
    pub(crate) fn badges(&self) -> HashMap<String, usize> {
        let mut out = HashMap::new();
        for diag in &self.diags {
            out.entry(diag.node.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
        out
    }
}

/// True when whatever listens on 11258 is the `GameCraft` arcade: it must answer
/// `/api/games` with the JSON game list. A bare TCP connect is not enough - any
/// process holding the port would otherwise get a browser opened at it.
fn probe_web_arcade() -> bool {
    use std::io::{Read, Write};
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 11_258));
    let Ok(mut stream) =
        std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(400))
    else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(900)));
    let request = "GET /api/games HTTP/1.0\r\nHost: 127.0.0.1:11258\r\n\r\n";
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut body = String::new();
    let _ = stream.take(8192).read_to_string(&mut body);
    body.contains(" 200 ") && body.contains("\"slug\"")
}

/// Locate bun: explicit override, then the user install (`~/.bun/bin/bun.exe`),
/// then PATH. The viewer used to spawn bare `bun` and failed with "program not
/// found" on a machine where bun is installed but not on the ambient PATH.
fn bun_binary() -> String {
    if let Ok(explicit) = std::env::var("GAMECRAFT_BUN") {
        if !explicit.trim().is_empty() {
            return explicit;
        }
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        let candidate = std::path::Path::new(&home)
            .join(".bun")
            .join("bin")
            .join("bun.exe");
        if candidate.is_file() {
            return candidate.to_string_lossy().into_owned();
        }
    }
    "bun".to_string()
}

/// Every playable project via the shared catalog (Godot under `depot/games/`,
/// three.js under `web/games/`). The root comes from `GAMECRAFT_ROOT` or by
/// walking up from the working directory and the executable.
fn discover_depot() -> Vec<DepotGame> {
    gamecraft_mcp::catalog::find_root(None)
        .map(|root| gamecraft_mcp::catalog::discover(root.as_path()))
        .unwrap_or_default()
}

/// Was the gate result a pass? Web results carry `verify.pass`, Godot ones `won`.
fn gate_passed(value: &serde_json::Value) -> bool {
    value
        .pointer("/verify/pass")
        .or_else(|| value.get("won"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

impl DesktopState {
    /// Games passing the filter, grouped as the sidebar and the grid show them:
    /// runtime, then group (known groups first), then title.
    #[must_use]
    pub(crate) fn visible_sections(&self) -> Vec<(GameKind, String, Vec<usize>)> {
        let filter = self.depot_filter.trim().to_lowercase();
        let mut out = Vec::new();
        for kind in GameKind::ALL {
            let known = kind.groups();
            let mut groups: Vec<String> = known.iter().map(|g| (*g).to_string()).collect();
            for game in self.depot_games.iter().filter(|g| g.kind == kind) {
                if !groups.contains(&game.group) {
                    groups.push(game.group.clone());
                }
            }
            for group in groups {
                let members: Vec<usize> = (0..self.depot_games.len())
                    .filter(|&i| {
                        self.depot_games.get(i).is_some_and(|g| {
                            g.kind == kind
                                && g.group == group
                                && gamecraft_mcp::catalog::matches(g, filter.as_str())
                        })
                    })
                    .collect();
                if !members.is_empty() {
                    out.push((kind, group, members));
                }
            }
        }
        out
    }

    /// Move the selection `step` places through the visible games, wrapping.
    pub(crate) fn step_selection(&mut self, step: i64) {
        let order: Vec<usize> = self
            .visible_sections()
            .into_iter()
            .flat_map(|(_, _, members)| members)
            .collect();
        let Some(&first) = order.first() else {
            return;
        };
        let next = match self
            .selected_game
            .and_then(|s| order.iter().position(|&i| i == s))
        {
            Some(pos) => {
                let len = i64::try_from(order.len()).unwrap_or(1);
                let at = (i64::try_from(pos).unwrap_or(0) + step).rem_euclid(len);
                order
                    .get(usize::try_from(at).unwrap_or(0))
                    .copied()
                    .unwrap_or(first)
            }
            None => first,
        };
        self.load_depot(next);
        self.scroll_to_selected = true;
    }

    /// The selected game's cheat sheet, read from disk once per selection.
    pub(crate) fn cheatsheet(&mut self) -> Option<String> {
        let game = self.selected_game.and_then(|i| self.depot_games.get(i))?;
        if let Some((slug, text)) = &self.sheet {
            if *slug == game.slug {
                return Some(text.clone()).filter(|t| !t.is_empty());
            }
        }
        let text = gamecraft_mcp::catalog::report(game)
            .get("cheatsheet")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        self.sheet = Some((game.slug.clone(), text.clone()));
        Some(text).filter(|t| !t.is_empty())
    }

    /// Run the selected game's gate on a background thread (web: headless
    /// self-play; Godot: headless boot to a win marker). One at a time.
    pub(crate) fn run_tests(&mut self) {
        if let Some(job) = &self.verify_job {
            self.status = format!("the gate for {} is still running", job.slug);
            return;
        }
        let Some(game) = self
            .selected_game
            .and_then(|i| self.depot_games.get(i))
            .cloned()
        else {
            self.status = "pick a game first".to_string();
            return;
        };
        let Some(root) = gamecraft_mcp::catalog::find_root(None) else {
            self.status = "no GameCraft repo found; set GAMECRAFT_ROOT".to_string();
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let slug = game.slug.clone();
        std::thread::spawn(move || {
            let result = match game.kind {
                GameKind::Web => {
                    gamecraft_mcp::catalog::web_verify(root.as_path(), game.slug.as_str(), 300)
                }
                GameKind::Godot => {
                    gamecraft_mcp::catalog::godot_verify(game.dir.as_path(), 3600, true, Some(300))
                }
                GameKind::Bevy => {
                    gamecraft_mcp::catalog::bevy_verify(root.as_path(), game.slug.as_str(), 300)
                }
            };
            let _ = tx.send(result);
        });
        self.status = format!("running the gate for {slug} (headless, up to 5 min)");
        self.verify_job = Some(VerifyJob { slug, rx });
    }

    /// Pick up a finished gate: report it and re-read the catalog so the card
    /// shows the new result.
    pub(crate) fn poll_verify(&mut self) {
        let Some(job) = &self.verify_job else {
            return;
        };
        let outcome = match job.rx.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err("the gate thread ended without a result".to_string())
            }
        };
        let slug = job.slug.clone();
        self.verify_job = None;
        self.status = match outcome {
            Ok(value) if gate_passed(&value) => format!("{slug}: gate passed"),
            Ok(value) => {
                let why = value
                    .pointer("/verify/failed")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("no win marker");
                format!("{slug}: gate FAILED ({why})")
            }
            Err(reason) => format!("{slug}: could not run the gate: {reason}"),
        };
        self.rediscover();
    }

    /// Re-read the catalog, keeping the selection by slug.
    pub(crate) fn rediscover(&mut self) {
        let keep = self
            .selected_game
            .and_then(|i| self.depot_games.get(i))
            .map(|g| (g.slug.clone(), g.kind));
        self.depot_games = discover_depot();
        self.selected_game = keep.and_then(|(slug, kind)| {
            self.depot_games
                .iter()
                .position(|g| g.slug == slug && g.kind == kind)
        });
        self.sheet = None;
    }

    /// Write the current graph (edits included) as glTF: `.gltf` gives text with
    /// the buffer embedded, anything else binary `.glb`.
    pub(crate) fn export_gltf(&mut self, path: &Path) {
        let subs: Vec<gamecraft_gltf::SubRes> = self
            .source
            .as_ref()
            .map(|file| {
                file.sub_resources
                    .iter()
                    .map(gamecraft_gltf::SubRes::from)
                    .collect()
            })
            .unwrap_or_default();
        let text_form = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("gltf"));
        let textures = match (self.source.as_ref(), self.scene_path.as_ref()) {
            (Some(file), Some(scene)) => {
                gamecraft_gltf::load_textures(file, gamecraft_gltf::project_root(scene).as_path())
            }
            _ => gamecraft_gltf::Textures::new(),
        };
        let written =
            gamecraft_gltf::export_textured(&self.graph, &subs, &textures).and_then(|gltf| {
                let bytes = if text_form {
                    gltf.to_gltf_embedded().into_bytes()
                } else {
                    gltf.to_glb()?
                };
                std::fs::write(path, &bytes).map_err(|e| e.to_string())?;
                Ok(gltf.stats)
            });
        self.status = match written {
            Ok(s) => format!(
                "exported {}: {} meshes, {} triangles, {} lights, {} textures",
                path.display(),
                s.meshes,
                s.triangles,
                s.lights,
                s.textures
            ),
            Err(err) => format!("glTF export failed: {err}"),
        };
    }

    /// Show the selected game's folder in Explorer.
    pub(crate) fn open_folder(&mut self) {
        let Some(dir) = self.selected_game_dir() else {
            self.status = "pick a game first".to_string();
            return;
        };
        let opener = if cfg!(windows) {
            "explorer"
        } else {
            "xdg-open"
        };
        if let Err(err) = std::process::Command::new(opener).arg(&dir).spawn() {
            self.status = format!("cannot open {}: {err}", dir.display());
        }
    }
}

/// Resolve a repo-relative path by walking up from the working directory.
///
/// A binary launched from Explorer or a shortcut has an arbitrary cwd; hardcoding
/// the author's repo path would work on exactly one machine (BUG-079).
fn find_up(relative: &str) -> Option<PathBuf> {
    let mut cursor = std::env::current_dir().ok()?;
    for _ in 0..5 {
        let candidate = cursor.join(relative);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !cursor.pop() {
            break;
        }
    }
    None
}

/// World-space translation of one node, if it exists.
fn world_pos_of(graph: &SceneGraph, id: NodeId) -> Option<Vec3> {
    for (node, matrix) in spatial::world_matrices(graph) {
        if node == id {
            return Some(Transform::world_position(matrix));
        }
    }
    None
}

/// First `wgpu` adapter label (probed once; no device is opened).
fn wgpu_adapter_label() -> String {
    let instance = wgpu::Instance::default();
    let adapters = futures::executor::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
    if let Some(adapter) = adapters.into_iter().next() {
        let info = adapter.get_info();
        format!("wgpu: {} ({:?})", info.name, info.backend)
    } else {
        "wgpu: no adapter".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Root plus one healthy body with a shape.
    const TINY: &str = "[gd_scene load_steps=2 format=3 uid=\"uid://tiny\"]\n\n[node name=\"Root\" type=\"Node3D\"]\n\n[node name=\"Good\" type=\"StaticBody3D\" parent=\".\"]\n\n[node name=\"Shape\" type=\"CollisionShape3D\" parent=\"Good\"]\n";

    /// A catalog entry for navigation tests.
    fn entry(kind: GameKind, slug: &str, group: &str) -> DepotGame {
        DepotGame {
            kind,
            slug: slug.to_string(),
            title: slug.to_string(),
            group: group.to_string(),
            description: String::new(),
            dir: PathBuf::from(slug),
            verify: None,
        }
    }

    #[test]
    fn keyboard_navigation_follows_grid_order_and_filter() {
        let mut state = DesktopState::from_text(TINY, "uid://fallback");
        // Stored alphabetically; shown Godot first, then by known group order.
        state.depot_games = vec![
            entry(GameKind::Web, "bento", "Calm puzzles"),
            entry(GameKind::Godot, "maze", "Puzzles"),
            entry(GameKind::Godot, "pong", "Arcade"),
            entry(GameKind::Web, "hoshi", "Seek and find"),
        ];
        state.selected_game = None;
        let slug = |s: &DesktopState| {
            s.selected_game
                .and_then(|i| s.depot_games.get(i))
                .map(|g| g.slug.clone())
                .unwrap_or_default()
        };
        state.step_selection(1);
        assert_eq!(slug(&state), "pong");
        state.step_selection(1);
        assert_eq!(slug(&state), "maze");
        state.step_selection(1);
        assert_eq!(slug(&state), "hoshi");
        state.step_selection(1);
        assert_eq!(slug(&state), "bento");
        state.step_selection(1);
        assert_eq!(slug(&state), "pong", "wraps");
        state.step_selection(-1);
        assert_eq!(slug(&state), "bento", "wraps backwards");
        state.depot_filter = "seek".to_string();
        state.step_selection(1);
        assert_eq!(
            slug(&state),
            "hoshi",
            "filtered-out selection jumps to the first match"
        );
    }

    #[test]
    fn undo_redo_depth_tracks_stack() {
        let mut state = DesktopState::from_text(TINY, "uid://fallback");
        assert_eq!(state.stack.undo_len(), 0);
        state.add_node("RigidBody3D");
        assert_eq!(state.stack.undo_len(), 1);
        state.undo();
        assert_eq!(state.stack.undo_len(), 0);
        assert_eq!(state.stack.redo_len(), 1);
        state.redo();
        assert_eq!(state.stack.undo_len(), 1);
        assert_eq!(state.stack.redo_len(), 0);
    }

    #[test]
    fn select_path_resolves_lint_string() {
        let mut state = DesktopState::from_text(TINY, "uid://fallback");
        state.select_path("Root/Good");
        let want = state
            .graph
            .root
            .and_then(|root| state.graph.resolve_path(root, "Good"));
        assert_ne!(want, state.graph.root);
        assert_eq!(state.selected, want);
    }

    #[test]
    fn scrubbed_ghost_absent_without_selection() {
        let mut state = DesktopState::from_text(TINY, "uid://fallback");
        state.selected = None;
        state.record_frame();
        if let Some(entry) = state.scrubbed() {
            assert_eq!(entry.ghost, None::<Vec3>);
        }
        assert_eq!(state.trace.len(), 1);
    }

    #[test]
    fn scene_uid_prefers_header_tag() {
        let state = DesktopState::from_text(TINY, "uid://fallback");
        assert_eq!(state.scene_uid, "uid://tiny");
        let bare = DesktopState::from_text("[gd_scene load_steps=2 format=3]\n", "uid://fallback");
        assert_eq!(bare.scene_uid, "uid://fallback");
    }
}

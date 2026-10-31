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
}

/// Which runtime a depot entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GameKind {
    /// Godot 4 project under `depot/games/`.
    Godot,
    /// three.js page under `web/games/`.
    Web,
}

/// One playable project discovered under `depot/games/` or `web/games/`.
#[derive(Debug, Clone)]
pub(crate) struct DepotGame {
    /// Which runtime launches it.
    pub(crate) kind: GameKind,
    /// Folder name (`platformer`, `flight`, ...).
    pub(crate) slug: String,
    /// Display label.
    pub(crate) title: String,
    /// Project directory (passed to Godot as `--path`).
    pub(crate) dir: PathBuf,
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
        let mut visuals = egui::Visuals::dark();
        // Contrast: egui's defaults are mid-greys on near-black, which is hard to
        // read on a dark panel. Force a bright text colour and give each widget
        // state a clearly different stroke, then bump the type sizes.
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
        ctx.egui_ctx.set_visuals(visuals);
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
            adapter_label: wgpu_adapter_label(),
            root_name,
            scene_uid,
            scene_path: None,
            source: scene,
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
        if kind == GameKind::Web {
            // No Godot scene to parse; keep the current viewport and say so.
            self.status = format!("selected {} (three.js, opens in the browser)", game.slug);
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
        let binary = if self.app_cfg.godot_binary.trim().is_empty() {
            std::env::var("GAMECRAFT_GODOT").unwrap_or_else(|_| "godot".to_string())
        } else {
            self.app_cfg.godot_binary.clone()
        };
        let mut command = std::process::Command::new(binary.as_str());
        command.arg("--path").arg(&game.dir);
        if demo {
            command.env("GAMECRAFT_AUTOPLAY", "1");
        }
        match command.spawn() {
            Ok(child) => {
                let mode = if demo { "demo" } else { "play" };
                self.status = format!("started {} [{mode}] (pid {})", game.slug, child.id());
            }
            Err(err) => self.status = format!("cannot start godot ({binary}): {err}"),
        }
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

/// Find every playable project: Godot scenes under `depot/games/`, three.js
/// pages under `web/games/`. Two runtimes, one list.
fn discover_depot() -> Vec<DepotGame> {
    let mut games: Vec<DepotGame> = Vec::new();
    if let Some(root) = find_up_dir("depot/games") {
        collect_games(root.as_path(), "main.tscn", GameKind::Godot, &mut games);
    }
    if let Some(root) = find_up_dir("web/games") {
        collect_games(root.as_path(), "game.json", GameKind::Web, &mut games);
    }
    games.sort_by(|left, right| left.slug.cmp(&right.slug));
    games
}

/// Append every child directory of `root` that contains `marker`.
fn collect_games(root: &Path, marker: &str, kind: GameKind, out: &mut Vec<DepotGame>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.join(marker).is_file() {
            continue;
        }
        let slug = dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        if slug.is_empty() {
            continue;
        }
        out.push(DepotGame {
            kind,
            title: slug.replace('-', " "),
            slug,
            dir,
        });
    }
}

/// Resolve a repo-relative **directory** by walking up from the cwd.
///
/// `find_up` checks `is_file`, which never matches `depot/games` - the reason the
/// depot list first came up empty.
fn find_up_dir(relative: &str) -> Option<PathBuf> {
    let mut cursor = std::env::current_dir().ok()?;
    for _ in 0..5 {
        let candidate = cursor.join(relative);
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !cursor.pop() {
            break;
        }
    }
    None
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

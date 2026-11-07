//! Side panels: command palette, scene tree, inspector, lint list, trace bar.

use crate::state::{DesktopState, Dialog, GameKind};
use gamecraft_core::NodeId;
use std::path::PathBuf;

/// Node types offered by the Edit > Add node submenu.
const ADDABLE: [&str; 5] = [
    "Node3D",
    "StaticBody3D",
    "RigidBody3D",
    "CollisionShape3D",
    "MeshInstance3D",
];

/// File menu: open, reload, save as, export glTF, quit.
fn file_menu(state: &mut DesktopState, menu: &mut egui::Ui) {
    if menu.button("Open scene").clicked() {
        if let Some(path) = pick_scene() {
            state.load_from_path(path.as_path());
        }
        menu.close();
    }
    let can_reload = state.scene_path.is_some();
    if menu
        .add_enabled(can_reload, egui::Button::new("Reload"))
        .clicked()
    {
        state.reload();
        menu.close();
    }
    if menu.button("Save as").clicked() {
        if let Some(path) = save_scene_as(state.scene_path.as_ref()) {
            state.save_to_path(path.as_path());
        }
        menu.close();
    }
    if menu
        .button("Export glTF")
        .on_hover_text("The scene as it is now (edits included) as .glb, or .gltf text")
        .clicked()
    {
        if let Some(path) = export_gltf_as(state.scene_path.as_ref()) {
            state.export_gltf(path.as_path());
        }
        menu.close();
    }
    menu.separator();
    if menu.button("Quit").clicked() {
        menu.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

/// Menu bar: File / Edit / View / Help.
pub(crate) fn menu_bar(state: &mut DesktopState, ui: &mut egui::Ui) {
    egui::MenuBar::new().ui(ui, |bar| {
        bar.menu_button("File", |menu| file_menu(state, menu));
        bar.menu_button("Edit", |menu| {
            let can_undo = state.stack.undo_len() > 0;
            if menu
                .add_enabled(can_undo, egui::Button::new("Undo"))
                .clicked()
            {
                state.undo();
                menu.close();
            }
            let can_redo = state.stack.redo_len() > 0;
            if menu
                .add_enabled(can_redo, egui::Button::new("Redo"))
                .clicked()
            {
                state.redo();
                menu.close();
            }
            menu.separator();
            let mut chosen: Option<&'static str> = None;
            menu.menu_button("Add node", |add| {
                for node_type in ADDABLE {
                    if add.button(node_type).clicked() {
                        chosen = Some(node_type);
                    }
                }
            });
            if let Some(node_type) = chosen {
                state.add_node(node_type);
                menu.close();
            }
            menu.separator();
            if menu.button("Reset camera").clicked() {
                state.reset_camera();
                menu.close();
            }
        });
        bar.menu_button("View", |menu| {
            menu.checkbox(&mut state.show_meshes, "Meshes (shaded)");
            menu.checkbox(&mut state.show_boxes, "AABB boxes (wireframe)");
            menu.checkbox(&mut state.show_collision, "Collision colours");
            menu.checkbox(&mut state.show_frustum, "Frustum");
            menu.checkbox(&mut state.show_axes, "Ground axes");
            menu.separator();
            menu.checkbox(&mut state.recording, "Record trace");
            menu.checkbox(&mut state.follow, "Follow newest frame");
            if menu
                .button("Reset layout")
                .on_hover_text("Forget dragged panel sizes")
                .clicked()
            {
                state.layout_gen = state.layout_gen.wrapping_add(1);
            }
        });
        bar.menu_button("Commands", |menu| {
            if menu.button("Command palette  Ctrl+Shift+P").clicked() {
                state.palette.open = true;
                menu.close();
            }
        });
        bar.menu_button("Settings", |menu| {
            if menu.button("App settings").clicked() {
                state.dialog = Some(Dialog::AppSettings);
                menu.close();
            }
            if menu
                .add_enabled(
                    state.selected_game.is_some(),
                    egui::Button::new("Game settings"),
                )
                .clicked()
            {
                state.dialog = Some(Dialog::GameOverlay);
                menu.close();
            }
        });
        bar.menu_button("Security", |menu| {
            if state.is_authenticated {
                menu.label(format!("Operator: {}", state.auth_username));
                if menu.button("Sign Out").clicked() {
                    state.is_authenticated = false;
                    state.status = "Signed out".to_string();
                    menu.close();
                }
            } else if menu.button("Basic Auth Login...").clicked() {
                state.dialog = Some(Dialog::BasicAuth);
                menu.close();
            }
        });
        bar.menu_button("Help", |menu| {
            if menu.button("About / controls").clicked() {
                state.dialog = Some(Dialog::About);
                menu.close();
            }
        });
    });
}

/// Native open dialog, starting in the depot when it can be found.
fn pick_scene() -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Open Godot scene")
        .add_filter("Godot scene", &["tscn", "tres"]);
    if let Some(dir) = scenes_dir() {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_file()
}

/// Native save dialog, defaulting to the current path or `scene.tscn`.
fn save_scene_as(current: Option<&PathBuf>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Save scene as")
        .add_filter("Godot scene", &["tscn"]);
    if let Some(path) = current {
        if let Some(parent) = path.parent() {
            dialog = dialog.set_directory(parent);
        }
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            dialog = dialog.set_file_name(name);
        }
    } else {
        dialog = dialog.set_file_name("scene.tscn");
    }
    dialog.save_file()
}

/// Native save dialog for a glTF export, defaulting next to the scene.
fn export_gltf_as(current: Option<&PathBuf>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Export glTF")
        .add_filter("glTF binary", &["glb"])
        .add_filter("glTF text", &["gltf"]);
    let stem = current
        .and_then(|path| {
            if let Some(parent) = path.parent() {
                dialog = dialog.clone().set_directory(parent);
            }
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "scene".to_string());
    dialog.set_file_name(format!("{stem}.glb")).save_file()
}

/// Locate `depot/games` by walking up from the working directory.
///
/// Deliberately not a compile-time path: a binary that only works on the machine
/// that built it is not a tool.
fn scenes_dir() -> Option<PathBuf> {
    let mut cursor = std::env::current_dir().ok()?;
    for _ in 0..5 {
        let candidate = cursor.join("depot").join("games");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !cursor.pop() {
            break;
        }
    }
    None
}

/// Model-id shortcuts per provider. The text field stays editable; these are
/// suggestions, not a closed list.
fn model_presets(provider: &str) -> &'static [(&'static str, &'static str)] {
    match provider {
        "openai" => &[("gpt-4o-mini (cheap)", "gpt-4o-mini"), ("gpt-4o", "gpt-4o")],
        "anthropic" => &[("claude-sonnet-4-5", "claude-sonnet-4-5")],
        "deepseek" => &[
            ("deepseek-chat (general)", "deepseek-chat"),
            ("deepseek-reasoner (thinking)", "deepseek-reasoner"),
        ],
        "meta" => &[
            (
                "Llama-4-Maverick (general)",
                "Llama-4-Maverick-17B-128E-Instruct-FP8",
            ),
            (
                "Llama-4-Scout (long context)",
                "Llama-4-Scout-17B-16E-Instruct-FP8",
            ),
        ],
        _ => &[("auto (recommended)", ""), ("gemma4:12b", "gemma4:12b")],
    }
}

/// Modal windows opened from the menu bar.
pub(crate) fn dialogs(state: &mut DesktopState, ctx: &egui::Context) {
    let Some(dialog) = state.dialog else {
        return;
    };
    let mut open = true;
    match dialog {
        Dialog::About => about_window(state, ctx, &mut open),
        Dialog::AppSettings => app_settings_window(state, ctx, &mut open),
        Dialog::GameOverlay => game_overlay_window(state, ctx, &mut open),
        Dialog::BasicAuth => basic_auth_window(state, ctx, &mut open),
    }
    if !open {
        state.dialog = None;
    }
}

/// About window.
fn about_window(state: &mut DesktopState, ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("About GameCraft")
        .collapsible(false)
        .resizable(false)
        .open(open)
        .show(ctx, |ui| {
            ui.strong(format!("GameCraft {}", env!("CARGO_PKG_VERSION")));
            ui.label("Spatial compiler and scene verifier for Godot 4.");
            ui.separator();
            ui.monospace(format!("scene: {}", state.scene_uid));
            ui.monospace(format!("nodes: {}", state.graph.nodes.len()));
            ui.monospace(format!("findings: {}", state.diags.len()));
            ui.monospace(format!("renderer: {}", state.adapter_label));
            ui.separator();
            ui.label("Drag the viewport to orbit, scroll to zoom.");
            ui.label("Click a finding to select its node.");
            ui.label("Ctrl+Z / Ctrl+Y undo and redo. Esc clears the palette.");
        });
}

/// App settings window: provider, model, keys, binaries.
fn basic_auth_window(state: &mut DesktopState, ctx: &egui::Context, open: &mut bool) {
    let mut close = false;
    egui::Window::new("Security: Basic Authentication")
        .collapsible(false)
        .resizable(false)
        .open(open)
        .show(ctx, |ui| {
            ui.label("Log in with your operator credentials:");
            ui.add_space(6.0);
            ui.horizontal(|row| {
                row.label("Username:");
                row.text_edit_singleline(&mut state.auth_username);
            });
            ui.horizontal(|row| {
                row.label("Password:");
                row.add(egui::TextEdit::singleline(&mut state.auth_password_input).password(true));
            });
            ui.add_space(8.0);
            ui.horizontal(|row| {
                if row.button("Login").clicked() {
                    if !state.auth_username.is_empty() && !state.auth_password_input.is_empty() {
                        state.is_authenticated = true;
                        state.status = format!("Authenticated as {}", state.auth_username);
                        state.auth_password_input.clear();
                        close = true;
                    } else {
                        state.status = "Username and password required".to_string();
                    }
                }
                if row.button("Cancel").clicked() {
                    state.auth_password_input.clear();
                    close = true;
                }
            });
        });
    if close {
        *open = false;
    }
}

/// App settings window: provider, model, keys, binaries.
fn app_settings_window(state: &mut DesktopState, ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("App settings")
        .collapsible(false)
        .resizable(false)
        .open(open)
        .show(ctx, |ui| {
            ui.label("Provider");
            egui::ComboBox::from_id_salt("app_provider")
                .selected_text(state.app_cfg.provider.as_str())
                .show_ui(ui, |combo| {
                    for name in ["auto", "local", "openai", "anthropic", "deepseek", "meta"] {
                        combo.selectable_value(&mut state.app_cfg.provider, name.to_string(), name);
                    }
                });
            ui.label("Model (empty = auto-pick, gemma4 family preferred)");
            ui.text_edit_singleline(&mut state.app_cfg.model);
            ui.label("Model presets (fills the field above)");
            egui::ComboBox::from_id_salt("app_model_preset")
                .selected_text("Pick a preset…")
                .show_ui(ui, |combo| {
                    for (label, id) in model_presets(state.app_cfg.provider.as_str()) {
                        if combo.button(*label).clicked() {
                            state.app_cfg.model = (*id).to_string();
                        }
                    }
                });
            ui.label("Local endpoint (empty = provider default)");
            ui.text_edit_singleline(&mut state.app_cfg.ollama_endpoint);
            ui.label("OpenAI API key (empty = OPENAI_API_KEY env)");
            ui.add(egui::TextEdit::singleline(&mut state.app_cfg.openai_api_key).password(true));
            ui.label("Anthropic API key (empty = ANTHROPIC_API_KEY env)");
            ui.add(egui::TextEdit::singleline(&mut state.app_cfg.anthropic_api_key).password(true));
            ui.label("DeepSeek API key (empty = DEEPSEEK_API_KEY env)");
            ui.add(egui::TextEdit::singleline(&mut state.app_cfg.deepseek_api_key).password(true));
            ui.label("Meta API key (empty = META_API_KEY env)");
            ui.add(egui::TextEdit::singleline(&mut state.app_cfg.meta_api_key).password(true));
            ui.label("Godot binary (empty = godot)");
            ui.text_edit_singleline(&mut state.app_cfg.godot_binary);
            ui.separator();
            ui.horizontal(|row| {
                if row.button("Save").clicked() {
                    state.save_app_config();
                    state.dialog = None;
                }
                if row.button("Cancel").clicked() {
                    state.dialog = None;
                }
            });
        });
}

/// Per-game overlay window: speed, gravity, jump, autoplay.
fn game_overlay_window(state: &mut DesktopState, ctx: &egui::Context, open: &mut bool) {
    let title = state
        .selected_game
        .and_then(|index| state.depot_games.get(index))
        .map_or_else(String::new, |game| game.title.clone());
    egui::Window::new(format!("Game settings - {title}"))
        .collapsible(false)
        .resizable(false)
        .open(open)
        .show(ctx, |ui| {
            ui.label("Multipliers applied to the baked-in base values.");
            ui.add(egui::Slider::new(&mut state.overlay.speed_mult, 0.25..=2.0).text("Speed"));
            ui.add(egui::Slider::new(&mut state.overlay.gravity_mult, 0.25..=2.0).text("Gravity"));
            ui.add(egui::Slider::new(&mut state.overlay.jump_mult, 0.25..=2.0).text("Jump"));
            ui.label("Autoplay (Default follows the demo switch)");
            ui.horizontal(|row| {
                row.selectable_value(&mut state.overlay.autoplay, None, "Default");
                row.selectable_value(&mut state.overlay.autoplay, Some(true), "On");
                row.selectable_value(&mut state.overlay.autoplay, Some(false), "Off");
            });
            ui.separator();
            ui.horizontal(|row| {
                if row.button("Save").clicked() {
                    state.save_overlay();
                    state.dialog = None;
                }
                if row.button("Save & play").clicked() {
                    state.save_overlay();
                    state.dialog = None;
                    state.start_game();
                }
                if row.button("Cancel").clicked() {
                    state.dialog = None;
                }
            });
        });
}

/// Palette entry: label plus what running it does.
struct PaletteEntry {
    /// Display label.
    label: String,
    /// Action tag.
    kind: PaletteKind,
}

/// Palette action tags.
enum PaletteKind {
    /// Add this node type under the selection.
    Add(String),
    /// Select the scene root.
    SelectRoot,
    /// Reset the orbit camera.
    ResetCamera,
    /// Undo one step.
    Undo,
    /// Redo one step.
    Redo,
}

/// Top bar: input, undo/redo, findings, status, palette dropdown, shortcuts.
pub(crate) fn top_bar(state: &mut DesktopState, ui: &mut egui::Ui) {
    egui::containers::Panel::top("topbar").show(ui, |ui| {
        menu_bar(state, ui);
        ui.horizontal(|row| {
            row.label("GameCraft");
            row.selectable_value(&mut state.library, true, "Library")
                .on_hover_text("All games as cards, with details and the gate");
            row.selectable_value(&mut state.library, false, "Editor")
                .on_hover_text("Scene tree, 3D viewport, inspector and lint");
            mode_extras(state, row);
            row.separator();
            if state.library {
                row.label(state.status.clone());
                return;
            }
            row.monospace(state.scene_uid.as_str());
            row.add(
                egui::TextEdit::singleline(&mut state.palette_query)
                    .hint_text("command")
                    .desired_width(150.0),
            );
            let undo = row
                .add_enabled(state.stack.undo_len() > 0, egui::Button::new("Undo"))
                .clicked();
            let redo = row
                .add_enabled(state.stack.redo_len() > 0, egui::Button::new("Redo"))
                .clicked();
            if undo {
                state.undo();
            }
            if redo {
                state.redo();
            }
            row.label(format!("{} findings", state.diags.len()));
            row.label(state.status.clone());
        });
        show_palette_dropdown(state, ui);
    });
    palette_keys(state, ui);
}

/// Theme toggle and zoom readout (click resets to 100 %).
fn mode_extras(state: &mut DesktopState, row: &mut egui::Ui) {
    let theme_label = if state.light { "Dark" } else { "Light" };
    if row
        .button(theme_label)
        .on_hover_text("Switch between light and dark")
        .clicked()
    {
        state.light = !state.light;
        crate::state::apply_theme(row.ctx(), state.light);
    }
    let zoom = row.ctx().zoom_factor();
    let pct = format!("{:.0}%", zoom * 100.0);
    if row
        .small_button(pct)
        .on_hover_text("UI zoom: Ctrl + mouse wheel or Ctrl +/-. Click to reset.")
        .clicked()
    {
        row.ctx().set_zoom_factor(1.0);
    }
}

/// Vertical dropdown under the input; click runs, keys navigate.
fn show_palette_dropdown(state: &mut DesktopState, ui: &mut egui::Ui) {
    if state.palette_query.is_empty() {
        return;
    }
    let matches = palette_matches(state.palette_query.as_str());
    if matches.is_empty() {
        return;
    }
    if state.palette_index >= matches.len() {
        state.palette_index = 0;
    }
    let highlight = state.palette_index;
    ui.vertical(|list| {
        for (index, entry) in matches.iter().enumerate() {
            let label = if index == highlight {
                format!("> {}", entry.label)
            } else {
                format!("  {}", entry.label)
            };
            if list.button(label).clicked() {
                run_palette(state, &entry.kind);
            }
        }
    });
}

/// Filtered palette entries for a query (case-insensitive substring).
fn palette_matches(query: &str) -> Vec<PaletteEntry> {
    let folded = query.to_lowercase();
    let mut out: Vec<PaletteEntry> = palette_entries()
        .into_iter()
        .filter(|entry| entry.label.to_lowercase().contains(folded.as_str()))
        .collect();
    out.truncate(6);
    out
}

/// Keyboard: Esc clears, Up/Down cycle, Enter runs, Ctrl+Z/Y undo/redo.
fn palette_keys(state: &mut DesktopState, ui: &mut egui::Ui) {
    if ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) {
        state.palette_query.clear();
        state.palette_index = 0;
    }
    let matches = palette_matches(state.palette_query.as_str());
    if !matches.is_empty() {
        if state.palette_index >= matches.len() {
            state.palette_index = 0;
        }
        if ui
            .ctx()
            .input(|input| input.key_pressed(egui::Key::ArrowDown))
        {
            state.palette_index = (state.palette_index + 1) % matches.len();
        }
        if ui
            .ctx()
            .input(|input| input.key_pressed(egui::Key::ArrowUp))
        {
            state.palette_index = if state.palette_index == 0 {
                matches.len() - 1
            } else {
                state.palette_index - 1
            };
        }
        if ui.ctx().input(|input| input.key_pressed(egui::Key::Enter)) {
            if let Some(entry) = matches.get(state.palette_index) {
                run_palette(state, &entry.kind);
            }
        }
    }
    if ui
        .ctx()
        .input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z))
    {
        state.undo();
    }
    if ui
        .ctx()
        .input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y))
    {
        state.redo();
    }
}

/// All palette actions with labels.
fn palette_entries() -> Vec<PaletteEntry> {
    let mut out = Vec::new();
    for node_type in [
        "Node3D",
        "StaticBody3D",
        "RigidBody3D",
        "CollisionShape3D",
        "MeshInstance3D",
    ] {
        out.push(PaletteEntry {
            label: format!("Add {node_type}"),
            kind: PaletteKind::Add(node_type.to_string()),
        });
    }
    out.push(PaletteEntry {
        label: "Select root".to_string(),
        kind: PaletteKind::SelectRoot,
    });
    out.push(PaletteEntry {
        label: "Reset camera".to_string(),
        kind: PaletteKind::ResetCamera,
    });
    out.push(PaletteEntry {
        label: "Undo".to_string(),
        kind: PaletteKind::Undo,
    });
    out.push(PaletteEntry {
        label: "Redo".to_string(),
        kind: PaletteKind::Redo,
    });
    out
}

/// Execute one palette action through the command bus.
fn run_palette(state: &mut DesktopState, kind: &PaletteKind) {
    match kind {
        PaletteKind::Add(node_type) => state.add_node(node_type.as_str()),
        PaletteKind::SelectRoot => {
            state.selected = state.graph.root;
        }
        PaletteKind::ResetCamera => state.reset_camera(),
        PaletteKind::Undo => state.undo(),
        PaletteKind::Redo => state.redo(),
    }
}

/// Groups of one runtime in display order: the known ones first, then any new ones.
fn ordered_groups(state: &DesktopState, kind: GameKind, known: &[&str]) -> Vec<String> {
    let mut groups: Vec<String> = known.iter().map(|g| (*g).to_string()).collect();
    for game in state.depot_games.iter().filter(|g| g.kind == kind) {
        if !groups.contains(&game.group) {
            groups.push(game.group.clone());
        }
    }
    groups
}

/// Does a game pass the sidebar filter? Matches title, slug or group, any case.
fn matches(game: &crate::state::DepotGame, filter: &str) -> bool {
    filter.is_empty()
        || [game.title.as_str(), game.slug.as_str(), game.group.as_str()]
            .iter()
            .any(|text| text.to_lowercase().contains(filter))
}

/// One depot row: thumbnail and selectable title. Returns true when clicked.
fn depot_row(state: &DesktopState, ui: &mut egui::Ui, index: usize) -> bool {
    let Some(game) = state.depot_games.get(index) else {
        return false;
    };
    let selected = state.selected_game == Some(index);
    let mut clicked = false;
    // horizontal_top + fixed height + thumb placeholder: rows with and without
    // thumbnails share the same height, so the left list stacks vertically
    // instead of stairstepping when thumbnails pop in late.
    ui.horizontal_top(|row| {
        row.set_min_height(30.0);
        if let Some(thumb) = state.thumbs.get(game.slug.as_str()) {
            row.add(
                egui::Image::new(&thumb.texture).fit_to_exact_size(egui::Vec2::new(48.0, 27.0)),
            );
        } else {
            // Same footprint as a thumb: keeps every row 30px tall.
            let (rect, _) =
                row.allocate_exact_size(egui::Vec2::new(48.0, 27.0), egui::Sense::hover());
            row.painter()
                .rect_filled(rect, 2.0, row.visuals().faint_bg_color);
        }
        clicked = row
            .selectable_label(selected, game.title.as_str())
            .clicked();
    });
    clicked
}

/// The depot list: runtime sections, each with collapsible groups. Returns the clicked game.
fn depot_list(state: &DesktopState, ui: &mut egui::Ui) -> Option<usize> {
    let filter = state.depot_filter.trim().to_lowercase();
    let mut pick: Option<usize> = None;
    egui::ScrollArea::vertical()
        .id_salt("depot_list")
        .auto_shrink([false, false])
        .show(ui, |scroll| {
            if state.depot_games.is_empty() {
                scroll.label("no depot found next to the working directory");
            }
            for kind in GameKind::ALL {
                let (label, known) = (kind.label(), kind.groups());
                let in_kind: Vec<usize> = (0..state.depot_games.len())
                    .filter(|&i| {
                        state
                            .depot_games
                            .get(i)
                            .is_some_and(|g| g.kind == kind && matches(g, filter.as_str()))
                    })
                    .collect();
                if in_kind.is_empty() {
                    continue;
                }
                let mut header =
                    egui::CollapsingHeader::new(format!("{label} ({})", in_kind.len()))
                        .id_salt(("depot_kind", label))
                        .default_open(true);
                if !filter.is_empty() {
                    header = header.open(Some(true));
                }
                header.show(scroll, |section| {
                    for group in ordered_groups(state, kind, known) {
                        let members: Vec<usize> = in_kind
                            .iter()
                            .copied()
                            .filter(|&i| state.depot_games.get(i).is_some_and(|g| g.group == group))
                            .collect();
                        if members.is_empty() {
                            continue;
                        }
                        // closed by default, except the group holding the current game; a filter opens all
                        let holds_selected =
                            state.selected_game.is_some_and(|s| members.contains(&s));
                        let mut sub =
                            egui::CollapsingHeader::new(format!("{group} ({})", members.len()))
                                .id_salt(("depot_group", label, group.as_str()))
                                .default_open(holds_selected);
                        if !filter.is_empty() {
                            sub = sub.open(Some(true));
                        }
                        sub.show(section, |list| {
                            for &index in &members {
                                if depot_row(state, list, index) {
                                    pick = Some(index);
                                }
                            }
                        });
                    }
                });
            }
            if !filter.is_empty()
                && !state
                    .depot_games
                    .iter()
                    .any(|g| matches(g, filter.as_str()))
            {
                scroll.weak("nothing matches");
            }
        });
    pick
}

/// The resizable left panel; the app and the tests share this one definition.
pub(crate) fn left_panel(state: &mut DesktopState, ui: &mut egui::Ui) {
    egui::containers::Panel::left(("scene_tree", state.layout_gen))
        .resizable(true)
        .default_size(240.0)
        .size_range(160.0..=600.0)
        .show(ui, |inner| left_tree(state, inner));
}

/// Left panel: depot quick-select (runtime, then collapsible groups) above the scene tree.
pub(crate) fn left_tree(state: &mut DesktopState, ui: &mut egui::Ui) {
    egui::containers::Panel::bottom(("scene_split", state.layout_gen))
        .resizable(true)
        .default_size(200.0)
        .size_range(60.0..=f32::INFINITY)
        .show(ui, |pane| scene_section(state, pane));
    ui.heading(format!("Depot ({})", state.depot_games.len()));
    crate::library::filter_box(state, ui, 170.0);
    let can_start = state.selected_game.is_some();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(can_start, egui::Button::new("Play"))
            .on_hover_text("Launch the game with you in control")
            .clicked()
        {
            state.start_game();
        }
        if ui
            .add_enabled(can_start, egui::Button::new("Watch demo"))
            .on_hover_text("Launch with the built-in autoplay driving")
            .clicked()
        {
            state.start_game_demo();
        }
    });
    ui.separator();
    if let Some(index) = depot_list(state, ui) {
        state.load_depot(index);
    }
}

/// Scene tree section, shown in the draggable bottom pane of the left panel.
fn scene_section(state: &mut DesktopState, ui: &mut egui::Ui) {
    ui.heading("Scene");
    ui.separator();
    egui::ScrollArea::both()
        .id_salt("scene_tree")
        .auto_shrink([false, false])
        .show(ui, |scroll| {
            if let Some(root) = state.graph.root {
                show_node(state, scroll, root, 0.0);
            } else {
                scroll.label("empty scene");
            }
        });
}

/// One tree row plus its children, indented.
fn show_node(state: &mut DesktopState, ui: &mut egui::Ui, id: NodeId, indent: f32) {
    let (name, children, badge) = match state.graph.get(id) {
        Some(node) => {
            let path = state.graph.node_path(id).unwrap_or_else(|| "?".to_string());
            let count = state.badges().get(&path).copied().unwrap_or(0);
            (node.name.clone(), node.children.clone(), count)
        }
        None => return,
    };
    ui.horizontal(|row| {
        row.add_space(indent);
        row.selectable_value(&mut state.selected, Some(id), name.as_str());
        if badge > 0 {
            row.colored_label(egui::Color32::from_rgb(255, 90, 90), format!("[{badge}]"));
        }
    });
    for child in children {
        show_node(state, ui, child, indent + 12.0);
    }
}

/// Right panel: inspector for the selection plus the live lint list.
pub(crate) fn right_panels(state: &mut DesktopState, ui: &mut egui::Ui) {
    egui::containers::Panel::bottom(("findings_split", state.layout_gen))
        .resizable(true)
        .default_size(180.0)
        .size_range(60.0..=f32::INFINITY)
        .show(ui, |pane| {
            pane.heading(format!("Findings ({})", state.diags.len()));
            show_lint_list(state, pane);
        });
    ui.heading("Inspector");
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("inspector_scroll")
        .auto_shrink([false, false])
        .show(ui, |scroll| show_inspector(state, scroll));
}

/// Position/scale editing (via commands) plus read-only properties.
fn show_inspector(state: &mut DesktopState, ui: &mut egui::Ui) {
    let Some(id) = state.selected else {
        ui.label("nothing selected");
        return;
    };
    let snapshot = match state.graph.get(id) {
        Some(node) => {
            let props: Vec<(String, String)> = node
                .properties
                .iter()
                .map(|(k, v)| {
                    let value = match v {
                        gamecraft_core::PropertyValue::Raw(text) => text.clone(),
                        other => format!("{other:?}"),
                    };
                    (k.clone(), value)
                })
                .collect();
            Some((
                node.name.clone(),
                node.node_type.clone(),
                node.transform,
                props,
            ))
        }
        None => None,
    };
    let Some((name, kind, transform, mut props)) = snapshot else {
        ui.label("selection is stale");
        return;
    };
    let path = state.graph.node_path(id).unwrap_or_else(|| "?".to_string());
    ui.label(format!("{name} ({kind})"));
    ui.monospace(path.as_str());
    let mut position = transform.position;
    let mut scale = transform.scale;
    let rotation = transform.rotation;
    let mut dirty = false;
    ui.horizontal(|row| {
        row.label("pos");
        dirty |= row
            .add(egui::DragValue::new(&mut position.x).speed(0.1))
            .changed();
        dirty |= row
            .add(egui::DragValue::new(&mut position.y).speed(0.1))
            .changed();
        dirty |= row
            .add(egui::DragValue::new(&mut position.z).speed(0.1))
            .changed();
    });
    ui.horizontal(|row| {
        row.label("scl");
        dirty |= row
            .add(egui::DragValue::new(&mut scale.x).speed(0.05))
            .changed();
        dirty |= row
            .add(egui::DragValue::new(&mut scale.y).speed(0.05))
            .changed();
        dirty |= row
            .add(egui::DragValue::new(&mut scale.z).speed(0.05))
            .changed();
    });
    ui.monospace(format!(
        "rot xyzw {:.2} {:.2} {:.2} {:.2}",
        rotation.x, rotation.y, rotation.z, rotation.w
    ));
    if dirty {
        state.apply_transform(
            id,
            gamecraft_core::Transform {
                position,
                rotation,
                scale,
            },
        );
    }
    props.sort();
    for (key, value) in props {
        ui.monospace(format!("{key} = {value}"));
    }
}

/// Live [`scene_lint`](gamecraft_mcp::scene_lint) output; click selects.
fn show_lint_list(state: &mut DesktopState, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |scroll| {
            for diag in state.diags.clone() {
                let label = format!("[{}] {} @ {}", diag.code, diag.message, diag.node);
                let widget = egui::Button::new(egui::RichText::new(label).monospace());
                if scroll.add(widget).clicked() {
                    state.select_path(diag.node.as_str());
                }
            }
        });
}

/// Bottom panel: 60-frame trace scrubber.
pub(crate) fn bottom_trace(state: &mut DesktopState, ui: &mut egui::Ui) {
    ui.horizontal(|row| {
        row.label("trace");
        row.checkbox(&mut state.recording, "record");
        row.checkbox(&mut state.follow, "follow");
        if row.button("<").clicked() {
            state.scrub_step = state.scrub_step.saturating_sub(1);
        }
        let max = state.scrub_max();
        if state.scrub_step > max {
            state.scrub_step = max;
        }
        row.add(egui::DragValue::new(&mut state.scrub_step).range(0..=max));
        if row.button(">").clicked() {
            state.scrub_step = state.scrub_step.saturating_add(1).min(max);
        }
        row.label(format!("{} frames", state.trace.len()));
        if let Some(entry) = state.scrubbed() {
            row.monospace(format!(
                "yaw {:.1} pitch {:.1} findings {} [{}]",
                entry.yaw, entry.pitch, entry.findings, entry.selected
            ));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    /// Top edge of the "Scene" heading, which sits at the top of the draggable pane.
    fn scene_top(harness: &egui_kittest::Harness<'_, DesktopState>) -> f32 {
        harness.get_by_label("Scene").rect().min.y
    }

    #[test]
    fn scene_divider_drags_and_reset_restores() {
        let mut state = DesktopState::from_text(
            "[gd_scene format=3]\n\n[node name=\"Root\" type=\"Node3D\"]\n",
            "uid://divider",
        );
        state.library = false;
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(900.0, 700.0))
            .build_ui_state(
                |ui, s: &mut DesktopState| {
                    left_panel(s, ui);
                    egui::containers::CentralPanel::default().show(ui, |_| {});
                },
                state,
            );
        harness.run();
        let before = scene_top(&harness);
        let x = harness.get_by_label("Scene").rect().min.x + 20.0;
        let edge = egui::pos2(x, before - 9.0);
        harness.hover_at(edge);
        harness.run();
        harness.drag_at(edge);
        harness.run();
        harness.hover_at(egui::pos2(x, edge.y - 120.0));
        harness.run();
        harness.drop_at(egui::pos2(x, edge.y - 120.0));
        harness.run();
        let after = scene_top(&harness);
        assert!(
            before - after > 80.0,
            "divider drag should grow the scene pane: before {before}, after {after}"
        );
        harness.state_mut().layout_gen += 1;
        harness.run();
        let reset = scene_top(&harness);
        assert!(
            (reset - before).abs() < 2.0,
            "reset layout should restore the default: {reset} vs {before}"
        );
    }
}

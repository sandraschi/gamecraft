//! Library mode: a card grid of every game and a detail card for the selected one.
//!
//! Keyboard: `/` focuses the filter, arrows move the selection, Enter plays,
//! Esc in the filter clears it. Double-click a card to play.

use crate::state::{DepotGame, DesktopState, GameKind, ACCENT, ACCENT_DARK};

/// Card width in points (thumbnail is 16:9 inside it).
const CARD_W: f32 = 196.0;
/// Thumbnail height for [`CARD_W`].
const THUMB_H: f32 = 110.0;

/// Stable id of the game filter, shared by the library header and the editor sidebar.
pub(crate) fn filter_id() -> egui::Id {
    egui::Id::unique("gamecraft_depot_filter")
}

/// The filter box with its clear button.
pub(crate) fn filter_box(state: &mut DesktopState, ui: &mut egui::Ui, width: f32) {
    ui.horizontal(|row| {
        row.add(
            egui::TextEdit::singleline(&mut state.depot_filter)
                .id(filter_id())
                .hint_text("filter games  ( / )")
                .desired_width(width),
        );
        if !state.depot_filter.is_empty() && row.small_button("x").clicked() {
            state.depot_filter.clear();
        }
    });
}

/// Accent that reads on the current theme.
fn accent(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode {
        ACCENT
    } else {
        ACCENT_DARK
    }
}

/// Short gate status text and its colour.
fn status(game: &DepotGame, dark: bool) -> (String, egui::Color32) {
    let (good, bad, none) = if dark {
        (
            egui::Color32::from_rgb(110, 210, 130),
            egui::Color32::from_rgb(255, 110, 100),
            egui::Color32::from_rgb(150, 156, 170),
        )
    } else {
        (
            egui::Color32::from_rgb(30, 130, 60),
            egui::Color32::from_rgb(190, 40, 30),
            egui::Color32::from_rgb(110, 112, 120),
        )
    };
    match &game.verify {
        Some(v) if v.pass => (format!("passed  {}", v.progress), good),
        Some(v) => (
            format!("failed  {}", v.failed.clone().unwrap_or_default()),
            bad,
        ),
        None => ("never verified".to_string(), none),
    }
}

/// Thumbnail, or a quiet placeholder with the slug, at an exact size.
fn thumb(state: &DesktopState, ui: &mut egui::Ui, game: &DepotGame, size: egui::Vec2) {
    if let Some(t) = state.thumbs.get(game.slug.as_str()) {
        ui.add(
            egui::Image::new(&t.texture)
                .fit_to_exact_size(size)
                .corner_radius(4.0),
        );
        return;
    }
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, ui.visuals().faint_bg_color);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        game.slug.as_str(),
        egui::FontId::proportional(13.0),
        ui.visuals().weak_text_color(),
    );
}

/// Thumbnail, title and gate line of one card.
fn card_body(state: &DesktopState, inner: &mut egui::Ui, game: &DepotGame, dark: bool) {
    inner.set_width(CARD_W);
    inner.spacing_mut().item_spacing.y = 4.0;
    thumb(state, inner, game, egui::vec2(CARD_W, THUMB_H));
    inner.add(egui::Label::new(egui::RichText::new(game.title.as_str()).size(15.5)).truncate());
    let (text, colour) = status(game, dark);
    inner.horizontal(|row| {
        row.label(egui::RichText::new(game.kind.as_str()).small().weak());
        row.add(egui::Label::new(egui::RichText::new(text).small().color(colour)).truncate());
    });
}

/// One card. Returns (clicked, double-clicked).
fn card(state: &DesktopState, ui: &mut egui::Ui, index: usize) -> (bool, bool) {
    let Some(game) = state.depot_games.get(index) else {
        return (false, false);
    };
    let selected = state.selected_game == Some(index);
    let dark = ui.visuals().dark_mode;
    let stroke = if selected {
        egui::Stroke::new(1.5, accent(ui))
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    };
    let frame = egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(stroke)
        .corner_radius(6.0)
        .inner_margin(6.0);
    let response = frame
        .show(ui, |outer| {
            // Grid rows are top-aligned horizontals; the card itself stacks vertically.
            outer.allocate_ui_with_layout(
                egui::vec2(CARD_W, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |inner| card_body(state, inner, game, dark),
            );
        })
        .response
        .interact(egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if selected && state.scroll_to_selected {
        response.scroll_to_me(None);
    }
    (response.clicked(), response.double_clicked())
}

/// Central panel in library mode: filter header, then the grouped card grid.
pub(crate) fn grid(state: &mut DesktopState, ui: &mut egui::Ui) {
    ui.horizontal(|row| {
        row.heading(format!("Library ({})", state.depot_games.len()));
        row.add_space(12.0);
        filter_box(state, row, 220.0);
    });
    ui.add_space(4.0);
    let sections = state.visible_sections();
    let mut pick: Option<(usize, bool)> = None;
    egui::ScrollArea::vertical()
        .id_salt("library_grid")
        .auto_shrink([false, false])
        .show(ui, |scroll| {
            if state.depot_games.is_empty() {
                scroll
                    .label("No games found. Run inside the GameCraft repo or set GAMECRAFT_ROOT.");
            } else if sections.is_empty() {
                scroll.weak("nothing matches the filter");
            }
            let mut last_kind: Option<GameKind> = None;
            for (kind, group, members) in &sections {
                if last_kind != Some(*kind) {
                    scroll.add_space(6.0);
                    scroll.label(egui::RichText::new(kind.label()).size(18.0).strong());
                    last_kind = Some(*kind);
                }
                scroll.label(egui::RichText::new(group.as_str()).color(accent(scroll)));
                // Top-aligned rows of fixed-width cards. horizontal_wrapped
                // vertically centers each card in the row height established
                // so far, so groups with 2+ cards stairstepped down-right.
                let per = CARD_W + 12.0 + 10.0;
                // Whole cards that fit, at least one (no float-to-int cast).
                let avail = scroll.available_width();
                let mut cols = 1_usize;
                while f32::from(u16::try_from(cols + 1).unwrap_or(u16::MAX)) * per <= avail {
                    cols += 1;
                }
                for (ri, chunk) in members.chunks(cols).enumerate() {
                    if ri > 0 {
                        scroll.add_space(6.0);
                    }
                    scroll.horizontal_top(|row| {
                        row.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
                        for &index in chunk {
                            let (clicked, double) = card(state, row, index);
                            if double {
                                pick = Some((index, true));
                            } else if clicked {
                                pick = Some((index, false));
                            }
                        }
                    });
                }
                scroll.add_space(8.0);
            }
        });
    state.scroll_to_selected = false;
    if let Some((index, play)) = pick {
        if state.selected_game != Some(index) {
            state.load_depot(index);
        }
        if play {
            state.start_game();
        }
    }
}

/// Right panel in library mode: the selected game in detail.
pub(crate) fn detail(state: &mut DesktopState, ui: &mut egui::Ui) {
    ui.set_min_width(330.0);
    let Some(game) = state
        .selected_game
        .and_then(|i| state.depot_games.get(i))
        .cloned()
    else {
        ui.add_space(12.0);
        ui.weak("Pick a game. Arrows move, Enter plays, / filters.");
        return;
    };
    ui.heading(game.title.as_str());
    ui.label(
        egui::RichText::new(format!("{} / {}", game.kind.as_str(), game.group)).color(accent(ui)),
    );
    ui.add_space(4.0);
    let width = ui.available_width().min(420.0);
    thumb(state, ui, &game, egui::vec2(width, width * 9.0 / 16.0));
    ui.add_space(4.0);
    detail_buttons(state, ui);
    ui.separator();
    let (text, colour) = status(&game, ui.visuals().dark_mode);
    ui.horizontal(|row| {
        row.label("Last gate:");
        row.colored_label(colour, text);
    });
    if let Some(v) = &game.verify {
        if !v.when.is_empty() {
            ui.small(format!("run {}", v.when));
        }
        if let Some(st) = &v.self_test {
            ui.small(format!("self-test: {st}"));
        }
    }
    if !game.description.is_empty() {
        ui.add_space(4.0);
        ui.label(game.description.as_str());
    }
    // The sheet's own `# Title: cheat sheet` line repeats the heading above.
    let sheet = state.cheatsheet().map(|text| {
        text.lines()
            .filter(|line| !line.starts_with("# "))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    });
    egui::ScrollArea::vertical()
        .id_salt("detail_sheet")
        .auto_shrink([false, true])
        .show(ui, |scroll| {
            if let Some(text) = sheet {
                egui::CollapsingHeader::new("Cheat sheet")
                    .id_salt("detail_cheatsheet")
                    .default_open(true)
                    .show(scroll, |body| {
                        body.add(
                            egui::Label::new(egui::RichText::new(text).monospace().size(12.5))
                                .wrap(),
                        );
                    });
            }
            scroll.weak(game.dir.display().to_string());
        });
}

/// Play / Watch demo / Run tests / Open folder.
fn detail_buttons(state: &mut DesktopState, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|row| {
        if row
            .button(egui::RichText::new("Play").strong())
            .on_hover_text("Launch with you in control (Enter, or double-click the card)")
            .clicked()
        {
            state.start_game();
        }
        if row
            .button("Watch demo")
            .on_hover_text("Launch with the built-in autoplay driving")
            .clicked()
        {
            state.start_game_demo();
        }
        let running = state.verify_job.is_some();
        let label = if running { "Running..." } else { "Run tests" };
        if row
            .add_enabled(!running, egui::Button::new(label))
            .on_hover_text("Run the headless gate now and record the result")
            .clicked()
        {
            state.run_tests();
        }
        if running {
            row.spinner();
        }
        if row
            .button("Open folder")
            .on_hover_text("Show the project folder")
            .clicked()
        {
            state.open_folder();
        }
    });
}

/// Library keyboard: `/` focuses the filter, arrows move, Enter plays, Esc in
/// the filter clears it. Runs after the panels so the `/` that focused the
/// filter is not also typed into it.
pub(crate) fn keys(state: &mut DesktopState, ctx: &egui::Context) {
    if state.dialog.is_some() || !state.palette_query.is_empty() {
        return;
    }
    let focused = ctx.memory(egui::Memory::focused);
    let in_filter = focused == Some(filter_id());
    if focused.is_some() && !in_filter {
        return;
    }
    if in_filter {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.depot_filter.clear();
            ctx.memory_mut(|m| m.surrender_focus(filter_id()));
        }
    } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Slash)) {
        ctx.memory_mut(|m| m.request_focus(filter_id()));
        return;
    }
    let (down, up, right, left, enter) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::ArrowRight),
            i.key_pressed(egui::Key::ArrowLeft),
            i.key_pressed(egui::Key::Enter),
        )
    });
    // In the filter, left/right belong to the text cursor.
    let next = down || (right && !in_filter);
    let prev = up || (left && !in_filter);
    if next {
        state.step_selection(1);
    } else if prev {
        state.step_selection(-1);
    }
    if enter && state.library {
        // After typing a filter, Enter plays the first match if the selection is filtered out.
        let visible = state
            .visible_sections()
            .into_iter()
            .any(|(_, _, members)| state.selected_game.is_some_and(|s| members.contains(&s)));
        if !visible {
            state.step_selection(0);
        }
        if state.selected_game.is_some() {
            state.start_game();
        }
    }
}

/// Ctrl + mouse wheel zooms the whole UI (egui turns Ctrl+wheel into a zoom
/// delta, so the viewport's own wheel-zoom does not also fire).
pub(crate) fn wheel_zoom(ctx: &egui::Context) {
    let (delta, command) = ctx.input(|i| (i.zoom_delta(), i.modifiers.command));
    if command && (delta - 1.0).abs() > f32::EPSILON {
        ctx.set_zoom_factor((ctx.zoom_factor() * delta).clamp(0.6, 2.4));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamecraft_mcp::catalog::VerifySummary;

    /// One fixture card with a chosen gate state.
    fn game(
        kind: GameKind,
        slug: &str,
        title: &str,
        group: &str,
        verify: Option<VerifySummary>,
    ) -> DepotGame {
        DepotGame {
            kind,
            slug: slug.to_string(),
            title: title.to_string(),
            group: group.to_string(),
            description: format!("{title}: fixture description for the snapshot."),
            dir: std::path::PathBuf::from(slug),
            verify,
        }
    }

    /// A gate record.
    fn gate(pass: bool, progress: &str) -> VerifySummary {
        VerifySummary {
            pass,
            failed: (!pass).then(|| "stuck at checkpoint 3".to_string()),
            progress: progress.to_string(),
            when: "2026-11-01T00:48:00Z".to_string(),
            self_test: pass.then(|| "pass: 7 figures".to_string()),
        }
    }

    /// Fixed state: no disk discovery, no thumbnails (placeholders render instead).
    fn fixture() -> DesktopState {
        let mut state = DesktopState::from_text(
            "[gd_scene format=3]\n\n[node name=\"Root\" type=\"Node3D\"]\n",
            "uid://snap",
        );
        state.depot_games = vec![
            game(
                GameKind::Godot,
                "pong",
                "Pong",
                "Arcade",
                Some(gate(true, "won at frame 812")),
            ),
            game(GameKind::Godot, "maze", "Maze", "Puzzles", None),
            game(
                GameKind::Web,
                "hoshi",
                "Hoshi",
                "Seek and find",
                Some(gate(true, "5 checkpoints")),
            ),
            game(
                GameKind::Web,
                "where-is-benny",
                "Where is Benny?",
                "Seek and find",
                Some(gate(true, "6 checkpoints")),
            ),
            game(
                GameKind::Web,
                "bento",
                "Bento",
                "Calm puzzles",
                Some(gate(false, "2 checkpoints")),
            ),
        ];
        state.selected_game = Some(3);
        state.sheet = Some((
            "where-is-benny".to_string(),
            "# Where is Benny?: cheat sheet\n\n| What | Value |\n| Click | find the GSD |"
                .to_string(),
        ));
        state
    }

    /// Render the library screen in one theme and compare with the golden PNG.
    fn snapshot(light: bool, name: &str) {
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1080.0, 620.0))
            .build_ui_state(
                |ui, state: &mut DesktopState| {
                    egui::containers::Panel::right("game_detail").show(ui, |ui| detail(state, ui));
                    egui::containers::CentralPanel::default().show(ui, |ui| grid(state, ui));
                },
                fixture(),
            );
        crate::state::apply_theme(&harness.ctx, light);
        harness.run();
        harness.snapshot(name);
    }

    #[test]
    fn library_dark_matches_snapshot() {
        snapshot(false, "library_dark");
    }

    #[test]
    fn library_light_matches_snapshot() {
        snapshot(true, "library_light");
    }
}

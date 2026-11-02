//! `gamecraft-desktop`: `egui`/`wgpu` game library and viewport shell, in-process on L1.
//!
//! Library mode (the default): every game as a card grouped by runtime and
//! category, a detail card with description, last gate result, cheat sheet and
//! Play / Watch demo / Run tests / Open folder. Keyboard: `/` filter, arrows,
//! Enter. Ctrl+wheel zooms the UI; the top bar switches light/dark.
//!
//! Editor mode: left scene tree with lint badges, center `wgpu` viewport with AABB and
//! collision overlays plus frustum, right inspector with the live lint list,
//! bottom 60-frame trace scrubber, top command palette with undo/redo.
//!
//! A gamepad is polled each frame: left stick orbits the camera, D-pad or stick
//! vertical moves the depot selection, A starts the highlighted game and B
//! returns to the scene root. Keyboard and mouse stay fully functional.

mod gamepad;
mod library;
mod mesh;
mod palette;
mod raster;
mod state;
mod textures;
mod view;
mod viewport;

use state::DesktopState;

fn main() {
    // Games and the web server this viewer starts die with it (Windows Job Object).
    let _ = gamecraft_mcp::proc_guard::kill_children_on_exit();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("GameCraft")
            .with_inner_size([980.0, 600.0])
            .with_position([8.0, 8.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    if let Err(err) = eframe::run_native(
        "GameCraft",
        options,
        Box::new(|ctx| Ok(Box::new(DesktopState::load(ctx)))),
    ) {
        eprintln!("gamecraft-desktop failed: {err}");
        std::process::exit(1);
    }
}

impl DesktopState {
    /// Apply one polled gamepad frame to the shell.
    fn apply_gamepad(&mut self, pad: &gamepad::PadFrame) {
        if pad.confirm {
            self.start_game();
        }
        if pad.back {
            self.selected = self.graph.root;
        }
        if pad.next_item || pad.prev_item {
            let count = self.depot_games.len();
            if count > 0 {
                let count_i = i64::try_from(count).unwrap_or(1);
                let current = i64::try_from(self.selected_game.unwrap_or(0)).unwrap_or(0);
                let step = i64::from(pad.next_item) - i64::from(pad.prev_item);
                let next = (current + step).rem_euclid(count_i);
                self.load_depot(usize::try_from(next).unwrap_or(0));
            }
        }
        if pad.stick_x != 0.0 {
            self.cam_yaw -= pad.stick_x * 0.02;
        }
    }
}

impl eframe::App for DesktopState {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        view::top_bar(self, ui);
        let generation = self.layout_gen;
        if self.library {
            egui::containers::Panel::right(("game_detail", generation))
                .resizable(true)
                .default_size(300.0)
                .size_range(200.0..=700.0)
                .show(ui, |ui| {
                    library::detail(self, ui);
                });
            egui::containers::CentralPanel::default().show(ui, |ui| {
                library::grid(self, ui);
            });
        } else {
            view::left_panel(self, ui);
            egui::containers::Panel::right(("inspector", generation))
                .resizable(true)
                .default_size(280.0)
                .size_range(180.0..=600.0)
                .show(ui, |ui| {
                    view::right_panels(self, ui);
                });
            egui::containers::Panel::bottom("trace_bar").show(ui, |ui| {
                view::bottom_trace(self, ui);
            });
            egui::containers::CentralPanel::default().show(ui, |ui| {
                viewport::draw_viewport(self, ui);
            });
        }
        view::dialogs(self, ui.ctx());
        library::keys(self, ui.ctx());
        palette::keys(self, ui.ctx());
        palette::window(self, ui.ctx());
        library::wheel_zoom(ui.ctx());
        self.poll_verify();
        if self.verify_job.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        }
        if !self.library {
            self.refresh();
            self.record_frame();
        }

        let pad = self.gamepad.poll();
        if pad.connected {
            self.apply_gamepad(&pad);
            // Without this the shell sleeps and the next poll is stale.
            ui.ctx().request_repaint();
        }

        if self.recording {
            ui.ctx().request_repaint();
        }
    }
}

//! `gamecraft-desktop`: `egui`/`wgpu` viewport shell, in-process on L1.
//!
//! Left scene tree with lint badges, center `wgpu` viewport with AABB and
//! collision overlays plus frustum, right inspector with the live lint list,
//! bottom 60-frame trace scrubber, top command palette with undo/redo.
//!
//! A gamepad is polled each frame: left stick orbits the camera, D-pad or stick
//! vertical moves the depot selection, A starts the highlighted game and B
//! returns to the scene root. Keyboard and mouse stay fully functional.

mod gamepad;
mod mesh;
mod state;
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
        egui::containers::Panel::left("scene_tree").show(ui, |ui| {
            view::left_tree(self, ui);
        });
        egui::containers::Panel::right("inspector").show(ui, |ui| {
            view::right_panels(self, ui);
        });
        egui::containers::Panel::bottom("trace_bar").show(ui, |ui| {
            view::bottom_trace(self, ui);
        });
        egui::containers::CentralPanel::default().show(ui, |ui| {
            viewport::draw_viewport(self, ui);
        });
        view::dialogs(self, ui.ctx());
        self.refresh();
        self.record_frame();

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

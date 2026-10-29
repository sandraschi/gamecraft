//! `gamecraft-desktop`: `egui`/`wgpu` viewport shell, in-process on L1.
//!
//! Left scene tree with lint badges, center `wgpu` viewport with AABB and
//! collision overlays plus frustum, right inspector with the live lint list,
//! bottom 60-frame trace scrubber, top command palette with undo/redo.

mod mesh;
mod state;
mod view;
mod viewport;

use state::DesktopState;

fn main() {
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
        if self.recording {
            ui.ctx().request_repaint();
        }
    }
}

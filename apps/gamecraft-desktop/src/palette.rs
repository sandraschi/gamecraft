//! Command palette: every registry command, runnable from the desktop app.
//!
//! The palette calls `gamecraft_mcp::registry::run` in-process, the same function the
//! CLI and the MCP server call, so the GUI cannot disagree with them. Commands run on a
//! worker thread (`game.verify` can take minutes); the window polls for the answer.
//! `Ctrl+Shift+P` toggles the palette and a command's declared shortcut opens it with
//! that command selected.

use crate::state::DesktopState;
use gamecraft_mcp::registry::{self, CommandSpec};
use serde_json::{json, Value};
use std::sync::mpsc::{channel, Receiver, TryRecvError};

/// A command running on a worker thread.
#[derive(Debug)]
pub(crate) struct PaletteJob {
    id: &'static str,
    rx: Receiver<(bool, String)>,
}

/// Palette window state.
#[derive(Debug, Default)]
pub(crate) struct Palette {
    /// Window visible.
    pub open: bool,
    /// Text filter over id, label and menu path.
    pub filter: String,
    /// Selected command id.
    pub selected: Option<&'static str>,
    /// Params as JSON text.
    pub params: String,
    /// Last result: success flag and pretty JSON.
    pub output: Option<(bool, String)>,
    /// Command in flight.
    pub job: Option<PaletteJob>,
}

/// Params object with a placeholder for every required parameter.
pub(crate) fn skeleton(spec: &CommandSpec) -> String {
    let mut map = serde_json::Map::new();
    for a in spec.params.iter().filter(|a| a.required) {
        let value = match (a.choices.first(), a.kind) {
            (Some(first), _) => json!(first),
            (None, "integer") => json!(0),
            (None, "boolean") => json!(false),
            (None, "object") => json!({}),
            _ => json!(""),
        };
        map.insert(a.name.to_string(), value);
    }
    serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_else(|_| "{}".into())
}

/// Run a command from JSON text, exactly as `run --cmd` would. Returns success and the
/// pretty-printed flat result object.
pub(crate) fn execute(id: &str, params_text: &str) -> (bool, String) {
    let params: Value = if params_text.trim().is_empty() {
        json!({})
    } else {
        match serde_json::from_str(params_text) {
            Ok(v) => v,
            Err(e) => {
                let v =
                    json!({"success": false, "error": "params_not_json", "message": e.to_string()});
                return (false, pretty(&v));
            }
        }
    };
    let outcome = registry::run(id, &params);
    let ok = outcome.is_ok();
    (ok, pretty(&registry::outcome_json(&outcome)))
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

impl Palette {
    /// Select a command and prefill its params.
    pub(crate) fn select(&mut self, spec: &'static CommandSpec) {
        self.selected = Some(spec.id);
        self.params = skeleton(spec);
        self.output = None;
    }

    /// Start the selected command on a worker thread.
    pub(crate) fn run_selected(&mut self) {
        let Some(id) = self.selected else {
            return;
        };
        if self.job.is_some() {
            return;
        }
        let text = self.params.clone();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(execute(id, &text));
        });
        self.output = None;
        self.job = Some(PaletteJob { id, rx });
    }

    /// Pick up a finished job. Returns true while one is still running.
    pub(crate) fn poll(&mut self) -> bool {
        let Some(job) = &self.job else {
            return false;
        };
        match job.rx.try_recv() {
            Ok(result) => {
                self.output = Some(result);
                self.job = None;
                false
            }
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => {
                self.output = Some((false, "the command thread ended without a result".into()));
                self.job = None;
                false
            }
        }
    }
}

/// `Ctrl+X` style shortcut from the registry as an egui key.
fn parse_shortcut(text: &str) -> Option<(egui::Modifiers, egui::Key)> {
    let mut mods = egui::Modifiers::NONE;
    let mut key = None;
    for part in text.split('+') {
        match part.trim() {
            "Ctrl" | "Cmd" => mods.command = true,
            "Shift" => mods.shift = true,
            "Alt" => mods.alt = true,
            other => key = egui::Key::from_name(other),
        }
    }
    key.map(|k| (mods, k))
}

/// Keyboard: `Ctrl+Shift+P` toggles, registry shortcuts open the palette on their command.
pub(crate) fn keys(state: &mut DesktopState, ctx: &egui::Context) {
    let toggle = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
    if ctx.input_mut(|i| i.consume_key(toggle, egui::Key::P)) {
        state.palette.open = !state.palette.open;
        return;
    }
    for spec in registry::COMMANDS {
        let Some((mods, key)) = spec.shortcut.and_then(parse_shortcut) else {
            continue;
        };
        if ctx.input_mut(|i| i.consume_key(mods, key)) {
            state.palette.open = true;
            state.palette.select(spec);
            return;
        }
    }
}

fn matches(spec: &CommandSpec, filter: &str) -> bool {
    let f = filter.trim().to_lowercase();
    f.is_empty()
        || spec.id.contains(&f)
        || spec.label.to_lowercase().contains(&f)
        || spec.menu.to_lowercase().contains(&f)
}

/// The palette window.
pub(crate) fn window(state: &mut DesktopState, ctx: &egui::Context) {
    let running = state.palette.poll();
    if running {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    if !state.palette.open {
        return;
    }
    let mut open = true;
    egui::Window::new("Command palette")
        .open(&mut open)
        .default_size([720.0, 460.0])
        .show(ctx, |ui| body(&mut state.palette, ui, running));
    state.palette.open = open;
}

fn body(p: &mut Palette, ui: &mut egui::Ui, running: bool) {
    let filter = p.filter.clone();
    ui.horizontal(|ui| {
        let label = ui.label("Filter");
        ui.text_edit_singleline(&mut p.filter).labelled_by(label.id);
    });
    ui.separator();
    ui.columns(2, |cols| {
        egui::ScrollArea::vertical()
            .id_salt("palette_list")
            .auto_shrink([false, false])
            .show(&mut cols[0], |ui| {
                for spec in registry::COMMANDS.iter().filter(|s| matches(s, &filter)) {
                    let shortcut = spec.shortcut.map_or(String::new(), |k| format!("   {k}"));
                    let label = format!("{}{shortcut}", spec.menu);
                    if ui
                        .selectable_label(p.selected == Some(spec.id), label)
                        .clicked()
                    {
                        p.select(spec);
                    }
                }
            });
        let ui = &mut cols[1];
        let Some(spec) = p.selected.and_then(registry::find) else {
            ui.label("Pick a command on the left.");
            return;
        };
        ui.strong(spec.id);
        ui.label(spec.summary);
        for a in spec.params {
            let req = if a.required { " (required)" } else { "" };
            ui.small(format!("{}: {}{req} - {}", a.name, a.kind, a.help));
        }
        let params_label = ui.label("Params (JSON)");
        let edit = ui.add(
            egui::TextEdit::multiline(&mut p.params)
                .code_editor()
                .desired_rows(5)
                .desired_width(f32::INFINITY),
        );
        edit.labelled_by(params_label.id);
        ui.horizontal(|ui| {
            if ui.add_enabled(!running, egui::Button::new("Run")).clicked() {
                p.run_selected();
            }
            if running {
                ui.spinner();
                ui.label(format!("running {}", p.job.as_ref().map_or("", |j| j.id)));
            }
        });
        if let Some((ok, text)) = &p.output {
            ui.colored_label(
                if *ok {
                    egui::Color32::from_rgb(90, 190, 110)
                } else {
                    egui::Color32::from_rgb(230, 100, 90)
                },
                if *ok { "done" } else { "failed" },
            );
            egui::ScrollArea::vertical()
                .id_salt("palette_out")
                .max_height(180.0)
                .show(ui, |ui| {
                    ui.monospace(text.as_str());
                });
        }
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn gui_execute_equals_registry_run() {
        let cases = [
            ("replay.info", json!({"path": "missing.gcraft"})),
            ("replay.info", json!({"path": "wrong.txt"})),
            ("replay.verify", json!({})),
            ("replay.record", json!({"game": "chess"})),
            ("nope.nothing", json!({})),
        ];
        for (id, params) in cases {
            let (ok, text) = execute(id, &params.to_string());
            let outcome = registry::run(id, &params);
            assert_eq!(ok, outcome.is_ok(), "{id}");
            let via_gui: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(via_gui, registry::outcome_json(&outcome), "{id}");
        }
        let (ok, text) = execute("replay.info", "{not json");
        assert!(!ok);
        assert!(text.contains("params_not_json"));
    }

    #[test]
    fn skeleton_fills_required_params_only() {
        let spec = registry::find("replay.record").unwrap();
        let v: Value = serde_json::from_str(&skeleton(spec)).unwrap();
        assert_eq!(v, json!({"game": "maze"}));
        let spec = registry::find("scene.export").unwrap();
        let v: Value = serde_json::from_str(&skeleton(spec)).unwrap();
        assert_eq!(v, json!({"scene": {}}));
    }

    #[test]
    fn registry_shortcuts_parse() {
        for spec in registry::COMMANDS {
            if let Some(s) = spec.shortcut {
                assert!(
                    parse_shortcut(s).is_some(),
                    "{} has an unparseable shortcut {s}",
                    spec.id
                );
            }
        }
    }

    #[test]
    fn palette_records_and_verifies_a_replay_through_the_window() {
        let dir = std::env::temp_dir().join(format!("gcraft-palette-{}", std::process::id()));
        let file = dir.join("depot.gcraft");
        let mut state = DesktopState::from_text(
            "[gd_scene format=3]\n\n[node name=\"Root\" type=\"Node3D\"]\n",
            "uid://palette",
        );
        state.palette.open = true;
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(900.0, 600.0))
            .build_ui_state(|ui, s: &mut DesktopState| window(s, ui.ctx()), state);
        harness.run();
        harness.get_by_label_contains("Record").click();
        harness.run();
        let out = file.to_string_lossy().replace('\\', "/");
        harness.state_mut().palette.params =
            json!({"game": "maze", "level": "depot", "out": out}).to_string();
        harness.get_by_label("Run").click();
        for _ in 0..200 {
            harness.step();
            if harness.state().palette.output.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let (ok, text) = harness.state().palette.output.clone().unwrap();
        assert!(ok, "{text}");
        assert!(text.contains("\"status\": \"won\""), "{text}");
        let (ok, text) = execute("replay.verify", &json!({"path": out}).to_string());
        assert!(ok, "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

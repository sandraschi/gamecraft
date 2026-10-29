//! Gamepad input for the desktop viewer.
//!
//! `gilrs` reports state rather than deltas, so this wrapper adds the two things
//! a UI needs and a raw pad does not provide: edge detection for buttons and a
//! repeat timer for menu navigation. Everything degrades to "not connected" if
//! the subsystem fails to start, so a missing pad never blocks the keyboard.
//!
//! Mapping (fixed, matching the in-game bindings):
//! - left stick / D-pad vertical -> move the depot selection
//! - left stick horizontal       -> orbit the viewport camera
//! - South (A)                   -> start the selected game
//! - East (B)                    -> back to the scene root

use gilrs::{Axis, Button, GamepadId, Gilrs};
use std::time::{Duration, Instant};

/// Stick magnitude below which input is ignored.
const DEADZONE: f32 = 0.35;
/// Hold-to-repeat interval for menu navigation.
const REPEAT: Duration = Duration::from_millis(260);

/// One poll of the pad, with edges already resolved.
///
/// A per-frame input snapshot, not configuration: several independent booleans
/// is the honest shape here, and grouping them would only obscure the call site.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, Clone, Copy)]
pub struct PadFrame {
    /// A pad is present and reported state this frame.
    pub connected: bool,
    /// Left stick horizontal, deadzoned (`-1.0..1.0`).
    pub stick_x: f32,
    /// South button pressed this frame (edge).
    pub confirm: bool,
    /// East button pressed this frame (edge).
    pub back: bool,
    /// Selection moved up (edge, with repeat).
    pub prev_item: bool,
    /// Selection moved down (edge, with repeat).
    pub next_item: bool,
}

/// Owns the `gilrs` context and the edge/repeat state between frames.
pub struct Gamepad {
    gilrs: Option<Gilrs>,
    held_confirm: bool,
    held_back: bool,
    last_nav: Option<Instant>,
    status: String,
}

impl Gamepad {
    /// Create the context. A failure is recorded, not fatal.
    #[must_use]
    pub fn new() -> Self {
        match Gilrs::new() {
            Ok(gilrs) => Self {
                gilrs: Some(gilrs),
                held_confirm: false,
                held_back: false,
                last_nav: None,
                status: "gamepad: ready".to_string(),
            },
            Err(err) => Self {
                gilrs: None,
                held_confirm: false,
                held_back: false,
                last_nav: None,
                status: format!("gamepad unavailable: {err}"),
            },
        }
    }

    /// Poll the pad and resolve edges.
    pub fn poll(&mut self) -> PadFrame {
        let Some(gilrs) = self.gilrs.as_mut() else {
            return PadFrame::default();
        };
        // Drain events so `is_pressed`/`value` reflect the current state. Not
        // reading them makes gilrs report stale values.
        while gilrs.next_event().is_some() {}

        let id: Option<GamepadId> = gilrs.gamepads().next().map(|(id, _)| id);
        let Some(id) = id else {
            self.status = "gamepad: none connected".to_string();
            return PadFrame::default();
        };

        let (stick_x, stick_y, south, east, d_up, d_down, name) = {
            let pad = gilrs.gamepad(id);
            (
                deadzone(pad.value(Axis::LeftStickX)),
                deadzone(pad.value(Axis::LeftStickY)),
                pad.is_pressed(Button::South),
                pad.is_pressed(Button::East),
                pad.is_pressed(Button::DPadUp),
                pad.is_pressed(Button::DPadDown),
                pad.name().to_string(),
            )
        };

        self.status = format!("gamepad: {name}");

        let frame = PadFrame {
            connected: true,
            stick_x,
            confirm: south && !self.held_confirm,
            back: east && !self.held_back,
            prev_item: false,
            next_item: false,
        };
        self.held_confirm = south;
        self.held_back = east;

        let up = d_up || stick_y < -DEADZONE;
        let down = d_down || stick_y > DEADZONE;
        let direction: i8 = match (up, down) {
            (true, false) => -1,
            (false, true) => 1,
            _ => 0,
        };
        let mut frame = frame;
        if direction == 0 {
            self.last_nav = None;
        } else {
            let now = Instant::now();
            let due = match self.last_nav {
                None => true,
                Some(last) => now.duration_since(last) >= REPEAT,
            };
            if due {
                self.last_nav = Some(now);
                if direction < 0 {
                    frame.prev_item = true;
                } else {
                    frame.next_item = true;
                }
            }
        }
        frame
    }
}

// The gilrs context is an opaque handle with no Debug impl; every other
// field is reported so a failing pad can still be diagnosed from a log.
#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for Gamepad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gamepad")
            .field("status", &self.status)
            .finish()
    }
}

impl Default for Gamepad {
    fn default() -> Self {
        Self::new()
    }
}

/// Zero small stick drift so a resting pad never moves the camera.
fn deadzone(value: f32) -> f32 {
    if value.abs() < DEADZONE {
        0.0
    } else {
        value
    }
}

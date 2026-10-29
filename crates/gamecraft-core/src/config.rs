//! Persistent app configuration and per-game play overlays.
//!
//! One config file feeds every entry point: the CLI reads it for provider,
//! model, endpoints and keys, and the desktop viewer edits it through the
//! Settings dialog. A game overlay is a second, smaller file that lives next
//! to the game as `settings.json` and is read by the shared game runtime
//! at startup, so it applies to all fifteen depot games without
//! regenerating any of them.
//!
//! Precedence for every key-bearing field: explicit CLI flag > config file >
//! environment variable > built-in default. Secrets are never logged; the file
//! itself is a plain JSON document in the user's profile, so it inherits the
//! profile's permissions.
//!
//! Stays WASM-safe: filesystem access goes through `std` only, no OS bindings.

use std::path::{Path, PathBuf};

/// Location of the user-level config file.
#[must_use]
pub fn config_path() -> Option<PathBuf> {
    let home = std::env::var("USERPROFILE")
        .ok()
        .or_else(|| std::env::var("HOME").ok())?;
    Some(PathBuf::from(home).join(".gamecraft").join("config.json"))
}

/// Everything the viewer Settings dialog edits and the CLI consumes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// `auto` | `local` | `openai` | `anthropic`.
    pub provider: String,
    /// Model id; empty means auto-pick (gemma4 family preferred).
    pub model: String,
    /// Base URL for a local server. Empty means the provider default.
    pub ollama_endpoint: String,
    /// Cloud key; empty means the environment variable.
    pub openai_api_key: String,
    /// Cloud key; empty means the environment variable.
    pub anthropic_api_key: String,
    /// `godot` binary override (empty means `godot`).
    pub godot_binary: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider: "auto".to_string(),
            model: String::new(),
            ollama_endpoint: String::new(),
            openai_api_key: String::new(),
            anthropic_api_key: String::new(),
            godot_binary: String::new(),
        }
    }
}

impl AppConfig {
    /// Read the config file, or the defaults when it is missing or invalid.
    /// A corrupt file falls back silently: a broken settings file must never
    /// wedge the tool.
    #[must_use]
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::default();
        };
        let Ok(text) = std::fs::read_to_string(path.as_path()) else {
            return Self::default();
        };
        serde_json::from_str(text.as_str()).unwrap_or_default()
    }

    /// Write the config file, creating `~/.gamecraft` when needed.
    ///
    /// # Errors
    ///
    /// Returns a message when the directory or file cannot be written.
    pub fn save(&self) -> Result<(), String> {
        let Some(path) = config_path() else {
            return Err("no home directory for the config file".to_string());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|err| err.to_string())?;
        std::fs::write(path.as_path(), text).map_err(|err| err.to_string())
    }

    /// Config file, else the environment. The key itself is never printed.
    #[must_use]
    pub fn openai_key(&self) -> Option<String> {
        if self.openai_api_key.trim().is_empty() {
            std::env::var("OPENAI_API_KEY").ok()
        } else {
            Some(self.openai_api_key.clone())
        }
    }

    /// Config file, else the environment.
    #[must_use]
    pub fn anthropic_key(&self) -> Option<String> {
        if self.anthropic_api_key.trim().is_empty() {
            std::env::var("ANTHROPIC_API_KEY").ok()
        } else {
            Some(self.anthropic_api_key.clone())
        }
    }

    /// Model id when the user set one, else `None` (auto-pick).
    #[must_use]
    pub fn effective_model(&self) -> Option<&str> {
        if self.model.trim().is_empty() {
            None
        } else {
            Some(self.model.as_str())
        }
    }
}

/// Runtime-tunable play overlay, written by the viewer and read by the shared
/// game runtime at startup. Multiplicative so one file works for every
/// game regardless of its baked-in base values.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct GameOverlay {
    /// Player speed multiplier.
    pub speed_mult: f64,
    /// Gravity multiplier.
    pub gravity_mult: f64,
    /// Jump impulse multiplier.
    pub jump_mult: f64,
    /// `None` means "whatever the environment says" (the verifier and the
    /// demo button set `GAMECRAFT_AUTOPLAY=1`).
    pub autoplay: Option<bool>,
}

impl Default for GameOverlay {
    fn default() -> Self {
        Self {
            speed_mult: 1.0,
            gravity_mult: 1.0,
            jump_mult: 1.0,
            autoplay: None,
        }
    }
}

/// Name of the overlay file inside each game folder.
pub const OVERLAY_FILE: &str = "settings.json";

/// Read the overlay for one game, or the defaults when absent or invalid.
/// A hand-edited file with a typo must not wedge a game launch.
#[must_use]
pub fn load_overlay(dir: &Path) -> GameOverlay {
    let Ok(text) = std::fs::read_to_string(dir.join(OVERLAY_FILE)) else {
        return GameOverlay::default();
    };
    serde_json::from_str(text.as_str()).unwrap_or_default()
}

/// Write the overlay for one game.
///
/// # Errors
///
/// Returns a message when the file cannot be written.
pub fn save_overlay(dir: &Path, overlay: &GameOverlay) -> Result<(), String> {
    let text = serde_json::to_string_pretty(overlay).map_err(|err| err.to_string())?;
    std::fs::write(dir.join(OVERLAY_FILE), text).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_neutral() {
        let overlay = GameOverlay::default();
        assert!((overlay.speed_mult - 1.0).abs() < f64::EPSILON);
        assert_eq!(overlay.autoplay, None);
        let cfg = AppConfig::default();
        assert_eq!(cfg.effective_model(), None);
        assert_eq!(cfg.provider.as_str(), "auto");
    }

    #[test]
    fn empty_config_loads_defaults() {
        let cfg = AppConfig::default();
        assert!(cfg.effective_model().is_none());
    }
}

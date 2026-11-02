//! `.gcraft` replay documents.
//!
//! A replay is a small JSON document: which game, its config (seed and level
//! parameters), the input log, and the result the recording ended with. Nothing about
//! the run is stored beyond the inputs, so [`Replay::verify`] re-simulates it from
//! scratch and compares the outcome and a state fingerprint with the claim. An edited
//! input, a swapped seed or a hand-written result all fail that check.
//!
//! The same shape (config plus an input log keyed by tick) is what a lockstep
//! multiplayer session needs on the wire; this crate is the single-player half of that.
//!
//! Everything read from a file is hostile: sizes, tick counts, counts of actors and
//! coordinates are range-checked before any simulation runs.

mod hash;
pub mod maze;
pub mod murmur;

use maze::{MazeConfig, MazeInput};
use murmur::{FlockSpec, OrderChange};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The `format` field of every document.
pub const FORMAT: &str = "gcraft";

/// The document version this crate reads and writes.
pub const VERSION: u32 = 1;

/// Most ticks a replay may declare (a little over four hours at 60 Hz).
pub const MAX_TICKS: u64 = 1_000_000;

/// Largest file [`Replay::load`] reads.
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Everything that can go wrong with a replay.
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    /// Not valid JSON, or not the expected shape.
    #[error("not a readable .gcraft document: {0}")]
    Parse(String),
    /// Wrong `format` or unsupported `version`.
    #[error("unsupported document: {0}")]
    Unsupported(String),
    /// A field is outside what the sim accepts.
    #[error("invalid replay: {0}")]
    Invalid(String),
    /// Re-simulation disagrees with the recorded result.
    #[error("replay does not verify: {field} claimed {claimed}, simulation gives {actual}")]
    Mismatch {
        /// Which result field differs.
        field: &'static str,
        /// What the file says.
        claimed: String,
        /// What the simulation produced.
        actual: String,
    },
    /// File could not be read or written.
    #[error("io: {0}")]
    Io(String),
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// The win condition was met.
    Won,
    /// The loss condition was met.
    Lost,
    /// Neither: the log ran out first.
    Incomplete,
}

/// Result of a run. `status`, `ticks` and `state_hash` are what [`Replay::verify`] checks;
/// `summary` is informational counters for humans and tools.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    /// How the run ended.
    pub status: Status,
    /// Fixed ticks the run lasted.
    pub ticks: u64,
    /// FNV-1a 64 fingerprint of the final state, 16 hex digits.
    pub state_hash: String,
    /// Game specific counters (moves, bumps, boids alive, ...).
    pub summary: serde_json::Value,
}

/// The game, its config and its inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "game", rename_all = "lowercase")]
pub enum Session {
    /// Grid maze.
    Maze {
        /// Level parameters.
        config: MazeConfig,
        /// Steps keyed by tick.
        inputs: Vec<MazeInput>,
    },
    /// Starling murmuration.
    Murmuration {
        /// Level parameters.
        config: FlockSpec,
        /// Order changes keyed by tick.
        orders: Vec<OrderChange>,
    },
}

/// A `.gcraft` document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Replay {
    /// Always [`FORMAT`].
    pub format: String,
    /// Document version.
    pub version: u32,
    /// Which build wrote it (informational, never checked).
    pub engine: String,
    /// The game and its inputs.
    #[serde(flatten)]
    pub session: Session,
    /// What the recording ended with.
    pub result: Outcome,
}

impl Replay {
    fn wrap(session: Session, result: Outcome) -> Self {
        Self {
            format: FORMAT.to_string(),
            version: VERSION,
            engine: concat!("gamecraft-replay ", env!("CARGO_PKG_VERSION")).to_string(),
            session,
            result,
        }
    }

    /// Record the maze autoplay on `config`.
    ///
    /// # Errors
    /// [`ReplayError::Invalid`] for a bad config or an unsolvable maze.
    pub fn record_maze(config: MazeConfig) -> Result<Self, ReplayError> {
        let (inputs, ticks) = maze::autoplay(&config)?;
        let result = maze::simulate(&config, &inputs, ticks)?;
        Ok(Self::wrap(Session::Maze { config, inputs }, result))
    }

    /// Record the murmuration autopilot for up to `max_ticks` steps.
    ///
    /// # Errors
    /// [`ReplayError::Invalid`] for a bad config or too many ticks.
    pub fn record_murmuration(config: FlockSpec, max_ticks: u32) -> Result<Self, ReplayError> {
        let (orders, ran) = murmur::record_autopilot(&config, max_ticks)?;
        let result = murmur::simulate(&config, &orders, ran)?;
        Ok(Self::wrap(Session::Murmuration { config, orders }, result))
    }

    /// Wrap a hand-made input log: the result is computed by running it for `ticks`.
    ///
    /// # Errors
    /// [`ReplayError::Invalid`] when the session does not pass the sim's range checks.
    pub fn from_session(session: Session, ticks: u64) -> Result<Self, ReplayError> {
        let result = Self::run(&session, ticks)?;
        Ok(Self::wrap(session, result))
    }

    fn run(session: &Session, ticks: u64) -> Result<Outcome, ReplayError> {
        match session {
            Session::Maze { config, inputs } => maze::simulate(config, inputs, ticks),
            Session::Murmuration { config, orders } => {
                let t = u32::try_from(ticks)
                    .map_err(|_| ReplayError::Invalid(format!("ticks {ticks} out of range")))?;
                murmur::simulate(config, orders, t)
            }
        }
    }

    /// Short name of the game: `maze` or `murmuration`.
    #[must_use]
    pub const fn game(&self) -> &'static str {
        match self.session {
            Session::Maze { .. } => "maze",
            Session::Murmuration { .. } => "murmuration",
        }
    }

    /// Run the log again and return what it produces, without comparing.
    ///
    /// # Errors
    /// [`ReplayError::Invalid`] when the document fails the sim's range checks.
    pub fn resimulate(&self) -> Result<Outcome, ReplayError> {
        Self::run(&self.session, self.result.ticks)
    }

    /// Re-simulate and compare status, tick count and state fingerprint with the claim.
    /// Returns the recomputed outcome.
    ///
    /// # Errors
    /// [`ReplayError::Mismatch`] naming the first field that differs, or
    /// [`ReplayError::Invalid`] when the document is out of range.
    pub fn verify(&self) -> Result<Outcome, ReplayError> {
        let actual = self.resimulate()?;
        let claimed = &self.result;
        if actual.status != claimed.status {
            return Err(ReplayError::Mismatch {
                field: "status",
                claimed: format!("{:?}", claimed.status),
                actual: format!("{:?}", actual.status),
            });
        }
        if actual.ticks != claimed.ticks {
            return Err(ReplayError::Mismatch {
                field: "ticks",
                claimed: claimed.ticks.to_string(),
                actual: actual.ticks.to_string(),
            });
        }
        if actual.state_hash != claimed.state_hash {
            return Err(ReplayError::Mismatch {
                field: "state_hash",
                claimed: claimed.state_hash.clone(),
                actual: actual.state_hash,
            });
        }
        Ok(actual)
    }

    /// Parse a document and check its format and version (not the simulation).
    ///
    /// # Errors
    /// [`ReplayError::Parse`] for bad JSON, [`ReplayError::Unsupported`] for another
    /// format or version.
    pub fn from_json(text: &str) -> Result<Self, ReplayError> {
        let r: Self = serde_json::from_str(text).map_err(|e| ReplayError::Parse(e.to_string()))?;
        if r.format != FORMAT {
            return Err(ReplayError::Unsupported(format!(
                "format {:?}, expected {FORMAT:?}",
                r.format
            )));
        }
        if r.version != VERSION {
            return Err(ReplayError::Unsupported(format!(
                "version {}, this build reads version {VERSION}",
                r.version
            )));
        }
        Ok(r)
    }

    /// Compact JSON with a trailing newline.
    ///
    /// # Errors
    /// [`ReplayError::Parse`] if serialisation fails (it does not for these types).
    pub fn to_json(&self) -> Result<String, ReplayError> {
        let mut s = serde_json::to_string(self).map_err(|e| ReplayError::Parse(e.to_string()))?;
        s.push('\n');
        Ok(s)
    }

    /// Read a `.gcraft` file (refuses files over [`MAX_FILE_BYTES`]).
    ///
    /// # Errors
    /// [`ReplayError::Io`], or anything [`Replay::from_json`] returns.
    pub fn load(path: &Path) -> Result<Self, ReplayError> {
        let len = std::fs::metadata(path)
            .map_err(|e| ReplayError::Io(format!("{}: {e}", path.display())))?
            .len();
        if len > MAX_FILE_BYTES {
            return Err(ReplayError::Invalid(format!(
                "{} is {len} bytes, the limit is {MAX_FILE_BYTES}",
                path.display()
            )));
        }
        let text = std::fs::read_to_string(path)
            .map_err(|e| ReplayError::Io(format!("{}: {e}", path.display())))?;
        Self::from_json(&text)
    }

    /// Write a `.gcraft` file, creating parent folders.
    ///
    /// # Errors
    /// [`ReplayError::Io`] when the folder or file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), ReplayError> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)
                .map_err(|e| ReplayError::Io(format!("{}: {e}", dir.display())))?;
        }
        std::fs::write(path, self.to_json()?)
            .map_err(|e| ReplayError::Io(format!("{}: {e}", path.display())))
    }

    /// Header facts without the input log: game, config, input count, result.
    #[must_use]
    pub fn info(&self) -> serde_json::Value {
        let (config, inputs) = match &self.session {
            Session::Maze { config, inputs } => (serde_json::json!(config), inputs.len()),
            Session::Murmuration { config, orders } => (serde_json::json!(config), orders.len()),
        };
        serde_json::json!({
            "format": self.format,
            "version": self.version,
            "engine": self.engine,
            "game": self.game(),
            "config": config,
            "inputs": inputs,
            "result": self.result,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("gcraft-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn maze_autoplay_verifies_and_wins() {
        let r = Replay::record_maze(MazeConfig::depot()).unwrap();
        assert_eq!(r.result.status, Status::Won);
        // 15 x 15 seed 9 has a 32-move solution (measured); step k lands on tick 1 + 6k.
        assert_eq!(r.result.summary["moves"], 32);
        assert_eq!(r.result.summary["bumps"], 0);
        assert_eq!(r.result.ticks, 1 + 6 * 31);
        assert!(r.verify().is_ok());
    }

    #[test]
    fn json_round_trip_is_lossless_and_stable() {
        let r = Replay::record_maze(MazeConfig::depot()).unwrap();
        let text = r.to_json().unwrap();
        let back = Replay::from_json(&text).unwrap();
        assert_eq!(back, r);
        assert_eq!(back.to_json().unwrap_or_default(), text);
        assert!(text.starts_with("{\"format\":\"gcraft\",\"version\":1,"));
    }

    #[test]
    fn murmuration_round_trips_and_verifies() {
        let spec = FlockSpec::small(400);
        let r = Replay::record_murmuration(spec, 900).unwrap();
        assert_eq!(r.game(), "murmuration");
        let text = r.to_json().unwrap();
        let back = Replay::from_json(&text).unwrap();
        assert_eq!(back, r);
        assert!(back.verify().is_ok());
    }

    #[test]
    fn two_recordings_are_identical() {
        let a = Replay::record_murmuration(FlockSpec::small(300), 600);
        let b = Replay::record_murmuration(FlockSpec::small(300), 600);
        assert_eq!(a.ok(), b.ok());
    }

    #[test]
    fn edited_maze_input_is_caught() {
        let mut r = Replay::record_maze(MazeConfig::depot()).unwrap();
        if let Session::Maze { inputs, .. } = &mut r.session {
            inputs[3].dx = -inputs[3].dx;
            inputs[3].dy = -inputs[3].dy;
        }
        assert!(matches!(r.verify(), Err(ReplayError::Mismatch { .. })));
    }

    #[test]
    fn swapped_seed_is_caught() {
        let mut r = Replay::record_maze(MazeConfig::depot()).unwrap();
        if let Session::Maze { config, .. } = &mut r.session {
            config.seed += 1;
        }
        assert!(r.verify().is_err());
    }

    #[test]
    fn edited_orders_and_hash_are_caught() {
        let mut r = Replay::record_murmuration(FlockSpec::small(300), 600).unwrap();
        let good = r.clone();
        if let Session::Murmuration { orders, .. } = &mut r.session {
            orders[0].x += 25.0;
        }
        assert!(r.verify().is_err());
        let mut forged = good;
        forged.result.state_hash = "0000000000000000".into();
        assert!(matches!(
            forged.verify(),
            Err(ReplayError::Mismatch {
                field: "state_hash",
                ..
            })
        ));
    }

    #[test]
    fn hostile_documents_are_rejected_before_simulating() {
        let r = Replay::record_maze(MazeConfig::depot()).unwrap();
        let text = r.to_json().unwrap_or_default();
        assert!(matches!(
            Replay::from_json("not json"),
            Err(ReplayError::Parse(_))
        ));
        assert!(matches!(
            Replay::from_json(&text.replace("gcraft", "other")),
            Err(ReplayError::Unsupported(_))
        ));
        assert!(matches!(
            Replay::from_json(&text.replace("\"version\":1", "\"version\":9")),
            Err(ReplayError::Unsupported(_))
        ));
        let mut huge = r.clone();
        huge.result.ticks = u64::MAX;
        assert!(matches!(huge.verify(), Err(ReplayError::Invalid(_))));
        let mut even = r.clone();
        if let Session::Maze { config, .. } = &mut even.session {
            config.size = 1000;
        }
        assert!(matches!(even.verify(), Err(ReplayError::Invalid(_))));
        let mut unordered = r;
        if let Session::Maze { inputs, .. } = &mut unordered.session {
            inputs.swap(1, 2);
        }
        assert!(matches!(unordered.verify(), Err(ReplayError::Invalid(_))));
        let mut flock = Replay::record_murmuration(FlockSpec::small(100), 60).unwrap();
        if let Session::Murmuration { config, .. } = &mut flock.session {
            config.count = 10_000_000;
        }
        assert!(matches!(flock.verify(), Err(ReplayError::Invalid(_))));
    }

    #[test]
    fn invalid_steps_are_ignored_like_the_game_does() {
        let session = Session::Maze {
            config: MazeConfig::depot(),
            inputs: vec![
                MazeInput {
                    tick: 1,
                    dx: 1,
                    dy: 1,
                },
                MazeInput {
                    tick: 2,
                    dx: 0,
                    dy: -1,
                },
            ],
        };
        let r = Replay::from_session(session, 10).unwrap();
        assert_eq!(r.result.summary["moves"], 0);
        assert_eq!(r.result.summary["bumps"], 1);
        assert_eq!(r.result.status, Status::Incomplete);
    }

    #[test]
    fn save_and_load_round_trip() {
        let r = Replay::record_maze(MazeConfig::depot()).unwrap();
        let path = tmp("roundtrip").join("nested").join("depot.gcraft");
        r.save(&path).unwrap();
        assert_eq!(Replay::load(&path).ok(), Some(r));
        assert!(matches!(
            Replay::load(&path.with_extension("missing")),
            Err(ReplayError::Io(_))
        ));
        let _ = std::fs::remove_dir_all(tmp("roundtrip"));
    }

    #[test]
    fn info_leaves_out_the_log() {
        let r = Replay::record_maze(MazeConfig::depot()).unwrap();
        let info = r.info();
        assert_eq!(info["game"], "maze");
        assert_eq!(info["inputs"], 32);
        assert!(info.get("session").is_none());
    }
}

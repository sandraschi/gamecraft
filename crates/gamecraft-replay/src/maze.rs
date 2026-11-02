//! Maze rules as a pure function of (level, inputs).
//!
//! These are the same rules `gamecraft-bevy` runs inside its fixed-step schedule
//! (`apply_move`, `check_win`). A parity test in that crate drives the real Bevy app
//! and checks it lands on the same numbers, so the two cannot drift silently.

use crate::hash::Fnv;
use crate::{Outcome, ReplayError, Status, MAX_TICKS};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Fixed ticks between autoplay steps (mirrors `gamecraft_bevy::maze::STEP_TICKS`).
pub const STEP_TICKS: u64 = 6;

/// Smallest and largest accepted maze size (cells per side, odd).
pub const SIZE_RANGE: (i32, i32) = (5, 151);

/// A generated maze: square, odd size, start top-left, goal bottom-right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MazeConfig {
    /// Cells per side (odd).
    pub size: i32,
    /// Generator seed.
    pub seed: u64,
}

/// One requested step, applied on `tick` (1 is the first tick of the run).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MazeInput {
    /// Fixed tick the step is applied on.
    pub tick: u64,
    /// Column delta, expected -1, 0 or 1.
    pub dx: i32,
    /// Row delta, expected -1, 0 or 1.
    pub dy: i32,
}

impl MazeConfig {
    /// The shipped depot maze (15 x 15, seed 9).
    #[must_use]
    pub const fn depot() -> Self {
        Self { size: 15, seed: 9 }
    }

    /// The Bevy default (29 x 29, seed 146).
    #[must_use]
    pub const fn challenge() -> Self {
        Self {
            size: 29,
            seed: 146,
        }
    }

    /// Reject sizes the generator was never meant for.
    ///
    /// # Errors
    /// [`ReplayError::Invalid`] when the size is even or outside [`SIZE_RANGE`].
    pub fn validate(&self) -> Result<(), ReplayError> {
        if self.size % 2 == 0 || self.size < SIZE_RANGE.0 || self.size > SIZE_RANGE.1 {
            return Err(ReplayError::Invalid(format!(
                "maze size {} must be odd and within {}..={}",
                self.size, SIZE_RANGE.0, SIZE_RANGE.1
            )));
        }
        Ok(())
    }

    /// Wall cells, sorted.
    #[must_use]
    pub fn walls(&self) -> Vec<(i32, i32)> {
        let mut w = gamecraft_sim::maze_walls(self.size, self.size, self.seed);
        w.sort_unstable();
        w
    }

    /// Player start cell.
    #[must_use]
    pub const fn start(&self) -> (i32, i32) {
        (1, 1)
    }

    /// Goal cell.
    #[must_use]
    pub const fn goal(&self) -> (i32, i32) {
        (self.size - 2, self.size - 2)
    }
}

/// Play `inputs` on the maze for `ticks` ticks.
///
/// Rules, exactly as in the Bevy schedule: a step must change exactly one axis by one
/// (anything else is ignored), a step into a wall or off the grid is a bump, standing
/// on the goal wins and ends the run.
///
/// # Errors
/// [`ReplayError::Invalid`] for a bad config, `ticks` over [`MAX_TICKS`], or input ticks
/// that are zero, past `ticks`, or not strictly increasing.
pub fn simulate(
    cfg: &MazeConfig,
    inputs: &[MazeInput],
    ticks: u64,
) -> Result<Outcome, ReplayError> {
    cfg.validate()?;
    if ticks > MAX_TICKS {
        return Err(ReplayError::Invalid(format!(
            "ticks {ticks} exceeds the limit {MAX_TICKS}"
        )));
    }
    let mut last = 0u64;
    for i in inputs {
        if i.tick == 0 || i.tick > ticks || i.tick <= last {
            return Err(ReplayError::Invalid(format!(
                "input tick {} must be in 1..={ticks} and strictly increasing",
                i.tick
            )));
        }
        last = i.tick;
    }
    let walls: HashSet<(i32, i32)> = cfg.walls().into_iter().collect();
    let open = |c: (i32, i32)| {
        c.0 >= 0 && c.1 >= 0 && c.0 < cfg.size && c.1 < cfg.size && !walls.contains(&c)
    };
    let goal = cfg.goal();
    let mut pos = cfg.start();
    let (mut moves, mut bumps) = (0u32, 0u32);
    let mut won_at: Option<u64> = None;
    for i in inputs {
        if i.dx.unsigned_abs().saturating_add(i.dy.unsigned_abs()) != 1 {
            continue;
        }
        let target = (pos.0 + i.dx, pos.1 + i.dy);
        if open(target) {
            pos = target;
            moves += 1;
        } else {
            bumps += 1;
        }
        if pos == goal {
            won_at = Some(i.tick);
            break;
        }
    }
    let ran = won_at.unwrap_or(ticks);
    let mut h = Fnv::new();
    h.i32(pos.0);
    h.i32(pos.1);
    h.u32(moves);
    h.u32(bumps);
    h.u64(won_at.unwrap_or(0));
    Ok(Outcome {
        status: if won_at.is_some() {
            Status::Won
        } else {
            Status::Incomplete
        },
        ticks: ran,
        state_hash: h.hex(),
        summary: serde_json::json!({
            "moves": moves,
            "bumps": bumps,
            "position": [pos.0, pos.1],
            "won_at_tick": won_at,
        }),
    })
}

/// The autoplay run: walk the shortest path, one step every [`STEP_TICKS`] ticks,
/// first step on tick 1. Returns the input log and the tick count.
///
/// # Errors
/// [`ReplayError::Invalid`] for a bad config or a maze with no solution.
pub fn autoplay(cfg: &MazeConfig) -> Result<(Vec<MazeInput>, u64), ReplayError> {
    cfg.validate()?;
    let path = gamecraft_sim::bfs_path(cfg.start(), cfg.goal(), &cfg.walls(), cfg.size, cfg.size)
        .ok_or_else(|| ReplayError::Invalid("maze has no solution".into()))?;
    let mut inputs = Vec::new();
    let mut tick = 1u64;
    for pair in path.windows(2) {
        inputs.push(MazeInput {
            tick,
            dx: pair[1].0 - pair[0].0,
            dy: pair[1].1 - pair[0].1,
        });
        tick += STEP_TICKS;
    }
    let ticks = inputs.last().map_or(0, |i| i.tick);
    Ok((inputs, ticks))
}

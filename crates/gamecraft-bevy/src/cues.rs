//! When to make a sound: the games' state in, a list of [`Cue`]s out.
//!
//! Kept free of Bevy audio so the rules are unit-tested everywhere; the `audio` feature
//! only maps each cue to a clip and plays it.

use crate::maze::MazeRun;
use gamecraft_sim::flock::FlockStats;

/// A sound a game wants to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cue {
    /// A hawk took a starling.
    Screech,
    /// The flare went off.
    Flare,
    /// The player won.
    Win,
    /// The player lost.
    Loss,
    /// The maze player stepped one cell.
    Step,
    /// The maze player walked into a wall.
    Bump,
}

/// Ticks between two hawk cries, so a feeding frenzy is a few cries, not a buzz.
const SCREECH_GAP_TICKS: u32 = 20;

/// What the murmuration listener has already heard.
#[derive(Debug, Clone, Copy)]
pub struct FlockEars {
    eaten: usize,
    flare_age: u32,
    last_screech: Option<u32>,
    over: bool,
}

impl Default for FlockEars {
    fn default() -> Self {
        Self {
            eaten: 0,
            flare_age: u32::MAX,
            last_screech: None,
            over: false,
        }
    }
}

impl FlockEars {
    /// Compare the world with what was heard last time and return the new cues.
    /// `flare_age` is the sim's steps since the last flare (it drops when one fires).
    pub fn listen(&mut self, stats: &FlockStats, flare_age: u32) -> Vec<Cue> {
        let mut cues = Vec::new();
        if stats.eaten > self.eaten {
            let quiet = self
                .last_screech
                .is_none_or(|t| stats.tick.saturating_sub(t) >= SCREECH_GAP_TICKS);
            if quiet {
                cues.push(Cue::Screech);
                self.last_screech = Some(stats.tick);
            }
            self.eaten = stats.eaten;
        }
        if flare_age < self.flare_age {
            cues.push(Cue::Flare);
        }
        self.flare_age = flare_age;
        if !self.over && (stats.won || stats.lost) {
            self.over = true;
            cues.push(if stats.won { Cue::Win } else { Cue::Loss });
        }
        cues
    }
}

/// What the maze listener has already heard.
#[derive(Debug, Clone, Copy, Default)]
pub struct MazeEars {
    moves: u32,
    bumps: u32,
    won: bool,
}

impl MazeEars {
    /// Compare the run with what was heard last time and return the new cues.
    pub fn listen(&mut self, run: &MazeRun) -> Vec<Cue> {
        let mut cues = Vec::new();
        if run.moves > self.moves {
            cues.push(Cue::Step);
        }
        if run.bumps > self.bumps {
            cues.push(Cue::Bump);
        }
        if run.won && !self.won {
            cues.push(Cue::Win);
        }
        self.moves = run.moves;
        self.bumps = run.bumps;
        self.won = run.won;
        cues
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(tick: u32, eaten: usize, won: bool, lost: bool) -> FlockStats {
        FlockStats {
            tick,
            alive: 100,
            eaten,
            in_roost: 0,
            needed: 85,
            won,
            lost,
        }
    }

    #[test]
    fn a_hawk_strike_screeches_once_per_burst() {
        let mut ears = FlockEars::default();
        assert_eq!(
            ears.listen(&stats(1, 0, false, false), u32::MAX),
            Vec::<Cue>::new()
        );
        assert_eq!(
            ears.listen(&stats(2, 3, false, false), u32::MAX),
            [Cue::Screech]
        );
        // more eaten right after: same burst, no second cry
        assert_eq!(
            ears.listen(&stats(5, 6, false, false), u32::MAX),
            Vec::<Cue>::new()
        );
        // a later strike cries again
        assert_eq!(
            ears.listen(&stats(60, 7, false, false), u32::MAX),
            [Cue::Screech]
        );
    }

    #[test]
    fn the_flare_whooshes_when_age_resets() {
        let mut ears = FlockEars::default();
        assert_eq!(
            ears.listen(&stats(1, 0, false, false), u32::MAX),
            Vec::<Cue>::new()
        );
        assert_eq!(ears.listen(&stats(2, 0, false, false), 0), [Cue::Flare]);
        assert_eq!(
            ears.listen(&stats(3, 0, false, false), 1),
            Vec::<Cue>::new()
        );
        assert_eq!(
            ears.listen(&stats(4, 0, false, false), 2),
            Vec::<Cue>::new()
        );
    }

    #[test]
    fn the_result_sounds_exactly_once() {
        let mut ears = FlockEars::default();
        assert_eq!(
            ears.listen(&stats(8, 0, false, false), u32::MAX),
            Vec::<Cue>::new()
        );
        assert_eq!(ears.listen(&stats(9, 0, true, false), u32::MAX), [Cue::Win]);
        assert_eq!(
            ears.listen(&stats(10, 0, true, false), u32::MAX),
            Vec::<Cue>::new()
        );
        let mut lost = FlockEars::default();
        assert_eq!(
            lost.listen(&stats(9, 0, false, true), u32::MAX),
            [Cue::Loss]
        );
    }

    #[test]
    fn the_maze_steps_bumps_and_wins() {
        let mut ears = MazeEars::default();
        let mut run = MazeRun::default();
        assert_eq!(ears.listen(&run), Vec::<Cue>::new());
        run.moves = 1;
        assert_eq!(ears.listen(&run), [Cue::Step]);
        run.bumps = 1;
        assert_eq!(ears.listen(&run), [Cue::Bump]);
        run.won = true;
        assert_eq!(ears.listen(&run), [Cue::Win]);
        assert_eq!(ears.listen(&run), Vec::<Cue>::new());
    }
}

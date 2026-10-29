//! Deterministic trace models: the same physics the `GDScript` runs, in Rust.
//!
//! The platformer family shares one kinematic stepper; grid games share a
//! BFS pathfinder. Every verifier is pure and seedable, so traces replay
//! byte-identically.

use std::collections::{HashMap, HashSet, VecDeque};

/// Fixed timestep: 60 Hz, matching Godot physics.
pub const DT: f32 = 1.0 / 60.0;

/// Landing tolerance: float rounding can put resting feet ~1e-7 below a top.
/// `0.001` is four orders of magnitude above the noise and far below any
/// gameplay scale, so contacts stay deterministic.
const LAND_EPS: f32 = 0.001;

/// 2D platforming body: center position plus velocity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body2 {
    /// Center x.
    pub x: f32,
    /// Center y.
    pub y: f32,
    /// Velocity x.
    pub vx: f32,
    /// Velocity y.
    pub vy: f32,
}

/// Walkable top surface spanning `x..x + w` at height `y`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Platform {
    /// Left edge.
    pub x: f32,
    /// Surface height.
    pub y: f32,
    /// Width.
    pub w: f32,
}

/// One input tick of a scripted trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceInput {
    /// No input.
    None,
    /// Move left.
    Left,
    /// Move right.
    Right,
    /// Jump in place.
    Jump,
    /// Jump while moving left.
    LeftJump,
    /// Jump while moving right.
    RightJump,
    /// Flap / lift impulse.
    Flap,
}

/// Shared movement parameters (mirrored in `player_controller.gd`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Phys {
    /// Horizontal speed (units/s); also auto-run speed when set.
    pub speed: f32,
    /// Jump / flap impulse (units/s).
    pub jump_v: f32,
    /// Gravity (units/s^2).
    pub gravity: f32,
    /// Constant +x motion (runner, flappy, driving).
    pub auto_run: bool,
    /// Player half width.
    pub half_w: f32,
    /// Player half height.
    pub half_h: f32,
}

/// One kinematic step. Returns true while the body is grounded afterwards.
#[must_use]
pub fn step_body(
    body: &mut Body2,
    input: TraceInput,
    phys: Phys,
    grounds: &[Platform],
    kill_y: f32,
) -> bool {
    let dir = match input {
        TraceInput::Left | TraceInput::LeftJump => -1.0,
        TraceInput::Right | TraceInput::RightJump => 1.0,
        TraceInput::None | TraceInput::Jump | TraceInput::Flap => 0.0,
    };
    if phys.auto_run {
        body.x += phys.speed * DT;
    } else {
        body.x += dir * phys.speed * DT;
    }
    let old_feet = body.y - phys.half_h;
    body.vy -= phys.gravity * DT;
    body.y += body.vy * DT;
    let mut grounded = false;
    if body.vy <= 0.0 {
        for ground in grounds {
            let feet = body.y - phys.half_h;
            if old_feet >= ground.y - LAND_EPS
                && feet <= ground.y
                && body.x + phys.half_w > ground.x
                && body.x - phys.half_w < ground.x + ground.w
            {
                body.y = ground.y + phys.half_h;
                body.vy = 0.0;
                grounded = true;
            }
        }
    }
    let jump = matches!(
        input,
        TraceInput::Jump | TraceInput::LeftJump | TraceInput::RightJump | TraceInput::Flap
    );
    if jump && (grounded || matches!(input, TraceInput::Flap)) {
        body.vy = phys.jump_v;
        grounded = false;
    }
    if body.y < kill_y {
        body.vy = 0.0;
    }
    grounded
}

/// Full scripted run: script is consumed tick by tick, then `None` holds.
#[derive(Debug, Clone)]
pub struct TraceRun {
    /// Reached the goal.
    pub won: bool,
    /// Ticks simulated.
    pub frames: usize,
    /// Coins touched.
    pub coins: usize,
    /// Per-tick center positions (for GIF frames).
    pub path: Vec<(f32, f32)>,
    /// Tick at which each coin was taken (`None` = never).
    pub coin_at: Vec<Option<usize>>,
}

/// Run a scripted trace against platforms, coins, and a goal gate.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn run_trace(
    start: Body2,
    script: &[TraceInput],
    phys: Phys,
    grounds: &[Platform],
    coins: &[(f32, f32)],
    goal_x: f32,
    kill_y: f32,
    max_frames: usize,
) -> TraceRun {
    let mut body = start;
    let mut taken: Vec<Option<usize>> = vec![None; coins.len()];
    let mut taken_count = 0_usize;
    let mut path: Vec<(f32, f32)> = Vec::new();
    let mut won = false;
    let mut frames = 0_usize;
    for tick in 0..max_frames {
        let input = script.get(tick).copied().unwrap_or(TraceInput::None);
        let _grounded = step_body(&mut body, input, phys, grounds, kill_y);
        for (index, coin) in coins.iter().enumerate() {
            if taken[index].is_none()
                && (body.x - coin.0).abs() < 0.7
                && (body.y - coin.1).abs() < 0.9
            {
                taken[index] = Some(path.len());
                taken_count += 1;
            }
        }
        path.push((body.x, body.y));
        frames = tick + 1;
        if body.x >= goal_x && body.y > kill_y {
            won = true;
            break;
        }
        if body.y < kill_y {
            break;
        }
    }
    TraceRun {
        won,
        frames,
        coins: taken_count,
        path,
        coin_at: taken,
    }
}

/// Grid BFS path avoiding blocked cells inside `0..w`, `0..h`.
#[must_use]
pub fn bfs_path(
    start: (i32, i32),
    goal: (i32, i32),
    blocked: &[(i32, i32)],
    w: i32,
    h: i32,
) -> Option<Vec<(i32, i32)>> {
    let walls: HashSet<(i32, i32)> = blocked.iter().copied().collect();
    let mut prev: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut queue: VecDeque<(i32, i32)> = VecDeque::new();
    queue.push_back(start);
    prev.insert(start, start);
    while let Some(cell) = queue.pop_front() {
        if cell == goal {
            let mut path = vec![cell];
            let mut cursor = cell;
            while cursor != start {
                let parent = prev.get(&cursor)?;
                cursor = *parent;
                path.push(cursor);
            }
            path.reverse();
            return Some(path);
        }
        for step in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let next = (cell.0 + step.0, cell.1 + step.1);
            if next.0 < 0 || next.1 < 0 || next.0 >= w || next.1 >= h {
                continue;
            }
            if walls.contains(&next) || prev.contains_key(&next) {
                continue;
            }
            prev.insert(next, cell);
            queue.push_back(next);
        }
    }
    None
}

/// Sokoban solver state: player plus sorted crate cells.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SokoState {
    /// Player cell.
    pub player: (i32, i32),
    /// Crate cells, sorted.
    pub crates: Vec<(i32, i32)>,
}

/// BFS crate-pushing solution. Returns move steps (dx, dy) or `None`.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn solve_sokoban(
    player: (i32, i32),
    crates: &[(i32, i32)],
    targets: &[(i32, i32)],
    walls: &[(i32, i32)],
    w: i32,
    h: i32,
    max_visits: usize,
) -> Option<Vec<(i32, i32)>> {
    let wall_set: HashSet<(i32, i32)> = walls.iter().copied().collect();
    let target_set: HashSet<(i32, i32)> = targets.iter().copied().collect();
    let mut start_crates: Vec<(i32, i32)> = crates.to_vec();
    start_crates.sort_unstable();
    let start = SokoState {
        player,
        crates: start_crates,
    };
    let mut prev: HashMap<SokoState, (SokoState, (i32, i32))> = HashMap::new();
    let mut seen: HashSet<SokoState> = HashSet::new();
    let mut queue: VecDeque<SokoState> = VecDeque::new();
    queue.push_back(start.clone());
    seen.insert(start);
    while let Some(state) = queue.pop_front() {
        if state.crates.iter().all(|c| target_set.contains(c)) {
            let mut moves: Vec<(i32, i32)> = Vec::new();
            let mut cursor = state;
            while let Some((parent, step)) = prev.get(&cursor) {
                moves.push(*step);
                cursor = parent.clone();
            }
            moves.reverse();
            return Some(moves);
        }
        if seen.len() > max_visits {
            return None;
        }
        for step in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let dest = (state.player.0 + step.0, state.player.1 + step.1);
            if dest.0 < 0 || dest.1 < 0 || dest.0 >= w || dest.1 >= h {
                continue;
            }
            if wall_set.contains(&dest) {
                continue;
            }
            let mut next = state.clone();
            if let Some(slot) = next.crates.iter().position(|c| *c == dest) {
                let beyond = (dest.0 + step.0, dest.1 + step.1);
                if beyond.0 < 0
                    || beyond.1 < 0
                    || beyond.0 >= w
                    || beyond.1 >= h
                    || wall_set.contains(&beyond)
                    || next.crates.contains(&beyond)
                {
                    continue;
                }
                next.crates[slot] = beyond;
                next.crates.sort_unstable();
            }
            next.player = dest;
            if seen.contains(&next) {
                continue;
            }
            seen.insert(next.clone());
            prev.insert(next.clone(), (state.clone(), step));
            queue.push_back(next);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flat ground: holding right must cross x = 6.
    #[test]
    fn flat_run_reaches_gate() {
        let grounds = [Platform {
            x: -50.0,
            y: 0.0,
            w: 200.0,
        }];
        let phys = Phys {
            speed: 4.0,
            jump_v: 7.0,
            gravity: 20.0,
            auto_run: false,
            half_w: 0.3,
            half_h: 0.5,
        };
        let start = Body2 {
            x: 0.0,
            y: 0.5,
            vx: 0.0,
            vy: 0.0,
        };
        let script = vec![TraceInput::Right; 120];
        let run = run_trace(
            start,
            script.as_slice(),
            phys,
            &grounds,
            &[],
            6.0,
            -10.0,
            600,
        );
        assert!(run.won);
        assert!(run.frames < 120);
    }

    /// A 2-wide, 1-high gap is jumpable but not walkable.
    #[test]
    fn gap_needs_jump() {
        let grounds = [
            Platform {
                x: -50.0,
                y: 0.0,
                w: 50.0,
            },
            Platform {
                x: 2.0,
                y: 0.0,
                w: 200.0,
            },
        ];
        let phys = Phys {
            speed: 4.0,
            jump_v: 7.0,
            gravity: 20.0,
            auto_run: false,
            half_w: 0.3,
            half_h: 0.5,
        };
        let start = Body2 {
            x: -4.0,
            y: 0.5,
            vx: 0.0,
            vy: 0.0,
        };
        let walk = vec![TraceInput::Right; 300];
        let walked = run_trace(start, walk.as_slice(), phys, &grounds, &[], 6.0, -10.0, 300);
        assert!(!walked.won);
        let mut jumpy = vec![TraceInput::Right; 300];
        jumpy[52] = TraceInput::RightJump;
        let jumped = run_trace(
            start,
            jumpy.as_slice(),
            phys,
            &grounds,
            &[],
            6.0,
            -10.0,
            300,
        );
        assert!(jumped.won);
    }

    #[test]
    fn maze_bfs_avoids_walls() {
        let walls = [(1, 0), (1, 1)];
        let path = bfs_path((0, 0), (2, 0), &walls, 3, 3);
        assert!(path.is_some());
        if let Some(path) = path {
            assert_eq!(path.first(), Some(&(0, 0)));
            assert_eq!(path.last(), Some(&(2, 0)));
            assert!(path.len() > 3);
        }
    }

    #[test]
    fn sokoban_single_push_solves() {
        let walls: Vec<(i32, i32)> = Vec::new();
        let solved = solve_sokoban((0, 0), &[(1, 0)], &[(2, 0)], &walls, 4, 1, 10_000);
        assert_eq!(solved, Some(vec![(1, 0)]));
    }
}

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

// ---------------------------------------------------------------------------
// Flight
// ---------------------------------------------------------------------------

/// Flight state: forward position, altitude, climb rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightBody {
    /// Forward position.
    pub x: f32,
    /// Altitude.
    pub y: f32,
    /// Climb rate (units/s).
    pub vy: f32,
}

/// One flight tick toward a target altitude, matching `flight_controller.gd`.
pub fn step_flight(body: &mut FlightBody, target_y: f32, speed: f32, climb: f32, dt: f32) {
    let error = target_y - body.y;
    let want = if error > 0.15 {
        climb
    } else if error < -0.15 {
        -climb
    } else {
        0.0
    };
    body.vy += (want - body.vy) * (6.0 * dt).min(1.0);
    body.x += speed * dt;
    body.y = (body.y + body.vy * dt).max(0.8);
}

/// Result of a flight trace.
#[derive(Debug, Clone)]
pub struct FlightRun {
    /// Cleared every gate.
    pub won: bool,
    /// Ticks simulated.
    pub frames: usize,
    /// Gates cleared.
    pub rings: usize,
    /// Per-tick `(x, y)` for GIF frames.
    pub path: Vec<(f32, f32)>,
}

/// Fly the gate course with an autopilot that tracks the next gate's altitude.
///
/// Deterministic: same constants, same gates, same result every run.
#[must_use]
pub fn run_flight(
    speed: f32,
    climb: f32,
    gates: &[(f32, f32)],
    tolerance: f32,
    max_frames: usize,
) -> FlightRun {
    let mut body = FlightBody {
        x: 0.0,
        y: 3.0,
        vy: 0.0,
    };
    let mut cleared = 0_usize;
    let mut path: Vec<(f32, f32)> = Vec::new();
    let mut frames = 0_usize;
    let mut won = false;
    for _ in 0..max_frames {
        let target = gates.get(cleared).map_or(body.y, |gate| gate.1);
        step_flight(&mut body, target, speed, climb, DT);
        path.push((body.x, body.y));
        frames += 1;
        if let Some(gate) = gates.get(cleared) {
            if body.x >= gate.0 {
                if (body.y - gate.1).abs() <= tolerance {
                    cleared += 1;
                    if cleared >= gates.len() {
                        won = true;
                        break;
                    }
                } else {
                    break;
                }
            }
        } else {
            won = true;
            break;
        }
    }
    FlightRun {
        won,
        frames,
        rings: cleared,
        path,
    }
}

#[cfg(test)]
mod flight_tests {
    use super::*;

    #[test]
    fn autopilot_clears_ascending_course() {
        let gates = [(20.0, 3.0), (40.0, 6.0), (60.0, 4.0)];
        let run = run_flight(12.0, 6.0, &gates, 1.6, 600);
        assert!(run.won, "gates={} frames={}", run.rings, run.frames);
        assert_eq!(run.rings, 3);
        assert!(run.frames < 600);
    }

    #[test]
    fn unreachable_gate_stops_the_run() {
        // A gate 40 units up is beyond what the climb rate reaches in time.
        let gates = [(10.0, 40.0)];
        let run = run_flight(12.0, 6.0, &gates, 1.6, 600);
        assert!(!run.won);
    }
}

// ---------------------------------------------------------------------------
// Driving (top-down car on a closed circuit)
// ---------------------------------------------------------------------------

/// Car state on the XZ plane. Forward is `(cos heading, -sin heading)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarBody {
    /// World x.
    pub x: f32,
    /// World z.
    pub z: f32,
    /// Heading in radians.
    pub heading: f32,
}

/// One car tick steering toward `target`, matching `player_controller.gd` mode 6.
pub fn step_car(car: &mut CarBody, target: (f32, f32), speed: f32, turn_rate: f32, dt: f32) {
    let desired = (-(target.1 - car.z)).atan2(target.0 - car.x);
    let mut error = desired - car.heading;
    while error > std::f32::consts::PI {
        error -= std::f32::consts::TAU;
    }
    while error < -std::f32::consts::PI {
        error += std::f32::consts::TAU;
    }
    let step = (turn_rate * dt).min(error.abs());
    car.heading += step.copysign(error);
    car.x += car.heading.cos() * speed * dt;
    car.z -= car.heading.sin() * speed * dt;
}

/// Result of a circuit run.
#[derive(Debug, Clone)]
pub struct DriveRun {
    /// Completed the requested laps.
    pub won: bool,
    /// Ticks simulated.
    pub frames: usize,
    /// Laps completed.
    pub laps: u32,
    /// Per-tick `(x, z)` for GIF frames.
    pub path: Vec<(f32, f32)>,
}

/// Follow a closed circuit with a pure-pursuit steering policy.
#[must_use]
pub fn run_drive(
    waypoints: &[(f32, f32)],
    start_heading: f32,
    speed: f32,
    turn_rate: f32,
    advance: f32,
    laps: u32,
    max_frames: usize,
) -> DriveRun {
    let mut car = CarBody {
        x: waypoints.first().map_or(0.0, |p| p.0),
        z: waypoints.first().map_or(0.0, |p| p.1),
        heading: start_heading,
    };
    let mut index = 1_usize;
    let mut done = 0_u32;
    let mut path: Vec<(f32, f32)> = Vec::new();
    let mut frames = 0_usize;
    let mut won = false;
    for _ in 0..max_frames {
        if waypoints.is_empty() {
            break;
        }
        let target = waypoints[index % waypoints.len()];
        step_car(&mut car, target, speed, turn_rate, DT);
        path.push((car.x, car.z));
        frames += 1;
        let dx = target.0 - car.x;
        let dz = target.1 - car.z;
        if (dx * dx + dz * dz).sqrt() < advance {
            index += 1;
            if index.is_multiple_of(waypoints.len()) {
                done += 1;
                if done >= laps {
                    won = true;
                    break;
                }
            }
        }
    }
    DriveRun {
        won,
        frames,
        laps: done,
        path,
    }
}
#[cfg(test)]
mod drive_tests {
    use super::*;

    fn circle(radius: f32) -> Vec<(f32, f32)> {
        (0..8_u16)
            .map(|i| {
                let angle = std::f32::consts::TAU * f32::from(i) / 8.0;
                (radius * angle.cos(), radius * angle.sin())
            })
            .collect()
    }

    #[test]
    fn car_completes_a_lap() {
        let course = circle(12.0);
        let heading = (-(course[1].1 - course[0].1)).atan2(course[1].0 - course[0].0);
        let run = run_drive(&course, heading, 10.0, 3.0, 3.0, 1, 600);
        assert!(run.won, "laps={} frames={}", run.laps, run.frames);
        assert!(run.frames < 600);
    }

    #[test]
    fn car_cannot_turn_fast_enough_for_a_tight_course() {
        let course = circle(2.0);
        let heading = (-(course[1].1 - course[0].1)).atan2(course[1].0 - course[0].0);
        let run = run_drive(&course, heading, 10.0, 0.5, 1.0, 1, 600);
        assert!(!run.won);
    }
}

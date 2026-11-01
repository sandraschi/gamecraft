//! Maze: walk from the top-left corner to the goal in the bottom-right.
//!
//! The level is `gamecraft_sim::maze_walls(29, 29, 146)`, the challenge maze. Moves are
//! grid steps: input or autoplay writes an [`Intent`], `apply_move` accepts it if
//! the target cell is open, `check_win` ends the run on the goal. Autoplay follows
//! the BFS shortest path, so a correct game wins in exactly `path.len() - 1` moves
//! with zero bumps; the tests assert both.

use bevy::prelude::*;
use std::collections::HashSet;

/// Printed once on a win. Contains "goal reached", one of the catalog's win markers.
pub const WIN_MARKER: &str = "GameCraft: maze solved, goal reached";

/// Fixed ticks between autoplay steps (64 Hz / 6 is about 10 cells per second).
pub const STEP_TICKS: u32 = 6;

/// The level: grid size, wall cells, start and goal.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct MazeLevel {
    /// Cells across.
    pub width: i32,
    /// Cells down.
    pub height: i32,
    /// Blocked cells.
    pub walls: HashSet<(i32, i32)>,
    /// Player start.
    pub start: (i32, i32),
    /// Goal cell.
    pub goal: (i32, i32),
}

impl MazeLevel {
    /// A square maze of `size` cells (odd) from `seed`, start top-left, goal bottom-right.
    #[must_use]
    pub fn generate(size: i32, seed: u64) -> Self {
        Self {
            width: size,
            height: size,
            walls: gamecraft_sim::maze_walls(size, size, seed)
                .into_iter()
                .collect(),
            start: (1, 1),
            goal: (size - 2, size - 2),
        }
    }

    /// The same level as `depot/games/maze` (15 x 15, seed 9).
    #[must_use]
    pub fn depot() -> Self {
        Self::generate(15, 9)
    }

    /// The Bevy default: 29 x 29, seed 146. Against the depot maze it has 8x the solution
    /// length (272 moves), 3x the dead ends and junctions (21 and 19) and 4x the open cells.
    /// Picked with the ignored survey test.
    #[must_use]
    pub fn challenge() -> Self {
        Self::generate(29, 146)
    }

    /// Inside the grid and not a wall.
    #[must_use]
    pub fn open(&self, cell: (i32, i32)) -> bool {
        cell.0 >= 0
            && cell.1 >= 0
            && cell.0 < self.width
            && cell.1 < self.height
            && !self.walls.contains(&cell)
    }

    /// How hard the level is: solution moves, dead ends, junctions, open cells.
    #[must_use]
    pub fn stats(&self) -> MazeStats {
        let mut stats = MazeStats {
            solution_moves: self.solution().map_or(0, |p| p.len().saturating_sub(1)),
            ..MazeStats::default()
        };
        for x in 0..self.width {
            for y in 0..self.height {
                if !self.open((x, y)) {
                    continue;
                }
                stats.open_cells += 1;
                let exits = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .filter(|(dx, dy)| self.open((x + dx, y + dy)))
                    .count();
                if exits == 1 && (x, y) != self.start && (x, y) != self.goal {
                    stats.dead_ends += 1;
                }
                if exits >= 3 {
                    stats.junctions += 1;
                }
            }
        }
        stats
    }

    /// Shortest path start to goal (inclusive), via the shared BFS.
    #[must_use]
    pub fn solution(&self) -> Option<Vec<(i32, i32)>> {
        let mut walls: Vec<(i32, i32)> = self.walls.iter().copied().collect();
        walls.sort_unstable();
        gamecraft_sim::bfs_path(self.start, self.goal, &walls, self.width, self.height)
    }
}

/// Difficulty measures of a level (see [`MazeLevel::stats`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MazeStats {
    /// Steps on the shortest path.
    pub solution_moves: usize,
    /// Open cells with one exit (excluding start and goal): wrong turns end here.
    pub dead_ends: usize,
    /// Open cells with three or more exits: decisions.
    pub junctions: usize,
    /// Walkable cells.
    pub open_cells: usize,
}

/// Grid cell of an entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPos(pub i32, pub i32);

/// The walker.
#[derive(Component, Debug)]
pub struct Player;

/// The goal tile.
#[derive(Component, Debug)]
pub struct Goal;

/// A wall block.
#[derive(Component, Debug)]
pub struct Wall;

/// Progress of the current run.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct MazeRun {
    /// Accepted steps.
    pub moves: u32,
    /// Steps refused by a wall.
    pub bumps: u32,
    /// Goal reached.
    pub won: bool,
    /// Fixed ticks elapsed.
    pub tick: u64,
    /// Tick of the win.
    pub won_at_tick: Option<u64>,
}

/// One requested step `(dx, dy)`, written by input or autoplay, consumed by `apply_move`.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Intent(pub Option<(i32, i32)>);

/// Autoplay: the solution path and where along it we are.
#[derive(Resource, Debug, Clone)]
pub struct Autoplay {
    path: Vec<(i32, i32)>,
    next: usize,
    cooldown: u32,
}

/// Game rules for the maze. `autoplay` makes the built-in driver walk the solution.
#[derive(Debug, Clone)]
pub struct MazeLogicPlugin {
    /// The level to play.
    pub level: MazeLevel,
    /// Walk the shortest path automatically.
    pub autoplay: bool,
}

impl Plugin for MazeLogicPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.level.clone())
            .init_resource::<MazeRun>()
            .init_resource::<Intent>()
            .add_systems(Startup, spawn_level)
            .add_systems(
                FixedUpdate,
                (
                    tick,
                    autoplay_step.run_if(resource_exists::<Autoplay>),
                    apply_move,
                    check_win,
                )
                    .chain(),
            );
        if self.autoplay {
            if let Some(path) = self.level.solution() {
                app.insert_resource(Autoplay {
                    path,
                    next: 1,
                    cooldown: 0,
                });
            }
        }
    }
}

/// Spawn the player, the goal and one entity per wall cell (rendering is added
/// separately, so this runs the same with or without a window).
pub fn spawn_level(mut commands: Commands, level: Res<MazeLevel>) {
    commands.spawn((Player, GridPos(level.start.0, level.start.1)));
    commands.spawn((Goal, GridPos(level.goal.0, level.goal.1)));
    let mut walls: Vec<&(i32, i32)> = level.walls.iter().collect();
    walls.sort_unstable();
    for &(x, y) in walls {
        commands.spawn((Wall, GridPos(x, y)));
    }
}

fn tick(mut run: ResMut<MazeRun>) {
    run.tick += 1;
}

/// Every [`STEP_TICKS`], ask for the next step along the solution.
fn autoplay_step(
    mut auto: ResMut<Autoplay>,
    mut intent: ResMut<Intent>,
    run: Res<MazeRun>,
    player: Query<&GridPos, With<Player>>,
) {
    if run.won {
        return;
    }
    if auto.cooldown > 0 {
        auto.cooldown -= 1;
        return;
    }
    let (Ok(pos), Some(&target)) = (player.single(), auto.path.get(auto.next)) else {
        return;
    };
    intent.0 = Some((target.0 - pos.0, target.1 - pos.1));
    auto.next += 1;
    auto.cooldown = STEP_TICKS - 1;
}

/// Take the pending intent: move if the target cell is open, else count a bump.
fn apply_move(
    mut intent: ResMut<Intent>,
    level: Res<MazeLevel>,
    mut run: ResMut<MazeRun>,
    mut player: Query<&mut GridPos, With<Player>>,
) {
    let Some((dx, dy)) = intent.0.take() else {
        return;
    };
    if run.won || (dx.abs() + dy.abs()) != 1 {
        return;
    }
    let Ok(mut pos) = player.single_mut() else {
        return;
    };
    let target = (pos.0 + dx, pos.1 + dy);
    if level.open(target) {
        *pos = GridPos(target.0, target.1);
        run.moves += 1;
    } else {
        run.bumps += 1;
    }
}

/// Standing on the goal wins, once.
fn check_win(
    level: Res<MazeLevel>,
    mut run: ResMut<MazeRun>,
    player: Query<&GridPos, With<Player>>,
) {
    if run.won {
        return;
    }
    if player.single().is_ok_and(|p| (p.0, p.1) == level.goal) {
        run.won = true;
        run.won_at_tick = Some(run.tick);
        println!(
            "{WIN_MARKER} (moves {}, bumps {}, tick {})",
            run.moves, run.bumps, run.tick
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// A windowless app on a manual clock: each `update()` is exactly one fixed tick.
    fn headless(level: MazeLevel, autoplay: bool) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_secs_f64(1.0 / 64.0),
            ))
            .add_plugins(MazeLogicPlugin { level, autoplay });
        app
    }

    fn player_cell(app: &mut App) -> Option<(i32, i32)> {
        let mut q = app.world_mut().query_filtered::<&GridPos, With<Player>>();
        q.single(app.world()).ok().map(|p| (p.0, p.1))
    }

    #[test]
    fn challenge_maze_is_solvable() {
        let level = MazeLevel::challenge();
        let path = level.solution().unwrap_or_default();
        assert_eq!(path.first(), Some(&level.start));
        assert_eq!(path.last(), Some(&level.goal));
    }

    /// The user-facing promise: at least three times the depot maze on every measure.
    #[test]
    fn challenge_maze_is_at_least_three_times_the_depot_maze() {
        let base = MazeLevel::depot().stats();
        let hard = MazeLevel::challenge().stats();
        assert!(
            hard.solution_moves >= 3 * base.solution_moves,
            "{hard:?} vs {base:?}"
        );
        assert!(hard.dead_ends >= 3 * base.dead_ends, "{hard:?} vs {base:?}");
        assert!(hard.junctions >= 3 * base.junctions, "{hard:?} vs {base:?}");
        assert!(
            hard.open_cells >= 3 * base.open_cells,
            "{hard:?} vs {base:?}"
        );
    }
    #[test]
    fn autoplay_wins_on_the_shortest_path_with_no_bumps() {
        let level = MazeLevel::challenge();
        let shortest = level.solution().map_or(0, |p| p.len() - 1);
        let mut app = headless(level.clone(), true);
        for _ in 0..20_000 {
            app.update();
            if app.world().resource::<MazeRun>().won {
                break;
            }
        }
        let run = app.world().resource::<MazeRun>().clone();
        assert!(run.won, "autoplay did not reach the goal: {run:?}");
        assert_eq!(usize::try_from(run.moves).unwrap_or(0), shortest);
        assert_eq!(run.bumps, 0);
        assert_eq!(player_cell(&mut app), Some(level.goal));
    }

    #[test]
    fn walls_block_and_count_a_bump() {
        let mut app = headless(MazeLevel::challenge(), false);
        app.update();
        // (1, 1) has the outer wall to its left.
        app.world_mut().resource_mut::<Intent>().0 = Some((-1, 0));
        app.update();
        app.update();
        let run = app.world().resource::<MazeRun>().clone();
        assert_eq!((run.moves, run.bumps), (0, 1));
        assert_eq!(player_cell(&mut app), Some((1, 1)));
    }

    #[test]
    fn diagonal_or_long_steps_are_ignored() {
        let mut app = headless(MazeLevel::challenge(), false);
        app.update();
        app.world_mut().resource_mut::<Intent>().0 = Some((1, 1));
        app.update();
        let run = app.world().resource::<MazeRun>().clone();
        assert_eq!((run.moves, run.bumps), (0, 0));
    }

    /// Seed survey for picking the depot level: `cargo test -p gamecraft-bevy survey -- --ignored --nocapture`.
    #[test]
    #[ignore = "survey, prints candidates"]
    fn survey() {
        println!("15/9 today: {:?}", MazeLevel::generate(15, 9).stats());
        for size in [25, 27, 29, 31] {
            let mut best: Vec<(u64, MazeStats)> = (1..=200)
                .map(|seed| (seed, MazeLevel::generate(size, seed).stats()))
                .collect();
            best.sort_by_key(|(_, s)| std::cmp::Reverse(s.solution_moves));
            for (seed, s) in best.iter().take(4) {
                println!("size {size} seed {seed}: {s:?}");
            }
        }
    }

    #[test]
    fn same_seed_same_level() {
        assert_eq!(MazeLevel::generate(15, 9), MazeLevel::depot());
        assert_ne!(MazeLevel::generate(15, 10).walls, MazeLevel::depot().walls);
    }
}

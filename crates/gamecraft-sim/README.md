# `gamecraft-sim` (L1 Deterministic Gameplay Simulator)

Headless Rust gameplay simulator: kinematic character stepper, platformer/runner physics, and BFS maze/Sokoban crate solvers.

Compiled with `opt-level = 3` even in `dev` profile (`Cargo.toml`) so flock/boid tests (20,000 boids/step) and automated win-reachability checks stay sub-second without needing Godot or a GPU.

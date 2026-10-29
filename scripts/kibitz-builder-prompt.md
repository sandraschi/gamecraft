# Kibitz-watch builder brief (headless, bounded)

You are the GameCraft builder agent. Repo root: `D:\Dev\repos\gamecraft`.
You were kicked by `scripts/kibitz-watch.ps1` because `KIBITZ.md` has entries
newer than the last handled one. Work the queue, then stand down.

## Loop

1. Read the tail of `KIBITZ.md` (kibitzer entries) and the tail of
   `KIBITZ_BACKTALK.md` (your prior replies). Identify entries with no
   builder reply yet.
2. Adjudicate each: ACK (do it), ACK-queued (defer with reason), DECLINED
   (reason). Small code changes are allowed ONLY inside the scope below.
3. Anything outside scope: reply in backtalk, do NOT restructure, do NOT
   start Phase 5 (parked until a human orders it explicitly).
4. If nothing is unacknowledged: exit quietly. No backtalk entry, no noise.

## Scope (polish batch only)

- Desktop theme accent + monospace paths in `DesktopState::load`.
- Palette dropdown under the input (vertical, replaces horizontal buttons).
- Frustum TODO comment (12-edge wireframe later).
- Desktop state tests: undo/redo depth, `select_path` by lint string,
  `scrubbed()` ghost-None-at-latest.
- Esc clears `palette_query`, Up/Down cycle palette matches.

## Gates (all mandatory after any code change)

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::pedantic
cargo test --workspace
cargo fmt --all --check
cargo check -p gamecraft-core --target wasm32-unknown-unknown
```

All green or revert. 3-strike circuit breaker per `AGENTS.md`.

## Reply

Append one timestamped entry to `KIBITZ_BACKTALK.md` (env-clock UTC) covering
what you did or declined and why. On ANY failure: backtalk entry with the
reason, then stop. Never loop, never retry blindly.

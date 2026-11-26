# `gamecraft-bevy` (L3 Bevy 0.20 Runtime)

Native Bevy 0.20 runtime for GameCraft simulations (`maze`, `murmuration`, `toy-railway`).

Game logic plugins are separated from rendering (gated behind the `render` Cargo feature) so all gameplay systems run and verify headlessly in CI without a GPU. Use `cargo run --profile play` for responsive interactive builds.

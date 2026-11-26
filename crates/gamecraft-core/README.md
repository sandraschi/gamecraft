# `gamecraft-core` (L1 Spatial Kernel)

WASM-safe spatial kernel for **GameCraft**: node graph, 2D/3D transforms (`glam`), AABB bounds, collision primitives, and signal wiring.

## Guarantees

- Compiles cleanly to `wasm32-unknown-unknown` (`cargo check -p gamecraft-core --target wasm32-unknown-unknown`).
- `#![forbid(unsafe_code)]`, `clippy::pedantic`, zero `unwrap`/`expect`/`panic`.
- Deterministic data structures: identical scene inputs always serialize to identical bytes.

//! `gamecraft-core`: WASM-safe spatial kernel for `GameCraft`.
//!
//! Node graph, `glam` transform hierarchy, axis-aligned bounding boxes,
//! collision-layer matrix, and the signal registry. This crate must stay
//! free of `wgpu`, `egui`, and godot bindings so it keeps compiling to
//! `wasm32-unknown-unknown` (`just check-wasm`).

pub mod aabb;
pub mod collision;
pub mod node;
pub mod signals;
pub mod spatial;
pub mod transform;

pub use aabb::Aabb;
pub use collision::CollisionMatrix;
pub use node::{NodeId, PropertyValue, SceneGraph, SceneNode};
pub use signals::{SignalConnection, SignalRegistry};
pub use transform::Transform;

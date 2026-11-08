//! `scene_lint`, `spatial_query` and `export_scene` for the `GameCraft` MCP server.
//!
//! The stdio JSON-RPC framing lives in `main.rs`. The lint rules themselves live
//! in `gamecraft-godot` (so `gamecraft-forge` does not depend on this crate) and
//! are re-exported here unchanged.

pub mod catalog;
pub mod guide;
pub mod input;
pub mod inspect;
pub mod launch;
pub mod proc_guard;
pub mod registry;
pub mod replay_cmds;
pub mod spec;
pub mod tools;

use gamecraft_godot::{build_graph, parse_tscn};

pub use gamecraft_godot::lint::{
    lint_live, scene_lint, LintDiagnostic, ERR_MISSING_COLLISION_SHAPE, ERR_SCENE_PARSE,
    ERR_UNRESOLVED_RESOURCE, ERR_UNRESOLVED_SIGNAL_TARGET,
};
pub use input::resolve_scene_text;
pub use inspect::{
    scene_inspect, InspectOptions, DEFAULT_DEPTH, DEFAULT_LIMIT, OP_FIND, OP_PROPERTY, OP_SIGNALS,
    OP_TREE,
};
pub use spec::{export_spec, ConnSpec, ExportOutcome, ExtSpec, NodeSpec, SceneSpec, SubSpec};

/// World-space AABB of one node subtree, as JSON.
///
/// # Errors
///
/// Returns a message when the scene fails to parse/build or when `node_path`
/// does not resolve from the scene root.
pub fn spatial_query(scene_text: &str, node_path: &str) -> Result<serde_json::Value, String> {
    let scene = parse_tscn(scene_text).map_err(|e| format!("scene failed to parse: {e}"))?;
    let graph = build_graph(&scene).map_err(|e| format!("scene failed to build: {e}"))?;
    let root = graph.root.ok_or_else(|| "scene has no root".to_string())?;
    let target = if node_path == "." || node_path.is_empty() {
        root
    } else {
        graph
            .resolve_path(root, node_path)
            .ok_or_else(|| format!("node path {node_path} did not resolve"))?
    };
    let bounds = gamecraft_core::spatial::subtree_aabb(&graph, target);
    let path = graph.node_path(target).unwrap_or_else(|| "?".to_string());
    Ok(serde_json::json!({
        "node": path,
        "empty": bounds.is_empty(),
        "min": bounds.min.to_array(),
        "max": bounds.max.to_array(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamecraft_godot::build_registry;

    /// Regression: Godot refuses a scene whose property points at a resource id
    /// the file never declares. This shipped once (enemy script id drift) and
    /// only Godot caught it, so the lint now owns it.
    #[test]
    fn undeclared_resource_reference_is_flagged() {
        const BAD: &str = "[gd_scene load_steps=2 format=3]\n\n[ext_resource type=\"Script\" path=\"res://a.gd\" id=\"1_a\"]\n\n[node name=\"Root\" type=\"Node3D\"]\nscript = ExtResource(\"9_missing\")\n";
        let diags = scene_lint(BAD);
        let hits: Vec<&LintDiagnostic> = diags
            .iter()
            .filter(|diag| diag.code == ERR_UNRESOLVED_RESOURCE)
            .collect();
        assert_eq!(hits.len(), 1);
        if let Some(hit) = hits.first() {
            assert!(hit.message.contains("9_missing"));
        }
    }

    /// The negative case: a correctly declared id must stay silent.
    #[test]
    fn declared_resource_reference_is_quiet() {
        const GOOD: &str = "[gd_scene load_steps=2 format=3]\n\n[ext_resource type=\"Script\" path=\"res://a.gd\" id=\"1_a\"]\n\n[node name=\"Root\" type=\"Node3D\"]\nscript = ExtResource(\"1_a\")\n";
        let diags = scene_lint(GOOD);
        assert_eq!(diags, Vec::new());
    }

    /// One clean body, one bare body, one dangling connection.
    const SAMPLE: &str = "[gd_scene load_steps=2 format=3]\n\
        \n\
        [node name=\"Root\" type=\"Node3D\"]\n\
        \n\
        [node name=\"Good\" type=\"StaticBody3D\" parent=\".\"]\n\
        \n\
        [node name=\"Shape\" type=\"CollisionShape3D\" parent=\"Good\"]\n\
        \n\
        [node name=\"Bare\" type=\"RigidBody3D\" parent=\".\"]\n\
        \n\
        [connection signal=\"pressed\" from=\"Bare\" to=\"Ghost\" method=\"_on_x\"]\n";

    #[test]
    fn missing_shape_is_flagged() {
        let diags = scene_lint(SAMPLE);
        let shape_errs: Vec<&LintDiagnostic> = diags
            .iter()
            .filter(|d| d.code == ERR_MISSING_COLLISION_SHAPE)
            .collect();
        assert_eq!(shape_errs.len(), 1);
        if let Some(first) = shape_errs.first() {
            assert_eq!(first.node, "Root/Bare");
        }
    }

    #[test]
    fn dangling_signal_target_is_flagged() {
        let diags = scene_lint(SAMPLE);
        let sig_errs: Vec<&LintDiagnostic> = diags
            .iter()
            .filter(|d| d.code == ERR_UNRESOLVED_SIGNAL_TARGET)
            .collect();
        assert_eq!(sig_errs.len(), 1);
    }

    #[test]
    fn clean_scene_is_quiet() {
        /// Root plus one body that owns a shape; no connections at all.
        const CLEAN: &str = "[gd_scene load_steps=2 format=3]\n\
            \n\
            [node name=\"Root\" type=\"Node3D\"]\n\
            \n\
            [node name=\"Good\" type=\"StaticBody3D\" parent=\".\"]\n\
            \n\
            [node name=\"Shape\" type=\"CollisionShape3D\" parent=\"Good\"]\n";
        let diags = scene_lint(CLEAN);
        assert_eq!(diags, Vec::new());
    }

    #[test]
    fn live_lint_matches_text_lint() {
        if let Ok(scene) = parse_tscn(SAMPLE) {
            if let Ok(graph) = build_graph(&scene) {
                let registry = build_registry(&scene, &graph);
                let live = lint_live(&graph, &registry);
                let text = scene_lint(SAMPLE);
                assert_eq!(live.len(), text.len());
            }
        }
    }

    #[test]
    fn spatial_query_returns_box() {
        let res = spatial_query(SAMPLE, "Good");
        assert!(res.is_ok());
        if let Ok(value) = res {
            assert_eq!(
                value.get("node").and_then(serde_json::Value::as_str),
                Some("Root/Good")
            );
            assert_eq!(
                value.get("empty").and_then(serde_json::Value::as_bool),
                Some(false)
            );
        }
    }

    #[test]
    fn spatial_query_rejects_unknown_path() {
        assert!(spatial_query(SAMPLE, "Nope/Missing").is_err());
    }
}

//! Semantic scene checks over the parsed graph.
//!
//! These live beside the parser rather than in the MCP layer so that the depot
//! forge can run them without depending on the server crate.

use crate::{build_graph, build_registry, parse_tscn, SceneFile};
use gamecraft_core::{NodeId, PropertyValue, SceneGraph, SignalRegistry};
use serde::{Deserialize, Serialize};

/// Physics body without a `CollisionShape3D` descendant.
pub const ERR_MISSING_COLLISION_SHAPE: &str = "ERR_MISSING_COLLISION_SHAPE";
/// Connection whose target path or method cannot be resolved.
pub const ERR_UNRESOLVED_SIGNAL_TARGET: &str = "ERR_UNRESOLVED_SIGNAL_TARGET";
/// Scene text failed to parse.
pub const ERR_SCENE_PARSE: &str = "ERR_SCENE_PARSE";
/// A property references an `ExtResource("id")` / `SubResource("id")` that the
/// file never declares. Godot refuses to instantiate the whole scene on this,
/// so it is a hard error rather than a warning.
pub const ERR_UNRESOLVED_RESOURCE: &str = "ERR_UNRESOLVED_RESOURCE";

/// One verification finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintDiagnostic {
    /// Machine code (`ERR_...`).
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
    /// Scene path of the implicated node (`"."` for file-level issues).
    pub node: String,
}

impl LintDiagnostic {
    /// Build a finding.
    #[must_use]
    pub fn new(code: &str, message: String, node: String) -> Self {
        Self {
            code: code.to_string(),
            message,
            node,
        }
    }
}

/// Lint a live in-memory graph (no re-parse): missing collision shapes plus
/// unresolvable signal targets and methods.
///
/// The desktop shell calls this every frame; [`scene_lint`] funnels through
/// the same checks after parsing.
#[must_use]
pub fn lint_live(graph: &SceneGraph, registry: &SignalRegistry) -> Vec<LintDiagnostic> {
    let mut out: Vec<LintDiagnostic> = check_collision_shapes(graph);
    if let Some(root) = graph.root {
        let (_, unresolved) = registry.resolve(graph, root);
        for (index, reason) in unresolved {
            let from = registry
                .connections
                .get(index)
                .map_or_else(|| "?".to_string(), |c| c.from.clone());
            out.push(LintDiagnostic::new(
                ERR_UNRESOLVED_SIGNAL_TARGET,
                reason,
                from,
            ));
        }
    }
    out
}

/// Lint `.tscn` / `.tres` text: missing collision shapes plus unresolvable
/// signal targets and methods.
#[must_use]
pub fn scene_lint(scene_text: &str) -> Vec<LintDiagnostic> {
    let scene = match parse_tscn(scene_text) {
        Ok(scene) => scene,
        Err(err) => {
            return vec![LintDiagnostic::new(
                ERR_SCENE_PARSE,
                format!("scene failed to parse: {err}"),
                ".".to_string(),
            )];
        }
    };
    let graph = match build_graph(&scene) {
        Ok(graph) => graph,
        Err(err) => {
            return vec![LintDiagnostic::new(
                ERR_SCENE_PARSE,
                format!("scene failed to build: {err}"),
                ".".to_string(),
            )];
        }
    };
    let registry = build_registry(&scene, &graph);
    let mut out = lint_live(&graph, &registry);
    out.extend(check_resource_refs(&scene, &graph));
    out
}

/// Collect every `prefix("id")` reference id inside a raw property value.
fn extract_ids<'a>(text: &'a str, prefix: &str) -> Vec<&'a str> {
    let needle = format!("{prefix}(\"");
    let mut out: Vec<&'a str> = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find(needle.as_str()) {
        let after = &rest[pos + needle.len()..];
        if let Some(end) = after.find('"') {
            out.push(&after[..end]);
            rest = &after[end..];
        } else {
            break;
        }
    }
    out
}

/// Flag property references to undeclared external / sub resources.
///
/// This is text-level (it needs the declarations), so it lives in
/// [`scene_lint`] rather than [`lint_live`], which only sees the graph.
fn check_resource_refs(scene: &SceneFile, graph: &SceneGraph) -> Vec<LintDiagnostic> {
    let ext: std::collections::HashSet<&str> = scene
        .ext_resources
        .iter()
        .map(|res| res.id.as_str())
        .collect();
    let sub: std::collections::HashSet<&str> = scene
        .sub_resources
        .iter()
        .map(|res| res.id.as_str())
        .collect();
    let mut ids: Vec<NodeId> = graph.nodes.keys().copied().collect();
    ids.sort_unstable();
    let mut out: Vec<LintDiagnostic> = Vec::new();
    for id in ids {
        let Some(node) = graph.get(id) else { continue };
        let path = graph.node_path(id).unwrap_or_else(|| node.name.clone());
        let mut keys: Vec<&String> = node.properties.keys().collect();
        keys.sort();
        for key in keys {
            let Some(PropertyValue::Raw(text)) = node.properties.get(key) else {
                continue;
            };
            for resource_id in extract_ids(text, "ExtResource") {
                if !ext.contains(resource_id) {
                    out.push(LintDiagnostic::new(
                        ERR_UNRESOLVED_RESOURCE,
                        format!("{key} references undeclared ExtResource(\"{resource_id}\")"),
                        path.clone(),
                    ));
                }
            }
            for resource_id in extract_ids(text, "SubResource") {
                if !sub.contains(resource_id) {
                    out.push(LintDiagnostic::new(
                        ERR_UNRESOLVED_RESOURCE,
                        format!("{key} references undeclared SubResource(\"{resource_id}\")"),
                        path.clone(),
                    ));
                }
            }
        }
    }
    out
}

/// Flag every physics body whose subtree holds no `CollisionShape3D`.
fn check_collision_shapes(graph: &SceneGraph) -> Vec<LintDiagnostic> {
    let mut out: Vec<LintDiagnostic> = Vec::new();
    let mut bodies: Vec<NodeId> = graph
        .nodes
        .values()
        .filter(|n| n.is_physics_body())
        .map(|n| n.id)
        .collect();
    bodies.sort_unstable();
    for body in bodies {
        if !subtree_has_shape(graph, body) {
            let path = graph.node_path(body).unwrap_or_else(|| "?".to_string());
            let kind = graph
                .get(body)
                .map_or_else(|| "?".to_string(), |n| n.node_type.clone());
            out.push(LintDiagnostic::new(
                ERR_MISSING_COLLISION_SHAPE,
                format!("{kind} has no CollisionShape3D descendant"),
                path,
            ));
        }
    }
    out
}

/// True when `id` or any descendant is a `CollisionShape3D`.
fn subtree_has_shape(graph: &SceneGraph, id: NodeId) -> bool {
    if let Some(node) = graph.get(id) {
        if node.node_type == "CollisionShape3D" {
            return true;
        }
        for child in node.children.iter().copied() {
            if subtree_has_shape(graph, child) {
                return true;
            }
        }
    }
    false
}

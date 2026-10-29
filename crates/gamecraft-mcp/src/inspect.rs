//! `scene_inspect`: read-only projections of a parsed scene.
//!
//! Four operations over one parse: `tree`, `signals`, `property`, `find`.
//! Paginated by `offset`/`limit` because depot scenes reach 151 nodes and an
//! unbounded dump is unusable in a client's context.

use gamecraft_core::{NodeId, PropertyValue, SceneGraph};
use gamecraft_godot::{build_graph, build_registry, parse_tscn, SceneFile};
use serde_json::{json, Value};

/// Operation: flattened node tree.
pub const OP_TREE: &str = "tree";
/// Operation: signal connections with resolution status.
pub const OP_SIGNALS: &str = "signals";
/// Operation: one node's transform and properties.
pub const OP_PROPERTY: &str = "property";
/// Operation: substring match over node names and types.
pub const OP_FIND: &str = "find";

/// Default page size when the caller does not ask for one.
pub const DEFAULT_LIMIT: usize = 100;
/// Default traversal depth (effectively unlimited for real scenes).
pub const DEFAULT_DEPTH: usize = 64;

/// One inspection call's parameters.
#[derive(Debug, Clone, Copy)]
pub struct InspectOptions<'a> {
    /// One of [`OP_TREE`], [`OP_SIGNALS`], [`OP_PROPERTY`], [`OP_FIND`].
    pub operation: &'a str,
    /// Target node path for [`OP_PROPERTY`].
    pub node_path: Option<&'a str>,
    /// Search text for [`OP_FIND`].
    pub query: Option<&'a str>,
    /// Maximum tree depth below the root.
    pub depth: usize,
    /// Page size.
    pub limit: usize,
    /// Page offset.
    pub offset: usize,
}

impl Default for InspectOptions<'_> {
    fn default() -> Self {
        Self {
            operation: OP_TREE,
            node_path: None,
            query: None,
            depth: DEFAULT_DEPTH,
            limit: DEFAULT_LIMIT,
            offset: 0,
        }
    }
}

/// Run one inspection over scene text.
///
/// # Errors
///
/// Returns a message when the scene fails to parse or build, when the operation
/// is unknown, or when a required argument for that operation is missing.
pub fn scene_inspect(scene_text: &str, options: &InspectOptions<'_>) -> Result<Value, String> {
    let scene = parse_tscn(scene_text).map_err(|err| format!("scene failed to parse: {err}"))?;
    let graph = build_graph(&scene).map_err(|err| format!("scene failed to build: {err}"))?;
    match options.operation {
        OP_TREE => Ok(inspect_tree(&graph, options)),
        OP_SIGNALS => Ok(inspect_signals(&scene, &graph)),
        OP_PROPERTY => inspect_property(&graph, options),
        OP_FIND => Ok(inspect_find(&graph, options)),
        other => Err(format!(
            "unknown operation '{other}' (supported: {OP_TREE}, {OP_SIGNALS}, {OP_PROPERTY}, {OP_FIND})"
        )),
    }
}

/// Flattened pre-order tree, paginated.
fn inspect_tree(graph: &SceneGraph, options: &InspectOptions<'_>) -> Value {
    let mut rows: Vec<Value> = Vec::new();
    if let Some(root) = graph.root {
        collect_rows(graph, root, 0, options.depth, &mut rows);
    }
    let total = rows.len();
    let page = paginate(&rows, options.offset, options.limit);
    let next_offset = options.offset.saturating_add(page.len());
    json!({
        "operation": OP_TREE,
        "total": total,
        "offset": options.offset,
        "limit": options.limit,
        "next_offset": if next_offset < total { Value::from(next_offset) } else { Value::Null },
        "nodes": page,
    })
}

/// Depth-first collector producing one row per node.
fn collect_rows(
    graph: &SceneGraph,
    id: NodeId,
    depth: usize,
    max_depth: usize,
    rows: &mut Vec<Value>,
) {
    if depth > max_depth {
        return;
    }
    let Some(node) = graph.get(id) else {
        return;
    };
    let path = graph.node_path(id).unwrap_or_else(|| node.name.clone());
    rows.push(node_row(graph, node, path.as_str(), depth));
    for child in &node.children {
        collect_rows(graph, *child, depth.saturating_add(1), max_depth, rows);
    }
}

/// One JSON row for the tree operation.
fn node_row(
    graph: &SceneGraph,
    node: &gamecraft_core::SceneNode,
    path: &str,
    depth: usize,
) -> Value {
    let parent = node
        .parent
        .and_then(|parent| graph.node_path(parent))
        .unwrap_or_default();
    let script = node
        .properties
        .get("script")
        .and_then(PropertyValue::as_str)
        .unwrap_or_default();
    json!({
        "path": path,
        "name": node.name,
        "type": node.node_type,
        "parent": parent,
        "depth": depth,
        "unique": node.unique_name,
        "children": node.children.len(),
        "script": script,
        "position": f32_triple(node.transform.position.to_array()),
        "rotation_deg": euler_degrees(node.transform.rotation),
        "scale": f32_triple(node.transform.scale.to_array()),
    })
}

/// Signal connections with per-edge resolution status.
fn inspect_signals(scene: &SceneFile, graph: &SceneGraph) -> Value {
    let registry = build_registry(scene, graph);
    let mut edges: Vec<Value> = Vec::new();
    let (resolved, unresolved) = graph.root.map_or((Vec::new(), Vec::new()), |root| {
        registry.resolve(graph, root)
    });
    for (index, conn) in registry.connections.iter().enumerate() {
        let reason = unresolved
            .iter()
            .find(|(bad_index, _)| *bad_index == index)
            .map(|(_, reason)| reason.clone());
        edges.push(json!({
            "signal": conn.signal,
            "from": conn.from,
            "to": conn.to,
            "method": conn.method,
            "resolved": reason.is_none(),
            "reason": reason,
        }));
    }
    json!({
        "operation": OP_SIGNALS,
        "total": edges.len(),
        "resolved": resolved.len(),
        "unresolved": unresolved.len(),
        "connections": edges,
    })
}

/// One node's transform plus every property.
///
/// # Errors
///
/// Returns a message when `node_path` is missing or does not resolve.
fn inspect_property(graph: &SceneGraph, options: &InspectOptions<'_>) -> Result<Value, String> {
    let Some(requested) = options.node_path else {
        return Err("operation 'property' requires node_path".to_string());
    };
    let root = graph.root.ok_or_else(|| "scene has no root".to_string())?;
    let id = if requested == "." || requested.is_empty() {
        root
    } else {
        graph
            .resolve_path(root, requested)
            .ok_or_else(|| format!("node path '{requested}' did not resolve"))?
    };
    let node = graph
        .get(id)
        .ok_or_else(|| format!("node {id} is missing from the graph"))?;
    let mut properties = serde_json::Map::new();
    for (key, value) in &node.properties {
        properties.insert(key.clone(), value_json(value));
    }
    Ok(json!({
        "operation": OP_PROPERTY,
        "path": graph.node_path(id).unwrap_or_else(|| node.name.clone()),
        "name": node.name,
        "type": node.node_type,
        "unique": node.unique_name,
        "transform": {
            "position": f32_triple(node.transform.position.to_array()),
            "rotation_deg": euler_degrees(node.transform.rotation),
            "scale": f32_triple(node.transform.scale.to_array()),
        },
        "properties": Value::Object(properties),
    }))
}

/// Substring match over node names and types, paginated.
fn inspect_find(graph: &SceneGraph, options: &InspectOptions<'_>) -> Value {
    let needle = options.query.unwrap_or_default().to_lowercase();
    let mut rows: Vec<Value> = Vec::new();
    if needle.is_empty() {
        return json!({
            "operation": OP_FIND,
            "query": "",
            "total": 0,
            "offset": 0,
            "limit": options.limit,
            "next_offset": Value::Null,
            "nodes": [],
        });
    }
    if let Some(root) = graph.root {
        let mut all: Vec<Value> = Vec::new();
        collect_rows(graph, root, 0, options.depth, &mut all);
        rows = all
            .into_iter()
            .filter(|row| {
                let name = row.get("name").and_then(Value::as_str).unwrap_or_default();
                let kind = row.get("type").and_then(Value::as_str).unwrap_or_default();
                name.to_lowercase().contains(needle.as_str())
                    || kind.to_lowercase().contains(needle.as_str())
            })
            .collect();
    }
    let total = rows.len();
    let page = paginate(&rows, options.offset, options.limit);
    let next_offset = options.offset.saturating_add(page.len());
    json!({
        "operation": OP_FIND,
        "query": options.query.unwrap_or_default(),
        "total": total,
        "offset": options.offset,
        "limit": options.limit,
        "next_offset": if next_offset < total { Value::from(next_offset) } else { Value::Null },
        "nodes": page,
    })
}

/// Slice one page out of `rows`.
fn paginate(rows: &[Value], offset: usize, limit: usize) -> Vec<Value> {
    rows.iter()
        .skip(offset)
        .take(limit.max(1))
        .cloned()
        .collect()
}

/// Property value as JSON. Raw stays distinguishable from a typed string.
fn value_json(value: &PropertyValue) -> Value {
    match value {
        PropertyValue::Bool(flag) => Value::Bool(*flag),
        PropertyValue::Int(num) => Value::from(*num),
        PropertyValue::Float(num) => Value::from(*num),
        PropertyValue::Str(text) => Value::String(text.clone()),
        PropertyValue::Raw(text) => json!({ "raw": text }),
        PropertyValue::Vec3(items) => f32_triple(*items),
    }
}

/// `[f32; 3]` as a JSON array of numbers.
fn f32_triple(items: [f32; 3]) -> Value {
    json!([items[0], items[1], items[2]])
}

/// Quaternion as YXZ Euler degrees, rounded to 3 decimals for readability.
fn euler_degrees(rotation: glam::Quat) -> Value {
    let (x, y, z) = rotation.to_euler(glam::EulerRot::YXZ);
    let to_deg =
        |rad: f32| (f64::from(rad) * 180.0 / std::f64::consts::PI * 1000.0).round() / 1000.0;
    json!([to_deg(x), to_deg(y), to_deg(z)])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENE: &str = "[gd_scene load_steps=2 format=3]\n\n[node name=\"Root\" type=\"Node3D\"]\n\n[node name=\"Body\" type=\"StaticBody3D\" parent=\".\"]\nposition = Vector3(1, 2, 3)\n\n[node name=\"Shape\" type=\"CollisionShape3D\" parent=\"Body\"]\n\n[connection signal=\"body_entered\" from=\"Body\" to=\".\" method=\"_on_hit\"]\n";

    #[test]
    fn tree_is_paginated_and_ordered() {
        let options = InspectOptions {
            operation: OP_TREE,
            limit: 2,
            ..InspectOptions::default()
        };
        let value = scene_inspect(SCENE, &options);
        assert!(value.is_ok());
        if let Ok(value) = value {
            assert_eq!(value["total"], 3);
            assert_eq!(value["nodes"].as_array().map(Vec::len), Some(2));
            assert!(value["next_offset"].is_number());
            assert_eq!(value["nodes"][0]["path"], "Root");
            assert_eq!(value["nodes"][1]["path"], "Root/Body");
        }
    }

    #[test]
    fn property_exposes_transform_and_raw() {
        let options = InspectOptions {
            operation: OP_PROPERTY,
            node_path: Some("Body"),
            ..InspectOptions::default()
        };
        let value = scene_inspect(SCENE, &options);
        assert!(value.is_ok());
        if let Ok(value) = value {
            assert_eq!(value["transform"]["position"][0], 1.0);
            assert_eq!(value["transform"]["position"][1], 2.0);
        }
    }

    #[test]
    fn signals_report_resolution() {
        let options = InspectOptions {
            operation: OP_SIGNALS,
            ..InspectOptions::default()
        };
        let value = scene_inspect(SCENE, &options);
        assert!(value.is_ok());
        if let Ok(value) = value {
            assert_eq!(value["total"], 1);
            // No script in this fixture, so the handler cannot be resolved.
            assert_eq!(value["unresolved"], 1);
            assert_eq!(value["connections"][0]["resolved"], false);
        }
    }

    #[test]
    fn find_matches_type() {
        let options = InspectOptions {
            operation: OP_FIND,
            query: Some("collision"),
            ..InspectOptions::default()
        };
        let value = scene_inspect(SCENE, &options);
        assert!(value.is_ok());
        if let Ok(value) = value {
            assert_eq!(value["total"], 1);
            assert_eq!(value["nodes"][0]["name"], "Shape");
        }
    }

    #[test]
    fn unknown_operation_is_rejected() {
        let options = InspectOptions {
            operation: "wat",
            ..InspectOptions::default()
        };
        assert!(scene_inspect(SCENE, &options).is_err());
    }
}

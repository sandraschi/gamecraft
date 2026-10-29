//! Deterministic `.tscn` serializer: [`SceneGraph`] back to Godot 4 text.
//!
//! Inverse of [`parse_tscn`](crate::parse_tscn). Node order follows the scene
//! tree (pre-order from the root), property keys are sorted, floats use
//! shortest-roundtrip formatting, so exporting the same graph twice yields
//! byte-identical text. Packed `transform` always wins over raw
//! `position` / `rotation` / `scale` body properties (those keys are skipped).

use crate::ConnectionDecl;
use gamecraft_core::{NodeId, PropertyValue, SceneGraph, Transform};

/// `(type, path, id)` external resource declaration.
pub type ExtResourceDecl = (String, String, String);

/// `(type, id, properties)` sub-resource declaration.
pub type SubResourceDecl = (String, String, Vec<(String, String)>);

/// Append formatted text to the output buffer (infallible for `String`).
fn push(out: &mut String, args: std::fmt::Arguments<'_>) {
    use std::fmt::Write as _;
    let _ = out.write_fmt(args);
}

/// Everything the serializer needs beyond the graph itself.
#[derive(Debug, Clone)]
pub struct ExportRequest<'a> {
    /// Scene graph to serialize (tree order = file order).
    pub graph: &'a SceneGraph,
    /// Connections in file order.
    pub connections: &'a [ConnectionDecl],
    /// Scene `uid` tag (`uid://...`).
    pub uid: String,
    /// Scene format version (3 for Godot 4.x).
    pub format: u32,
    /// `(type, path, id)` external resources in file order.
    pub ext_resources: Vec<ExtResourceDecl>,
    /// `(type, id, properties)` sub-resources in file order.
    pub sub_resources: Vec<SubResourceDecl>,
}

impl<'a> ExportRequest<'a> {
    /// New request with no resources and format 3.
    #[must_use]
    pub fn new(graph: &'a SceneGraph, connections: &'a [ConnectionDecl], uid: String) -> Self {
        Self {
            graph,
            connections,
            uid,
            format: 3,
            ext_resources: Vec::new(),
            sub_resources: Vec::new(),
        }
    }
}

/// Serialize a scene graph to Godot 4 `.tscn` text.
///
/// # Errors
///
/// Returns a message when the graph has no root or a parent path cannot be
/// resolved (both indicate a malformed graph, never valid builder output).
pub fn export_tscn(request: &ExportRequest<'_>) -> Result<String, String> {
    let graph = request.graph;
    let root = graph
        .root
        .ok_or_else(|| "cannot export a graph without a root".to_string())?;
    let mut out = String::new();
    let load_steps = u32::try_from(
        request
            .ext_resources
            .len()
            .saturating_add(request.sub_resources.len())
            .saturating_add(1),
    )
    .unwrap_or(u32::MAX);
    push(
        &mut out,
        format_args!(
            "[gd_scene load_steps={load_steps} format={} uid=\"{}\"]\n",
            request.format, request.uid
        ),
    );
    for (resource_type, path, id) in &request.ext_resources {
        push(
            &mut out,
            format_args!("\n[ext_resource type=\"{resource_type}\" path=\"{path}\" id=\"{id}\"]\n"),
        );
    }
    for (resource_type, id, props) in &request.sub_resources {
        push(
            &mut out,
            format_args!("\n[sub_resource type=\"{resource_type}\" id=\"{id}\"]\n"),
        );
        for (key, value) in props {
            push(&mut out, format_args!("{key} = {value}\n"));
        }
    }
    for id in tree_order(graph, root) {
        write_node(&mut out, graph, root, id)?;
    }
    for conn in request.connections {
        push(
            &mut out,
            format_args!(
                "\n[connection signal=\"{}\" from=\"{}\" to=\"{}\" method=\"{}\"]\n",
                conn.signal, conn.from, conn.to, conn.method
            ),
        );
    }
    Ok(out)
}

/// Pre-order node ids from the root (children in insertion order).
fn tree_order(graph: &SceneGraph, root: NodeId) -> Vec<NodeId> {
    let mut out: Vec<NodeId> = Vec::new();
    let mut stack: Vec<NodeId> = vec![root];
    while let Some(id) = stack.pop() {
        out.push(id);
        if let Some(node) = graph.get(id) {
            for child in node.children.iter().rev() {
                stack.push(*child);
            }
        }
    }
    out
}

/// Write one `[node]` section plus its body properties.
///
/// # Errors
///
/// Returns a message when the node's parent path cannot be resolved.
fn write_node(
    out: &mut String,
    graph: &SceneGraph,
    root: NodeId,
    id: NodeId,
) -> Result<(), String> {
    let node = graph
        .get(id)
        .ok_or_else(|| format!("export hit unknown node {id}"))?;
    let mut header = format!("{}[node name=\"{}\"", out_separator(out), quoted_name(node));
    push(&mut header, format_args!(" type=\"{}\"", node.node_type));
    if let Some(parent) = node.parent {
        let path = godot_parent_path(graph, root, parent)?;
        push(&mut header, format_args!(" parent=\"{path}\""));
    }
    header.push_str("]\n");
    out.push_str(header.as_str());
    // Godot 4 spells unique names as a node flag, never as a '%' in the name.
    // Writing '%Name' makes child paths ('Name' vs '%Name') unresolvable.
    if node.unique_name {
        push(out, format_args!("unique_name_in_owner = true\n"));
    }
    // `script` must precede every property the script defines. Godot applies
    // node properties in file order, so an alphabetically-sorted list sets
    // `flight`/`mode` on a node that has no script yet: the values are silently
    // dropped and the node keeps its script defaults.
    if let Some(script) = node.properties.get("script") {
        push(out, format_args!("script = {}\n", encode_value(script)));
    }
    push(
        out,
        format_args!("transform = {}\n", encode_transform(node.transform)),
    );
    let mut keys: Vec<&String> = node.properties.keys().collect();
    keys.sort();
    for key in keys {
        if key.as_str() == "position"
            || key.as_str() == "rotation"
            || key.as_str() == "scale"
            || key.as_str() == "transform"
            || key.as_str() == "unique_name_in_owner"
            || key.as_str() == "script"
        {
            continue;
        }
        if let Some(value) = node.properties.get(key) {
            push(out, format_args!("{key} = {}\n", encode_value(value)));
        }
    }
    Ok(())
}

/// Blank separator between sections (none before the very first line).
fn out_separator(out: &str) -> &str {
    if out.is_empty() {
        ""
    } else {
        "\n"
    }
}

/// `%`-prefixed name for unique-name owners, plain otherwise.
fn quoted_name(node: &gamecraft_core::SceneNode) -> String {
    node.name.clone()
}

/// Godot-style parent path relative to the scene root (`.` = direct child).
///
/// # Errors
///
/// Returns a message when the parent chain leaves the graph.
fn godot_parent_path(graph: &SceneGraph, root: NodeId, parent: NodeId) -> Result<String, String> {
    if parent == root {
        return Ok(".".to_string());
    }
    let mut parts: Vec<String> = Vec::new();
    let mut cursor = parent;
    loop {
        let node = graph
            .get(cursor)
            .ok_or_else(|| format!("export hit unknown parent {cursor}"))?;
        parts.push(node.name.clone());
        if let Some(next) = node.parent {
            if next == root {
                break;
            }
            cursor = next;
        } else {
            break;
        }
    }
    parts.reverse();
    Ok(parts.join("/"))
}

/// Packed `Transform3D(...)` from a local transform.
///
/// Godot's 12-float constructor takes the basis **row by row**, not column by
/// column. `to_cols_array` gives column-major data, so the upper 3x3 is
/// transposed here. Emitting columns verbatim silently flipped every rotation
/// (a "look down" camera became "look up"), which is invisible on axis-aligned
/// translation-only nodes and wrong everywhere else.
fn encode_transform(transform: Transform) -> String {
    let m = transform.matrix().to_cols_array();
    let nums = [
        m[0], m[4], m[8], // row 0
        m[1], m[5], m[9], // row 1
        m[2], m[6], m[10], // row 2
        m[12], m[13], m[14], // origin
    ];
    let mut parts: Vec<String> = Vec::new();
    for num in nums {
        parts.push(format!("{num:?}"));
    }
    format!("Transform3D({})", parts.join(", "))
}

/// Godot literal for a property value.
///
/// `Str` is a typed string and is always quoted. `Raw` is already-valid Godot
/// syntax (`ExtResource("1")`, `NodePath("../Ball")`, `Array[String]([...])`)
/// and is emitted verbatim - quoting it would break every path property.
fn encode_value(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Bool(flag) => {
            if *flag {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        PropertyValue::Int(num) => format!("{num}"),
        PropertyValue::Float(num) => format!("{num:?}"),
        PropertyValue::Str(text) => format!("\"{}\"", escape_string(text)),
        PropertyValue::Raw(text) => text.clone(),
        PropertyValue::Vec3(items) => {
            format!("Vector3({:?}, {:?}, {:?})", items[0], items[1], items[2])
        }
    }
}

/// Escape a quoted Godot string.
fn escape_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out
}

/// Encode a Godot `Vector3` for tests and builders.
#[must_use]
pub fn encode_vector3(items: [f32; 3]) -> String {
    format!("Vector3({:?}, {:?}, {:?})", items[0], items[1], items[2])
}

/// Encode a Godot `Color` literal for tests and builders.
#[must_use]
pub fn encode_color(red: f32, green: f32, blue: f32) -> String {
    format!("Color({red:?}, {green:?}, {blue:?}, 1)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_graph, parse_tscn};

    /// Round trip: parse the sample scene, export, re-parse, compare counts.
    #[test]
    fn export_round_trip_is_stable() {
        let first = parse_tscn(crate::tests::SAMPLE);
        assert!(first.is_ok());
        if let Ok(scene) = first {
            if let Ok(graph) = build_graph(&scene) {
                let request = ExportRequest::new(
                    &graph,
                    &scene.connections,
                    scene
                        .header
                        .uid
                        .clone()
                        .unwrap_or_else(|| "uid://test".to_string()),
                );
                let text = export_tscn(&request);
                assert!(text.is_ok());
                if let Ok(text) = text {
                    let second = parse_tscn(text.as_str());
                    assert!(second.is_ok());
                    if let Ok(second) = second {
                        assert_eq!(second.nodes.len(), scene.nodes.len());
                        let request_b = ExportRequest::new(
                            &graph,
                            &scene.connections,
                            scene
                                .header
                                .uid
                                .clone()
                                .unwrap_or_else(|| "uid://test".to_string()),
                        );
                        if let Ok(again) = export_tscn(&request_b) {
                            assert_eq!(text, again);
                        }
                    }
                }
            }
        }
    }

    /// Regression: a typed string must be quoted (`mode = "maze"`), while an
    /// already-valid Godot expression must not be (`NodePath("../Ball")`).
    /// Getting either backwards produces a scene Godot refuses to parse.
    #[test]
    fn string_values_quote_and_raw_values_do_not() {
        assert_eq!(
            encode_value(&PropertyValue::Str("maze".to_string())),
            "\"maze\""
        );
        assert_eq!(
            encode_value(&PropertyValue::Raw("NodePath(\"../Ball\")".to_string())),
            "NodePath(\"../Ball\")"
        );
        assert_eq!(
            encode_value(&PropertyValue::Raw("ExtResource(\"1_a\")".to_string())),
            "ExtResource(\"1_a\")"
        );
    }

    /// Regression: Godot 4 unique names are a node flag, not a `%` prefix.
    /// The `%Name` form makes every child parent path unresolvable.
    #[test]
    fn unique_names_use_the_flag_not_a_percent_prefix() {
        let mut graph = SceneGraph::new();
        if let Ok(root) = graph.add_node("World".to_string(), "Node3D".to_string(), None) {
            if let Ok(child) =
                graph.add_node("Player".to_string(), "Node3D".to_string(), Some(root))
            {
                if let Some(node) = graph.get_mut(child) {
                    node.unique_name = true;
                }
                if let Ok(kid) = graph.add_node(
                    "Mesh".to_string(),
                    "MeshInstance3D".to_string(),
                    Some(child),
                ) {
                    let _ = kid;
                }
            }
        }
        let empty: Vec<crate::ConnectionDecl> = Vec::new();
        let request = ExportRequest::new(&graph, &empty, "uid://unique".to_string());
        let text = export_tscn(&request);
        assert!(text.is_ok());
        if let Ok(text) = text {
            assert!(text.contains("name=\"Player\""));
            assert!(!text.contains("%Player"));
            assert!(text.contains("unique_name_in_owner = true"));
            assert!(text.contains("parent=\"Player\""));
        }
    }

    #[test]
    fn empty_graph_is_rejected() {
        let graph = SceneGraph::new();
        let connections: Vec<ConnectionDecl> = Vec::new();
        let request = ExportRequest::new(&graph, &connections, "uid://empty".to_string());
        assert!(export_tscn(&request).is_err());
    }
}

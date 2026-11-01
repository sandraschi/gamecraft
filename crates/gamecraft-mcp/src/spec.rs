//! JSON scene spec -> command-built `SceneGraph` -> deterministic engine text.
//!
//! This is the write path behind the `export_scene` MCP tool. Nodes are created
//! through `gamecraft-commands` (the same bus the desktop and the depot forge
//! use), so an export is reproducible from one provenance rule everywhere.
//!
//! Format is a parameter, not a tool: `godot4` (.tscn text) and `gltf` (glTF 2.0
//! JSON with the buffer embedded) today; further targets are new values here,
//! never a rename or a second schema in the client's context.

use gamecraft_commands::{AddNodeCommand, Command, SetPropertyCommand, SetTransformCommand};
use gamecraft_core::{NodeId, PropertyValue, SceneGraph, Transform};
use gamecraft_godot::{export_tscn, ConnectionDecl, ExportRequest};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Godot 4 `.tscn` text.
pub const FORMAT_GODOT4: &str = "godot4";
/// glTF 2.0, as one self-contained JSON string (base64 buffer).
pub const FORMAT_GLTF: &str = "gltf";
/// Every format `export_spec` accepts.
pub const FORMATS: [&str; 2] = [FORMAT_GODOT4, FORMAT_GLTF];

/// One node in a JSON scene spec.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeSpec {
    /// Node name (unique among siblings).
    pub name: String,
    /// Engine type (`Node3D`, `StaticBody3D`, `MeshInstance3D`, ...).
    #[serde(rename = "type")]
    pub node_type: String,
    /// Parent path; `None` for the scene root, `"."` for a direct child of it.
    #[serde(default)]
    pub parent: Option<String>,
    /// `unique_name_in_owner = true` (the Godot 4 unique-name flag).
    #[serde(default)]
    pub unique: bool,
    /// Local position.
    #[serde(default)]
    pub position: Option<[f64; 3]>,
    /// Local rotation in degrees (Godot's `rotation_degrees`).
    #[serde(default)]
    pub rotation_degrees: Option<[f64; 3]>,
    /// Local scale.
    #[serde(default)]
    pub scale: Option<[f64; 3]>,
    /// Extra properties. Plain JSON scalars become typed values; an object
    /// `{"raw": "..."}` is emitted verbatim as Godot syntax, e.g.
    /// `{"script": {"raw": "ExtResource(\"1_a\")"}}`.
    #[serde(default)]
    pub properties: BTreeMap<String, serde_json::Value>,
}

/// One `[connection]` edge.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnSpec {
    /// Signal name on the source node.
    pub signal: String,
    /// Emitting node path.
    pub from: String,
    /// Receiving node path.
    pub to: String,
    /// Method invoked on the receiver.
    pub method: String,
}

/// One `[ext_resource]` declaration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtSpec {
    /// Resource type (`Script`, `Texture2D`, ...).
    #[serde(rename = "type")]
    pub resource_type: String,
    /// `res://` path.
    pub path: String,
    /// Reference id.
    pub id: String,
}

/// One `[sub_resource]` declaration with verbatim body properties.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubSpec {
    /// Resource type (`BoxShape3D`, `StandardMaterial3D`, ...).
    #[serde(rename = "type")]
    pub resource_type: String,
    /// Reference id.
    pub id: String,
    /// Body properties, already in Godot syntax.
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}

/// A whole scene as data.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SceneSpec {
    /// Scene `uid://...` tag. Derived deterministically when omitted.
    #[serde(default)]
    pub uid: Option<String>,
    /// Format version written into the header (3 for Godot 4.x).
    #[serde(default)]
    pub format: Option<u32>,
    /// Nodes, in file order.
    pub nodes: Vec<NodeSpec>,
    /// Signal connections.
    #[serde(default)]
    pub connections: Vec<ConnSpec>,
    /// External resources.
    #[serde(default)]
    pub ext_resources: Vec<ExtSpec>,
    /// Sub-resources.
    #[serde(default)]
    pub sub_resources: Vec<SubSpec>,
}

/// Result of a successful export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOutcome {
    /// Serialized scene text.
    pub text: String,
    /// Resolved scene `uid`.
    pub uid: String,
    /// Node count written.
    pub nodes: usize,
}

/// Convert one JSON value to a Godot property value.
///
/// # Errors
///
/// Returns a message for `null`, nested objects that are not `{"raw": "..."}`,
/// or arrays that are not exactly three numbers.
fn to_property(value: &serde_json::Value) -> Result<PropertyValue, String> {
    match value {
        serde_json::Value::Bool(flag) => Ok(PropertyValue::Bool(*flag)),
        serde_json::Value::Number(num) => {
            if let Some(int) = num.as_i64() {
                Ok(PropertyValue::Int(int))
            } else if let Some(float) = num.as_f64() {
                Ok(PropertyValue::Float(float))
            } else {
                Err(format!("unsupported number: {num}"))
            }
        }
        serde_json::Value::String(text) => Ok(PropertyValue::Str(text.clone())),
        serde_json::Value::Object(map) => match map.get("raw").and_then(|raw| raw.as_str()) {
            Some(raw) => Ok(PropertyValue::Raw(raw.to_string())),
            None => Err("object property must be {\"raw\": \"<godot syntax>\"}".to_string()),
        },
        serde_json::Value::Array(items) => {
            if items.len() != 3 {
                return Err(format!(
                    "array property must have 3 numbers for Vector3, got {}",
                    items.len()
                ));
            }
            let mut out = [0.0_f32; 3];
            for (slot, item) in out.iter_mut().zip(items.iter()) {
                let num = item
                    .as_f64()
                    .ok_or_else(|| "array property must contain numbers".to_string())?;
                *slot = narrow(num);
            }
            Ok(PropertyValue::Vec3(out))
        }
        serde_json::Value::Null => Err("null is not a property value".to_string()),
    }
}

/// Narrow an `f64` scene coordinate to the engine's `f32`.
///
/// Scene coordinates are authored at human scale, so the narrowing is exact
/// enough; a silent wrap is impossible because `f32` saturates to inf instead.
#[allow(clippy::cast_possible_truncation)]
fn narrow(value: f64) -> f32 {
    value as f32
}

/// Convert a degrees triple to radians.
fn radians(degrees: &[f64; 3]) -> Vec3 {
    const TO_RAD: f64 = std::f64::consts::PI / 180.0;
    Vec3::new(
        narrow(degrees[0] * TO_RAD),
        narrow(degrees[1] * TO_RAD),
        narrow(degrees[2] * TO_RAD),
    )
}

/// Depth of a scene path (`"."` and `None` are depth 0).
fn depth_of(path: &str) -> usize {
    if path == "." || path.is_empty() {
        0
    } else {
        path.split('/').filter(|part| !part.is_empty()).count()
    }
}

/// Full path of a node, exactly as `parent` strings refer to each other.
fn full_path(node: &NodeSpec) -> String {
    match node.parent.as_deref() {
        None | Some(".") => node.name.clone(),
        Some(path) => format!("{path}/{}", node.name),
    }
}

/// Parent-first node order. Returns indices into `nodes`.
///
/// Sorts by tree rank (root = 0, child of root = 1, ...), stable on input
/// order, after proving every `parent` path exists and every path is unique.
///
/// # Errors
///
/// Returns a message on zero/multiple roots, duplicate paths, unknown parents,
/// or a parent cycle.
fn order_nodes(nodes: &[NodeSpec]) -> Result<Vec<usize>, String> {
    let roots: Vec<usize> = (0..nodes.len())
        .filter(|index| nodes.get(*index).is_some_and(|node| node.parent.is_none()))
        .collect();
    if roots.len() != 1 {
        return Err(format!(
            "scene spec needs exactly 1 root node (parent omitted), got {}",
            roots.len()
        ));
    }
    let mut known: BTreeSet<String> = BTreeSet::new();
    for node in nodes {
        let path = full_path(node);
        if !known.insert(path.clone()) {
            return Err(format!("duplicate node path '{path}'"));
        }
    }
    for node in nodes {
        if let Some(parent) = node.parent.as_deref() {
            if parent != "." && !known.contains(parent) {
                return Err(format!(
                    "node '{}' has unknown parent '{parent}'",
                    node.name
                ));
            }
        }
    }
    let mut order: Vec<usize> = (0..nodes.len()).collect();
    order.sort_by_key(|index| {
        nodes
            .get(*index)
            .map_or(usize::MAX, |node| match node.parent.as_deref() {
                None => 0,
                Some(parent) => depth_of(parent).saturating_add(1),
            })
    });
    Ok(order)
}

/// Build the graph, apply transforms and properties.
///
/// # Errors
///
/// Returns a message on structural problems (roots, parents) or bad property
/// values.
fn build_graph(spec: &SceneSpec) -> Result<SceneGraph, String> {
    let mut graph = SceneGraph::new();
    let order = order_nodes(spec.nodes.as_slice())?;
    // path -> id, so `parent` strings resolve in O(1).
    let mut paths: BTreeMap<String, NodeId> = BTreeMap::new();
    for index in order {
        let node = spec
            .nodes
            .get(index)
            .ok_or_else(|| "bad node index".to_string())?;
        let parent_id = match node.parent.as_deref() {
            None => None,
            Some(".") => graph.root,
            Some(path) => paths.get(path).copied(),
        };
        if node.parent.is_some() && parent_id.is_none() {
            return Err(format!(
                "node '{}' has unknown parent '{}'",
                node.name,
                node.parent.clone().unwrap_or_default()
            ));
        }
        let mut cmd = AddNodeCommand::new(node.name.clone(), node.node_type.clone(), parent_id);
        let id = cmd
            .execute_mut(&mut graph)
            .map_err(|err| format!("add '{}': {err}", node.name))?;
        let full_path = match node.parent.as_deref() {
            None | Some(".") => node.name.clone(),
            Some(path) => format!("{path}/{}", node.name),
        };
        paths.insert(full_path, id);
        let position = node.position.map_or(Vec3::ZERO, |value| {
            Vec3::new(narrow(value[0]), narrow(value[1]), narrow(value[2]))
        });
        let rotation = node.rotation_degrees.map_or(glam::Quat::IDENTITY, |value| {
            let euler = radians(&value);
            glam::Quat::from_euler(glam::EulerRot::YXZ, euler.y, euler.x, euler.z)
        });
        let scale = node.scale.map_or(Vec3::ONE, |value| {
            Vec3::new(narrow(value[0]), narrow(value[1]), narrow(value[2]))
        });
        let next = Transform {
            position,
            rotation,
            scale,
        };
        let transform_cmd = SetTransformCommand::new(id, Transform::identity(), next);
        transform_cmd
            .execute(&mut graph)
            .map_err(|err| format!("transform '{}': {err}", node.name))?;
        if let Some(graph_node) = graph.get_mut(id) {
            graph_node.unique_name = node.unique;
        }
        for (key, value) in &node.properties {
            let prop = to_property(value)?;
            let cmd = SetPropertyCommand::new(id, key.clone(), prop);
            cmd.execute(&mut graph)
                .map_err(|err| format!("property '{key}' on '{}': {err}", node.name))?;
        }
    }
    Ok(graph)
}

/// Export a scene spec to engine text.
///
/// # Errors
///
/// Returns a message when the format is unsupported, the spec is structurally
/// invalid, or serialization fails.
pub fn export_spec(spec: &SceneSpec, format: &str) -> Result<ExportOutcome, String> {
    if !FORMATS.contains(&format) {
        return Err(format!(
            "unsupported format '{format}' (supported: {})",
            FORMATS.join(", ")
        ));
    }
    let graph = build_graph(spec)?;
    let uid = spec.uid.clone().unwrap_or_else(|| {
        let root_name = graph
            .root
            .and_then(|root| graph.get(root))
            .map_or_else(|| "scene".to_string(), |node| node.name.clone());
        gamecraft_godot::deterministic_uid(format!("res://{root_name}.tscn").as_str())
    });
    if format == FORMAT_GLTF {
        let subs: Vec<gamecraft_gltf::SubRes> = spec
            .sub_resources
            .iter()
            .map(|res| gamecraft_gltf::SubRes {
                kind: res.resource_type.clone(),
                id: res.id.clone(),
                properties: res
                    .properties
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            })
            .collect();
        let gltf = gamecraft_gltf::export(&graph, &subs)?;
        return Ok(ExportOutcome {
            text: gltf.to_gltf_embedded(),
            uid,
            nodes: spec.nodes.len(),
        });
    }
    let connections: Vec<ConnectionDecl> = spec
        .connections
        .iter()
        .map(|conn| ConnectionDecl {
            signal: conn.signal.clone(),
            from: conn.from.clone(),
            to: conn.to.clone(),
            method: conn.method.clone(),
        })
        .collect();
    let ext: Vec<gamecraft_godot::ExtResourceDecl> = spec
        .ext_resources
        .iter()
        .map(|res| (res.resource_type.clone(), res.path.clone(), res.id.clone()))
        .collect();
    let sub: Vec<gamecraft_godot::SubResourceDecl> = spec
        .sub_resources
        .iter()
        .map(|res| {
            (
                res.resource_type.clone(),
                res.id.clone(),
                res.properties
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )
        })
        .collect();
    let request = ExportRequest {
        graph: &graph,
        connections: connections.as_slice(),
        uid: uid.clone(),
        format: spec.format.unwrap_or(3),
        ext_resources: ext,
        sub_resources: sub,
    };
    let text = export_tscn(&request)?;
    Ok(ExportOutcome {
        text,
        uid,
        nodes: spec.nodes.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> serde_json::Value {
        serde_json::json!({
            "uid": "uid://spec1",
            "nodes": [
                {"name": "World", "type": "Node3D", "properties": {"script": {"raw": "ExtResource(\"1_rt\")"}}},
                {"name": "Body", "type": "StaticBody3D", "parent": ".", "position": [0.0, 0.5, 0.0]},
                {"name": "Shape", "type": "CollisionShape3D", "parent": "Body", "properties": {"shape": {"raw": "SubResource(\"box\")"}}}
            ],
            "connections": [
                {"signal": "body_entered", "from": "Body", "to": ".", "method": "_on_hit"}
            ],
            "ext_resources": [{"type": "Script", "path": "res://rt.gd", "id": "1_rt"}],
            "sub_resources": [{"type": "BoxShape3D", "id": "box", "properties": {"size": "Vector3(1, 1, 1)"}}]
        })
    }

    #[test]
    fn spec_exports_and_reparses() {
        let parsed = serde_json::from_value::<SceneSpec>(tiny());
        assert!(parsed.is_ok(), "fixture must parse as a SceneSpec");
        let Ok(spec) = parsed else { return };
        let outcome = export_spec(&spec, FORMAT_GODOT4);
        assert!(outcome.is_ok());
        if let Ok(outcome) = outcome {
            assert_eq!(outcome.uid, "uid://spec1");
            assert_eq!(outcome.nodes, 3);
            assert!(outcome.text.contains("parent=\"Body\""));
            assert!(outcome.text.contains("[connection signal=\"body_entered\""));
            // Round trip through the parser.
            let reparsed = gamecraft_godot::parse_tscn(outcome.text.as_str());
            assert!(reparsed.is_ok());
            if let Ok(reparsed) = reparsed {
                assert_eq!(reparsed.nodes.len(), 3);
                assert_eq!(reparsed.ext_resources.len(), 1);
                assert_eq!(reparsed.sub_resources.len(), 1);
            }
        }
    }

    #[test]
    fn exported_spec_lints_clean() {
        let parsed = serde_json::from_value::<SceneSpec>(tiny());
        assert!(parsed.is_ok(), "fixture must parse as a SceneSpec");
        let Ok(spec) = parsed else { return };
        if let Ok(outcome) = export_spec(&spec, FORMAT_GODOT4) {
            assert_eq!(crate::scene_lint(outcome.text.as_str()), Vec::new());
        }
    }

    #[test]
    fn two_roots_are_rejected() {
        let spec = SceneSpec {
            nodes: vec![
                NodeSpec {
                    name: "A".to_string(),
                    node_type: "Node3D".to_string(),
                    parent: None,
                    unique: false,
                    position: None,
                    rotation_degrees: None,
                    scale: None,
                    properties: BTreeMap::new(),
                },
                NodeSpec {
                    name: "B".to_string(),
                    node_type: "Node3D".to_string(),
                    parent: None,
                    unique: false,
                    position: None,
                    rotation_degrees: None,
                    scale: None,
                    properties: BTreeMap::new(),
                },
            ],
            ..SceneSpec::default()
        };
        let outcome = export_spec(&spec, FORMAT_GODOT4);
        assert!(outcome.is_err());
    }

    #[test]
    fn spec_exports_to_gltf() {
        let parsed = serde_json::from_value::<SceneSpec>(serde_json::json!({
            "nodes": [
                {"name": "World", "type": "Node3D"},
                {"name": "Crate", "type": "MeshInstance3D", "parent": ".", "position": [1.0, 0.5, 0.0],
                 "properties": {"mesh": {"raw": "SubResource(\"box\")"}}}
            ],
            "sub_resources": [{"type": "BoxMesh", "id": "box", "properties": {"size": "Vector3(1, 1, 1)"}}]
        }));
        assert!(parsed.is_ok());
        let Ok(spec) = parsed else { return };
        let outcome = export_spec(&spec, FORMAT_GLTF);
        assert!(outcome.is_ok(), "{outcome:?}");
        if let Ok(outcome) = outcome {
            let doc: serde_json::Value = serde_json::from_str(&outcome.text).unwrap_or_default();
            assert_eq!(
                doc.pointer("/asset/version"),
                Some(&serde_json::json!("2.0"))
            );
            assert_eq!(
                doc.pointer("/nodes/1/translation"),
                Some(&serde_json::json!([1.0, 0.5, 0.0]))
            );
            assert_eq!(doc.pointer("/nodes/1/mesh"), Some(&serde_json::json!(0)));
            assert!(outcome
                .text
                .contains("data:application/octet-stream;base64,"));
        }
    }

    #[test]
    fn unknown_format_is_rejected() {
        let parsed = serde_json::from_value::<SceneSpec>(tiny());
        assert!(parsed.is_ok(), "fixture must parse as a SceneSpec");
        let Ok(spec) = parsed else { return };
        let outcome = export_spec(&spec, "overte");
        assert!(outcome.is_err());
        if let Err(reason) = outcome {
            assert!(reason.contains("unsupported format"));
        }
    }
}

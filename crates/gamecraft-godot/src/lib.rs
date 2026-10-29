//! Deterministic `nom`-based parser for Godot 4 `.tscn` / `.tres` text.
//!
//! Covers the section shapes the engine writes: the `gd_scene` (or
//! `gd_resource`) header, `ext_resource`, `sub_resource`, `node`, and
//! `connection`. Section headers and `key = value` lines are parsed with
//! `nom` combinators; a small line driver assigns body lines to the open
//! section. Output order always follows input order, so repeated parses of
//! the same file hash identically.

use gamecraft_core::{
    NodeId, PropertyValue, SceneGraph, SignalConnection, SignalRegistry, Transform,
};
use glam::{Mat3, Quat, Vec3};
use nom::{
    bytes::complete::{take_until, take_while1},
    character::complete::{char, multispace0, multispace1},
    combinator::map,
    error::{Error as NomError, ErrorKind},
    multi::many0,
    sequence::tuple,
    Err as NomErr, IResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub mod export;
pub mod lint;

pub use export::{
    encode_color, encode_vector3, export_tscn, ExportRequest, ExtResourceDecl, SubResourceDecl,
};
pub use lint::{
    lint_live, scene_lint, LintDiagnostic, ERR_MISSING_COLLISION_SHAPE, ERR_SCENE_PARSE,
    ERR_UNRESOLVED_RESOURCE, ERR_UNRESOLVED_SIGNAL_TARGET,
};

/// Parse failure with a 1-based line number.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("parse error at line {line}: {message}")]
pub struct ParseError {
    /// 1-based line number (`0` = whole-file problem).
    pub line: usize,
    /// Human-readable cause.
    pub message: String,
}

impl ParseError {
    /// Build a parse error.
    #[must_use]
    pub fn new(line: usize, message: String) -> Self {
        Self { line, message }
    }
}

/// Graph-build failure (unknown parent path, duplicate wiring, ...).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("build error: {message}")]
pub struct BuildError {
    /// Human-readable cause.
    pub message: String,
}

/// `[gd_scene load_steps=.. format=.. uid=..]` (or `gd_resource`) header.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SceneHeader {
    /// Declared load steps.
    pub load_steps: u32,
    /// Scene format version.
    pub format: u32,
    /// Optional `uid://...` tag.
    pub uid: Option<String>,
}

/// `[ext_resource type=.. path=.. id=..]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtResource {
    /// Resource type (`Script`, `Texture2D`, ...).
    pub resource_type: String,
    /// `res://...` path.
    pub path: String,
    /// String id referenced by `instance=ExtResource("...")`.
    pub id: String,
}

/// `[sub_resource type=.. id=..]` entry plus its body properties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubResource {
    /// Resource type (`BoxShape3D`, ...).
    pub resource_type: String,
    /// String id referenced by properties.
    pub id: String,
    /// Body properties in file order.
    pub properties: Vec<(String, String)>,
}

/// `[node ...]` declaration plus its body properties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneNodeDecl {
    /// Godot node name.
    pub name: String,
    /// Godot type (`None` = typeless instance root).
    pub node_type: Option<String>,
    /// Parent scene path (`None` = scene root, `"."` = child of root).
    pub parent: Option<String>,
    /// `instance=ExtResource("...")` target, when present.
    pub instance: Option<String>,
    /// `%Name` unique-name owner flag.
    pub unique: bool,
    /// Body properties in file order.
    pub properties: Vec<(String, String)>,
}

/// `[connection signal=.. from=.. to=.. method=..]` edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionDecl {
    /// Signal name on the source node.
    pub signal: String,
    /// Scene path of the emitting node.
    pub from: String,
    /// Scene path of the receiving node.
    pub to: String,
    /// Method invoked on the receiver.
    pub method: String,
}

/// Whole parsed scene file, in input order.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SceneFile {
    /// File header.
    pub header: SceneHeader,
    /// External resources in file order.
    pub ext_resources: Vec<ExtResource>,
    /// Sub-resources in file order.
    pub sub_resources: Vec<SubResource>,
    /// Node declarations in file order.
    pub nodes: Vec<SceneNodeDecl>,
    /// Connections in file order.
    pub connections: Vec<ConnectionDecl>,
}

/// Parsed section header: name plus key/value attributes in file order.
pub type SectionHeader = (String, Vec<(String, String)>);

/// Parse Godot 4 `.tscn` (or `.tres`) text into a [`SceneFile`].
///
/// # Errors
///
/// Returns [`ParseError`] with a 1-based line number on malformed section
/// headers or on unknown section names.
pub fn parse_tscn(input: &str) -> Result<SceneFile, ParseError> {
    let mut scene = SceneFile::default();
    let mut current: Option<CurrentSection> = None;
    for (line_no, raw) in input.lines().enumerate() {
        let line_number = line_no + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            let (_, (name, attrs)) = parse_section_header(line).map_err(|_| {
                ParseError::new(line_number, "malformed section header".to_string())
            })?;
            current = Some(begin_section(&mut scene, &name, &attrs, line_number)?);
        } else if let Some(section) = current.as_ref() {
            let (_, kv) = parse_property_line(line)
                .map_err(|_| ParseError::new(line_number, "malformed property line".to_string()))?;
            match section {
                CurrentSection::Node(index) => {
                    if let Some(decl) = scene.nodes.get_mut(*index) {
                        decl.properties.push(kv);
                    }
                }
                CurrentSection::SubResource(index) => {
                    if let Some(sub) = scene.sub_resources.get_mut(*index) {
                        sub.properties.push(kv);
                    }
                }
            }
        }
    }
    Ok(scene)
}

/// Section currently collecting body lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CurrentSection {
    /// Body lines belong to `nodes[index]`.
    Node(usize),
    /// Body lines belong to `sub_resources[index]`.
    SubResource(usize),
}

/// Dispatch one parsed section header into the scene under construction.
fn begin_section(
    scene: &mut SceneFile,
    name: &str,
    attrs: &[(String, String)],
    line: usize,
) -> Result<CurrentSection, ParseError> {
    match name {
        "gd_scene" | "gd_resource" => {
            scene.header = SceneHeader {
                load_steps: optional_u32(attrs, "load_steps").unwrap_or(1),
                format: optional_u32(attrs, "format").unwrap_or(3),
                uid: attr(attrs, "uid").map(str::to_string),
            };
            Ok(CurrentSection::Node(usize::MAX))
        }
        "ext_resource" => {
            scene.ext_resources.push(ExtResource {
                resource_type: required_attr(attrs, "type", line)?,
                path: required_attr(attrs, "path", line)?,
                id: required_attr(attrs, "id", line)?,
            });
            Ok(CurrentSection::Node(usize::MAX))
        }
        "sub_resource" => {
            scene.sub_resources.push(SubResource {
                resource_type: required_attr(attrs, "type", line)?,
                id: required_attr(attrs, "id", line)?,
                properties: Vec::new(),
            });
            Ok(CurrentSection::SubResource(scene.sub_resources.len() - 1))
        }
        "node" => {
            let name_attr = required_attr(attrs, "name", line)?;
            let parent = attr(attrs, "parent").map(str::to_string);
            let instance = attr(attrs, "instance").and_then(|raw| {
                raw.strip_prefix("ExtResource(\"")
                    .and_then(|tail| tail.strip_suffix("\")"))
                    .map(str::to_string)
            });
            let unique = name_attr.starts_with('%');
            let clean_name = if unique {
                match name_attr.strip_prefix('%') {
                    Some(stripped) => stripped.to_string(),
                    None => name_attr.clone(),
                }
            } else {
                name_attr.clone()
            };
            scene.nodes.push(SceneNodeDecl {
                name: clean_name,
                node_type: attr(attrs, "type").map(str::to_string),
                parent,
                instance,
                unique,
                properties: Vec::new(),
            });
            Ok(CurrentSection::Node(scene.nodes.len() - 1))
        }
        "connection" => {
            scene.connections.push(ConnectionDecl {
                signal: required_attr(attrs, "signal", line)?,
                from: required_attr(attrs, "from", line)?,
                to: required_attr(attrs, "to", line)?,
                method: required_attr(attrs, "method", line)?,
            });
            Ok(CurrentSection::Node(usize::MAX))
        }
        other => Err(ParseError::new(line, format!("unknown section [{other}]"))),
    }
}

/// Fetch an optional attribute value.
fn attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    for pair in attrs {
        if pair.0.as_str() == key {
            return Some(pair.1.as_str());
        }
    }
    None
}

/// Fetch a required attribute value.
///
/// # Errors
///
/// Returns [`ParseError`] when the attribute is absent.
fn required_attr(attrs: &[(String, String)], key: &str, line: usize) -> Result<String, ParseError> {
    attr(attrs, key).map_or_else(
        || {
            Err(ParseError::new(
                line,
                format!("section is missing attribute {key}"),
            ))
        },
        |v| Ok(v.to_string()),
    )
}

/// Fetch an optional integer attribute.
///
/// # Errors
///
/// Returns [`ParseError`] when the attribute is present but not an integer.
fn optional_u32(attrs: &[(String, String)], key: &str) -> Result<u32, ParseError> {
    if let Some(raw) = attr(attrs, key) {
        raw.parse::<u32>()
            .map_err(|_| ParseError::new(0, format!("attribute {key} is not an integer: {raw}")))
    } else {
        Ok(0)
    }
}

/// Fetch an optional body property value.
fn prop<'a>(props: &'a [(String, String)], key: &str) -> Option<&'a str> {
    for pair in props {
        if pair.0.as_str() == key {
            return Some(pair.1.as_str());
        }
    }
    None
}

/// Key characters for section names and attribute keys.
fn is_key_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Parse one identifier.
fn parse_key(input: &str) -> IResult<&str, &str> {
    take_while1(is_key_char)(input)
}

/// Parse a quoted string with `\"` / `\\` / `\n` / `\t` / `\r` escapes.
fn parse_quoted(input: &str) -> IResult<&str, String> {
    let (mut rest, _) = char('"')(input)?;
    let mut out = String::new();
    loop {
        if let Some(end) = rest.find(['"', '\\']) {
            let (head, tail) = rest.split_at(end);
            out.push_str(head);
            let mut chars = tail.chars();
            match chars.next() {
                Some('"') => {
                    rest = chars.as_str();
                    break;
                }
                Some('\\') => {
                    if let Some(escaped) = chars.next() {
                        match escaped {
                            'n' => out.push('\n'),
                            't' => out.push('\t'),
                            'r' => out.push('\r'),
                            '"' => out.push('"'),
                            '\\' => out.push('\\'),
                            other => {
                                out.push('\\');
                                out.push(other);
                            }
                        }
                        rest = chars.as_str();
                    } else {
                        rest = "";
                        break;
                    }
                }
                Some(_) | None => {
                    return Err(NomErr::Error(NomError::new(input, ErrorKind::Char)));
                }
            }
        } else {
            return Err(NomErr::Error(NomError::new(input, ErrorKind::Char)));
        }
    }
    Ok((rest, out))
}

/// Parse one header attribute value: quoted, `[...]`-wrapped, or bare.
fn parse_attr_value(input: &str) -> IResult<&str, String> {
    nom::branch::alt((
        parse_quoted,
        map(
            tuple((char('['), take_until("]"), char(']'))),
            |(_, inner, _): (char, &str, char)| format!("[{inner}]"),
        ),
        map(
            take_while1(|c: char| !c.is_whitespace() && c != ']'),
            str::to_string,
        ),
    ))(input)
}

/// Parse one `key = value` header attribute (leading whitespace required).
fn parse_attr(input: &str) -> IResult<&str, (String, String)> {
    let (input, _) = multispace1(input)?;
    let (input, key) = parse_key(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, value) = parse_attr_value(input)?;
    Ok((input, (key.to_string(), value)))
}

/// Parse one `[...]` section header line into its name plus attributes.
///
/// # Errors
///
/// This is a `nom` parser; callers map the failure to [`ParseError`].
pub fn parse_section_header(input: &str) -> IResult<&str, SectionHeader> {
    let (input, _) = char('[')(input)?;
    let (input, name) = parse_key(input)?;
    let (input, attrs) = many0(parse_attr)(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(']')(input)?;
    Ok((input, (name.to_string(), attrs)))
}

/// Parse one `key = <rest of line>` body property line.
///
/// # Errors
///
/// This is a `nom` parser; callers map the failure to [`ParseError`].
pub fn parse_property_line(input: &str) -> IResult<&str, (String, String)> {
    let (input, key) = parse_key(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = multispace0(input)?;
    let value = input.trim_end().to_string();
    Ok(("", (key.to_string(), value)))
}

/// Parse `Vector3(x, y, z)` into an array.
#[must_use]
pub fn parse_vector3(text: &str) -> Option<[f32; 3]> {
    let inner = text.strip_prefix("Vector3(")?.strip_suffix(')')?;
    let mut parts = inner.split(',');
    let x: f32 = parts.next()?.trim().parse::<f32>().ok()?;
    let y: f32 = parts.next()?.trim().parse::<f32>().ok()?;
    let z: f32 = parts.next()?.trim().parse::<f32>().ok()?;
    Some([x, y, z])
}

/// Decode a packed `Transform3D(12 floats)` into a [`Transform`].
fn decode_transform3d(text: &str) -> Option<Transform> {
    let inner = text.strip_prefix("Transform3D(")?.strip_suffix(')')?;
    let floats: Vec<f32> = inner
        .split(',')
        .map(|p| p.trim().parse::<f32>().ok())
        .collect::<Option<Vec<f32>>>()?;
    if floats.len() != 12 {
        return None;
    }
    // Godot serializes the basis row by row, so the axes are the columns formed
    // by reading across. Treating the 12 floats as three axis triples transposes
    // every rotation - and a parse/export round trip hides it by transposing
    // twice.
    let x = Vec3::new(floats[0], floats[3], floats[6]);
    let y = Vec3::new(floats[1], floats[4], floats[7]);
    let z = Vec3::new(floats[2], floats[5], floats[8]);
    let origin = Vec3::new(floats[9], floats[10], floats[11]);
    let sx = x.length();
    let sy = y.length();
    let sz = z.length();
    let rotation = if sx < f32::EPSILON || sy < f32::EPSILON || sz < f32::EPSILON {
        Quat::IDENTITY
    } else {
        Quat::from_mat3(&Mat3::from_cols(x / sx, y / sy, z / sz))
    };
    Some(Transform {
        position: origin,
        rotation,
        scale: Vec3::new(sx, sy, sz),
    })
}

/// Decode a node's transform from `position` / `rotation` / `scale` vector
/// properties, falling back to a packed `transform` property.
#[must_use]
pub fn decode_node_transform(props: &[(String, String)]) -> Transform {
    let position = prop(props, "position")
        .and_then(parse_vector3)
        .map_or(Vec3::ZERO, Vec3::from_array);
    let euler = prop(props, "rotation")
        .and_then(parse_vector3)
        .map_or(Vec3::ZERO, Vec3::from_array);
    let scale = prop(props, "scale")
        .and_then(parse_vector3)
        .map_or(Vec3::ONE, Vec3::from_array);
    let mut out = Transform::from_position_euler_scale(position, euler, scale);
    if let Some(packed) = prop(props, "transform").and_then(decode_transform3d) {
        if prop(props, "position").is_none() {
            out.position = packed.position;
        }
        if prop(props, "rotation").is_none() {
            out.rotation = packed.rotation;
        }
        if prop(props, "scale").is_none() {
            out.scale = packed.scale;
        }
    }
    out
}

/// Canonical scene path of a declaration: `parent/name`, with `"."` parents
/// collapsed to the bare name.
#[must_use]
pub fn full_path(decl: &SceneNodeDecl) -> String {
    match decl.parent.as_deref() {
        None | Some(".") => decl.name.clone(),
        Some(parent) => format!("{parent}/{}", decl.name),
    }
}

/// Build a [`SceneGraph`] from parsed declarations.
///
/// # Errors
///
/// Returns [`BuildError`] when a `parent` path names a node that does not
/// exist in the file.
pub fn build_graph(scene: &SceneFile) -> Result<SceneGraph, BuildError> {
    let mut graph = SceneGraph::new();
    let mut path_to_id: HashMap<String, NodeId> = HashMap::new();
    let mut root_id: Option<NodeId> = None;
    for decl in &scene.nodes {
        let node_type = decl.node_type.clone().unwrap_or_else(|| "Node".to_string());
        let id = graph
            .add_node(decl.name.clone(), node_type, None)
            .map_err(|m| BuildError { message: m })?;
        path_to_id.insert(full_path(decl), id);
        if decl.parent.is_none() && root_id.is_none() {
            root_id = Some(id);
        }
        if let Some(node) = graph.get_mut(id) {
            node.unique_name = decl.unique
                || prop(&decl.properties, "unique_name_in_owner")
                    .is_some_and(|value| value == "true");
            for (key, value) in &decl.properties {
                node.properties
                    .insert(key.clone(), PropertyValue::Raw(value.clone()));
            }
            node.transform = decode_node_transform(&decl.properties);
        }
    }
    for decl in &scene.nodes {
        if let Some(parent_path) = decl.parent.as_ref() {
            let child = path_to_id.get(&full_path(decl)).copied();
            let parent = if parent_path == "." {
                root_id
            } else {
                path_to_id.get(parent_path).copied()
            };
            if let (Some(child_id), Some(parent_id)) = (child, parent) {
                if let Some(node) = graph.get_mut(child_id) {
                    node.parent = Some(parent_id);
                }
                if let Some(parent_node) = graph.get_mut(parent_id) {
                    if !parent_node.children.contains(&child_id) {
                        parent_node.children.push(child_id);
                    }
                }
            } else {
                return Err(BuildError {
                    message: format!("unknown parent path {parent_path}"),
                });
            }
        }
    }
    if root_id.is_some() {
        graph.root = root_id;
    }
    Ok(graph)
}

/// Build a [`SignalRegistry`] from parsed connections.
///
/// Nodes carrying a `script` property are trusted to expose the called
/// method (static analysis cannot see `GDScript`); method resolution only
/// fails for scriptless targets with no declared method.
#[must_use]
pub fn build_registry(scene: &SceneFile, graph: &SceneGraph) -> SignalRegistry {
    let mut registry = SignalRegistry::new();
    for conn in &scene.connections {
        registry.add_connection(SignalConnection::new(
            conn.signal.clone(),
            conn.from.clone(),
            conn.to.clone(),
            conn.method.clone(),
        ));
    }
    if let Some(root) = graph.root {
        for node in graph.nodes.values() {
            if node.properties.contains_key("script") {
                for conn in &scene.connections {
                    if graph.resolve_path(root, conn.to.as_str()) == Some(node.id) {
                        registry.add_method(node.id, conn.method.clone());
                    }
                }
            }
        }
    }
    registry
}

/// FNV-1a 64 over bytes (deterministic across runs and platforms).
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// Lowercase base-36 (`0-9a-z`, the Godot uid alphabet subset).
fn base36(mut value: u64) -> String {
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_string();
    }
    let mut buf: Vec<u8> = Vec::new();
    while value > 0 {
        let digit = value % 36;
        let index = usize::try_from(digit).unwrap_or_default();
        buf.push(ALPHABET[index]);
        value /= 36;
    }
    buf.iter().rev().map(|b| char::from(*b)).collect()
}

/// Deterministic `uid://...` for a scene path.
///
/// Normalizes `\` to `/`, strips leading `./` or `/`, then hashes with
/// FNV-1a so the same relative path always yields the same uid.
#[must_use]
pub fn deterministic_uid(relative_path: &str) -> String {
    let normalized = relative_path.replace('\\', "/");
    let trimmed = normalized.trim_start_matches(['.', '/']);
    let hash = fnv1a_64(trimmed.as_bytes());
    format!("uid://{}", base36(hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal scene: root + one physics body with shape + one bare body.
    pub const SAMPLE: &str = "[gd_scene load_steps=3 format=3 uid=\"uid://abc123\"]\n\
        \n\
        [node name=\"Root\" type=\"Node3D\"]\n\
        \n\
        [node name=\"Body\" type=\"StaticBody3D\" parent=\".\"]\n\
        \n\
        [node name=\"Shape\" type=\"CollisionShape3D\" parent=\"Body\"]\n\
        \n\
        [node name=\"Bare\" type=\"RigidBody3D\" parent=\".\"]\n\
        \n\
        [connection signal=\"pressed\" from=\"Body\" to=\".\" method=\"_on_pressed\"]\n";

    #[test]
    fn header_parses() {
        let scene = parse_tscn(SAMPLE);
        assert!(scene.is_ok());
        if let Ok(scene) = scene {
            assert_eq!(scene.header.load_steps, 3);
            assert_eq!(scene.header.format, 3);
            assert_eq!(scene.header.uid.as_deref(), Some("uid://abc123"));
        }
    }

    #[test]
    fn nodes_and_connection_parses() {
        let scene = parse_tscn(SAMPLE);
        assert!(scene.is_ok());
        if let Ok(scene) = scene {
            assert_eq!(scene.nodes.len(), 4);
            assert_eq!(scene.connections.len(), 1);
            if let Some(conn) = scene.connections.first() {
                assert_eq!(conn.signal, "pressed");
                assert_eq!(conn.to, ".");
            }
        }
    }

    #[test]
    fn uid_is_deterministic() {
        let a = deterministic_uid("res://scenes/world.tscn");
        let b = deterministic_uid("res://scenes/world.tscn");
        let c = deterministic_uid("res://scenes/other.tscn");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("uid://"));
    }

    #[test]
    fn graph_builds_with_paths() {
        let scene = parse_tscn(SAMPLE);
        assert!(scene.is_ok());
        if let Ok(scene) = scene {
            let graph = build_graph(&scene);
            assert!(graph.is_ok());
            if let Ok(graph) = graph {
                let root = graph.root;
                assert!(root.is_some());
                if let Some(r) = root {
                    assert!(graph.resolve_path(r, "Body/Shape").is_some());
                    assert!(graph.resolve_path(r, "Bare").is_some());
                }
            }
        }
    }

    /// Regression: Godot writes the basis row by row. Reading the 12 floats as
    /// three axis triples transposes every rotation, which turned a look-down
    /// camera into a look-up one and made every top-down scene render nothing.
    #[test]
    fn transform3d_reads_rows_not_columns() {
        // A camera pitched -90 degrees about X, exactly as Godot writes it:
        // rows (1,0,0) (0,0,1) (0,-1,0), origin (5, 9, 3).
        let props = vec![(
            "transform".to_string(),
            "Transform3D(1, 0, 0, 0, 0, 1, 0, -1, 0, 5, 9, 3)".to_string(),
        )];
        let decoded = decode_node_transform(&props);
        let forward = decoded.rotation * glam::Vec3::NEG_Z;
        assert!(
            forward.y < -0.9,
            "camera should look down, forward was {forward:?}"
        );
        assert!((decoded.position.x - 5.0).abs() < f32::EPSILON);
        assert!((decoded.position.y - 9.0).abs() < f32::EPSILON);
        assert!((decoded.position.z - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn unknown_section_is_rejected() {
        let res = parse_tscn("[weird x=1]\n");
        assert!(res.is_err());
    }
}

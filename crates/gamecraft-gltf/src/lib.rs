//! `gamecraft-gltf`: deterministic glTF 2.0 export of Godot 4 scenes.
//!
//! A [`SceneGraph`] plus the scene's sub-resources becomes one glTF asset:
//!
//! - one glTF node per scene node, with local translation/rotation/scale, the
//!   same hierarchy and the same names;
//! - every `MeshInstance3D` whose `mesh` is a Godot primitive (`BoxMesh`,
//!   `SphereMesh`, ...) becomes a flat-shaded mesh, tessellated by [`geometry`]
//!   (the same triangles the desktop viewport draws);
//! - `material_override` (or the mesh's own `material`) becomes a PBR material,
//!   with Godot's sRGB `albedo_color` converted to glTF's linear
//!   `baseColorFactor`; identical materials and identical mesh+material pairs
//!   are shared, not duplicated;
//! - `DirectionalLight3D`, `OmniLight3D`, `SpotLight3D` become
//!   `KHR_lights_punctual` lights; `Camera3D` becomes a perspective camera.
//!
//! - textures: a material with `uv1_triplanar = true` and `ExtResource`
//!   textures exports `albedo_texture` (base colour), `orm_texture` (or a
//!   `roughness_texture` and `metallic_texture` that are the same image) as the
//!   metallic-roughness and occlusion map, and `normal_texture` when
//!   `normal_enabled`. Images are embedded unchanged (PNG or JPEG). UVs are
//!   generated from Godot's triplanar formula for the dominant axis of each
//!   face; Godot's blend between axes at curved edges is not reproduced.
//!   Anything else a material refers to (non-triplanar UVs, emission, height,
//!   world triplanar, missing files) is not exported and is counted in
//!   [`Stats::textures_skipped`], never silently approximated.
//!
//! Same scene in, same bytes out: no timestamps, stable ordering. Collision
//! shapes, scripts and animation are not exported (they are not visual
//! geometry, or the generated scenes do not use them).
//!
//! Light intensity is copied from Godot's unitless `light_energy`; it is not a
//! physical lux/candela conversion, so viewers may need an exposure tweak.

pub mod geometry;

use gamecraft_core::{NodeId, PropertyValue, SceneGraph, SceneNode, Transform};
use glam::Vec3;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

/// One scene sub-resource (`[sub_resource type=... id=...]` plus its body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubRes {
    /// Resource type, e.g. `BoxMesh`, `StandardMaterial3D`.
    pub kind: String,
    /// Reference id used by `SubResource("id")`.
    pub id: String,
    /// Body properties in Godot syntax.
    pub properties: Vec<(String, String)>,
}

impl From<&gamecraft_godot::SubResource> for SubRes {
    fn from(sub: &gamecraft_godot::SubResource) -> Self {
        Self {
            kind: sub.resource_type.clone(),
            id: sub.id.clone(),
            properties: sub.properties.clone(),
        }
    }
}

/// What an export contains.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// glTF nodes (one per scene node).
    pub nodes: usize,
    /// Distinct meshes after sharing.
    pub meshes: usize,
    /// Distinct materials after sharing.
    pub materials: usize,
    /// Triangles across all distinct meshes.
    pub triangles: usize,
    /// Punctual lights.
    pub lights: usize,
    /// Cameras.
    pub cameras: usize,
    /// Distinct images embedded in the asset.
    pub textures: usize,
    /// Texture properties a material set that were not exported.
    pub textures_skipped: usize,
}

/// Texture files by Godot `ExtResource` id: the raw PNG or JPEG bytes.
pub type Textures = BTreeMap<String, Vec<u8>>;

/// A finished export: the glTF JSON document plus its binary buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct Gltf {
    /// The glTF JSON (buffer 0 has no `uri`; see [`Gltf::to_glb`] and
    /// [`Gltf::to_gltf_embedded`]).
    pub json: Value,
    /// Buffer 0: positions and normals, little-endian `f32`.
    pub bin: Vec<u8>,
    /// Counts for summaries.
    pub stats: Stats,
}

/// GLB magic `glTF`.
const GLB_MAGIC: u32 = 0x4654_6C67;
/// GLB chunk type `JSON`.
const CHUNK_JSON: u32 = 0x4E4F_534A;
/// GLB chunk type `BIN\0`.
const CHUNK_BIN: u32 = 0x004E_4942;

impl Gltf {
    /// Binary glTF (`.glb`): header, JSON chunk, BIN chunk, 4-byte aligned.
    ///
    /// # Errors
    ///
    /// Returns a message if the asset exceeds the 4 GiB GLB limit.
    pub fn to_glb(&self) -> Result<Vec<u8>, String> {
        let mut json = serde_json::to_string(&self.json)
            .map_err(|e| e.to_string())?
            .into_bytes();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        let mut bin = self.bin.clone();
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        let bin_chunk = if bin.is_empty() { 0 } else { 8 + bin.len() };
        let total = 12 + 8 + json.len() + bin_chunk;
        let u32_of =
            |n: usize| u32::try_from(n).map_err(|_| "asset too large for GLB (4 GiB)".to_string());
        let mut out = Vec::with_capacity(total);
        for word in [
            GLB_MAGIC,
            2,
            u32_of(total)?,
            u32_of(json.len())?,
            CHUNK_JSON,
        ] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend_from_slice(&json);
        if !bin.is_empty() {
            out.extend_from_slice(&u32_of(bin.len())?.to_le_bytes());
            out.extend_from_slice(&CHUNK_BIN.to_le_bytes());
            out.extend_from_slice(&bin);
        }
        Ok(out)
    }

    /// Text glTF (`.gltf`) with the buffer embedded as a base64 data URI: one
    /// self-contained string, suitable for an MCP result or a diff.
    #[must_use]
    pub fn to_gltf_embedded(&self) -> String {
        let mut json = self.json.clone();
        if let Some(buffer) = json
            .pointer_mut("/buffers/0")
            .and_then(Value::as_object_mut)
        {
            buffer.insert(
                "uri".to_string(),
                Value::String(format!(
                    "data:application/octet-stream;base64,{}",
                    base64(&self.bin)
                )),
            );
        }
        serde_json::to_string(&json).unwrap_or_default()
    }
}

/// Standard base64 with padding (RFC 4648).
#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk.first().copied().unwrap_or(0);
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let n = (u32::from(b0) << 16) | (u32::from(b1) << 8) | u32::from(b2);
        let sextets = [(n >> 18) & 63, (n >> 12) & 63, (n >> 6) & 63, n & 63];
        for (i, s) in sextets.iter().enumerate() {
            if i <= chunk.len() {
                let index = usize::try_from(*s).unwrap_or(0);
                out.push(char::from(TABLE.get(index).copied().unwrap_or(b'A')));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Export a parsed `.tscn` file.
///
/// # Errors
///
/// Returns a message if the scene does not build into a graph or has no root.
pub fn from_scene_file(file: &gamecraft_godot::SceneFile) -> Result<Gltf, String> {
    let graph = gamecraft_godot::build_graph(file).map_err(|e| e.to_string())?;
    let subs: Vec<SubRes> = file.sub_resources.iter().map(SubRes::from).collect();
    export(&graph, &subs)
}

/// Export `.tscn` source text.
///
/// # Errors
///
/// Returns a message if the text does not parse or build.
pub fn from_tscn(text: &str) -> Result<Gltf, String> {
    let file = gamecraft_godot::parse_tscn(text).map_err(|e| e.to_string())?;
    from_scene_file(&file)
}

/// Export a scene graph with its sub-resources, without textures.
///
/// # Errors
///
/// Returns a message if the graph has no root node.
pub fn export(graph: &SceneGraph, subs: &[SubRes]) -> Result<Gltf, String> {
    export_textured(graph, subs, &Textures::new())
}

/// Read the image files a scene's `ext_resource`s point at (`res://` paths below
/// `project_dir`, `png`/`jpg`/`jpeg` only). Missing or unreadable files and paths
/// that climb out of the project are left out; the export counts them as skipped.
#[must_use]
pub fn load_textures(file: &gamecraft_godot::SceneFile, project_dir: &Path) -> Textures {
    file.ext_resources
        .iter()
        .filter_map(|ext| {
            let relative = Path::new(ext.path.strip_prefix("res://")?);
            let image = relative
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| {
                    ["png", "jpg", "jpeg"]
                        .iter()
                        .any(|known| e.eq_ignore_ascii_case(known))
                });
            let inside = relative
                .components()
                .all(|c| matches!(c, Component::Normal(_)));
            if !image || !inside {
                return None;
            }
            let bytes = std::fs::read(project_dir.join(relative)).ok()?;
            Some((ext.id.clone(), bytes))
        })
        .collect()
}

/// The Godot project folder for a scene: the nearest parent with a `project.godot`,
/// else the scene's own folder.
#[must_use]
pub fn project_root(scene: &Path) -> PathBuf {
    scene
        .ancestors()
        .skip(1)
        .find(|dir| dir.join("project.godot").is_file())
        .or_else(|| scene.parent())
        .map_or_else(PathBuf::new, Path::to_path_buf)
}

/// Export a `.tscn` file with the textures its materials use.
///
/// # Errors
///
/// Returns a message if the file cannot be read, parsed or built into a graph.
pub fn from_tscn_file(path: &Path) -> Result<Gltf, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file = gamecraft_godot::parse_tscn(text.as_str()).map_err(|e| e.to_string())?;
    let graph = gamecraft_godot::build_graph(&file).map_err(|e| e.to_string())?;
    let subs: Vec<SubRes> = file.sub_resources.iter().map(SubRes::from).collect();
    let textures = load_textures(&file, project_root(path).as_path());
    export_textured(&graph, &subs, &textures)
}

/// Export a scene graph with its sub-resources and the textures its materials
/// refer to (keyed by `ExtResource` id).
///
/// # Errors
///
/// Returns a message if the graph has no root node.
pub fn export_textured(
    graph: &SceneGraph,
    subs: &[SubRes],
    textures: &Textures,
) -> Result<Gltf, String> {
    let root = graph
        .root
        .ok_or_else(|| "scene has no root node".to_string())?;
    let mut builder = Builder::new(graph, subs, textures);
    let root_index = builder
        .visit(root)
        .ok_or_else(|| "root node is missing from the graph".to_string())?;
    let root_name = graph.get(root).map_or("scene", |n| n.name.as_str());
    Ok(builder.finish(root_name, root_index))
}

/// Accumulates glTF arrays while walking the scene.
struct Builder<'a> {
    graph: &'a SceneGraph,
    subs: &'a [SubRes],
    nodes: Vec<Value>,
    meshes: Vec<Value>,
    materials: Vec<Value>,
    accessors: Vec<Value>,
    views: Vec<Value>,
    lights: Vec<Value>,
    cameras: Vec<Value>,
    bin: Vec<u8>,
    /// (mesh type + sorted properties, material index) -> mesh index. Keyed by
    /// geometry, not id: generated scenes often declare one `BoxMesh` per wall
    /// with identical sizes.
    mesh_cache: BTreeMap<(String, usize), Option<usize>>,
    /// Canonical material JSON -> material index.
    material_cache: BTreeMap<String, usize>,
    triangles: usize,
    textures: &'a Textures,
    images: Vec<Value>,
    gl_textures: Vec<Value>,
    /// `ExtResource` id -> glTF texture index.
    texture_cache: BTreeMap<String, usize>,
    textures_skipped: usize,
}

/// UV generation for a textured material (Godot's `uv1_scale` / `uv1_offset`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Uv {
    /// `uv1_scale`.
    pub scale: Vec3,
    /// `uv1_offset`.
    pub offset: Vec3,
}

/// The albedo texture of a triplanar material: its `ExtResource` id and UV mapping.
///
/// Same rule the exporter uses: `uv1_triplanar = true`, not `uv1_world_triplanar`, and
/// an `albedo_texture = ExtResource("id")`. Anything else is not textured here.
#[must_use]
pub fn triplanar_albedo(properties: &[(String, String)]) -> Option<(String, Uv)> {
    let flag = |key: &str| prop(properties, key).is_some_and(|v| v.trim() == "true");
    if !flag("uv1_triplanar") || flag("uv1_world_triplanar") {
        return None;
    }
    let id = sub_id(prop(properties, "albedo_texture")?)?.to_string();
    Some((
        id,
        Uv {
            scale: geometry::vector3(properties, "uv1_scale").unwrap_or(Vec3::ONE),
            offset: geometry::vector3(properties, "uv1_offset").unwrap_or(Vec3::ZERO),
        },
    ))
}

impl<'a> Builder<'a> {
    fn new(graph: &'a SceneGraph, subs: &'a [SubRes], textures: &'a Textures) -> Self {
        Self {
            graph,
            subs,
            nodes: Vec::new(),
            meshes: Vec::new(),
            materials: Vec::new(),
            accessors: Vec::new(),
            views: Vec::new(),
            lights: Vec::new(),
            cameras: Vec::new(),
            bin: Vec::new(),
            mesh_cache: BTreeMap::new(),
            material_cache: BTreeMap::new(),
            triangles: 0,
            textures,
            images: Vec::new(),
            gl_textures: Vec::new(),
            texture_cache: BTreeMap::new(),
            textures_skipped: 0,
        }
    }

    fn sub(&self, id: &str) -> Option<&'a SubRes> {
        self.subs.iter().find(|s| s.id == id)
    }

    /// Depth-first: a node gets its index before its children, so indices follow
    /// file order and are stable.
    fn visit(&mut self, id: NodeId) -> Option<usize> {
        let graph = self.graph;
        let node = graph.get(id)?;
        // A collision shape is physics, not geometry; as a leaf it would only be
        // an empty node. Named markers (spawn points, triggers) are kept.
        if node.node_type == "CollisionShape3D" && node.children.is_empty() {
            return None;
        }
        let index = self.nodes.len();
        self.nodes.push(Value::Null);
        let mut obj = Map::new();
        obj.insert("name".to_string(), json!(node.name));
        trs(&mut obj, &node.transform);
        if let Some(mesh) = self.mesh_for(node) {
            obj.insert("mesh".to_string(), json!(mesh));
        }
        match node.node_type.as_str() {
            "Camera3D" => {
                obj.insert("camera".to_string(), json!(self.cameras.len()));
                self.cameras.push(camera(node));
            }
            "DirectionalLight3D" | "OmniLight3D" | "SpotLight3D" => {
                let light = self.lights.len();
                self.lights.push(punctual(node));
                obj.insert(
                    "extensions".to_string(),
                    json!({"KHR_lights_punctual": {"light": light}}),
                );
            }
            _ => {}
        }
        let children: Vec<usize> = node
            .children
            .iter()
            .filter_map(|c| self.visit(*c))
            .collect();
        if !children.is_empty() {
            obj.insert("children".to_string(), json!(children));
        }
        if let Some(slot) = self.nodes.get_mut(index) {
            *slot = Value::Object(obj);
        }
        Some(index)
    }

    /// The glTF mesh for a node's `mesh = SubResource(...)`, built once per
    /// (mesh, material) pair.
    fn mesh_for(&mut self, node: &SceneNode) -> Option<usize> {
        let mesh_id = node
            .properties
            .get("mesh")
            .and_then(PropertyValue::as_str)
            .and_then(sub_id)?;
        let mesh = self
            .sub(mesh_id)
            .filter(|s| geometry::is_primitive_mesh(s.kind.as_str()))?;
        let material_sub = node
            .properties
            .get("material_override")
            .and_then(PropertyValue::as_str)
            .and_then(sub_id)
            .or_else(|| prop(&mesh.properties, "material").and_then(sub_id))
            .and_then(|id| self.sub(id));
        let (material, uv) = self.material_for(material_sub);
        let mut shape = mesh.properties.clone();
        shape.sort();
        let key = (format!("{}|{shape:?}|{uv:?}", mesh.kind), material);
        if let Some(cached) = self.mesh_cache.get(&key) {
            return *cached;
        }
        let built = self.build_mesh(mesh, material, uv);
        self.mesh_cache.insert(key, built);
        built
    }

    fn build_mesh(&mut self, mesh: &SubRes, material: usize, uv: Option<Uv>) -> Option<usize> {
        let mut positions: Vec<Vec3> = Vec::new();
        let mut normals: Vec<Vec3> = Vec::new();
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        for tri in geometry::shape_triangles(mesh.kind.as_str(), &mesh.properties) {
            if let Some((points, normal)) = geometry::oriented(&tri) {
                positions.extend_from_slice(&points);
                normals.extend_from_slice(&[normal; 3]);
                if let Some(uv) = uv {
                    uvs.extend_from_slice(&triplanar_uvs(&points, normal, uv));
                }
            }
        }
        if positions.is_empty() {
            return None;
        }
        self.triangles += positions.len() / 3;
        let position = self.push_vec3(&positions, true);
        let normal = self.push_vec3(&normals, false);
        let mut attributes = json!({"POSITION": position, "NORMAL": normal});
        if uv.is_some() {
            attributes["TEXCOORD_0"] = json!(self.push_vec2(&uvs));
        }
        let index = self.meshes.len();
        self.meshes.push(json!({
            "name": mesh.id,
            "primitives": [{
                "attributes": attributes,
                "material": material,
                "mode": 4
            }]
        }));
        Some(index)
    }

    /// Append a VEC2 float accessor and return its index.
    fn push_vec2(&mut self, data: &[[f32; 2]]) -> usize {
        let offset = self.bin.len();
        for pair in data {
            for c in pair {
                self.bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        let view = self.views.len();
        self.views.push(json!({
            "buffer": 0,
            "byteOffset": offset,
            "byteLength": self.bin.len() - offset,
            "target": 34962
        }));
        let index = self.accessors.len();
        self.accessors.push(json!({
            "bufferView": view,
            "componentType": 5126,
            "count": data.len(),
            "type": "VEC2"
        }));
        index
    }

    /// Append a VEC3 float accessor (with min/max when asked) and return its index.
    fn push_vec3(&mut self, data: &[Vec3], bounds: bool) -> usize {
        let offset = self.bin.len();
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for v in data {
            for c in v.to_array() {
                self.bin.extend_from_slice(&c.to_le_bytes());
            }
            min = min.min(*v);
            max = max.max(*v);
        }
        let view = self.views.len();
        self.views.push(json!({
            "buffer": 0,
            "byteOffset": offset,
            "byteLength": self.bin.len() - offset,
            "target": 34962
        }));
        let mut accessor = json!({
            "bufferView": view,
            "componentType": 5126,
            "count": data.len(),
            "type": "VEC3"
        });
        if bounds {
            accessor["min"] = json!(min.to_array().map(f64::from));
            accessor["max"] = json!(max.to_array().map(f64::from));
        }
        let index = self.accessors.len();
        self.accessors.push(accessor);
        index
    }

    /// Material index for a `StandardMaterial3D` / `ORMMaterial3D` (or the neutral
    /// default), plus the UV generation when textures were bound.
    fn material_for(&mut self, sub: Option<&SubRes>) -> (usize, Option<Uv>) {
        let mut uv = None;
        let value = match sub {
            None => material_json("default", [0.8, 0.8, 0.8, 1.0], 0.0, 1.0),
            Some(s) => {
                let colour = prop(&s.properties, "albedo_color")
                    .and_then(parse_color)
                    .unwrap_or([1.0, 1.0, 1.0, 1.0]);
                let metallic = prop(&s.properties, "metallic")
                    .and_then(|t| t.trim().parse::<f64>().ok())
                    .unwrap_or(0.0);
                let roughness = prop(&s.properties, "roughness")
                    .and_then(|t| t.trim().parse::<f64>().ok())
                    .unwrap_or(1.0);
                let mut value = material_json(s.id.as_str(), colour, metallic, roughness);
                uv = self.bind_textures(&mut value, s);
                value
            }
        };
        // Share by look, not by name: two ids with the same colour are one material.
        let mut look = value.clone();
        if let Some(obj) = look.as_object_mut() {
            obj.remove("name");
        }
        let key = look.to_string();
        if let Some(index) = self.material_cache.get(&key) {
            return (*index, uv);
        }
        let index = self.materials.len();
        self.materials.push(value);
        self.material_cache.insert(key, index);
        (index, uv)
    }

    /// Attach the images a material refers to. Only triplanar materials can be
    /// exported (their UVs follow from the geometry); everything not bound is
    /// counted in `textures_skipped`.
    fn bind_textures(&mut self, material: &mut Value, sub: &SubRes) -> Option<Uv> {
        let refs: Vec<(&str, &str)> = sub
            .properties
            .iter()
            .filter(|(key, _)| key.ends_with("_texture"))
            .filter_map(|(key, value)| Some((key.as_str(), sub_id(value)?)))
            .collect();
        if refs.is_empty() {
            return None;
        }
        let flag = |key: &str| prop(&sub.properties, key).is_some_and(|v| v.trim() == "true");
        if !flag("uv1_triplanar") || flag("uv1_world_triplanar") {
            self.textures_skipped += refs.len();
            return None;
        }
        let id_of = |name: &str| refs.iter().find(|(key, _)| *key == name).map(|(_, id)| *id);
        let orm = id_of("orm_texture").or_else(|| {
            match (id_of("roughness_texture"), id_of("metallic_texture")) {
                (Some(r), Some(m)) if r == m => Some(r),
                _ => None,
            }
        });
        let mut used: Vec<&str> = Vec::new();
        if let Some(index) = id_of("albedo_texture").and_then(|id| self.texture_index(id)) {
            material["pbrMetallicRoughness"]["baseColorTexture"] = json!({"index": index});
            used.push("albedo_texture");
        }
        if let Some(index) = orm.and_then(|id| self.texture_index(id)) {
            material["pbrMetallicRoughness"]["metallicRoughnessTexture"] = json!({"index": index});
            let packed = id_of("orm_texture").is_some() || id_of("ao_texture") == orm;
            if packed {
                material["occlusionTexture"] = json!({"index": index});
            }
            for key in [
                "orm_texture",
                "roughness_texture",
                "metallic_texture",
                "ao_texture",
            ] {
                if id_of(key) == orm && (packed || key != "ao_texture") {
                    used.push(key);
                }
            }
        }
        if flag("normal_enabled") {
            if let Some(index) = id_of("normal_texture").and_then(|id| self.texture_index(id)) {
                material["normalTexture"] = json!({"index": index});
                used.push("normal_texture");
            }
        }
        if used.is_empty() {
            self.textures_skipped += refs.len();
            return None;
        }
        self.textures_skipped += refs.len() - used.len();
        Some(Uv {
            scale: geometry::vector3(&sub.properties, "uv1_scale").unwrap_or(Vec3::ONE),
            offset: geometry::vector3(&sub.properties, "uv1_offset").unwrap_or(Vec3::ZERO),
        })
    }

    /// glTF texture index for an `ExtResource` id, embedding its image on first use.
    fn texture_index(&mut self, id: &str) -> Option<usize> {
        if let Some(index) = self.texture_cache.get(id) {
            return Some(*index);
        }
        let textures = self.textures;
        let bytes = textures.get(id)?;
        let mime = image_mime(bytes)?;
        let offset = self.bin.len();
        self.bin.extend_from_slice(bytes);
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        let view = self.views.len();
        self.views.push(json!({
            "buffer": 0,
            "byteOffset": offset,
            "byteLength": bytes.len()
        }));
        let image = self.images.len();
        self.images
            .push(json!({"bufferView": view, "mimeType": mime, "name": id}));
        let index = self.gl_textures.len();
        self.gl_textures
            .push(json!({"sampler": 0, "source": image}));
        self.texture_cache.insert(id.to_string(), index);
        Some(index)
    }

    fn finish(self, root_name: &str, root_index: usize) -> Gltf {
        let stats = Stats {
            nodes: self.nodes.len(),
            meshes: self.meshes.len(),
            materials: self.materials.len(),
            triangles: self.triangles,
            lights: self.lights.len(),
            cameras: self.cameras.len(),
            textures: self.images.len(),
            textures_skipped: self.textures_skipped,
        };
        let has_textures = !self.gl_textures.is_empty();
        let mut doc = Map::new();
        doc.insert(
            "asset".to_string(),
            json!({"version": "2.0", "generator": concat!("GameCraft gamecraft-gltf ", env!("CARGO_PKG_VERSION"))}),
        );
        doc.insert("scene".to_string(), json!(0));
        doc.insert(
            "scenes".to_string(),
            json!([{"name": root_name, "nodes": [root_index]}]),
        );
        doc.insert("nodes".to_string(), Value::Array(self.nodes));
        for (key, list) in [
            ("meshes", self.meshes),
            ("materials", self.materials),
            ("accessors", self.accessors),
            ("bufferViews", self.views),
            ("cameras", self.cameras),
            ("images", self.images),
            ("textures", self.gl_textures),
        ] {
            if !list.is_empty() {
                doc.insert(key.to_string(), Value::Array(list));
            }
        }
        if has_textures {
            doc.insert(
                "samplers".to_string(),
                json!([{"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}]),
            );
        }
        if !self.bin.is_empty() {
            doc.insert(
                "buffers".to_string(),
                json!([{"byteLength": self.bin.len()}]),
            );
        }
        if !self.lights.is_empty() {
            doc.insert("extensionsUsed".to_string(), json!(["KHR_lights_punctual"]));
            doc.insert(
                "extensions".to_string(),
                json!({"KHR_lights_punctual": {"lights": self.lights}}),
            );
        }
        Gltf {
            json: Value::Object(doc),
            bin: self.bin,
            stats,
        }
    }
}

/// Write translation/rotation/scale, omitting identity parts.
fn trs(obj: &mut Map<String, Value>, t: &Transform) {
    if t.position != Vec3::ZERO {
        obj.insert(
            "translation".to_string(),
            json!(t.position.to_array().map(f64::from)),
        );
    }
    if t.rotation != glam::Quat::IDENTITY {
        let q = t.rotation.normalize();
        obj.insert("rotation".to_string(), json!(q.to_array().map(f64::from)));
    }
    if t.scale != Vec3::ONE {
        obj.insert(
            "scale".to_string(),
            json!(t.scale.to_array().map(f64::from)),
        );
    }
}

/// `image/png` or `image/jpeg` from the magic bytes; anything else is not embedded.
fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else {
        None
    }
}

/// Godot's triplanar mapping for the dominant axis of a flat face:
/// `q = (position * uv1_scale + uv1_offset) * (1, -1, 1)`, then `q.zy * (-1, 1)`
/// for X faces, `q.xz` for Y faces and `q.xy` for Z faces.
#[must_use]
pub fn triplanar_uvs(points: &[Vec3; 3], normal: Vec3, uv: Uv) -> [[f32; 2]; 3] {
    let (ax, ay, az) = (normal.x.abs(), normal.y.abs(), normal.z.abs());
    let axis = if ax >= ay && ax >= az {
        0
    } else if ay >= az {
        1
    } else {
        2
    };
    points.map(|p| {
        let q = p * uv.scale + uv.offset;
        let q = Vec3::new(q.x, -q.y, q.z);
        match axis {
            0 => [-q.z, q.y],
            1 => [q.x, q.z],
            _ => [q.x, q.y],
        }
    })
}

/// A PBR material value.
fn material_json(name: &str, srgba: [f64; 4], metallic: f64, roughness: f64) -> Value {
    let [r, g, b, a] = srgba;
    let mut value = json!({
        "name": name,
        "pbrMetallicRoughness": {
            "baseColorFactor": [tidy(to_linear(r)), tidy(to_linear(g)), tidy(to_linear(b)), tidy(a.clamp(0.0, 1.0))],
            "metallicFactor": tidy(metallic.clamp(0.0, 1.0)),
            "roughnessFactor": tidy(roughness.clamp(0.0, 1.0))
        }
    });
    if a < 1.0 {
        value["alphaMode"] = json!("BLEND");
    }
    value
}

/// A `KHR_lights_punctual` light from a Godot light node.
fn punctual(node: &SceneNode) -> Value {
    let colour = node
        .properties
        .get("light_color")
        .and_then(PropertyValue::as_str)
        .and_then(parse_color)
        .unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let mut light = json!({
        "name": node.name,
        "color": [tidy(to_linear(colour[0])), tidy(to_linear(colour[1])), tidy(to_linear(colour[2]))],
        "intensity": tidy(num(node, "light_energy").unwrap_or(1.0).max(0.0)),
    });
    match node.node_type.as_str() {
        "OmniLight3D" => {
            light["type"] = json!("point");
            light["range"] = json!(tidy(num(node, "omni_range").unwrap_or(5.0).max(0.001)));
        }
        "SpotLight3D" => {
            light["type"] = json!("spot");
            light["range"] = json!(tidy(num(node, "spot_range").unwrap_or(5.0).max(0.001)));
            let outer = num(node, "spot_angle")
                .unwrap_or(45.0)
                .clamp(0.1, 89.9)
                .to_radians();
            light["spot"] = json!({"innerConeAngle": 0.0, "outerConeAngle": tidy(outer)});
        }
        _ => light["type"] = json!("directional"),
    }
    light
}

/// A perspective camera from a Godot `Camera3D` (`fov` is vertical, in degrees).
fn camera(node: &SceneNode) -> Value {
    let fov = num(node, "fov").unwrap_or(75.0).clamp(1.0, 179.0);
    let near = num(node, "near").unwrap_or(0.05).max(0.0001);
    let far = num(node, "far").unwrap_or(4000.0).max(near * 2.0);
    json!({
        "name": node.name,
        "type": "perspective",
        "perspective": {"yfov": tidy(fov.to_radians()), "znear": tidy(near), "zfar": tidy(far)}
    })
}

/// A numeric node property, whatever form the parser left it in.
fn num(node: &SceneNode, key: &str) -> Option<f64> {
    match node.properties.get(key)? {
        PropertyValue::Float(f) => Some(*f),
        PropertyValue::Int(i) => i.to_string().parse().ok(),
        PropertyValue::Str(s) | PropertyValue::Raw(s) => s.trim().parse().ok(),
        PropertyValue::Bool(_) | PropertyValue::Vec3(_) => None,
    }
}

/// A sub-resource body property.
fn prop<'p>(properties: &'p [(String, String)], key: &str) -> Option<&'p str> {
    properties
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// `id` out of `SubResource("id")` / `ExtResource("id")`.
#[must_use]
pub fn sub_id(reference: &str) -> Option<&str> {
    let start = reference.find('"')? + 1;
    let end = reference.rfind('"')?;
    if end <= start {
        return None;
    }
    reference.get(start..end)
}

/// `Color(r, g, b[, a])` as sRGB components.
#[must_use]
pub fn parse_color(text: &str) -> Option<[f64; 4]> {
    let inner = text.trim().strip_prefix("Color(")?.strip_suffix(')')?;
    let parts: Vec<f64> = inner
        .split(',')
        .map(|p| p.trim().parse::<f64>())
        .collect::<Result<_, _>>()
        .ok()?;
    match parts.as_slice() {
        [r, g, b] => Some([*r, *g, *b, 1.0]),
        [r, g, b, a] => Some([*r, *g, *b, *a]),
        _ => None,
    }
}

/// sRGB component to linear (the IEC 61966-2-1 curve).
#[must_use]
pub fn to_linear(c: f64) -> f64 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Round to 6 decimals so factors read cleanly and diff stably.
fn tidy(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENE: &str = r#"[gd_scene load_steps=4 format=3]

[sub_resource type="BoxMesh" id="box"]
size = Vector3(2, 1, 1)

[sub_resource type="StandardMaterial3D" id="red"]
albedo_color = Color(1, 0, 0, 1)

[sub_resource type="StandardMaterial3D" id="red_again"]
albedo_color = Color(1, 0, 0, 1)

[sub_resource type="BoxShape3D" id="shape"]
size = Vector3(2, 1, 1)

[node name="World" type="Node3D"]

[node name="A" type="MeshInstance3D" parent="."]
mesh = SubResource("box")
material_override = SubResource("red")

[node name="B" type="MeshInstance3D" parent="."]
transform = Transform3D(1, 0, 0, 0, 1, 0, 0, 0, 1, 3, 0, 0)
mesh = SubResource("box")
material_override = SubResource("red_again")

[node name="Body" type="StaticBody3D" parent="."]

[node name="Shape" type="CollisionShape3D" parent="Body"]
shape = SubResource("shape")

[node name="Sun" type="DirectionalLight3D" parent="."]
light_energy = 1.5

[node name="Cam" type="Camera3D" parent="."]
fov = 60.0
"#;

    fn exported() -> Gltf {
        let result = from_tscn(SCENE);
        assert!(result.is_ok(), "{result:?}");
        result.unwrap_or_else(|_| Gltf {
            json: Value::Null,
            bin: Vec::new(),
            stats: Stats::default(),
        })
    }

    #[test]
    fn structure_and_sharing() {
        let g = exported();
        assert_eq!(g.stats.nodes, 6, "collision shape leaf dropped");
        assert_eq!(
            g.stats.meshes, 1,
            "same box and same red look share one mesh"
        );
        assert_eq!(g.stats.materials, 1);
        assert_eq!(g.stats.triangles, 12);
        assert_eq!((g.stats.lights, g.stats.cameras), (1, 1));
        assert_eq!(
            g.json.pointer("/nodes/2/translation"),
            Some(&json!([3.0, 0.0, 0.0]))
        );
        assert_eq!(
            g.json.pointer("/accessors/0/max"),
            Some(&json!([1.0, 0.5, 0.5]))
        );
        assert_eq!(
            g.json.pointer("/extensionsUsed/0"),
            Some(&json!("KHR_lights_punctual"))
        );
        // The collision shape is not visual geometry: dropped, so Sun is node 4.
        assert_eq!(g.json.pointer("/nodes/4/name"), Some(&json!("Sun")));
        assert_eq!(g.json.pointer("/nodes/3/name"), Some(&json!("Body")));
        assert_eq!(
            g.bin.len(),
            12 * 3 * 2 * 12,
            "positions + normals, 12 bytes each"
        );
    }

    #[test]
    fn glb_layout_is_valid() {
        let glb = exported().to_glb().unwrap_or_default();
        assert_eq!(glb.get(0..4), Some(&b"glTF"[..]));
        let word = |at: usize| {
            glb.get(at..at + 4)
                .and_then(|b| b.try_into().ok())
                .map_or(0, u32::from_le_bytes)
        };
        assert_eq!(word(4), 2);
        assert_eq!(usize::try_from(word(8)).ok(), Some(glb.len()));
        let json_len = usize::try_from(word(12)).unwrap_or(0);
        assert!(json_len.is_multiple_of(4));
        assert_eq!(word(16), CHUNK_JSON);
        assert_eq!(word(20 + json_len + 4), CHUNK_BIN);
        assert!(glb.len().is_multiple_of(4));
    }

    #[test]
    fn export_is_deterministic() {
        assert_eq!(exported().to_glb(), exported().to_glb());
    }

    #[test]
    fn embedded_text_carries_the_buffer() {
        let text = exported().to_gltf_embedded();
        assert!(text.contains("\"uri\":\"data:application/octet-stream;base64,"));
        assert!(serde_json::from_str::<Value>(&text).is_ok());
    }

    #[test]
    fn colour_and_base64_helpers() {
        assert!((to_linear(0.5) - 0.214_041).abs() < 1e-6);
        assert_eq!(parse_color("Color(1, 0.5, 0)"), Some([1.0, 0.5, 0.0, 1.0]));
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(sub_id("SubResource(\"box\")"), Some("box"));
    }

    const TEXTURED: &str = r#"[gd_scene load_steps=6 format=3]

[ext_resource type="Texture2D" path="res://t/a.png" id="1_a"]
[ext_resource type="Texture2D" path="res://t/o.png" id="2_o"]
[ext_resource type="Texture2D" path="res://t/n.png" id="3_n"]

[sub_resource type="BoxMesh" id="box"]
size = Vector3(2, 1, 1)

[sub_resource type="ORMMaterial3D" id="stone"]
uv1_scale = Vector3(0.5, 0.5, 0.5)
uv1_triplanar = true
albedo_texture = ExtResource("1_a")
orm_texture = ExtResource("2_o")
normal_enabled = true
normal_texture = ExtResource("3_n")

[node name="World" type="Node3D"]

[node name="Block" type="MeshInstance3D" parent="."]
mesh = SubResource("box")
material_override = SubResource("stone")
"#;

    fn fake_image(tag: u8) -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        bytes.extend_from_slice(&[tag; 5]);
        bytes
    }

    fn textures() -> Textures {
        [("1_a", 1), ("2_o", 2), ("3_n", 3)]
            .into_iter()
            .map(|(id, tag)| (id.to_string(), fake_image(tag)))
            .collect()
    }

    fn export_with(text: &str, textures: &Textures) -> Gltf {
        let built = gamecraft_godot::parse_tscn(text)
            .map_err(|e| e.to_string())
            .and_then(|file| {
                let graph = gamecraft_godot::build_graph(&file).map_err(|e| e.to_string())?;
                let subs: Vec<SubRes> = file.sub_resources.iter().map(SubRes::from).collect();
                export_textured(&graph, &subs, textures)
            });
        assert!(built.is_ok(), "{built:?}");
        built.unwrap_or_else(|_| Gltf {
            json: Value::Null,
            bin: Vec::new(),
            stats: Stats::default(),
        })
    }

    fn near(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6
    }

    #[test]
    fn orm_material_exports_images_uvs_and_normal_map() {
        let g = export_with(TEXTURED, &textures());
        assert_eq!((g.stats.textures, g.stats.textures_skipped), (3, 0));
        let base = "/materials/0/pbrMetallicRoughness";
        assert_eq!(
            g.json.pointer(&format!("{base}/baseColorTexture/index")),
            Some(&json!(0))
        );
        assert_eq!(
            g.json
                .pointer(&format!("{base}/metallicRoughnessTexture/index")),
            Some(&json!(1))
        );
        assert_eq!(
            g.json.pointer("/materials/0/occlusionTexture/index"),
            Some(&json!(1))
        );
        assert_eq!(
            g.json.pointer("/materials/0/normalTexture/index"),
            Some(&json!(2))
        );
        assert_eq!(
            g.json.pointer("/images/2/mimeType"),
            Some(&json!("image/png"))
        );
        assert_eq!(
            g.json.pointer("/textures/1"),
            Some(&json!({"sampler": 0, "source": 1}))
        );
        assert_eq!(g.json.pointer("/samplers/0/wrapS"), Some(&json!(10497)));
        let uv_accessor = g
            .json
            .pointer("/meshes/0/primitives/0/attributes/TEXCOORD_0")
            .and_then(Value::as_u64);
        assert!(uv_accessor.is_some(), "UVs written");
        let count = uv_accessor.and_then(|i| g.json.pointer(&format!("/accessors/{i}/count")));
        assert_eq!(count, Some(&json!(36)), "one UV per vertex, 12 triangles");
        let glb = g.to_glb();
        assert!(
            glb.as_ref().is_ok_and(|b| b.len() % 4 == 0),
            "GLB stays aligned"
        );
    }

    #[test]
    fn triplanar_uvs_follow_godots_dominant_axis_formula() {
        let uv = Uv {
            scale: Vec3::splat(0.5),
            offset: Vec3::ZERO,
        };
        let z_face = [
            Vec3::new(1.0, 0.5, 0.5),
            Vec3::new(-1.0, 0.5, 0.5),
            Vec3::new(-1.0, -0.5, 0.5),
        ];
        let got = triplanar_uvs(&z_face, Vec3::Z, uv);
        assert!(
            near(got[0], [0.5, -0.25]) && near(got[1], [-0.5, -0.25]) && near(got[2], [-0.5, 0.25])
        );
        let unit = Uv {
            scale: Vec3::ONE,
            offset: Vec3::ZERO,
        };
        let p = [Vec3::new(1.0, 0.5, 0.25); 3];
        assert!(
            near(triplanar_uvs(&p, Vec3::X, unit)[0], [-0.25, -0.5]),
            "X face: (-q.z, q.y)"
        );
        assert!(
            near(triplanar_uvs(&p, Vec3::Y, unit)[0], [1.0, 0.25]),
            "Y face: (q.x, q.z)"
        );
        let shifted = Uv {
            scale: Vec3::ONE,
            offset: Vec3::new(0.25, 0.0, 0.0),
        };
        assert!(
            near(triplanar_uvs(&p, Vec3::Z, shifted)[0], [1.25, -0.5]),
            "offset applies before the flip"
        );
    }

    #[test]
    fn textures_without_triplanar_are_counted_not_guessed() {
        let flat = TEXTURED.replace("uv1_triplanar = true\n", "");
        let g = export_with(flat.as_str(), &textures());
        assert_eq!((g.stats.textures, g.stats.textures_skipped), (0, 3));
        assert!(g.json.pointer("/images").is_none() && g.json.pointer("/samplers").is_none());
        assert!(g
            .json
            .pointer("/meshes/0/primitives/0/attributes/TEXCOORD_0")
            .is_none());
    }

    #[test]
    fn missing_or_unknown_image_bytes_are_skipped() {
        let g = export_with(TEXTURED, &Textures::new());
        assert_eq!((g.stats.textures, g.stats.textures_skipped), (0, 3));
        let mut odd = textures();
        odd.insert("1_a".to_string(), b"not an image".to_vec());
        let g = export_with(TEXTURED, &odd);
        assert_eq!(
            (g.stats.textures, g.stats.textures_skipped),
            (2, 1),
            "albedo dropped, ORM and normal kept"
        );
    }

    #[test]
    fn standard_material_with_one_packed_image_maps_like_orm() {
        let text = TEXTURED
            .replace("ORMMaterial3D", "StandardMaterial3D")
            .replace(
                "orm_texture = ExtResource(\"2_o\")",
                "roughness_texture = ExtResource(\"2_o\")\nmetallic_texture = ExtResource(\"2_o\")\nao_texture = ExtResource(\"2_o\")",
            );
        let g = export_with(text.as_str(), &textures());
        assert_eq!((g.stats.textures, g.stats.textures_skipped), (3, 0));
        assert_eq!(
            g.json
                .pointer("/materials/0/pbrMetallicRoughness/metallicRoughnessTexture/index"),
            Some(&json!(1))
        );
        assert_eq!(
            g.json.pointer("/materials/0/occlusionTexture/index"),
            Some(&json!(1))
        );
    }

    #[test]
    fn textured_export_is_deterministic_and_untextured_stays_unchanged() {
        let a = export_with(TEXTURED, &textures());
        let b = export_with(TEXTURED, &textures());
        assert_eq!((a.json.clone(), a.bin.clone()), (b.json, b.bin));
        let plain = exported();
        assert_eq!(plain.stats.textures, 0);
        assert_eq!(
            plain.bin.len(),
            12 * 3 * 2 * 12,
            "no UV bytes without textures"
        );
    }
}

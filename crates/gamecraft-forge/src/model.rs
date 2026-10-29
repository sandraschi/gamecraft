//! Forge builder: command-built scenes plus shared entity makers.
//!
//! Every node is created through `gamecraft-commands` (`AddNodeCommand`,
//! `SetTransformCommand`, `SetPropertyCommand`) on a [`CommandStack`]; each
//! executed command's JSON is logged for `commands.jsonl` provenance.

use gamecraft_commands::{
    AddNodeCommand, Command, CommandError, CommandStack, SetPropertyCommand, SetTransformCommand,
};
use gamecraft_core::{NodeId, PropertyValue, SceneGraph, Transform};
use gamecraft_godot::{encode_color, ConnectionDecl};
use glam::Vec3;
use serde_json::Value;

use crate::render::{Glyph, View};

/// Shared unshaded material decks keep every game light-independent.
pub const DECK_GROUND: &str = "ground";
/// One finished game: scene, scripts, traces, and renders.
pub struct GameParts {
    /// URL slug and folder name.
    pub slug: &'static str,
    /// Display title.
    pub title: &'static str,
    /// Controls line for the README.
    pub controls: &'static str,
    /// Win-condition line for the README.
    pub win_text: &'static str,
    /// Deterministic seed (trace scripts derive from it).
    pub seed: u32,
    /// Exported `.tscn` text.
    pub scene_text: String,
    /// `(filename, content)` scripts for `scripts/`.
    pub scripts: Vec<(&'static str, &'static str)>,
    /// `trace.json` content.
    pub trace_json: String,
    /// `commands.jsonl` provenance log.
    pub commands_json: String,
    /// Static GIF entities.
    pub statics: Vec<Glyph>,
    /// Per-frame dynamic entities (downsampled by the writer).
    pub frames: Vec<Vec<Glyph>>,
    /// World rect for the renderer.
    pub view: View,
    /// Extra README section (design notes, tuning).
    pub readme_body: String,
    /// One-line verification summary for the final report.
    pub summary: String,
}

/// Command-bus scene builder with resource tables and a command log.
pub struct Forge {
    /// Live graph under construction.
    pub graph: SceneGraph,
    /// Undoable bus (builds forward; log proves provenance).
    pub stack: CommandStack,
    /// `serialize_json` per executed command, in order.
    pub log: Vec<Value>,
    /// `(type, path, id)` external resources.
    pub ext: Vec<(String, String, String)>,
    /// `(type, id, properties)` sub-resources.
    pub sub: Vec<gamecraft_godot::SubResourceDecl>,
    /// Connection declarations in file order.
    pub conns: Vec<ConnectionDecl>,
    /// Next sub-resource serial.
    serial: u32,
}

impl Forge {
    /// Empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            graph: SceneGraph::new(),
            stack: CommandStack::new(),
            log: Vec::new(),
            ext: Vec::new(),
            sub: Vec::new(),
            conns: Vec::new(),
            serial: 0,
        }
    }

    /// Record a command's JSON snapshot.
    fn logged(&mut self, value: Value) {
        self.log.push(value);
    }

    /// Add a node; returns its id.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn add_node(
        &mut self,
        name: &str,
        node_type: &str,
        parent: Option<NodeId>,
    ) -> Result<NodeId, CommandError> {
        let mut cmd = AddNodeCommand::new(name.to_string(), node_type.to_string(), parent);
        let id = cmd.execute_mut(&mut self.graph)?;
        self.logged(cmd.serialize_json());
        Ok(id)
    }

    /// Write a property through the bus.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the node is unknown.
    pub fn set_prop(
        &mut self,
        node: NodeId,
        key: &str,
        value: PropertyValue,
    ) -> Result<(), CommandError> {
        let cmd = SetPropertyCommand::new(node, key.to_string(), value);
        cmd.execute(&mut self.graph)?;
        self.logged(cmd.serialize_json());
        Ok(())
    }

    /// Write a local transform through the bus.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the node is unknown.
    pub fn set_transform(&mut self, node: NodeId, next: Transform) -> Result<(), CommandError> {
        let old = self
            .graph
            .get(node)
            .map_or_else(Transform::identity, |current| current.transform);
        let cmd = SetTransformCommand::new(node, old, next);
        cmd.execute(&mut self.graph)?;
        self.logged(cmd.serialize_json());
        Ok(())
    }

    /// Place a node at `position` in one step.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn place(
        &mut self,
        name: &str,
        node_type: &str,
        parent: Option<NodeId>,
        position: [f32; 3],
    ) -> Result<NodeId, CommandError> {
        let id = self.add_node(name, node_type, parent)?;
        self.set_transform(
            id,
            Transform {
                position: Vec3::from_array(position),
                ..Transform::identity()
            },
        )?;
        Ok(id)
    }

    /// Declare an external resource, returning its id.
    pub fn ext_resource(&mut self, resource_type: &str, path: &str, id: &str) -> String {
        self.ext
            .push((resource_type.to_string(), path.to_string(), id.to_string()));
        id.to_string()
    }

    /// Solid-colour background for top-down scenes.
    ///
    /// A procedural sky is wrong when the camera looks straight down: you never
    /// see the horizon, only whatever is behind the floor.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn bg_color(
        &mut self,
        parent: NodeId,
        red: f32,
        green: f32,
        blue: f32,
    ) -> Result<NodeId, CommandError> {
        self.sub.push((
            "Environment".to_string(),
            "environment".to_string(),
            vec![
                ("background_mode".to_string(), "1".to_string()),
                (
                    "background_color".to_string(),
                    encode_color(red, green, blue),
                ),
                ("ambient_light_source".to_string(), "2".to_string()),
                (
                    "ambient_light_color".to_string(),
                    encode_color(red, green, blue),
                ),
                ("ambient_light_energy".to_string(), "1.0".to_string()),
            ],
        ));
        let id = self.add_node("WorldEnvironment", "WorldEnvironment", Some(parent))?;
        self.set_prop(id, "environment", Self::sub_ref("environment"))?;
        Ok(id)
    }

    /// Declare a `CylinderMesh`, returning its sub id.
    pub fn cylinder_mesh(
        &mut self,
        base: &str,
        top_radius: f32,
        bottom_radius: f32,
        height: f32,
    ) -> String {
        let id = format!("{base}_cylinder{}", self.serial);
        self.serial = self.serial.saturating_add(1);
        self.sub.push((
            "CylinderMesh".to_string(),
            id.clone(),
            vec![
                ("top_radius".to_string(), top_radius.to_string()),
                ("bottom_radius".to_string(), bottom_radius.to_string()),
                ("height".to_string(), height.to_string()),
            ],
        ));
        id
    }

    /// Declare a `TorusMesh` (flight gates), returning its sub id.
    pub fn torus_mesh(&mut self, base: &str, inner: f32, outer: f32) -> String {
        let id = format!("{base}_torus{}", self.serial);
        self.serial = self.serial.saturating_add(1);
        self.sub.push((
            "TorusMesh".to_string(),
            id.clone(),
            vec![
                ("inner_radius".to_string(), inner.to_string()),
                ("outer_radius".to_string(), outer.to_string()),
                ("rings".to_string(), "32".to_string()),
                ("ring_segments".to_string(), "12".to_string()),
            ],
        ));
        id
    }

    /// Declare a `SphereMesh`, returning its sub id.
    pub fn sphere_mesh(&mut self, base: &str, radius: f32, height: f32) -> String {
        let id = format!("{base}_sphere{}", self.serial);
        self.serial = self.serial.saturating_add(1);
        self.sub.push((
            "SphereMesh".to_string(),
            id.clone(),
            vec![
                ("radius".to_string(), radius.to_string()),
                ("height".to_string(), height.to_string()),
                ("radial_segments".to_string(), "24".to_string()),
                ("rings".to_string(), "12".to_string()),
            ],
        ));
        id
    }

    /// Declare a `CapsuleMesh`, returning its sub id.
    pub fn capsule_mesh(&mut self, base: &str, radius: f32, height: f32) -> String {
        let id = format!("{base}_capsule{}", self.serial);
        self.serial = self.serial.saturating_add(1);
        self.sub.push((
            "CapsuleMesh".to_string(),
            id.clone(),
            vec![
                ("radius".to_string(), radius.to_string()),
                ("height".to_string(), height.to_string()),
                ("radial_segments".to_string(), "20".to_string()),
                ("rings".to_string(), "6".to_string()),
            ],
        ));
        id
    }

    /// Declare a `SphereShape3D` collider, returning its sub id.
    pub fn sphere_shape(&mut self, base: &str, radius: f32) -> String {
        let id = format!("{base}_sphere_shape{}", self.serial);
        self.serial = self.serial.saturating_add(1);
        self.sub.push((
            "SphereShape3D".to_string(),
            id.clone(),
            vec![("radius".to_string(), radius.to_string())],
        ));
        id
    }

    /// Declare a `CapsuleShape3D` collider, returning its sub id.
    pub fn capsule_shape(&mut self, base: &str, radius: f32, height: f32) -> String {
        let id = format!("{base}_capsule_shape{}", self.serial);
        self.serial = self.serial.saturating_add(1);
        self.sub.push((
            "CapsuleShape3D".to_string(),
            id.clone(),
            vec![
                ("radius".to_string(), radius.to_string()),
                ("height".to_string(), height.to_string()),
            ],
        ));
        id
    }

    /// Declare a `PrismMesh`, returning its sub id.
    pub fn prism_mesh(&mut self, base: &str, size: [f32; 3]) -> String {
        let id = format!("{base}_prism{}", self.serial);
        self.serial = self.serial.saturating_add(1);
        self.sub.push((
            "PrismMesh".to_string(),
            id.clone(),
            vec![("size".to_string(), gamecraft_godot::encode_vector3(size))],
        ));
        id
    }

    /// Sun: a `DirectionalLight3D` aimed from `pitch`/`yaw` (radians).
    ///
    /// Depot materials are unshaded, so this changes nothing in Godot; it exists
    /// so the desktop viewer has real scene lighting to read.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn sun(
        &mut self,
        parent: NodeId,
        pitch: f32,
        yaw: f32,
        energy: f32,
    ) -> Result<NodeId, CommandError> {
        let id = self.add_node("Sun", "DirectionalLight3D", Some(parent))?;
        self.set_transform(
            id,
            Transform::from_position_euler_scale(Vec3::ZERO, Vec3::new(pitch, yaw, 0.0), Vec3::ONE),
        )?;
        self.set_prop(id, "light_energy", PropertyValue::Float(f64::from(energy)))?;
        self.set_prop(id, "shadow_enabled", PropertyValue::Bool(true))?;
        Ok(id)
    }

    /// Procedural sky + environment, so the scene is not on default gray.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn sky(
        &mut self,
        parent: NodeId,
        top: [f32; 3],
        horizon: [f32; 3],
    ) -> Result<NodeId, CommandError> {
        self.sub.push((
            "ProceduralSkyMaterial".to_string(),
            "sky_material".to_string(),
            vec![
                (
                    "sky_top_color".to_string(),
                    encode_color(top[0], top[1], top[2]),
                ),
                (
                    "sky_horizon_color".to_string(),
                    encode_color(horizon[0], horizon[1], horizon[2]),
                ),
                (
                    "ground_bottom_color".to_string(),
                    encode_color(0.55, 0.66, 0.80),
                ),
                (
                    "ground_horizon_color".to_string(),
                    encode_color(horizon[0], horizon[1], horizon[2]),
                ),
            ],
        ));
        self.sub.push((
            "Sky".to_string(),
            "sky".to_string(),
            vec![(
                "sky_material".to_string(),
                "SubResource(\"sky_material\")".to_string(),
            )],
        ));
        self.sub.push((
            "Environment".to_string(),
            "environment".to_string(),
            vec![
                ("background_mode".to_string(), "2".to_string()),
                ("sky".to_string(), "SubResource(\"sky\")".to_string()),
                ("ambient_light_source".to_string(), "3".to_string()),
                ("ambient_light_energy".to_string(), "1.0".to_string()),
            ],
        ));
        let id = self.add_node("WorldEnvironment", "WorldEnvironment", Some(parent))?;
        self.set_prop(id, "environment", Self::sub_ref("environment"))?;
        Ok(id)
    }

    /// Declare an unshaded flat-color material, returning its sub id.
    pub fn material(&mut self, base: &str, red: f32, green: f32, blue: f32) -> String {
        let id = format!("{base}_mat{}", self.serial);
        self.serial = self.serial.saturating_add(1);
        self.sub.push((
            "StandardMaterial3D".to_string(),
            id.clone(),
            vec![
                ("shading_mode".to_string(), "0".to_string()),
                ("albedo_color".to_string(), encode_color(red, green, blue)),
            ],
        ));
        id
    }

    /// Declare a `BoxMesh`, returning its sub id.
    pub fn box_mesh(&mut self, base: &str, size: [f32; 3]) -> String {
        let id = format!("{base}_mesh{}", self.serial);
        self.serial = self.serial.saturating_add(1);
        self.sub.push((
            "BoxMesh".to_string(),
            id.clone(),
            vec![("size".to_string(), gamecraft_godot::encode_vector3(size))],
        ));
        id
    }

    /// Declare a `BoxShape3D`, returning its sub id.
    pub fn box_shape(&mut self, base: &str, size: [f32; 3]) -> String {
        let id = format!("{base}_shape{}", self.serial);
        self.serial = self.serial.saturating_add(1);
        self.sub.push((
            "BoxShape3D".to_string(),
            id.clone(),
            vec![("size".to_string(), gamecraft_godot::encode_vector3(size))],
        ));
        id
    }

    /// Mesh child with a material override.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn mesh_child(
        &mut self,
        name: &str,
        parent: NodeId,
        mesh: &str,
        material: &str,
    ) -> Result<NodeId, CommandError> {
        let id = self.add_node(name, "MeshInstance3D", Some(parent))?;
        self.set_prop(
            id,
            "mesh",
            PropertyValue::Raw(format!("SubResource(\"{mesh}\")")),
        )?;
        self.set_prop(
            id,
            "material_override",
            PropertyValue::Raw(format!("SubResource(\"{material}\")")),
        )?;
        Ok(id)
    }

    /// Collision child with a shape reference.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn shape_child(
        &mut self,
        name: &str,
        parent: NodeId,
        shape: &str,
    ) -> Result<NodeId, CommandError> {
        let id = self.add_node(name, "CollisionShape3D", Some(parent))?;
        self.set_prop(
            id,
            "shape",
            PropertyValue::Raw(format!("SubResource(\"{shape}\")")),
        )?;
        Ok(id)
    }

    /// Full physics/static body: body node plus shape and mesh children.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    #[allow(clippy::too_many_arguments)]
    pub fn solid_box(
        &mut self,
        name: &str,
        body_type: &str,
        parent: Option<NodeId>,
        center: [f32; 3],
        size: [f32; 3],
        red: f32,
        green: f32,
        blue: f32,
    ) -> Result<NodeId, CommandError> {
        let id = self.place(name, body_type, parent, center)?;
        let mesh = self.box_mesh(name, size);
        let material = self.material(name, red, green, blue);
        self.mesh_child("Mesh", id, mesh.as_str(), material.as_str())?;
        let shape = self.box_shape(name, size);
        self.shape_child("Shape", id, shape.as_str())?;
        Ok(id)
    }

    /// Record a signal connection for export.
    pub fn connect(&mut self, signal: &str, from: &str, to: &str, method: &str) {
        self.conns.push(ConnectionDecl {
            signal: signal.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            method: method.to_string(),
        });
    }

    /// Script attachment property value.
    #[must_use]
    pub fn ext_ref(id: &str) -> PropertyValue {
        PropertyValue::Raw(format!("ExtResource(\"{id}\")"))
    }

    /// Sub-resource attachment property value.
    #[must_use]
    pub fn sub_ref(id: &str) -> PropertyValue {
        PropertyValue::Raw(format!("SubResource(\"{id}\")"))
    }

    /// Camera looking toward `-z` with a downward pitch.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn camera_look(
        &mut self,
        name: &str,
        parent: Option<NodeId>,
        pos: [f32; 3],
        pitch: f32,
        fov: f32,
    ) -> Result<NodeId, CommandError> {
        let id = self.add_node(name, "Camera3D", parent)?;
        self.set_transform(
            id,
            Transform::from_position_euler_scale(
                Vec3::from_array(pos),
                Vec3::new(pitch, 0.0, 0.0),
                Vec3::ONE,
            ),
        )?;
        self.set_prop(id, "current", PropertyValue::Bool(true))?;
        self.set_prop(id, "fov", PropertyValue::Float(f64::from(fov)))?;
        Ok(id)
    }

    /// Autoplay in driving mode: pure-pursuit steering toward each checkpoint.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn autoplay_drive(
        &mut self,
        parent: NodeId,
        script_id: &str,
        checkpoint_prefix: &str,
    ) -> Result<NodeId, CommandError> {
        let id = self.add_node("Autoplay", "Node3D", Some(parent))?;
        self.set_prop(id, "script", Self::ext_ref(script_id))?;
        self.set_prop(id, "drive", PropertyValue::Bool(true))?;
        self.set_prop(id, "hold_action", PropertyValue::Str(String::new()))?;
        self.set_prop(
            id,
            "ring_prefix",
            PropertyValue::Str(checkpoint_prefix.to_string()),
        )?;
        Ok(id)
    }

    /// Autoplay in flight mode: tracks each gate's altitude instead of jumping.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn autoplay_flight(
        &mut self,
        parent: NodeId,
        script_id: &str,
    ) -> Result<NodeId, CommandError> {
        let id = self.add_node("Autoplay", "Node3D", Some(parent))?;
        self.set_prop(id, "script", Self::ext_ref(script_id))?;
        self.set_prop(id, "flight", PropertyValue::Bool(true))?;
        self.set_prop(id, "hold_action", PropertyValue::Str(String::new()))?;
        Ok(id)
    }

    /// Autoplay driver: replays the verified trace through synthesized input.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn autoplay(
        &mut self,
        parent: NodeId,
        script_id: &str,
        hold_action: &str,
        jump_frames: &[usize],
    ) -> Result<NodeId, CommandError> {
        let id = self.add_node("Autoplay", "Node3D", Some(parent))?;
        self.set_prop(id, "script", Self::ext_ref(script_id))?;
        self.set_prop(
            id,
            "hold_action",
            PropertyValue::Str(hold_action.to_string()),
        )?;
        let list = jump_frames
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>()
            .join(", ");
        self.set_prop(
            id,
            "jump_frames",
            PropertyValue::Raw(format!("Array[int]([{list}])")),
        )?;
        Ok(id)
    }

    /// Side-scroller camera: follow script, no pitch, narrow vertical FOV.
    ///
    /// The player is centred horizontally at runtime; `z` is the fixed distance
    /// from the action plane.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn follow_camera(
        &mut self,
        name: &str,
        parent: Option<NodeId>,
        start: [f32; 3],
        fov: f32,
        script_id: &str,
    ) -> Result<NodeId, CommandError> {
        let id = self.add_node(name, "Camera3D", parent)?;
        self.set_transform(
            id,
            Transform::from_position_euler_scale(Vec3::from_array(start), Vec3::ZERO, Vec3::ONE),
        )?;
        self.set_prop(id, "current", PropertyValue::Bool(true))?;
        self.set_prop(id, "fov", PropertyValue::Float(f64::from(fov)))?;
        self.set_prop(id, "script", Self::ext_ref(script_id))?;
        Ok(id)
    }

    /// Material that samples a texture instead of a flat colour.
    pub fn textured_material(
        &mut self,
        base: &str,
        texture_id: &str,
        red: f32,
        green: f32,
        blue: f32,
    ) -> String {
        let id = format!("{base}_tex{}", self.serial);
        self.serial = self.serial.saturating_add(1);
        self.sub.push((
            "StandardMaterial3D".to_string(),
            id.clone(),
            vec![
                ("shading_mode".to_string(), "0".to_string()),
                ("albedo_color".to_string(), encode_color(red, green, blue)),
                (
                    "albedo_texture".to_string(),
                    format!("ExtResource(\"{texture_id}\")"),
                ),
                ("uv1_scale".to_string(), "Vector3(2, 2, 1)".to_string()),
            ],
        ));
        id
    }

    /// Swap a solid body's mesh material for a textured one.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the node has no mesh child.
    pub fn apply_texture(
        &mut self,
        node: NodeId,
        texture_id: &str,
        red: f32,
        green: f32,
        blue: f32,
    ) -> Result<(), CommandError> {
        let Some(child) = self
            .graph
            .get(node)
            .and_then(|current| current.children.first().copied())
        else {
            return Err(CommandError::Invalid(format!(
                "node {node} has no mesh child to texture"
            )));
        };
        let material = self.textured_material("wall", texture_id, red, green, blue);
        self.set_prop(child, "material_override", Self::sub_ref(material.as_str()))?;
        Ok(())
    }

    /// Top-down camera for XZ-plane games.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn top_camera(
        &mut self,
        name: &str,
        parent: Option<NodeId>,
        center: [f32; 2],
        height: f32,
    ) -> Result<NodeId, CommandError> {
        // Every top-down game gets a solid backdrop; a sky is invisible from
        // straight above, so without this the floor floats on default gray.
        if let Some(root) = parent {
            self.bg_color(root, 0.09, 0.11, 0.16)?;
        }
        self.camera_look(
            name,
            parent,
            [center[0], height, center[1] + 0.01],
            -std::f32::consts::FRAC_PI_2,
            55.0,
        )
    }

    /// Trigger volume: `Area3D` plus a box shape, no mesh.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    pub fn area_trigger(
        &mut self,
        name: &str,
        parent: Option<NodeId>,
        center: [f32; 3],
        size: [f32; 3],
    ) -> Result<NodeId, CommandError> {
        let id = self.place(name, "Area3D", parent, center)?;
        let shape = self.box_shape(name, size);
        self.shape_child("Shape", id, shape.as_str())?;
        Ok(id)
    }

    /// Decoration mesh with no collision.
    ///
    /// # Errors
    ///
    /// Returns the bus error when the parent is unknown.
    #[allow(clippy::too_many_arguments)]
    pub fn deco_box(
        &mut self,
        name: &str,
        parent: Option<NodeId>,
        center: [f32; 3],
        size: [f32; 3],
        red: f32,
        green: f32,
        blue: f32,
    ) -> Result<NodeId, CommandError> {
        let id = self.place(name, "MeshInstance3D", parent, center)?;
        let mesh = self.box_mesh(name, size);
        let material = self.material(name, red, green, blue);
        self.set_prop(id, "mesh", Self::sub_ref(mesh.as_str()))?;
        self.set_prop(id, "material_override", Self::sub_ref(material.as_str()))?;
        Ok(id)
    }
}

impl Default for Forge {
    fn default() -> Self {
        Self::new()
    }
}

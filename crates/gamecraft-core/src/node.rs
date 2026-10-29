//! Scene node arena: ids, property values, parent/child graph.

use crate::Transform;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stable node handle inside a [`SceneGraph`].
pub type NodeId = u32;

/// Untyped Godot-style property value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum PropertyValue {
    /// Boolean flag.
    Bool(bool),
    /// Integer (`i64` covers Godot int + enum ordinals).
    Int(i64),
    /// Floating point.
    Float(f64),
    /// Quoted string / path / `NodePath`.
    Str(String),
    /// `Vector3(x, y, z)`.
    Vec3([f32; 3]),
    /// Raw source text for values the parser did not decode further.
    Raw(String),
}

impl PropertyValue {
    /// Convenience: string slice when the value is textual.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) | Self::Raw(s) => Some(s),
            Self::Bool(_) | Self::Int(_) | Self::Float(_) | Self::Vec3(_) => None,
        }
    }
}

/// One node in the scene tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneNode {
    /// Arena id.
    pub id: NodeId,
    /// Godot node name (unique among siblings).
    pub name: String,
    /// Godot type (`Node3D`, `MeshInstance3D`, `StaticBody3D`, ...).
    pub node_type: String,
    /// Parent arena id, or `None` for the scene root.
    pub parent: Option<NodeId>,
    /// Children arena ids in scene order.
    pub children: Vec<NodeId>,
    /// Property dictionary (`position`, `mesh`, `collision_layer`, ...).
    pub properties: HashMap<String, PropertyValue>,
    /// Local transform.
    pub transform: Transform,
    /// Whether the name is marked unique (`%Name`).
    pub unique_name: bool,
}

impl SceneNode {
    /// Fresh node with identity transform and no properties.
    #[must_use]
    pub fn new(id: NodeId, name: String, node_type: String) -> Self {
        Self {
            id,
            name,
            node_type,
            parent: None,
            children: Vec::new(),
            properties: HashMap::new(),
            transform: Transform::identity(),
            unique_name: false,
        }
    }

    /// True for physics bodies that require a `CollisionShape3D` child.
    #[must_use]
    pub fn is_physics_body(&self) -> bool {
        matches!(
            self.node_type.as_str(),
            "PhysicsBody3D"
                | "StaticBody3D"
                | "RigidBody3D"
                | "CharacterBody3D"
                | "AnimatableBody3D"
                | "VehicleBody3D"
        )
    }
}

/// Arena holding every node plus a scene-path index.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneGraph {
    /// All nodes by id.
    pub nodes: HashMap<NodeId, SceneNode>,
    /// Scene root id, if any.
    pub root: Option<NodeId>,
    /// Next id to hand out.
    next_id: NodeId,
}

impl SceneGraph {
    /// Empty graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a node, wiring it under `parent` when given.
    ///
    /// # Errors
    ///
    /// Returns the node name when `parent` does not exist.
    pub fn add_node(
        &mut self,
        name: String,
        node_type: String,
        parent: Option<NodeId>,
    ) -> Result<NodeId, String> {
        if let Some(p) = parent {
            if !self.nodes.contains_key(&p) {
                return Err(format!("unknown parent {p} for node {name}"));
            }
        }
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let mut node = SceneNode::new(id, name, node_type);
        node.parent = parent;
        if let Some(p) = parent {
            if let Some(parent_node) = self.nodes.get_mut(&p) {
                parent_node.children.push(id);
            }
        } else if self.root.is_none() {
            self.root = Some(id);
        }
        self.nodes.insert(id, node);
        Ok(id)
    }

    /// Fetch a node by id.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&SceneNode> {
        self.nodes.get(&id)
    }

    /// Fetch a node by id (mutable).
    #[must_use]
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut SceneNode> {
        self.nodes.get_mut(&id)
    }

    /// Resolve a Godot scene path (`"."` = `from`, `"%Name"` = unique search).
    #[must_use]
    pub fn resolve_path(&self, from: NodeId, path: &str) -> Option<NodeId> {
        if path == "." {
            return Some(from);
        }
        if let Some(unique) = path.strip_prefix('%') {
            return self
                .nodes
                .values()
                .find(|n| n.unique_name && n.name == unique)
                .map(|n| n.id);
        }
        let mut current = if path.starts_with('/') {
            self.root?
        } else {
            from
        };
        for part in path.split('/').filter(|p| !p.is_empty() && *p != ".") {
            if part == ".." {
                current = self.nodes.get(&current)?.parent?;
                continue;
            }
            let next = self
                .nodes
                .get(&current)?
                .children
                .iter()
                .filter_map(|c| self.nodes.get(c))
                .find(|n| n.name == part)
                .map(|n| n.id)?;
            current = next;
        }
        Some(current)
    }

    /// Scene path of a node (`Root/Child/Grandchild`), root-relative.
    #[must_use]
    pub fn node_path(&self, id: NodeId) -> Option<String> {
        let mut parts: Vec<&str> = Vec::new();
        let mut current = self.nodes.get(&id)?;
        loop {
            parts.push(current.name.as_str());
            match current.parent {
                Some(p) => {
                    current = self.nodes.get(&p)?;
                }
                None => break,
            }
        }
        parts.reverse();
        Some(parts.join("/"))
    }

    /// Direct children of a node.
    #[must_use]
    pub fn children_of(&self, id: NodeId) -> Vec<NodeId> {
        self.nodes.get(&id).map_or_else(Vec::new, |n| {
            n.children
                .iter()
                .filter(|c| self.nodes.contains_key(c))
                .copied()
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> SceneGraph {
        let mut g = SceneGraph::new();
        let mut root: NodeId = 0;
        if let Ok(id) = g.add_node("Root".to_string(), "Node3D".to_string(), None) {
            root = id;
        }
        if let Ok(id) = g.add_node("Body".to_string(), "StaticBody3D".to_string(), Some(root)) {
            let _ = g.add_node(
                "Shape".to_string(),
                "CollisionShape3D".to_string(),
                Some(id),
            );
        }
        g
    }

    #[test]
    fn child_paths_resolve() {
        let g = sample_graph();
        let root = g.root.unwrap_or(NodeId::MAX);
        assert!(root != NodeId::MAX);
        assert!(g.resolve_path(root, "Body/Shape").is_some());
        assert!(g.resolve_path(root, "Body/Missing").is_none());
    }

    #[test]
    fn unknown_parent_is_rejected() {
        let mut g = SceneGraph::new();
        let res = g.add_node("Orphan".to_string(), "Node3D".to_string(), Some(999));
        assert!(res.is_err());
    }
}

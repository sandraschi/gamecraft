//! Undoable command bus over [`SceneGraph`](gamecraft_core::SceneGraph).
//!
//! Every mutation implements [`Command`] (`execute` / `undo` /
//! `serialize_json`); [`CommandStack`] adds undo/redo on top.

use gamecraft_core::{NodeId, PropertyValue, SceneGraph, Transform};
use serde_json::Value;
use std::fmt::Debug;
use thiserror::Error;

/// Failures a command can report.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CommandError {
    /// Referenced node id does not exist.
    #[error("unknown node {0}")]
    UnknownNode(NodeId),
    /// Operation is not valid in the current graph state.
    #[error("invalid operation: {0}")]
    Invalid(String),
}

/// Undoable scene mutation.
pub trait Command: Debug {
    /// Short verb for logs and the MCP surface.
    fn label(&self) -> &'static str;
    /// Apply the mutation.
    ///
    /// # Errors
    ///
    /// Returns [`CommandError`] when the graph lacks a referenced node or the
    /// operation is invalid in the current state.
    fn execute(&self, graph: &mut SceneGraph) -> Result<(), CommandError>;
    /// Revert a previous [`execute`](Command::execute).
    ///
    /// # Errors
    ///
    /// Returns [`CommandError`] when the graph changed underneath the command
    /// (for example the node was deleted after execution).
    fn undo(&self, graph: &mut SceneGraph) -> Result<(), CommandError>;
    /// Deterministic JSON snapshot (round-trips through the MCP surface).
    fn serialize_json(&self) -> Value;
}

/// Adds a node under `parent` (`None` = scene root).
#[derive(Debug, Clone)]
pub struct AddNodeCommand {
    /// Node name.
    pub name: String,
    /// Godot type.
    pub node_type: String,
    /// Parent id, or `None` for the scene root.
    pub parent: Option<NodeId>,
    /// Id assigned at execute time (reused on redo).
    pub assigned: Option<NodeId>,
}

impl AddNodeCommand {
    /// Stage an add (nothing is applied until `execute` runs).
    #[must_use]
    pub fn new(name: String, node_type: String, parent: Option<NodeId>) -> Self {
        Self {
            name,
            node_type,
            parent,
            assigned: None,
        }
    }

    /// Execute and record the assigned id (forge-style builds).
    ///
    /// # Errors
    ///
    /// Returns [`CommandError`] when the parent is unknown.
    pub fn execute_mut(&mut self, graph: &mut SceneGraph) -> Result<NodeId, CommandError> {
        let id = graph
            .add_node(self.name.clone(), self.node_type.clone(), self.parent)
            .map_err(CommandError::Invalid)?;
        self.assigned = Some(id);
        Ok(id)
    }
}

impl Command for AddNodeCommand {
    fn label(&self) -> &'static str {
        "add_node"
    }

    fn execute(&self, graph: &mut SceneGraph) -> Result<(), CommandError> {
        graph
            .add_node(self.name.clone(), self.node_type.clone(), self.parent)
            .map(|_| ())
            .map_err(CommandError::Invalid)
    }

    fn undo(&self, graph: &mut SceneGraph) -> Result<(), CommandError> {
        let target = self.assigned.map_or_else(
            || {
                graph
                    .nodes
                    .values()
                    .find(|n| {
                        n.name == self.name
                            && n.node_type == self.node_type
                            && n.parent == self.parent
                    })
                    .map(|n| n.id)
            },
            Some,
        );
        if let Some(id) = target {
            remove_subtree(graph, id);
            Ok(())
        } else {
            Err(CommandError::Invalid(format!(
                "nothing to undo for {}",
                self.name
            )))
        }
    }

    fn serialize_json(&self) -> Value {
        serde_json::json!({
            "op": self.label(),
            "name": self.name,
            "type": self.node_type,
            "parent": self.parent,
            "assigned": self.assigned,
        })
    }
}

/// Moves a node (plus its subtree) to a new parent.
#[derive(Debug, Clone)]
pub struct ReparentNodeCommand {
    /// Node to move.
    pub node: NodeId,
    /// Parent recorded before `execute` (filled in on first run).
    pub old_parent: Option<Option<NodeId>>,
    /// Destination parent.
    pub new_parent: Option<NodeId>,
}

impl ReparentNodeCommand {
    /// Stage a reparent.
    #[must_use]
    pub fn new(node: NodeId, new_parent: Option<NodeId>) -> Self {
        Self {
            node,
            old_parent: None,
            new_parent,
        }
    }
}

impl Command for ReparentNodeCommand {
    fn label(&self) -> &'static str {
        "reparent_node"
    }

    fn execute(&self, graph: &mut SceneGraph) -> Result<(), CommandError> {
        if let Some(p) = self.new_parent {
            if graph.get(p).is_none() {
                return Err(CommandError::UnknownNode(p));
            }
        }
        let node = graph
            .get(self.node)
            .ok_or(CommandError::UnknownNode(self.node))?;
        if node.id == self.new_parent.unwrap_or(NodeId::MAX) {
            return Err(CommandError::Invalid(
                "cannot parent a node to itself".to_string(),
            ));
        }
        if is_descendant(graph, self.node, self.new_parent) {
            return Err(CommandError::Invalid(
                "cannot parent a node under its own descendant".to_string(),
            ));
        }
        detach(graph, self.node);
        attach(graph, self.node, self.new_parent);
        Ok(())
    }

    fn undo(&self, graph: &mut SceneGraph) -> Result<(), CommandError> {
        if let Some(old) = self.old_parent {
            detach(graph, self.node);
            attach(graph, self.node, old);
            Ok(())
        } else {
            Err(CommandError::Invalid(
                "reparent was never executed".to_string(),
            ))
        }
    }

    fn serialize_json(&self) -> Value {
        serde_json::json!({
            "op": self.label(),
            "node": self.node,
            "new_parent": self.new_parent,
        })
    }
}

/// Sets one property, remembering the previous value for undo.
#[derive(Debug, Clone)]
pub struct SetPropertyCommand {
    /// Target node.
    pub node: NodeId,
    /// Property key.
    pub key: String,
    /// Value to set.
    pub value: PropertyValue,
    /// Value present before `execute` (`None` = key absent).
    pub old_value: Option<Option<PropertyValue>>,
}

impl SetPropertyCommand {
    /// Stage a property write.
    #[must_use]
    pub fn new(node: NodeId, key: String, value: PropertyValue) -> Self {
        Self {
            node,
            key,
            value,
            old_value: None,
        }
    }
}

impl Command for SetPropertyCommand {
    fn label(&self) -> &'static str {
        "set_property"
    }

    fn execute(&self, graph: &mut SceneGraph) -> Result<(), CommandError> {
        let node = graph
            .get_mut(self.node)
            .ok_or(CommandError::UnknownNode(self.node))?;
        node.properties.insert(self.key.clone(), self.value.clone());
        Ok(())
    }

    fn undo(&self, graph: &mut SceneGraph) -> Result<(), CommandError> {
        let node = graph
            .get_mut(self.node)
            .ok_or(CommandError::UnknownNode(self.node))?;
        match &self.old_value {
            Some(Some(old)) => {
                node.properties.insert(self.key.clone(), old.clone());
            }
            _ => {
                node.properties.remove(self.key.as_str());
            }
        }
        Ok(())
    }

    fn serialize_json(&self) -> Value {
        serde_json::json!({
            "op": self.label(),
            "node": self.node,
            "key": self.key,
        })
    }
}

/// Remove a node and its whole subtree.
fn remove_subtree(graph: &mut SceneGraph, id: NodeId) {
    let children: Vec<NodeId> = graph.children_of(id);
    for child in children {
        remove_subtree(graph, child);
    }
    detach(graph, id);
    graph.nodes.remove(&id);
}

/// Unlink `id` from its parent (and fix root tracking).
fn detach(graph: &mut SceneGraph, id: NodeId) {
    let parent = graph.get(id).and_then(|n| n.parent);
    if let Some(p) = parent {
        if let Some(parent_node) = graph.get_mut(p) {
            parent_node.children.retain(|c| *c != id);
        }
    }
    if let Some(node) = graph.get_mut(id) {
        node.parent = None;
    }
    if graph.root == Some(id) {
        graph.root = None;
    }
}

/// Link `id` under `parent` (`None` = adopt as root when empty).
fn attach(graph: &mut SceneGraph, id: NodeId, parent: Option<NodeId>) {
    if let Some(p) = parent {
        if let Some(parent_node) = graph.get_mut(p) {
            if !parent_node.children.contains(&id) {
                parent_node.children.push(id);
            }
        }
    } else if graph.root.is_none() {
        graph.root = Some(id);
    }
    if graph.get(id).is_some() {
        if let Some(node) = graph.get_mut(id) {
            node.parent = parent;
        }
    }
}

/// True when `maybe_parent` is `ancestor` itself or lies under it.
fn is_descendant(graph: &SceneGraph, ancestor: NodeId, maybe_parent: Option<NodeId>) -> bool {
    let mut cursor = maybe_parent;
    while let Some(c) = cursor {
        if c == ancestor {
            return true;
        }
        cursor = graph.get(c).and_then(|n| n.parent);
    }
    false
}

/// Replaces a node's local transform, remembering the previous one for undo.
#[derive(Debug, Clone)]
pub struct SetTransformCommand {
    /// Target node.
    pub node: NodeId,
    /// Transform present before [`execute`](Command::execute).
    pub old: Transform,
    /// Transform to install.
    pub new: Transform,
}

impl SetTransformCommand {
    /// Stage a transform write.
    #[must_use]
    pub fn new(node: NodeId, old: Transform, new: Transform) -> Self {
        Self { node, old, new }
    }
}

impl Command for SetTransformCommand {
    fn label(&self) -> &'static str {
        "set_transform"
    }

    fn execute(&self, graph: &mut SceneGraph) -> Result<(), CommandError> {
        let node = graph
            .get_mut(self.node)
            .ok_or(CommandError::UnknownNode(self.node))?;
        node.transform = self.new;
        Ok(())
    }

    fn undo(&self, graph: &mut SceneGraph) -> Result<(), CommandError> {
        let node = graph
            .get_mut(self.node)
            .ok_or(CommandError::UnknownNode(self.node))?;
        node.transform = self.old;
        Ok(())
    }

    fn serialize_json(&self) -> Value {
        serde_json::json!({
            "op": self.label(),
            "node": self.node,
            "position": self.new.position.to_array(),
            "scale": self.new.scale.to_array(),
        })
    }
}

/// Undo/redo stack over boxed [`Command`]s.
#[derive(Debug, Default)]
pub struct CommandStack {
    /// Commands available for redo (cleared on every new execute).
    redo: Vec<Box<dyn Command>>,
    /// Commands available for undo.
    undo: Vec<Box<dyn Command>>,
}

impl CommandStack {
    /// Empty stack.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Execute and push; clears the redo lane.
    ///
    /// # Errors
    ///
    /// Propagates the command's [`execute`](Command::execute) failure without
    /// pushing it onto the undo lane.
    pub fn execute(
        &mut self,
        command: Box<dyn Command>,
        graph: &mut SceneGraph,
    ) -> Result<(), CommandError> {
        command.execute(graph)?;
        self.redo.clear();
        self.undo.push(command);
        Ok(())
    }

    /// Undo the newest command.
    ///
    /// # Errors
    ///
    /// Returns [`CommandError::Invalid`] when the stack is empty, otherwise
    /// propagates the command's [`undo`](Command::undo) failure.
    pub fn undo(&mut self, graph: &mut SceneGraph) -> Result<(), CommandError> {
        if let Some(command) = self.undo.pop() {
            command.undo(graph)?;
            self.redo.push(command);
            Ok(())
        } else {
            Err(CommandError::Invalid("undo stack is empty".to_string()))
        }
    }

    /// Redo the newest undone command.
    ///
    /// # Errors
    ///
    /// Returns [`CommandError::Invalid`] when the stack is empty, otherwise
    /// propagates the command's [`execute`](Command::execute) failure.
    pub fn redo(&mut self, graph: &mut SceneGraph) -> Result<(), CommandError> {
        if let Some(command) = self.redo.pop() {
            command.execute(graph)?;
            self.undo.push(command);
            Ok(())
        } else {
            Err(CommandError::Invalid("redo stack is empty".to_string()))
        }
    }

    /// Depth of the undo lane.
    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// Depth of the redo lane.
    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rooted() -> (SceneGraph, NodeId) {
        let mut g = SceneGraph::new();
        let mut root: NodeId = 0;
        if let Ok(id) = g.add_node("Root".to_string(), "Node3D".to_string(), None) {
            root = id;
        }
        (g, root)
    }

    #[test]
    fn add_and_undo_round_trip() {
        let (mut g, root) = rooted();
        let mut stack = CommandStack::new();
        let add = Box::new(AddNodeCommand::new(
            "Body".to_string(),
            "StaticBody3D".to_string(),
            Some(root),
        ));
        assert!(stack.execute(add, &mut g).is_ok());
        assert_eq!(g.children_of(root).len(), 1);
        assert!(stack.undo(&mut g).is_ok());
        assert_eq!(g.children_of(root), Vec::<NodeId>::new());
        assert!(stack.redo(&mut g).is_ok());
        assert_eq!(g.children_of(root).len(), 1);
    }

    #[test]
    fn set_property_undo_restores_old() {
        let (mut g, root) = rooted();
        let mut cmd =
            SetPropertyCommand::new(root, "visible".to_string(), PropertyValue::Bool(false));
        assert!(cmd.execute(&mut g).is_ok());
        cmd.old_value = Some(Some(PropertyValue::Bool(true)));
        if let Some(n) = g.get(root) {
            assert_eq!(
                n.properties.get("visible"),
                Some(&PropertyValue::Bool(false))
            );
        }
        assert!(cmd.undo(&mut g).is_ok());
        if let Some(n) = g.get(root) {
            assert_eq!(
                n.properties.get("visible"),
                Some(&PropertyValue::Bool(true))
            );
        }
    }

    #[test]
    fn reparent_cycle_is_rejected() {
        let (mut g, root) = rooted();
        let mut child: NodeId = 0;
        if let Ok(id) = g.add_node("Child".to_string(), "Node3D".to_string(), Some(root)) {
            child = id;
        }
        let mut grand: NodeId = 0;
        if let Ok(id) = g.add_node("Grand".to_string(), "Node3D".to_string(), Some(child)) {
            grand = id;
        }
        let cmd = ReparentNodeCommand::new(root, Some(grand));
        assert!(cmd.execute(&mut g).is_err());
    }

    #[test]
    fn execute_mut_records_id() {
        let (mut g, root) = rooted();
        let mut cmd =
            AddNodeCommand::new("Body".to_string(), "StaticBody3D".to_string(), Some(root));
        let id = cmd.execute_mut(&mut g);
        assert!(id.is_ok());
        if let Ok(id) = id {
            assert_eq!(cmd.assigned, Some(id));
            assert!(g.get(id).is_some());
        }
    }

    #[test]
    fn set_transform_undo_restores() {
        use gamecraft_core::Transform;
        use glam::Vec3;
        let (mut g, root) = rooted();
        let before = if let Some(n) = g.get(root) {
            n.transform
        } else {
            Transform::identity()
        };
        let mut moved = before;
        moved.position = Vec3::new(1.0, 2.0, 3.0);
        let cmd = SetTransformCommand::new(root, before, moved);
        assert!(cmd.execute(&mut g).is_ok());
        if let Some(n) = g.get(root) {
            assert_eq!(n.transform.position, Vec3::new(1.0, 2.0, 3.0));
        }
        assert!(cmd.undo(&mut g).is_ok());
        if let Some(n) = g.get(root) {
            assert_eq!(n.transform.position, before.position);
        }
    }

    #[test]
    fn serialize_json_carries_op() {
        let cmd = AddNodeCommand::new("X".to_string(), "Node3D".to_string(), None);
        let v = cmd.serialize_json();
        assert_eq!(v.get("op").and_then(|o| o.as_str()), Some("add_node"));
    }
}

//! Signal connection registry with target/method resolution.

use crate::{NodeId, SceneGraph};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One Godot `[connection signal=... from=... to=... method=...]` edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalConnection {
    /// Signal name on the source node.
    pub signal: String,
    /// Scene path (or unique name) of the emitting node.
    pub from: String,
    /// Scene path (or unique name) of the receiving node.
    pub to: String,
    /// Method invoked on the receiver.
    pub method: String,
}

impl SignalConnection {
    /// New connection edge.
    #[must_use]
    pub fn new(signal: String, from: String, to: String, method: String) -> Self {
        Self {
            signal,
            from,
            to,
            method,
        }
    }
}

/// Registry of connections plus the set of methods each node exposes.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalRegistry {
    /// All declared connections.
    pub connections: Vec<SignalConnection>,
    /// Node id -> method names the node exposes (script surface).
    pub methods: HashMap<NodeId, Vec<String>>,
}

impl SignalRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a connection edge.
    pub fn add_connection(&mut self, connection: SignalConnection) {
        self.connections.push(connection);
    }

    /// Declare that `node` exposes `method`.
    pub fn add_method(&mut self, node: NodeId, method: String) {
        self.methods.entry(node).or_default().push(method);
    }

    /// Resolve every connection against `graph` rooted at `root`.
    ///
    /// Returns `(resolved, unresolved)` where unresolved entries carry the
    /// connection index plus a human reason.
    #[must_use]
    pub fn resolve(&self, graph: &SceneGraph, root: NodeId) -> (Vec<usize>, Vec<(usize, String)>) {
        let mut ok: Vec<usize> = Vec::new();
        let mut bad: Vec<(usize, String)> = Vec::new();
        for (index, conn) in self.connections.iter().enumerate() {
            let from = graph.resolve_path(root, conn.from.as_str());
            let to = graph.resolve_path(root, conn.to.as_str());
            match (from, to) {
                (Some(_), Some(target)) => {
                    let known = self
                        .methods
                        .get(&target)
                        .is_some_and(|m| m.iter().any(|x| x == &conn.method));
                    if known {
                        ok.push(index);
                    } else {
                        bad.push((
                            index,
                            format!("method '{}' not exposed on '{}'", conn.method, conn.to),
                        ));
                    }
                }
                (None, _) => {
                    bad.push((index, format!("signal source '{}' unresolvable", conn.from)));
                }
                (_, None) => {
                    bad.push((index, format!("signal target '{}' unresolvable", conn.to)));
                }
            }
        }
        (ok, bad)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_with_methods() -> (SceneGraph, NodeId, SignalRegistry) {
        let mut g = SceneGraph::new();
        let mut root: NodeId = 0;
        if let Ok(id) = g.add_node("Root".to_string(), "Node".to_string(), None) {
            root = id;
        }
        let mut button: NodeId = 0;
        if let Ok(id) = g.add_node("Button".to_string(), "Button".to_string(), Some(root)) {
            button = id;
        }
        let mut reg = SignalRegistry::new();
        reg.add_method(button, "pressed".to_string());
        reg.add_method(root, "_on_pressed".to_string());
        (g, root, reg)
    }

    #[test]
    fn known_method_resolves() {
        let (g, root, mut reg) = graph_with_methods();
        reg.add_connection(SignalConnection::new(
            "pressed".to_string(),
            "Button".to_string(),
            ".".to_string(),
            "_on_pressed".to_string(),
        ));
        let (ok, bad) = reg.resolve(&g, root);
        assert_eq!(ok.len(), 1);
        assert_eq!(bad, Vec::new());
    }

    #[test]
    fn unknown_target_is_unresolved() {
        let (g, root, mut reg) = graph_with_methods();
        reg.add_connection(SignalConnection::new(
            "pressed".to_string(),
            "Button".to_string(),
            "Ghost".to_string(),
            "_on_pressed".to_string(),
        ));
        let (ok, bad) = reg.resolve(&g, root);
        assert_eq!(ok, Vec::<usize>::new());
        assert_eq!(bad.len(), 1);
    }
}

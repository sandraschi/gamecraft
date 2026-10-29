//! World-space aggregation: matrices and bounding boxes over the tree.

use crate::{Aabb, NodeId, SceneGraph, SceneNode};
use glam::{Mat4, Vec3};

/// World matrix per node id.
#[must_use]
pub fn world_matrices(graph: &SceneGraph) -> Vec<(NodeId, Mat4)> {
    let mut out: Vec<(NodeId, Mat4)> = Vec::new();
    if let Some(root) = graph.root {
        walk(graph, root, Mat4::IDENTITY, &mut out);
    }
    out
}

fn walk(graph: &SceneGraph, id: NodeId, parent_world: Mat4, out: &mut Vec<(NodeId, Mat4)>) {
    if let Some(node) = graph.get(id) {
        let world = parent_world * node.transform.matrix();
        out.push((id, world));
        for child in node.children.iter().copied() {
            walk(graph, child, world, out);
        }
    }
}

/// Local AABB guess for node types that carry implicit geometry.
///
/// `MeshInstance3D` without an explicit `aabb` property falls back to a unit
/// box; `CollisionShape3D` falls back to a half-extent box; everything else
/// contributes nothing on its own (children still aggregate).
#[must_use]
pub fn local_aabb_guess(node: &SceneNode) -> Aabb {
    if let Some(crate::PropertyValue::Vec3(e)) = node.properties.get("aabb_extents") {
        return Aabb::from_center_extents(Vec3::ZERO, Vec3::from_array(*e));
    }
    match node.node_type.as_str() {
        "MeshInstance3D" => Aabb::from_center_extents(Vec3::ZERO, Vec3::ONE),
        "CollisionShape3D" => Aabb::from_center_extents(Vec3::ZERO, Vec3::splat(0.5)),
        _ => Aabb::empty(),
    }
}

/// World-space AABB of the subtree rooted at `id` (self + descendants).
#[must_use]
pub fn subtree_aabb(graph: &SceneGraph, id: NodeId) -> Aabb {
    let matrices = world_matrices(graph);
    let world_of = |n: NodeId| {
        matrices
            .iter()
            .find(|(m, _)| *m == n)
            .map_or(Mat4::IDENTITY, |(_, w)| *w)
    };
    subtree_aabb_inner(graph, id, &world_of)
}

fn subtree_aabb_inner(graph: &SceneGraph, id: NodeId, world_of: &dyn Fn(NodeId) -> Mat4) -> Aabb {
    let Some(node) = graph.get(id) else {
        return Aabb::empty();
    };
    let mut acc = local_aabb_guess(node).transformed(world_of(id));
    for child in node.children.iter().copied() {
        acc = acc.union(subtree_aabb_inner(graph, child, world_of));
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SceneGraph, Transform};

    #[test]
    fn subtree_box_covers_translated_child() {
        let mut g = SceneGraph::new();
        let mut root: NodeId = 0;
        if let Ok(id) = g.add_node("Root".to_string(), "Node3D".to_string(), None) {
            root = id;
        }
        if let Ok(id) = g.add_node("Mesh".to_string(), "MeshInstance3D".to_string(), Some(root)) {
            if let Some(n) = g.get_mut(id) {
                n.transform = Transform {
                    position: Vec3::new(10.0, 0.0, 0.0),
                    ..Transform::identity()
                };
            }
        }
        let b = subtree_aabb(&g, root);
        assert!(!b.is_empty());
        assert!(b.contains(Vec3::new(10.0, 0.0, 0.0)));
    }
}

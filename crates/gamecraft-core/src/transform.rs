//! Local position / rotation / scale with `glam` matrix composition.

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Local transform of one scene node.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    /// Local translation in parent space.
    pub position: Vec3,
    /// Local rotation in parent space.
    pub rotation: Quat,
    /// Local scale in parent space.
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform {
    /// Identity transform (no translation, rotation, or scale).
    #[must_use]
    pub fn identity() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    /// Build from Godot-style `position` / Euler `rotation` (radians) / `scale`.
    #[must_use]
    pub fn from_position_euler_scale(position: Vec3, euler: Vec3, scale: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::from_euler(glam::EulerRot::YXZ, euler.y, euler.x, euler.z),
            scale,
        }
    }

    /// Local matrix: `T * R * S`.
    #[must_use]
    pub fn matrix(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    /// Compose a parent world matrix with this local transform.
    #[must_use]
    pub fn compose_world(parent_world: Mat4, local: Self) -> Mat4 {
        parent_world * local.matrix()
    }

    /// World-space translation of a world matrix.
    #[must_use]
    pub fn world_position(world: Mat4) -> Vec3 {
        world.w_axis.truncate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_matrix_is_identity() {
        let m: [[f32; 4]; 4] = Transform::identity().matrix().to_cols_array_2d();
        assert_eq!(m, Mat4::IDENTITY.to_cols_array_2d());
    }

    #[test]
    fn child_world_inherits_parent_translation() {
        let parent = Transform {
            position: Vec3::new(1.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        let child = Transform {
            position: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        let world = Transform::compose_world(parent.matrix(), child);
        let p = Transform::world_position(world);
        assert!((p.x - 1.0).abs() < f32::EPSILON);
        assert!((p.y - 2.0).abs() < f32::EPSILON);
    }
}

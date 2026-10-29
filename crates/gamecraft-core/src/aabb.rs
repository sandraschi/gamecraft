//! Axis-aligned bounding boxes and their aggregation.

use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

/// Axis-aligned bounding box in world space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    /// Minimum corner.
    pub min: Vec3,
    /// Maximum corner.
    pub max: Vec3,
}

impl Aabb {
    /// Empty box: `min` at `+inf`, `max` at `-inf`, so any `union` adopts the other side.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    /// Box from center + full extents.
    #[must_use]
    pub fn from_center_extents(center: Vec3, extents: Vec3) -> Self {
        let half = extents * 0.5;
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// True when no volume has been unioned in yet.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }

    /// Smallest box containing both inputs.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// True when `point` lies inside (inclusive).
    #[must_use]
    pub fn contains(self, point: Vec3) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    /// Transform by a world matrix (transforms all 8 corners, re-bounds).
    #[must_use]
    pub fn transformed(self, world: Mat4) -> Self {
        let mut acc = Self::empty();
        for dx in [self.min.x, self.max.x] {
            for dy in [self.min.y, self.max.y] {
                for dz in [self.min.z, self.max.z] {
                    let p = world.transform_point3(Vec3::new(dx, dy, dz));
                    acc = acc.union(Self { min: p, max: p });
                }
            }
        }
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_union_adopts_other() {
        let full = Aabb::from_center_extents(Vec3::ZERO, Vec3::ONE);
        assert_eq!(Aabb::empty().union(full), full);
        assert_eq!(full.union(Aabb::empty()), full);
    }

    #[test]
    fn union_spans_both() {
        let a = Aabb::from_center_extents(Vec3::new(-1.0, 0.0, 0.0), Vec3::ONE);
        let b = Aabb::from_center_extents(Vec3::new(1.0, 0.0, 0.0), Vec3::ONE);
        let u = a.union(b);
        assert!(u.contains(Vec3::new(-1.5, 0.0, 0.0)));
        assert!(u.contains(Vec3::new(1.5, 0.0, 0.0)));
    }
}

//! 32-layer Godot collision matrix.

use serde::{Deserialize, Serialize};

/// Number of physics layers Godot supports.
pub const LAYER_COUNT: usize = 32;

/// Symmetric scan/collide matrix over the 32 physics layers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollisionMatrix {
    /// `cells[a][b]` is true when layer `a` scans layer `b`.
    cells: [[bool; LAYER_COUNT]; LAYER_COUNT],
}

impl Default for CollisionMatrix {
    fn default() -> Self {
        Self::identity()
    }
}

impl CollisionMatrix {
    /// Default Godot matrix: every layer only scans layer 1.
    #[must_use]
    pub fn identity() -> Self {
        let mut cells = [[false; LAYER_COUNT]; LAYER_COUNT];
        for row in &mut cells {
            row[0] = true;
        }
        Self { cells }
    }

    /// Set the scan bit for `layer` -> `mask_bit`.
    pub fn set(&mut self, layer: usize, mask_bit: usize, value: bool) {
        if layer < LAYER_COUNT && mask_bit < LAYER_COUNT {
            self.cells[layer][mask_bit] = value;
        }
    }

    /// Read one scan bit; out-of-range layers read as `false`.
    #[must_use]
    pub fn get(&self, layer: usize, mask_bit: usize) -> bool {
        if layer < LAYER_COUNT && mask_bit < LAYER_COUNT {
            self.cells[layer][mask_bit]
        } else {
            false
        }
    }

    /// True when bodies on `layer_a` and `layer_b` can collide with each other.
    #[must_use]
    pub fn can_collide(&self, layer_a: usize, layer_b: usize) -> bool {
        self.get(layer_a, layer_b) || self.get(layer_b, layer_a)
    }

    /// Decode a Godot `collision_layer` / `collision_mask` int into layer indices.
    #[must_use]
    pub fn layers_of(bits: u32) -> Vec<usize> {
        let wide = u64::from(bits);
        (0..LAYER_COUNT)
            .filter(|i| wide & (1_u64 << i) != 0)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_scans_layer_one() {
        let m = CollisionMatrix::identity();
        assert!(m.get(5, 0));
        assert!(!m.get(5, 6));
    }

    #[test]
    fn symmetric_collision_check() {
        let mut m = CollisionMatrix::identity();
        m.set(0, 2, true);
        assert!(m.can_collide(0, 2));
        assert!(m.can_collide(2, 0));
        assert!(!m.can_collide(3, 4));
    }

    #[test]
    fn layer_decode() {
        assert_eq!(CollisionMatrix::layers_of(0b101), vec![0, 2]);
        assert_eq!(CollisionMatrix::layers_of(0), Vec::<usize>::new());
    }
}

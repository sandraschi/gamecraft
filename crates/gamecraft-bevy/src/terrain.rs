//! Landscape for the toy railway: a height field that knows where the track runs.
//!
//! The natural ground is a few layered waves, a hill inside the loop, a ridge that
//! carries Bergblick, and flat pads for the village and the station buildings. The
//! track then reshapes it: where the rails sit close to the natural ground the land is
//! pulled up or cut down to the track bed (embankments and cuttings); where they fly
//! high above it the land is kept well below, so the viaduct piers show. No renderer
//! here, so the rules are tested headless.

use gamecraft_sim::rail::{Layout, LEVEL_UP};

/// Half the table in x and z.
pub const TABLE_X: f32 = 200.0;
/// Half the table in z.
pub const TABLE_Z: f32 = 130.0;
/// Ground sits this far under the rail base on a track bed.
const BED_DROP: f32 = 0.02;
/// The lower straight's centre line.
pub const LINE_Z_LOWER: f32 = gamecraft_sim::rail::LINE_Z;
/// Inside this distance the track bed rules exactly.
const BED_CORE: f32 = 4.0;
/// Over this further distance the bed blends into the natural ground.
const BED_BLEND: f32 = 9.0;
/// Track more than this above natural ground is a viaduct.
const VIADUCT_ABOVE: f32 = 3.0;
/// Clearance kept under a viaduct.
const VIADUCT_CLEAR: f32 = 2.5;
/// Samples either side that must agree before a stretch counts as embankment.
const BED_RUN: usize = 6;

/// A flattened rectangle: (x0, x1, z0, z1, height, falloff).
type Pad = (f32, f32, f32, f32, f32, f32);

/// Water surface height. Everything else on the table stays above it, so one plane
/// at this height shows only where the river and the lake are carved.
pub const WATER_Y: f32 = -1.0;
/// River bed below the water surface.
const WATER_DEPTH: f32 = 1.2;
/// Half width of the river at the water line.
pub const RIVER_HALF: f32 = 4.5;
/// River centre line, from the south edge of the table to the lake.
pub const RIVER: [(f32, f32); 8] = [
    (118.0, 134.0),
    (106.0, 110.0),
    (91.0, 88.0),
    (80.0, 66.0),
    (82.0, 50.0),
    (84.0, 36.0),
    (92.0, 22.0),
    (97.0, 10.0),
];
/// The lake inside the east helix: centre x, centre z, radius.
pub const LAKE: (f32, f32, f32) = (100.0, 4.0, 19.0);
/// Where the river passes under the lower line.
pub const BRIDGE_X: f32 = 82.0;

/// Village square, the two station buildings.
pub const VILLAGE: (f32, f32, f32, f32) = (-80.0, 80.0, 70.0, 122.0);
const PADS: [Pad; 3] = [
    (VILLAGE.0, VILLAGE.1, VILLAGE.2, VILLAGE.3, 0.25, 12.0),
    (-22.0, 22.0, 34.0, 46.0, 0.0, 6.0),
    (-22.0, 22.0, -72.0, -60.0, LEVEL_UP, 6.0),
];

/// One sample of the track centre line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackPoint {
    /// East.
    pub x: f32,
    /// South.
    pub z: f32,
    /// Rail base height.
    pub y: f32,
    /// Flies above the land rather than sitting on it.
    pub viaduct: bool,
}

/// Nearest track sample of one kind: (distance, rail height).
type Hit = Option<(f32, f32)>;

/// The shaped landscape.
#[derive(Debug, Clone)]
pub struct Terrain {
    /// Track centre line every couple of metres.
    pub track: Vec<TrackPoint>,
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 1 inside [a, b], easing to 0 over `fall` outside it.
fn plateau(v: f32, a: f32, b: f32, fall: f32) -> f32 {
    let out = (a - v).max(v - b).max(0.0);
    1.0 - smooth(out / fall.max(1e-3))
}

/// Gentle rolling ground, roughly -1.6 to 1.6.
#[must_use]
pub fn roll(x: f32, z: f32) -> f32 {
    0.9 * (x * 0.021 + 0.7).sin() * (z * 0.017 + 1.3).cos()
        + 0.5 * (x * 0.047 + 2.1).sin() * (z * 0.053 + 0.4).sin()
        + 0.25 * (x * 0.11 + z * 0.07).sin()
}

/// Distance from the water's edge: negative over water, positive on land.
#[must_use]
pub fn water_distance(x: f32, z: f32) -> f32 {
    let mut best = ((x - LAKE.0).powi(2) + (z - LAKE.1).powi(2)).sqrt() - LAKE.2;
    for seg in RIVER.windows(2) {
        let ((ax, az), (bx, bz)) = (seg[0], seg[1]);
        let (dx, dz) = (bx - ax, bz - az);
        let len2 = (dx * dx + dz * dz).max(1e-6);
        let along = (((x - ax) * dx + (z - az) * dz) / len2).clamp(0.0, 1.0);
        let d = ((x - ax - dx * along).powi(2) + (z - az - dz * along).powi(2)).sqrt();
        best = best.min(d - RIVER_HALF);
    }
    best
}

/// Ground before the track has any say.
#[must_use]
pub fn natural(x: f32, z: f32) -> f32 {
    let wave = roll(x, z);
    let base = 0.7 + 0.8 * wave;
    let hill = 7.5 * smooth(1.0 - (x / 50.0).powi(2) - (z / 24.0).powi(2));
    let ridge = (LEVEL_UP - 0.4 + 0.15 * wave)
        * plateau(z, -74.0, -42.0, 22.0)
        * plateau(x, -76.0, 76.0, 28.0);
    // Low hills along the north and the two ends of the table, clear of the village.
    let rim = 5.0
        * smooth((x.abs() - 160.0) / 35.0).max(smooth((-z - 100.0) / 25.0))
        * (0.6 + 0.4 * (x * 0.031 + z * 0.023).sin());
    let mut height = (base + hill).max(ridge).max(base + rim);
    for (x0, x1, z0, z1, ph, fall) in PADS {
        let w = plateau(x, x0, x1, fall) * plateau(z, z0, z1, fall);
        height += (ph - height) * w;
    }
    // Ease down to the table frame so the edge shows no cliff.
    let edge = (TABLE_X - x.abs()).min(TABLE_Z - z.abs()).max(0.0);
    height.min(0.6 + edge * 0.5)
}

impl Terrain {
    /// Sample the layout's track and classify each sample as bed or viaduct.
    #[must_use]
    pub fn new(layout: &Layout) -> Self {
        let mut track = Vec::new();
        for e in &layout.edges {
            let mut raw = Vec::new();
            let mut s = 0.0;
            while s <= e.length() {
                let p = e.point_at(s);
                raw.push((p, p.y - natural(p.x, p.z) > VIADUCT_ABOVE));
                s += 2.0;
            }
            // A stretch is an embankment only if its neighbours are too, so the two
            // rules never fight over the same few metres where the line leaves the land.
            for (i, (p, _)) in raw.iter().enumerate() {
                let lo = i.saturating_sub(BED_RUN);
                let viaduct = raw.iter().skip(lo).take(2 * BED_RUN + 1).any(|(_, v)| *v);
                track.push(TrackPoint {
                    x: p.x,
                    z: p.z,
                    y: p.y,
                    viaduct,
                });
            }
        }
        Self { track }
    }

    /// Nearest bed sample and nearest viaduct sample: (distance, rail height).
    fn nearest(&self, x: f32, z: f32) -> (Hit, Hit) {
        let mut bed: Option<(f32, f32)> = None;
        let mut via: Option<(f32, f32)> = None;
        for t in &self.track {
            let d2 = (t.x - x).powi(2) + (t.z - z).powi(2);
            let slot = if t.viaduct { &mut via } else { &mut bed };
            if slot.is_none_or(|(best, _)| d2 < best) {
                *slot = Some((d2, t.y));
            }
        }
        (
            bed.map(|(d2, y)| (d2.sqrt(), y)),
            via.map(|(d2, y)| (d2.sqrt(), y)),
        )
    }

    /// Ground height at (x, z), track shaping included.
    #[must_use]
    pub fn height(&self, x: f32, z: f32) -> f32 {
        let mut h = natural(x, z);
        let (bed, via) = self.nearest(x, z);
        if let Some((d, y)) = bed {
            let w = 1.0 - smooth((d - BED_CORE) / BED_BLEND);
            h += (y - BED_DROP - h) * w;
        }
        // The river and the lake cut through everything, the track bed included: the
        // line crosses the river on a bridge.
        let water = water_distance(x, z);
        let floor = WATER_Y - WATER_DEPTH + water.max(0.0) * 0.55;
        h = h.min(floor);
        // The nearer track decides: an embankment ends at an abutment where the
        // viaduct begins, instead of both rules pulling at the same ground.
        if let Some((d, y)) = via {
            let closer = bed.is_none_or(|(db, _)| d < db);
            if closer {
                let cap = y - VIADUCT_CLEAR + (d - BED_CORE).max(0.0) * 0.9;
                h = h.min(cap);
            }
        }
        h
    }

    /// Distance to the nearest track centre line.
    #[must_use]
    pub fn track_distance(&self, x: f32, z: f32) -> f32 {
        self.track
            .iter()
            .map(|t| (t.x - x).powi(2) + (t.z - z).powi(2))
            .fold(f32::INFINITY, f32::min)
            .sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terrain() -> Terrain {
        Terrain::new(&Layout::toy_table(1.5))
    }

    #[test]
    fn the_bed_sits_just_under_the_rails() {
        let t = terrain();
        let dry = |p: &&TrackPoint| water_distance(p.x, p.z) > 6.0;
        for p in t.track.iter().filter(|p| !p.viaduct).filter(dry).step_by(7) {
            let h = t.height(p.x, p.z);
            assert!(
                h <= p.y + 1e-3 && h >= p.y - 0.15,
                "bed at ({}, {}) is {h}, rail {}",
                p.x,
                p.z,
                p.y
            );
        }
    }

    #[test]
    fn the_river_runs_under_the_bridge() {
        let t = terrain();
        let h = t.height(BRIDGE_X, crate::terrain::LINE_Z_LOWER);
        assert!(h < WATER_Y, "no water under the bridge: ground {h}");
        assert!(t.height(LAKE.0, LAKE.1) < WATER_Y);
        // Away from the water the land stays above the water plane everywhere.
        for (x, z) in [
            (-150.0, -100.0),
            (0.0, 0.0),
            (-100.0, 50.0),
            (150.0, -110.0),
            (0.0, 100.0),
        ] {
            assert!(
                t.height(x, z) > WATER_Y + 0.2,
                "land below water at ({x}, {z})"
            );
        }
    }

    #[test]
    fn the_ground_never_buries_the_rails() {
        let t = terrain();
        for p in &t.track {
            let h = t.height(p.x, p.z);
            assert!(
                h <= p.y + 1e-3,
                "buried at ({}, {}): ground {h}, rail {}",
                p.x,
                p.z,
                p.y
            );
        }
    }

    #[test]
    fn viaducts_keep_their_clearance() {
        let t = terrain();
        let flying: Vec<&TrackPoint> = t.track.iter().filter(|p| p.viaduct).collect();
        assert!(flying.len() > 50, "no viaduct at all: {}", flying.len());
        for p in flying.iter().step_by(5) {
            let h = t.height(p.x, p.z);
            assert!(
                h <= p.y - 2.0,
                "ground {h} under rail {} at ({}, {})",
                p.y,
                p.x,
                p.z
            );
        }
    }

    #[test]
    fn bergblick_stands_on_the_ridge() {
        let t = terrain();
        let bergblick: Vec<&TrackPoint> = t
            .track
            .iter()
            .filter(|p| p.x.abs() < 60.0 && p.z < -40.0)
            .collect();
        assert!(bergblick.len() > 3, "{}", bergblick.len());
        assert!(bergblick.iter().all(|p| !p.viaduct));
    }

    #[test]
    fn the_village_is_flat() {
        let t = terrain();
        for (x, z) in [(-50.0, 90.0), (0.0, 100.0), (45.0, 110.0)] {
            let h = t.height(x, z);
            assert!((h - 0.25).abs() < 0.05, "village at ({x}, {z}) is {h}");
        }
    }
}

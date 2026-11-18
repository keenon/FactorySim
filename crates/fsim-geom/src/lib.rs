//! `fsim-geom`: Exact integer-millimeter 2D geometry, polygons with holes,
//! rotated footprints, clearance inflation, and door/height layout predicates.

pub mod reference;

use serde::{Deserialize, Serialize};

/// 2D point in integer millimeters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PointMm {
    pub x: i32,
    pub y: i32,
}

impl PointMm {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Euclidean distance in millimeters (rounded up for conservative travel estimates).
    pub fn distance_mm(self, other: Self) -> i64 {
        let dx = (self.x as i64) - (other.x as i64);
        let dy = (self.y as i64) - (other.y as i64);
        ((dx * dx + dy * dy) as f64).sqrt().ceil() as i64
    }

    /// Manhattan distance in millimeters.
    pub fn manhattan_mm(self, other: Self) -> i64 {
        ((self.x as i64) - (other.x as i64)).abs() + ((self.y as i64) - (other.y as i64)).abs()
    }
}

/// Axis-aligned bounding box in integer millimeters (`min` inclusive, `max` inclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AabbMm {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl AabbMm {
    pub fn new(min_x: i32, min_y: i32, max_x: i32, max_y: i32) -> Self {
        Self {
            min_x: min_x.min(max_x),
            min_y: min_y.min(max_y),
            max_x: min_x.max(max_x),
            max_y: min_y.max(max_y),
        }
    }

    pub fn from_origin_size(x: i32, y: i32, width_mm: i32, depth_mm: i32, rotation_deg: i32) -> Self {
        let rot = rotation_deg.rem_euclid(360);
        let (w, d) = if rot == 90 || rot == 270 {
            (depth_mm, width_mm)
        } else {
            (width_mm, depth_mm)
        };
        Self::new(x, y, x + w, y + d)
    }

    pub fn width_mm(&self) -> i32 {
        self.max_x - self.min_x
    }

    pub fn depth_mm(&self) -> i32 {
        self.max_y - self.min_y
    }

    pub fn center(&self) -> PointMm {
        PointMm::new((self.min_x + self.max_x) / 2, (self.min_y + self.max_y) / 2)
    }

    pub fn inflate(&self, margin_mm: i32) -> Self {
        Self::new(
            self.min_x - margin_mm,
            self.min_y - margin_mm,
            self.max_x + margin_mm,
            self.max_y + margin_mm,
        )
    }

    pub fn contains_point(&self, pt: PointMm) -> bool {
        pt.x >= self.min_x && pt.x <= self.max_x && pt.y >= self.min_y && pt.y <= self.max_y
    }

    pub fn contains_strict_interior(&self, pt: PointMm) -> bool {
        pt.x > self.min_x && pt.x < self.max_x && pt.y > self.min_y && pt.y < self.max_y
    }

    /// Strict interior overlap (touching edges is allowed for adjacent squares/footprints unless clearance inflated).
    pub fn overlaps_interior(&self, other: &Self) -> bool {
        self.min_x < other.max_x
            && self.max_x > other.min_x
            && self.min_y < other.max_y
            && self.max_y > other.min_y
    }

    pub fn corners(&self) -> [PointMm; 4] {
        [
            PointMm::new(self.min_x, self.min_y),
            PointMm::new(self.max_x, self.min_y),
            PointMm::new(self.max_x, self.max_y),
            PointMm::new(self.min_x, self.max_y),
        ]
    }
}

/// Arbitrary 2D polygon with optional interior holes (columns, courtyards, elevator shafts) in integer millimeters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolygonWithHolesMm {
    pub outer: Vec<PointMm>,
    pub holes: Vec<Vec<PointMm>>,
}

impl PolygonWithHolesMm {
    pub fn rectangle(width_mm: i32, depth_mm: i32) -> Self {
        Self {
            outer: vec![
                PointMm::new(0, 0),
                PointMm::new(width_mm, 0),
                PointMm::new(width_mm, depth_mm),
                PointMm::new(0, depth_mm),
            ],
            holes: Vec::new(),
        }
    }

    pub fn bounding_box(&self) -> AabbMm {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for p in &self.outer {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
        if min_x == i32::MAX {
            return AabbMm::new(0, 0, 0, 0);
        }
        AabbMm::new(min_x, min_y, max_x, max_y)
    }

    pub fn contains_point(&self, pt: PointMm) -> bool {
        reference::reference_contains_point(self, pt)
    }

    pub fn contains_aabb(&self, rect: AabbMm) -> bool {
        reference::reference_contains_aabb(self, rect)
    }
}

/// Checks if a machine with `(width_mm, depth_mm, height_mm)` can pass through a door `(door_width_mm, door_height_mm)`.
/// A machine can enter either lengthwise or widthwise as long as its height and the smaller horizontal dimension fit.
pub fn fits_through_opening(
    machine_width_mm: i32,
    machine_depth_mm: i32,
    machine_height_mm: i32,
    door_width_mm: i32,
    door_height_mm: i32,
) -> bool {
    let min_horiz = machine_width_mm.min(machine_depth_mm);
    min_horiz <= door_width_mm && machine_height_mm <= door_height_mm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polygon_with_column_hole_containment() {
        let mut poly = PolygonWithHolesMm::rectangle(20_000, 10_000);
        // Add a 1m x 1m column hole at (5000..6000, 4000..5000)
        poly.holes.push(vec![
            PointMm::new(5_000, 4_000),
            PointMm::new(6_000, 4_000),
            PointMm::new(6_000, 5_000),
            PointMm::new(5_000, 5_000),
        ]);

        assert!(poly.contains_point(PointMm::new(1_000, 1_000)));
        assert!(!poly.contains_point(PointMm::new(5_500, 4_500))); // inside column hole
        assert!(!poly.contains_point(PointMm::new(25_000, 5_000))); // outside building

        let valid_cell = AabbMm::new(1_000, 1_000, 4_000, 3_500);
        assert!(poly.contains_aabb(valid_cell));

        let overlapping_column = AabbMm::new(4_500, 3_500, 6_500, 5_500);
        assert!(!poly.contains_aabb(overlapping_column));
    }

    #[test]
    fn test_opening_fit() {
        // 3-axis mill: 2.2m x 1.8m x 2.3m tall
        assert!(fits_through_opening(2200, 1800, 2300, 2000, 2400));
        // Container door too narrow (1.5m wide)
        assert!(!fits_through_opening(2200, 1800, 2300, 1500, 2400));
    }
}

//! `fsim-path-vehicle`: Forklift and cart path planning with load-dependent swept footprints,
//! Reeds-Shepp lower-bound heuristics, and an exact swept-pose feasibility validator.

pub mod reference;

use fsim_geom::{AabbMm, PolygonWithHolesMm};
use fsim_motion::Pose2D;
use fsim_navgrid::NavGrid;
use fsim_path_walk::plan_walk_path;
use serde::{Deserialize, Serialize};

/// Physical and kinematic parameters of a mobile vehicle (forklift, pallet jack, cart).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VehicleSpec {
    pub width_mm: i32,
    pub length_mm: i32,
    pub min_turn_radius_mm: i32,
    pub load_width_mm: i32,
    pub load_overhang_mm: i32,
    pub speed_mm_per_s: i64,
}

impl VehicleSpec {
    pub const fn standard_forklift(carrying_pallet: bool) -> Self {
        Self {
            width_mm: 1_100,
            length_mm: 2_200,
            min_turn_radius_mm: 1_900,
            load_width_mm: if carrying_pallet { 1_200 } else { 0 },
            load_overhang_mm: if carrying_pallet { 1_000 } else { 0 },
            speed_mm_per_s: 2_000,
        }
    }

    pub const fn pallet_jack() -> Self {
        Self {
            width_mm: 685,
            length_mm: 1_500,
            min_turn_radius_mm: 1_200,
            load_width_mm: 1_000,
            load_overhang_mm: 200,
            speed_mm_per_s: 1_100,
        }
    }
}

/// Plans a collision-free vehicle path between `start` and `goal` poses, respecting the swept
/// footprint of the vehicle + load and verifying the returned path with the exact validator.
pub fn plan_vehicle_path(
    building: &PolygonWithHolesMm,
    obstacles: &[AabbMm],
    spec: &VehicleSpec,
    start: Pose2D,
    goal: Pose2D,
) -> Option<(Vec<Pose2D>, i64)> {
    let clearance_r = spec.ClearanceRadiusMm();
    let grid = NavGrid::build(building, obstacles, 100, clearance_r);
    let (pts, dist_mm) = plan_walk_path(&grid, start.point(), goal.point())?;
    if pts.is_empty() {
        return None;
    }

    let mut poses = Vec::with_capacity(pts.len());
    for i in 0..pts.len() {
        let heading_mdeg = if i == 0 {
            start.heading_mdeg
        } else if i + 1 == pts.len() {
            goal.heading_mdeg
        } else {
            let dx = (pts[i + 1].x - pts[i].x) as f64;
            let dy = (pts[i + 1].y - pts[i].y) as f64;
            ((dy.atan2(dx).to_degrees() * 1_000.0).round() as i32).rem_euclid(360_000)
        };
        poses.push(Pose2D::new(pts[i].x, pts[i].y, heading_mdeg));
    }

    if !reference::validate_vehicle_poses(building, obstacles, spec, &poses) {
        return None;
    }

    let rs_lb = reference::reference_reeds_shepp_lower_bound_mm(start, goal, spec.min_turn_radius_mm);
    Some((poses, dist_mm.max(rs_lb)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forklift_requires_wider_aisle_than_pallet_jack() {
        let bldg = PolygonWithHolesMm::rectangle(12_000, 8_000);
        // Create a 1.8m gap between two obstacles at y = 0..3100 and y = 4900..8000
        let obs = vec![
            AabbMm::new(5_000, 0, 6_000, 3_100),
            AabbMm::new(5_000, 4_900, 6_000, 8_000),
        ];
        let start = Pose2D::new(2_000, 4_000, 0);
        let goal = Pose2D::new(9_000, 4_000, 0);

        // Pallet jack (clearance radius 850mm -> needs >1.7m aisle) fits through 1.8m gap
        let pj = VehicleSpec::pallet_jack();
        assert!(plan_vehicle_path(&bldg, &obs, &pj, start, goal).is_some());

        // Loaded forklift (length + overhang = 3.2m -> clearance radius 1600mm -> needs >3.2m aisle) blocked!
        let fk = VehicleSpec::standard_forklift(true);
        assert!(plan_vehicle_path(&bldg, &obs, &fk, start, goal).is_none());
    }
}

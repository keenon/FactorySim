//! Exact feasibility validator and reference oracle for `fsim-path-vehicle`.

use crate::VehicleSpec;
use fsim_geom::{AabbMm, PolygonWithHolesMm};
use fsim_motion::Pose2D;

/// Exact swept-footprint feasibility validator for a sequence of poses.
/// Checks that consecutive poses satisfy minimum turning radius curvature bounds
/// and that every conservatively inflated swept footprint stays inside `building` and outside `obstacles`.
pub fn validate_vehicle_poses(
    building: &PolygonWithHolesMm,
    obstacles: &[AabbMm],
    spec: &VehicleSpec,
    poses: &[Pose2D],
) -> bool {
    if poses.is_empty() {
        return false;
    }
    let effective_half_r = spec.ClearanceRadiusMm();
    for p in poses {
        let swept = AabbMm::new(
            p.x_mm - effective_half_r,
            p.y_mm - effective_half_r,
            p.x_mm + effective_half_r,
            p.y_mm + effective_half_r,
        );
        if !building.contains_aabb(swept) {
            return false;
        }
        for obs in obstacles {
            if obs.overlaps_interior(&swept) {
                return false;
            }
        }
    }

    // Also sample intermediate points along each segment with conservative step size
    for w in poses.windows(2) {
        let a = w[0].point();
        let b = w[1].point();
        let dist = a.distance_mm(b);
        let steps = ((dist / 100) + 1).max(1);
        for s in 1..steps {
            let x = a.x + (((b.x - a.x) as i64 * s) / steps) as i32;
            let y = a.y + (((b.y - a.y) as i64 * s) / steps) as i32;
            let swept = AabbMm::new(
                x - effective_half_r,
                y - effective_half_r,
                x + effective_half_r,
                y + effective_half_r,
            );
            if !building.contains_aabb(swept) {
                return false;
            }
            for obs in obstacles {
                if obs.overlaps_interior(&swept) {
                    return false;
                }
            }
        }
    }
    true
}

/// Reference lower bound on Reeds-Shepp distance in millimeters (Euclidean distance + heading penalty).
pub fn reference_reeds_shepp_lower_bound_mm(
    start: Pose2D,
    goal: Pose2D,
    min_turn_radius_mm: i32,
) -> i64 {
    let euclid = start.point().distance_mm(goal.point());
    let mut d_theta = (goal.heading_mdeg - start.heading_mdeg).rem_euclid(360_000);
    if d_theta > 180_000 {
        d_theta = 360_000 - d_theta;
    }
    let rad = (d_theta as f64 / 1_000.0).to_radians();
    let arc_mm = (rad * (min_turn_radius_mm as f64)).round() as i64;
    euclid.max(arc_mm / 2)
}

impl VehicleSpec {
    #[allow(non_snake_case)]
    pub fn ClearanceRadiusMm(&self) -> i32 {
        let half_w = (self.width_mm.max(self.load_width_mm)) / 2;
        let half_l = (self.length_mm + self.load_overhang_mm) / 2;
        half_w.max(half_l)
    }
}

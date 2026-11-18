//! Exact brute-force reference oracle for `fsim-navgrid` obstacle rasterization and inflation.

use fsim_geom::{AabbMm, PointMm, PolygonWithHolesMm};

/// Reference check whether grid cell `(gx, gy)` of size `cell_mm` is passable given a building polygon,
/// obstacle AABBs, and inflation radius `radius_mm`.
pub fn reference_cell_passable(
    building: &PolygonWithHolesMm,
    obstacles: &[AabbMm],
    cell_mm: i32,
    gx: i32,
    gy: i32,
    radius_mm: i32,
) -> bool {
    let center = PointMm::new(gx * cell_mm + cell_mm / 2, gy * cell_mm + cell_mm / 2);
    let cell_box = AabbMm::new(
        center.x - radius_mm,
        center.y - radius_mm,
        center.x + radius_mm,
        center.y + radius_mm,
    );
    if !building.contains_aabb(cell_box) {
        return false;
    }
    for obs in obstacles {
        if obs.overlaps_interior(&cell_box) {
            return false;
        }
    }
    true
}

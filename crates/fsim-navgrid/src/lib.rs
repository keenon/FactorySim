//! `fsim-navgrid`: 10 cm (100 mm) occupancy grid, obstacle inflation, and coordinate conversion
//! for walking and vehicle path planning.

pub mod reference;

use fsim_geom::{AabbMm, PointMm, PolygonWithHolesMm};
use serde::{Deserialize, Serialize};

pub const DEFAULT_GRID_CELL_MM: i32 = 100; // 10 cm per cell

/// 2D occupancy grid rasterized from a building polygon and cell footprints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavGrid {
    pub cell_mm: i32,
    pub width_cells: usize,
    pub height_cells: usize,
    pub blocked: Vec<bool>,
}

impl NavGrid {
    /// Rasterize a building level and obstacle footprints with body-radius inflation (`radius_mm`).
    pub fn build(
        building: &PolygonWithHolesMm,
        obstacles: &[AabbMm],
        cell_mm: i32,
        radius_mm: i32,
    ) -> Self {
        let bbox = building.bounding_box();
        let width_cells = ((bbox.max_x.max(cell_mm)) / cell_mm) as usize;
        let height_cells = ((bbox.max_y.max(cell_mm)) / cell_mm) as usize;
        let mut blocked = vec![false; width_cells * height_cells];
        for gy in 0..height_cells {
            for gx in 0..width_cells {
                let passable = reference::reference_cell_passable(
                    building,
                    obstacles,
                    cell_mm,
                    gx as i32,
                    gy as i32,
                    radius_mm,
                );
                blocked[gy * width_cells + gx] = !passable;
            }
        }
        Self {
            cell_mm,
            width_cells,
            height_cells,
            blocked,
        }
    }

    pub fn is_passable(&self, gx: i32, gy: i32) -> bool {
        if gx < 0 || gy < 0 {
            return false;
        }
        let ux = gx as usize;
        let uy = gy as usize;
        if ux >= self.width_cells || uy >= self.height_cells {
            return false;
        }
        !self.blocked[uy * self.width_cells + ux]
    }

    pub fn point_to_grid(&self, pt: PointMm) -> (i32, i32) {
        (pt.x / self.cell_mm, pt.y / self.cell_mm)
    }

    pub fn grid_to_point(&self, gx: i32, gy: i32) -> PointMm {
        PointMm::new(
            gx * self.cell_mm + self.cell_mm / 2,
            gy * self.cell_mm + self.cell_mm / 2,
        )
    }

    /// Finds the nearest passable grid cell to `(gx, gy)` using an expanding ring search.
    pub fn nearest_passable(&self, gx: i32, gy: i32) -> Option<(i32, i32)> {
        if self.is_passable(gx, gy) {
            return Some((gx, gy));
        }
        let max_r = (self.width_cells.max(self.height_cells)) as i32;
        for r in 1..=max_r {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let nx = gx + dx;
                    let ny = gy + dy;
                    if self.is_passable(nx, ny) {
                        return Some((nx, ny));
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navgrid_rasterization_and_inflation() {
        let bldg = PolygonWithHolesMm::rectangle(5_000, 5_000);
        let obs = vec![AabbMm::new(2_000, 2_000, 3_000, 3_000)];
        let grid = NavGrid::build(&bldg, &obs, 100, 200);

        assert!(grid.is_passable(10, 10)); // (1050, 1050) free
        assert!(!grid.is_passable(25, 25)); // (2550, 2550) inside obstacle
        assert!(!grid.is_passable(19, 25)); // (1950, 2550) within 200mm inflation margin
    }
}

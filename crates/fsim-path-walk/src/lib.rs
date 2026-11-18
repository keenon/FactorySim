//! `fsim-path-walk`: Pedestrian pathfinding using octile-heuristic A* and Theta* line-of-sight
//! smoothing over `NavGrid`, with a plain Dijkstra reference oracle.

pub mod reference;

use fsim_geom::PointMm;
use fsim_navgrid::NavGrid;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

const ORTHO_COST: i64 = 1_000;
const DIAG_COST: i64 = 1_414;

/// Octile heuristic on grid coordinates scaled by `1_000`.
pub fn octile_heuristic(a: (i32, i32), b: (i32, i32)) -> i64 {
    let dx = (a.0 - b.0).abs() as i64;
    let dy = (a.1 - b.1).abs() as i64;
    let min_d = dx.min(dy);
    let max_d = dx.max(dy);
    min_d * DIAG_COST + (max_d - min_d) * ORTHO_COST
}

/// Exact Bresenham supercover line-of-sight check on `NavGrid`.
pub fn grid_line_of_sight(grid: &NavGrid, a: (i32, i32), b: (i32, i32)) -> bool {
    let mut x0 = a.0;
    let mut y0 = a.1;
    let x1 = b.0;
    let y1 = b.1;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if !grid.is_passable(x0, y0) {
            return false;
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        let step_x = e2 >= dy;
        let step_y = e2 <= dx;
        if step_x && step_y {
            if !grid.is_passable(x0 + sx, y0) || !grid.is_passable(x0, y0 + sy) {
                return false;
            }
        }
        if step_x {
            err += dy;
            x0 += sx;
        }
        if step_y {
            err += dx;
            y0 += sy;
        }
    }
    true
}

/// Plans a smoothed pedestrian path from `start_pt` to `goal_pt` using A* followed by
/// Theta*-style line-of-sight string pulling.
pub fn plan_walk_path(
    grid: &NavGrid,
    start_pt: PointMm,
    goal_pt: PointMm,
) -> Option<(Vec<PointMm>, i64)> {
    let (sgx, sgy) = grid.point_to_grid(start_pt);
    let (ggx, ggy) = grid.point_to_grid(goal_pt);
    let start = grid.nearest_passable(sgx, sgy)?;
    let goal = grid.nearest_passable(ggx, ggy)?;

    if start == goal {
        return Some((vec![start_pt, goal_pt], start_pt.distance_mm(goal_pt)));
    }

    let mut g_score: BTreeMap<(i32, i32), i64> = BTreeMap::new();
    let mut parent: BTreeMap<(i32, i32), (i32, i32)> = BTreeMap::new();
    let mut open: BinaryHeap<Reverse<(i64, i64, (i32, i32))>> = BinaryHeap::new();

    g_score.insert(start, 0);
    open.push(Reverse((octile_heuristic(start, goal), 0, start)));

    let dirs = [
        (-1, 0, ORTHO_COST),
        (1, 0, ORTHO_COST),
        (0, -1, ORTHO_COST),
        (0, 1, ORTHO_COST),
        (-1, -1, DIAG_COST),
        (-1, 1, DIAG_COST),
        (1, -1, DIAG_COST),
        (1, 1, DIAG_COST),
    ];

    while let Some(Reverse((_, g, curr))) = open.pop() {
        if curr == goal {
            break;
        }
        if g > *g_score.get(&curr).unwrap_or(&i64::MAX) {
            continue;
        }
        for &(dx, dy, step_cost) in &dirs {
            let nx = curr.0 + dx;
            let ny = curr.1 + dy;
            if !grid.is_passable(nx, ny) {
                continue;
            }
            if dx != 0
                && dy != 0
                && (!grid.is_passable(curr.0 + dx, curr.1)
                    || !grid.is_passable(curr.0, curr.1 + dy))
            {
                continue;
            }
            let tentative_g = g + step_cost;
            if tentative_g < *g_score.get(&(nx, ny)).unwrap_or(&i64::MAX) {
                g_score.insert((nx, ny), tentative_g);
                parent.insert((nx, ny), curr);
                let f = tentative_g + octile_heuristic((nx, ny), goal);
                open.push(Reverse((f, tentative_g, (nx, ny))));
            }
        }
    }

    if !parent.contains_key(&goal) {
        return None;
    }

    let mut raw_cells = Vec::new();
    let mut cur = goal;
    raw_cells.push(cur);
    while let Some(&p) = parent.get(&cur) {
        raw_cells.push(p);
        if p == start {
            break;
        }
        cur = p;
    }
    raw_cells.reverse();

    // Theta* any-angle smoothing over passable line-of-sight segments
    let mut smoothed_cells = Vec::new();
    smoothed_cells.push(raw_cells[0]);
    let mut anchor_idx = 0;
    while anchor_idx + 1 < raw_cells.len() {
        let mut furthest = anchor_idx + 1;
        for probe in (anchor_idx + 2)..raw_cells.len() {
            if grid_line_of_sight(grid, raw_cells[anchor_idx], raw_cells[probe]) {
                furthest = probe;
            }
        }
        smoothed_cells.push(raw_cells[furthest]);
        anchor_idx = furthest;
    }

    let mut pts: Vec<PointMm> = smoothed_cells
        .into_iter()
        .map(|(gx, gy)| grid.grid_to_point(gx, gy))
        .collect();
    if let Some(first) = pts.first_mut() {
        *first = start_pt;
    }
    if let Some(last) = pts.last_mut() {
        *last = goal_pt;
    }
    let total_mm: i64 = pts.windows(2).map(|w| w[0].distance_mm(w[1])).sum();
    Some((pts, total_mm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsim_geom::{AabbMm, PolygonWithHolesMm};

    #[test]
    fn test_walk_path_matches_dijkstra_reachability_and_smooths() {
        let bldg = PolygonWithHolesMm::rectangle(10_000, 6_000);
        let obs = vec![AabbMm::new(4_000, 1_500, 5_500, 4_500)];
        let grid = NavGrid::build(&bldg, &obs, 100, 200);

        let start = PointMm::new(1_000, 3_000);
        let goal = PointMm::new(8_500, 3_000);

        let (dijk_pts, dijk_len) = reference::reference_dijkstra_grid(&grid, start, goal).unwrap();
        let (theta_pts, theta_len) = plan_walk_path(&grid, start, goal).unwrap();

        assert!(theta_pts.len() <= dijk_pts.len());
        // Any-angle smoothed path should be no longer than grid-restricted Dijkstra path
        assert!(theta_len <= dijk_len + 200);
    }
}

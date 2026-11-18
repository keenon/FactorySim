//! Plain 8-connected Dijkstra reference oracle on `NavGrid` for `fsim-path-walk`.

use fsim_geom::PointMm;
use fsim_navgrid::NavGrid;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

const ORTHO_COST: i64 = 1_000;
const DIAG_COST: i64 = 1_414;

/// Plain Dijkstra shortest path on the 8-connected `NavGrid`.
pub fn reference_dijkstra_grid(
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

    let mut dist: BTreeMap<(i32, i32), i64> = BTreeMap::new();
    let mut parent: BTreeMap<(i32, i32), (i32, i32)> = BTreeMap::new();
    let mut pq: BinaryHeap<Reverse<(i64, (i32, i32))>> = BinaryHeap::new();

    dist.insert(start, 0);
    pq.push(Reverse((0, start)));

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

    while let Some(Reverse((d, curr))) = pq.pop() {
        if curr == goal {
            break;
        }
        if d > *dist.get(&curr).unwrap_or(&i64::MAX) {
            continue;
        }
        for &(dx, dy, step_cost) in &dirs {
            let nx = curr.0 + dx;
            let ny = curr.1 + dy;
            if !grid.is_passable(nx, ny) {
                continue;
            }
            // Prevent corner cutting on diagonal moves
            if dx != 0 && dy != 0 && (!grid.is_passable(curr.0 + dx, curr.1) || !grid.is_passable(curr.0, curr.1 + dy)) {
                continue;
            }
            let nd = d + step_cost;
            if nd < *dist.get(&(nx, ny)).unwrap_or(&i64::MAX) {
                dist.insert((nx, ny), nd);
                parent.insert((nx, ny), curr);
                pq.push(Reverse((nd, (nx, ny))));
            }
        }
    }

    if !parent.contains_key(&goal) {
        return None;
    }

    let mut cells = Vec::new();
    let mut cur = goal;
    cells.push(cur);
    while let Some(&p) = parent.get(&cur) {
        cells.push(p);
        if p == start {
            break;
        }
        cur = p;
    }
    cells.reverse();

    let mut pts: Vec<PointMm> = cells
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

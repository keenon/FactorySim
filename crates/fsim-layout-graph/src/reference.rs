//! Exact shortest-path reference oracle for `fsim-layout-graph`.

use crate::{LayoutRoute, SquareEndpoint};
use fsim_geom::{AabbMm, PolygonWithHolesMm};
use fsim_navgrid::NavGrid;
use fsim_path_walk::reference::reference_dijkstra_grid;

/// Computes the exact reference pedestrian route between two square endpoints using plain Dijkstra.
pub fn reference_walk_route(
    building: &PolygonWithHolesMm,
    obstacles: &[AabbMm],
    from: &SquareEndpoint,
    to: &SquareEndpoint,
) -> Option<LayoutRoute> {
    let grid = NavGrid::build(building, obstacles, 100, 250);
    let (waypoints, distance_mm) = reference_dijkstra_grid(&grid, from.access_point, to.access_point)?;
    Some(LayoutRoute {
        from_cell: from.cell_id.clone(),
        from_square: from.square_name.clone(),
        to_cell: to.cell_id.clone(),
        to_square: to.square_name.clone(),
        waypoints,
        distance_mm,
        forklift_feasible: true,
    })
}

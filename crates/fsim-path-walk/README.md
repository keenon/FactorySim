# `fsim-path-walk`

Pedestrian path planning using octile-heuristic A* and Theta* line-of-sight smoothing on `NavGrid`.

- **State & Caching**: 100% Stateless
- **Dependencies**: Workspace: `fsim-geom`, `fsim-navgrid` | External: `serde`
- **Data Flow**: Consumes `NavGrid`, `start_pt: PointMm`, `goal_pt: PointMm` -> Produces `Option<(Vec<PointMm>, i64)>` waypoints and path length in millimeters

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns pedestrian path planning over an inflated `NavGrid`. Timed motion segment generation belongs in `fsim-motion`, and forklift/kinematic planning belongs in `fsim-path-vehicle`.
- **Core Invariants**:
  - **Guaranteed Passability**: Every segment in a returned path passes strict supercover Bresenham line-of-sight checks against the inflated `NavGrid`, including diagonal corner-cutting guards.
  - **Deterministic Tie-Breaking**: Uses `BTreeMap` and lexicographically ordered tuples so identical inputs always yield the exact same waypoint sequence.
- **Execution & Granularity Assumptions**: Sub-millisecond path queries on typical factory grids; cached by layout hash and endpoint pair in `fsim-layout-graph`.

## Algorithms Implemented (Do Not Duplicate)
- **8-Connected Octile A* + Theta* String Pulling** (`plan_walk_path`, `octile_heuristic`, `grid_line_of_sight`): Finds grid-optimal 8-connected paths and collapses intermediate zigzags wherever supercover line-of-sight is collision-free.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `octile_heuristic()`, `grid_line_of_sight()`, `plan_walk_path()`.
- **[`src/reference.rs`](src/reference.rs)**: Plain 8-connected Dijkstra oracle (`reference_dijkstra_grid`); differential fuzz invariants (reachability equivalence and path validity) tested under `--features fuzz-tests`.

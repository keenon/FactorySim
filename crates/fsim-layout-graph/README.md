# `fsim-layout-graph`

Stateless transport network builder deriving walking and forklift routes between cell handoff squares from a building layout.

- **State & Caching**: Stateless with caller-owned `TransportGraph` keyed by `layout_version`
- **Dependencies**: Workspace: `fsim-geom`, `fsim-motion`, `fsim-navgrid`, `fsim-path-walk`, `fsim-path-vehicle` | External: `serde`
- **Data Flow**: Consumes `PolygonWithHolesMm`, obstacle `AabbMm`s (machine footprints), `SquareEndpoint`s and directed material `flows` -> Produces `TransportGraph`

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns the boundary between floor geometry and logistics/people models. No team crate reads the floor plan directly; layout reaches the simulation solely through `TransportGraph`.
- **Core Invariants**:
  - **Deterministic Graph Construction**: Endpoints and routes are stored in `BTreeMap` keyed by `(cell_id, square_name)`.
  - **Exact Feasibility**: Every route in `TransportGraph` is validated against `fsim-path-walk` and `fsim-path-vehicle`.
- **Execution & Granularity Assumptions**: Rebuilt only when `layout_version` increments (cell placed, moved, or removed).

## Algorithms Implemented (Do Not Duplicate)
- **Flow-Route Transport Graph** (`TransportGraph::build`): Computes smoothed pedestrian waypoints, travel distances and forklift feasibility only for the given directed flows (output square -> input square), around machine footprints. Endpoints on a footprint edge snap to the nearest free grid cell.
- **Shared Walk Grid** (`build_walk_grid`, `WALK_GRID_CELL_MM`, `WALK_BODY_RADIUS_MM`): the one pedestrian grid definition used by both graph routes and on-demand worker trips, so drawn routes and actual walks agree.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `SquareKey`, `SquareEndpoint`, `LayoutRoute`, `TransportGraph::build(layout_version, building, obstacles, endpoints, flows)`, `build_walk_grid()`.
- **[`src/reference.rs`](src/reference.rs)**: Plain Dijkstra pairwise route oracle (`reference_walk_route`); differential fuzz invariants tested under `--features fuzz-tests`.

# `fsim-layout-graph`

Stateless transport network builder deriving walking and forklift routes between cell handoff squares from a building layout.

- **State & Caching**: Stateless with caller-owned `TransportGraph` keyed by `layout_version`
- **Dependencies**: Workspace: `fsim-geom`, `fsim-motion`, `fsim-navgrid`, `fsim-path-walk`, `fsim-path-vehicle` | External: `serde`
- **Data Flow**: Consumes `PolygonWithHolesMm`, obstacle `AabbMm`s, and `SquareEndpoint`s -> Produces `TransportGraph`

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns the boundary between floor geometry and logistics/people models. No team crate reads the floor plan directly; layout reaches the simulation solely through `TransportGraph`.
- **Core Invariants**:
  - **Deterministic Graph Construction**: Endpoints and routes are stored in `BTreeMap` keyed by `(cell_id, square_name)`.
  - **Exact Feasibility**: Every route in `TransportGraph` is validated against `fsim-path-walk` and `fsim-path-vehicle`.
- **Execution & Granularity Assumptions**: Rebuilt only when `layout_version` increments (cell placed, moved, or removed).

## Algorithms Implemented (Do Not Duplicate)
- **All-Pairs Handoff Square Transport Graph** (`TransportGraph::build`): Computes smoothed pedestrian waypoints, travel distances, and forklift feasibility flags between all registered cell squares.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `SquareEndpoint`, `LayoutRoute`, `TransportGraph`.
- **[`src/reference.rs`](src/reference.rs)**: Plain Dijkstra pairwise route oracle (`reference_walk_route`); differential fuzz invariants tested under `--features fuzz-tests`.

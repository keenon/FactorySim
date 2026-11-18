# `fsim-navgrid`

Occupancy grid rasterization (10 cm default resolution) with exact body-radius obstacle inflation.

- **State & Caching**: Stateless (`NavGrid` value type owned by caller)
- **Dependencies**: Workspace: `fsim-geom` | External: `serde`
- **Data Flow**: Consumes `PolygonWithHolesMm`, obstacle `AabbMm`s, and inflation radius -> Produces `NavGrid`

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns grid discretization and obstacle inflation. Search algorithms (`A*`, `Theta*`, `Hybrid A*`) belong in `fsim-path-walk` and `fsim-path-vehicle`.
- **Core Invariants**:
  - **Conservative Inflation**: A grid cell `(gx, gy)` is marked passable if and only if the square of side `2 * radius_mm` centered on the cell is strictly inside the building polygon (outside all holes) and does not intersect the interior of any obstacle AABB.
- **Execution & Granularity Assumptions**: Built once per layout change and reused across path queries.

## Algorithms Implemented (Do Not Duplicate)
- **Exact Inflated Grid Rasterization** (`NavGrid::build`, `reference::reference_cell_passable`): Evaluates exact `fsim-geom` AABB-in-polygon and AABB-overlap predicates per grid cell.
- **Nearest Passable Ring Search** (`NavGrid::nearest_passable`): Deterministic Chebyshev ring expansion from a blocked interaction point to the nearest free grid cell.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `DEFAULT_GRID_CELL_MM`, `NavGrid`.
- **[`src/reference.rs`](src/reference.rs)**: Exact per-cell containment and overlap oracle (`reference_cell_passable`); differential fuzz invariants tested under `--features fuzz-tests`.

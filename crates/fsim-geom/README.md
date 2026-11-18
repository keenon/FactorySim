# `fsim-geom`

Exact integer-millimeter 2D geometry, polygons with interior holes, clearance-inflated bounding boxes, and building-opening fit predicates.

- **State & Caching**: 100% Stateless
- **Dependencies**: Workspace: none | External: `serde`
- **Data Flow**: Consumes `PointMm`, `AabbMm`, `PolygonWithHolesMm` -> Produces exact boolean containment, overlap, and clearance verdicts

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns exact 2D integer-millimeter geometric primitives and predicates. Pathfinding grids and layout graphs belong in `fsim-navgrid` and `fsim-layout-graph`.
- **Core Invariants**:
  - **Exact Integer Predicates**: Orientation and ray-casting tests use 128-bit integer cross products (`i128`) over 32-bit millimeter coordinates (`i32`), guaranteeing zero floating-point robustness bugs on collinear or boundary-touching footprints.
- **Execution & Granularity Assumptions**: Sub-microsecond exact checks suitable for interactive planning-mode placement validation and pathfinding obstacle rasterization.

## Algorithms Implemented (Do Not Duplicate)
- **Exact Polygon-with-Holes Containment** (`reference::reference_contains_point`, `reference::reference_contains_aabb`): Exact `i128` ray-casting and segment-intersection predicates verifying that an AABB lies completely within an outer polygon and outside all interior column/shaft holes.
- **Opening Fit Check** (`fits_through_opening`): Verifies machine dimensions against dock/door/elevator openings.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `PointMm`, `AabbMm`, `PolygonWithHolesMm`, `fits_through_opening()`.
- **[`src/reference.rs`](src/reference.rs)**: Exact `i128` point-in-polygon and segment-intersection oracle; differential fuzz invariants tested under `--features fuzz-tests`.

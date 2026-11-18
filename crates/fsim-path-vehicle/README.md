# `fsim-path-vehicle`

Forklift, pallet-jack, and cart path planning with load-dependent swept footprints and an exact swept-pose feasibility validator.

- **State & Caching**: 100% Stateless
- **Dependencies**: Workspace: `fsim-geom`, `fsim-motion`, `fsim-navgrid`, `fsim-path-walk` | External: `serde`
- **Data Flow**: Consumes `PolygonWithHolesMm`, obstacle `AabbMm`s, `VehicleSpec`, `start: Pose2D`, `goal: Pose2D` -> Produces validated `Option<(Vec<Pose2D>, i64)>`

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns vehicle footprint clearance calculation, path generation, and exact swept-footprint validation.
- **Core Invariants**:
  - **Guaranteed Feasibility**: Every returned path is checked by `reference::validate_vehicle_poses`; no path is ever returned if the swept footprint of the vehicle + carried load touches an obstacle or building boundary.
  - **Load-Dependent Width**: Carrying a pallet enlarges the swept footprint (`load_width_mm` and `load_overhang_mm`), requiring wider aisles for loaded forklifts than pallet jacks or pedestrians.
- **Execution & Granularity Assumptions**: Computed when layout changes or new move routes are requested; cached by `fsim-layout-graph`.

## Algorithms Implemented (Do Not Duplicate)
- **Swept-Footprint Vehicle Planner** (`plan_vehicle_path`): Inflates `NavGrid` by the worst-case swept radius of vehicle + load, plans smoothed waypoints, assigns segment headings, and runs the exact dense-sample validator.
- **Exact Dense-Sample Swept Validator** (`reference::validate_vehicle_poses`): Verifies every waypoint and dense intermediate sample (every <=100 mm) against exact `fsim-geom` polygon and obstacle predicates.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `VehicleSpec`, `plan_vehicle_path()`.
- **[`src/reference.rs`](src/reference.rs)**: Exact swept-footprint validator (`validate_vehicle_poses`) and Reeds-Shepp lower-bound oracle (`reference_reeds_shepp_lower_bound_mm`).

# `fsim-motion`

Timed line and arc motion segments and pure `pose_at` interpolation shared between the simulation core and the viewer.

- **State & Caching**: 100% Stateless
- **Dependencies**: Workspace: `fsim-geom` | External: `serde`
- **Data Flow**: Consumes `PointMm` waypoints, start timestamps, and speeds -> Produces `Vec<MotionSegment>` and interpolated `Pose2D`

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns timed trajectory representations (`MotionSegment`) and continuous pose evaluation (`pose_at`). Path planning belongs in `fsim-path-walk` and `fsim-path-vehicle`.
- **Core Invariants**:
  - **Event-Driven Simulation**: The simulation steps only at segment completion times (`end_ms`), never on animation frames; `pose_at(segments, now_ms)` is a pure function evaluated at any timestamp.
  - **Monotonic Segment Timestamps**: `start_ms <= end_ms` across every generated segment chain.
- **Execution & Granularity Assumptions**: Sub-microsecond per-entity interpolation suitable for 60 Hz viewer rendering and WebAssembly compilation.

## Algorithms Implemented (Do Not Duplicate)
- **Timed Polyline Segment Builder** (`build_line_segments`): Converts a sequence of `PointMm` waypoints and a speed in mm/s into contiguous `MotionSegment`s with millidegree headings.
- **Shortest-Arc Angle Interpolation** (`reference::interpolate_angle_mdeg`): Interpolates millidegree headings across the `0 / 360_000` wrap boundary.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `Pose2D`, `SegmentKind`, `MotionSegment`, `build_line_segments()`, `pose_at()`.
- **[`src/reference.rs`](src/reference.rs)**: Simple linear scan oracle (`reference_pose_at`); differential fuzz invariants tested under `--features fuzz-tests`.

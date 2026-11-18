# `fsim-plant-host`

Runs the deterministic factory simulation plant (`SimWorld`), manages virtual device sessions (`fsim-net`) for every placed cell and worker iPad, streams observations into `fsim-gateway`, and provides the sim control channel (skip-ahead, runway summary cards, fork, rewind, and god view).

- **State & Caching**: Stateless functions over caller-owned `SimWorld`
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols`, `fsim-net`, `fsim-gateway`, `fsim-geom`, `fsim-layout-graph`, `fsim-motion`, `fsim-navgrid`, `fsim-path-walk`, and all 12 team crates | External: `serde`, `serde_json`
- **Data Flow**: Consumes `SimWorld`, `PlantCommand`s, and target `SimTime` -> Produces updated `SimWorld`, `SkipSummaryCard`, `CellStatusReport`s, and `SceneNode`s

## Design Intent & Assumptions
- **Encapsulation Boundary**: Composes all 12 team vertical slices, `fsim-net`, and `fsim-gateway` into a single deterministic plant host. HTTP/WebSocket serving and SQLite account management belong in `fsim-server`.
- **Core Invariants**:
  - **Skip-Ahead Equivalence**: Jumping event-by-event via `advance_to` or `skip_ahead` yields bit-identical state, cash, and order outcomes to fine-grained stepping (`reference::reference_verify_skip_equivalence`).
  - **Device Session Parity**: Every placed cell and hired worker registers a virtual device on `fsim-net` (`plant/<p>/cell/<c>/controller` and `plant/<p>/person/<w>/ipad`) and logs Protobuf envelopes into `fsim-gateway`.
  - **Repeat-Until-Quiet Passes**: At each event timestamp, `step_to` runs first-come-first-served passes across worker task completions, cell cycle completions, backward-chaining order unblocking, logistics scheduling, and iPad task dispatch until no further transitions occur.

## Algorithms Implemented (Do Not Duplicate)
- **Event-Driven Multi-Pass Plant Step** (`SimWorld::step_to`, `SimWorld::advance_to`, `SimWorld::skip_ahead`): Advances to the earliest `next_event_at` across all modules and runs repeat-until-quiet passes.
- **"Why Isn't This Running?" Status Generator** (`SimWorld::cell_status_reports`): Produces plain-language status and clickable blocker links from estimated state for every cell.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `InstalledCell`, `PipelineItem`, `AlertItem`, `SkipSummaryCard`, `SimWorld`, `full_starter_catalog()`, `status_color_hex(CellRunState)`, `catalog_preview_model(&CellCatalogEntry) -> Option<CellModel>` (default-config `Planned` model for planning ghosts). `square_layout(catalog, x, y, rot)` (handoff squares `SQUARE_SETBACK_MM` in front of the operator face, rotated about the footprint center like the viewer), `material_flows(squares)` (output -> SKU-compatible input of another cell: the upstream/downstream dependency graph routed by the transport graph). Worker trips are planned with `fsim-path-walk` on a walk grid cached per layout version in the non-serialized `SimWorld::nav_cache`, so workers walk around machine footprints along the same paths the viewer draws. `SimWorld::visualize_plant` attaches team `CellModel`s (machining today) to cell `SceneNode`s using each cell's add-ons and current run state.
- **[`src/reference.rs`](src/reference.rs)**: Skip-ahead vs. fine-stepping equivalence oracle (`reference_verify_skip_equivalence`).

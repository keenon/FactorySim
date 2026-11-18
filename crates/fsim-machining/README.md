# `fsim-machining`

CNC machining vertical slice: 3-axis VMC, 5-axis trunnion, turning center, and compact mill; conditional models for cycle time, tool wear, in-process probing, and scrap.

- **State & Caching**: Stateless (`MachiningCellConfig` is caller-owned)
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols` | External: `serde`
- **Data Flow**: Consumes `CellCatalogEntry`, `MachiningCellConfig`, `CellRunState`, tool wear and part specs -> Produces cycle-time and scrap distributions, starter catalog options, and procedural `CellModel` 3D models

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns CNC machining cell catalogs, add-on bundles (`probing`, `bar-feeder`, `pallet-changer`, `chip-conveyor`), cycle/scrap conditional distributions, and *what each machine looks like* (`model::machining_cell_model`). The viewer only maps primitives to meshes.
- **Core Invariants**:
  - **Probing Variance Reduction**: Enabling `has_in_process_probing` increases cycle time by 8% while reducing out-of-tolerance scrap probability by 67%.
  - **Model Fits Spec Envelope**: For every catalog entry, every add-on subset and every run state, all model parts lie within `[0,width] x [0,depth] x [0,height]` (`CellModel::fits_envelope`), and the named nodes `base, enclosure, door, window, spindle, table, pendant, screen, status_light` exist.
  - **State Bindings**: The sliding `door` is open unless `Running`/`Installing`/`Planned`; exactly one `status_light` segment is `Emissive` (green running, amber idle/setup/blocked, red down; none while planned/installing).
- **Conditioning Variables**: Part material/complexity, active add-on bundle (`MachiningCellConfig`), and tool wear percentage (`0..100`).

## Algorithms Implemented (Do Not Duplicate)
- **Configuration-Conditioned Cycle Time & Scrap Model** (`reference::reference_cycle_time_ms`, `reference::reference_scrap_permille`): Evaluates cycle duration and defect permille rate.
- **Procedural Machine Models** (`model::machining_cell_model`, `model::MachineKind`): Integer-mm parametric layout of base, enclosure, sliding door + window, work zone (VMC table/spindle, 5-axis trunnion + rotary table, lathe headstock/chuck/turret), spindle column, side-bay pendant, coolant tank or pallet pool, chip conveyor, bar feeder, and status tower, all scaled from catalog `width/depth/height`. Local frame: origin at footprint min corner, front at y = 0, z up.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `MachiningCellConfig`, `starter_machining_catalog()`.
- **[`src/model.rs`](src/model.rs)**: `MachineKind`, `machining_cell_model(&CellCatalogEntry, &MachiningCellConfig, CellRunState) -> CellModel`.
- **[`src/reference.rs`](src/reference.rs)**: Reference cycle time and scrap permille oracle (`reference_cycle_time_ms`, `reference_scrap_permille`).

# `fsim-machining`

CNC machining vertical slice: 3-axis VMC, 5-axis trunnion, turning center, and compact mill; conditional models for cycle time, tool wear, in-process probing, and scrap.

- **State & Caching**: Stateless (`MachiningCellConfig` is caller-owned)
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols` | External: `serde`
- **Data Flow**: Consumes `MachiningCellConfig`, tool wear, and part specs -> Produces cycle-time and scrap distributions and starter catalog options

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns CNC machining cell catalogs, add-on bundles (`probing`, `bar-feeder`, `pallet-changer`, `chip-conveyor`), and cycle/scrap conditional distributions.
- **Core Invariants**:
  - **Probing Variance Reduction**: Enabling `has_in_process_probing` increases cycle time by 8% while reducing out-of-tolerance scrap probability by 67%.
- **Conditioning Variables**: Part material/complexity, active add-on bundle (`MachiningCellConfig`), and tool wear percentage (`0..100`).

## Algorithms Implemented (Do Not Duplicate)
- **Configuration-Conditioned Cycle Time & Scrap Model** (`reference::reference_cycle_time_ms`, `reference::reference_scrap_permille`): Evaluates cycle duration and defect permille rate.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `MachiningCellConfig`, `starter_machining_catalog()`.
- **[`src/reference.rs`](src/reference.rs)**: Reference cycle time and scrap permille oracle (`reference_cycle_time_ms`, `reference_scrap_permille`).

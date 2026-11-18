# `fsim-materials-logistics`

Materials & Logistics vertical slice: Receiving Bench (lot traceability), Cantilever Rack, Manual/Automatic Bandsaws, Shipping Staging Lanes, and freeform move duration models.

- **State & Caching**: 100% Stateless
- **Dependencies**: Workspace: `fsim-core`, `fsim-layout-graph`, `fsim-protocols` | External: `serde`
- **Data Flow**: Consumes `TransportGraph` distances and saw catalog IDs -> Produces move durations, sawing durations, and starter catalog entries

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns receiving, raw/consumable storage racks, stock-prep saws (`raw-bar-*` -> `blank-*`), shipping staging lanes (`packed-shipment` -> carrier dispatch), and freeform move durations.
- **Core Invariants**:
  - **Layout-Driven Move Time**: Move duration scales strictly with the `TransportGraph` path distance between the origin and destination handoff squares (`reference_move_duration_ms`).
- **Conditioning Variables**: Transport graph route distance (`distance_mm`), mover speed (`speed_mm_per_s`), and saw type (`catalog_id`).

## Algorithms Implemented (Do Not Duplicate)
- **Saw Cut & Freeform Move Duration Models** (`reference::reference_saw_cut_ms`, `reference::reference_move_duration_ms`): Evaluates sawing cycle time and inter-square move time.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `starter_materials_logistics_catalog()`.
- **[`src/reference.rs`](src/reference.rs)**: Reference saw cut and move duration oracle (`reference_saw_cut_ms`, `reference_move_duration_ms`).

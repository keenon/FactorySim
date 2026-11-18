# `fsim-finishing-packing`

Finishing, deburring, and packing vertical slice: Manual Pack Station, Vibratory Tumbler & Laser Marker, Optical Bagger, and Case Taper/Wrapper.

- **State & Caching**: 100% Stateless
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols` | External: `serde`
- **Data Flow**: Consumes `part_in` and `consumable_in` handling units -> Produces `packed-shipment` handling units and starter catalog entries

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns deburr/marking and multi-input packing stations. Every packing station declares two named input squares (`part_in` for the manufactured part and `consumable_in` for the `carton-box` packaging inventory) so running out of packaging blocks packing just like running out of raw stock.
- **Core Invariants**:
  - **Dual-Input Gate**: A packing order cannot enter `IN_PROCESS` until both `part_in` and `consumable_in` squares are occupied.
- **Conditioning Variables**: Station catalog ID, active tape-dispenser add-on, and unit count per carton.

## Algorithms Implemented (Do Not Duplicate)
- **Station Packing Time Oracle** (`reference::reference_packing_time_ms`): Computes baseline packing cycle duration in milliseconds.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `starter_finishing_packing_catalog()`.
- **[`src/reference.rs`](src/reference.rs)**: Reference packing cycle oracle (`reference_packing_time_ms`).

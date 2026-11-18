# `fsim-molding`

Injection molding vertical slice: 55T all-electric press, 180T hydraulic press, and 35T vertical insert press; conditional models for wall-thickness cooling time and shot-by-shot process window discovery.

- **State & Caching**: 100% Stateless
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols` | External: `serde`
- **Data Flow**: Consumes wall thickness, resin type, and tuning shot count -> Produces cycle time, defect permille, and starter catalog options

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns injection presses, multi-input squares (`part_in` for resin + `mold_in` or `insert_in`), and hidden process window tuning dynamics.
- **Core Invariants**:
  - **Quadratic Cooling Law**: Cooling time scales quadratically with wall thickness (`wall_thickness_tenth_mm^2`).
  - **Monotonic Process Window Convergence**: Defect permille decreases monotonically as tuning shots discover the press/mold/resin process window.
- **Conditioning Variables**: Part wall thickness, resin moisture state, and cumulative tuning shots on the current mold setup.

## Algorithms Implemented (Do Not Duplicate)
- **Wall-Thickness Cooling & Process Window Model** (`reference::reference_molding_cycle_ms`, `reference::reference_window_defect_permille`): Computes shot cycle duration and defect probability.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `starter_molding_catalog()`.
- **[`src/reference.rs`](src/reference.rs)**: Reference cooling cycle and process-window defect oracle (`reference_molding_cycle_ms`, `reference_window_defect_permille`).

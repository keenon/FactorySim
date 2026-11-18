# `fsim-toolroom`

Toolroom vertical slice: CNC Sinker EDM, Wire EDM, and Moldmaker Spotting Bench for mold build and maintenance.

- **State & Caching**: 100% Stateless
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols` | External: `serde`
- **Data Flow**: Consumes cavity complexity tier and steel type -> Produces mold build durations and starter catalog entries

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns mold fabrication and repair cells (`tool-sinker-edm`, `tool-wire-edm`, `tool-spotting-bench`). Injection presses and shot-by-shot process windows belong in `fsim-molding`.
- **Core Invariants**:
  - **Monotonic Complexity Scaling**: Mold fabrication hours increase strictly with cavity complexity tier (`1..=5`).
- **Conditioning Variables**: Cavity complexity tier (`1..=5`), steel hardness, and active electrode-changer add-on.

## Algorithms Implemented (Do Not Duplicate)
- **Lookup-Table Mold Build Duration** (`reference::reference_mold_build_ms`): Computes baseline mold fabrication duration in milliseconds.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `starter_toolroom_catalog()`.
- **[`src/reference.rs`](src/reference.rs)**: Reference mold build time oracle (`reference_mold_build_ms`).

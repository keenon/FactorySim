# `fsim-quality`

Quality inspection vertical slice: Manual Metrology Bench, Optical Vision Measuring System, and Bridge CMM.

- **State & Caching**: 100% Stateless
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols` | External: `serde`
- **Data Flow**: Consumes station catalog ID and shop climate status -> Produces `(true_positive_permille, false_positive_permille)` detection odds and starter catalog entries

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns inspection equipment catalogs, climate-control prerequisites (`qual-bridge-cmm` requires `needs_climate_room: true`), and true/false positive detection odds.
- **Core Invariants**:
  - **Climate Sensitivity**: Bridge CMM achieves 99.2% defect detection inside a temperature-controlled room, but degrades if shop climate is out of spec.
- **Conditioning Variables**: Inspection station type (`catalog_id`) and climate enclosure status (`climate_in_spec`).

## Algorithms Implemented (Do Not Duplicate)
- **Conditioned Detection & False-Rejection Model** (`reference::reference_inspection_odds_permille`): Computes true-positive catch rate and false-positive rejection rate in permille.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `starter_quality_catalog()`.
- **[`src/reference.rs`](src/reference.rs)**: Reference inspection odds oracle (`reference_inspection_odds_permille`).

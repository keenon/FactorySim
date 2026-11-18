# `fsim-heat-treat`

Heat treatment vertical slice covering the 4 canonical options: outside vendor staging, bought box furnace, bought vacuum furnace, and home-built furnace.

- **State & Caching**: 100% Stateless
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols` | External: `serde`
- **Data Flow**: Consumes furnace catalog ID and cumulative batch tuning count -> Produces warp/scrap permille and starter catalog entries

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns heat-treatment cell options, financing eligibility (`ht-homebuilt-furnace` is non-financeable), hazmat permit requirements, and batch yield distributions.
- **Core Invariants**:
  - **Home-Built Tuning Curve**: `ht-homebuilt-furnace` costs 1/3 of a commercial box furnace, is non-financeable (`financeable: false`), and starts at a higher warp/scrap rate that decays as recipe batches are run.
- **Conditioning Variables**: Furnace type (`catalog_id`), alloy family, and cumulative tuned batches (`batches_run`).

## Algorithms Implemented (Do Not Duplicate)
- **Furnace Scrap & Recipe Tuning Model** (`reference::reference_heat_treat_scrap_permille`): Evaluates batch warp/cracking rate in permille.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `starter_heat_treat_catalog()`.
- **[`src/reference.rs`](src/reference.rs)**: Reference batch scrap oracle (`reference_heat_treat_scrap_permille`).

# `fsim-core`

Deterministic simulation clock (`SimTime`), tenth-cent integer currency (`Money`), per-module seeded RNG (`StreamRng`), double-entry accounting (`DoubleEntryLedger`), size-1 named handoff squares (`BufferView`), and the composed `StepModule` contract.

- **State & Caching**: Stateless (`DoubleEntryLedger` and `BufferView` are caller-owned value types)
- **Dependencies**: Workspace: `fsim-geom`, `fsim-layout-graph`, `fsim-motion`, `fsim-protocols` | External: `rand`, `rand_chacha`, `serde`, `sha2`, `thiserror`
- **Data Flow**: Consumes `BufferView`, `Envelope` inbox, `SimTime`, `StreamRng` -> Produces `StepOut<S>` with atomic `BufferOp`s and `SharedFact`s

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns shared simulation primitives, buffer ownership, double-entry accounting, and module pass ordering. Team-specific physics and conditional distributions belong in the 12 team crates.
- **Core Invariants**:
  - **Double-Entry Balance**: Every `DoubleEntryLedger::post` credits one account and debits another by the exact same `Money(i64)` tenth-cent amount; `sum(account_balances) == 0` across all inputs.
  - **Physical Conservation**: Every `HandlingUnit` lives in at most one `HandoffSquare`, one `shared_racks` slot, or one module's internal state.
  - **Early-Call Invariant**: Calling `StepModule::step` at an intermediate time with no new inputs never alters future outputs or resamples stored completion times (`check_early_call_invariant`).
- **Execution & Granularity Assumptions**: Ordered `BTreeMap` collections only; one SHA-256 derived `ChaCha8Rng` stream per `(game_seed, module_id, entity_id, event_count)`.

## Algorithms Implemented (Do Not Duplicate)
- **Seeded First-Come-First-Served Pass Ordering** (`reference::reference_module_order`): Evaluates priority modules first, then orders remaining modules by `SHA-256(seed || timestamp || module_id)` under `OrderPolicy::Seeded`.
- **Early-Call Conformance Harness** (`check_early_call_invariant`): Verifies time-subdivision invariance for any `StepModule`.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `SimTime`, `Money`, `StreamRng`, `HandlingUnit`, `HandoffSquare`, `BufferView`, `BufferOp`, `LedgerRecord`, `DoubleEntryLedger`, `OrderPolicy`, `StepOut`, `StepModule`, `CellCatalogEntry`, `CellAddonSpec`, `SceneNode`, `CellStatusReport`, `check_early_call_invariant()`.
- **[`src/reference.rs`](src/reference.rs)**: Ledger balance oracle (`reference_ledger_is_balanced`), buffer conservation counter (`reference_total_units_in_buffers`), and pass-order oracle (`reference_module_order`).

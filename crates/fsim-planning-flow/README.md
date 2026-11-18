# `fsim-planning-flow`

Planning & Flow vertical slice: recursive backward-chaining pull order construction, bottom-up dependency unblocking, bottleneck detection, and Kingman shadow-price estimation.

- **State & Caching**: 100% Stateless
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols`, `fsim-sales` | External: `serde`
- **Data Flow**: Consumes `CustomerOrder` and `BTreeMap<String, InternalOrder>` -> Produces backward-chaining `InternalOrder` trees, unblocking transitions, and `BottleneckReport`

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns the backward-chaining order tree (`Shipping -> Packing -> [Inspection] -> CNC -> [Saw] -> Raw/Precut Stock + Carton Stock`) and bottleneck detection. Physical handoff execution belongs in `fsim-plant-host` and `fsim-people`.
- **Core Invariants**:
  - **Leaf-Only Initial Unblocking**: When a customer order arrives, every generated `InternalOrder` starts in `BlockedUpstream` except the leaf stock orders (`cust-*-stock-*`), which start in `ReadyForLogistics`.
  - **Monotonic DAG Unblocking**: A parent order transitions from `BlockedUpstream` to `ReadyForLogistics` if and only if every order ID in its `upstream_order_ids` has reached `Completed`.

## Algorithms Implemented (Do Not Duplicate)
- **Backward-Chaining Order DAG Builder** (`create_backward_order_chain`, `reference::reference_build_backward_order_chain`): Generates the pull order hierarchy from Shipping back to raw stock and packaging consumables.
- **DAG Dependency Unblocker** (`refresh_order_unblocking`, `reference::reference_refresh_order_unblocking`): Propagates `Completed` states up the order tree.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `BottleneckReport`, `create_backward_order_chain()`, `refresh_order_unblocking()`, `detect_bottleneck()`.
- **[`src/reference.rs`](src/reference.rs)**: Reference backward-chaining DAG builder and unblocker (`reference_build_backward_order_chain`, `reference_refresh_order_unblocking`).

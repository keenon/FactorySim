# `fsim-net`

Authenticated device sessions, hierarchical device directory (`plant/1/cell/2/mill-controller`, `plant/1/person/1/ipad`), and deterministic in-memory virtual network with fault injection.

- **State & Caching**: Stateless (`VirtualNetworkState` is a caller-owned value type)
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols` | External: `serde`, `thiserror`
- **Data Flow**: Consumes `DeviceIdentity`, `NetworkImpairment`, `Envelope` -> Produces ordered `DeviceSessionFrame` deliveries

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns the transport abstraction between reporting devices (virtual or physical) and `fsim-gateway`. Every machine controller and worker iPad opens its own authenticated session here from Wave 1.
- **Core Invariants**:
  - **Mandatory Device Authentication**: Unregistered device IDs cannot inject frames into the gateway.
  - **Deterministic Impairment & Delivery**: Latency, jitter, drops, and clock skew are seeded from `(game_seed, "fsim-net", device_id, event_counter)` and delivered in strict `(deliver_at, device_id, sequence_number)` order.
- **Execution & Granularity Assumptions**: In-memory transport runs in-process at zero socket overhead during sim games and tests.

## Algorithms Implemented (Do Not Duplicate)
- **Seeded Impairment & Ordered Frame Drain** (`VirtualNetworkState::send_frame`, `reference::reference_drain_due_frames`): Applies per-device latency, jitter, packet drop rate (ppm), and clock skew deterministically.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `DeviceIdentity`, `NetworkImpairment`, `DeviceSessionFrame`, `FrameDirection`, `VirtualNetworkState`.
- **[`src/reference.rs`](src/reference.rs)**: Ideal ordered frame delivery oracle (`reference_drain_due_frames`).

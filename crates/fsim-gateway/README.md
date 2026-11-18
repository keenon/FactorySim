# `fsim-gateway`

Ingests authenticated device frames from `fsim-net`, maintains a tamper-evident SHA-256 hash-chained log of raw wire envelopes, quarantines malformed bytes, and compresses closed `.log.zst` segments.

- **State & Caching**: Stateless (`GatewayState` is caller-owned)
- **Dependencies**: Workspace: `fsim-core`, `fsim-net`, `fsim-protocols` | External: `serde`, `sha2`, `thiserror`, `zstd`
- **Data Flow**: Consumes `DeviceSessionFrame`s / raw wire bytes -> Produces `ChainedLogEntry` segments and decoded `Envelope` streams for the controller estimator

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns the boundary between raw device sessions and the controller's estimator/storage. Stores raw Protobuf bytes as received so a payload decoder bug never corrupts the historical record.
- **Core Invariants**:
  - **Tamper-Evident Hash Chain**: Entry `k` hashes `(log_seq_k || prev_hash_{k-1} || raw_envelope_bytes_k)` with SHA-256.
  - **Quarantine on Malformed Bytes**: Invalid wire frames are isolated into `quarantined_payloads` without panicking or breaking the hash chain.
- **Execution & Granularity Assumptions**: Append-only in memory during active stepping; closed segments are compressed with `zstd` level 3.

## Algorithms Implemented (Do Not Duplicate)
- **SHA-256 Chained Log & Quarantine** (`GatewayState::append_raw_bytes`, `reference::reference_verify_hash_chain`): Appends and verifies tamper-evident log chains.
- **Zstd Segment Codec** (`GatewayState::compress_segment_zst`, `GatewayState::decompress_segment_zst`): Compresses and verifies `.log.zst` segments.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `ChainedLogEntry`, `GatewayState`.
- **[`src/reference.rs`](src/reference.rs)**: Full-chain SHA-256 verifier (`reference_verify_hash_chain`, `compute_entry_hash`).

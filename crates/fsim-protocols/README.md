# `fsim-protocols`

Canonical wire protocol envelopes, Protobuf-3 wire codec, shared cross-team facts, backward-chaining internal order schemas, and schema registry.

- **State & Caching**: 100% Stateless
- **Dependencies**: Workspace: none | External: `bytes`, `serde`, `serde_json`, `thiserror`
- **Data Flow**: Consumes raw wire bytes / typed domain facts -> Produces versioned `Envelope`, `InternalOrder`, `SharedFact`, and `PlantCommand` messages

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns the single definition of every cross-team protocol message, the `Envelope` format (`proto/envelope.proto`), and the backward-chaining `InternalOrder` contract (`proto/shared.proto`). Contains zero simulation physics or team-specific dynamics.
- **Core Invariants**:
  - **Decode-Forever Wire Compatibility**: Unknown Protobuf field numbers are skipped without error (forward compatibility) and missing fields default cleanly (backward compatibility), allowing floor controllers and stored `.log.zst` segments to upgrade independently.
  - **No Decoder Panics**: Every decoder returns `Result<T, WireError>` and never panics on truncated or malformed byte slices.
- **Execution & Granularity Assumptions**: Zero-allocation slice scanning where possible; `Envelope` header inspection does not decode the inner `payload` bytes.

## Algorithms Implemented (Do Not Duplicate)
- **Protobuf-3 Wire Codec** (`wire::encode_varint`, `wire::decode_varint`, `wire::skip_field`, `wire::ProtoMessage`): Deterministic Protobuf-3 binary encoding and decoding with unknown-field skipping.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `TeamTag`, `Envelope`, `HandlingUnitSpec`, `InternalOrderStatus`, `RequiredInputEntry`, `InternalOrder`, `SharedFact`, `CellRunState`, `BlockerReason`, `OperatorTaskKind`, `PlantCommand`, `standard_schema_registry()`.
- **[`src/wire.rs`](src/wire.rs)**: `WireError`, `WireType`, `ProtoMessage`.
- **[`src/reference.rs`](src/reference.rs)**: Simple unoptimized round-trip oracle (`reference_envelope_roundtrip`, `reference_internal_order_roundtrip`); differential fuzz invariants tested under `--features fuzz-tests`.

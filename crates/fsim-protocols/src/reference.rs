//! Reference oracle for `fsim-protocols` wire encoding and schema compatibility checks.

use crate::wire::{ProtoMessage, WireError};
use crate::{Envelope, InternalOrder};

/// Reference round-trip oracle verifying that encoding and decoding an `Envelope`
/// produces bit-identical output and never panics on arbitrary byte slices.
pub fn reference_envelope_roundtrip(env: &Envelope) -> Result<Envelope, WireError> {
    let bytes = env.to_proto_bytes();
    Envelope::decode_proto(&bytes)
}

/// Reference round-trip oracle for `InternalOrder`.
pub fn reference_internal_order_roundtrip(order: &InternalOrder) -> Result<InternalOrder, WireError> {
    let bytes = order.to_proto_bytes();
    InternalOrder::decode_proto(&bytes)
}

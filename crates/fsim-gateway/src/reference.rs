//! Reference hash-chain verifier and zstd segment round-trip oracle for `fsim-gateway`.

use crate::{ChainedLogEntry, GatewayState};
use sha2::{Digest, Sha256};

/// Verifies that every entry in the gateway's tamper-evident log has a valid SHA-256 hash chain.
pub fn reference_verify_hash_chain(entries: &[ChainedLogEntry]) -> bool {
    let mut prev_hash = [0u8; 32];
    for (idx, entry) in entries.iter().enumerate() {
        if entry.log_seq != (idx as u64) + 1 {
            return false;
        }
        if entry.prev_hash != prev_hash {
            return false;
        }
        let expected = compute_entry_hash(entry.log_seq, &entry.prev_hash, &entry.raw_envelope_bytes);
        if entry.entry_hash != expected {
            return false;
        }
        prev_hash = entry.entry_hash;
    }
    true
}

pub fn compute_entry_hash(log_seq: u64, prev_hash: &[u8; 32], raw_envelope_bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(log_seq.to_le_bytes());
    hasher.update(prev_hash);
    hasher.update(raw_envelope_bytes);
    hasher.finalize().into()
}

pub fn reference_gateway_chain_valid(gw: &GatewayState) -> bool {
    reference_verify_hash_chain(&gw.log_entries)
}

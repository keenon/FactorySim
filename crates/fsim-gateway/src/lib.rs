//! `fsim-gateway`: Accepts authenticated device sessions from `fsim-net`, appends raw wire
//! envelopes into a tamper-evident SHA-256 hash-chained log, compresses closed `.log.zst`
//! segments, and routes observations to the controller's state estimator.

pub mod reference;

use fsim_net::{DeviceSessionFrame, FrameDirection};
use fsim_protocols::wire::ProtoMessage;
use fsim_protocols::Envelope;
use serde::{Deserialize, Serialize};

/// A single tamper-evident record in the gateway's append-only message log.
/// Stores raw Protobuf bytes as received so a decoder bug can never corrupt the historical record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainedLogEntry {
    pub log_seq: u64,
    pub prev_hash: [u8; 32],
    pub entry_hash: [u8; 32],
    pub raw_envelope_bytes: Vec<u8>,
}

impl ChainedLogEntry {
    pub fn decode_envelope(&self) -> Result<Envelope, fsim_protocols::wire::WireError> {
        Envelope::decode_proto(&self.raw_envelope_bytes)
    }
}

/// Gateway state owning the hash-chained message log and quarantine list for malformed frames.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayState {
    pub log_entries: Vec<ChainedLogEntry>,
    pub head_hash: [u8; 32],
    pub quarantined_payloads: Vec<(String, Vec<u8>)>,
}

impl GatewayState {
    pub fn append_envelope(&mut self, env: &Envelope) -> u64 {
        let raw_bytes = env.to_proto_bytes();
        self.append_raw_bytes(&env.source_device, raw_bytes)
            .unwrap_or(self.log_entries.len() as u64)
    }

    /// Validates that `raw_bytes` can be parsed as an `Envelope` header; if malformed, quarantines it
    /// without panicking. Otherwise appends to the SHA-256 hash chain.
    pub fn append_raw_bytes(&mut self, source_device: &str, raw_bytes: Vec<u8>) -> Result<u64, String> {
        if Envelope::decode_proto(&raw_bytes).is_err() {
            self.quarantined_payloads
                .push((source_device.to_string(), raw_bytes));
            return Err("malformed envelope quarantined".to_string());
        }
        let log_seq = (self.log_entries.len() as u64) + 1;
        let prev_hash = self.head_hash;
        let entry_hash = reference::compute_entry_hash(log_seq, &prev_hash, &raw_bytes);
        self.head_hash = entry_hash;
        self.log_entries.push(ChainedLogEntry {
            log_seq,
            prev_hash,
            entry_hash,
            raw_envelope_bytes: raw_bytes,
        });
        Ok(log_seq)
    }

    /// Ingests due frames from `fsim-net` and returns decoded envelopes for the controller estimator.
    pub fn ingest_frames(&mut self, frames: Vec<DeviceSessionFrame>) -> Vec<Envelope> {
        let mut out = Vec::with_capacity(frames.len());
        for f in frames {
            if f.direction == FrameDirection::DeviceToGateway {
                self.append_envelope(&f.envelope);
                out.push(f.envelope);
            }
        }
        out
    }

    pub fn verify_integrity(&self) -> bool {
        reference::reference_gateway_chain_valid(self)
    }

    /// Encodes a slice of `ChainedLogEntry` records into a zstd-compressed binary segment.
    pub fn compress_segment_zst(entries: &[ChainedLogEntry]) -> Result<Vec<u8>, String> {
        let json = serde_json::to_vec(entries).map_err(|e| e.to_string())?;
        zstd::encode_all(&json[..], 3).map_err(|e| e.to_string())
    }

    /// Decodes a zstd-compressed binary segment back into `ChainedLogEntry` records and verifies its hash chain.
    pub fn decompress_segment_zst(compressed: &[u8]) -> Result<Vec<ChainedLogEntry>, String> {
        let raw = zstd::decode_all(compressed).map_err(|e| e.to_string())?;
        let entries: Vec<ChainedLogEntry> =
            serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
        if !reference::reference_verify_hash_chain(&entries) {
            return Err("log segment hash chain verification failed".to_string());
        }
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsim_protocols::TeamTag;

    #[test]
    fn test_gateway_hash_chain_and_zstd_segment_roundtrip() {
        let mut gw = GatewayState::default();
        for seq in 1..=5 {
            let env = Envelope {
                team_tag: TeamTag::Machining,
                schema_id: "fsim.machining.v1.Obs".to_string(),
                schema_version: 1,
                source_device: "plant/1/cell/1/mill-controller".to_string(),
                entity_id: "cell-1".to_string(),
                plant_timestamp_ms: (seq * 1000) as i64,
                receive_timestamp_ms: (seq * 1000) as i64,
                sequence_number: seq,
                correlation_id: format!("ord-{seq}"),
                payload: vec![seq as u8],
            };
            gw.append_envelope(&env);
        }
        assert!(gw.verify_integrity());

        let zst = GatewayState::compress_segment_zst(&gw.log_entries).unwrap();
        let restored = GatewayState::decompress_segment_zst(&zst).unwrap();
        assert_eq!(gw.log_entries, restored);

        // Quarantining malformed bytes does not corrupt chain
        assert!(gw.append_raw_bytes("bad-dev", vec![0x07]).is_err());
        assert_eq!(gw.quarantined_payloads.len(), 1);
        assert!(gw.verify_integrity());
    }
}

//! `fsim-net`: Authenticated device sessions, hierarchical device directory
//! (`plant/1/cell/2/mill-controller`, `plant/1/person/1/ipad`), and deterministic
//! in-memory virtual network with optional latency/jitter/dropout impairment injection.

pub mod reference;

use fsim_core::{SimTime, StreamRng};
use fsim_protocols::Envelope;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Identity and credentials of a reporting device (machine controller, sensor hub, or worker iPad).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub device_id: String,
    pub plant_id: String,
    pub entity_id: String,
    pub device_kind: String, // "machine-controller", "worker-ipad", "sensor-hub"
    pub cert_fingerprint: String,
}

/// Network impairment configuration per device or link (zero by default; configurable for fault tests).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkImpairment {
    pub base_latency_ms: i64,
    pub max_jitter_ms: i64,
    pub drop_rate_ppm: u32, // parts per million (0..1_000_000)
    pub clock_skew_ms: i64,
    pub disconnected: bool,
}

/// A framed message in flight across a virtual or physical device session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceSessionFrame {
    pub device_id: String,
    pub direction: FrameDirection,
    pub deliver_at: SimTime,
    pub envelope: Envelope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FrameDirection {
    DeviceToGateway,
    GatewayToDevice,
}

/// Deterministic in-memory virtual network state connecting `fsim-plant-host` devices to `fsim-gateway`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualNetworkState {
    pub registered_devices: BTreeMap<String, DeviceIdentity>,
    pub impairments: BTreeMap<String, NetworkImpairment>,
    pub next_seq_by_device: BTreeMap<String, u64>,
    pub in_flight: Vec<DeviceSessionFrame>,
    pub event_counter: u64,
}

impl VirtualNetworkState {
    pub fn register_device(&mut self, identity: DeviceIdentity) {
        self.next_seq_by_device
            .entry(identity.device_id.clone())
            .or_insert(1);
        self.registered_devices
            .insert(identity.device_id.clone(), identity);
    }

    pub fn set_impairment(&mut self, device_id: &str, imp: NetworkImpairment) {
        self.impairments.insert(device_id.to_string(), imp);
    }

    /// Sends an envelope over an authenticated device session, applying deterministic impairments if configured.
    pub fn send_frame(
        &mut self,
        game_seed: u64,
        device_id: &str,
        direction: FrameDirection,
        mut envelope: Envelope,
        now: SimTime,
    ) -> Result<bool, String> {
        if !self.registered_devices.contains_key(device_id) {
            return Err(format!("unauthenticated device session: {device_id}"));
        }
        let imp = self
            .impairments
            .get(device_id)
            .copied()
            .unwrap_or_default();
        if imp.disconnected {
            return Ok(false);
        }

        self.event_counter += 1;
        let mut rng = StreamRng::new(game_seed, "fsim-net", device_id, self.event_counter);
        if imp.drop_rate_ppm > 0 {
            let draw = rng.gen_range_i64(0, 999_999) as u32;
            if draw < imp.drop_rate_ppm {
                return Ok(false);
            }
        }

        let seq = self.next_seq_by_device.entry(device_id.to_string()).or_insert(1);
        envelope.sequence_number = *seq;
        *seq += 1;
        envelope.source_device = device_id.to_string();
        envelope.plant_timestamp_ms = now.0 + imp.clock_skew_ms;

        let jitter = if imp.max_jitter_ms > 0 {
            rng.gen_range_i64(0, imp.max_jitter_ms)
        } else {
            0
        };
        let deliver_at = now.add_ms(imp.base_latency_ms + jitter);
        envelope.receive_timestamp_ms = deliver_at.0;

        self.in_flight.push(DeviceSessionFrame {
            device_id: device_id.to_string(),
            direction,
            deliver_at,
            envelope,
        });
        Ok(true)
    }

    /// Drains all frames whose delivery timestamp is `<= now`.
    pub fn drain_due_frames(&mut self, now: SimTime) -> Vec<DeviceSessionFrame> {
        reference::reference_drain_due_frames(self, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsim_protocols::TeamTag;

    #[test]
    fn test_authenticated_device_session_and_deterministic_impairments() {
        let mut net = VirtualNetworkState::default();
        net.register_device(DeviceIdentity {
            device_id: "plant/1/cell/1/mill-controller".to_string(),
            plant_id: "plant-1".to_string(),
            entity_id: "cell-1".to_string(),
            device_kind: "machine-controller".to_string(),
            cert_fingerprint: "sha256:abcd".to_string(),
        });

        let env = Envelope {
            team_tag: TeamTag::Machining,
            schema_id: "fsim.machining.v1.Cycle".to_string(),
            schema_version: 1,
            source_device: String::new(),
            entity_id: "cell-1".to_string(),
            plant_timestamp_ms: 0,
            receive_timestamp_ms: 0,
            sequence_number: 0,
            correlation_id: "ord-1".to_string(),
            payload: vec![42],
        };

        // Unregistered device is rejected
        assert!(net
            .send_frame(
                99,
                "plant/1/cell/999/rogue",
                FrameDirection::DeviceToGateway,
                env.clone(),
                SimTime::from_ms(1_000)
            )
            .is_err());

        // Registered device succeeds and assigns sequence number 1
        assert!(net
            .send_frame(
                99,
                "plant/1/cell/1/mill-controller",
                FrameDirection::DeviceToGateway,
                env,
                SimTime::from_ms(1_000)
            )
            .unwrap());

        let delivered = net.drain_due_frames(SimTime::from_ms(1_000));
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].envelope.sequence_number, 1);
    }
}

//! Reference ideal zero-impairment network oracle for `fsim-net`.

use crate::{DeviceSessionFrame, VirtualNetworkState};
use fsim_core::SimTime;

/// Reference ideal delivery: delivers all queued frames with arrival time `<= now` in exact `(delivery_at, sequence)` order.
pub fn reference_drain_due_frames(
    net: &mut VirtualNetworkState,
    now: SimTime,
) -> Vec<DeviceSessionFrame> {
    let mut ready = Vec::new();
    let mut pending = Vec::new();
    for frame in net.in_flight.drain(..) {
        if frame.deliver_at <= now {
            ready.push(frame);
        } else {
            pending.push(frame);
        }
    }
    net.in_flight = pending;
    ready.sort_by(|a, b| {
        a.deliver_at
            .cmp(&b.deliver_at)
            .then_with(|| a.device_id.cmp(&b.device_id))
            .then_with(|| a.envelope.sequence_number.cmp(&b.envelope.sequence_number))
    });
    ready
}

//! Reference oracle for `fsim-plant-host` skip-ahead equivalence and conservation checks.

use crate::SimWorld;
use fsim_core::SimTime;

/// Reference skip-ahead equivalence test: verifies that stepping `world` in fine increments
/// up to `target_time` produces the exact same state as jumping event-by-event via `advance_to`.
pub fn reference_verify_skip_equivalence(
    initial: &SimWorld,
    target_time: SimTime,
    fine_step_ms: i64,
) -> bool {
    let mut world_skip = initial.clone();
    world_skip.advance_to(target_time);

    let mut world_fine = initial.clone();
    while world_fine.now < target_time {
        let cap = world_fine
            .now
            .add_ms(fine_step_ms.max(1_000))
            .min(target_time);
        let next_t = world_fine
            .next_event_at()
            .unwrap_or(cap)
            .min(cap);
        world_fine.step_to(next_t);
    }

    world_skip.now == world_fine.now
        && world_skip.ledger.cash_balance() == world_fine.ledger.cash_balance()
        && world_skip.internal_orders == world_fine.internal_orders
}

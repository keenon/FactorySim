//! Reference oracle for `fsim-sales` win probability, marketplace acceptance, and late/defect penalties.

use crate::{CustomerOrder, MarketBiddingPolicy};
use fsim_core::Money;

/// Reference win probability in permille (`0..1000`) given price multiplier percentage (`50..200`),
/// promised lead days, and market reputation permille (`0..1000`).
pub fn reference_win_probability_permille(
    policy: &MarketBiddingPolicy,
    reputation_permille: u32,
) -> u32 {
    if !policy.enabled {
        return 0;
    }
    let price_factor: i32 = 160 - (policy.price_multiplier_pct as i32); // 100% -> 60% base
    let lead_bonus: i32 = (14 - (policy.promised_lead_days as i32)).clamp(-20, 25) * 2;
    let rep_bonus: i32 = ((reputation_permille as i32) - 500) / 10;
    ((price_factor * 10) + (lead_bonus * 10) + (rep_bonus * 10)).clamp(50, 950) as u32
}

/// Reference payout and reputation delta when a customer order ships.
pub fn reference_settle_shipment(
    order: &CustomerOrder,
    shipped_at_ms: i64,
    in_spec: bool,
) -> (Money, i32, bool) {
    let on_time = shipped_at_ms <= order.due_at_ms;
    if !in_spec {
        // Out-of-tolerance part: customer rejection + chargeback penalty (-25% of reference payout) and -40 reputation
        let chargeback = Money(-order.payout.0 / 4);
        return (chargeback, -40, on_time);
    }
    if !on_time {
        // Late delivery penalty: 30% payout haircut and -15 reputation
        let reduced = Money((order.payout.0 * 70) / 100);
        return (reduced, -15, false);
    }
    // On time and in spec: full payout and +8 reputation
    (order.payout, 8, true)
}

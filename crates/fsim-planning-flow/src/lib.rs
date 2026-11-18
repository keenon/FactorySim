//! `fsim-planning-flow`: Backward-chaining pull order engine, bottleneck detection, and Kingman
//! queueing / WIP cap policies.
//!
//! When a customer order arrives, it creates a Shipping Order at the shipping bay. Shipping sends a
//! blocking upstream order to Packing (`part_in` + `consumable_in` carton). Packing sends blocking
//! upstream orders to Inspection/CNC and to Storage for the box. CNC sends a blocking order to the Saw
//! (or precut stock), and the Saw sends a blocking order for raw bar stock.

pub mod reference;

use fsim_protocols::{InternalOrder, InternalOrderStatus, TeamTag};
use fsim_sales::CustomerOrder;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Summary of the current and forecasted plant bottleneck.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BottleneckReport {
    pub bottleneck_team: TeamTag,
    pub active_ready_orders: usize,
    pub utilization_permille: u32,
    pub shadow_price_dollars_per_hr: i64,
    pub recommendation: String,
}

/// Builds the backward-chaining tree of `InternalOrder`s for a customer order.
pub fn create_backward_order_chain(cust: &CustomerOrder) -> Vec<InternalOrder> {
    reference::reference_build_backward_order_chain(cust)
}

/// Unblocks any `BlockedUpstream` order whose upstream dependencies have all completed.
pub fn refresh_order_unblocking(orders: &mut BTreeMap<String, InternalOrder>) -> Vec<String> {
    reference::reference_refresh_order_unblocking(orders)
}

/// Detects which team currently has the deepest queue of unblocked (`ReadyForLogistics` / `InputsStaged` / `InProcess`)
/// orders and estimates its local shadow price.
pub fn detect_bottleneck(orders: &BTreeMap<String, InternalOrder>) -> BottleneckReport {
    let mut counts: BTreeMap<TeamTag, usize> = BTreeMap::new();
    for o in orders.values() {
        if matches!(
            o.status,
            InternalOrderStatus::ReadyForLogistics
                | InternalOrderStatus::InputsStaged
                | InternalOrderStatus::InProcess
        ) {
            *counts.entry(o.target_team).or_insert(0) += 1;
        }
    }

    let (top_team, top_count) = counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .unwrap_or((TeamTag::Machining, 0));

    let util_permille = ((top_count as u32) * 180).clamp(150, 980);
    // Kingman-style nonlinear shadow price as utilization nears 100%
    let shadow_price = if util_permille > 850 {
        280
    } else if util_permille > 650 {
        125
    } else {
        45
    };

    BottleneckReport {
        bottleneck_team: top_team,
        active_ready_orders: top_count,
        utilization_permille: util_permille,
        shadow_price_dollars_per_hr: shadow_price,
        recommendation: format!(
            "Bottleneck in {} ({} unblocked orders queued; est. shadow value ${}/hr)",
            top_team.as_str(),
            top_count,
            shadow_price
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsim_core::Money;

    #[test]
    fn test_backward_order_chain_blocks_all_except_leaves_and_unblocks_bottom_up() {
        let cust = CustomerOrder {
            customer_order_id: "cust-1".to_string(),
            plant_id: "plant-1".to_string(),
            market_id: "mkt-prototyping-online".to_string(),
            part_name: "Bracket".to_string(),
            material_sku: "blank-alu".to_string(),
            requires_sawing: false,
            requires_inspection: false,
            quantity: 1,
            payout: Money::from_dollars(420),
            created_at_ms: 0,
            due_at_ms: 86_400_000,
            status: "active".to_string(),
        };

        let chain = create_backward_order_chain(&cust);
        let mut map: BTreeMap<String, InternalOrder> =
            chain.into_iter().map(|o| (o.order_id.clone(), o)).collect();

        // Only the two leaf stock orders (blank stock + carton box stock) start ReadyForLogistics;
        // CNC, Packing, and Shipping start BlockedUpstream.
        assert_eq!(
            map["cust-1-stock-blank"].status,
            InternalOrderStatus::ReadyForLogistics
        );
        assert_eq!(
            map["cust-1-stock-box"].status,
            InternalOrderStatus::ReadyForLogistics
        );
        assert_eq!(
            map["cust-1-cnc"].status,
            InternalOrderStatus::BlockedUpstream
        );
        assert_eq!(
            map["cust-1-pack"].status,
            InternalOrderStatus::BlockedUpstream
        );
        assert_eq!(
            map["cust-1-ship"].status,
            InternalOrderStatus::BlockedUpstream
        );

        // Complete blank stock order -> unblocks CNC!
        map.get_mut("cust-1-stock-blank").unwrap().status = InternalOrderStatus::Completed;
        let unblocked = refresh_order_unblocking(&mut map);
        assert_eq!(unblocked, vec!["cust-1-cnc".to_string()]);
        assert_eq!(
            map["cust-1-cnc"].status,
            InternalOrderStatus::ReadyForLogistics
        );

        // Complete CNC and box stock -> unblocks Packing!
        map.get_mut("cust-1-cnc").unwrap().status = InternalOrderStatus::Completed;
        map.get_mut("cust-1-stock-box").unwrap().status = InternalOrderStatus::Completed;
        let unblocked2 = refresh_order_unblocking(&mut map);
        assert_eq!(unblocked2, vec!["cust-1-pack".to_string()]);

        // Complete Packing -> unblocks Shipping!
        map.get_mut("cust-1-pack").unwrap().status = InternalOrderStatus::Completed;
        let unblocked3 = refresh_order_unblocking(&mut map);
        assert_eq!(unblocked3, vec!["cust-1-ship".to_string()]);
    }
}

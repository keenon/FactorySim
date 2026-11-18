//! Reference oracle for `fsim-planning-flow` backward-chaining order tree construction
//! and dependency unblocking.

use fsim_protocols::{
    HandlingUnitSpec, InternalOrder, InternalOrderStatus, RequiredInputEntry, TeamTag,
};
use fsim_sales::CustomerOrder;
use std::collections::BTreeMap;

/// Builds the complete backward-chaining tree of `InternalOrder`s for a newly accepted `CustomerOrder`.
///
/// Chain structure:
/// 1. `Shipping` order at `mat-shipping-bay` (blocked by `Packing` order).
/// 2. `Packing` order at `pack-manual-station` (requires `part_in` from `Inspection` or `Machining`,
///    and `consumable_in` (`carton-box`) from `Receiving/Storage`).
/// 3. Optional `Inspection` order at `qual-*` (if `cust.requires_inspection`), blocked by `Machining`.
/// 4. `Machining` order at `cnc-*` (requires `blank-alu` on `part_in`, blocked by `Saw` if `cust.requires_sawing`
///    or by `Receiving/Storage` precut blank stock).
/// 5. Optional `Saw` order at `mat-*bandsaw` (requires `raw-bar-alu` from `Receiving/Storage`).
pub fn reference_build_backward_order_chain(cust: &CustomerOrder) -> Vec<InternalOrder> {
    let cid = &cust.customer_order_id;
    let ship_id = format!("{cid}-ship");
    let pack_id = format!("{cid}-pack");
    let cnc_id = format!("{cid}-cnc");
    let box_stock_id = format!("{cid}-stock-box");

    let mut orders = Vec::new();

    // Determine what feeds Packing's `part_in` square
    let (part_upstream_id, part_upstream_sku) = if cust.requires_inspection {
        (format!("{cid}-insp"), "inspected-part".to_string())
    } else {
        (cnc_id.clone(), "machined-part".to_string())
    };

    // 1. Root Shipping Order (blocked by Packing)
    orders.push(InternalOrder {
        order_id: ship_id.clone(),
        parent_order_id: String::new(),
        customer_order_id: cid.clone(),
        target_team: TeamTag::MaterialsLogistics,
        target_cell_id: String::new(),
        output_spec: HandlingUnitSpec {
            container_type: "shipment".to_string(),
            item_sku: "packed-shipment".to_string(),
            quantity: cust.quantity,
            lot_id: format!("lot-{cid}"),
        },
        status: InternalOrderStatus::BlockedUpstream,
        upstream_order_ids: vec![pack_id.clone()],
        required_inputs: vec![RequiredInputEntry {
            square_name: "part_in".to_string(),
            spec: HandlingUnitSpec {
                container_type: "box".to_string(),
                item_sku: "packed-shipment".to_string(),
                quantity: cust.quantity,
                lot_id: format!("lot-{cid}"),
            },
        }],
        created_at_ms: cust.created_at_ms,
        due_at_ms: cust.due_at_ms,
    });

    // 2. Packing Order (multi-input: `part_in` + `consumable_in` for the cardboard carton)
    orders.push(InternalOrder {
        order_id: pack_id.clone(),
        parent_order_id: ship_id,
        customer_order_id: cid.clone(),
        target_team: TeamTag::FinishingPacking,
        target_cell_id: String::new(),
        output_spec: HandlingUnitSpec {
            container_type: "box".to_string(),
            item_sku: "packed-shipment".to_string(),
            quantity: cust.quantity,
            lot_id: format!("lot-{cid}"),
        },
        status: InternalOrderStatus::BlockedUpstream,
        upstream_order_ids: vec![part_upstream_id.clone(), box_stock_id.clone()],
        required_inputs: vec![
            RequiredInputEntry {
                square_name: "part_in".to_string(),
                spec: HandlingUnitSpec {
                    container_type: "tray".to_string(),
                    item_sku: part_upstream_sku,
                    quantity: cust.quantity,
                    lot_id: format!("lot-{cid}"),
                },
            },
            RequiredInputEntry {
                square_name: "consumable_in".to_string(),
                spec: HandlingUnitSpec {
                    container_type: "bundle".to_string(),
                    item_sku: "carton-box".to_string(),
                    quantity: cust.quantity,
                    lot_id: format!("lot-{cid}-box"),
                },
            },
        ],
        created_at_ms: cust.created_at_ms,
        due_at_ms: cust.due_at_ms,
    });

    // 2b. Leaf Stock Order for the Cardboard Box (`carton-box`)
    orders.push(InternalOrder {
        order_id: box_stock_id,
        parent_order_id: pack_id.clone(),
        customer_order_id: cid.clone(),
        target_team: TeamTag::MaterialsLogistics,
        target_cell_id: String::new(),
        output_spec: HandlingUnitSpec {
            container_type: "bundle".to_string(),
            item_sku: "carton-box".to_string(),
            quantity: cust.quantity,
            lot_id: format!("lot-{cid}-box"),
        },
        status: InternalOrderStatus::ReadyForLogistics,
        upstream_order_ids: vec![],
        required_inputs: vec![],
        created_at_ms: cust.created_at_ms,
        due_at_ms: cust.due_at_ms,
    });

    // 3. Optional Quality Inspection Order
    let cnc_parent_id = if cust.requires_inspection {
        let insp_id = part_upstream_id;
        orders.push(InternalOrder {
            order_id: insp_id.clone(),
            parent_order_id: pack_id,
            customer_order_id: cid.clone(),
            target_team: TeamTag::Quality,
            target_cell_id: String::new(),
            output_spec: HandlingUnitSpec {
                container_type: "tray".to_string(),
                item_sku: "inspected-part".to_string(),
                quantity: cust.quantity,
                lot_id: format!("lot-{cid}"),
            },
            status: InternalOrderStatus::BlockedUpstream,
            upstream_order_ids: vec![cnc_id.clone()],
            required_inputs: vec![RequiredInputEntry {
                square_name: "part_in".to_string(),
                spec: HandlingUnitSpec {
                    container_type: "tray".to_string(),
                    item_sku: "machined-part".to_string(),
                    quantity: cust.quantity,
                    lot_id: format!("lot-{cid}"),
                },
            }],
            created_at_ms: cust.created_at_ms,
            due_at_ms: cust.due_at_ms,
        });
        insp_id
    } else {
        pack_id
    };

    // 4. CNC Machining Order
    let cnc_upstream_id = if cust.requires_sawing {
        format!("{cid}-saw")
    } else {
        format!("{cid}-stock-blank")
    };
    orders.push(InternalOrder {
        order_id: cnc_id.clone(),
        parent_order_id: cnc_parent_id,
        customer_order_id: cid.clone(),
        target_team: TeamTag::Machining,
        target_cell_id: String::new(),
        output_spec: HandlingUnitSpec {
            container_type: "tray".to_string(),
            item_sku: "machined-part".to_string(),
            quantity: cust.quantity,
            lot_id: format!("lot-{cid}"),
        },
        status: InternalOrderStatus::BlockedUpstream,
        upstream_order_ids: vec![cnc_upstream_id.clone()],
        required_inputs: vec![RequiredInputEntry {
            square_name: "part_in".to_string(),
            spec: HandlingUnitSpec {
                container_type: "tray".to_string(),
                item_sku: "blank-alu".to_string(),
                quantity: cust.quantity,
                lot_id: format!("lot-{cid}"),
            },
        }],
        created_at_ms: cust.created_at_ms,
        due_at_ms: cust.due_at_ms,
    });

    // 5. Either Saw Order + Raw Bar Stock Order, OR Precut Blank Stock Order
    if cust.requires_sawing {
        let raw_stock_id = format!("{cid}-stock-bar");
        orders.push(InternalOrder {
            order_id: cnc_upstream_id.clone(),
            parent_order_id: cnc_id,
            customer_order_id: cid.clone(),
            target_team: TeamTag::MaterialsLogistics,
            target_cell_id: String::new(),
            output_spec: HandlingUnitSpec {
                container_type: "tray".to_string(),
                item_sku: "blank-alu".to_string(),
                quantity: cust.quantity,
                lot_id: format!("lot-{cid}"),
            },
            status: InternalOrderStatus::BlockedUpstream,
            upstream_order_ids: vec![raw_stock_id.clone()],
            required_inputs: vec![RequiredInputEntry {
                square_name: "part_in".to_string(),
                spec: HandlingUnitSpec {
                    container_type: "bar".to_string(),
                    item_sku: "raw-bar-alu".to_string(),
                    quantity: cust.quantity,
                    lot_id: format!("lot-{cid}"),
                },
            }],
            created_at_ms: cust.created_at_ms,
            due_at_ms: cust.due_at_ms,
        });
        orders.push(InternalOrder {
            order_id: raw_stock_id,
            parent_order_id: cnc_upstream_id,
            customer_order_id: cid.clone(),
            target_team: TeamTag::MaterialsLogistics,
            target_cell_id: String::new(),
            output_spec: HandlingUnitSpec {
                container_type: "bar".to_string(),
                item_sku: "raw-bar-alu".to_string(),
                quantity: cust.quantity,
                lot_id: format!("lot-{cid}"),
            },
            status: InternalOrderStatus::ReadyForLogistics,
            upstream_order_ids: vec![],
            required_inputs: vec![],
            created_at_ms: cust.created_at_ms,
            due_at_ms: cust.due_at_ms,
        });
    } else {
        orders.push(InternalOrder {
            order_id: cnc_upstream_id,
            parent_order_id: cnc_id,
            customer_order_id: cid.clone(),
            target_team: TeamTag::MaterialsLogistics,
            target_cell_id: String::new(),
            output_spec: HandlingUnitSpec {
                container_type: "tray".to_string(),
                item_sku: "blank-alu".to_string(),
                quantity: cust.quantity,
                lot_id: format!("lot-{cid}"),
            },
            status: InternalOrderStatus::ReadyForLogistics,
            upstream_order_ids: vec![],
            required_inputs: vec![],
            created_at_ms: cust.created_at_ms,
            due_at_ms: cust.due_at_ms,
        });
    }

    orders
}

/// Propagates `Completed` status from child orders to unblock parent orders whose
/// `upstream_order_ids` are now all `Completed`.
pub fn reference_refresh_order_unblocking(orders: &mut BTreeMap<String, InternalOrder>) -> Vec<String> {
    let completed_ids: std::collections::BTreeSet<String> = orders
        .iter()
        .filter(|(_, o)| o.status == InternalOrderStatus::Completed)
        .map(|(k, _)| k.clone())
        .collect();

    let mut newly_unblocked = Vec::new();
    for (id, order) in orders.iter_mut() {
        if order.status == InternalOrderStatus::BlockedUpstream
            && order
                .upstream_order_ids
                .iter()
                .all(|up_id| completed_ids.contains(up_id))
        {
            order.status = InternalOrderStatus::ReadyForLogistics;
            newly_unblocked.push(id.clone());
        }
    }
    newly_unblocked
}

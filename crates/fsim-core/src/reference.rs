//! Reference oracle for `fsim-core`: simple single-threaded unoptimized pass evaluator,
//! conservation verifier, and double-entry ledger balance checker.

use crate::{BufferView, DoubleEntryLedger, OrderPolicy, SimTime};
use sha2::{Digest, Sha256};

/// Verifies that the double-entry ledger is strictly balanced (total debits == total credits).
pub fn reference_ledger_is_balanced(ledger: &DoubleEntryLedger) -> bool {
    let mut sum: i128 = 0;
    for (_, bal) in &ledger.account_balances {
        sum += *bal as i128;
    }
    sum == 0
}

/// Counts total physical handling units present across all squares and shared racks in `BufferView`.
pub fn reference_total_units_in_buffers(buffers: &BufferView) -> usize {
    let in_squares = buffers
        .squares
        .values()
        .filter(|sq| sq.occupant.is_some())
        .count();
    let in_racks: usize = buffers.shared_racks.values().map(|v| v.len()).sum();
    in_squares + in_racks
}

/// Computes the deterministic module evaluation order for a given `OrderPolicy`, timestamp, and module list.
pub fn reference_module_order(
    policy: &OrderPolicy,
    now: SimTime,
    priority_modules: &[String],
    all_modules: &[String],
) -> Vec<String> {
    let mut result = Vec::with_capacity(all_modules.len());
    // Priority modules first (preserving their declared priority order)
    for pm in priority_modules {
        if all_modules.contains(pm) && !result.contains(pm) {
            result.push(pm.clone());
        }
    }
    let remaining: Vec<String> = all_modules
        .iter()
        .filter(|m| !result.contains(m))
        .cloned()
        .collect();

    match policy {
        OrderPolicy::Forced(list) => {
            for m in list {
                if remaining.contains(m) && !result.contains(m) {
                    result.push(m.clone());
                }
            }
            for m in remaining {
                if !result.contains(&m) {
                    result.push(m);
                }
            }
        }
        OrderPolicy::Seeded(seed) => {
            let mut keyed: Vec<([u8; 32], String)> = remaining
                .into_iter()
                .map(|m| {
                    let mut hasher = Sha256::new();
                    hasher.update(seed.to_le_bytes());
                    hasher.update(now.0.to_le_bytes());
                    hasher.update(m.as_bytes());
                    let digest: [u8; 32] = hasher.finalize().into();
                    (digest, m)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
            for (_, m) in keyed {
                result.push(m);
            }
        }
    }
    result
}

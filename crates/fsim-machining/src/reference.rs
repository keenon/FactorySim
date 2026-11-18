//! Reference oracle for `fsim-machining` cycle time and scrap rate given configuration bundles.

use crate::MachiningCellConfig;

/// Reference nominal cycle time in milliseconds for a part given base cycle time and active add-ons.
pub fn reference_cycle_time_ms(base_ms: i64, cfg: &MachiningCellConfig) -> i64 {
    let mut mult_pct = 100i64;
    if cfg.has_bar_feeder {
        mult_pct -= 15;
    }
    if cfg.has_pallet_changer {
        mult_pct -= 20;
    }
    if cfg.has_in_process_probing {
        mult_pct += 8; // Probing adds a small time cost while cutting scrap
    }
    ((base_ms * mult_pct.max(30)) / 100).max(5_000)
}

/// Reference scrap probability in parts per thousand (`0..1000`) given tool wear and probing.
pub fn reference_scrap_permille(tool_wear_pct: u32, has_probing: bool) -> u32 {
    let base = if tool_wear_pct > 80 { 45 } else { 12 };
    if has_probing {
        base / 3
    } else {
        base
    }
}

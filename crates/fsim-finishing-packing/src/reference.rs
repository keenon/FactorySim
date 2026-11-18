//! Reference oracle for `fsim-finishing-packing` multi-input packing verification and cycle time.

/// Reference packing time in milliseconds per box given station catalog ID.
pub fn reference_packing_time_ms(catalog_id: &str) -> i64 {
    match catalog_id {
        "pack-auto-bagger" => 12_000,
        "pack-case-taper-wrapper" => 20_000,
        _ => 45_000, // manual pack station
    }
}

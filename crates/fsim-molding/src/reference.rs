//! Reference oracle for `fsim-molding` cycle time and hidden process-window defect rate.

/// Reference molding cycle time in milliseconds given nominal wall thickness in tenths of a millimeter.
/// Cooling time scales roughly with the square of wall thickness.
pub fn reference_molding_cycle_ms(wall_thickness_tenth_mm: u32) -> i64 {
    let t = (wall_thickness_tenth_mm.max(10)) as i64; // 1.0 mm min
    let cooling_ms = (t * t) * 45;
    (8_000 + cooling_ms).clamp(10_000, 120_000)
}

/// Reference defect permille (`0..1000`) given how many tuning shots have been run on the mold.
pub fn reference_window_defect_permille(tuning_shots: u32) -> u32 {
    if tuning_shots >= 25 {
        8
    } else {
        (250 - tuning_shots * 9).max(8)
    }
}

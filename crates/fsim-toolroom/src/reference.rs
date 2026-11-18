//! Reference oracle for `fsim-toolroom` mold fabrication time and trial loop counts.

/// Reference mold fabrication duration in milliseconds given cavity complexity (`1..=5`).
pub fn reference_mold_build_ms(complexity_tier: u32) -> i64 {
    let base_hours = 12i64 * (complexity_tier.max(1) as i64);
    base_hours * 3_600_000
}

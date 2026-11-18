//! Reference oracle for `fsim-quality` defect detection sensitivity and false-rejection odds.

/// Returns `(true_positive_permille, false_positive_permille)` given inspection station catalog ID
/// and whether the shop climate is in spec.
pub fn reference_inspection_odds_permille(
    catalog_id: &str,
    climate_in_spec: bool,
) -> (u32, u32) {
    match catalog_id {
        "qual-bridge-cmm" => {
            if climate_in_spec {
                (992, 4)
            } else {
                // Thermal drift degrades CMM accuracy if climate room is off
                (750, 95)
            }
        }
        "qual-vision-system" => (940, 12),
        _ => (860, 20), // manual granite bench
    }
}

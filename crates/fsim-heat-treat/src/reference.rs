//! Reference oracle for `fsim-heat-treat` yield and cycle duration across the 4 heat-treatment options.

/// Reference scrap/warp permille (`0..1000`) for a heat-treat batch given furnace catalog ID and recipe tuning count.
pub fn reference_heat_treat_scrap_permille(catalog_id: &str, batches_run: u32) -> u32 {
    match catalog_id {
        "ht-vacuum-furnace" => 5,
        "ht-box-furnace" => 18,
        "ht-homebuilt-furnace" => {
            if batches_run >= 20 {
                22
            } else {
                (140 - batches_run * 6).max(22)
            }
        }
        _ => 10, // outside vendor
    }
}

//! Reference oracle for `fsim-materials-logistics` sawing time, remnant calculation, and vendor lead time.

/// Reference sawing duration in milliseconds per cut given saw catalog ID.
pub fn reference_saw_cut_ms(catalog_id: &str) -> i64 {
    match catalog_id {
        "mat-carbide-circular-saw" => 8_000,
        "mat-auto-bandsaw" => 25_000,
        _ => 60_000, // manual horizontal bandsaw
    }
}

/// Reference move duration in milliseconds given transport-graph distance in millimeters and mover speed (mm/s).
pub fn reference_move_duration_ms(distance_mm: i64, speed_mm_per_s: i64) -> i64 {
    let walk_ms = ((distance_mm.max(500) * 1_000) / speed_mm_per_s.max(200)).max(1_000);
    // Add 3 seconds pickup + 3 seconds putdown handoff time
    walk_ms + 6_000
}

//! Reference oracle for `fsim-people` domain ramp-up curve `1 - exp(-t / tau)` and task priority dispatch.

use crate::{PendingFloorTask, WorkerState};

/// Reference productivity multiplier in permille (`250..=1000`) given domain experience in hours.
/// Follows `1.0 - 0.75 * exp(-hours / 80.0)`.
pub fn reference_ramp_up_permille(domain_hours: f64) -> u32 {
    let factor = 1.0 - 0.75 * (-domain_hours.max(0.0) / 80.0).exp();
    ((factor * 1_000.0).round() as u32).clamp(250, 1_000)
}

/// Deterministic priority comparator for selecting the next task to dispatch to an idle worker's iPad.
pub fn reference_select_next_task_index(
    worker: &WorkerState,
    tasks: &[PendingFloorTask],
    priority_categories: &[String],
) -> Option<usize> {
    if !worker.is_idle() || tasks.is_empty() {
        return None;
    }
    let mut best_idx: Option<usize> = None;
    let mut best_key: (usize, i64, i64, String) = (usize::MAX, i64::MAX, i64::MAX, String::new());

    for (idx, task) in tasks.iter().enumerate() {
        let cat_rank = priority_categories
            .iter()
            .position(|c| c == &task.category)
            .unwrap_or(priority_categories.len());
        let dist = worker.current_pose.point().distance_mm(task.location_mm);
        let key = (cat_rank, task.created_at.0, dist, task.task_id.clone());
        if best_idx.is_none() || key < best_key {
            best_idx = Some(idx);
            best_key = key;
        }
    }
    best_idx
}

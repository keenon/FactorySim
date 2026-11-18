//! `fsim-people`: Plant worker roster, domain ramp-up (`1 - e^(-t/tau)`), labor market hiring
//! pipeline, and the central iPad task dispatch algorithm.
//! Workers carry iPads (virtual devices on `fsim-net`), cannot be interrupted mid-task, and
//! both iPad dispatch communication and physical execution take a stochastic amount of time.

pub mod reference;

use fsim_core::{Money, SimTime, StreamRng};
use fsim_geom::PointMm;
use fsim_motion::{build_line_segments, pose_at, MotionSegment, Pose2D};
use fsim_protocols::OperatorTaskKind;
use serde::{Deserialize, Serialize};

/// A floor task waiting to be dispatched to an available worker's iPad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingFloorTask {
    pub task_id: String,
    pub plant_id: String,
    pub category: String, // "alarms", "load_unload", "logistics", "packing", "sawing", "waste"
    pub order_id: String,
    pub location_mm: PointMm,
    pub destination_mm: PointMm,
    pub base_duration_ms: i64,
    pub created_at: SimTime,
    pub kind: OperatorTaskKind,
}

/// Active non-interruptible task state on a worker's iPad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveWorkerTask {
    pub task: PendingFloorTask,
    pub ipad_ack_done_at: SimTime,
    pub walk_done_at: SimTime,
    pub work_done_at: SimTime,
}

/// State of a single plant worker carrying an iPad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerState {
    pub worker_id: String,
    pub name: String,
    pub role: String,
    pub plant_id: String,
    pub ipad_device_id: String,
    pub wage_per_hour: Money,
    pub hired_at: SimTime,
    pub ready_at: SimTime, // onboarding completion time
    pub worked_ms_in_domain: i64,
    pub quit_risk_permille: u32,
    pub current_pose: Pose2D,
    pub motion_segments: Vec<MotionSegment>,
    pub active_task: Option<ActiveWorkerTask>,
    pub dispatch_count: u64,
}

impl WorkerState {
    pub fn is_onboarded(&self, now: SimTime) -> bool {
        now >= self.ready_at
    }

    pub fn is_idle(&self) -> bool {
        self.active_task.is_none()
    }

    pub fn ramp_up_permille(&self) -> u32 {
        let hours = (self.worked_ms_in_domain as f64) / 3_600_000.0;
        reference::reference_ramp_up_permille(hours)
    }

    pub fn pose_at(&self, now: SimTime) -> Pose2D {
        pose_at(&self.motion_segments, now.0).unwrap_or(self.current_pose)
    }
}

/// Default shift task priority order when a player starts a new game.
pub fn default_task_priorities() -> Vec<String> {
    vec![
        "alarms".to_string(),
        "load_unload".to_string(),
        "packing".to_string(),
        "sawing".to_string(),
        "logistics".to_string(),
        "waste".to_string(),
    ]
}

/// Dispatches the highest-priority pending floor task to an idle, onboarded worker's iPad.
/// Samples both the stochastic iPad communication/reading delay (2–6 seconds) and the
/// ramp-up-scaled physical task execution duration.
pub fn dispatch_task_to_worker(
    worker: &mut WorkerState,
    task: PendingFloorTask,
    waypoints_to_start: &[PointMm],
    waypoints_to_dest: &[PointMm],
    now: SimTime,
    game_seed: u64,
) -> ActiveWorkerTask {
    worker.dispatch_count += 1;
    let mut rng = StreamRng::new(
        game_seed,
        "fsim-people",
        &worker.worker_id,
        worker.dispatch_count,
    );

    // 1. Stochastic iPad notification + reading/acknowledgement time (2s..6s)
    let ipad_comm_ms = rng.gen_range_i64(2_000, 6_000);
    let ipad_ack_done_at = now.add_ms(ipad_comm_ms);

    // 2. Walking segments (1.3 m/s nominal, stochastic 1.15..1.40 m/s)
    let walk_speed_mm_s = rng.gen_range_i64(1_150, 1_400);
    let mut all_waypoints = Vec::new();
    if waypoints_to_start.is_empty() {
        all_waypoints.push(worker.current_pose.point());
        all_waypoints.push(task.location_mm);
    } else {
        all_waypoints.extend_from_slice(waypoints_to_start);
    }
    if !waypoints_to_dest.is_empty() {
        all_waypoints.extend_from_slice(waypoints_to_dest);
    } else if task.destination_mm != task.location_mm {
        all_waypoints.push(task.destination_mm);
    }

    let segments = build_line_segments(&all_waypoints, ipad_ack_done_at.0, walk_speed_mm_s);
    let walk_done_ms = segments
        .last()
        .map(|s| s.end_ms)
        .unwrap_or(ipad_ack_done_at.0);
    let walk_done_at = SimTime(walk_done_ms);

    // 3. Stochastic task execution time conditioned on worker ramp-up
    let ramp_permille = worker.ramp_up_permille().max(250) as i64;
    let noise_pct = rng.gen_range_i64(85, 120);
    let scaled_work_ms = ((task.base_duration_ms * 1_000) / ramp_permille) * noise_pct / 100;
    let work_done_at = walk_done_at.add_ms(scaled_work_ms.max(1_000));

    worker.motion_segments = segments;
    if let Some(last_seg) = worker.motion_segments.last() {
        worker.current_pose = last_seg.end_pose;
    }

    let active = ActiveWorkerTask {
        task,
        ipad_ack_done_at,
        walk_done_at,
        work_done_at,
    };
    worker.active_task = Some(active.clone());
    active
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsim_protocols::HandlingUnitSpec;

    #[test]
    fn test_worker_ipad_dispatch_is_non_interruptible_and_stochastic() {
        let mut worker = WorkerState {
            worker_id: "w-1".to_string(),
            name: "Alex Rivera".to_string(),
            role: "Machinist / Operator".to_string(),
            plant_id: "plant-1".to_string(),
            ipad_device_id: "plant/1/person/w-1/ipad".to_string(),
            wage_per_hour: Money::from_dollars(32),
            hired_at: SimTime::ZERO,
            ready_at: SimTime::ZERO,
            worked_ms_in_domain: 0,
            quit_risk_permille: 15,
            current_pose: Pose2D::new(1_000, 1_000, 0),
            motion_segments: Vec::new(),
            active_task: None,
            dispatch_count: 0,
        };

        let task = PendingFloorTask {
            task_id: "t-1".to_string(),
            plant_id: "plant-1".to_string(),
            category: "logistics".to_string(),
            order_id: "ord-1".to_string(),
            location_mm: PointMm::new(3_000, 1_000),
            destination_mm: PointMm::new(6_000, 1_000),
            base_duration_ms: 10_000,
            created_at: SimTime::ZERO,
            kind: OperatorTaskKind::MoveHandlingUnit {
                from_cell: "recv-1".to_string(),
                from_square: "out".to_string(),
                to_cell: "cnc-1".to_string(),
                to_square: "part_in".to_string(),
                spec: HandlingUnitSpec {
                    container_type: "tray".to_string(),
                    item_sku: "blank-alu".to_string(),
                    quantity: 1,
                    lot_id: "lot-1".to_string(),
                },
            },
        };

        let active = dispatch_task_to_worker(&mut worker, task.clone(), &[], &[], SimTime::ZERO, 42);
        assert!(active.ipad_ack_done_at.0 >= 2_000);
        assert!(active.walk_done_at > active.ipad_ack_done_at);
        assert!(active.work_done_at > active.walk_done_at);
        // While active, worker is not idle and cannot be interrupted by a second task
        assert!(!worker.is_idle());
        assert!(reference::reference_select_next_task_index(
            &worker,
            &[task],
            &default_task_priorities()
        )
        .is_none());
    }
}

//! Reference oracle for `fsim-motion` timed segment evaluation.

use crate::{MotionSegment, Pose2D};

/// Simple reference evaluator for a list of timed motion segments at timestamp `now_ms`.
pub fn reference_pose_at(segments: &[MotionSegment], now_ms: i64) -> Option<Pose2D> {
    if segments.is_empty() {
        return None;
    }
    if now_ms <= segments[0].start_ms {
        return Some(segments[0].start_pose);
    }
    for seg in segments {
        if now_ms >= seg.start_ms && now_ms <= seg.end_ms {
            let dur = (seg.end_ms - seg.start_ms).max(1) as f64;
            let frac = ((now_ms - seg.start_ms) as f64 / dur).clamp(0.0, 1.0);
            let x = (seg.start_pose.x_mm as f64
                + (seg.end_pose.x_mm - seg.start_pose.x_mm) as f64 * frac)
                .round() as i32;
            let y = (seg.start_pose.y_mm as f64
                + (seg.end_pose.y_mm - seg.start_pose.y_mm) as f64 * frac)
                .round() as i32;
            let heading_mdeg = interpolate_angle_mdeg(
                seg.start_pose.heading_mdeg,
                seg.end_pose.heading_mdeg,
                frac,
            );
            return Some(Pose2D {
                x_mm: x,
                y_mm: y,
                heading_mdeg,
            });
        }
    }
    segments.last().map(|s| s.end_pose)
}

pub fn interpolate_angle_mdeg(start_mdeg: i32, end_mdeg: i32, frac: f64) -> i32 {
    let mut diff = (end_mdeg - start_mdeg).rem_euclid(360_000);
    if diff > 180_000 {
        diff -= 360_000;
    }
    (start_mdeg + (diff as f64 * frac).round() as i32).rem_euclid(360_000)
}

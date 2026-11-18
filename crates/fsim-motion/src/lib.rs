//! `fsim-motion`: Timed line and arc motion segments and pure `pose_at` interpolation
//! shared between the Rust simulation core and the WebAssembly/TypeScript viewer.

pub mod reference;

use fsim_geom::PointMm;
use serde::{Deserialize, Serialize};

/// 2D pose in integer millimeters and millidegrees (`0..360_000`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pose2D {
    pub x_mm: i32,
    pub y_mm: i32,
    pub heading_mdeg: i32,
}

impl Pose2D {
    pub const fn new(x_mm: i32, y_mm: i32, heading_mdeg: i32) -> Self {
        Self {
            x_mm,
            y_mm,
            heading_mdeg,
        }
    }

    pub const fn point(self) -> PointMm {
        PointMm::new(self.x_mm, self.y_mm)
    }
}

/// Geometry of a single motion segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentKind {
    Line,
    Arc {
        center_x_mm: i32,
        center_y_mm: i32,
        radius_mm: i32,
        clockwise: bool,
    },
}

/// A timed motion segment stored in a mover's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotionSegment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub start_pose: Pose2D,
    pub end_pose: Pose2D,
    pub kind: SegmentKind,
}

/// Builds a timed chain of straight-line segments from a polyline of `PointMm` waypoints,
/// given a start timestamp `start_ms` and speed in millimeters per second (`speed_mm_per_s`).
pub fn build_line_segments(
    waypoints: &[PointMm],
    start_ms: i64,
    speed_mm_per_s: i64,
) -> Vec<MotionSegment> {
    let speed = speed_mm_per_s.max(100);
    if waypoints.is_empty() {
        return Vec::new();
    }
    if waypoints.len() == 1 {
        let p = Pose2D::new(waypoints[0].x, waypoints[0].y, 0);
        return vec![MotionSegment {
            start_ms,
            end_ms: start_ms,
            start_pose: p,
            end_pose: p,
            kind: SegmentKind::Line,
        }];
    }
    let mut segments = Vec::with_capacity(waypoints.len() - 1);
    let mut now = start_ms;
    for i in 0..(waypoints.len() - 1) {
        let a = waypoints[i];
        let b = waypoints[i + 1];
        let dist_mm = a.distance_mm(b);
        let dur_ms = ((dist_mm * 1_000) / speed).max(1);
        let dx = (b.x - a.x) as f64;
        let dy = (b.y - a.y) as f64;
        let heading_mdeg = (dy.atan2(dx).to_degrees() * 1_000.0).round() as i32;
        let heading_norm = heading_mdeg.rem_euclid(360_000);
        segments.push(MotionSegment {
            start_ms: now,
            end_ms: now + dur_ms,
            start_pose: Pose2D::new(a.x, a.y, heading_norm),
            end_pose: Pose2D::new(b.x, b.y, heading_norm),
            kind: SegmentKind::Line,
        });
        now += dur_ms;
    }
    segments
}

/// Pure interpolation of mover pose at timestamp `now_ms`.
pub fn pose_at(segments: &[MotionSegment], now_ms: i64) -> Option<Pose2D> {
    reference::reference_pose_at(segments, now_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_and_interpolate_segments() {
        let pts = vec![
            PointMm::new(0, 0),
            PointMm::new(1_200, 0),
            PointMm::new(1_200, 1_200),
        ];
        let segs = build_line_segments(&pts, 1_000, 1_200);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].start_ms, 1_000);
        assert_eq!(segs[0].end_ms, 2_000);
        assert_eq!(segs[1].end_ms, 3_000);

        let mid = pose_at(&segs, 1_500).unwrap();
        assert_eq!(mid.x_mm, 600);
        assert_eq!(mid.y_mm, 0);

        let end = pose_at(&segs, 5_000).unwrap();
        assert_eq!(end.x_mm, 1_200);
        assert_eq!(end.y_mm, 1_200);
    }
}

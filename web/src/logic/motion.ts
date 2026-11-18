// Pure motion interpolation, mirroring `fsim-motion::reference::reference_pose_at`.

import type { MotionSegment, Pose2D, SimTimeMs } from '../types';

export function interpolateAngleMdeg(startMdeg: number, endMdeg: number, frac: number): number {
  let diff = (((endMdeg - startMdeg) % 360000) + 360000) % 360000;
  if (diff > 180000) diff -= 360000;
  const raw = startMdeg + Math.round(diff * frac);
  return ((raw % 360000) + 360000) % 360000;
}

export function poseAt(segments: readonly MotionSegment[], nowMs: SimTimeMs): Pose2D | null {
  if (segments.length === 0) return null;
  if (nowMs <= segments[0].start_ms) return { ...segments[0].start_pose };
  for (const seg of segments) {
    if (nowMs >= seg.start_ms && nowMs <= seg.end_ms) {
      const frac = Math.min(1, Math.max(0, (nowMs - seg.start_ms) / Math.max(1, seg.end_ms - seg.start_ms)));
      return {
        x_mm: Math.round(seg.start_pose.x_mm + (seg.end_pose.x_mm - seg.start_pose.x_mm) * frac),
        y_mm: Math.round(seg.start_pose.y_mm + (seg.end_pose.y_mm - seg.start_pose.y_mm) * frac),
        heading_mdeg: interpolateAngleMdeg(seg.start_pose.heading_mdeg, seg.end_pose.heading_mdeg, frac),
      };
    }
  }
  return { ...segments[segments.length - 1].end_pose };
}

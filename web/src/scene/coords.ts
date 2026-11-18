// Pure mapping between fsim millimeter floor coordinates and Three.js world meters.
// The scene is Z-up (camera.up = +Z), so x/y are floor axes exactly as in the sim.

import { rotatedSize } from '../logic/placement';

export const MM = 0.001;

export interface CellTransform {
  /** Footprint center in meters (group origin). */
  cx: number;
  cy: number;
  /** Rotation about +Z in radians. */
  yaw: number;
}

/**
 * Transform for a cell whose footprint origin (min corner after rotation) is `(xMm, yMm)` and
 * whose unrotated catalog size is `widthMm x depthMm`. Models are built centered on their
 * footprint, so rotating the group about its center reproduces `AabbMm::from_origin_size`.
 */
export function cellTransform(xMm: number, yMm: number, widthMm: number, depthMm: number, rotationDeg: number): CellTransform {
  const [w, d] = rotatedSize(widthMm, depthMm, rotationDeg);
  return {
    cx: (xMm + w / 2) * MM,
    cy: (yMm + d / 2) * MM,
    yaw: (rotationDeg * Math.PI) / 180,
  };
}

/** Inverse of `cellTransform` for dragging: footprint origin from a desired center (meters). */
export function originFromCenter(cxM: number, cyM: number, widthMm: number, depthMm: number, rotationDeg: number): { x_mm: number; y_mm: number } {
  const [w, d] = rotatedSize(widthMm, depthMm, rotationDeg);
  return { x_mm: Math.round(cxM / MM - w / 2), y_mm: Math.round(cyM / MM - d / 2) };
}

// Pure footprint geometry for planning-mode ghosts.
//
// `previewPlacementErrors` gives *instant* per-frame feedback while dragging (AABB checks only).
// It is a UI preview, not the authority: on drop the client asks the server
// (`/validate-placement`, backed by fsim-facilities-ehs exact polygon predicates) and replaces
// the draft's errors with the server's verdict.

import type { BuildingListing, CellCatalogEntry, InstalledCell } from '../types';

export interface RectMm {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

/** Footprint size after rotation (90/270 swap width and depth), matching `AabbMm::from_origin_size`. */
export function rotatedSize(widthMm: number, depthMm: number, rotationDeg: number): [number, number] {
  const r = ((rotationDeg % 360) + 360) % 360;
  return r === 90 || r === 270 ? [depthMm, widthMm] : [widthMm, depthMm];
}

export function footprintRect(
  xMm: number,
  yMm: number,
  widthMm: number,
  depthMm: number,
  rotationDeg: number,
  inflateMm = 0,
): RectMm {
  const [w, d] = rotatedSize(widthMm, depthMm, rotationDeg);
  return { minX: xMm - inflateMm, minY: yMm - inflateMm, maxX: xMm + w + inflateMm, maxY: yMm + d + inflateMm };
}

export function rectsOverlap(a: RectMm, b: RectMm): boolean {
  return a.minX < b.maxX && a.maxX > b.minX && a.minY < b.maxY && a.maxY > b.minY;
}

export function polygonBounds(points: readonly { x: number; y: number }[]): RectMm {
  const r: RectMm = { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity };
  for (const p of points) {
    r.minX = Math.min(r.minX, p.x);
    r.minY = Math.min(r.minY, p.y);
    r.maxX = Math.max(r.maxX, p.x);
    r.maxY = Math.max(r.maxY, p.y);
  }
  return r;
}

export function snapMm(v: number, gridMm = 100): number {
  return Math.round(v / gridMm) * gridMm;
}

export interface PlacementDraft {
  draft_id: string;
  plant_id: string;
  catalog_id: string;
  x_mm: number;
  y_mm: number;
  rotation_deg: number;
  width_mm: number;
  depth_mm: number;
  height_mm: number;
  clearance_mm: number;
  display_name: string;
}

export function previewPlacementErrors(
  draft: PlacementDraft,
  listing: BuildingListing | null,
  installed: readonly InstalledCell[],
  catalogs: readonly CellCatalogEntry[],
  otherDrafts: readonly PlacementDraft[],
): string[] {
  const errors: string[] = [];
  if (!listing) return errors;
  const me = footprintRect(draft.x_mm, draft.y_mm, draft.width_mm, draft.depth_mm, draft.rotation_deg, draft.clearance_mm);

  const b = polygonBounds(listing.floor_polygon.outer);
  if (me.minX < b.minX || me.minY < b.minY || me.maxX > b.maxX || me.maxY > b.maxY) {
    errors.push('Footprint + clearance extends outside building walls');
  }
  for (const hole of listing.floor_polygon.holes) {
    if (rectsOverlap(me, polygonBounds(hole))) {
      errors.push('Overlaps a structural column');
      break;
    }
  }
  if (draft.height_mm > listing.clear_height_mm) {
    errors.push('Exceeds building clear height');
  }
  for (const c of installed) {
    if (c.plant_id !== draft.plant_id) continue;
    const cat = catalogs.find((x) => x.catalog_id === c.catalog_id);
    if (!cat) continue;
    const other = footprintRect(c.x_mm, c.y_mm, cat.width_mm, cat.depth_mm, c.rotation_deg, cat.clearance_mm);
    if (rectsOverlap(me, other)) errors.push(`Overlaps clearance of '${c.cell_id}'`);
  }
  for (const o of otherDrafts) {
    if (o.draft_id === draft.draft_id || o.plant_id !== draft.plant_id) continue;
    const other = footprintRect(o.x_mm, o.y_mm, o.width_mm, o.depth_mm, o.rotation_deg, o.clearance_mm);
    if (rectsOverlap(me, other)) errors.push(`Overlaps planned '${o.display_name}'`);
  }
  return errors;
}

/** Finds a free spot for a new draft by scanning a coarse grid inside the building. */
export function findFreeSpot(
  base: Omit<PlacementDraft, 'x_mm' | 'y_mm'>,
  listing: BuildingListing | null,
  installed: readonly InstalledCell[],
  catalogs: readonly CellCatalogEntry[],
  otherDrafts: readonly PlacementDraft[],
): { x_mm: number; y_mm: number } {
  if (!listing) return { x_mm: 2000, y_mm: 2000 };
  const b = polygonBounds(listing.floor_polygon.outer);
  const step = 500;
  for (let y = b.minY + base.clearance_mm; y < b.maxY; y += step) {
    for (let x = b.minX + base.clearance_mm; x < b.maxX; x += step) {
      const candidate = { ...base, x_mm: x, y_mm: y };
      if (previewPlacementErrors(candidate, listing, installed, catalogs, otherDrafts).length === 0) {
        return { x_mm: x, y_mm: y };
      }
    }
  }
  return { x_mm: b.minX + base.clearance_mm, y_mm: b.minY + base.clearance_mm };
}

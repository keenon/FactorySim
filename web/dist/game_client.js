// Pure helper functions and central OOP `GameClient` state class for Factory Sim.
// Follows AGENTS.md:
// - The server owns simulation state; the browser never simulates.
// - One central OOP state class (`GameClient`) holds a mirror of the server's estimated state
//   plus UI-local state (active tab, selected plant, selected entity, planning mode drafts,
//   undo stack, 3D orbit camera, unit/currency settings).
// - Complex logic is factored into pure, unit-tested functions (`poseAt`, `formatMoney`,
//   `formatDimension`, `project3DPoint`, `unprojectScreenToFloorMm`, `validateDraftLocally`,
//   `packScenePrimitives`, `hitTestPrimitiveAtScreen`).

/**
 * Interpolates millidegree angles across the 0..360_000 boundary (matches `fsim-motion::reference`).
 * @param {number} startMdeg
 * @param {number} endMdeg
 * @param {number} frac
 * @returns {number}
 */
export function interpolateAngleMdeg(startMdeg, endMdeg, frac) {
  let diff = ((endMdeg - startMdeg) % 360000 + 360000) % 360000;
  if (diff > 180000) {
    diff -= 360000;
  }
  const raw = startMdeg + Math.round(diff * frac);
  return ((raw % 360000) + 360000) % 360000;
}

/**
 * Pure motion interpolation matching `fsim-motion::pose_at(segments, now_ms)`.
 * @param {Array<object>} segments
 * @param {number} nowMs
 * @returns {{x_mm: number, y_mm: number, heading_mdeg: number} | null}
 */
export function poseAt(segments, nowMs) {
  if (!Array.isArray(segments) || segments.length === 0) {
    return null;
  }
  if (nowMs <= segments[0].start_ms) {
    return { ...segments[0].start_pose };
  }
  for (const seg of segments) {
    if (nowMs >= seg.start_ms && nowMs <= seg.end_ms) {
      const dur = Math.max(1, seg.end_ms - seg.start_ms);
      const frac = Math.min(1, Math.max(0, (nowMs - seg.start_ms) / dur));
      const x_mm = Math.round(
        seg.start_pose.x_mm + (seg.end_pose.x_mm - seg.start_pose.x_mm) * frac
      );
      const y_mm = Math.round(
        seg.start_pose.y_mm + (seg.end_pose.y_mm - seg.start_pose.y_mm) * frac
      );
      const heading_mdeg = interpolateAngleMdeg(
        seg.start_pose.heading_mdeg,
        seg.end_pose.heading_mdeg,
        frac
      );
      return { x_mm, y_mm, heading_mdeg };
    }
  }
  return { ...segments[segments.length - 1].end_pose };
}

/**
 * Formats integer tenth-cents (`$1.00 = 1_000`) into currency strings.
 * @param {number} tenthCents
 * @param {'USD' | 'EUR'} currency
 * @returns {string}
 */
export function formatMoney(tenthCents, currency = 'USD') {
  const dollars = (tenthCents || 0) / 1000;
  const symbol = currency === 'EUR' ? '€' : '$';
  const sign = dollars < 0 ? '-' : '';
  const abs = Math.abs(dollars);
  if (abs >= 1000) {
    return `${sign}${symbol}${abs.toLocaleString('en-US', {
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    })}`;
  }
  return `${sign}${symbol}${abs.toLocaleString('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })}`;
}

/**
 * Formats millimeter dimensions in metric (`m`) or imperial (`ft`).
 * @param {number} mm
 * @param {'metric' | 'imperial'} unitSystem
 * @returns {string}
 */
export function formatDimension(mm, unitSystem = 'metric') {
  if (unitSystem === 'imperial') {
    const feet = (mm || 0) / 304.8;
    return `${feet.toFixed(1)} ft`;
  }
  const meters = (mm || 0) / 1000;
  return `${meters.toFixed(1)} m`;
}

/**
 * True 3D isometric / orbital projection from millimeter world coordinates
 * `(x_mm, y_mm, z_mm)` to 2D canvas screen coordinates `(sx, sy, depth)`.
 *
 * Camera parameters:
 * - `yawRad`: horizontal rotation angle around the vertical Z axis (default `0.62` rad ≈ 35.5°)
 * - `pitchRad`: elevation tilt angle from top-down (`0.0` = top-down, `0.95` = 55° 3D tilt)
 * - `zoom`: pixels per millimeter (default `0.028`)
 * - `panX`, `panY`: screen center offset in pixels
 *
 * @param {number} x_mm
 * @param {number} y_mm
 * @param {number} z_mm
 * @param {{panX: number, panY: number, zoom: number, yawRad?: number, pitchRad?: number, tilt?: number}} camera
 * @returns {{sx: number, sy: number, depth: number}}
 */
export function project3DPoint(x_mm, y_mm, z_mm, camera) {
  const scale = camera.zoom || 0.028;
  const yaw = camera.yawRad ?? 0.58;
  const pitch = camera.pitchRad ?? (camera.tilt !== undefined ? camera.tilt : 0.88);

  const cosY = Math.cos(yaw);
  const sinY = Math.sin(yaw);
  const cosP = Math.cos(pitch);
  const sinP = Math.sin(pitch);

  // Rotate around Z axis (floor plane)
  const rx = x_mm * cosY - y_mm * sinY;
  const ry = x_mm * sinY + y_mm * cosY;

  // Tilt by pitch and elevate Z upwards on screen
  const sx = camera.panX + rx * scale;
  const sy = camera.panY + (ry * cosP - z_mm * sinP) * scale;
  const depth = ry * sinP + z_mm * cosP;
  return { sx, sy, depth };
}

/**
 * Inverse projection from 2D canvas screen coordinates `(sx, sy)` onto the `z_mm = 0`
 * factory floor plane `(x_mm, y_mm)`. Exact inverse of `project3DPoint(x, y, 0, camera)`.
 *
 * @param {number} sx
 * @param {number} sy
 * @param {{panX: number, panY: number, zoom: number, yawRad?: number, pitchRad?: number, tilt?: number}} camera
 * @returns {{x_mm: number, y_mm: number}}
 */
export function unprojectScreenToFloorMm(sx, sy, camera) {
  const scale = camera.zoom || 0.028;
  const yaw = camera.yawRad ?? 0.58;
  const pitch = camera.pitchRad ?? (camera.tilt !== undefined ? camera.tilt : 0.88);

  const cosY = Math.cos(yaw);
  const sinY = Math.sin(yaw);
  const cosP = Math.max(0.15, Math.cos(pitch));

  const rx = (sx - camera.panX) / scale;
  const ry = (sy - camera.panY) / (scale * cosP);

  const x_mm = Math.round(rx * cosY + ry * sinY);
  const y_mm = Math.round(-rx * sinY + ry * cosY);
  return { x_mm, y_mm };
}

/**
 * Instant client-side placement validator for planning-mode ghost cells and live dragging.
 * Checks building boundaries, column holes, clear height, door opening fit, and overlap
 * against both installed cells and other planned ghost cells.
 *
 * @param {object} draft
 * @param {object} listing
 * @param {Array<object>} installedCells
 * @param {Array<object>} allCatalogs
 * @param {Array<object>} otherDrafts
 * @returns {{valid: boolean, errors: Array<string>}}
 */
export function validateDraftLocally(
  draft,
  listing,
  installedCells = [],
  allCatalogs = [],
  otherDrafts = []
) {
  const errors = [];
  if (!listing) {
    return { valid: true, errors };
  }

  const rot = ((draft.rotation_deg || 0) % 360 + 360) % 360;
  const w = rot === 90 || rot === 270 ? draft.depth_mm : draft.width_mm;
  const d = rot === 90 || rot === 270 ? draft.width_mm : draft.depth_mm;
  const clearance = draft.clearance_mm ?? 300;

  const minX = draft.x_mm - clearance;
  const minY = draft.y_mm - clearance;
  const maxX = draft.x_mm + w + clearance;
  const maxY = draft.y_mm + d + clearance;

  // Check building outer bounds
  const outer = listing.floor_polygon?.outer || [];
  if (outer.length >= 3) {
    let bMinX = Infinity,
      bMinY = Infinity,
      bMaxX = -Infinity,
      bMaxY = -Infinity;
    for (const p of outer) {
      bMinX = Math.min(bMinX, p.x);
      bMinY = Math.min(bMinY, p.y);
      bMaxX = Math.max(bMaxX, p.x);
      bMaxY = Math.max(bMaxY, p.y);
    }
    if (minX < bMinX || minY < bMinY || maxX > bMaxX || maxY > bMaxY) {
      errors.push('Footprint + clearance extends outside building walls');
    }
  }

  // Check interior structural column holes
  for (const hole of listing.floor_polygon?.holes || []) {
    let hMinX = Infinity,
      hMinY = Infinity,
      hMaxX = -Infinity,
      hMaxY = -Infinity;
    for (const p of hole) {
      hMinX = Math.min(hMinX, p.x);
      hMinY = Math.min(hMinY, p.y);
      hMaxX = Math.max(hMaxX, p.x);
      hMaxY = Math.max(hMaxY, p.y);
    }
    if (minX < hMaxX && maxX > hMinX && minY < hMaxY && maxY > hMinY) {
      errors.push('Overlaps an interior structural column');
    }
  }

  if (draft.height_mm > listing.clear_height_mm) {
    errors.push('Exceeds building clear height');
  }

  // Check overlap against installed cells
  for (const c of installedCells) {
    if (c.plant_id !== draft.plant_id) continue;
    if (c.cell_id === draft.ignore_cell_id) continue;
    const cat = allCatalogs.find((x) => x.catalog_id === c.catalog_id);
    const cw = cat ? cat.width_mm : 2000;
    const cd = cat ? cat.depth_mm : 2000;
    const cc = cat ? cat.clearance_mm : 300;
    const cMinX = c.x_mm - cc;
    const cMinY = c.y_mm - cc;
    const cMaxX = c.x_mm + cw + cc;
    const cMaxY = c.y_mm + cd + cc;
    if (minX < cMaxX && maxX > cMinX && minY < cMaxY && maxY > cMinY) {
      errors.push(`Overlaps clearance of installed cell '${c.cell_id}'`);
    }
  }

  // Check overlap against other planned ghost cells
  for (const od of otherDrafts) {
    if (od.draft_id === draft.draft_id || od.plant_id !== draft.plant_id) continue;
    const oRot = ((od.rotation_deg || 0) % 360 + 360) % 360;
    const ow = oRot === 90 || oRot === 270 ? od.depth_mm : od.width_mm;
    const odDepth = oRot === 90 || oRot === 270 ? od.width_mm : od.depth_mm;
    const oc = od.clearance_mm ?? 300;
    const oMinX = od.x_mm - oc;
    const oMinY = od.y_mm - oc;
    const oMaxX = od.x_mm + ow + oc;
    const oMaxY = od.y_mm + odDepth + oc;
    if (minX < oMaxX && maxX > oMinX && minY < oMaxY && maxY > oMinY) {
      errors.push(`Overlaps planned ghost '${od.display_name}'`);
    }
  }

  return { valid: errors.length === 0, errors };
}

/**
 * Pure scene buffer packer that converts the server's estimated `ClientGameSnapshot` and
 * any local planning-mode ghost cells into depth-sorted 3D solids for rendering and picking.
 *
 * @param {object} snapshot
 * @param {string} plantId
 * @param {Array<object>} plannedDrafts
 * @param {number} renderNowMs
 * @returns {Array<object>}
 */
export function packScenePrimitives(snapshot, plantId, plannedDrafts = [], renderNowMs = 0) {
  if (!snapshot || !snapshot.world) {
    return [];
  }
  const nodes = (snapshot.scene_nodes_by_plant && snapshot.scene_nodes_by_plant[plantId]) || [];
  const primitives = [];

  for (const n of nodes) {
    const isCell = n.node_id.startsWith('cell-');
    const entityId = isCell ? n.node_id.slice(5) : n.node_id;
    primitives.push({
      id: n.node_id,
      entityId,
      kind: isCell ? 'cell' : 'square',
      x_mm: n.x_mm,
      y_mm: n.y_mm,
      z_mm: n.z_mm || 0,
      width_mm: n.width_mm,
      depth_mm: n.depth_mm,
      height_mm: n.height_mm,
      color: n.color_hex,
      label: n.label,
      ghost: Boolean(n.ghost),
      valid: true,
      draggable: isCell,
    });
  }

  // Add planning-mode ghost drafts with instant validity coloring
  for (const draft of plannedDrafts) {
    if (draft.plant_id !== plantId) continue;
    const rot = ((draft.rotation_deg || 0) % 360 + 360) % 360;
    const w = rot === 90 || rot === 270 ? draft.depth_mm : draft.width_mm;
    const d = rot === 90 || rot === 270 ? draft.width_mm : draft.depth_mm;
    primitives.push({
      id: draft.draft_id,
      entityId: draft.draft_id,
      kind: 'planned_ghost',
      x_mm: draft.x_mm,
      y_mm: draft.y_mm,
      z_mm: 0,
      width_mm: w,
      depth_mm: d,
      height_mm: draft.height_mm,
      clearance_mm: draft.clearance_mm ?? 300,
      color: draft.valid ? '#38bdf8' : '#ef4444',
      label: `[PLAN] ${draft.display_name}`,
      ghost: true,
      valid: draft.valid,
      draggable: true,
    });
  }

  // Add interpolated worker cylinders (`poseAt`)
  const workers = Object.values(snapshot.world.workers || {});
  const tMs = renderNowMs || snapshot.world.now || 0;
  for (const w of workers) {
    if (w.plant_id !== plantId) continue;
    const pose =
      poseAt(w.motion_segments || [], tMs) || w.current_pose || { x_mm: 1500, y_mm: 1500 };
    primitives.push({
      id: `worker-${w.worker_id}`,
      entityId: w.worker_id,
      kind: 'worker',
      x_mm: pose.x_mm - 300,
      y_mm: pose.y_mm - 300,
      z_mm: 0,
      width_mm: 600,
      depth_mm: 600,
      height_mm: 1750,
      color: w.active_task ? '#10b981' : '#f8fafc',
      label: `${w.name}`,
      ghost: false,
      valid: true,
      draggable: false,
    });
  }

  // Painter's algorithm sort along 3D camera depth (`x + y`)
  primitives.sort(
    (a, b) => a.x_mm + a.y_mm + (a.width_mm + a.depth_mm) * 0.5 - (b.x_mm + b.y_mm + (b.width_mm + b.depth_mm) * 0.5)
  );
  return primitives;
}

/**
 * 2D point-in-polygon helper for screen-space 3D bounding-box hit testing.
 */
function pointInScreenPoly(sx, sy, pts) {
  let inside = false;
  for (let i = 0, j = pts.length - 1; i < pts.length; j = i++) {
    const xi = pts[i].sx,
      yi = pts[i].sy;
    const xj = pts[j].sx,
      yj = pts[j].sy;
    const intersect =
      yi > sy !== yj > sy && sx < ((xj - xi) * (sy - yi)) / (yj - yi + 1e-9) + xi;
    if (intersect) inside = !inside;
  }
  return inside;
}

/**
 * Hit-tests the packed 3D primitives from front-to-back at canvas coordinates `(sx, sy)`.
 * Checks both the 3D extruded roof/walls and the floor footprint so clicking any part of a 3D machine selects it.
 *
 * @param {Array<object>} primitives
 * @param {number} sx
 * @param {number} sy
 * @param {object} camera
 * @returns {object | null}
 */
export function hitTestPrimitiveAtScreen(primitives, sx, sy, camera) {
  // Iterate front-to-back (reverse of painter's draw order)
  for (let i = primitives.length - 1; i >= 0; i--) {
    const p = primitives[i];
    const x0 = p.x_mm,
      y0 = p.y_mm,
      x1 = p.x_mm + p.width_mm,
      y1 = p.y_mm + p.depth_mm,
      z1 = p.height_mm;

    const b0 = project3DPoint(x0, y0, 0, camera);
    const b1 = project3DPoint(x1, y0, 0, camera);
    const b2 = project3DPoint(x1, y1, 0, camera);
    const b3 = project3DPoint(x0, y1, 0, camera);

    const t0 = project3DPoint(x0, y0, z1, camera);
    const t1 = project3DPoint(x1, y0, z1, camera);
    const t2 = project3DPoint(x1, y1, z1, camera);
    const t3 = project3DPoint(x0, y1, z1, camera);

    if (
      pointInScreenPoly(sx, sy, [t0, t1, t2, t3]) ||
      pointInScreenPoly(sx, sy, [b0, b1, b2, b3]) ||
      pointInScreenPoly(sx, sy, [b3, b2, t2, t3]) ||
      pointInScreenPoly(sx, sy, [b1, b2, t2, t1]) ||
      pointInScreenPoly(sx, sy, [b0, b3, t3, t0])
    ) {
      return p;
    }
  }
  return null;
}

/**
 * Central OOP state class for the Factory Sim client (`AGENTS.md` architecture).
 * Holds:
 * (a) Mirror of server estimated state (`snapshot`, `catalogs`, `gamesList`, `currentUser`),
 *     mutated only when server responses/updates arrive.
 * (b) UI-local state (`activeTab`, `selectedPlantId`, `selectedEntityId`, `planningMode`,
 *     `plannedCells`, `undoStack`, `camera`, `unitSystem`, `currency`).
 */
export class GameClient {
  constructor() {
    this.version = 0;
    this.listeners = new Set();

    // Server-mirrored state
    this.currentUser = null;
    this.sessionToken = null;
    this.catalogs = { buildings: [], cells: [], markets: [], certifications: [] };
    this.gamesList = [];
    this.snapshot = null;

    // UI-local state
    this.activeTab = 'Plants'; // 'Plants' | 'Markets' | 'Floor' | 'Timeline' | 'Ledger' | 'People'
    this.selectedPlantId = null;
    this.selectedEntityId = null;
    this.planningMode = true; // Default ON so placing from the palette drops instant draggable 3D ghosts!
    this.plannedCells = []; // controller-side planning ghosts
    this.undoStack = []; // snapshots of `plannedCells` for undo in planning mode
    this.unitSystem = 'metric'; // 'metric' | 'imperial'
    this.currency = 'USD'; // 'USD' | 'EUR'
    // True 3D isometric orbit camera
    this.camera = {
      panX: 340,
      panY: 110,
      zoom: 0.025,
      yawRad: 0.52,
      pitchRad: 0.85,
    };
    this.statusBanner = null;
  }

  subscribe(listener) {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  notify() {
    this.version += 1;
    for (const fn of this.listeners) {
      fn(this.version);
    }
  }

  applyServerSnapshot(snap) {
    this.snapshot = snap;
    const plantIds = Object.keys((snap && snap.world && snap.world.plants) || {});
    if (!this.selectedPlantId && plantIds.length > 0) {
      this.selectedPlantId = plantIds[0];
    }
    if (plantIds.length === 0) {
      this.activeTab = 'Plants';
    }
    this.revalidateAllPlannedCells();
    this.notify();
  }

  currentBuildingListing() {
    if (!this.snapshot?.world || !this.selectedPlantId) return null;
    const plant = this.snapshot.world.plants?.[this.selectedPlantId];
    if (!plant) return null;
    return (this.catalogs.buildings || []).find((b) => b.listing_id === plant.listing_id) || null;
  }

  revalidateAllPlannedCells() {
    const listing = this.currentBuildingListing();
    const installed = Object.values(this.snapshot?.world?.cells || {});
    for (const draft of this.plannedCells) {
      const res = validateDraftLocally(
        draft,
        listing,
        installed,
        this.catalogs.cells || [],
        this.plannedCells
      );
      draft.valid = res.valid;
      draft.errors = res.errors;
    }
  }

  setTab(tabName) {
    this.activeTab = tabName;
    this.notify();
  }

  selectPlant(plantId) {
    this.selectedPlantId = plantId;
    this.revalidateAllPlannedCells();
    this.notify();
  }

  selectEntity(entityId) {
    this.selectedEntityId = entityId;
    this.notify();
  }

  togglePlanningMode(enabled) {
    this.planningMode = enabled ?? !this.planningMode;
    this.notify();
  }

  addPlannedCellDraft(catalogEntry, x_mm, y_mm, rotation_deg = 0) {
    if (!this.selectedPlantId) return null;
    this.undoStack.push(this.plannedCells.map((c) => ({ ...c })));
    const draftId = `plan-${Date.now().toString(36)}-${this.plannedCells.length + 1}`;
    const draft = {
      draft_id: draftId,
      plant_id: this.selectedPlantId,
      catalog_id: catalogEntry.catalog_id,
      team: catalogEntry.team,
      display_name: catalogEntry.display_name,
      price_tenth_cents: catalogEntry.price,
      width_mm: catalogEntry.width_mm,
      depth_mm: catalogEntry.depth_mm,
      height_mm: catalogEntry.height_mm,
      clearance_mm: catalogEntry.clearance_mm ?? 300,
      x_mm: Math.round(x_mm / 100) * 100,
      y_mm: Math.round(y_mm / 100) * 100,
      rotation_deg,
      addons: [],
      valid: true,
      errors: [],
    };
    this.plannedCells.push(draft);
    this.revalidateAllPlannedCells();
    this.selectedEntityId = draftId;
    this.notify();
    return draft;
  }

  movePlannedCellDraft(draftId, x_mm, y_mm) {
    const draft = this.plannedCells.find((d) => d.draft_id === draftId);
    if (!draft) return false;
    draft.x_mm = Math.max(0, Math.round(x_mm / 100) * 100);
    draft.y_mm = Math.max(0, Math.round(y_mm / 100) * 100);
    this.revalidateAllPlannedCells();
    return true;
  }

  rotatePlannedCellDraft(draftId) {
    const draft = this.plannedCells.find((d) => d.draft_id === draftId);
    if (!draft) return false;
    this.undoStack.push(this.plannedCells.map((c) => ({ ...c })));
    draft.rotation_deg = ((draft.rotation_deg || 0) + 90) % 360;
    this.revalidateAllPlannedCells();
    this.notify();
    return true;
  }

  removePlannedCellDraft(draftId) {
    const idx = this.plannedCells.findIndex((d) => d.draft_id === draftId);
    if (idx < 0) return false;
    this.undoStack.push(this.plannedCells.map((c) => ({ ...c })));
    this.plannedCells.splice(idx, 1);
    if (this.selectedEntityId === draftId) {
      this.selectedEntityId = null;
    }
    this.revalidateAllPlannedCells();
    this.notify();
    return true;
  }

  undoPlannedChange() {
    if (this.undoStack.length === 0) return false;
    this.plannedCells = this.undoStack.pop();
    this.revalidateAllPlannedCells();
    this.notify();
    return true;
  }

  clearPlannedCells() {
    if (this.plannedCells.length > 0) {
      this.undoStack.push(this.plannedCells.map((c) => ({ ...c })));
      this.plannedCells = [];
      this.notify();
    }
  }

  plannedTotalCostTenthCents() {
    return this.plannedCells.reduce((acc, c) => acc + (c.price_tenth_cents || 0), 0);
  }
}

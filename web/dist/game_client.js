// Pure helper functions and central OOP `GameClient` state class for Factory Sim.
// Follows AGENTS.md:
// - The server owns simulation state; the browser never simulates.
// - One central OOP state class (`GameClient`) holds a mirror of the server's estimated state
//   plus UI-local state (active tab, selected plant, selected entity, planning mode drafts,
//   undo stack, camera, unit/currency settings).
// - Complex logic is factored into pure, unit-tested functions (`poseAt`, `formatMoney`,
//   `formatDimension`, `projectIso3D`, `packScenePrimitives`).

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
 * Pure 3D top-down / tilted isometric projection from millimeter world coordinates
 * `(x_mm, y_mm, z_mm)` to 2D canvas screen coordinates `(sx, sy)`.
 * @param {number} x_mm
 * @param {number} y_mm
 * @param {number} z_mm
 * @param {{panX: number, panY: number, zoom: number, tilt: number}} camera
 * @returns {{sx: number, sy: number}}
 */
export function project3DPoint(x_mm, y_mm, z_mm, camera) {
  const scale = camera.zoom || 0.042;
  const tilt = camera.tilt ?? 0.55; // 1.0 = flat top-down, 0.55 = tilted 3D view
  const sx = camera.panX + x_mm * scale;
  const sy = camera.panY + y_mm * scale * tilt - z_mm * scale * (1.0 - tilt * 0.4);
  return { sx, sy };
}

/**
 * Pure scene buffer packer that converts the server's estimated `ClientGameSnapshot` and
 * any local planning-mode ghost cells into sorted 3D extruded draw primitives for the canvas.
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
    primitives.push({
      id: n.node_id,
      kind: 'cell_or_square',
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
    });
  }

  // Add planning-mode ghost drafts with validity coloring
  for (const draft of plannedDrafts) {
    if (draft.plant_id !== plantId) continue;
    primitives.push({
      id: draft.draft_id,
      kind: 'planned_ghost',
      x_mm: draft.x_mm,
      y_mm: draft.y_mm,
      z_mm: 0,
      width_mm: draft.width_mm,
      depth_mm: draft.depth_mm,
      height_mm: draft.height_mm,
      color: draft.valid ? '#38bdf8' : '#ef4444',
      label: `[PLANNED] ${draft.display_name}`,
      ghost: true,
      valid: draft.valid,
    });
  }

  // Add interpolated worker cylinders (`poseAt`)
  const workers = Object.values(snapshot.world.workers || {});
  const tMs = renderNowMs || snapshot.world.now || 0;
  for (const w of workers) {
    if (w.plant_id !== plantId) continue;
    const pose = poseAt(w.motion_segments || [], tMs) || w.current_pose || { x_mm: 1500, y_mm: 1500 };
    primitives.push({
      id: `worker-${w.worker_id}`,
      kind: 'worker',
      x_mm: pose.x_mm - 250,
      y_mm: pose.y_mm - 250,
      z_mm: 0,
      width_mm: 500,
      depth_mm: 500,
      height_mm: 1750,
      color: w.active_task ? '#10b981' : '#f8fafc',
      label: `${w.name} (${w.active_task ? w.active_task.task.category : 'Idle on iPad'})`,
      ghost: false,
      valid: true,
    });
  }

  // Sort back-to-front by `y_mm + depth_mm` for painter's algorithm 3D extrusion
  primitives.sort((a, b) => a.y_mm + a.depth_mm - (b.y_mm + b.depth_mm));
  return primitives;
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
    this.planningMode = false;
    this.plannedCells = []; // controller-side planning ghosts
    this.undoStack = []; // snapshots of `plannedCells` for undo in planning mode
    this.unitSystem = 'metric'; // 'metric' | 'imperial'
    this.currency = 'USD'; // 'USD' | 'EUR'
    this.camera = { panX: 60, panY: 70, zoom: 0.042, tilt: 0.62 };
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
    this.notify();
  }

  setTab(tabName) {
    this.activeTab = tabName;
    this.notify();
  }

  selectPlant(plantId) {
    this.selectedPlantId = plantId;
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

  addPlannedCellDraft(catalogEntry, x_mm, y_mm, rotation_deg = 0, valid = true, errors = []) {
    if (!this.selectedPlantId) return;
    this.undoStack.push(this.plannedCells.map((c) => ({ ...c })));
    const draftId = `plan-${this.plannedCells.length + 1}-${catalogEntry.catalog_id}`;
    this.plannedCells.push({
      draft_id: draftId,
      plant_id: this.selectedPlantId,
      catalog_id: catalogEntry.catalog_id,
      team: catalogEntry.team,
      display_name: catalogEntry.display_name,
      price_tenth_cents: catalogEntry.price,
      width_mm: catalogEntry.width_mm,
      depth_mm: catalogEntry.depth_mm,
      height_mm: catalogEntry.height_mm,
      x_mm,
      y_mm,
      rotation_deg,
      addons: [],
      valid,
      errors,
    });
    this.selectedEntityId = draftId;
    this.notify();
  }

  undoPlannedChange() {
    if (this.undoStack.length === 0) return false;
    this.plannedCells = this.undoStack.pop();
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

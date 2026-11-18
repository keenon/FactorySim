// Interactive perspective 3D view of the selected plant (game-style camera, shadows, picking,
// drag-to-place). Rendering stays outside React: this class owns a persistent WebGL renderer
// and re-syncs from `GameClient` whenever `client.sceneVersion` changes.
//
// Controls: left-drag empty space = orbit, right-drag = pan, wheel = zoom, WASD/arrows = move,
// left-drag a ghost or machine = move it on the floor, click = select.

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { CSS2DObject, CSS2DRenderer } from 'three/addons/renderers/CSS2DRenderer.js';
import type { GameClient } from '../state/GameClient';
import { chevronTriangles, plantRoutes, ribbonTriangles, routeRole, type RouteRole } from '../logic/flows';
import { poseAt } from '../logic/motion';
import { polygonBounds } from '../logic/placement';
import type { BuildingListing, CellRunState, LayoutRoute, SceneNode, WorkerState } from '../types';
import { cellTransform, MM, originFromCenter } from './coords';
import { buildCellModel, buildFallbackBox, buildFootprintOutline, type GhostStyle } from './modelBuilder';

type PickKind = 'cell' | 'draft' | 'square' | 'worker';

interface Entity {
  root: THREE.Group;
  modelKey: string;
  kind: PickKind;
  entityId: string;
  label?: CSS2DObject;
}

const SPIN_NODES = new Set(['spindle', 'chuck']);

export class FloorScene {
  private readonly renderer: THREE.WebGLRenderer;
  private readonly labelRenderer: CSS2DRenderer;
  private readonly scene = new THREE.Scene();
  private readonly camera = new THREE.PerspectiveCamera(48, 1, 0.1, 600);
  private readonly controls: OrbitControls;
  private readonly raycaster = new THREE.Raycaster();
  private readonly floorPlane = new THREE.Plane(new THREE.Vector3(0, 0, 1), 0);
  private readonly buildingGroup = new THREE.Group();
  private readonly entityGroup = new THREE.Group();
  private readonly entities = new Map<string, Entity>();
  private readonly routeGroup = new THREE.Group();
  private routesKey = '';
  private workers: WorkerState[] = [];
  private labelShown = new Set<string>();
  private readonly selectionBox = new THREE.Box3Helper(new THREE.Box3(), new THREE.Color('#3f8f84'));
  private readonly hoverBox = new THREE.Box3Helper(new THREE.Box3(), new THREE.Color('#9cc9c2'));
  private readonly sun: THREE.DirectionalLight;

  private container: HTMLElement | null = null;
  private resizeObserver: ResizeObserver | null = null;
  private frameHandle = 0;
  private lastSceneVersion = -1;
  private buildingKey = '';
  private runningCells = new Set<string>();
  private keys = new Set<string>();
  private lastFrameTime = performance.now();
  private hoveredId: string | null = null;
  private hoveredPickId: string | null = null;
  private pointerNdc: THREE.Vector2 | null = null;
  private drag: null | {
    kind: 'draft' | 'cell';
    id: string;
    offset: THREE.Vector2;
    widthMm: number;
    depthMm: number;
    rotationDeg: number;
    moved: boolean;
  } = null;
  private pointerDownAt: { x: number; y: number } | null = null;

  constructor(private readonly client: GameClient) {
    this.renderer = new THREE.WebGLRenderer({ antialias: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.05;
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.domElement.tabIndex = 0;
    this.renderer.domElement.className = 'floor-canvas';

    this.labelRenderer = new CSS2DRenderer();
    this.labelRenderer.domElement.className = 'floor-labels';

    this.scene.background = new THREE.Color('#e8eef0');
    this.scene.fog = new THREE.Fog('#e8eef0', 60, 160);
    this.camera.up.set(0, 0, 1);
    this.camera.position.set(-6, -14, 14);

    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.08;
    this.controls.screenSpacePanning = false;
    this.controls.maxPolarAngle = Math.PI * 0.47;
    this.controls.minDistance = 3;
    this.controls.maxDistance = 120;
    this.controls.mouseButtons = { LEFT: THREE.MOUSE.ROTATE, MIDDLE: THREE.MOUSE.DOLLY, RIGHT: THREE.MOUSE.PAN };

    // Lighting: soft sky fill + warm sun with shadows.
    this.scene.add(new THREE.HemisphereLight('#ffffff', '#d9d2c5', 1.15));
    this.sun = new THREE.DirectionalLight('#fff4e0', 2.0);
    this.sun.castShadow = true;
    this.sun.shadow.mapSize.set(2048, 2048);
    this.sun.shadow.bias = -0.0004;
    this.sun.shadow.normalBias = 0.02;
    this.scene.add(this.sun, this.sun.target);

    // Ground outside the building.
    const ground = new THREE.Mesh(new THREE.PlaneGeometry(400, 400), new THREE.MeshStandardMaterial({ color: '#dbe3d3', roughness: 1 }));
    ground.position.z = -0.02;
    ground.receiveShadow = true;
    this.scene.add(ground);

    this.selectionBox.visible = false;
    this.hoverBox.visible = false;
    this.scene.add(this.buildingGroup, this.routeGroup, this.entityGroup, this.selectionBox, this.hoverBox);

    this.bindInput();
  }

  // ---------- lifecycle ----------
  mount(container: HTMLElement): void {
    this.container = container;
    container.appendChild(this.renderer.domElement);
    container.appendChild(this.labelRenderer.domElement);
    this.resizeObserver = new ResizeObserver(() => this.resize());
    this.resizeObserver.observe(container);
    this.resize();
    this.lastSceneVersion = -1;
    this.lastFrameTime = performance.now();
    this.frameHandle = requestAnimationFrame(this.frame);
  }

  unmount(): void {
    cancelAnimationFrame(this.frameHandle);
    this.resizeObserver?.disconnect();
    this.resizeObserver = null;
    this.renderer.domElement.remove();
    this.labelRenderer.domElement.remove();
    this.container = null;
    this.keys.clear();
  }

  resetView(mode: 'game' | 'top'): void {
    const listing = this.client.currentListing();
    const b = listing ? polygonBounds(listing.floor_polygon.outer) : { minX: 0, minY: 0, maxX: 20000, maxY: 15000 };
    const cx = ((b.minX + b.maxX) / 2) * MM;
    const cy = ((b.minY + b.maxY) / 2) * MM;
    const span = Math.max(b.maxX - b.minX, b.maxY - b.minY) * MM;
    this.controls.target.set(cx, cy, 0);
    if (mode === 'top') this.camera.position.set(cx, cy - 0.01, span * 1.25);
    else this.camera.position.set(cx - span * 0.35, cy - span * 0.85, span * 0.6);
    this.controls.update();
  }

  private resize(): void {
    if (!this.container) return;
    const w = Math.max(1, this.container.clientWidth);
    const h = Math.max(1, this.container.clientHeight);
    this.renderer.setSize(w, h);
    this.labelRenderer.setSize(w, h);
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
  }

  // ---------- frame loop ----------
  private frame = (t: number): void => {
    this.frameHandle = requestAnimationFrame(this.frame);
    const dt = Math.min(0.1, (t - this.lastFrameTime) / 1000);
    this.lastFrameTime = t;

    if (this.client.sceneVersion !== this.lastSceneVersion) {
      this.lastSceneVersion = this.client.sceneVersion;
      this.sync();
    }
    this.applyKeyboardPan(dt);
    this.animate(dt);
    this.updateWorkers(dt * 1000, t);
    this.updateHover();
    this.updateLabels();
    this.controls.update();
    this.renderer.render(this.scene, this.camera);
    this.labelRenderer.render(this.scene, this.camera);
  };

  private applyKeyboardPan(dt: number): void {
    if (this.keys.size === 0) return;
    const forward = new THREE.Vector3().subVectors(this.controls.target, this.camera.position).setZ(0).normalize();
    const right = new THREE.Vector3(forward.y, -forward.x, 0);
    const speed = Math.max(4, this.camera.position.distanceTo(this.controls.target)) * 0.9 * dt;
    const move = new THREE.Vector3();
    if (this.keys.has('w') || this.keys.has('arrowup')) move.add(forward);
    if (this.keys.has('s') || this.keys.has('arrowdown')) move.sub(forward);
    if (this.keys.has('d') || this.keys.has('arrowright')) move.add(right);
    if (this.keys.has('a') || this.keys.has('arrowleft')) move.sub(right);
    if (move.lengthSq() === 0) return;
    move.normalize().multiplyScalar(speed);
    this.camera.position.add(move);
    this.controls.target.add(move);
  }

  private animate(dt: number): void {
    for (const id of this.runningCells) {
      const e = this.entities.get(id);
      e?.root.traverse((o) => {
        if (!SPIN_NODES.has(o.name)) return;
        // Spin about the cylinder's own axis (local +Y of CylinderGeometry).
        o.rotateY(dt * 18);
      });
    }
  }

  // ---------- sync from client state ----------
  private sync(): void {
    const snap = this.client.snapshot;
    const plantId = this.client.selectedPlantId;
    const listing = this.client.currentListing();
    if (!snap || !plantId || !listing) {
      this.clearEntities();
      this.clearRoutes();
      return;
    }
    this.syncBuilding(listing);

    const statusById = new Map(snap.cell_statuses.map((s) => [s.cell_id, s.state] as const));
    this.runningCells = new Set();
    const seen = new Set<string>();
    const nodes = snap.scene_nodes_by_plant[plantId] ?? [];

    for (const node of nodes) {
      if (node.node_id.startsWith('cell-')) {
        const cellId = node.node_id.slice(5);
        const state = statusById.get(cellId) ?? 'Idle';
        if (state === 'Running') this.runningCells.add(node.node_id);
        this.syncCell(node, cellId, state);
        seen.add(node.node_id);
      } else if (node.node_id.startsWith('sq-')) {
        this.syncSquare(node);
        seen.add(node.node_id);
      }
    }

    for (const d of this.client.drafts) {
      if (d.plant_id !== plantId) continue;
      const style: GhostStyle = d.errors.length > 0 ? 'invalid' : 'valid';
      const key = `${d.catalog_id}|${style}`;
      const ent = this.ensureEntity(d.draft_id, key, 'draft', d.draft_id, () => {
        const model = this.client.catalogs.preview_models[d.catalog_id];
        const g = model ? buildCellModel(model, style) : buildFallbackBox(d.width_mm, d.depth_mm, d.height_mm, '#b8c1c8', style);
        g.add(buildFootprintOutline(d.width_mm, d.depth_mm, d.clearance_mm, style === 'invalid' ? '#d0604f' : '#3f8f84'));
        return g;
      }, `Planned · ${d.display_name}`);
      const tf = cellTransform(d.x_mm, d.y_mm, d.width_mm, d.depth_mm, d.rotation_deg);
      ent.root.position.set(tf.cx, tf.cy, 0);
      ent.root.rotation.set(0, 0, tf.yaw);
      seen.add(d.draft_id);
    }

    this.workers = Object.values(snap.world.workers).filter((w) => w.plant_id === plantId);
    for (const w of this.workers) {
      const id = `worker-${w.worker_id}`;
      const busy = w.active_task !== null;
      this.ensureEntity(id, busy ? `busy:${w.active_task!.task.category}` : 'idle', 'worker', w.worker_id, () => buildWorker(busy), `${w.name}${busy ? ` · ${w.active_task!.task.category}` : ''}`);
      seen.add(id);
    }
    this.updateWorkers(0, performance.now());
    this.syncRoutes(plantId, snap.world.transport_graphs[plantId]?.layout_version ?? 0);

    for (const [id, ent] of this.entities) {
      if (!seen.has(id)) this.removeEntity(id, ent);
    }
    this.updateSelectionBox();
  }

  /** Places workers at the client's smoothly advancing display time; bobs them while walking. */
  private updateWorkers(dtWallMs: number, wallMs: number): void {
    const t = this.client.advanceDisplayTime(dtWallMs);
    for (const w of this.workers) {
      const ent = this.entities.get(`worker-${w.worker_id}`);
      if (!ent) continue;
      const pose = poseAt(w.motion_segments, t) ?? w.current_pose;
      const walking = w.motion_segments.some((s) => t > s.start_ms && t < s.end_ms && (s.start_pose.x_mm !== s.end_pose.x_mm || s.start_pose.y_mm !== s.end_pose.y_mm));
      ent.root.position.set(pose.x_mm * MM, pose.y_mm * MM, walking ? Math.abs(Math.sin(wallMs / 110)) * 0.05 : 0);
      ent.root.rotation.set(0, 0, (pose.heading_mdeg / 1000) * (Math.PI / 180));
    }
  }

  /** Material-flow routes as subtle floor ribbons; routes into/out of the selected cell are highlighted. */
  private syncRoutes(plantId: string, layoutVersion: number): void {
    const selected = this.client.selectedEntityId;
    const key = `${plantId}|${layoutVersion}|${selected ?? ''}`;
    if (key === this.routesKey) return;
    this.routesKey = key;
    this.clearRoutes();
    const routes = plantRoutes(this.client.world, plantId);
    // Draw idle first so highlighted ribbons layer on top.
    const ordered = [...routes].sort((a, b) => Number(routeRole(a, selected) !== 'idle') - Number(routeRole(b, selected) !== 'idle'));
    for (const r of ordered) this.routeGroup.add(buildRoute(r, routeRole(r, selected)));
  }

  private clearRoutes(): void {
    for (const child of [...this.routeGroup.children]) {
      child.traverse((o) => {
        if (o instanceof THREE.Mesh) {
          o.geometry.dispose();
          (o.material as THREE.Material).dispose();
        }
      });
      child.removeFromParent();
    }
    this.routesKey = '';
  }

  private updateLabels(): void {
    const want = new Set<string>();
    if (this.hoveredPickId) want.add(this.hoveredPickId);
    const sel = this.client.selectedEntityId;
    if (sel) for (const [id, e] of this.entities) if (e.entityId === sel && e.kind !== 'square') want.add(id);
    for (const id of this.labelShown) {
      if (!want.has(id)) {
        const l = this.entities.get(id)?.label;
        if (l) l.visible = false;
      }
    }
    for (const id of want) {
      const l = this.entities.get(id)?.label;
      if (l) l.visible = true;
    }
    this.labelShown = want;
  }

  private syncCell(node: SceneNode, cellId: string, state: CellRunState): void {
    const preview = this.client.cellDragPreview?.cellId === cellId ? this.client.cellDragPreview : null;
    const style: GhostStyle = preview ? 'valid' : node.ghost ? 'installing' : 'none';
    const key = `${JSON.stringify(node.model)}|${node.color_hex}|${style}|${node.width_mm}x${node.depth_mm}x${node.height_mm}`;
    const ent = this.ensureEntity(node.node_id, key, 'cell', cellId, () =>
      node.model ? buildCellModel(node.model, style) : buildFallbackBox(node.width_mm, node.depth_mm, node.height_mm, node.color_hex, style),
    `${node.label}`);
    if (ent.label) ent.label.element.dataset.state = state;
    const x = preview?.x_mm ?? node.x_mm;
    const y = preview?.y_mm ?? node.y_mm;
    const tf = cellTransform(x, y, node.width_mm, node.depth_mm, node.rotation_deg);
    ent.root.position.set(tf.cx, tf.cy, 0);
    ent.root.rotation.set(0, 0, tf.yaw);
  }

  private syncSquare(node: SceneNode): void {
    const occupied = node.height_mm > 100;
    const key = `${node.color_hex}|${occupied}`;
    const ent = this.ensureEntity(node.node_id, key, 'square', node.node_id.slice(3), () => buildSquare(node.color_hex, occupied));
    ent.root.position.set((node.x_mm + node.width_mm / 2) * MM, (node.y_mm + node.depth_mm / 2) * MM, 0);
  }

  private ensureEntity(id: string, modelKey: string, kind: PickKind, entityId: string, build: () => THREE.Group, label?: string): Entity {
    const existing = this.entities.get(id);
    if (existing && existing.modelKey === modelKey) return existing;
    if (existing) this.removeEntity(id, existing);
    const root = build();
    root.traverse((o) => {
      o.userData.pickId = id;
    });
    const ent: Entity = { root, modelKey, kind, entityId };
    if (label) {
      const el = document.createElement('div');
      el.className = `scene-label scene-label-${kind}`;
      el.textContent = label;
      const obj = new CSS2DObject(el);
      obj.visible = false; // names show only on hover / selection
      const box = new THREE.Box3().setFromObject(root);
      obj.position.set(0, 0, (box.isEmpty() ? 1 : box.max.z) + 0.35);
      root.add(obj);
      ent.label = obj;
    }
    this.entityGroup.add(root);
    this.entities.set(id, ent);
    return ent;
  }

  private removeEntity(id: string, ent: Entity): void {
    ent.label?.element.remove();
    ent.root.removeFromParent();
    // Outline helpers own their geometries; shared model geometries/materials are cached.
    ent.root.traverse((o) => {
      if (o instanceof THREE.Line || (o instanceof THREE.Mesh && o.geometry instanceof THREE.PlaneGeometry)) {
        o.geometry.dispose();
        (o.material as THREE.Material).dispose();
      }
    });
    this.entities.delete(id);
  }

  private clearEntities(): void {
    for (const [id, ent] of this.entities) this.removeEntity(id, ent);
  }

  private syncBuilding(listing: BuildingListing): void {
    if (listing.listing_id === this.buildingKey) return;
    this.buildingKey = listing.listing_id;
    this.buildingGroup.clear();
    this.buildingGroup.add(buildBuilding(listing));

    const b = polygonBounds(listing.floor_polygon.outer);
    const cx = ((b.minX + b.maxX) / 2) * MM;
    const cy = ((b.minY + b.maxY) / 2) * MM;
    const span = Math.max(b.maxX - b.minX, b.maxY - b.minY) * MM;
    this.sun.position.set(cx - span * 0.6, cy - span * 0.9, span * 1.4);
    this.sun.target.position.set(cx, cy, 0);
    const cam = this.sun.shadow.camera;
    cam.left = -span;
    cam.right = span;
    cam.top = span;
    cam.bottom = -span;
    cam.near = 1;
    cam.far = span * 4;
    cam.updateProjectionMatrix();
    this.resetView('game');
  }

  private updateSelectionBox(): void {
    const sel = this.client.selectedEntityId;
    const ent = sel ? [...this.entities.values()].find((e) => e.entityId === sel && e.kind !== 'square') : undefined;
    if (ent) {
      this.selectionBox.box.setFromObject(ent.root);
      this.selectionBox.box.expandByScalar(0.05);
      this.selectionBox.visible = true;
    } else {
      this.selectionBox.visible = false;
    }
  }

  // ---------- input ----------
  private bindInput(): void {
    const el = this.renderer.domElement;
    el.addEventListener('contextmenu', (e) => e.preventDefault());
    el.addEventListener('pointerdown', this.onPointerDown, { capture: true });
    el.addEventListener('pointermove', this.onPointerMove);
    el.addEventListener('pointerup', this.onPointerUp);
    el.addEventListener('pointerleave', () => {
      this.pointerNdc = null;
    });
    el.addEventListener('keydown', (e) => {
      const k = e.key.toLowerCase();
      if (['w', 'a', 's', 'd', 'arrowup', 'arrowdown', 'arrowleft', 'arrowright'].includes(k)) {
        this.keys.add(k);
        e.preventDefault();
        e.stopPropagation();
      }
    });
    el.addEventListener('keyup', (e) => this.keys.delete(e.key.toLowerCase()));
    el.addEventListener('blur', () => this.keys.clear());
  }

  private ndc(e: PointerEvent): THREE.Vector2 {
    const r = this.renderer.domElement.getBoundingClientRect();
    return new THREE.Vector2(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
  }

  private pick(ndc: THREE.Vector2): Entity | null {
    this.raycaster.setFromCamera(ndc, this.camera);
    const hits = this.raycaster.intersectObjects(this.entityGroup.children, true);
    for (const h of hits) {
      const id = h.object.userData.pickId as string | undefined;
      if (!id) continue;
      if (h.object instanceof THREE.Line) continue;
      const ent = this.entities.get(id);
      if (ent) return ent;
    }
    return null;
  }

  private floorPoint(ndc: THREE.Vector2): THREE.Vector3 | null {
    this.raycaster.setFromCamera(ndc, this.camera);
    return this.raycaster.ray.intersectPlane(this.floorPlane, new THREE.Vector3());
  }

  private onPointerDown = (e: PointerEvent): void => {
    this.renderer.domElement.focus();
    this.pointerDownAt = { x: e.clientX, y: e.clientY };
    if (e.button !== 0) return;
    const ndc = this.ndc(e);
    const ent = this.pick(ndc);
    if (!ent || (ent.kind !== 'draft' && ent.kind !== 'cell')) return;
    const floor = this.floorPoint(ndc);
    if (!floor) return;

    let widthMm: number, depthMm: number, rotationDeg: number;
    if (ent.kind === 'draft') {
      const d = this.client.drafts.find((x) => x.draft_id === ent.entityId);
      if (!d) return;
      [widthMm, depthMm, rotationDeg] = [d.width_mm, d.depth_mm, d.rotation_deg];
    } else {
      const cell = this.client.world?.cells[ent.entityId];
      const cat = cell ? this.client.catalogEntry(cell.catalog_id) : undefined;
      if (!cell || !cat) return;
      [widthMm, depthMm, rotationDeg] = [cat.width_mm, cat.depth_mm, cell.rotation_deg];
    }
    // Stop OrbitControls from starting a rotate for this gesture.
    e.stopImmediatePropagation();
    this.controls.enabled = false;
    this.renderer.domElement.setPointerCapture(e.pointerId);
    this.drag = {
      kind: ent.kind,
      id: ent.entityId,
      offset: new THREE.Vector2(floor.x - ent.root.position.x, floor.y - ent.root.position.y),
      widthMm,
      depthMm,
      rotationDeg,
      moved: false,
    };
    if (ent.kind === 'draft') this.client.beginDraftDrag();
    this.client.select(ent.entityId);
  };

  private onPointerMove = (e: PointerEvent): void => {
    this.pointerNdc = this.ndc(e);
    if (!this.drag) return;
    const floor = this.floorPoint(this.pointerNdc);
    if (!floor) return;
    const o = originFromCenter(floor.x - this.drag.offset.x, floor.y - this.drag.offset.y, this.drag.widthMm, this.drag.depthMm, this.drag.rotationDeg);
    this.drag.moved = true;
    if (this.drag.kind === 'draft') this.client.moveDraftLive(this.drag.id, o.x_mm, o.y_mm);
    else this.client.previewCellMove(this.drag.id, o.x_mm, o.y_mm);
  };

  private onPointerUp = (e: PointerEvent): void => {
    const down = this.pointerDownAt;
    this.pointerDownAt = null;
    const isClick = down !== null && Math.hypot(e.clientX - down.x, e.clientY - down.y) < 5;

    if (this.drag) {
      const drag = this.drag;
      this.drag = null;
      this.controls.enabled = true;
      this.renderer.domElement.releasePointerCapture(e.pointerId);
      if (drag.kind === 'draft') void this.client.endDraftDrag(drag.id);
      else if (drag.moved) void this.client.commitCellMove();
      else this.client.select(drag.id);
      return;
    }
    if (isClick && e.button === 0) {
      const ent = this.pick(this.ndc(e));
      this.client.select(ent ? ent.entityId : null);
    }
  };

  private updateHover(): void {
    if (!this.pointerNdc || this.drag) {
      this.hoverBox.visible = false;
      this.hoveredPickId = null;
      return;
    }
    const ent = this.pick(this.pointerNdc);
    this.hoveredPickId = ent && ent.kind !== 'square' ? (ent.root.userData.pickId as string) : null;
    const id = ent && ent.kind !== 'square' ? ent.entityId : null;
    this.renderer.domElement.style.cursor = ent && (ent.kind === 'draft' || ent.kind === 'cell') ? 'grab' : 'default';
    if (id === this.hoveredId && !id) return;
    this.hoveredId = id;
    if (ent && id && id !== this.client.selectedEntityId) {
      this.hoverBox.box.setFromObject(ent.root);
      this.hoverBox.box.expandByScalar(0.03);
      this.hoverBox.visible = true;
    } else {
      this.hoverBox.visible = false;
    }
  }
}

// ---------- static scene pieces ----------

function buildBuilding(listing: BuildingListing): THREE.Group {
  const g = new THREE.Group();
  const outer = listing.floor_polygon.outer;
  const b = polygonBounds(outer);

  // Concrete slab
  const shape = new THREE.Shape(outer.map((p) => new THREE.Vector2(p.x * MM, p.y * MM)));
  for (const hole of listing.floor_polygon.holes) {
    shape.holes.push(new THREE.Path(hole.map((p) => new THREE.Vector2(p.x * MM, p.y * MM))));
  }
  const slab = new THREE.Mesh(
    new THREE.ExtrudeGeometry(shape, { depth: 0.1, bevelEnabled: false }),
    new THREE.MeshStandardMaterial({ color: '#efebe4', roughness: 0.92 }),
  );
  slab.position.z = -0.1;
  slab.receiveShadow = true;
  g.add(slab);

  // 1 m grid lines on the slab
  const pts: THREE.Vector3[] = [];
  for (let x = Math.ceil(b.minX / 1000) * 1000; x <= b.maxX; x += 1000) {
    pts.push(new THREE.Vector3(x * MM, b.minY * MM, 0.002), new THREE.Vector3(x * MM, b.maxY * MM, 0.002));
  }
  for (let y = Math.ceil(b.minY / 1000) * 1000; y <= b.maxY; y += 1000) {
    pts.push(new THREE.Vector3(b.minX * MM, y * MM, 0.002), new THREE.Vector3(b.maxX * MM, y * MM, 0.002));
  }
  g.add(new THREE.LineSegments(new THREE.BufferGeometry().setFromPoints(pts), new THREE.LineBasicMaterial({ color: '#d6cfc3', transparent: true, opacity: 0.8 })));

  // Walls: solid low wainscot + translucent upper wall, so the floor stays readable from any angle.
  const wallH = listing.clear_height_mm * MM;
  const t = 0.2;
  const lowMat = new THREE.MeshStandardMaterial({ color: '#c9d3cf', roughness: 0.8 });
  const highMat = new THREE.MeshStandardMaterial({ color: '#f4f1ea', transparent: true, opacity: 0.22, depthWrite: false, side: THREE.DoubleSide });
  for (let i = 0; i < outer.length; i++) {
    const a = outer[i];
    const c = outer[(i + 1) % outer.length];
    const len = Math.hypot(c.x - a.x, c.y - a.y) * MM;
    const ang = Math.atan2(c.y - a.y, c.x - a.x);
    const mx = ((a.x + c.x) / 2) * MM;
    const my = ((a.y + c.y) / 2) * MM;
    const low = new THREE.Mesh(new THREE.BoxGeometry(len + t, t, 1.0), lowMat);
    low.position.set(mx, my, 0.5);
    low.rotation.z = ang;
    low.castShadow = true;
    low.receiveShadow = true;
    const high = new THREE.Mesh(new THREE.BoxGeometry(len + t, t * 0.6, wallH - 1.0), highMat);
    high.position.set(mx, my, 1.0 + (wallH - 1.0) / 2);
    high.rotation.z = ang;
    g.add(low, high);
  }

  // Structural columns
  const colMat = new THREE.MeshStandardMaterial({ color: '#b9b2a6', roughness: 0.7 });
  for (const hole of listing.floor_polygon.holes) {
    const hb = polygonBounds(hole);
    const col = new THREE.Mesh(new THREE.BoxGeometry((hb.maxX - hb.minX) * MM, (hb.maxY - hb.minY) * MM, wallH), colMat);
    col.position.set(((hb.minX + hb.maxX) / 2) * MM, ((hb.minY + hb.maxY) / 2) * MM, wallH / 2);
    col.castShadow = true;
    col.receiveShadow = true;
    g.add(col);
  }
  return g;
}

const SQUARE_PAD = new THREE.BoxGeometry(0.42, 0.42, 0.02);
const CRATE = new THREE.BoxGeometry(0.34, 0.34, 0.26);

function buildSquare(color: string, occupied: boolean): THREE.Group {
  const g = new THREE.Group();
  const pad = new THREE.Mesh(SQUARE_PAD, new THREE.MeshStandardMaterial({ color, roughness: 0.6 }));
  pad.position.z = 0.01;
  pad.receiveShadow = true;
  g.add(pad);
  if (occupied) {
    const crate = new THREE.Mesh(CRATE, new THREE.MeshStandardMaterial({ color: '#d8b07a', roughness: 0.85 }));
    crate.position.z = 0.15;
    crate.castShadow = true;
    g.add(crate);
  }
  return g;
}

const BODY = new THREE.CapsuleGeometry(0.2, 0.9, 6, 12);
const HEAD = new THREE.SphereGeometry(0.15, 16, 12);
const VEST = new THREE.CylinderGeometry(0.215, 0.215, 0.35, 16);

function buildWorker(busy: boolean): THREE.Group {
  const g = new THREE.Group();
  const body = new THREE.Mesh(BODY, new THREE.MeshStandardMaterial({ color: '#5b6f86', roughness: 0.7 }));
  body.rotation.x = Math.PI / 2;
  body.position.z = 0.65;
  const vest = new THREE.Mesh(VEST, new THREE.MeshStandardMaterial({ color: busy ? '#e8a54b' : '#9cc9c2', roughness: 0.6 }));
  vest.rotation.x = Math.PI / 2;
  vest.position.z = 0.95;
  const head = new THREE.Mesh(HEAD, new THREE.MeshStandardMaterial({ color: '#e8c4a0', roughness: 0.6 }));
  head.position.z = 1.48;
  for (const m of [body, vest, head]) m.castShadow = true;
  g.add(body, vest, head);
  return g;
}

const ROUTE_STYLE: Record<RouteRole, { color: string; width: number; opacity: number; z: number }> = {
  idle: { color: '#a99f8f', width: 110, opacity: 0.38, z: 0.008 },
  in: { color: '#5d8fbf', width: 220, opacity: 0.9, z: 0.012 },
  out: { color: '#e09a45', width: 220, opacity: 0.9, z: 0.012 },
};

function flatMesh(positions: number[], color: string, opacity: number): THREE.Mesh {
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  const mat = new THREE.MeshBasicMaterial({ color, transparent: true, opacity, depthWrite: false, side: THREE.DoubleSide });
  const mesh = new THREE.Mesh(geo, mat);
  mesh.renderOrder = 1;
  return mesh;
}

function buildRoute(route: LayoutRoute, role: RouteRole): THREE.Group {
  const style = ROUTE_STYLE[role];
  const g = new THREE.Group();
  g.add(flatMesh(ribbonTriangles(route.waypoints, style.width, style.z), style.color, style.opacity));
  const chevrons = chevronTriangles(route.waypoints, role === 'idle' ? 2400 : 1000, role === 'idle' ? 110 : 190, style.z + 0.001);
  if (chevrons.length > 0) g.add(flatMesh(chevrons, role === 'idle' ? '#8f8577' : '#ffffff', role === 'idle' ? 0.45 : 0.95));
  return g;
}

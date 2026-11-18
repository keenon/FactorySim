// Central client state (AGENTS.md frontend rules):
// (a) a mirror of server state, replaced only by server responses, and
// (b) UI-local state (tab, selection, camera-independent planning drafts, settings).
// User actions are methods here; they send commands to the server and never mutate the mirror.
//
// Two change counters:
// - `version`: bumped by `notify()`; React re-renders via useSyncExternalStore.
// - `sceneVersion`: bumped on any change the 3D view must reflect. Live drags bump only this,
//   so dragging at 60 FPS never re-renders React.

import type { Api } from '../api';
import {
  findFreeSpot,
  previewPlacementErrors,
  snapMm,
  type PlacementDraft,
} from '../logic/placement';
import type { Currency, UnitSystem } from '../logic/format';
import { nextDisplayTime } from '../logic/clock';
import type {
  BuildingListing,
  CellCatalogEntry,
  ClientGameSnapshot,
  GameSummaryItem,
  PlantCommand,
  ProblemReportBundle,
  StaticCatalogBundle,
  UserAccount,
} from '../types';

export type TabName = 'Plants' | 'Markets' | 'Floor' | 'Timeline' | 'Ledger' | 'People';
export const TABS: readonly TabName[] = ['Plants', 'Markets', 'Floor', 'Timeline', 'Ledger', 'People'];

export interface Draft extends PlacementDraft {
  team: CellCatalogEntry['team'];
  price_tenth_cents: number;
  errors: string[];
}

export interface CellDragPreview {
  cellId: string;
  x_mm: number;
  y_mm: number;
}

const EMPTY_CATALOGS: StaticCatalogBundle = {
  buildings: [],
  cells: [],
  markets: [],
  certifications: [],
  preview_models: {},
};

export const SPEEDS = [1, 5, 30, 300] as const;
export type Speed = (typeof SPEEDS)[number];

export class GameClient {
  /** Wall-clock interval between server steps while playing. */
  static readonly TICK_MS = 250;

  // --- change tracking ---
  private listeners = new Set<() => void>();
  version = 0;
  sceneVersion = 0;

  // --- server mirror ---
  user: UserAccount | null = null;
  catalogs: StaticCatalogBundle = EMPTY_CATALOGS;
  games: GameSummaryItem[] = [];
  snapshot: ClientGameSnapshot | null = null;

  // --- UI-local state ---
  booting = true;
  busy = false;
  banner: string | null = null;
  activeTab: TabName = 'Plants';
  selectedPlantId: string | null = null;
  selectedEntityId: string | null = null;
  paletteTeam: CellCatalogEntry['team'] = 'Machining';
  drafts: Draft[] = [];
  private undoStack: Draft[][] = [];
  cellDragPreview: CellDragPreview | null = null;
  units: UnitSystem = 'metric';
  currency: Currency = 'USD';
  showSkipSummary = false;
  showProblemModal = false;
  lastProblemReport: ProblemReportBundle | null = null;
  generatedInvite: string | null = null;
  generatedReset: string | null = null;
  private draftSeq = 0;

  // --- play / time ---
  playing = false;
  speed: Speed = 5;
  /** Sim time the 3D view is currently drawing (smoothly interpolated while playing). */
  displayTimeMs = 0;
  private tickTimer: ReturnType<typeof setInterval> | null = null;
  private tickInFlight = false;

  constructor(private readonly api: Api) {}

  // ---------- subscription ----------
  subscribe = (fn: () => void): (() => void) => {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  };
  getVersion = (): number => this.version;

  notify(): void {
    this.version += 1;
    this.sceneVersion += 1;
    for (const fn of this.listeners) fn();
  }

  private touchScene(): void {
    this.sceneVersion += 1;
  }

  // ---------- derived ----------
  get world() {
    return this.snapshot?.world ?? null;
  }

  get canUndo(): boolean {
    return this.undoStack.length > 0;
  }

  get plannedTotalTenthCents(): number {
    return this.drafts.reduce((acc, d) => acc + d.price_tenth_cents, 0);
  }

  get anyInvalidDraft(): boolean {
    return this.drafts.some((d) => d.errors.length > 0);
  }

  currentListing(): BuildingListing | null {
    const plant = this.selectedPlantId ? this.world?.plants[this.selectedPlantId] : undefined;
    if (!plant) return null;
    return this.catalogs.buildings.find((b) => b.listing_id === plant.listing_id) ?? null;
  }

  catalogEntry(catalogId: string): CellCatalogEntry | undefined {
    return this.catalogs.cells.find((c) => c.catalog_id === catalogId);
  }

  // ---------- session ----------
  async boot(): Promise<void> {
    try {
      this.user = await this.api.me();
      await this.loadLobby();
    } catch {
      this.user = null;
    }
    this.booting = false;
    this.notify();
  }

  private async guarded(fn: () => Promise<void>): Promise<void> {
    this.busy = true;
    this.notify();
    try {
      await fn();
      this.banner = null;
    } catch (err) {
      this.banner = err instanceof Error ? err.message : String(err);
    } finally {
      this.busy = false;
      this.notify();
    }
  }

  login(email: string, password: string) {
    return this.guarded(async () => {
      const res = await this.api.login(email, password);
      this.api.setToken(res.session_token);
      this.user = res.user;
      await this.loadLobby();
    });
  }

  register(invite: string, email: string, password: string) {
    return this.guarded(async () => {
      const res = await this.api.register(invite, email, password);
      this.api.setToken(res.session_token);
      this.user = res.user;
      await this.loadLobby();
    });
  }

  resetPassword(token: string, password: string) {
    return this.guarded(() => this.api.resetPassword(token, password));
  }

  logout() {
    this.setPlaying(false);
    return this.guarded(async () => {
      await this.api.logout().catch(() => undefined);
      this.api.setToken(null);
      this.user = null;
      this.snapshot = null;
    });
  }

  private async loadLobby(): Promise<void> {
    const [catalogs, games] = await Promise.all([this.api.catalogs(), this.api.listGames()]);
    this.catalogs = catalogs;
    this.games = games;
  }

  backToLobby() {
    this.setPlaying(false);
    this.snapshot = null;
    this.drafts = [];
    this.undoStack = [];
    return this.guarded(() => this.loadLobby());
  }

  createInvite() {
    return this.guarded(async () => {
      this.generatedInvite = (await this.api.createInvite('player')).invite_token;
    });
  }

  createResetLink(email: string) {
    return this.guarded(async () => {
      this.generatedReset = (await this.api.createResetLink(email)).reset_token;
    });
  }

  // ---------- game lifecycle ----------
  /** The single entry point for server snapshots into the mirror. */
  applySnapshot(snap: ClientGameSnapshot): void {
    this.snapshot = snap;
    const plantIds = Object.keys(snap.world.plants);
    if (!this.selectedPlantId || !snap.world.plants[this.selectedPlantId]) {
      this.selectedPlantId = plantIds[0] ?? null;
    }
    if (plantIds.length === 0) this.activeTab = 'Plants';
    this.revalidateDrafts();
  }

  createGame(scenario: string, seed: number, autoPause: boolean) {
    return this.guarded(async () => this.applySnapshot(await this.api.createGame(scenario, seed, autoPause)));
  }

  openGame(gameId: string) {
    this.setPlaying(false);
    return this.guarded(async () => this.applySnapshot(await this.api.getGame(gameId)));
  }

  private get gameId(): string {
    if (!this.snapshot) throw new Error('No game open');
    return this.snapshot.world.game_id;
  }

  send(cmd: PlantCommand) {
    return this.guarded(async () => this.applySnapshot(await this.api.command(this.gameId, cmd)));
  }

  skipAhead(days = 7) {
    return this.guarded(async () => {
      this.applySnapshot(await this.api.skip(this.gameId, days * 86_400_000));
      this.showSkipSummary = true;
    });
  }

  step(hours = 6) {
    return this.guarded(async () => this.applySnapshot(await this.api.step(this.gameId, hours * 3_600_000)));
  }

  saveCheckpoint(name: string) {
    return this.guarded(async () => this.applySnapshot(await this.api.saveCheckpoint(this.gameId, name)));
  }

  rewind(checkpointId: string) {
    this.setPlaying(false);
    return this.guarded(async () => this.applySnapshot(await this.api.rewind(this.gameId, checkpointId)));
  }

  fileProblemReport(note: string) {
    return this.guarded(async () => {
      this.lastProblemReport = await this.api.report(this.gameId, note);
    });
  }

  // ---------- simple UI state ----------
  setTab(tab: TabName) {
    this.activeTab = tab;
    this.notify();
  }
  selectPlant(plantId: string) {
    this.selectedPlantId = plantId;
    this.revalidateDrafts();
    this.notify();
  }
  select(entityId: string | null) {
    this.selectedEntityId = entityId;
    this.notify();
  }
  setPaletteTeam(team: CellCatalogEntry['team']) {
    this.paletteTeam = team;
    this.notify();
  }
  setUnits(u: UnitSystem) {
    this.units = u;
    this.notify();
  }
  setCurrency(c: Currency) {
    this.currency = c;
    this.notify();
  }
  setModal(which: 'skip' | 'problem', open: boolean) {
    if (which === 'skip') this.showSkipSummary = open;
    else this.showProblemModal = open;
    this.notify();
  }
  dismissBanner() {
    this.banner = null;
    this.notify();
  }
  showBanner(message: string) {
    this.banner = message;
    this.notify();
  }

  // ---------- play controls (server owns time; the client only asks it to step) ----------
  setPlaying(on: boolean) {
    if (on === this.playing || (on && !this.snapshot)) return;
    this.playing = on;
    if (on) this.tickTimer = setInterval(() => void this.tick(), GameClient.TICK_MS);
    else if (this.tickTimer) {
      clearInterval(this.tickTimer);
      this.tickTimer = null;
    }
    this.notify();
  }

  togglePlay() {
    this.setPlaying(!this.playing);
  }

  setSpeed(speed: Speed) {
    this.speed = speed;
    this.notify();
    this.setPlaying(true);
  }

  /** Called by the 3D view once per frame; returns the sim time to draw. */
  advanceDisplayTime(dtWallMs: number): number {
    const serverNow = this.world?.now ?? 0;
    this.displayTimeMs = nextDisplayTime(this.displayTimeMs, serverNow, dtWallMs, this.speed, this.playing, this.speed * GameClient.TICK_MS * 2);
    return this.displayTimeMs;
  }

  private async tick(): Promise<void> {
    if (!this.playing || this.tickInFlight || !this.snapshot) return;
    this.tickInFlight = true;
    try {
      const snap = await this.api.step(this.gameId, this.speed * GameClient.TICK_MS);
      if (!this.playing) return;
      this.applySnapshot(snap);
      if (snap.runway.bankrupt) this.setPlaying(false);
      this.notify();
    } catch (err) {
      this.banner = err instanceof Error ? err.message : String(err);
      this.setPlaying(false);
    } finally {
      this.tickInFlight = false;
    }
  }

  // ---------- planning mode (controller-side, never sent until commit) ----------
  private pushUndo() {
    this.undoStack.push(this.drafts.map((d) => ({ ...d, errors: [...d.errors] })));
    if (this.undoStack.length > 100) this.undoStack.shift();
  }

  private revalidateDrafts() {
    const listing = this.currentListing();
    const installed = Object.values(this.world?.cells ?? {});
    for (const d of this.drafts) {
      d.errors = previewPlacementErrors(d, listing, installed, this.catalogs.cells, this.drafts);
    }
    this.touchScene();
  }

  addDraft(catalogId: string): Draft | null {
    const cat = this.catalogEntry(catalogId);
    if (!cat || !this.selectedPlantId) return null;
    this.pushUndo();
    this.draftSeq += 1;
    const base = {
      draft_id: `plan-${this.draftSeq}`,
      plant_id: this.selectedPlantId,
      catalog_id: cat.catalog_id,
      team: cat.team,
      display_name: cat.display_name,
      price_tenth_cents: cat.price,
      width_mm: cat.width_mm,
      depth_mm: cat.depth_mm,
      height_mm: cat.height_mm,
      clearance_mm: cat.clearance_mm,
      rotation_deg: 0,
      errors: [] as string[],
    };
    const spot = findFreeSpot(base, this.currentListing(), Object.values(this.world?.cells ?? {}), this.catalogs.cells, this.drafts);
    const draft: Draft = { ...base, ...spot };
    this.drafts.push(draft);
    this.selectedEntityId = draft.draft_id;
    this.revalidateDrafts();
    this.notify();
    return draft;
  }

  beginDraftDrag() {
    this.pushUndo();
  }

  /** Live drag: updates only the 3D scene, not React. */
  moveDraftLive(draftId: string, xMm: number, yMm: number) {
    const d = this.drafts.find((x) => x.draft_id === draftId);
    if (!d) return;
    d.x_mm = Math.max(0, snapMm(xMm));
    d.y_mm = Math.max(0, snapMm(yMm));
    this.revalidateDrafts();
  }

  /** Drop: re-render UI and ask the server for the authoritative placement verdict. */
  async endDraftDrag(draftId: string) {
    this.notify();
    const d = this.drafts.find((x) => x.draft_id === draftId);
    if (!d || !this.snapshot || d.errors.length > 0) return;
    const at = { x: d.x_mm, y: d.y_mm, r: d.rotation_deg };
    try {
      const res = await this.api.validatePlacement(this.gameId, {
        plant_id: d.plant_id,
        catalog_id: d.catalog_id,
        x_mm: d.x_mm,
        y_mm: d.y_mm,
        rotation_deg: d.rotation_deg,
      });
      // Ignore stale verdicts if the draft moved again while the request was in flight.
      if (d.x_mm === at.x && d.y_mm === at.y && d.rotation_deg === at.r && !res.valid) {
        d.errors = res.errors;
        this.notify();
      }
    } catch {
      /* preview verdict stands */
    }
  }

  rotateDraft(draftId: string) {
    const d = this.drafts.find((x) => x.draft_id === draftId);
    if (!d) return;
    this.pushUndo();
    d.rotation_deg = (d.rotation_deg + 90) % 360;
    this.revalidateDrafts();
    this.notify();
  }

  removeDraft(draftId: string) {
    const idx = this.drafts.findIndex((x) => x.draft_id === draftId);
    if (idx < 0) return;
    this.pushUndo();
    this.drafts.splice(idx, 1);
    if (this.selectedEntityId === draftId) this.selectedEntityId = null;
    this.revalidateDrafts();
    this.notify();
  }

  undo() {
    const prev = this.undoStack.pop();
    if (!prev) return;
    this.drafts = prev;
    this.revalidateDrafts();
    this.notify();
  }

  clearDrafts() {
    if (this.drafts.length === 0) return;
    this.pushUndo();
    this.drafts = [];
    this.selectedEntityId = null;
    this.notify();
  }

  commitDrafts() {
    const drafts = [...this.drafts];
    return this.guarded(async () => {
      let n = Object.keys(this.world?.cells ?? {}).length + 1;
      for (const d of drafts) {
        const snap = await this.api.command(this.gameId, {
          PlaceCell: {
            plant_id: d.plant_id,
            cell_id: `${d.team.toLowerCase().slice(0, 4)}-${n++}`,
            team: d.team,
            catalog_id: d.catalog_id,
            addons: [],
            x_mm: d.x_mm,
            y_mm: d.y_mm,
            rotation_deg: d.rotation_deg,
          },
        });
        this.drafts = this.drafts.filter((x) => x.draft_id !== d.draft_id);
        this.applySnapshot(snap);
      }
      this.undoStack = [];
      this.selectedEntityId = null;
    });
  }

  // ---------- moving installed cells ----------
  previewCellMove(cellId: string, xMm: number, yMm: number) {
    this.cellDragPreview = { cellId, x_mm: Math.max(0, snapMm(xMm)), y_mm: Math.max(0, snapMm(yMm)) };
    this.touchScene();
  }

  async commitCellMove() {
    const p = this.cellDragPreview;
    this.cellDragPreview = null;
    const cell = p ? this.world?.cells[p.cellId] : undefined;
    if (!p || !cell || (cell.x_mm === p.x_mm && cell.y_mm === p.y_mm)) {
      this.notify();
      return;
    }
    await this.send({
      MoveCell: { plant_id: cell.plant_id, cell_id: cell.cell_id, x_mm: p.x_mm, y_mm: p.y_mm, rotation_deg: cell.rotation_deg },
    });
  }
}

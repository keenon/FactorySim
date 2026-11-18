// Wire types mirroring the Rust serde JSON shapes served by `fsim-server`.
// Source of truth: fsim-core (SimTime, Money, SceneNode, CellModel, CellStatusReport),
// fsim-protocols (PlantCommand, TeamTag, InternalOrder), fsim-plant-host (SimWorld),
// fsim-server (ClientGameSnapshot, StaticCatalogBundle). Keep in sync when those change.

/** Integer milliseconds since sim start. */
export type SimTimeMs = number;
/** Integer tenth-cents ($1.00 = 1000). */
export type TenthCents = number;

export type TeamTag =
  | 'Unspecified'
  | 'Platform'
  | 'Machining'
  | 'Toolroom'
  | 'Molding'
  | 'HeatTreat'
  | 'FinishingPacking'
  | 'Quality'
  | 'MaterialsLogistics'
  | 'FacilitiesEhs'
  | 'PlanningFlow'
  | 'PeopleOps'
  | 'Sales'
  | 'Finance';

export type CellRunState = 'Running' | 'Setup' | 'Idle' | 'Blocked' | 'Down' | 'Planned' | 'Installing';

export interface PointMm {
  x: number;
  y: number;
}

export interface PolygonWithHolesMm {
  outer: PointMm[];
  holes: PointMm[][];
}

export interface BuildingListing {
  listing_id: string;
  name: string;
  building_kind: string;
  description: string;
  floor_polygon: PolygonWithHolesMm;
  clear_height_mm: number;
  max_door_width_mm: number;
  max_door_height_mm: number;
  dock_doors: number;
  power_capacity_kw: number;
  has_three_phase_480v: boolean;
  monthly_rent: TenthCents;
  lease_deposit: TenthCents;
  keys_lead_days_min: number;
  keys_lead_days_max: number;
  utilities_lead_days_min: number;
  utilities_lead_days_max: number;
}

export interface CellAddonSpec {
  addon_id: string;
  name: string;
  price_delta: TenthCents;
  cycle_time_pct_delta: number;
  description: string;
}

export interface CellCatalogEntry {
  catalog_id: string;
  team: TeamTag;
  display_name: string;
  description: string;
  price: TenthCents;
  monthly_cost: TenthCents;
  lead_time_days_min: number;
  lead_time_days_max: number;
  width_mm: number;
  depth_mm: number;
  height_mm: number;
  clearance_mm: number;
  power_kw: number;
  needs_three_phase_480v: boolean;
  needs_compressed_air: boolean;
  needs_climate_room: boolean;
  needs_hazmat_permit: boolean;
  financeable: boolean;
  input_squares: [string, string[]][];
  output_square_skus: string[];
  cycle_time_summary: string;
  available_addons: CellAddonSpec[];
}

// --- Procedural 3D cell models (fsim-core::CellModel) ---

export type ModelAxis = 'X' | 'Y' | 'Z';
export type ModelMaterial = 'Paint' | 'Metal' | 'Glass' | 'Rubber' | 'Emissive' | 'Screen';
export type ModelShape =
  | { kind: 'Box'; size_mm: [number, number, number] }
  | { kind: 'Cylinder'; radius_mm: number; length_mm: number; axis: ModelAxis };

export interface ModelPart {
  node: string;
  shape: ModelShape;
  center_mm: [number, number, number];
  material: ModelMaterial;
  color_hex: string;
}

export interface CellModel {
  width_mm: number;
  depth_mm: number;
  height_mm: number;
  parts: ModelPart[];
}

export interface SceneNode {
  node_id: string;
  asset_id: string | null;
  primitive: string;
  x_mm: number;
  y_mm: number;
  z_mm: number;
  width_mm: number;
  depth_mm: number;
  height_mm: number;
  rotation_deg: number;
  color_hex: string;
  label: string;
  ghost: boolean;
  model: CellModel | null;
}

export interface MarketDefinition {
  market_id: string;
  name: string;
  description: string;
  required_certs: string[];
  requires_cmm: boolean;
  requires_salesperson: boolean;
  base_order_interval_hours: number;
  reference_payout_per_part: TenthCents;
}

export interface CertificationSpec {
  cert_id: string;
  name: string;
  company_wide: boolean;
  cost: TenthCents;
  lead_days: number;
  description: string;
}

export interface StaticCatalogBundle {
  buildings: BuildingListing[];
  cells: CellCatalogEntry[];
  markets: MarketDefinition[];
  certifications: CertificationSpec[];
  preview_models: Record<string, CellModel>;
}

// --- World state (fsim-plant-host::SimWorld) ---

export interface PlantSiteState {
  plant_id: string;
  listing_id: string;
  name: string;
  keys_ready_at: SimTimeMs;
  utilities_ready_at: SimTimeMs;
  keys_received: boolean;
  utilities_ready: boolean;
  climate_room_built: boolean;
  hazmat_permit_approved: boolean;
}

export interface InstalledCell {
  cell_id: string;
  plant_id: string;
  catalog_id: string;
  team: TeamTag;
  display_name: string;
  addons: string[];
  x_mm: number;
  y_mm: number;
  rotation_deg: number;
  commissioned_at: SimTimeMs;
  device_id: string;
  active_order_id: string | null;
  tool_wear_pct: number;
  chip_hopper_pct: number;
  cycles_completed: number;
}

export interface Pose2D {
  x_mm: number;
  y_mm: number;
  heading_mdeg: number;
}

export interface MotionSegment {
  start_ms: SimTimeMs;
  end_ms: SimTimeMs;
  start_pose: Pose2D;
  end_pose: Pose2D;
}

export interface WorkerState {
  worker_id: string;
  name: string;
  role: string;
  plant_id: string;
  ipad_device_id: string;
  wage_per_hour: TenthCents;
  ready_at: SimTimeMs;
  worked_ms_in_domain: number;
  current_pose: Pose2D;
  motion_segments: MotionSegment[];
  active_task: { task: { task_id: string; category: string; order_id: string } } | null;
}

export interface HandlingUnitSpec {
  container_type: string;
  item_sku: string;
  quantity: number;
  lot_id: string;
}

export interface InternalOrder {
  order_id: string;
  parent_order_id: string;
  customer_order_id: string;
  target_team: TeamTag;
  target_cell_id: string;
  output_spec: HandlingUnitSpec;
  status: string;
  upstream_order_ids: string[];
}

export interface CustomerOrder {
  customer_order_id: string;
  plant_id: string;
  market_id: string;
  part_name: string;
  material_sku: string;
  payout: TenthCents;
  due_at_ms: SimTimeMs;
  status: string;
}

export interface PipelineItem {
  item_id: string;
  category: string;
  title: string;
  started_at: SimTimeMs;
  eta_p10: SimTimeMs;
  eta_p50: SimTimeMs;
  eta_p90: SimTimeMs;
  cost: TenthCents;
  completed: boolean;
}

export interface MarketBiddingPolicy {
  market_id: string;
  enabled: boolean;
  min_payout_per_hour: TenthCents;
  price_multiplier_pct: number;
  promised_lead_days: number;
}

export interface LedgerRecord {
  entry_id: string;
  timestamp: SimTimeMs;
  debit_account: string;
  credit_account: string;
  amount: TenthCents;
  memo: string;
}

export interface SquareEndpoint {
  cell_id: string;
  square_name: string;
  access_point: PointMm;
}

/** Walking route for one material flow (fsim-layout-graph::LayoutRoute). */
export interface LayoutRoute {
  from_cell: string;
  from_square: string;
  to_cell: string;
  to_square: string;
  waypoints: PointMm[];
  distance_mm: number;
  forklift_feasible: boolean;
}

export interface TransportGraph {
  layout_version: number;
  endpoints: [[string, string], SquareEndpoint][];
  routes: [[[string, string], [string, string]], LayoutRoute][];
}

export interface SimWorld {
  game_id: string;
  scenario_name: string;
  game_seed: number;
  now: SimTimeMs;
  tutorial_step: number;
  plants: Record<string, PlantSiteState>;
  cells: Record<string, InstalledCell>;
  workers: Record<string, WorkerState>;
  shift_priorities: string[];
  market_policies: [[string, string], MarketBiddingPolicy][];
  market_reputation_permille: Record<string, number>;
  unlocked_certs: string[];
  customer_orders: Record<string, CustomerOrder>;
  internal_orders: Record<string, InternalOrder>;
  pipeline: Record<string, PipelineItem>;
  loans: Record<string, unknown>;
  ledger: { entries: LedgerRecord[]; account_balances: Record<string, number> };
  transport_graphs: Record<string, TransportGraph>;
}

export interface CellStatusReport {
  cell_id: string;
  state: CellRunState;
  reason: string;
  message: string;
  since: SimTimeMs;
  blocker_link: string | null;
  uncertainty_note: string | null;
}

export interface RunwayProjection {
  cash_now: TenthCents;
  credit_available: TenthCents;
  gross_monthly_burn: TenthCents;
  net_monthly_burn: TenthCents;
  runway_days_p10: number;
  runway_days_p50: number;
  runway_days_p90: number;
  bankrupt: boolean;
}

export interface BottleneckReport {
  bottleneck_team: TeamTag;
  active_ready_orders: number;
  utilization_permille: number;
  shadow_price_dollars_per_hr: number;
  recommendation: string;
}

export interface CheckpointMeta {
  checkpoint_id: string;
  name: string;
  is_autosave: boolean;
  sim_time_ms: SimTimeMs;
  cash_tenth_cents: TenthCents;
}

export interface SkipSummaryCard {
  days_skipped_tenths: number;
  cash_start: TenthCents;
  cash_end: TenthCents;
  runway_days_start: number;
  runway_days_end: number;
  stop_reason: string;
  events_occurred: string[];
}

export interface ClientGameSnapshot {
  world: SimWorld;
  cell_statuses: CellStatusReport[];
  scene_nodes_by_plant: Record<string, SceneNode[]>;
  runway: RunwayProjection;
  bottleneck: BottleneckReport;
  checkpoints: CheckpointMeta[];
  last_skip_summary: SkipSummaryCard | null;
  gateway_log_len: number;
}

export interface UserAccount {
  user_id: string;
  email: string;
  role: string;
}

export interface GameSummaryItem {
  game_id: string;
  scenario_name: string;
  game_seed: number;
  sim_time_ms: SimTimeMs;
  cash_tenth_cents: TenthCents;
  runway_days_p50: number;
  plant_count: number;
}

export interface ProblemReportBundle {
  report_id: string;
  game_seed: number;
  log_seq: number;
  latest_checkpoint_id: string | null;
}

/** Externally tagged `fsim_protocols::PlantCommand`. */
export type PlantCommand =
  | { LeasePlant: { plant_id: string; listing_id: string; name: string } }
  | {
      PlaceCell: {
        plant_id: string;
        cell_id: string;
        team: TeamTag;
        catalog_id: string;
        addons: string[];
        x_mm: number;
        y_mm: number;
        rotation_deg: number;
      };
    }
  | { MoveCell: { plant_id: string; cell_id: string; x_mm: number; y_mm: number; rotation_deg: number } }
  | { HireWorker: { plant_id: string; worker_id: string; role: string; wage_per_hour_tenth_cents: number } }
  | {
      ConfigureMarketPolicy: {
        plant_id: string;
        market_id: string;
        enabled: boolean;
        min_payout_per_hour_tenth_cents: number;
        price_multiplier_pct: number;
        promised_lead_days: number;
      };
    }
  | { UnlockMarketCert: { plant_id: string; cert_id: string } }
  | { OrderRawStock: { plant_id: string; sku: string; quantity: number } }
  | { TakeEquipmentLoan: { loan_id: string; principal_tenth_cents: number; collateral_cell_id: string } };

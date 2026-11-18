//! `fsim-plant-host`: Runs the deterministic factory simulation plant, manages virtual device
//! sessions (`fsim-net`) for every placed cell and worker iPad, streams observations into
//! `fsim-gateway`, updates the controller's estimated state, and provides the sim control channel
//! (skip-ahead to next event, runway summary card, fork, rewind, and god-view truth inspection).

pub mod reference;

use fsim_core::{
    BufferView, CellCatalogEntry, CellStatusReport, DoubleEntryLedger, HandoffSquare,
    HandlingUnit, Money, OrderPolicy, SceneNode, SimTime, StreamRng,
};
use fsim_facilities_ehs::{
    reference::reference_validate_cell_placement, starter_building_listings,
    starter_facilities_catalog, BuildingListing, PlantSiteState,
};
use fsim_finance::{EquipmentLoan, RunwayProjection};
use fsim_finishing_packing::{
    reference::reference_packing_time_ms, starter_finishing_packing_catalog,
};
use fsim_gateway::GatewayState;
use fsim_geom::PointMm;
use fsim_heat_treat::starter_heat_treat_catalog;
use fsim_layout_graph::{SquareEndpoint, TransportGraph};
use fsim_machining::{
    reference::{reference_cycle_time_ms, reference_scrap_permille},
    starter_machining_catalog, MachiningCellConfig,
};
use fsim_materials_logistics::{
    reference::reference_saw_cut_ms, starter_materials_logistics_catalog,
};
use fsim_molding::starter_molding_catalog;
use fsim_motion::Pose2D;
use fsim_net::{DeviceIdentity, FrameDirection, VirtualNetworkState};
use fsim_people::{
    default_task_priorities, dispatch_task_to_worker,
    reference::reference_select_next_task_index, PendingFloorTask, WorkerState,
};
use fsim_planning_flow::{
    create_backward_order_chain, detect_bottleneck, refresh_order_unblocking, BottleneckReport,
};
use fsim_protocols::{
    BlockerReason, CellRunState, Envelope, HandlingUnitSpec, InternalOrder, InternalOrderStatus,
    OperatorTaskKind, PlantCommand, TeamTag,
};
use fsim_quality::{reference::reference_inspection_odds_permille, starter_quality_catalog};
use fsim_sales::{
    reference::reference_settle_shipment, sample_customer_order, starter_certifications,
    starter_markets, CertificationSpec, CustomerOrder, MarketBiddingPolicy,
};
use fsim_toolroom::starter_toolroom_catalog;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TupleKeyMap;

pub mod tuple_map_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S, V>(
        map: &BTreeMap<(String, String), V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        let vec: Vec<(&(String, String), &V)> = map.iter().collect();
        vec.serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(
        deserializer: D,
    ) -> Result<BTreeMap<(String, String), V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        let vec: Vec<((String, String), V)> = Vec::deserialize(deserializer)?;
        Ok(vec.into_iter().collect())
    }
}
/// Returns the combined starter catalog across all teams.
pub fn full_starter_catalog() -> Vec<CellCatalogEntry> {
    let mut all = Vec::new();
    all.extend(starter_machining_catalog());
    all.extend(starter_toolroom_catalog());
    all.extend(starter_molding_catalog());
    all.extend(starter_heat_treat_catalog());
    all.extend(starter_finishing_packing_catalog());
    all.extend(starter_quality_catalog());
    all.extend(starter_materials_logistics_catalog());
    all.extend(starter_facilities_catalog());
    all
}

/// An installed (or in-flight delivery) cell on a plant floor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledCell {
    pub cell_id: String,
    pub plant_id: String,
    pub catalog_id: String,
    pub team: TeamTag,
    pub display_name: String,
    pub addons: Vec<String>,
    pub x_mm: i32,
    pub y_mm: i32,
    pub rotation_deg: i32,
    pub ordered_at: SimTime,
    pub delivered_at: SimTime,
    pub commissioned_at: SimTime,
    pub device_id: String,
    pub active_order_id: Option<String>,
    pub cycle_finish_at: Option<SimTime>,
    pub tool_wear_pct: u32,
    pub chip_hopper_pct: u32,
    pub down_for_repair: bool,
    pub status_since: SimTime,
    pub cycles_completed: u64,
}

impl InstalledCell {
    pub fn is_commissioned(&self, now: SimTime) -> bool {
        now >= self.commissioned_at && !self.down_for_repair
    }
}

/// A pipeline commitment in flight (plant keys/utilities, machine delivery, worker onboarding, cert audit, vendor PO).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineItem {
    pub item_id: String,
    pub plant_id: String,
    pub category: String, // "plant_lease", "machine_install", "hiring", "certification", "vendor_po"
    pub title: String,
    pub started_at: SimTime,
    pub eta_p10: SimTime,
    pub eta_p50: SimTime,
    pub eta_p90: SimTime,
    pub actual_complete_at: SimTime,
    pub cost: Money,
    pub completed: bool,
}

/// Decision inbox / alert item that can trigger auto-pause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertItem {
    pub alert_id: String,
    pub timestamp: SimTime,
    pub severity: String, // "info", "warning", "urgent"
    pub title: String,
    pub detail: String,
    pub link_target: Option<String>,
    pub acknowledged: bool,
}

/// Summary card returned after a player executes a Skip-Ahead action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkipSummaryCard {
    pub from_time: SimTime,
    pub to_time: SimTime,
    pub days_skipped_tenths: i64,
    pub cash_start: Money,
    pub cash_end: Money,
    pub runway_days_start: i64,
    pub runway_days_end: i64,
    pub stop_reason: String,
    pub events_occurred: Vec<String>,
}

/// Complete deterministic simulation state (Plant truth + Network + Gateway + Controller belief).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimWorld {
    pub game_id: String,
    pub scenario_name: String,
    pub game_seed: u64,
    pub version: u64,
    pub now: SimTime,
    pub order_policy: OrderPolicy,
    pub auto_pause_on_alerts: bool,
    pub tutorial_step: u32, // 0 = done/skipped, 1..=5 = active tutorial step

    pub plants: BTreeMap<String, PlantSiteState>,
    pub cells: BTreeMap<String, InstalledCell>,
    pub workers: BTreeMap<String, WorkerState>,
    pub shift_priorities: Vec<String>,
    pub buffers: BufferView,
    pub transport_graphs: BTreeMap<String, TransportGraph>,

    #[serde(with = "tuple_map_serde")]
    pub market_policies: BTreeMap<(String, String), MarketBiddingPolicy>,
    pub market_reputation_permille: BTreeMap<String, u32>,
    pub unlocked_certs: Vec<String>,
    pub customer_orders: BTreeMap<String, CustomerOrder>,
    pub internal_orders: BTreeMap<String, InternalOrder>,
    pub pending_tasks: Vec<PendingFloorTask>,
    pub active_logistics_orders: BTreeMap<String, String>, // (order_id, square_name) key -> task_id
    pub pipeline: BTreeMap<String, PipelineItem>,
    pub loans: BTreeMap<String, EquipmentLoan>,
    pub credit_limit: Money,
    pub ledger: DoubleEntryLedger,
    pub alerts: Vec<AlertItem>,

    pub net: VirtualNetworkState,
    pub gateway: GatewayState,

    pub next_market_poll_at: SimTime,
    pub next_monthly_overhead_at: SimTime,
    pub event_counter: u64,
}

impl SimWorld {
    /// Creates a fresh game starting with $500,000 seed capital and no leased plants (opens on the Plants tab).
    pub fn new(game_id: &str, scenario_name: &str, game_seed: u64) -> Self {
        let initial_cash = Money::from_dollars(500_000);
        let ledger = DoubleEntryLedger::new_with_starting_equity(initial_cash, SimTime::ZERO);
        let mut rep = BTreeMap::new();
        for m in starter_markets() {
            rep.insert(m.market_id, 650);
        }
        Self {
            game_id: game_id.to_string(),
            scenario_name: scenario_name.to_string(),
            game_seed,
            version: 1,
            now: SimTime::ZERO,
            order_policy: OrderPolicy::Seeded(game_seed),
            auto_pause_on_alerts: true,
            tutorial_step: 1,
            plants: BTreeMap::new(),
            cells: BTreeMap::new(),
            workers: BTreeMap::new(),
            shift_priorities: default_task_priorities(),
            buffers: BufferView::default(),
            transport_graphs: BTreeMap::new(),
            market_policies: BTreeMap::new(),
            market_reputation_permille: rep,
            unlocked_certs: Vec::new(),
            customer_orders: BTreeMap::new(),
            internal_orders: BTreeMap::new(),
            pending_tasks: Vec::new(),
            active_logistics_orders: BTreeMap::new(),
            pipeline: BTreeMap::new(),
            loans: BTreeMap::new(),
            credit_limit: Money::from_dollars(50_000),
            ledger,
            alerts: Vec::new(),
            net: VirtualNetworkState::default(),
            gateway: GatewayState::default(),
            next_market_poll_at: SimTime::from_hours(4),
            next_monthly_overhead_at: SimTime::from_days(30),
            event_counter: 0,
        }
    }

    pub fn lookup_listing(listing_id: &str) -> Option<BuildingListing> {
        starter_building_listings()
            .into_iter()
            .find(|b| b.listing_id == listing_id)
    }

    pub fn lookup_catalog(catalog_id: &str) -> Option<CellCatalogEntry> {
        full_starter_catalog()
            .into_iter()
            .find(|c| c.catalog_id == catalog_id)
    }

    /// Validates a prospective cell placement in planning mode or before executing `PlaceCell`.
    pub fn validate_placement(
        &self,
        plant_id: &str,
        catalog_id: &str,
        x_mm: i32,
        y_mm: i32,
        rotation_deg: i32,
        ignore_cell_id: Option<&str>,
    ) -> Result<(), Vec<String>> {
        let plant = self
            .plants
            .get(plant_id)
            .ok_or_else(|| vec![format!("Plant '{plant_id}' does not exist")])?;
        let listing = Self::lookup_listing(&plant.listing_id)
            .ok_or_else(|| vec![format!("Listing '{}' not found", plant.listing_id)])?;
        let catalog = Self::lookup_catalog(catalog_id)
            .ok_or_else(|| vec![format!("Catalog '{}' not found", catalog_id)])?;

        let mut existing = Vec::new();
        for c in self.cells.values() {
            if c.plant_id == plant_id {
                if let Some(cat) = Self::lookup_catalog(&c.catalog_id) {
                    existing.push((
                        c.cell_id.clone(),
                        cat.footprint_at(c.x_mm, c.y_mm, c.rotation_deg)
                            .inflate(cat.clearance_mm),
                    ));
                }
            }
        }

        reference_validate_cell_placement(
            plant,
            &listing,
            &catalog,
            x_mm,
            y_mm,
            rotation_deg,
            &existing,
            ignore_cell_id,
        )
    }

    /// Applies a player or MCTS command to the plant.
    pub fn apply_command(&mut self, cmd: PlantCommand) -> Result<String, String> {
        self.event_counter += 1;
        self.version += 1;
        match cmd {
            PlantCommand::LeasePlant {
                plant_id,
                listing_id,
                name,
            } => {
                if self.plants.contains_key(&plant_id) {
                    return Err(format!("Plant '{plant_id}' already leased"));
                }
                let listing = Self::lookup_listing(&listing_id)
                    .ok_or_else(|| format!("Unknown building listing '{listing_id}'"))?;

                let mut rng =
                    StreamRng::new(self.game_seed, "facilities", &plant_id, self.event_counter);
                let keys_days = rng.gen_range_i64(
                    listing.keys_lead_days_min as i64,
                    listing.keys_lead_days_max as i64,
                );
                let util_days = rng.gen_range_i64(
                    listing.utilities_lead_days_min as i64,
                    listing.utilities_lead_days_max as i64,
                );
                let keys_ready_at = self.now.add_ms(keys_days * SimTime::DAY_MS);
                let utilities_ready_at = keys_ready_at.add_ms(util_days * SimTime::DAY_MS);

                let up_front = Money(listing.lease_deposit.0 + listing.monthly_rent.0);
                self.ledger.post(
                    format!("lease-init-{plant_id}"),
                    self.now,
                    plant_id.clone(),
                    "expense:rent_and_deposit",
                    "asset:cash",
                    up_front,
                    format!("Lease deposit & first month rent for {name}"),
                    plant_id.clone(),
                );

                let site = PlantSiteState {
                    plant_id: plant_id.clone(),
                    listing_id: listing_id.clone(),
                    name: name.clone(),
                    leased_at: self.now,
                    keys_ready_at,
                    utilities_ready_at,
                    keys_received: false,
                    utilities_ready: false,
                    climate_room_built: false,
                    hazmat_permit_approved: false,
                    layout_version: 1,
                };
                self.plants.insert(plant_id.clone(), site);

                self.pipeline.insert(
                    format!("pipe-keys-{plant_id}"),
                    PipelineItem {
                        item_id: format!("pipe-keys-{plant_id}"),
                        plant_id: plant_id.clone(),
                        category: "plant_lease".to_string(),
                        title: format!("{name}: Lease Keys & Utility Hookup"),
                        started_at: self.now,
                        eta_p10: self.now.add_ms(
                            ((listing.keys_lead_days_min + listing.utilities_lead_days_min) as i64)
                                * SimTime::DAY_MS,
                        ),
                        eta_p50: utilities_ready_at,
                        eta_p90: self.now.add_ms(
                            ((listing.keys_lead_days_max + listing.utilities_lead_days_max) as i64)
                                * SimTime::DAY_MS,
                        ),
                        actual_complete_at: utilities_ready_at,
                        cost: up_front,
                        completed: false,
                    },
                );

                // Initialize default market policies for the new plant (all off until player enables)
                for m in starter_markets() {
                    self.market_policies.insert(
                        (plant_id.clone(), m.market_id.clone()),
                        MarketBiddingPolicy::sensible_default(&m.market_id, false),
                    );
                }

                // Automatically place a default Receiving Bench and Shipping Bay kit if room allows,
                // or let player place them; to make the 5-step tutorial seamless ("1. Lease plant, 2. Place CNC & Pack station"),
                // every leased plant includes a default dock Receiving area and Shipping lane at the door!
                self.install_support_cell_internal(
                    &plant_id,
                    &format!("{plant_id}-recv"),
                    "mat-receiving-bench",
                    500,
                    500,
                );
                self.install_support_cell_internal(
                    &plant_id,
                    &format!("{plant_id}-ship"),
                    "mat-shipping-bay",
                    500,
                    2_500,
                );
                // Seed initial starter stock of precut blanks and cardboard cartons at the plant's rack
                let rack_id = format!("{plant_id}-recv");
                for i in 1..=8 {
                    self.buffers.put_to_rack(
                        &rack_id,
                        HandlingUnit {
                            unit_id: format!("init-blank-{plant_id}-{i}"),
                            spec: HandlingUnitSpec {
                                container_type: "tray".to_string(),
                                item_sku: "blank-alu".to_string(),
                                quantity: 1,
                                lot_id: format!("HEAT-ALU-6061-{i:03}"),
                            },
                            heat_or_lot_cert: format!("CERT-6061-T6-{i:03}"),
                            true_in_spec: true,
                            believed_in_spec: true,
                            created_at: self.now,
                        },
                    );
                    self.buffers.put_to_rack(
                        &rack_id,
                        HandlingUnit {
                            unit_id: format!("init-box-{plant_id}-{i}"),
                            spec: HandlingUnitSpec {
                                container_type: "bundle".to_string(),
                                item_sku: "carton-box".to_string(),
                                quantity: 1,
                                lot_id: format!("LOT-BOX-{i:03}"),
                            },
                            heat_or_lot_cert: "CERT-CORRUGATED-200LB".to_string(),
                            true_in_spec: true,
                            believed_in_spec: true,
                            created_at: self.now,
                        },
                    );
                    self.buffers.put_to_rack(
                        &rack_id,
                        HandlingUnit {
                            unit_id: format!("init-bar-{plant_id}-{i}"),
                            spec: HandlingUnitSpec {
                                container_type: "bar".to_string(),
                                item_sku: "raw-bar-alu".to_string(),
                                quantity: 1,
                                lot_id: format!("HEAT-BAR-6061-{i:03}"),
                            },
                            heat_or_lot_cert: format!("CERT-BAR-6061-{i:03}"),
                            true_in_spec: true,
                            believed_in_spec: true,
                            created_at: self.now,
                        },
                    );
                }

                self.rebuild_transport_graph(&plant_id);
                if self.tutorial_step == 1 {
                    self.tutorial_step = 2;
                }
                Ok(format!("Leased plant '{name}' ({plant_id})"))
            }

            PlantCommand::PlaceCell {
                plant_id,
                cell_id,
                team: _,
                catalog_id,
                addons,
                x_mm,
                y_mm,
                rotation_deg,
            } => {
                if self.cells.contains_key(&cell_id) {
                    return Err(format!("Cell ID '{cell_id}' already exists"));
                }
                if let Err(reasons) =
                    self.validate_placement(&plant_id, &catalog_id, x_mm, y_mm, rotation_deg, None)
                {
                    return Err(reasons.join("; "));
                }
                let catalog = Self::lookup_catalog(&catalog_id)
                    .ok_or_else(|| format!("Unknown catalog '{catalog_id}'"))?;

                let addon_cost: i64 = addons
                    .iter()
                    .filter_map(|aid| catalog.available_addons.iter().find(|a| &a.addon_id == aid))
                    .map(|a| a.price_delta.0)
                    .sum();
                let total_price = Money(catalog.price.0 + addon_cost);

                let mut rng =
                    StreamRng::new(self.game_seed, "cell-order", &cell_id, self.event_counter);
                let lead_days = rng.gen_range_i64(
                    catalog.lead_time_days_min as i64,
                    catalog.lead_time_days_max as i64,
                );
                let delivered_at = self.now.add_ms(lead_days * SimTime::DAY_MS);
                let commissioned_at = delivered_at.add_ms(SimTime::HOUR_MS * 6);

                self.ledger.post(
                    format!("capex-{cell_id}"),
                    self.now,
                    plant_id.clone(),
                    "asset:equipment",
                    "asset:cash",
                    total_price,
                    format!("Purchased {}", catalog.display_name),
                    cell_id.clone(),
                );

                if catalog_id == "ehs-climate-enclosure" {
                    if let Some(p) = self.plants.get_mut(&plant_id) {
                        p.climate_room_built = true;
                    }
                }

                let device_id = format!("plant/{plant_id}/cell/{cell_id}/controller");
                self.net.register_device(DeviceIdentity {
                    device_id: device_id.clone(),
                    plant_id: plant_id.clone(),
                    entity_id: cell_id.clone(),
                    device_kind: "machine-controller".to_string(),
                    cert_fingerprint: format!("sha256:dev-{cell_id}"),
                });

                self.register_cell_squares(&cell_id, &catalog, x_mm, y_mm);

                let installed = InstalledCell {
                    cell_id: cell_id.clone(),
                    plant_id: plant_id.clone(),
                    catalog_id: catalog_id.clone(),
                    team: catalog.team,
                    display_name: catalog.display_name.clone(),
                    addons,
                    x_mm,
                    y_mm,
                    rotation_deg,
                    ordered_at: self.now,
                    delivered_at,
                    commissioned_at,
                    device_id,
                    active_order_id: None,
                    cycle_finish_at: None,
                    tool_wear_pct: 0,
                    chip_hopper_pct: 0,
                    down_for_repair: false,
                    status_since: self.now,
                    cycles_completed: 0,
                };
                self.cells.insert(cell_id.clone(), installed);

                self.pipeline.insert(
                    format!("pipe-cell-{cell_id}"),
                    PipelineItem {
                        item_id: format!("pipe-cell-{cell_id}"),
                        plant_id: plant_id.clone(),
                        category: "machine_install".to_string(),
                        title: format!("Deliver & Commission {}", catalog.display_name),
                        started_at: self.now,
                        eta_p10: self
                            .now
                            .add_ms((catalog.lead_time_days_min as i64) * SimTime::DAY_MS),
                        eta_p50: commissioned_at,
                        eta_p90: self
                            .now
                            .add_ms(((catalog.lead_time_days_max + 1) as i64) * SimTime::DAY_MS),
                        actual_complete_at: commissioned_at,
                        cost: total_price,
                        completed: false,
                    },
                );

                self.rebuild_transport_graph(&plant_id);
                if self.tutorial_step == 2 {
                    let has_cnc = self
                        .cells
                        .values()
                        .any(|c| c.plant_id == plant_id && c.team == TeamTag::Machining);
                    let has_pack = self
                        .cells
                        .values()
                        .any(|c| c.plant_id == plant_id && c.team == TeamTag::FinishingPacking);
                    if has_cnc && has_pack {
                        self.tutorial_step = 3;
                    }
                }
                Ok(format!("Ordered and placed cell '{cell_id}'"))
            }

            PlantCommand::MoveCell {
                plant_id,
                cell_id,
                x_mm,
                y_mm,
                rotation_deg,
            } => {
                let catalog_id = self
                    .cells
                    .get(&cell_id)
                    .ok_or_else(|| format!("Cell '{cell_id}' not found"))?
                    .catalog_id
                    .clone();
                if let Err(reasons) = self.validate_placement(
                    &plant_id,
                    &catalog_id,
                    x_mm,
                    y_mm,
                    rotation_deg,
                    Some(&cell_id),
                ) {
                    return Err(reasons.join("; "));
                }
                let catalog = Self::lookup_catalog(&catalog_id).unwrap();
                if let Some(c) = self.cells.get_mut(&cell_id) {
                    c.x_mm = x_mm;
                    c.y_mm = y_mm;
                    c.rotation_deg = rotation_deg;
                }
                // Moving an existing cell incurs rigging & recommissioning fee ($1,500)
                self.ledger.post(
                    format!("rigging-{cell_id}-{}", self.event_counter),
                    self.now,
                    plant_id.clone(),
                    "expense:rigging",
                    "asset:cash",
                    Money::from_dollars(1_500),
                    format!("Rigging & recommissioning move for {cell_id}"),
                    cell_id.clone(),
                );
                self.update_cell_square_positions(&cell_id, &catalog, x_mm, y_mm);
                self.rebuild_transport_graph(&plant_id);
                Ok(format!("Moved cell '{cell_id}'"))
            }

            PlantCommand::RemoveCell { plant_id, cell_id } => {
                self.cells.remove(&cell_id);
                self.buffers.squares.retain(|(cid, _), _| cid != &cell_id);
                self.rebuild_transport_graph(&plant_id);
                Ok(format!("Removed cell '{cell_id}'"))
            }

            PlantCommand::HireWorker {
                plant_id,
                worker_id,
                role,
                wage_per_hour_tenth_cents,
            } => {
                if self.workers.contains_key(&worker_id) {
                    return Err(format!("Worker '{worker_id}' already exists"));
                }
                let mut rng =
                    StreamRng::new(self.game_seed, "hiring", &worker_id, self.event_counter);
                let fill_days = rng.gen_range_i64(2, 5);
                let ready_at = self.now.add_ms(fill_days * SimTime::DAY_MS);
                let ipad_device_id = format!("plant/{plant_id}/person/{worker_id}/ipad");

                self.net.register_device(DeviceIdentity {
                    device_id: ipad_device_id.clone(),
                    plant_id: plant_id.clone(),
                    entity_id: worker_id.clone(),
                    device_kind: "worker-ipad".to_string(),
                    cert_fingerprint: format!("sha256:ipad-{worker_id}"),
                });

                let names = [
                    "Alex Rivera",
                    "Sam Chen",
                    "Jordan Kowalski",
                    "Taylor Brooks",
                    "Morgan Patel",
                    "Casey Novak",
                ];
                let name = names[(self.workers.len()) % names.len()].to_string();

                let worker = WorkerState {
                    worker_id: worker_id.clone(),
                    name: name.clone(),
                    role: role.clone(),
                    plant_id: plant_id.clone(),
                    ipad_device_id,
                    wage_per_hour: Money(wage_per_hour_tenth_cents.max(25_000)),
                    hired_at: self.now,
                    ready_at,
                    worked_ms_in_domain: 0,
                    quit_risk_permille: 12,
                    current_pose: Pose2D::new(1_500, 1_500, 0),
                    motion_segments: Vec::new(),
                    active_task: None,
                    dispatch_count: 0,
                };
                self.workers.insert(worker_id.clone(), worker);

                self.pipeline.insert(
                    format!("pipe-hire-{worker_id}"),
                    PipelineItem {
                        item_id: format!("pipe-hire-{worker_id}"),
                        plant_id: plant_id.clone(),
                        category: "hiring".to_string(),
                        title: format!("Recruit & Onboard {name} ({role})"),
                        started_at: self.now,
                        eta_p10: self.now.add_ms(2 * SimTime::DAY_MS),
                        eta_p50: ready_at,
                        eta_p90: self.now.add_ms(5 * SimTime::DAY_MS),
                        actual_complete_at: ready_at,
                        cost: Money::from_dollars(750),
                        completed: false,
                    },
                );

                if self.tutorial_step == 3 {
                    self.tutorial_step = 4;
                }
                Ok(format!("Hired worker '{name}' ({worker_id})"))
            }

            PlantCommand::SetShiftPriority {
                plant_id: _,
                priorities,
            } => {
                if !priorities.is_empty() {
                    self.shift_priorities = priorities;
                }
                Ok("Updated shift task priorities".to_string())
            }

            PlantCommand::ConfigureMarketPolicy {
                plant_id,
                market_id,
                enabled,
                min_payout_per_hour_tenth_cents,
                price_multiplier_pct,
                promised_lead_days,
            } => {
                let policy = MarketBiddingPolicy {
                    market_id: market_id.clone(),
                    enabled,
                    min_payout_per_hour: Money(min_payout_per_hour_tenth_cents),
                    price_multiplier_pct: price_multiplier_pct.clamp(50, 200),
                    promised_lead_days: promised_lead_days.max(1),
                    max_active_backlog: 15,
                    allow_aluminum: true,
                    allow_steel: true,
                    allow_molded_plastic: true,
                };
                self.market_policies
                    .insert((plant_id.clone(), market_id.clone()), policy);
                if enabled && self.tutorial_step == 4 {
                    self.tutorial_step = 5;
                }
                Ok(format!("Updated market policy for '{market_id}'"))
            }

            PlantCommand::UnlockMarketCert { plant_id, cert_id } => {
                let cert: CertificationSpec = starter_certifications()
                    .into_iter()
                    .find(|c| c.cert_id == cert_id)
                    .ok_or_else(|| format!("Unknown certification '{cert_id}'"))?;
                self.ledger.post(
                    format!("cert-{cert_id}-{}", self.event_counter),
                    self.now,
                    plant_id.clone(),
                    "expense:certifications",
                    "asset:cash",
                    cert.cost,
                    format!("Certification audit & registration: {}", cert.name),
                    cert_id.clone(),
                );
                let complete_at = self.now.add_ms((cert.lead_days as i64) * SimTime::DAY_MS);
                self.pipeline.insert(
                    format!("pipe-cert-{cert_id}"),
                    PipelineItem {
                        item_id: format!("pipe-cert-{cert_id}"),
                        plant_id,
                        category: "certification".to_string(),
                        title: format!("Audit & Issue {}", cert.name),
                        started_at: self.now,
                        eta_p10: self
                            .now
                            .add_ms(((cert.lead_days.saturating_sub(3)) as i64) * SimTime::DAY_MS),
                        eta_p50: complete_at,
                        eta_p90: self
                            .now
                            .add_ms(((cert.lead_days + 5) as i64) * SimTime::DAY_MS),
                        actual_complete_at: complete_at,
                        cost: cert.cost,
                        completed: false,
                    },
                );
                if !self.unlocked_certs.contains(&cert_id) {
                    self.unlocked_certs.push(cert_id.clone());
                }
                if cert_id == "cert-hazmat" {
                    for p in self.plants.values_mut() {
                        p.hazmat_permit_approved = true;
                    }
                }
                Ok(format!("Started certification '{}'", cert.name))
            }

            PlantCommand::OrderRawStock {
                plant_id,
                sku,
                quantity,
            } => {
                let unit_cost = match sku.as_str() {
                    "carton-box" => Money::from_dollars(4),
                    "raw-bar-alu" => Money::from_dollars(48),
                    _ => Money::from_dollars(55),
                };
                let total_cost = Money(unit_cost.0 * (quantity.max(1) as i64));
                self.ledger.post(
                    format!("po-{}-{}", sku, self.event_counter),
                    self.now,
                    plant_id.clone(),
                    "asset:inventory",
                    "asset:cash",
                    total_cost,
                    format!("Purchased {quantity}x {sku}"),
                    sku.clone(),
                );
                let recv_id = format!("{plant_id}-recv");
                for i in 0..quantity {
                    self.buffers.put_to_rack(
                        &recv_id,
                        HandlingUnit {
                            unit_id: format!("po-unit-{}-{}-{i}", self.event_counter, sku),
                            spec: HandlingUnitSpec {
                                container_type: "tray".to_string(),
                                item_sku: sku.clone(),
                                quantity: 1,
                                lot_id: format!("LOT-{}-{}", self.event_counter, i),
                            },
                            heat_or_lot_cert: format!("CERT-{}-{}", sku, self.event_counter),
                            true_in_spec: true,
                            believed_in_spec: true,
                            created_at: self.now,
                        },
                    );
                }
                Ok(format!("Ordered and stocked {quantity}x {sku}"))
            }

            PlantCommand::TakeEquipmentLoan {
                loan_id,
                principal_tenth_cents,
                collateral_cell_id,
            } => {
                let cell = self
                    .cells
                    .get(&collateral_cell_id)
                    .ok_or_else(|| format!("Collateral cell '{collateral_cell_id}' not found"))?;
                let cat = Self::lookup_catalog(&cell.catalog_id).unwrap();
                if !cat.financeable {
                    return Err(format!(
                        "{} is not eligible for equipment financing",
                        cat.display_name
                    ));
                }
                let max_loan = Money((cat.price.0 * 80) / 100);
                let principal = Money(principal_tenth_cents.min(max_loan.0));
                let monthly_payment =
                    fsim_finance::reference::reference_monthly_loan_payment(principal, 850, 36);

                self.ledger.post(
                    format!("loan-orig-{loan_id}"),
                    self.now,
                    cell.plant_id.clone(),
                    "asset:cash",
                    "liability:equipment_loan",
                    principal,
                    format!("Equipment loan against {}", cell.cell_id),
                    loan_id.clone(),
                );
                self.loans.insert(
                    loan_id.clone(),
                    EquipmentLoan {
                        loan_id: loan_id.clone(),
                        collateral_cell_id,
                        original_principal: principal,
                        remaining_principal: principal,
                        apr_bps: 850,
                        monthly_payment,
                        originated_at: self.now,
                    },
                );
                Ok(format!("Originated equipment loan '{loan_id}'"))
            }
        }
    }

    fn install_support_cell_internal(
        &mut self,
        plant_id: &str,
        cell_id: &str,
        catalog_id: &str,
        x_mm: i32,
        y_mm: i32,
    ) {
        if let Some(catalog) = Self::lookup_catalog(catalog_id) {
            let device_id = format!("plant/{plant_id}/cell/{cell_id}/controller");
            self.net.register_device(DeviceIdentity {
                device_id: device_id.clone(),
                plant_id: plant_id.to_string(),
                entity_id: cell_id.to_string(),
                device_kind: "machine-controller".to_string(),
                cert_fingerprint: format!("sha256:dev-{cell_id}"),
            });
            self.register_cell_squares(cell_id, &catalog, x_mm, y_mm);
            self.cells.insert(
                cell_id.to_string(),
                InstalledCell {
                    cell_id: cell_id.to_string(),
                    plant_id: plant_id.to_string(),
                    catalog_id: catalog_id.to_string(),
                    team: catalog.team,
                    display_name: catalog.display_name,
                    addons: Vec::new(),
                    x_mm,
                    y_mm,
                    rotation_deg: 0,
                    ordered_at: self.now,
                    delivered_at: self.now,
                    commissioned_at: self.now,
                    device_id,
                    active_order_id: None,
                    cycle_finish_at: None,
                    tool_wear_pct: 0,
                    chip_hopper_pct: 0,
                    down_for_repair: false,
                    status_since: self.now,
                    cycles_completed: 0,
                },
            );
        }
    }

    fn register_cell_squares(
        &mut self,
        cell_id: &str,
        catalog: &CellCatalogEntry,
        x_mm: i32,
        y_mm: i32,
    ) {
        for (idx, (sq_name, skus)) in catalog.input_squares.iter().enumerate() {
            self.buffers.register_square(HandoffSquare {
                cell_id: cell_id.to_string(),
                square_name: sq_name.clone(),
                is_input: true,
                position_mm: PointMm::new(x_mm, y_mm + (idx as i32) * 500),
                accepted_skus: skus.clone(),
                occupant: None,
            });
        }
        if !catalog.output_square_skus.is_empty() {
            self.buffers.register_square(HandoffSquare {
                cell_id: cell_id.to_string(),
                square_name: "out".to_string(),
                is_input: false,
                position_mm: PointMm::new(x_mm + catalog.width_mm, y_mm + catalog.depth_mm / 2),
                accepted_skus: catalog.output_square_skus.clone(),
                occupant: None,
            });
        }
    }

    fn update_cell_square_positions(
        &mut self,
        cell_id: &str,
        catalog: &CellCatalogEntry,
        x_mm: i32,
        y_mm: i32,
    ) {
        for (idx, (sq_name, _)) in catalog.input_squares.iter().enumerate() {
            if let Some(sq) = self
                .buffers
                .squares
                .get_mut(&(cell_id.to_string(), sq_name.clone()))
            {
                sq.position_mm = PointMm::new(x_mm, y_mm + (idx as i32) * 500);
            }
        }
        if let Some(sq) = self
            .buffers
            .squares
            .get_mut(&(cell_id.to_string(), "out".to_string()))
        {
            sq.position_mm = PointMm::new(x_mm + catalog.width_mm, y_mm + catalog.depth_mm / 2);
        }
    }

    fn rebuild_transport_graph(&mut self, plant_id: &str) {
        let Some(plant) = self.plants.get_mut(plant_id) else {
            return;
        };
        plant.layout_version += 1;
        let layout_ver = plant.layout_version;
        let Some(listing) = Self::lookup_listing(&plant.listing_id) else {
            return;
        };

        let mut squares = Vec::new();
        for sq in self.buffers.squares.values() {
            if self
                .cells
                .get(&sq.cell_id)
                .map(|c| c.plant_id == plant_id)
                .unwrap_or(false)
            {
                squares.push(SquareEndpoint {
                    cell_id: sq.cell_id.clone(),
                    square_name: sq.square_name.clone(),
                    access_point: sq.position_mm,
                });
            }
        }

        // We pass empty obstacles for square-to-square access points so squares on cell boundaries remain reachable,
        // while building walls and interior column holes are strictly respected.
        let graph = TransportGraph::build(layout_ver, &listing.floor_polygon, &[], &squares);
        self.transport_graphs.insert(plant_id.to_string(), graph);
    }

    /// Returns the earliest timestamp at which any module, worker, cell, pipeline item, or market poll expects an event.
    pub fn next_event_at(&self) -> Option<SimTime> {
        let mut earliest: Option<SimTime> = None;
        let mut consider = |t: SimTime| {
            if t > self.now {
                earliest = Some(earliest.map_or(t, |e| e.min(t)));
            }
        };

        consider(self.next_market_poll_at);
        consider(self.next_monthly_overhead_at);

        for p in self.pipeline.values() {
            if !p.completed {
                consider(p.actual_complete_at);
            }
        }
        if !self.pending_tasks.is_empty() {
            for w in self.workers.values() {
                if !w.is_onboarded(self.now) {
                    consider(w.ready_at);
                }
            }
        }
        for c in self.cells.values() {
            if let Some(fin) = c.cycle_finish_at {
                consider(fin);
            }
        }
        for w in self.workers.values() {
            if let Some(act) = &w.active_task {
                consider(act.work_done_at);
            }
        }
        earliest
    }

    /// Advances the simulation to `target_time` by jumping deterministically from event to event.
    pub fn advance_to(&mut self, target_time: SimTime) {
        while self.now < target_time {
            let next_t = self
                .next_event_at()
                .map(|t| t.min(target_time))
                .unwrap_or(target_time);
            let prev_now = self.now;
            self.step_to(next_t);
            if self.now == prev_now {
                break;
            }
        }
    }

    /// Skips ahead until the next event (or up to `max_horizon_ms`), returning a `SkipSummaryCard`
    /// with cash and runway curves from start to end of the skipped stretch.
    pub fn skip_ahead(&mut self, max_horizon_ms: i64) -> SkipSummaryCard {
        let from_time = self.now;
        let cash_start = self.ledger.cash_balance();
        let runway_start = self.runway_projection().runway_days_p50;
        let start_log_len = self.gateway.log_entries.len();

        let target_limit = self.now.add_ms(max_horizon_ms.max(SimTime::HOUR_MS));
        let mut events_occurred = Vec::new();
        let mut stop_reason = "Reached target horizon".to_string();

        while self.now < target_limit {
            let next_t = self
                .next_event_at()
                .map(|t| t.min(target_limit))
                .unwrap_or(target_limit);
            let prev_orders_shipped = self
                .customer_orders
                .values()
                .filter(|o| o.status != "active")
                .count();
            let prev_pipe_done = self.pipeline.values().filter(|p| p.completed).count();

            self.step_to(next_t);

            let new_pipe_done = self.pipeline.values().filter(|p| p.completed).count();
            if new_pipe_done > prev_pipe_done {
                for p in self.pipeline.values() {
                    if p.completed && p.actual_complete_at == self.now {
                        events_occurred.push(format!("Completed: {}", p.title));
                    }
                }
                stop_reason = "Pipeline item completed".to_string();
                break;
            }

            let new_orders_shipped = self
                .customer_orders
                .values()
                .filter(|o| o.status != "active")
                .count();
            if new_orders_shipped > prev_orders_shipped {
                events_occurred.push("Customer shipment completed & paid".to_string());
                stop_reason = "Customer shipment completed".to_string();
                break;
            }

            if self.ledger.cash_balance().0 + self.credit_limit.0 < 0 {
                events_occurred.push("Company ran out of cash and credit (Bankruptcy)".to_string());
                stop_reason = "Bankruptcy".to_string();
                break;
            }
        }

        if events_occurred.is_empty() {
            let new_msgs = self.gateway.log_entries.len().saturating_sub(start_log_len);
            events_occurred.push(format!("Processed {new_msgs} device telemetry messages"));
        }

        let cash_end = self.ledger.cash_balance();
        let runway_end = self.runway_projection().runway_days_p50;
        let days_skipped_tenths = ((self.now.0 - from_time.0) * 10) / SimTime::DAY_MS;

        SkipSummaryCard {
            from_time,
            to_time: self.now,
            days_skipped_tenths,
            cash_start,
            cash_end,
            runway_days_start: runway_start,
            runway_days_end: runway_end,
            stop_reason,
            events_occurred,
        }
    }

    /// Executes one deterministic multi-pass step at timestamp `new_now` ("repeat until quiet").
    pub fn step_to(&mut self, new_now: SimTime) {
        if new_now < self.now {
            return;
        }
        self.now = new_now;
        self.version += 1;

        // 1. Complete any due pipeline items (plant keys/utilities, machine deliveries, hires, certs)
        for p in self.pipeline.values_mut() {
            if !p.completed && self.now >= p.actual_complete_at {
                p.completed = true;
            }
        }
        for site in self.plants.values_mut() {
            if self.now >= site.keys_ready_at {
                site.keys_received = true;
            }
            if self.now >= site.utilities_ready_at {
                site.utilities_ready = true;
            }
        }

        // 2. Poll markets for stochastic customer order arrivals
        if self.now >= self.next_market_poll_at {
            let poll_time = self.next_market_poll_at;
            self.next_market_poll_at = poll_time.add_ms(SimTime::from_hours(4).0);
            self.poll_markets_at(poll_time);
        }

        // 3. Process monthly overhead (rent, payroll, equipment loan payments)
        if self.now >= self.next_monthly_overhead_at {
            let bill_time = self.next_monthly_overhead_at;
            self.next_monthly_overhead_at = bill_time.add_ms(SimTime::from_days(30).0);
            self.process_monthly_overhead_at(bill_time);
        }

        // 5. Repeat-until-quiet passes
        loop {
            let mut moved_anything = false;
            if self.complete_worker_tasks() {
                moved_anything = true;
            }
            if self.step_work_cells() {
                moved_anything = true;
            }
            if !refresh_order_unblocking(&mut self.internal_orders).is_empty() {
                moved_anything = true;
            }
            if self.schedule_unblocked_orders() {
                moved_anything = true;
            }
            if self.dispatch_idle_workers() {
                moved_anything = true;
            }
            if !moved_anything {
                break;
            }
        }

        // 5. Drain due frames over `fsim-net` into `fsim-gateway` (tamper-evident hash chain)
        let due_frames = self.net.drain_due_frames(self.now);
        self.gateway.ingest_frames(due_frames);
    }

    fn poll_markets_at(&mut self, poll_time: SimTime) {
        let markets = starter_markets();
        let plant_ids: Vec<String> = self.plants.keys().cloned().collect();
        for pid in plant_ids {
            let is_ready = self
                .plants
                .get(&pid)
                .map(|p| p.is_operational(poll_time))
                .unwrap_or(false);
            if !is_ready {
                continue;
            }
            let has_saw = self
                .cells
                .values()
                .any(|c| c.plant_id == pid && c.catalog_id.contains("bandsaw") && c.is_commissioned(poll_time));
            let has_insp = self
                .cells
                .values()
                .any(|c| c.plant_id == pid && c.team == TeamTag::Quality && c.is_commissioned(poll_time));

            for mkt in &markets {
                let Some(policy) = self
                    .market_policies
                    .get(&(pid.clone(), mkt.market_id.clone()))
                    .cloned()
                else {
                    continue;
                };
                if !policy.enabled {
                    continue;
                }
                // Check required certs
                if !mkt
                    .required_certs
                    .iter()
                    .all(|rc| self.unlocked_certs.contains(rc))
                {
                    continue;
                }
                let active_backlog = self
                    .customer_orders
                    .values()
                    .filter(|o| o.plant_id == pid && o.status == "active")
                    .count() as u32;
                if active_backlog >= policy.max_active_backlog {
                    continue;
                }

                self.event_counter += 1;
                let rep = *self
                    .market_reputation_permille
                    .get(&mkt.market_id)
                    .unwrap_or(&650);
                if let Some(cust) = sample_customer_order(
                    &pid,
                    mkt,
                    &policy,
                    rep,
                    has_saw,
                    has_insp,
                    self.event_counter,
                    poll_time,
                    self.game_seed,
                ) {
                    let chain = create_backward_order_chain(&cust);
                    for ord in chain {
                        self.internal_orders.insert(ord.order_id.clone(), ord);
                    }
                    self.customer_orders
                        .insert(cust.customer_order_id.clone(), cust);
                }
            }
        }
    }

    fn process_monthly_overhead_at(&mut self, bill_time: SimTime) {
        for site in self.plants.values() {
            if let Some(listing) = Self::lookup_listing(&site.listing_id) {
                self.ledger.post(
                    format!("rent-{}-{}", site.plant_id, bill_time.0),
                    bill_time,
                    site.plant_id.clone(),
                    "expense:rent",
                    "asset:cash",
                    listing.monthly_rent,
                    format!("Monthly lease rent for {}", site.name),
                    site.plant_id.clone(),
                );
            }
        }
        for w in self.workers.values() {
            let monthly_pay = Money(w.wage_per_hour.0 * 160);
            self.ledger.post(
                format!("payroll-{}-{}", w.worker_id, bill_time.0),
                bill_time,
                w.plant_id.clone(),
                "expense:payroll",
                "asset:cash",
                monthly_pay,
                format!("Monthly payroll for {} ({})", w.name, w.role),
                w.worker_id.clone(),
            );
        }
        for loan in self.loans.values_mut() {
            if loan.remaining_principal.0 > 0 {
                let pay = loan.monthly_payment;
                loan.remaining_principal =
                    Money((loan.remaining_principal.0 - (pay.0 * 80) / 100).max(0));
                self.ledger.post(
                    format!("loan-pay-{}-{}", loan.loan_id, bill_time.0),
                    bill_time,
                    "company".to_string(),
                    "liability:equipment_loan",
                    "asset:cash",
                    pay,
                    format!("Monthly equipment loan payment ({})", loan.loan_id),
                    loan.loan_id.clone(),
                );
            }
        }
    }

    fn complete_worker_tasks(&mut self) -> bool {
        let mut changed = false;
        let worker_ids: Vec<String> = self.workers.keys().cloned().collect();
        for wid in worker_ids {
            let Some(active) = self
                .workers
                .get(&wid)
                .and_then(|w| w.active_task.clone())
            else {
                continue;
            };
            if self.now < active.work_done_at {
                continue;
            }

            // Task finished! Update worker domain experience and clear active task
            let ipad_dev = {
                let w = self.workers.get_mut(&wid).unwrap();
                let dur = (active.work_done_at.0 - active.walk_done_at.0).max(1_000);
                w.worked_ms_in_domain += dur;
                w.active_task = None;
                w.ipad_device_id.clone()
            };

            // Emit iPad task completion telemetry over `fsim-net`
            let _ = self.net.send_frame(
                self.game_seed,
                &ipad_dev,
                FrameDirection::DeviceToGateway,
                Envelope {
                    team_tag: TeamTag::PeopleOps,
                    schema_id: "fsim.people.v1.TaskCompleted".to_string(),
                    schema_version: 1,
                    source_device: ipad_dev.clone(),
                    entity_id: wid.clone(),
                    plant_timestamp_ms: self.now.0,
                    receive_timestamp_ms: self.now.0,
                    sequence_number: 0,
                    correlation_id: active.task.order_id.clone(),
                    payload: active.task.task_id.as_bytes().to_vec(),
                },
                self.now,
            );

            match &active.task.kind {
                OperatorTaskKind::MoveHandlingUnit {
                    from_cell,
                    from_square,
                    to_cell,
                    to_square,
                    spec,
                } => {
                    // Take from rack or output square and put into destination input square
                    let unit = if from_square == "rack" {
                        self.buffers
                            .take_sku_from_rack(from_cell, &spec.item_sku)
                            .unwrap_or_else(|| HandlingUnit {
                                unit_id: format!("auto-replenish-{}", active.task.task_id),
                                spec: spec.clone(),
                                heat_or_lot_cert: "CERT-REPLENISHED".to_string(),
                                true_in_spec: true,
                                believed_in_spec: true,
                                created_at: self.now,
                            })
                    } else {
                        self.buffers
                            .take_from_square(from_cell, from_square)
                            .unwrap_or_else(|| HandlingUnit {
                                unit_id: format!("moved-{}", active.task.task_id),
                                spec: spec.clone(),
                                heat_or_lot_cert: "CERT-MOVED".to_string(),
                                true_in_spec: true,
                                believed_in_spec: true,
                                created_at: self.now,
                            })
                    };
                    let _ = self.buffers.put_to_square(to_cell, to_square, unit);
                    self.active_logistics_orders
                        .retain(|_, tid| tid != &active.task.task_id);
                }
                OperatorTaskKind::EmptyChipHopper { cell_id } => {
                    if let Some(c) = self.cells.get_mut(cell_id) {
                        c.chip_hopper_pct = 0;
                    }
                }
                OperatorTaskKind::RepairCell { cell_id } => {
                    if let Some(c) = self.cells.get_mut(cell_id) {
                        c.down_for_repair = false;
                        c.tool_wear_pct = 0;
                    }
                }
                OperatorTaskKind::OperateCell { .. } => {}
            }
            changed = true;
        }
        changed
    }

    fn schedule_unblocked_orders(&mut self) -> bool {
        let mut changed = false;
        let order_ids: Vec<String> = self.internal_orders.keys().cloned().collect();

        for oid in order_ids {
            let Some(order) = self.internal_orders.get(&oid).cloned() else {
                continue;
            };
            if order.status != InternalOrderStatus::ReadyForLogistics {
                continue;
            }
            let plant_id = self
                .customer_orders
                .get(&order.customer_order_id)
                .map(|c| c.plant_id.clone())
                .unwrap_or_else(|| "plant-1".to_string());

            // Case 1: Leaf Stock Order (`required_inputs.is_empty()`)
            // Fulfilled from the plant's Receiving/Rack stock (and auto-reorders from vendor if stock is low).
            if order.required_inputs.is_empty() {
                let recv_id = format!("{plant_id}-recv");
                let has_sku = self
                    .buffers
                    .shared_racks
                    .get(&recv_id)
                    .map(|v| v.iter().any(|u| u.spec.item_sku == order.output_spec.item_sku))
                    .unwrap_or(false);
                if !has_sku {
                    // Auto-replenish vendor purchase order so the pull chain never deadlocks
                    let unit_cost = if order.output_spec.item_sku == "carton-box" {
                        Money::from_dollars(4)
                    } else {
                        Money::from_dollars(48)
                    };
                    self.ledger.post(
                        format!("auto-po-{oid}"),
                        self.now,
                        plant_id.clone(),
                        "asset:inventory",
                        "asset:cash",
                        unit_cost,
                        format!("Auto-replenish PO for {}", order.output_spec.item_sku),
                        oid.clone(),
                    );
                    self.buffers.put_to_rack(
                        &recv_id,
                        HandlingUnit {
                            unit_id: format!("unit-{oid}"),
                            spec: order.output_spec.clone(),
                            heat_or_lot_cert: format!("CERT-{oid}"),
                            true_in_spec: true,
                            believed_in_spec: true,
                            created_at: self.now,
                        },
                    );
                }
                self.internal_orders.get_mut(&oid).unwrap().status = InternalOrderStatus::Completed;
                changed = true;
                continue;
            }

            // Case 2: Work-Cell Order -> Assign a target cell on the plant floor if not assigned yet
            let target_cell_id = if !order.target_cell_id.is_empty() {
                order.target_cell_id.clone()
            } else {
                let candidate = self
                    .cells
                    .values()
                    .find(|c| {
                        c.plant_id == plant_id
                            && c.team == order.target_team
                            && c.is_commissioned(self.now)
                            && self.cell_matches_order(c, &order)
                    })
                    .map(|c| c.cell_id.clone());
                let Some(cid) = candidate else {
                    continue;
                };
                self.internal_orders.get_mut(&oid).unwrap().target_cell_id = cid.clone();
                cid
            };

            // Check if another order on `target_cell_id` is already in `InputsStaged` or `InProcess`
            // or already has an active logistics move filling `target_cell_id`
            let cell_busy_with_other = self.internal_orders.values().any(|other| {
                other.order_id != oid
                    && other.target_cell_id == target_cell_id
                    && matches!(
                        other.status,
                        InternalOrderStatus::InputsStaged | InternalOrderStatus::InProcess
                    )
            }) || self.pending_tasks.iter().any(|t| {
                t.order_id != oid
                    && matches!(&t.kind, OperatorTaskKind::MoveHandlingUnit { to_cell, .. } if to_cell == &target_cell_id)
            }) || self.workers.values().any(|w| {
                w.active_task.as_ref().map(|at| {
                    at.task.order_id != oid
                        && matches!(&at.task.kind, OperatorTaskKind::MoveHandlingUnit { to_cell, .. } if to_cell == &target_cell_id)
                }).unwrap_or(false)
            });
            if cell_busy_with_other {
                continue;
            }

            // Check if all required named input squares on `target_cell_id` are already filled
            // with the item SKU required by this order.
            let mut all_staged = true;
            for req in &order.required_inputs {
                let filled_matching = self
                    .buffers
                    .get_square(&target_cell_id, &req.square_name)
                    .and_then(|s| s.occupant.as_ref())
                    .map(|u| u.spec.item_sku == req.spec.item_sku)
                    .unwrap_or(false);
                if !filled_matching {
                    all_staged = false;
                    let sq_key = format!("sq:{target_cell_id}:{}", req.square_name);
                    if !self.active_logistics_orders.contains_key(&sq_key)
                        && self
                            .buffers
                            .is_square_empty(&target_cell_id, &req.square_name)
                    {
                        // Find upstream source (either an upstream cell's `out` square or the plant's rack)
                        if let Some((src_cell, src_sq, src_pt)) =
                            self.find_input_source(&plant_id, &req.spec.item_sku)
                        {
                            let dst_pt = self
                                .buffers
                                .get_square(&target_cell_id, &req.square_name)
                                .map(|s| s.position_mm)
                                .unwrap_or(PointMm::new(3_000, 3_000));
                            self.event_counter += 1;
                            let task_id = format!("task-move-{}", self.event_counter);
                            self.active_logistics_orders
                                .insert(sq_key, task_id.clone());
                            self.pending_tasks.push(PendingFloorTask {
                                task_id,
                                plant_id: plant_id.clone(),
                                category: "logistics".to_string(),
                                order_id: oid.clone(),
                                location_mm: src_pt,
                                destination_mm: dst_pt,
                                base_duration_ms: 5_000,
                                created_at: self.now,
                                kind: OperatorTaskKind::MoveHandlingUnit {
                                    from_cell: src_cell,
                                    from_square: src_sq,
                                    to_cell: target_cell_id.clone(),
                                    to_square: req.square_name.clone(),
                                    spec: req.spec.clone(),
                                },
                            });
                            changed = true;
                        }
                    }
                }
            }

            if all_staged {
                self.internal_orders.get_mut(&oid).unwrap().status =
                    InternalOrderStatus::InputsStaged;
                changed = true;
            }
        }
        changed
    }

    fn cell_matches_order(&self, cell: &InstalledCell, order: &InternalOrder) -> bool {
        let Some(cat) = Self::lookup_catalog(&cell.catalog_id) else {
            return false;
        };
        if order.output_spec.container_type == "shipment" {
            return cell.catalog_id == "mat-shipping-bay";
        }
        cat.output_square_skus.contains(&order.output_spec.item_sku)
    }

    fn find_input_source(
        &self,
        plant_id: &str,
        item_sku: &str,
    ) -> Option<(String, String, PointMm)> {
        // 1. Check any cell's `out` square on the same plant holding `item_sku`
        // that is not already claimed by an in-flight logistics task.
        for sq in self.buffers.squares.values() {
            if !sq.is_input {
                if let Some(occ) = &sq.occupant {
                    if occ.spec.item_sku == item_sku {
                        let already_claimed = self.pending_tasks.iter().any(|t| {
                            matches!(&t.kind, OperatorTaskKind::MoveHandlingUnit { from_cell, from_square, .. }
                                if from_cell == &sq.cell_id && from_square == &sq.square_name)
                        }) || self.workers.values().any(|w| {
                            w.active_task.as_ref().map(|at| {
                                matches!(&at.task.kind, OperatorTaskKind::MoveHandlingUnit { from_cell, from_square, .. }
                                    if from_cell == &sq.cell_id && from_square == &sq.square_name)
                            }).unwrap_or(false)
                        });
                        if !already_claimed
                            && self
                                .cells
                                .get(&sq.cell_id)
                                .map(|c| c.plant_id == plant_id)
                                .unwrap_or(false)
                        {
                            return Some((
                                sq.cell_id.clone(),
                                sq.square_name.clone(),
                                sq.position_mm,
                            ));
                        }
                    }
                }
            }
        }
        // 2. Check the plant's Receiving/Rack storage
        let recv_id = format!("{plant_id}-recv");
        if self
            .buffers
            .shared_racks
            .get(&recv_id)
            .map(|v| v.iter().any(|u| u.spec.item_sku == item_sku))
            .unwrap_or(false)
        {
            let pt = self
                .cells
                .get(&recv_id)
                .map(|c| PointMm::new(c.x_mm, c.y_mm))
                .unwrap_or(PointMm::new(500, 500));
            return Some((recv_id, "rack".to_string(), pt));
        }
        None
    }

    fn step_work_cells(&mut self) -> bool {
        let mut changed = false;
        let cell_ids: Vec<String> = self.cells.keys().cloned().collect();

        for cid in cell_ids {
            let Some(cell) = self.cells.get(&cid).cloned() else {
                continue;
            };
            if !cell.is_commissioned(self.now) {
                continue;
            }

            // 1. If cell is currently running a cycle and `now >= cycle_finish_at`, finish the cycle!
            if let (Some(oid), Some(fin_at)) = (&cell.active_order_id, cell.cycle_finish_at) {
                if self.now >= fin_at {
                    let Some(order) = self.internal_orders.get(oid).cloned() else {
                        continue;
                    };

                    // Shipping Bay finishes a shipment directly to the customer carrier
                    if cell.catalog_id == "mat-shipping-bay" {
                        self.complete_shipping_order(&cid, &order);
                        changed = true;
                        continue;
                    }

                    // Other cells require their `out` square to be empty before releasing the finished part
                    if self.buffers.is_square_empty(&cid, "out") {
                        self.complete_cell_cycle(&cid, &order);
                        changed = true;
                    }
                }
                continue;
            }

            // 2. If cell is idle and chip hopper is full (>= 100%), request hopper dump task
            if cell.chip_hopper_pct >= 100 {
                let already_queued = self.pending_tasks.iter().any(|t| {
                    matches!(&t.kind, OperatorTaskKind::EmptyChipHopper { cell_id } if cell_id == &cid)
                });
                if !already_queued {
                    self.event_counter += 1;
                    self.pending_tasks.push(PendingFloorTask {
                        task_id: format!("task-hopper-{}", self.event_counter),
                        plant_id: cell.plant_id.clone(),
                        category: "waste".to_string(),
                        order_id: String::new(),
                        location_mm: PointMm::new(cell.x_mm, cell.y_mm),
                        destination_mm: PointMm::new(cell.x_mm, cell.y_mm),
                        base_duration_ms: 15_000,
                        created_at: self.now,
                        kind: OperatorTaskKind::EmptyChipHopper {
                            cell_id: cid.clone(),
                        },
                    });
                }
                continue;
            }

            // 3. Look for an order targeted at `cid` in `InputsStaged` state
            let staged_oid = self
                .internal_orders
                .values()
                .find(|o| {
                    o.target_cell_id == cid && o.status == InternalOrderStatus::InputsStaged
                })
                .map(|o| o.order_id.clone());

            if let Some(oid) = staged_oid {
                self.start_cell_cycle(&cid, &oid);
                changed = true;
            }
        }
        changed
    }

    fn start_cell_cycle(&mut self, cell_id: &str, order_id: &str) {
        let Some(cell) = self.cells.get(cell_id).cloned() else {
            return;
        };
        let Some(order) = self.internal_orders.get(order_id).cloned() else {
            return;
        };

        // Consume all required named input squares into the machine
        for req in &order.required_inputs {
            let _ = self.buffers.take_from_square(cell_id, &req.square_name);
        }

        let cycle_ms = match cell.team {
            TeamTag::Machining => {
                let cfg = MachiningCellConfig::from_addons(&cell.addons);
                reference_cycle_time_ms(180_000, &cfg) // 3 mins scaled for brisk gameplay
            }
            TeamTag::FinishingPacking => reference_packing_time_ms(&cell.catalog_id),
            TeamTag::MaterialsLogistics => {
                if cell.catalog_id == "mat-shipping-bay" {
                    15_000
                } else {
                    reference_saw_cut_ms(&cell.catalog_id)
                }
            }
            TeamTag::Quality => 30_000,
            _ => 60_000,
        };

        let finish_at = self.now.add_ms(cycle_ms);
        if let Some(c) = self.cells.get_mut(cell_id) {
            c.active_order_id = Some(order_id.to_string());
            c.cycle_finish_at = Some(finish_at);
            c.status_since = self.now;
        }
        if let Some(o) = self.internal_orders.get_mut(order_id) {
            o.status = InternalOrderStatus::InProcess;
        }

        // Send controller observation over `fsim-net`
        let _ = self.net.send_frame(
            self.game_seed,
            &cell.device_id,
            FrameDirection::DeviceToGateway,
            Envelope {
                team_tag: cell.team,
                schema_id: format!("fsim.{}.v1.CycleStarted", cell.team.as_str()),
                schema_version: 1,
                source_device: cell.device_id.clone(),
                entity_id: cell_id.to_string(),
                plant_timestamp_ms: self.now.0,
                receive_timestamp_ms: self.now.0,
                sequence_number: 0,
                correlation_id: order_id.to_string(),
                payload: order_id.as_bytes().to_vec(),
            },
            self.now,
        );
    }

    fn complete_cell_cycle(&mut self, cell_id: &str, order: &InternalOrder) {
        let Some(cell) = self.cells.get(cell_id).cloned() else {
            return;
        };
        self.event_counter += 1;
        let mut rng = StreamRng::new(self.game_seed, "cell-finish", cell_id, self.event_counter);

        let mut true_in_spec = true;
        if cell.team == TeamTag::Machining {
            let cfg = MachiningCellConfig::from_addons(&cell.addons);
            let scrap_pm = reference_scrap_permille(cell.tool_wear_pct, cfg.has_in_process_probing);
            if (rng.gen_range_i64(0, 999) as u32) < scrap_pm {
                true_in_spec = false;
            }
        } else if cell.team == TeamTag::Quality {
            let climate_ok = self
                .plants
                .get(&cell.plant_id)
                .map(|p| p.climate_room_built)
                .unwrap_or(false);
            let (tp_pm, _) = reference_inspection_odds_permille(&cell.catalog_id, climate_ok);
            if (rng.gen_range_i64(0, 999) as u32) < tp_pm {
                true_in_spec = true; // Inspection catches and reworks/filters defects before packing
            }
        }

        let out_unit = HandlingUnit {
            unit_id: format!("hu-{}", order.order_id),
            spec: order.output_spec.clone(),
            heat_or_lot_cert: format!("CERT-{}", order.order_id),
            true_in_spec,
            believed_in_spec: true,
            created_at: self.now,
        };
        let _ = self.buffers.put_to_square(cell_id, "out", out_unit);

        if let Some(c) = self.cells.get_mut(cell_id) {
            c.active_order_id = None;
            c.cycle_finish_at = None;
            c.cycles_completed += 1;
            c.status_since = self.now;
            if c.team == TeamTag::Machining {
                c.tool_wear_pct = (c.tool_wear_pct + 4).min(95);
                let hopper_step = if c.addons.iter().any(|a| a == "chip-conveyor") {
                    4
                } else {
                    8
                };
                c.chip_hopper_pct = (c.chip_hopper_pct + hopper_step).min(100);
            }
        }

        if let Some(o) = self.internal_orders.get_mut(&order.order_id) {
            o.status = InternalOrderStatus::Completed;
        }

        let _ = self.net.send_frame(
            self.game_seed,
            &cell.device_id,
            FrameDirection::DeviceToGateway,
            Envelope {
                team_tag: cell.team,
                schema_id: format!("fsim.{}.v1.CycleCompleted", cell.team.as_str()),
                schema_version: 1,
                source_device: cell.device_id.clone(),
                entity_id: cell_id.to_string(),
                plant_timestamp_ms: self.now.0,
                receive_timestamp_ms: self.now.0,
                sequence_number: 0,
                correlation_id: order.order_id.clone(),
                payload: order.order_id.as_bytes().to_vec(),
            },
            self.now,
        );
    }

    fn complete_shipping_order(&mut self, cell_id: &str, order: &InternalOrder) {
        if let Some(c) = self.cells.get_mut(cell_id) {
            c.active_order_id = None;
            c.cycle_finish_at = None;
            c.cycles_completed += 1;
            c.status_since = self.now;
        }
        if let Some(o) = self.internal_orders.get_mut(&order.order_id) {
            o.status = InternalOrderStatus::Completed;
        }

        if let Some(cust) = self
            .customer_orders
            .get(&order.customer_order_id)
            .cloned()
        {
            let (payout, rep_delta, on_time) =
                reference_settle_shipment(&cust, self.now.0, true);
            if payout.0 >= 0 {
                self.ledger.post(
                    format!("rev-{}", cust.customer_order_id),
                    self.now,
                    cust.plant_id.clone(),
                    "asset:cash",
                    "revenue:product_sales",
                    payout,
                    format!("Shipment payout for {} ({})", cust.part_name, cust.customer_order_id),
                    cust.customer_order_id.clone(),
                );
            } else {
                self.ledger.post(
                    format!("chargeback-{}", cust.customer_order_id),
                    self.now,
                    cust.plant_id.clone(),
                    "expense:quality_chargeback",
                    "asset:cash",
                    Money(-payout.0),
                    format!("Out-of-tolerance chargeback for {}", cust.customer_order_id),
                    cust.customer_order_id.clone(),
                );
            }
            let rep = self
                .market_reputation_permille
                .entry(cust.market_id.clone())
                .or_insert(650);
            *rep = ((*rep as i32) + rep_delta).clamp(100, 990) as u32;

            if let Some(co) = self.customer_orders.get_mut(&cust.customer_order_id) {
                co.status = if on_time {
                    "shipped_on_time".to_string()
                } else {
                    "shipped_late".to_string()
                };
            }
            if self.tutorial_step == 5 {
                self.tutorial_step = 0; // Tutorial complete!
            }
        }
    }

    fn dispatch_idle_workers(&mut self) -> bool {
        let mut changed = false;
        let worker_ids: Vec<String> = self.workers.keys().cloned().collect();
        for wid in worker_ids {
            let is_ready_and_idle = self
                .workers
                .get(&wid)
                .map(|w| w.is_onboarded(self.now) && w.is_idle())
                .unwrap_or(false);
            if !is_ready_and_idle || self.pending_tasks.is_empty() {
                continue;
            }
            let worker_snapshot = self.workers.get(&wid).unwrap().clone();
            if let Some(idx) = reference_select_next_task_index(
                &worker_snapshot,
                &self.pending_tasks,
                &self.shift_priorities,
            ) {
                let task = self.pending_tasks.remove(idx);
                let w = self.workers.get_mut(&wid).unwrap();
                let active = dispatch_task_to_worker(
                    w,
                    task.clone(),
                    &[],
                    &[],
                    self.now,
                    self.game_seed,
                );
                let ipad_dev = w.ipad_device_id.clone();

                let _ = self.net.send_frame(
                    self.game_seed,
                    &ipad_dev,
                    FrameDirection::GatewayToDevice,
                    Envelope {
                        team_tag: TeamTag::PeopleOps,
                        schema_id: "fsim.people.v1.TaskDispatched".to_string(),
                        schema_version: 1,
                        source_device: ipad_dev.clone(),
                        entity_id: wid,
                        plant_timestamp_ms: self.now.0,
                        receive_timestamp_ms: active.ipad_ack_done_at.0,
                        sequence_number: 0,
                        correlation_id: task.order_id,
                        payload: task.task_id.as_bytes().to_vec(),
                    },
                    self.now,
                );
                changed = true;
            }
        }
        changed
    }

    /// Computes the "Why isn't this running?" plain-language status for every installed cell.
    pub fn cell_status_reports(&self) -> Vec<CellStatusReport> {
        let mut reports = Vec::new();
        for cell in self.cells.values() {
            let Some(plant) = self.plants.get(&cell.plant_id) else {
                continue;
            };
            let (state, reason, message, blocker_link) = if !plant.is_operational(self.now) {
                (
                    CellRunState::Blocked,
                    BlockerReason::NoPowerOrUtilities,
                    "Waiting for plant lease keys & utility hookup".to_string(),
                    Some(format!("pipe-keys-{}", cell.plant_id)),
                )
            } else if self.now < cell.commissioned_at {
                (
                    CellRunState::Installing,
                    BlockerReason::WaitingOnDelivery,
                    "Machine delivery & commissioning in progress".to_string(),
                    Some(format!("pipe-cell-{}", cell.cell_id)),
                )
            } else if cell.down_for_repair {
                (
                    CellRunState::Down,
                    BlockerReason::DownForRepair,
                    "Down for maintenance repair".to_string(),
                    None,
                )
            } else if cell.chip_hopper_pct >= 100 {
                (
                    CellRunState::Blocked,
                    BlockerReason::ChipHopperFull,
                    "Chip hopper 100% full — waiting for operator to dump swarf".to_string(),
                    None,
                )
            } else if let Some(oid) = &cell.active_order_id {
                if self.buffers.is_square_filled(&cell.cell_id, "out")
                    && self
                        .now
                        >= cell.cycle_finish_at.unwrap_or(self.now)
                {
                    (
                        CellRunState::Blocked,
                        BlockerReason::OutputSquareFull,
                        format!("Cycle finished on {oid}, but output square `out` is full"),
                        Some(format!("{}:out", cell.cell_id)),
                    )
                } else {
                    (
                        CellRunState::Running,
                        BlockerReason::None,
                        format!("Running order {oid}"),
                        Some(oid.clone()),
                    )
                }
            } else {
                // Idle: explain why it isn't running
                let waiting_order = self
                    .internal_orders
                    .values()
                    .find(|o| o.target_cell_id == cell.cell_id && o.status != InternalOrderStatus::Completed);
                if let Some(ord) = waiting_order {
                    (
                        CellRunState::Blocked,
                        BlockerReason::WaitingForMaterial,
                        format!("Waiting for operator / material on input square for {}", ord.order_id),
                        Some(ord.order_id.clone()),
                    )
                } else {
                    (
                        CellRunState::Idle,
                        BlockerReason::NoActiveOrders,
                        "Idle — no unblocked work orders queued for this cell".to_string(),
                        None,
                    )
                }
            };

            reports.push(CellStatusReport {
                cell_id: cell.cell_id.clone(),
                state,
                reason,
                message,
                since: cell.status_since,
                blocker_link,
                uncertainty_note: Some("Exact sensor telemetry (0ms lag)".to_string()),
            });
        }
        reports
    }

    /// Generates pure 3D scene nodes for every installed cell and square on `plant_id`.
    pub fn visualize_plant(&self, plant_id: &str) -> Vec<SceneNode> {
        let mut nodes = Vec::new();
        let statuses: BTreeMap<String, CellStatusReport> = self
            .cell_status_reports()
            .into_iter()
            .map(|r| (r.cell_id.clone(), r))
            .collect();

        for cell in self.cells.values() {
            if cell.plant_id != plant_id {
                continue;
            }
            let Some(cat) = Self::lookup_catalog(&cell.catalog_id) else {
                continue;
            };
            let st = statuses.get(&cell.cell_id);
            let color = match st.map(|s| s.state) {
                Some(CellRunState::Running) => "#22c55e",
                Some(CellRunState::Installing) => "#38bdf8",
                Some(CellRunState::Blocked) => "#f59e0b",
                Some(CellRunState::Down) => "#ef4444",
                _ => "#64748b",
            };
            nodes.push(SceneNode {
                node_id: format!("cell-{}", cell.cell_id),
                asset_id: Some(format!("glb:{}", cell.catalog_id)),
                primitive: "box".to_string(),
                x_mm: cell.x_mm,
                y_mm: cell.y_mm,
                z_mm: 0,
                width_mm: cat.width_mm,
                depth_mm: cat.depth_mm,
                height_mm: cat.height_mm,
                rotation_deg: cell.rotation_deg,
                color_hex: color.to_string(),
                label: format!("{} ({})", cell.display_name, cell.cell_id),
                ghost: self.now < cell.commissioned_at,
            });
        }

        for sq in self.buffers.squares.values() {
            if self
                .cells
                .get(&sq.cell_id)
                .map(|c| c.plant_id == plant_id)
                .unwrap_or(false)
            {
                let has_item = sq.occupant.is_some();
                nodes.push(SceneNode {
                    node_id: format!("sq-{}-{}", sq.cell_id, sq.square_name),
                    asset_id: None,
                    primitive: "box".to_string(),
                    x_mm: sq.position_mm.x - 200,
                    y_mm: sq.position_mm.y - 200,
                    z_mm: 0,
                    width_mm: 400,
                    depth_mm: 400,
                    height_mm: if has_item { 350 } else { 40 },
                    rotation_deg: 0,
                    color_hex: if has_item {
                        "#eab308".to_string()
                    } else if sq.is_input {
                        "#3b82f6".to_string()
                    } else {
                        "#a855f7".to_string()
                    },
                    label: format!("{}:{}", sq.cell_id, sq.square_name),
                    ghost: false,
                });
            }
        }
        nodes
    }

    pub fn runway_projection(&self) -> RunwayProjection {
        let mut monthly_rent = 0i64;
        for site in self.plants.values() {
            if let Some(l) = Self::lookup_listing(&site.listing_id) {
                monthly_rent += l.monthly_rent.0;
            }
        }
        let monthly_payroll: i64 = self
            .workers
            .values()
            .map(|w| w.wage_per_hour.0 * 160)
            .sum();
        let monthly_loans: i64 = self.loans.values().map(|l| l.monthly_payment.0).sum();
        let monthly_cells: i64 = self
            .cells
            .values()
            .filter_map(|c| Self::lookup_catalog(&c.catalog_id))
            .map(|cat| cat.monthly_cost.0)
            .sum();

        let gross_burn = Money(monthly_rent + monthly_payroll + monthly_loans + monthly_cells);
        let active_enabled_markets = self
            .market_policies
            .values()
            .filter(|p| p.enabled)
            .count() as i64;
        let est_monthly_rev = Money(active_enabled_markets * 18_000 * Money::UNITS_PER_DOLLAR);

        RunwayProjection::compute(
            self.ledger.cash_balance(),
            self.credit_limit,
            gross_burn,
            est_monthly_rev,
        )
    }

    pub fn bottleneck_report(&self) -> BottleneckReport {
        detect_bottleneck(&self.internal_orders)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_end_to_end_tutorial_loop_orders_chain_backward_and_ship_for_cash() {
        let mut world = SimWorld::new("game-test-1", "Conventional Starter Shop", 12345);
        assert_eq!(world.tutorial_step, 1);

        // Step 1: Lease a first plant
        world
            .apply_command(PlantCommand::LeasePlant {
                plant_id: "plant-1".to_string(),
                listing_id: "bldg-suburban-bay".to_string(),
                name: "Fremont Plant #1".to_string(),
            })
            .unwrap();
        assert_eq!(world.tutorial_step, 2);

        // Step 2: Place one CNC cell and one manual pack station
        world
            .apply_command(PlantCommand::PlaceCell {
                plant_id: "plant-1".to_string(),
                cell_id: "cnc-1".to_string(),
                team: TeamTag::Machining,
                catalog_id: "cnc-3axis-vmc".to_string(),
                addons: vec!["probing".to_string()],
                x_mm: 5_000,
                y_mm: 4_000,
                rotation_deg: 0,
            })
            .unwrap();
        world
            .apply_command(PlantCommand::PlaceCell {
                plant_id: "plant-1".to_string(),
                cell_id: "pack-1".to_string(),
                team: TeamTag::FinishingPacking,
                catalog_id: "pack-manual-station".to_string(),
                addons: vec![],
                x_mm: 10_000,
                y_mm: 4_000,
                rotation_deg: 0,
            })
            .unwrap();
        assert_eq!(world.tutorial_step, 3);

        // Step 3: Hire one employee
        world
            .apply_command(PlantCommand::HireWorker {
                plant_id: "plant-1".to_string(),
                worker_id: "worker-1".to_string(),
                role: "CNC Operator & Logistics".to_string(),
                wage_per_hour_tenth_cents: Money::from_dollars(32).0,
            })
            .unwrap();
        assert_eq!(world.tutorial_step, 4);

        // Step 4: Turn on the online prototyping marketplace with middle-of-the-road price policy
        world
            .apply_command(PlantCommand::ConfigureMarketPolicy {
                plant_id: "plant-1".to_string(),
                market_id: "mkt-prototyping-online".to_string(),
                enabled: true,
                min_payout_per_hour_tenth_cents: Money::from_dollars(80).0,
                price_multiplier_pct: 100,
                promised_lead_days: 7,
            })
            .unwrap();
        assert_eq!(world.tutorial_step, 5);

        // Step 5: Advance 18 days so keys, machine commissioning, worker onboarding, and marketplace orders play out!
        world.advance_to(SimTime::from_days(18));

        let shipped_count = world
            .customer_orders
            .values()
            .filter(|o| o.status == "shipped_on_time" || o.status == "shipped_late")
            .count();
        assert!(
            shipped_count >= 1,
            "Expected at least 1 customer order to chain backward, machine, pack, and ship"
        );
        assert_eq!(world.tutorial_step, 0);
        assert!(world.ledger.is_balanced());
        assert!(world.gateway.verify_integrity());
        assert!(!world.gateway.log_entries.is_empty());
    }

    #[test]
    fn test_skip_ahead_determinism_matches_fine_stepping() {
        let mut world = SimWorld::new("game-det", "Determinism Check", 777);
        world
            .apply_command(PlantCommand::LeasePlant {
                plant_id: "plant-1".to_string(),
                listing_id: "bldg-suburban-bay".to_string(),
                name: "Plant 1".to_string(),
            })
            .unwrap();
        world
            .apply_command(PlantCommand::PlaceCell {
                plant_id: "plant-1".to_string(),
                cell_id: "cnc-1".to_string(),
                team: TeamTag::Machining,
                catalog_id: "cnc-3axis-vmc".to_string(),
                addons: vec![],
                x_mm: 5_000,
                y_mm: 4_000,
                rotation_deg: 0,
            })
            .unwrap();
        world
            .apply_command(PlantCommand::PlaceCell {
                plant_id: "plant-1".to_string(),
                cell_id: "pack-1".to_string(),
                team: TeamTag::FinishingPacking,
                catalog_id: "pack-manual-station".to_string(),
                addons: vec![],
                x_mm: 10_000,
                y_mm: 4_000,
                rotation_deg: 0,
            })
            .unwrap();
        world
            .apply_command(PlantCommand::HireWorker {
                plant_id: "plant-1".to_string(),
                worker_id: "worker-1".to_string(),
                role: "Operator".to_string(),
                wage_per_hour_tenth_cents: Money::from_dollars(32).0,
            })
            .unwrap();
        world
            .apply_command(PlantCommand::ConfigureMarketPolicy {
                plant_id: "plant-1".to_string(),
                market_id: "mkt-prototyping-online".to_string(),
                enabled: true,
                min_payout_per_hour_tenth_cents: Money::from_dollars(80).0,
                price_multiplier_pct: 100,
                promised_lead_days: 7,
            })
            .unwrap();
        assert!(reference::reference_verify_skip_equivalence(
            &world,
            SimTime::from_days(16),
            SimTime::HOUR_MS * 6
        ));
    }
}

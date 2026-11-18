//! `fsim-core`: Deterministic simulation clock (`SimTime`), integer tenth-cent money (`Money`),
//! seeded per-module random streams (`StreamRng`), double-entry accounting (`DoubleEntryLedger`),
//! physical handling units and size-1 named handoff squares (`BufferView`), and the composed
//! multi-pass `step` engine.

pub mod reference;

use fsim_geom::{AabbMm, PointMm};
use fsim_protocols::{
    BlockerReason, CellRunState, Envelope, HandlingUnitSpec, SharedFact, TeamTag,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Deterministic simulation timestamp in integer milliseconds since sim start (`0`).
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct SimTime(pub i64);

impl SimTime {
    pub const ZERO: Self = Self(0);
    pub const SECOND_MS: i64 = 1_000;
    pub const MINUTE_MS: i64 = 60_000;
    pub const HOUR_MS: i64 = 3_600_000;
    pub const DAY_MS: i64 = 86_400_000;

    pub const fn from_ms(ms: i64) -> Self {
        Self(ms)
    }

    pub const fn from_secs(secs: i64) -> Self {
        Self(secs * Self::SECOND_MS)
    }

    pub const fn from_minutes(mins: i64) -> Self {
        Self(mins * Self::MINUTE_MS)
    }

    pub const fn from_hours(hours: i64) -> Self {
        Self(hours * Self::HOUR_MS)
    }

    pub const fn from_days(days: i64) -> Self {
        Self(days * Self::DAY_MS)
    }

    pub const fn add_ms(self, delta_ms: i64) -> Self {
        Self(self.0.saturating_add(delta_ms))
    }
}

/// Deterministic monetary value in integer tenth-cents (`$1.00 = 1_000` tenth-cents).
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Money(pub i64);

impl Money {
    pub const ZERO: Self = Self(0);
    pub const UNITS_PER_DOLLAR: i64 = 1_000;

    pub const fn from_dollars(dollars: i64) -> Self {
        Self(dollars * Self::UNITS_PER_DOLLAR)
    }

    pub const fn from_cents(cents: i64) -> Self {
        Self(cents * 10)
    }

    pub const fn from_tenth_cents(tc: i64) -> Self {
        Self(tc)
    }

    pub fn to_dollars_f64(self) -> f64 {
        (self.0 as f64) / (Self::UNITS_PER_DOLLAR as f64)
    }
}

/// Deterministic random stream derived per `(game_seed, module_id, entity_id, event_count)`.
/// Never reads OS entropy; upgrading one module never reshuffles another module's random draws.
#[derive(Debug, Clone)]
pub struct StreamRng {
    rng: ChaCha8Rng,
}

impl StreamRng {
    pub fn new(game_seed: u64, module_id: &str, entity_id: &str, event_count: u64) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(game_seed.to_le_bytes());
        hasher.update(module_id.as_bytes());
        hasher.update([0xFF]);
        hasher.update(entity_id.as_bytes());
        hasher.update(event_count.to_le_bytes());
        let seed_bytes: [u8; 32] = hasher.finalize().into();
        Self {
            rng: ChaCha8Rng::from_seed(seed_bytes),
        }
    }

    pub fn gen_range_i64(&mut self, low_inclusive: i64, high_inclusive: i64) -> i64 {
        if low_inclusive >= high_inclusive {
            return low_inclusive;
        }
        self.rng.gen_range(low_inclusive..=high_inclusive)
    }

    pub fn gen_f64(&mut self) -> f64 {
        self.rng.gen::<f64>()
    }

    pub fn gen_bool(&mut self, probability: f64) -> bool {
        self.rng.gen_bool(probability.clamp(0.0, 1.0))
    }
}

/// Tracked physical handling unit (container + contents + material lot/heat traceability).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandlingUnit {
    pub unit_id: String,
    pub spec: HandlingUnitSpec,
    pub heat_or_lot_cert: String,
    pub true_in_spec: bool,
    pub believed_in_spec: bool,
    pub created_at: SimTime,
}

/// A size-1 named handoff square inside a cell's footprint (`part_in`, `consumable_in`, `out`, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffSquare {
    pub cell_id: String,
    pub square_name: String,
    pub is_input: bool,
    pub position_mm: PointMm,
    pub accepted_skus: Vec<String>,
    pub occupant: Option<HandlingUnit>,
}

pub mod tuple_key_map {
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

/// Live view of all cell handoff squares and shared storage racks on the plant floor.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BufferView {
    /// Keyed by `(cell_id, square_name)`
    #[serde(with = "tuple_key_map")]
    pub squares: BTreeMap<(String, String), HandoffSquare>,
    /// Multi-slot shared storage racks (keyed by rack `cell_id`)
    pub shared_racks: BTreeMap<String, Vec<HandlingUnit>>,
}

impl BufferView {
    pub fn register_square(&mut self, sq: HandoffSquare) {
        self.squares
            .insert((sq.cell_id.clone(), sq.square_name.clone()), sq);
    }

    pub fn get_square(&self, cell_id: &str, square_name: &str) -> Option<&HandoffSquare> {
        self.squares
            .get(&(cell_id.to_string(), square_name.to_string()))
    }

    pub fn is_square_empty(&self, cell_id: &str, square_name: &str) -> bool {
        self.get_square(cell_id, square_name)
            .map(|s| s.occupant.is_none())
            .unwrap_or(false)
    }

    pub fn is_square_filled(&self, cell_id: &str, square_name: &str) -> bool {
        self.get_square(cell_id, square_name)
            .map(|s| s.occupant.is_some())
            .unwrap_or(false)
    }

    pub fn take_from_square(&mut self, cell_id: &str, square_name: &str) -> Option<HandlingUnit> {
        self.squares
            .get_mut(&(cell_id.to_string(), square_name.to_string()))
            .and_then(|s| s.occupant.take())
    }

    pub fn put_to_square(
        &mut self,
        cell_id: &str,
        square_name: &str,
        unit: HandlingUnit,
    ) -> Result<(), String> {
        let sq = self
            .squares
            .get_mut(&(cell_id.to_string(), square_name.to_string()))
            .ok_or_else(|| format!("square ({cell_id}, {square_name}) does not exist"))?;
        if sq.occupant.is_some() {
            return Err(format!("square ({cell_id}, {square_name}) is already full"));
        }
        sq.occupant = Some(unit);
        Ok(())
    }

    pub fn put_to_rack(&mut self, rack_id: &str, unit: HandlingUnit) {
        self.shared_racks
            .entry(rack_id.to_string())
            .or_default()
            .push(unit);
    }

    pub fn take_sku_from_rack(&mut self, rack_id: &str, sku: &str) -> Option<HandlingUnit> {
        let rack = self.shared_racks.get_mut(rack_id)?;
        let idx = rack.iter().position(|u| u.spec.item_sku == sku)?;
        Some(rack.remove(idx))
    }

    pub fn find_sku_in_any_rack(&self, sku: &str) -> Option<(String, &HandlingUnit)> {
        for (rack_id, units) in &self.shared_racks {
            if let Some(u) = units.iter().find(|u| u.spec.item_sku == sku) {
                return Some((rack_id.clone(), u));
            }
        }
        None
    }

    pub fn apply_op(&mut self, op: BufferOp) -> Result<(), String> {
        match op {
            BufferOp::PutToSquare {
                cell_id,
                square_name,
                unit,
            } => self.put_to_square(&cell_id, &square_name, unit),
            BufferOp::TakeFromSquare {
                cell_id,
                square_name,
                expected_unit_id,
            } => {
                let taken = self
                    .take_from_square(&cell_id, &square_name)
                    .ok_or_else(|| format!("square ({cell_id}, {square_name}) was empty"))?;
                if taken.unit_id != expected_unit_id {
                    return Err(format!(
                        "expected unit {} in ({cell_id}, {square_name}), found {}",
                        expected_unit_id, taken.unit_id
                    ));
                }
                Ok(())
            }
            BufferOp::PutToRack { rack_id, unit } => {
                self.put_to_rack(&rack_id, unit);
                Ok(())
            }
            BufferOp::TakeFromRack {
                rack_id,
                sku,
                expected_unit_id,
            } => {
                let taken = self
                    .take_sku_from_rack(&rack_id, &sku)
                    .ok_or_else(|| format!("rack {rack_id} had no sku {sku}"))?;
                if taken.unit_id != expected_unit_id {
                    return Err(format!(
                        "expected unit {} in rack {rack_id}, found {}",
                        expected_unit_id, taken.unit_id
                    ));
                }
                Ok(())
            }
        }
    }
}

/// Atomic buffer operation emitted by a module during `step`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BufferOp {
    PutToSquare {
        cell_id: String,
        square_name: String,
        unit: HandlingUnit,
    },
    TakeFromSquare {
        cell_id: String,
        square_name: String,
        expected_unit_id: String,
    },
    PutToRack {
        rack_id: String,
        unit: HandlingUnit,
    },
    TakeFromRack {
        rack_id: String,
        sku: String,
        expected_unit_id: String,
    },
}

/// Double-entry accounting ledger entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerRecord {
    pub entry_id: String,
    pub timestamp: SimTime,
    pub plant_id: String,
    pub debit_account: String,
    pub credit_account: String,
    pub amount: Money,
    pub memo: String,
    pub reference_id: String,
}

/// Double-entry ledger guaranteeing `sum(all signed account balances) == 0`.
/// In conventional sign convention here:
/// - Debits add `+amount` to `debit_account`
/// - Credits add `-amount` to `credit_account`
/// So `cash` (an asset debited when cash comes in, credited when spent) has positive balance when cash > 0.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoubleEntryLedger {
    pub entries: Vec<LedgerRecord>,
    pub account_balances: BTreeMap<String, i64>,
}

impl DoubleEntryLedger {
    pub fn new_with_starting_equity(initial_cash: Money, now: SimTime) -> Self {
        let mut ledger = Self::default();
        if initial_cash.0 > 0 {
            ledger.post(
                "init-equity".to_string(),
                now,
                "company".to_string(),
                "asset:cash",
                "equity:paid_in_capital",
                initial_cash,
                "Initial venture capital financing".to_string(),
                "seed-round".to_string(),
            );
        }
        ledger
    }

    #[allow(clippy::too_many_arguments)]
    pub fn post(
        &mut self,
        entry_id: String,
        timestamp: SimTime,
        plant_id: String,
        debit_account: &str,
        credit_account: &str,
        amount: Money,
        memo: String,
        reference_id: String,
    ) {
        if amount.0 == 0 {
            return;
        }
        *self
            .account_balances
            .entry(debit_account.to_string())
            .or_insert(0) += amount.0;
        *self
            .account_balances
            .entry(credit_account.to_string())
            .or_insert(0) -= amount.0;
        self.entries.push(LedgerRecord {
            entry_id,
            timestamp,
            plant_id,
            debit_account: debit_account.to_string(),
            credit_account: credit_account.to_string(),
            amount,
            memo,
            reference_id,
        });
    }

    pub fn cash_balance(&self) -> Money {
        Money(*self.account_balances.get("asset:cash").unwrap_or(&0))
    }

    pub fn account_balance(&self, account: &str) -> Money {
        Money(*self.account_balances.get(account).unwrap_or(&0))
    }

    pub fn is_balanced(&self) -> bool {
        reference::reference_ledger_is_balanced(self)
    }
}

/// Evaluation order policy for first-come-first-served passes at a single timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderPolicy {
    Forced(Vec<String>),
    Seeded(u64),
}

/// Output of a pure `step()` call at any hierarchy level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepOut<S> {
    pub new_state: S,
    pub buffer_ops: Vec<BufferOp>,
    pub outbox: Vec<Envelope>,
    pub shared_facts: Vec<SharedFact>,
    pub next_event_at: Option<SimTime>,
}

/// Trait implemented by every team module and cell slice so the core can run
/// first-come-first-served passes until quiet and verify conformance invariants.
pub trait StepModule {
    type State: Clone + PartialEq;

    fn module_id(&self) -> &str;
    fn team_tag(&self) -> TeamTag;
    fn step(
        &self,
        state: &Self::State,
        buffers: &BufferView,
        inbox: &[Envelope],
        now: SimTime,
        rng: &mut StreamRng,
    ) -> StepOut<Self::State>;

    /// Returns the number of physical handling units held internally inside `state` (for conservation checks).
    fn internal_unit_count(&self, state: &Self::State) -> usize;
}

/// Catalog metadata for a placeable work cell or support area on a team's build palette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellCatalogEntry {
    pub catalog_id: String,
    pub team: TeamTag,
    pub display_name: String,
    pub description: String,
    pub price: Money,
    pub monthly_cost: Money,
    pub lead_time_days_min: u32,
    pub lead_time_days_max: u32,
    pub width_mm: i32,
    pub depth_mm: i32,
    pub height_mm: i32,
    pub clearance_mm: i32,
    pub power_kw: u32,
    pub needs_three_phase_480v: bool,
    pub needs_compressed_air: bool,
    pub needs_climate_room: bool,
    pub needs_hazmat_permit: bool,
    pub financeable: bool,
    pub input_squares: Vec<(String, Vec<String>)>, // (square_name, accepted_skus)
    pub output_square_skus: Vec<String>,
    pub cycle_time_summary: String,
    pub available_addons: Vec<CellAddonSpec>,
}

/// Optional configuration bundle / add-on for a cell type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellAddonSpec {
    pub addon_id: String,
    pub name: String,
    pub price_delta: Money,
    pub cycle_time_pct_delta: i32,
    pub description: String,
}

/// Generic 3D scene node emitted by a team's pure `visualize()` function.
/// `x_mm`, `y_mm` is the footprint origin (min corner before rotation); `width_mm`/`depth_mm`
/// are the unrotated catalog dimensions. When `model` is `None`, the viewer draws a fallback box.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneNode {
    pub node_id: String,
    pub asset_id: Option<String>,
    pub primitive: String, // "box", "cylinder", "label"
    pub x_mm: i32,
    pub y_mm: i32,
    pub z_mm: i32,
    pub width_mm: i32,
    pub depth_mm: i32,
    pub height_mm: i32,
    pub rotation_deg: i32,
    pub color_hex: String,
    pub label: String,
    pub ghost: bool,
    pub model: Option<CellModel>,
}

/// Axis of a cylinder primitive in a cell's local frame (x = width, y = depth, z = up).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelAxis {
    X,
    Y,
    Z,
}

/// Surface material hint for the viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelMaterial {
    Paint,
    Metal,
    Glass,
    Rubber,
    Emissive,
    Screen,
}

/// Primitive shape of one model part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ModelShape {
    Box { size_mm: [i32; 3] },
    Cylinder { radius_mm: i32, length_mm: i32, axis: ModelAxis },
}

/// One named, colored primitive of a procedural cell model.
/// `node` names are the state-binding contract with the viewer (e.g. `door`, `spindle`, `status_light`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPart {
    pub node: String,
    pub shape: ModelShape,
    pub center_mm: [i32; 3],
    pub material: ModelMaterial,
    pub color_hex: String,
}

impl ModelPart {
    /// Axis-aligned bounds `(min, max)` of this part in the cell's local frame.
    pub fn bounds_mm(&self) -> ([i32; 3], [i32; 3]) {
        let half = match self.shape {
            ModelShape::Box { size_mm } => [size_mm[0] / 2, size_mm[1] / 2, size_mm[2] / 2],
            ModelShape::Cylinder {
                radius_mm,
                length_mm,
                axis,
            } => match axis {
                ModelAxis::X => [length_mm / 2, radius_mm, radius_mm],
                ModelAxis::Y => [radius_mm, length_mm / 2, radius_mm],
                ModelAxis::Z => [radius_mm, radius_mm, length_mm / 2],
            },
        };
        let c = self.center_mm;
        (
            [c[0] - half[0], c[1] - half[1], c[2] - half[2]],
            [c[0] + half[0], c[1] + half[1], c[2] + half[2]],
        )
    }
}

/// Procedural 3D model of a cell in its local frame: origin at the footprint's min corner,
/// x along `width_mm`, y along `depth_mm` (front face at y = 0), z up from the floor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellModel {
    pub width_mm: i32,
    pub depth_mm: i32,
    pub height_mm: i32,
    pub parts: Vec<ModelPart>,
}

impl CellModel {
    /// Union of all part bounds `(min, max)`; `None` if the model has no parts.
    pub fn bounds_mm(&self) -> Option<([i32; 3], [i32; 3])> {
        let mut it = self.parts.iter().map(|p| p.bounds_mm());
        let first = it.next()?;
        Some(it.fold(first, |(lo, hi), (a, b)| {
            (
                [lo[0].min(a[0]), lo[1].min(a[1]), lo[2].min(a[2])],
                [hi[0].max(b[0]), hi[1].max(b[1]), hi[2].max(b[2])],
            )
        }))
    }

    /// True if every part lies inside the declared `width x depth x height` envelope.
    pub fn fits_envelope(&self) -> bool {
        match self.bounds_mm() {
            None => true,
            Some((lo, hi)) => {
                lo[0] >= 0
                    && lo[1] >= 0
                    && lo[2] >= 0
                    && hi[0] <= self.width_mm
                    && hi[1] <= self.depth_mm
                    && hi[2] <= self.height_mm
            }
        }
    }

    pub fn part(&self, node: &str) -> Option<&ModelPart> {
        self.parts.iter().find(|p| p.node == node)
    }
}

/// Status report returned by a team's `status(cell, estimated_state)` function ("Why isn't this running?").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellStatusReport {
    pub cell_id: String,
    pub state: CellRunState,
    pub reason: BlockerReason,
    pub message: String,
    pub since: SimTime,
    pub blocker_link: Option<String>,
    pub uncertainty_note: Option<String>,
}

impl CellCatalogEntry {
    pub fn footprint_at(&self, x_mm: i32, y_mm: i32, rotation_deg: i32) -> AabbMm {
        AabbMm::from_origin_size(x_mm, y_mm, self.width_mm, self.depth_mm, rotation_deg)
    }
}

/// Conformance test helper: verifies the Early-Call Invariant for any `StepModule`.
/// Calling `step` at an intermediate timestamp `mid_time` with no new inputs must produce
/// the exact same state and outputs at `target_time` as stepping directly to `target_time`.
pub fn check_early_call_invariant<M: StepModule>(
    module: &M,
    initial_state: &M::State,
    buffers: &BufferView,
    mid_time: SimTime,
    target_time: SimTime,
    seed: u64,
) -> bool {
    let mut rng_direct = StreamRng::new(seed, module.module_id(), "conformance", 1);
    let out_direct = module.step(initial_state, buffers, &[], target_time, &mut rng_direct);

    let mut rng_mid = StreamRng::new(seed, module.module_id(), "conformance", 1);
    let out_mid = module.step(initial_state, buffers, &[], mid_time, &mut rng_mid);
    let mut buffers_mid = buffers.clone();
    for op in out_mid.buffer_ops {
        let _ = buffers_mid.apply_op(op);
    }
    let out_after_mid = module.step(
        &out_mid.new_state,
        &buffers_mid,
        &[],
        target_time,
        &mut rng_mid,
    );

    out_direct.new_state == out_after_mid.new_state
        && out_direct.next_event_at == out_after_mid.next_event_at
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_double_entry_ledger_always_balances() {
        let mut ledger = DoubleEntryLedger::new_with_starting_equity(
            Money::from_dollars(500_000),
            SimTime::ZERO,
        );
        assert!(ledger.is_balanced());
        assert_eq!(ledger.cash_balance(), Money::from_dollars(500_000));

        ledger.post(
            "rent-1".to_string(),
            SimTime::from_days(1),
            "plant-1".to_string(),
            "expense:rent",
            "asset:cash",
            Money::from_dollars(8_500),
            "Monthly warehouse lease".to_string(),
            "lease-1".to_string(),
        );
        assert!(ledger.is_balanced());
        assert_eq!(ledger.cash_balance(), Money::from_dollars(491_500));
    }

    #[test]
    fn test_seeded_and_forced_module_ordering_determinism() {
        let modules = vec![
            "machining".to_string(),
            "packing".to_string(),
            "logistics".to_string(),
            "people".to_string(),
        ];
        let priorities = vec!["people".to_string()];
        let o1 = reference::reference_module_order(
            &OrderPolicy::Seeded(42),
            SimTime::from_secs(10),
            &priorities,
            &modules,
        );
        let o2 = reference::reference_module_order(
            &OrderPolicy::Seeded(42),
            SimTime::from_secs(10),
            &priorities,
            &modules,
        );
        assert_eq!(o1, o2);
        assert_eq!(o1[0], "people"); // Priority module always goes first
    }
}

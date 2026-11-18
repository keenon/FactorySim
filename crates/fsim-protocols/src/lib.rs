//! `fsim-protocols`: Canonical wire protocol envelopes, shared cross-team schemas,
//! backward-chaining internal orders, and plant/controller boundary contracts.

pub mod reference;
pub mod wire;

use serde::{Deserialize, Serialize};
use wire::*;

/// Canonical team identifier corresponding to `fsim.protocols.v1.TeamTag`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum TeamTag {
    Unspecified = 0,
    Platform = 1,
    Machining = 2,
    Toolroom = 3,
    Molding = 4,
    HeatTreat = 5,
    FinishingPacking = 6,
    Quality = 7,
    MaterialsLogistics = 8,
    FacilitiesEhs = 9,
    PlanningFlow = 10,
    PeopleOps = 11,
    Sales = 12,
    Finance = 13,
}

impl TeamTag {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Platform,
            2 => Self::Machining,
            3 => Self::Toolroom,
            4 => Self::Molding,
            5 => Self::HeatTreat,
            6 => Self::FinishingPacking,
            7 => Self::Quality,
            8 => Self::MaterialsLogistics,
            9 => Self::FacilitiesEhs,
            10 => Self::PlanningFlow,
            11 => Self::PeopleOps,
            12 => Self::Sales,
            13 => Self::Finance,
            _ => Self::Unspecified,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unspecified => "unspecified",
            Self::Platform => "platform",
            Self::Machining => "machining",
            Self::Toolroom => "toolroom",
            Self::Molding => "molding",
            Self::HeatTreat => "heat-treat",
            Self::FinishingPacking => "finishing-packing",
            Self::Quality => "quality",
            Self::MaterialsLogistics => "materials-logistics",
            Self::FacilitiesEhs => "facilities-ehs",
            Self::PlanningFlow => "planning-flow",
            Self::PeopleOps => "people-ops",
            Self::Sales => "sales",
            Self::Finance => "finance",
        }
    }
}

/// Standard protocol envelope (`fsim.protocols.v1.Envelope`).
/// Core routing, logging, and replay inspect only the envelope, never the payload bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub team_tag: TeamTag,
    pub schema_id: String,
    pub schema_version: u32,
    pub source_device: String,
    pub entity_id: String,
    pub plant_timestamp_ms: i64,
    pub receive_timestamp_ms: i64,
    pub sequence_number: u64,
    pub correlation_id: String,
    pub payload: Vec<u8>,
}

impl ProtoMessage for Envelope {
    fn encode_proto(&self, buf: &mut Vec<u8>) {
        encode_uint32_field(1, self.team_tag as u32, buf);
        encode_string_field(2, &self.schema_id, buf);
        encode_uint32_field(3, self.schema_version, buf);
        encode_string_field(4, &self.source_device, buf);
        encode_string_field(5, &self.entity_id, buf);
        encode_int64_field(6, self.plant_timestamp_ms, buf);
        encode_int64_field(7, self.receive_timestamp_ms, buf);
        encode_uint64_field(8, self.sequence_number, buf);
        encode_string_field(9, &self.correlation_id, buf);
        encode_bytes_field(10, &self.payload, buf);
    }

    fn decode_proto(buf: &[u8]) -> Result<Self, WireError> {
        let mut pos = 0;
        let mut env = Self {
            team_tag: TeamTag::Unspecified,
            schema_id: String::new(),
            schema_version: 0,
            source_device: String::new(),
            entity_id: String::new(),
            plant_timestamp_ms: 0,
            receive_timestamp_ms: 0,
            sequence_number: 0,
            correlation_id: String::new(),
            payload: Vec::new(),
        };
        while pos < buf.len() {
            let (field, wt) = decode_tag(buf, &mut pos)?;
            match (field, wt) {
                (1, WireType::Varint) => env.team_tag = TeamTag::from_u32(decode_varint(buf, &mut pos)? as u32),
                (2, WireType::LengthDelimited) => env.schema_id = decode_string(buf, &mut pos)?,
                (3, WireType::Varint) => env.schema_version = decode_varint(buf, &mut pos)? as u32,
                (4, WireType::LengthDelimited) => env.source_device = decode_string(buf, &mut pos)?,
                (5, WireType::LengthDelimited) => env.entity_id = decode_string(buf, &mut pos)?,
                (6, WireType::Varint) => env.plant_timestamp_ms = decode_varint(buf, &mut pos)? as i64,
                (7, WireType::Varint) => env.receive_timestamp_ms = decode_varint(buf, &mut pos)? as i64,
                (8, WireType::Varint) => env.sequence_number = decode_varint(buf, &mut pos)?,
                (9, WireType::LengthDelimited) => env.correlation_id = decode_string(buf, &mut pos)?,
                (10, WireType::LengthDelimited) => env.payload = decode_bytes(buf, &mut pos)?.to_vec(),
                (_, other_wt) => skip_field(other_wt, buf, &mut pos)?,
            }
        }
        Ok(env)
    }
}

/// Specification of a handling unit (container + SKU + quantity + traceability lot).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HandlingUnitSpec {
    pub container_type: String,
    pub item_sku: String,
    pub quantity: u32,
    pub lot_id: String,
}

impl ProtoMessage for HandlingUnitSpec {
    fn encode_proto(&self, buf: &mut Vec<u8>) {
        encode_string_field(1, &self.container_type, buf);
        encode_string_field(2, &self.item_sku, buf);
        encode_uint32_field(3, self.quantity, buf);
        encode_string_field(4, &self.lot_id, buf);
    }

    fn decode_proto(buf: &[u8]) -> Result<Self, WireError> {
        let mut pos = 0;
        let mut spec = Self {
            container_type: String::new(),
            item_sku: String::new(),
            quantity: 0,
            lot_id: String::new(),
        };
        while pos < buf.len() {
            let (field, wt) = decode_tag(buf, &mut pos)?;
            match (field, wt) {
                (1, WireType::LengthDelimited) => spec.container_type = decode_string(buf, &mut pos)?,
                (2, WireType::LengthDelimited) => spec.item_sku = decode_string(buf, &mut pos)?,
                (3, WireType::Varint) => spec.quantity = decode_varint(buf, &mut pos)? as u32,
                (4, WireType::LengthDelimited) => spec.lot_id = decode_string(buf, &mut pos)?,
                (_, other_wt) => skip_field(other_wt, buf, &mut pos)?,
            }
        }
        Ok(spec)
    }
}

/// Lifecycle state of a backward-chaining internal pull order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum InternalOrderStatus {
    Unspecified = 0,
    BlockedUpstream = 1,
    ReadyForLogistics = 2,
    InputsStaged = 3,
    InProcess = 4,
    Completed = 5,
    Aborted = 6,
}

impl InternalOrderStatus {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::BlockedUpstream,
            2 => Self::ReadyForLogistics,
            3 => Self::InputsStaged,
            4 => Self::InProcess,
            5 => Self::Completed,
            6 => Self::Aborted,
            _ => Self::Unspecified,
        }
    }
}

/// Named input square requirement for an internal order (e.g., `part_in`, `consumable_in`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequiredInputEntry {
    pub square_name: String,
    pub spec: HandlingUnitSpec,
}

impl ProtoMessage for RequiredInputEntry {
    fn encode_proto(&self, buf: &mut Vec<u8>) {
        encode_string_field(1, &self.square_name, buf);
        let spec_bytes = self.spec.to_proto_bytes();
        encode_bytes_field(2, &spec_bytes, buf);
    }

    fn decode_proto(buf: &[u8]) -> Result<Self, WireError> {
        let mut pos = 0;
        let mut entry = Self {
            square_name: String::new(),
            spec: HandlingUnitSpec {
                container_type: String::new(),
                item_sku: String::new(),
                quantity: 0,
                lot_id: String::new(),
            },
        };
        while pos < buf.len() {
            let (field, wt) = decode_tag(buf, &mut pos)?;
            match (field, wt) {
                (1, WireType::LengthDelimited) => entry.square_name = decode_string(buf, &mut pos)?,
                (2, WireType::LengthDelimited) => {
                    let b = decode_bytes(buf, &mut pos)?;
                    entry.spec = HandlingUnitSpec::decode_proto(b)?;
                }
                (_, other_wt) => skip_field(other_wt, buf, &mut pos)?,
            }
        }
        Ok(entry)
    }
}

/// Backward-chaining internal pull order (`fsim.protocols.v1.InternalOrder`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalOrder {
    pub order_id: String,
    pub parent_order_id: String,
    pub customer_order_id: String,
    pub target_team: TeamTag,
    pub target_cell_id: String,
    pub output_spec: HandlingUnitSpec,
    pub status: InternalOrderStatus,
    pub upstream_order_ids: Vec<String>,
    pub required_inputs: Vec<RequiredInputEntry>,
    pub created_at_ms: i64,
    pub due_at_ms: i64,
}

impl ProtoMessage for InternalOrder {
    fn encode_proto(&self, buf: &mut Vec<u8>) {
        encode_string_field(1, &self.order_id, buf);
        encode_string_field(2, &self.parent_order_id, buf);
        encode_string_field(3, &self.customer_order_id, buf);
        encode_uint32_field(4, self.target_team as u32, buf);
        encode_string_field(5, &self.target_cell_id, buf);
        let out_bytes = self.output_spec.to_proto_bytes();
        encode_bytes_field(6, &out_bytes, buf);
        encode_uint32_field(7, self.status as u32, buf);
        for up in &self.upstream_order_ids {
            encode_string_field(8, up, buf);
        }
        for req in &self.required_inputs {
            let req_bytes = req.to_proto_bytes();
            encode_bytes_field(9, &req_bytes, buf);
        }
        encode_int64_field(10, self.created_at_ms, buf);
        encode_int64_field(11, self.due_at_ms, buf);
    }

    fn decode_proto(buf: &[u8]) -> Result<Self, WireError> {
        let mut pos = 0;
        let mut order = Self {
            order_id: String::new(),
            parent_order_id: String::new(),
            customer_order_id: String::new(),
            target_team: TeamTag::Unspecified,
            target_cell_id: String::new(),
            output_spec: HandlingUnitSpec {
                container_type: String::new(),
                item_sku: String::new(),
                quantity: 0,
                lot_id: String::new(),
            },
            status: InternalOrderStatus::Unspecified,
            upstream_order_ids: Vec::new(),
            required_inputs: Vec::new(),
            created_at_ms: 0,
            due_at_ms: 0,
        };
        while pos < buf.len() {
            let (field, wt) = decode_tag(buf, &mut pos)?;
            match (field, wt) {
                (1, WireType::LengthDelimited) => order.order_id = decode_string(buf, &mut pos)?,
                (2, WireType::LengthDelimited) => order.parent_order_id = decode_string(buf, &mut pos)?,
                (3, WireType::LengthDelimited) => order.customer_order_id = decode_string(buf, &mut pos)?,
                (4, WireType::Varint) => order.target_team = TeamTag::from_u32(decode_varint(buf, &mut pos)? as u32),
                (5, WireType::LengthDelimited) => order.target_cell_id = decode_string(buf, &mut pos)?,
                (6, WireType::LengthDelimited) => {
                    let b = decode_bytes(buf, &mut pos)?;
                    order.output_spec = HandlingUnitSpec::decode_proto(b)?;
                }
                (7, WireType::Varint) => order.status = InternalOrderStatus::from_u32(decode_varint(buf, &mut pos)? as u32),
                (8, WireType::LengthDelimited) => order.upstream_order_ids.push(decode_string(buf, &mut pos)?),
                (9, WireType::LengthDelimited) => {
                    let b = decode_bytes(buf, &mut pos)?;
                    order.required_inputs.push(RequiredInputEntry::decode_proto(b)?);
                }
                (10, WireType::Varint) => order.created_at_ms = decode_varint(buf, &mut pos)? as i64,
                (11, WireType::Varint) => order.due_at_ms = decode_varint(buf, &mut pos)? as i64,
                (_, other_wt) => skip_field(other_wt, buf, &mut pos)?,
            }
        }
        Ok(order)
    }
}

/// Shared vocabulary of cross-team facts and commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SharedFact {
    HandlingUnitMoved {
        unit_id: String,
        spec: HandlingUnitSpec,
        from_cell: String,
        from_square: String,
        to_cell: String,
        to_square: String,
    },
    CellStateChanged {
        cell_id: String,
        status: CellRunState,
        reason: BlockerReason,
        detail: String,
        blocker_link: Option<String>,
    },
    LedgerPosted {
        entry_id: String,
        debit_account: String,
        credit_account: String,
        amount_tenth_cents: i64,
        memo: String,
        reference_id: String,
    },
    InternalOrderUpdated(InternalOrder),
    OperatorTaskDispatched {
        task_id: String,
        worker_id: String,
        ipad_device_id: String,
        kind: OperatorTaskKind,
        order_id: String,
    },
    OperatorTaskCompleted {
        task_id: String,
        worker_id: String,
        order_id: String,
    },
    PurchaseOrderPlaced {
        po_id: String,
        vendor_id: String,
        spec: HandlingUnitSpec,
        cost_tenth_cents: i64,
        expected_delivery_ms: i64,
    },
    PurchaseOrderDelivered {
        po_id: String,
        receiving_cell_id: String,
        spec: HandlingUnitSpec,
    },
    CustomerShipmentCompleted {
        shipment_id: String,
        customer_order_id: String,
        payout_tenth_cents: i64,
        on_time: bool,
        in_spec: bool,
    },
    IncidentReported {
        incident_id: String,
        severity: u8,
        plant_id: String,
        cell_id: Option<String>,
        description: String,
        cost_tenth_cents: i64,
    },
    CommandAck {
        command_id: String,
        accepted: bool,
        reason: String,
    },
}

/// Standardized cell run state for the "Why isn't this running?" contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CellRunState {
    Running,
    Setup,
    Idle,
    Blocked,
    Down,
    Planned,
    Installing,
}

/// Shared vocabulary of reasons why a cell is not running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BlockerReason {
    None,
    NoActiveOrders,
    WaitingForMaterial,
    WaitingForOperator,
    OutputSquareFull,
    SetupInProgress,
    DownForRepair,
    ChipHopperFull,
    NoPowerOrUtilities,
    WaitingOnPermit,
    WaitingOnDelivery,
    ClimateOutOfSpec,
}

/// Task types dispatched to worker iPads by `fsim-people`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatorTaskKind {
    MoveHandlingUnit {
        from_cell: String,
        from_square: String,
        to_cell: String,
        to_square: String,
        spec: HandlingUnitSpec,
    },
    OperateCell {
        cell_id: String,
        order_id: String,
    },
    EmptyChipHopper {
        cell_id: String,
    },
    RepairCell {
        cell_id: String,
    },
}

/// Controller-to-plant commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlantCommand {
    LeasePlant {
        plant_id: String,
        listing_id: String,
        name: String,
    },
    PlaceCell {
        plant_id: String,
        cell_id: String,
        team: TeamTag,
        catalog_id: String,
        addons: Vec<String>,
        x_mm: i32,
        y_mm: i32,
        rotation_deg: i32,
    },
    MoveCell {
        plant_id: String,
        cell_id: String,
        x_mm: i32,
        y_mm: i32,
        rotation_deg: i32,
    },
    RemoveCell {
        plant_id: String,
        cell_id: String,
    },
    HireWorker {
        plant_id: String,
        worker_id: String,
        role: String,
        wage_per_hour_tenth_cents: i64,
    },
    SetShiftPriority {
        plant_id: String,
        priorities: Vec<String>,
    },
    ConfigureMarketPolicy {
        plant_id: String,
        market_id: String,
        enabled: bool,
        min_payout_per_hour_tenth_cents: i64,
        price_multiplier_pct: u32,
        promised_lead_days: u32,
    },
    UnlockMarketCert {
        plant_id: String,
        cert_id: String,
    },
    OrderRawStock {
        plant_id: String,
        sku: String,
        quantity: u32,
    },
    TakeEquipmentLoan {
        loan_id: String,
        principal_tenth_cents: i64,
        collateral_cell_id: String,
    },
}

/// Versioned schema registry entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaRegistration {
    pub team: TeamTag,
    pub schema_id: &'static str,
    pub version: u32,
    pub description: &'static str,
}

/// Returns the central registry of shared and team schema versions.
pub fn standard_schema_registry() -> Vec<SchemaRegistration> {
    vec![
        SchemaRegistration {
            team: TeamTag::Platform,
            schema_id: "fsim.protocols.v1.Envelope",
            version: 1,
            description: "Standard wire envelope for all observations and commands",
        },
        SchemaRegistration {
            team: TeamTag::Platform,
            schema_id: "fsim.protocols.v1.SharedFact",
            version: 1,
            description: "Cross-team shared facts (squares, ledger, orders, shipments)",
        },
        SchemaRegistration {
            team: TeamTag::PlanningFlow,
            schema_id: "fsim.protocols.v1.InternalOrder",
            version: 1,
            description: "Backward-chaining pull order across work cells and storage",
        },
        SchemaRegistration {
            team: TeamTag::PeopleOps,
            schema_id: "fsim.protocols.v1.OperatorTask",
            version: 1,
            description: "iPad task dispatch and completion messages",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_proto_roundtrip_and_forward_compat() {
        let env = Envelope {
            team_tag: TeamTag::Machining,
            schema_id: "fsim.machining.v1.SpindleCycle".to_string(),
            schema_version: 2,
            source_device: "plant/1/cell/3/mill-controller".to_string(),
            entity_id: "cell-3".to_string(),
            plant_timestamp_ms: 123_456_789,
            receive_timestamp_ms: 123_456_800,
            sequence_number: 42,
            correlation_id: "ord-99".to_string(),
            payload: vec![1, 2, 3, 4, 5],
        };
        let decoded = reference::reference_envelope_roundtrip(&env).unwrap();
        assert_eq!(env, decoded);

        // Append an unknown future field (#99, varint 12345) and verify older decoder ignores it cleanly.
        let mut bytes = env.to_proto_bytes();
        encode_uint64_field(99, 12345, &mut bytes);
        let decoded_with_unknown = Envelope::decode_proto(&bytes).unwrap();
        assert_eq!(env, decoded_with_unknown);
    }

    #[test]
    fn test_internal_order_proto_roundtrip() {
        let order = InternalOrder {
            order_id: "ord-pack-1".to_string(),
            parent_order_id: "ord-ship-1".to_string(),
            customer_order_id: "cust-101".to_string(),
            target_team: TeamTag::FinishingPacking,
            target_cell_id: "pack-1".to_string(),
            output_spec: HandlingUnitSpec {
                container_type: "sealed-box".to_string(),
                item_sku: "bracket-alu-6061".to_string(),
                quantity: 1,
                lot_id: "lot-2026-01".to_string(),
            },
            status: InternalOrderStatus::BlockedUpstream,
            upstream_order_ids: vec!["ord-cnc-1".to_string(), "ord-box-1".to_string()],
            required_inputs: vec![
                RequiredInputEntry {
                    square_name: "part_in".to_string(),
                    spec: HandlingUnitSpec {
                        container_type: "tray".to_string(),
                        item_sku: "bracket-alu-6061".to_string(),
                        quantity: 1,
                        lot_id: "lot-2026-01".to_string(),
                    },
                },
                RequiredInputEntry {
                    square_name: "consumable_in".to_string(),
                    spec: HandlingUnitSpec {
                        container_type: "bundle".to_string(),
                        item_sku: "carton-small".to_string(),
                        quantity: 1,
                        lot_id: "lot-box-1".to_string(),
                    },
                },
            ],
            created_at_ms: 1_000,
            due_at_ms: 86_400_000,
        };
        let decoded = reference::reference_internal_order_roundtrip(&order).unwrap();
        assert_eq!(order, decoded);
    }

    #[test]
    fn test_malformed_bytes_never_panic() {
        for bad in [
            &[][..],
            &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x02][..],
            &[0x12, 0xFF, 0x01, 0x02][..],
            &[0x07][..],
        ] {
            let _ = Envelope::decode_proto(bad);
            let _ = InternalOrder::decode_proto(bad);
        }
    }
}

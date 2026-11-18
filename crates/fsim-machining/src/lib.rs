//! `fsim-machining`: CNC mills, lathes, and finishing mills; conditional models for cycle time,
//! per-tool wear, in-process probing, chip hopper accumulation, status reporting, and 3D scene nodes.

pub mod model;
pub mod reference;

pub use model::{machining_cell_model, MachineKind};

use fsim_core::{CellAddonSpec, CellCatalogEntry, Money};
use fsim_protocols::TeamTag;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachiningCellConfig {
    pub has_bar_feeder: bool,
    pub has_in_process_probing: bool,
    pub has_pallet_changer: bool,
    pub has_chip_conveyor: bool,
}

impl MachiningCellConfig {
    pub fn from_addons(addons: &[String]) -> Self {
        Self {
            has_bar_feeder: addons.iter().any(|a| a == "bar-feeder"),
            has_in_process_probing: addons.iter().any(|a| a == "probing"),
            has_pallet_changer: addons.iter().any(|a| a == "pallet-changer"),
            has_chip_conveyor: addons.iter().any(|a| a == "chip-conveyor"),
        }
    }
}

/// Starter catalog of 4 CNC machining cells with add-on bundles.
pub fn starter_machining_catalog() -> Vec<CellCatalogEntry> {
    let common_addons = vec![
        CellAddonSpec {
            addon_id: "probing".to_string(),
            name: "Renishaw Spindle & Tool Probing".to_string(),
            price_delta: Money::from_dollars(8_500),
            cycle_time_pct_delta: 8,
            description: "Cuts out-of-tolerance scrap by 67% at a small probing time cost.".to_string(),
        },
        CellAddonSpec {
            addon_id: "chip-conveyor".to_string(),
            name: "Auger Chip Conveyor".to_string(),
            price_delta: Money::from_dollars(5_500),
            cycle_time_pct_delta: -5,
            description: "Doubles chip hopper capacity before manual dump is needed.".to_string(),
        },
    ];

    vec![
        CellCatalogEntry {
            catalog_id: "cnc-3axis-vmc".to_string(),
            team: TeamTag::Machining,
            display_name: "3-Axis Vertical Machining Center (VMC)".to_string(),
            description: "Workhorse 3-axis CNC mill for aluminum and steel prismatic brackets. Financeable.".to_string(),
            price: Money::from_dollars(65_000),
            monthly_cost: Money::from_dollars(450),
            lead_time_days_min: 7,
            lead_time_days_max: 14,
            width_mm: 2_400,
            depth_mm: 2_000,
            height_mm: 2_400,
            clearance_mm: 400,
            power_kw: 22,
            needs_three_phase_480v: true,
            needs_compressed_air: true,
            needs_climate_room: false,
            needs_hazmat_permit: false,
            financeable: true,
            input_squares: vec![("part_in".to_string(), vec!["blank-alu".to_string(), "blank-steel".to_string()])],
            output_square_skus: vec!["machined-part".to_string()],
            cycle_time_summary: "18–35 min/part (material & feature dependent)".to_string(),
            available_addons: common_addons.clone(),
        },
        CellCatalogEntry {
            catalog_id: "cnc-5axis-trunnion".to_string(),
            team: TeamTag::Machining,
            display_name: "5-Axis Simultaneous Machining Cell".to_string(),
            description: "Single-setup complex geometry mill for aerospace/medical and multi-face parts.".to_string(),
            price: Money::from_dollars(165_000),
            monthly_cost: Money::from_dollars(900),
            lead_time_days_min: 14,
            lead_time_days_max: 28,
            width_mm: 2_800,
            depth_mm: 2_500,
            height_mm: 2_600,
            clearance_mm: 500,
            power_kw: 35,
            needs_three_phase_480v: true,
            needs_compressed_air: true,
            needs_climate_room: false,
            needs_hazmat_permit: false,
            financeable: true,
            input_squares: vec![("part_in".to_string(), vec!["blank-alu".to_string(), "blank-steel".to_string()])],
            output_square_skus: vec!["machined-part".to_string()],
            cycle_time_summary: "12–22 min/part (eliminates 2nd setup)".to_string(),
            available_addons: {
                let mut a = common_addons.clone();
                a.push(CellAddonSpec {
                    addon_id: "pallet-changer".to_string(),
                    name: "Zero-Point 6-Pallet Pool".to_string(),
                    price_delta: Money::from_dollars(42_000),
                    cycle_time_pct_delta: -20,
                    description: "Allows fast pallet swaps and unattended cycles.".to_string(),
                });
                a
            },
        },
        CellCatalogEntry {
            catalog_id: "cnc-turning-lathe".to_string(),
            team: TeamTag::Machining,
            display_name: "CNC Turning Center with Live Tooling".to_string(),
            description: "High-speed turning center for shafts, bushings, and threaded fittings.".to_string(),
            price: Money::from_dollars(78_000),
            monthly_cost: Money::from_dollars(500),
            lead_time_days_min: 10,
            lead_time_days_max: 21,
            width_mm: 2_600,
            depth_mm: 1_800,
            height_mm: 2_100,
            clearance_mm: 400,
            power_kw: 25,
            needs_three_phase_480v: true,
            needs_compressed_air: true,
            needs_climate_room: false,
            needs_hazmat_permit: false,
            financeable: true,
            input_squares: vec![("part_in".to_string(), vec!["blank-alu".to_string(), "blank-steel".to_string()])],
            output_square_skus: vec!["machined-part".to_string()],
            cycle_time_summary: "8–18 min/part".to_string(),
            available_addons: {
                let mut a = common_addons;
                a.push(CellAddonSpec {
                    addon_id: "bar-feeder".to_string(),
                    name: "Automatic Magazine Bar Feeder".to_string(),
                    price_delta: Money::from_dollars(24_000),
                    cycle_time_pct_delta: -15,
                    description: "Feeds bar stock automatically between cycles.".to_string(),
                });
                a
            },
        },
        CellCatalogEntry {
            catalog_id: "cnc-compact-mill".to_string(),
            team: TeamTag::Machining,
            display_name: "Compact High-Speed Prototyping Mill".to_string(),
            description: "Fits through narrow urban/container doors. Fast on small aluminum parts.".to_string(),
            price: Money::from_dollars(36_000),
            monthly_cost: Money::from_dollars(250),
            lead_time_days_min: 4,
            lead_time_days_max: 8,
            width_mm: 1_600,
            depth_mm: 1_400,
            height_mm: 2_000,
            clearance_mm: 300,
            power_kw: 12,
            needs_three_phase_480v: true,
            needs_compressed_air: true,
            needs_climate_room: false,
            needs_hazmat_permit: false,
            financeable: true,
            input_squares: vec![("part_in".to_string(), vec!["blank-alu".to_string()])],
            output_square_skus: vec!["machined-part".to_string()],
            cycle_time_summary: "20–40 min/part (aluminum only)".to_string(),
            available_addons: vec![],
        },
    ]
}

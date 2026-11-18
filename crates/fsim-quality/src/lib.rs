//! `fsim-quality`: Quality inspection vertical slice (manual granite metrology bench, optical
//! vision measuring system, and bridge CMM requiring a temperature-controlled room).

pub mod reference;

use fsim_core::{CellCatalogEntry, Money};
use fsim_protocols::TeamTag;

/// Starter catalog of quality inspection stations.
pub fn starter_quality_catalog() -> Vec<CellCatalogEntry> {
    vec![
        CellCatalogEntry {
            catalog_id: "qual-manual-bench".to_string(),
            team: TeamTag::Quality,
            display_name: "Manual Metrology Bench (Granite Plate, Height Gauge, Mics)".to_string(),
            description: "First-article and sample inspection bench. Catches ~86% of out-of-spec parts before shipping.".to_string(),
            price: Money::from_dollars(8_500),
            monthly_cost: Money::from_dollars(120),
            lead_time_days_min: 3,
            lead_time_days_max: 7,
            width_mm: 1_800,
            depth_mm: 1_200,
            height_mm: 1_500,
            clearance_mm: 300,
            power_kw: 1,
            needs_three_phase_480v: false,
            needs_compressed_air: false,
            needs_climate_room: false,
            needs_hazmat_permit: false,
            financeable: false,
            input_squares: vec![(
                "part_in".to_string(),
                vec!["machined-part".to_string(), "molded-part".to_string()],
            )],
            output_square_skus: vec!["inspected-part".to_string()],
            cycle_time_summary: "3 min/part (86% defect catch rate)".to_string(),
            available_addons: vec![],
        },
        CellCatalogEntry {
            catalog_id: "qual-vision-system".to_string(),
            team: TeamTag::Quality,
            display_name: "Optical Vision Measuring System".to_string(),
            description: "Sub-second 2D profile and hole-pattern inspection for turned and molded parts.".to_string(),
            price: Money::from_dollars(45_000),
            monthly_cost: Money::from_dollars(300),
            lead_time_days_min: 7,
            lead_time_days_max: 14,
            width_mm: 1_500,
            depth_mm: 1_400,
            height_mm: 1_800,
            clearance_mm: 300,
            power_kw: 2,
            needs_three_phase_480v: false,
            needs_compressed_air: false,
            needs_climate_room: false,
            needs_hazmat_permit: false,
            financeable: true,
            input_squares: vec![(
                "part_in".to_string(),
                vec!["machined-part".to_string(), "molded-part".to_string()],
            )],
            output_square_skus: vec!["inspected-part".to_string()],
            cycle_time_summary: "40 sec/part (94% defect catch rate)".to_string(),
            available_addons: vec![],
        },
        CellCatalogEntry {
            catalog_id: "qual-bridge-cmm".to_string(),
            team: TeamTag::Quality,
            display_name: "Bridge Coordinate Measuring Machine (CMM)".to_string(),
            description: "Required for AS9100 aerospace and ISO 13485 medical markets. Needs a temperature-controlled room.".to_string(),
            price: Money::from_dollars(125_000),
            monthly_cost: Money::from_dollars(750),
            lead_time_days_min: 14,
            lead_time_days_max: 28,
            width_mm: 2_400,
            depth_mm: 2_200,
            height_mm: 2_500,
            clearance_mm: 400,
            power_kw: 4,
            needs_three_phase_480v: false,
            needs_compressed_air: true,
            needs_climate_room: true,
            needs_hazmat_permit: false,
            financeable: true,
            input_squares: vec![(
                "part_in".to_string(),
                vec!["machined-part".to_string(), "molded-part".to_string()],
            )],
            output_square_skus: vec!["inspected-part".to_string()],
            cycle_time_summary: "60 sec/part (99.2% defect catch rate)".to_string(),
            available_addons: vec![],
        },
    ]
}

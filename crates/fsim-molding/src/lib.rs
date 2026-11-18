//! `fsim-molding`: Injection molding presses, resin dryers, hidden process windows tuned
//! shot-by-shot, and changeover models.

pub mod reference;

use fsim_core::{CellAddonSpec, CellCatalogEntry, Money};
use fsim_protocols::TeamTag;

/// Starter catalog of injection molding presses and auxiliary cells.
pub fn starter_molding_catalog() -> Vec<CellCatalogEntry> {
    let press_addons = vec![
        CellAddonSpec {
            addon_id: "robot-picker".to_string(),
            name: "3-Axis Servo Sprue & Part Picker".to_string(),
            price_delta: Money::from_dollars(16_000),
            cycle_time_pct_delta: -18,
            description: "Consistent part demolding and sprue separation.".to_string(),
        },
        CellAddonSpec {
            addon_id: " desiccant-dryer".to_string(),
            name: "Hopper Desiccant Resin Dryer".to_string(),
            price_delta: Money::from_dollars(6_500),
            cycle_time_pct_delta: 0,
            description: "Prevents moisture splay and brittle parts in hygroscopic resins (Nylon, PC, ABS).".to_string(),
        },
        CellAddonSpec {
            addon_id: "quick-mold-clamps".to_string(),
            name: "Magnetic Quick Mold Change Platens".to_string(),
            price_delta: Money::from_dollars(22_000),
            cycle_time_pct_delta: -10,
            description: "Cuts mold changeover time from 90 minutes to 15 minutes.".to_string(),
        },
    ];

    vec![
        CellCatalogEntry {
            catalog_id: "mold-press-55t-electric".to_string(),
            team: TeamTag::Molding,
            display_name: "55-Ton All-Electric Injection Press".to_string(),
            description: "Clean, repeatable small-tonnage press for medical and precision technical plastic parts.".to_string(),
            price: Money::from_dollars(92_000),
            monthly_cost: Money::from_dollars(520),
            lead_time_days_min: 10,
            lead_time_days_max: 21,
            width_mm: 3_400,
            depth_mm: 1_500,
            height_mm: 2_100,
            clearance_mm: 400,
            power_kw: 18,
            needs_three_phase_480v: true,
            needs_compressed_air: true,
            needs_climate_room: false,
            needs_hazmat_permit: false,
            financeable: true,
            input_squares: vec![
                ("part_in".to_string(), vec!["resin-pellet".to_string()]),
                ("mold_in".to_string(), vec!["injection-mold".to_string()]),
            ],
            output_square_skus: vec!["molded-part".to_string()],
            cycle_time_summary: "15–45 sec/shot (wall thickness dependent)".to_string(),
            available_addons: press_addons.clone(),
        },
        CellCatalogEntry {
            catalog_id: "mold-press-180t-hydraulic".to_string(),
            team: TeamTag::Molding,
            display_name: "180-Ton Servo-Hydraulic Injection Press".to_string(),
            description: "Mid-tonnage press for larger housings, enclosures, and multi-cavity family molds.".to_string(),
            price: Money::from_dollars(135_000),
            monthly_cost: Money::from_dollars(750),
            lead_time_days_min: 14,
            lead_time_days_max: 28,
            width_mm: 4_800,
            depth_mm: 1_900,
            height_mm: 2_400,
            clearance_mm: 500,
            power_kw: 38,
            needs_three_phase_480v: true,
            needs_compressed_air: true,
            needs_climate_room: false,
            needs_hazmat_permit: false,
            financeable: true,
            input_squares: vec![
                ("part_in".to_string(), vec!["resin-pellet".to_string()]),
                ("mold_in".to_string(), vec!["injection-mold".to_string()]),
            ],
            output_square_skus: vec!["molded-part".to_string()],
            cycle_time_summary: "22–60 sec/shot".to_string(),
            available_addons: press_addons,
        },
        CellCatalogEntry {
            catalog_id: "mold-press-vertical-insert".to_string(),
            team: TeamTag::Molding,
            display_name: "35-Ton Rotary Table Vertical Insert Press".to_string(),
            description: "Overmolds plastic onto CNC-machined brass/steel inserts.".to_string(),
            price: Money::from_dollars(74_000),
            monthly_cost: Money::from_dollars(420),
            lead_time_days_min: 10,
            lead_time_days_max: 20,
            width_mm: 2_200,
            depth_mm: 1_800,
            height_mm: 2_400,
            clearance_mm: 400,
            power_kw: 16,
            needs_three_phase_480v: true,
            needs_compressed_air: true,
            needs_climate_room: false,
            needs_hazmat_permit: false,
            financeable: true,
            input_squares: vec![
                ("part_in".to_string(), vec!["resin-pellet".to_string()]),
                ("insert_in".to_string(), vec!["machined-part".to_string()]),
            ],
            output_square_skus: vec!["molded-part".to_string()],
            cycle_time_summary: "30–55 sec/shot".to_string(),
            available_addons: vec![],
        },
    ]
}

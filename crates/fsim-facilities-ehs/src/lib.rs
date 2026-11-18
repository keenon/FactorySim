//! `fsim-facilities-ehs`: Building listings (open warehouse, urban multi-column building,
//! shipping container unit), leasing/keys/utility build-out pipelines, permits, layout
//! validation, and waste/chip-hopper support cells.

pub mod reference;

use fsim_core::{CellAddonSpec, CellCatalogEntry, Money, SimTime};
use fsim_geom::{PointMm, PolygonWithHolesMm};
use fsim_protocols::TeamTag;
use serde::{Deserialize, Serialize};

/// A real-estate listing available on the Plants tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingListing {
    pub listing_id: String,
    pub name: String,
    pub building_kind: String, // "warehouse", "urban", "container"
    pub description: String,
    pub floor_polygon: PolygonWithHolesMm,
    pub clear_height_mm: i32,
    pub max_door_width_mm: i32,
    pub max_door_height_mm: i32,
    pub dock_doors: u32,
    pub power_capacity_kw: u32,
    pub has_three_phase_480v: bool,
    pub monthly_rent: Money,
    pub lease_deposit: Money,
    pub keys_lead_days_min: u32,
    pub keys_lead_days_max: u32,
    pub utilities_lead_days_min: u32,
    pub utilities_lead_days_max: u32,
}

/// Active state of a leased plant site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlantSiteState {
    pub plant_id: String,
    pub listing_id: String,
    pub name: String,
    pub leased_at: SimTime,
    pub keys_ready_at: SimTime,
    pub utilities_ready_at: SimTime,
    pub keys_received: bool,
    pub utilities_ready: bool,
    pub climate_room_built: bool,
    pub hazmat_permit_approved: bool,
    pub layout_version: u64,
}

impl PlantSiteState {
    pub fn is_operational(&self, now: SimTime) -> bool {
        (self.keys_received || now >= self.keys_ready_at)
            && (self.utilities_ready || now >= self.utilities_ready_at)
    }
}

/// Starter catalog of building listings covering the three canonical site archetypes.
pub fn starter_building_listings() -> Vec<BuildingListing> {
    let mut urban_poly = PolygonWithHolesMm::rectangle(18_000, 14_000);
    // Interior structural columns in the urban building
    urban_poly.holes.push(vec![
        PointMm::new(6_000, 6_000),
        PointMm::new(6_600, 6_000),
        PointMm::new(6_600, 6_600),
        PointMm::new(6_000, 6_600),
    ]);
    urban_poly.holes.push(vec![
        PointMm::new(12_000, 6_000),
        PointMm::new(12_600, 6_000),
        PointMm::new(12_600, 6_600),
        PointMm::new(12_000, 6_600),
    ]);

    vec![
        BuildingListing {
            listing_id: "bldg-suburban-bay".to_string(),
            name: "Suburban Light-Industrial Bay (20m x 15m)".to_string(),
            building_kind: "warehouse".to_string(),
            description: "Clean rectangular bay with 480V 3-phase power, 5m clear height, and grade roll-up door. Ideal starter plant.".to_string(),
            floor_polygon: PolygonWithHolesMm::rectangle(20_000, 15_000),
            clear_height_mm: 5_000,
            max_door_width_mm: 3_600,
            max_door_height_mm: 4_000,
            dock_doors: 1,
            power_capacity_kw: 150,
            has_three_phase_480v: true,
            monthly_rent: Money::from_dollars(4_500),
            lease_deposit: Money::from_dollars(4_500),
            keys_lead_days_min: 3,
            keys_lead_days_max: 7,
            utilities_lead_days_min: 2,
            utilities_lead_days_max: 5,
        },
        BuildingListing {
            listing_id: "bldg-open-warehouse".to_string(),
            name: "Regional Distribution & Manufacturing Warehouse (40m x 25m)".to_string(),
            building_kind: "warehouse".to_string(),
            description: "Large high-bay warehouse with 8m clear height, 4 dock doors, and 500 kW service.".to_string(),
            floor_polygon: PolygonWithHolesMm::rectangle(40_000, 25_000),
            clear_height_mm: 8_000,
            max_door_width_mm: 4_500,
            max_door_height_mm: 4_500,
            dock_doors: 4,
            power_capacity_kw: 500,
            has_three_phase_480v: true,
            monthly_rent: Money::from_dollars(14_000),
            lease_deposit: Money::from_dollars(14_000),
            keys_lead_days_min: 7,
            keys_lead_days_max: 14,
            utilities_lead_days_min: 5,
            utilities_lead_days_max: 10,
        },
        BuildingListing {
            listing_id: "bldg-urban-loft".to_string(),
            name: "Urban Multi-Column Loft Shop (18m x 14m)".to_string(),
            building_kind: "urban".to_string(),
            description: "Close to customers, but interior columns, 3.0m low ceiling, and narrow 2.1m freight door constrain large machines.".to_string(),
            floor_polygon: urban_poly,
            clear_height_mm: 3_000,
            max_door_width_mm: 2_100,
            max_door_height_mm: 2_500,
            dock_doors: 0,
            power_capacity_kw: 80,
            has_three_phase_480v: true,
            monthly_rent: Money::from_dollars(3_200),
            lease_deposit: Money::from_dollars(3_200),
            keys_lead_days_min: 2,
            keys_lead_days_max: 5,
            utilities_lead_days_min: 3,
            utilities_lead_days_max: 7,
        },
        BuildingListing {
            listing_id: "bldg-container-40hc".to_string(),
            name: "40ft High-Cube Container Micro-Factory (12m x 2.35m)".to_string(),
            building_kind: "container".to_string(),
            description: "Relocatable ISO shipping container unit (12.0m x 2.35m x 2.7m). Fast deployment, tight footprint.".to_string(),
            floor_polygon: PolygonWithHolesMm::rectangle(12_000, 2_350),
            clear_height_mm: 2_700,
            max_door_width_mm: 2_300,
            max_door_height_mm: 2_580,
            dock_doors: 1,
            power_capacity_kw: 45,
            has_three_phase_480v: true,
            monthly_rent: Money::from_dollars(950),
            lease_deposit: Money::from_dollars(1_500),
            keys_lead_days_min: 1,
            keys_lead_days_max: 2,
            utilities_lead_days_min: 1,
            utilities_lead_days_max: 2,
        },
    ]
}

/// Starter catalog of Facilities & EHS waste/utility support cells.
pub fn starter_facilities_catalog() -> Vec<CellCatalogEntry> {
    vec![
        CellCatalogEntry {
            catalog_id: "ehs-chip-hopper".to_string(),
            team: TeamTag::FacilitiesEhs,
            display_name: "Self-Dumping Chip Hopper & Scrap Dumpster".to_string(),
            description: "Collects aluminum/steel swarf and offcuts; sorted chips earn scrap revenue.".to_string(),
            price: Money::from_dollars(1_800),
            monthly_cost: Money::from_dollars(150),
            lead_time_days_min: 2,
            lead_time_days_max: 5,
            width_mm: 1_500,
            depth_mm: 1_200,
            height_mm: 1_200,
            clearance_mm: 200,
            power_kw: 0,
            needs_three_phase_480v: false,
            needs_compressed_air: false,
            needs_climate_room: false,
            needs_hazmat_permit: false,
            financeable: false,
            input_squares: vec![("waste_in".to_string(), vec!["chips-metal".to_string()])],
            output_square_skus: vec![],
            cycle_time_summary: "Holds 500 kg swarf before dump".to_string(),
            available_addons: vec![CellAddonSpec {
                addon_id: "chip-wringer".to_string(),
                name: "Centrifugal Chip Wringer".to_string(),
                price_delta: Money::from_dollars(18_000),
                cycle_time_pct_delta: -20,
                description: "Recovers coolant and increases chip scrap payout by 25%".to_string(),
            }],
        },
        CellCatalogEntry {
            catalog_id: "ehs-drum-containment".to_string(),
            team: TeamTag::FacilitiesEhs,
            display_name: "Spent Coolant Drum Spill Containment".to_string(),
            description: "EPA-compliant pallet for spent coolant and way oil drums.".to_string(),
            price: Money::from_dollars(650),
            monthly_cost: Money::from_dollars(80),
            lead_time_days_min: 1,
            lead_time_days_max: 3,
            width_mm: 1_300,
            depth_mm: 1_300,
            height_mm: 900,
            clearance_mm: 200,
            power_kw: 0,
            needs_three_phase_480v: false,
            needs_compressed_air: false,
            needs_climate_room: false,
            needs_hazmat_permit: false,
            financeable: false,
            input_squares: vec![("waste_in".to_string(), vec!["spent-coolant".to_string()])],
            output_square_skus: vec![],
            cycle_time_summary: "4-drum spill pallet".to_string(),
            available_addons: vec![],
        },
        CellCatalogEntry {
            catalog_id: "ehs-climate-enclosure".to_string(),
            team: TeamTag::FacilitiesEhs,
            display_name: "Modular Temperature-Controlled CMM Room".to_string(),
            description: "20°C ± 0.5°C enclosure enabling Bridge CMM precision inspection.".to_string(),
            price: Money::from_dollars(28_000),
            monthly_cost: Money::from_dollars(350),
            lead_time_days_min: 7,
            lead_time_days_max: 14,
            width_mm: 3_000,
            depth_mm: 3_000,
            height_mm: 2_600,
            clearance_mm: 200,
            power_kw: 8,
            needs_three_phase_480v: true,
            needs_compressed_air: false,
            needs_climate_room: false,
            needs_hazmat_permit: false,
            financeable: true,
            input_squares: vec![],
            output_square_skus: vec![],
            cycle_time_summary: "Unlocks climate-controlled CMM cells".to_string(),
            available_addons: vec![],
        },
    ]
}

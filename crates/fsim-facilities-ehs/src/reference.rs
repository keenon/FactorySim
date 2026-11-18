//! Exact placement and building constraint validator reference oracle for `fsim-facilities-ehs`.

use crate::{BuildingListing, PlantSiteState};
use fsim_core::CellCatalogEntry;
use fsim_geom::{fits_through_opening, AabbMm};

/// Validates whether a cell with `catalog` placed at `(x_mm, y_mm, rotation_deg)` is legal inside `plant`,
/// returning `Ok(())` or a list of human-readable violation reasons.
pub fn reference_validate_cell_placement(
    plant: &PlantSiteState,
    listing: &BuildingListing,
    catalog: &CellCatalogEntry,
    x_mm: i32,
    y_mm: i32,
    rotation_deg: i32,
    existing_footprints: &[(String, AabbMm)],
    ignore_cell_id: Option<&str>,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if catalog.height_mm > listing.clear_height_mm {
        errors.push(format!(
            "Needs {}m clear height (building has {:.1}m)",
            (catalog.height_mm as f64) / 1000.0,
            (listing.clear_height_mm as f64) / 1000.0
        ));
    }

    if !fits_through_opening(
        catalog.width_mm,
        catalog.depth_mm,
        catalog.height_mm,
        listing.max_door_width_mm,
        listing.max_door_height_mm,
    ) {
        errors.push(format!(
            "Cannot fit through building door ({:.1}m x {:.1}m)",
            (listing.max_door_width_mm as f64) / 1000.0,
            (listing.max_door_height_mm as f64) / 1000.0
        ));
    }

    if catalog.needs_three_phase_480v && !listing.has_three_phase_480v && !plant.utilities_ready {
        errors.push("Needs 480V three-phase power".to_string());
    }

    if catalog.needs_hazmat_permit && !plant.hazmat_permit_approved {
        errors.push("Needs an approved hazmat permit".to_string());
    }

    if catalog.needs_climate_room && !plant.climate_room_built {
        errors.push("Needs a temperature-controlled room".to_string());
    }

    let footprint = catalog
        .footprint_at(x_mm, y_mm, rotation_deg)
        .inflate(catalog.clearance_mm);
    if !listing.floor_polygon.contains_aabb(footprint) {
        errors.push("Footprint + clearance extends outside building walls or overlaps a column".to_string());
    }

    for (other_id, other_box) in existing_footprints {
        if Some(other_id.as_str()) == ignore_cell_id {
            continue;
        }
        if footprint.overlaps_interior(other_box) {
            errors.push(format!("Overlaps clearance of cell '{other_id}'"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

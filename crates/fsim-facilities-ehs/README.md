# `fsim-facilities-ehs`

Building models (open warehouse, urban multi-column shop, shipping container micro-factory), plant leasing/keys/utility pipelines, layout placement validation, and waste/EHS support cells.

- **State & Caching**: Stateless (`PlantSiteState` is caller-owned)
- **Dependencies**: Workspace: `fsim-core`, `fsim-geom`, `fsim-protocols` | External: `serde`
- **Data Flow**: Consumes `PlantSiteState`, `BuildingListing`, `CellCatalogEntry` -> Produces placement validation verdicts and starter site/waste catalogs

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns building envelope rules (clear height, door openings, 480V power, hazmat permits, climate enclosures) and waste collection cells.
- **Core Invariants**:
  - **Door & Clear-Height Gate**: No cell can be placed if its height exceeds `clear_height_mm` or if it cannot pass through `max_door_width_mm x max_door_height_mm`.
  - **Exact Footprint Clearance**: Every placed cell's clearance-inflated AABB must lie strictly within `floor_polygon` (outside interior column holes) and never overlap another cell's clearance box.
- **Conditioning Variables**: Permit and build-out lead times conditioned on building archetype (`warehouse`, `urban`, `container`).

## Algorithms Implemented (Do Not Duplicate)
- **Exact Building & Placement Validator** (`reference::reference_validate_cell_placement`): Checks door fit, clear height, utility/permit prerequisites, building polygon containment, and pairwise cell clearance non-overlap.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `BuildingListing`, `PlantSiteState`, `starter_building_listings()`, `starter_facilities_catalog()`.
- **[`src/reference.rs`](src/reference.rs)**: Exact placement constraint validator (`reference_validate_cell_placement`).

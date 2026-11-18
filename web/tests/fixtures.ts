import type { BuildingListing } from '../src/types';

export const LISTING: BuildingListing = {
  listing_id: 'b',
  name: 'Bay',
  building_kind: 'warehouse',
  description: '',
  floor_polygon: {
    outer: [
      { x: 0, y: 0 },
      { x: 20000, y: 0 },
      { x: 20000, y: 15000 },
      { x: 0, y: 15000 },
    ],
    holes: [
      [
        { x: 9000, y: 7000 },
        { x: 9600, y: 7000 },
        { x: 9600, y: 7600 },
        { x: 9000, y: 7600 },
      ],
    ],
  },
  clear_height_mm: 5000,
  max_door_width_mm: 3600,
  max_door_height_mm: 4000,
  dock_doors: 1,
  power_capacity_kw: 150,
  has_three_phase_480v: true,
  monthly_rent: 0,
  lease_deposit: 0,
  keys_lead_days_min: 1,
  keys_lead_days_max: 2,
  utilities_lead_days_min: 1,
  utilities_lead_days_max: 2,
};

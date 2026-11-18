//! `fsim-layout-graph`: Derives a stateless transport network (walking and forklift routes
//! between cell handoff squares) from a building layout and installed cell footprints.
//! No team crate reads the floor plan directly; layout reaches the sim through this graph.

pub mod reference;

use fsim_geom::{AabbMm, PointMm, PolygonWithHolesMm};
use fsim_navgrid::NavGrid;
use fsim_path_vehicle::VehicleSpec;
use fsim_path_walk::plan_walk_path;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A handoff square endpoint registered in the layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquareEndpoint {
    pub cell_id: String,
    pub square_name: String,
    pub access_point: PointMm,
}

/// Computed route between two handoff squares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutRoute {
    pub from_cell: String,
    pub from_square: String,
    pub to_cell: String,
    pub to_square: String,
    pub waypoints: Vec<PointMm>,
    pub distance_mm: i64,
    pub forklift_feasible: bool,
}

pub mod vec_map_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S, K, V>(map: &BTreeMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        K: Serialize + Ord,
        V: Serialize,
    {
        let vec: Vec<(&K, &V)> = map.iter().collect();
        vec.serialize(serializer)
    }

    pub fn deserialize<'de, D, K, V>(deserializer: D) -> Result<BTreeMap<K, V>, D::Error>
    where
        D: Deserializer<'de>,
        K: Deserialize<'de> + Ord,
        V: Deserialize<'de>,
    {
        let vec: Vec<(K, V)> = Vec::deserialize(deserializer)?;
        Ok(vec.into_iter().collect())
    }
}

/// Transport graph for a single plant floor plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportGraph {
    pub layout_version: u64,
    #[serde(with = "vec_map_serde")]
    pub endpoints: BTreeMap<(String, String), SquareEndpoint>,
    #[serde(with = "vec_map_serde")]
    pub routes: BTreeMap<((String, String), (String, String)), LayoutRoute>,
}

/// Key of a handoff square: `(cell_id, square_name)`.
pub type SquareKey = (String, String);

/// Grid resolution and pedestrian body radius shared by graph routes and on-demand walking trips,
/// so a worker's walk and the drawn route are the same path.
pub const WALK_GRID_CELL_MM: i32 = 200;
pub const WALK_BODY_RADIUS_MM: i32 = 250;

/// Rasterizes the pedestrian navigation grid for a floor with machine footprints as obstacles.
pub fn build_walk_grid(building: &PolygonWithHolesMm, obstacles: &[AabbMm]) -> NavGrid {
    NavGrid::build(building, obstacles, WALK_GRID_CELL_MM, WALK_BODY_RADIUS_MM)
}

impl TransportGraph {
    /// Derives routes for the given directed material `flows` (output square -> input square),
    /// walking around `obstacles` (machine footprints). Endpoints lying on or inside a footprint
    /// edge snap to the nearest free grid cell, so squares at a cell's boundary stay reachable.
    pub fn build(
        layout_version: u64,
        building: &PolygonWithHolesMm,
        obstacles: &[AabbMm],
        squares: &[SquareEndpoint],
        flows: &[(SquareKey, SquareKey)],
    ) -> Self {
        let walk_grid = build_walk_grid(building, obstacles);
        let forklift_spec = VehicleSpec::standard_forklift(true);
        let forklift_grid = NavGrid::build(
            building,
            obstacles,
            WALK_GRID_CELL_MM,
            forklift_spec.ClearanceRadiusMm(),
        );
        let mut endpoints = BTreeMap::new();
        for sq in squares {
            endpoints.insert((sq.cell_id.clone(), sq.square_name.clone()), sq.clone());
        }

        let mut routes = BTreeMap::new();
        for (from, to) in flows {
            let (Some(a), Some(b)) = (endpoints.get(from), endpoints.get(to)) else {
                continue;
            };
            if from == to {
                continue;
            }
            if let Some((waypoints, distance_mm)) =
                plan_walk_path(&walk_grid, a.access_point, b.access_point)
            {
                let forklift_feasible =
                    plan_walk_path(&forklift_grid, a.access_point, b.access_point).is_some();
                routes.insert(
                    (from.clone(), to.clone()),
                    LayoutRoute {
                        from_cell: from.0.clone(),
                        from_square: from.1.clone(),
                        to_cell: to.0.clone(),
                        to_square: to.1.clone(),
                        waypoints,
                        distance_mm,
                        forklift_feasible,
                    },
                );
            }
        }

        Self {
            layout_version,
            endpoints,
            routes,
        }
    }

    pub fn route(&self, from_cell: &str, from_sq: &str, to_cell: &str, to_sq: &str) -> Option<&LayoutRoute> {
        let key = (
            (from_cell.to_string(), from_sq.to_string()),
            (to_cell.to_string(), to_sq.to_string()),
        );
        self.routes.get(&key)
    }

    /// Fallback Euclidean distance if a cell is not yet in the precomputed pair table.
    pub fn distance_or_default_mm(
        &self,
        from_cell: &str,
        from_sq: &str,
        to_cell: &str,
        to_sq: &str,
    ) -> i64 {
        if let Some(r) = self.route(from_cell, from_sq, to_cell, to_sq) {
            return r.distance_mm;
        }
        if let (Some(a), Some(b)) = (
            self.endpoints.get(&(from_cell.to_string(), from_sq.to_string())),
            self.endpoints.get(&(to_cell.to_string(), to_sq.to_string())),
        ) {
            return a.access_point.distance_mm(b.access_point);
        }
        5_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_graph_derives_routes_between_squares() {
        let bldg = PolygonWithHolesMm::rectangle(15_000, 10_000);
        let obs = vec![AabbMm::new(6_000, 2_000, 8_000, 6_000)];
        let squares = vec![
            SquareEndpoint {
                cell_id: "recv-1".to_string(),
                square_name: "out".to_string(),
                access_point: PointMm::new(2_000, 4_000),
            },
            SquareEndpoint {
                cell_id: "cnc-1".to_string(),
                square_name: "part_in".to_string(),
                access_point: PointMm::new(10_000, 4_000),
            },
        ];

        let flows = vec![(
            ("recv-1".to_string(), "out".to_string()),
            ("cnc-1".to_string(), "part_in".to_string()),
        )];
        let graph = TransportGraph::build(1, &bldg, &obs, &squares, &flows);
        let route = graph.route("recv-1", "out", "cnc-1", "part_in").unwrap();
        assert!(route.distance_mm > 8_000); // Detours around obstacle
        assert!(!route.waypoints.is_empty());
        // Only declared flows are routed.
        assert!(graph.route("cnc-1", "part_in", "recv-1", "out").is_none());
        assert_eq!(graph.routes.len(), 1);
    }
}

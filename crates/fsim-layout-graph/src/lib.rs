//! `fsim-layout-graph`: Derives a stateless transport network (walking and forklift routes
//! between cell handoff squares) from a building layout and installed cell footprints.
//! No team crate reads the floor plan directly; layout reaches the sim through this graph.

pub mod reference;

use fsim_geom::{AabbMm, PointMm, PolygonWithHolesMm};
use fsim_motion::Pose2D;
use fsim_navgrid::NavGrid;
use fsim_path_vehicle::{plan_vehicle_path, VehicleSpec};
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

impl TransportGraph {
    /// Derives the transport graph for all registered handoff squares on a plant floor.
    pub fn build(
        layout_version: u64,
        building: &PolygonWithHolesMm,
        obstacles: &[AabbMm],
        squares: &[SquareEndpoint],
    ) -> Self {
        let walk_grid = NavGrid::build(building, obstacles, 100, 250);
        let forklift_spec = VehicleSpec::standard_forklift(true);
        let mut endpoints = BTreeMap::new();
        for sq in squares {
            endpoints.insert((sq.cell_id.clone(), sq.square_name.clone()), sq.clone());
        }

        let mut routes = BTreeMap::new();
        for a in squares {
            for b in squares {
                if a.cell_id == b.cell_id && a.square_name == b.square_name {
                    continue;
                }
                if let Some((waypoints, distance_mm)) =
                    plan_walk_path(&walk_grid, a.access_point, b.access_point)
                {
                    let forklift_feasible = plan_vehicle_path(
                        building,
                        obstacles,
                        &forklift_spec,
                        Pose2D::new(a.access_point.x, a.access_point.y, 0),
                        Pose2D::new(b.access_point.x, b.access_point.y, 0),
                    )
                    .is_some();
                    let key = (
                        (a.cell_id.clone(), a.square_name.clone()),
                        (b.cell_id.clone(), b.square_name.clone()),
                    );
                    routes.insert(
                        key,
                        LayoutRoute {
                            from_cell: a.cell_id.clone(),
                            from_square: a.square_name.clone(),
                            to_cell: b.cell_id.clone(),
                            to_square: b.square_name.clone(),
                            waypoints,
                            distance_mm,
                            forklift_feasible,
                        },
                    );
                }
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

        let graph = TransportGraph::build(1, &bldg, &obs, &squares);
        let route = graph.route("recv-1", "out", "cnc-1", "part_in").unwrap();
        assert!(route.distance_mm > 8_000); // Detours around obstacle
        assert!(!route.waypoints.is_empty());
    }
}

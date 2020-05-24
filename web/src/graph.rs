// based on https://rosettacode.org/wiki/Dijkstra%27s_algorithm#Rust
use std::cmp::{Ordering};
use std::collections::{BinaryHeap, HashMap};
use std::usize;

use Node;
use Way;

// First three digits of coordinates are used for the grid hashing
const GRID_MULTIPLICATOR: usize = 100;
const MAX_F64: f64 = std::f64::MAX;

#[derive(Clone)]
pub struct Graph {
    nodes: Vec<Node>,
    ways: Vec<Way>,
    offset: Vec<usize>,
    grid: HashMap<(usize, usize), Vec<usize>>,
}

#[derive(Clone)]
pub struct DijkstraResult {
    pub path: Vec<usize>,
    pub distance: f64,
    pub ele_rise: f64,
    pub multiplier: Option<f64>,
}

impl PartialEq for DijkstraResult {
    fn eq(&self, other: &Self) -> bool {
        return self.distance == other.distance && self.ele_rise == other.ele_rise;
    }
}

pub enum Dijkstra {
    Elevation,
    Multiplier,
}

#[derive(Copy, Clone)]
struct State {
    node: usize,
    cost: f64,
    distance: f64,
    ele_rise: f64,
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        return self.node == other.node && self.cost == other.cost;
    }

    fn ne(&self, other: &Self) -> bool {
        return !self.eq(other);
    }
}

impl Eq for State {}

// Manually implement Ord so we get a min-heap instead of a max-heap
impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        let cmp = other.cost - self.cost;
        if cmp == 0.0 { return Ordering::Equal; };
        if cmp > 0.0 { return Ordering::Greater; }
        return Ordering::Less;
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Graph {
    pub fn new(
        nodes: Vec<Node>,
        ways: Vec<Way>,
        offset: Vec<usize>,
        grid: HashMap<(usize, usize), Vec<usize>>,
    ) -> Self {
        Graph {
            nodes,
            ways,
            offset,
            grid,
        }
    }

    /// returns closest point of given long & lat
    pub fn get_point_id(&self, lat: f32, long: f32, travel_type: usize) -> usize {
        let mut min_distance: f32 = std::f32::MAX;
        let mut min_distance_id: usize = 0;
        let allowed_types = self.get_allowed_types(travel_type);
        let adjacent_nodes = self.get_adjacent_node_ids(lat, long, &allowed_types);
        for node_id in adjacent_nodes {
            match self.nodes.get(node_id) {
                Some(node) => {
                    let distance = calc_distance(lat, long, node.latitude, node.longitude);
                    if distance < min_distance {
                        min_distance = distance;
                        min_distance_id = node_id;
                    }
                }
                None => continue
            }
        }
        return min_distance_id;
    }

    fn get_allowed_types(&self, road_type: usize) -> Vec<usize> {
        return match road_type {
            0 => vec![0, 1, 5],
            1 => vec![1, 2, 3, 5],
            2 => vec![4, 5],
            _ => vec![5]
        };
    }

    /// converts node ids to node-coordinates
    pub fn get_nodes(&self, path: Vec<usize>) -> Vec<Node> {
        return path.iter().map(|&x| self.nodes[x]).collect::<Vec<Node>>();
    }

    pub fn get_node(&self, id: usize) -> Node {
        return self.nodes[id];
    }

    fn get_edge_elevation_rise(&self, way: Way) -> f64 {
        let source_ele = self.get_node(way.source).elevation;
        let target_ele = self.get_node(way.target).elevation;
        let delta = (target_ele - source_ele) as f64;
        return if delta > 0.0 {
            delta
        } else {
            0.0
        };
    }

    /// returns the edge weight from source to target
    fn get_edge_distance(&self, way: Way, travel_type: usize, use_distance: bool) -> f64 {
        return if use_distance {
            way.distance as f64
        } else {
            if way.speed == 0 {
                return way.distance as f64;
            }
            let speed = match travel_type {
                0 => way.speed,
                1 if way.speed <= 20 => way.speed,
                1 if way.speed >= 20 => 20,
                2 => 7,
                _ => unreachable!(),
            };
            way.distance as f64 / speed as f64
        };
    }

    fn get_weight_with_multiplier(&self, distance: f64, elevation: f64, multiplier: f64) -> f64 {
        return distance + (multiplier * elevation);
    }

    fn is_valid_node_for_travel_type(&self, node_id: usize, allowed_types: &Vec<usize>) -> bool {
        let incl_start = self.offset[node_id];
        let excl_end = self.offset[node_id + 1];
        for i in incl_start..excl_end {
            let edge = &self.ways[i];
            if allowed_types.contains(&edge.travel_type) {
                return true;
            }
        }
        return false;
    }

    fn add_valid_node_ids_from_cell(&self, node_ids: &mut Vec<usize>, cell: &(usize, usize), allowed_types: &Vec<usize>) {
        match self.grid.get(cell) {
            Some(adjacent_node_ids) => node_ids.extend(adjacent_node_ids.iter().filter(|&&x| self.is_valid_node_for_travel_type(x, allowed_types)).collect::<Vec<&usize>>()),
            None => return
        }
    }


    /// returns node_ids in adjacent grid cells
    /// goes from most inner cell to cells with distance 1 to n until a node is found
    fn get_adjacent_node_ids(&self, lat: f32, lng: f32, allowed_types: &Vec<usize>) -> Vec<usize> {
        let lat_grid = (lat * GRID_MULTIPLICATOR as f32) as i32;
        let lng_grid = (lng * GRID_MULTIPLICATOR as f32) as i32;
        let mut node_ids = Vec::<usize>::new();
        self.add_valid_node_ids_from_cell(&mut node_ids, &(lat_grid as usize, lng_grid as usize), allowed_types);
        let mut in_dist: i32 = 1;
        loop {
            for i in -in_dist..in_dist {
                // top row left to right (increasing x, fix y)
                self.add_valid_node_ids_from_cell(&mut node_ids, &((lat_grid + i) as usize, (lng_grid + in_dist) as usize), allowed_types);
                // right column top to bottom (fix x, decreasing y)
                self.add_valid_node_ids_from_cell(&mut node_ids, &((lat_grid + in_dist) as usize, (lng_grid - i) as usize), allowed_types);
                // bottom row right to left (decreasing x, fix y)
                self.add_valid_node_ids_from_cell(&mut node_ids, &((lat_grid - i) as usize, (lng_grid - in_dist) as usize), allowed_types);
                // left column bottom to top (fix x, increasing y)
                self.add_valid_node_ids_from_cell(&mut node_ids, &((lat_grid - in_dist) as usize, (lng_grid + i) as usize), allowed_types);
            }
            if node_ids.len() > 0 {
                return node_ids;
            } else {
                // search in next level cells
                in_dist += 1;
            }
        }
    }

    /// executes the LARAC (Lagrange Relaxation based Aggregated Cost) algorithm and returns the shortest path with max_elevation as well as more recommendations below the max_elevation level
    pub fn find_optimal_path(&self, start: usize, end: usize, travel_type: usize, use_distance: bool, max_elevation: f64, all_paths: bool) -> Result<Vec<DijkstraResult>, String> {
        // Multiplier on 0 = 100% weight on distance
        let mut distance_result: DijkstraResult = match self.dijkstra(Dijkstra::Multiplier, start, end, travel_type, use_distance, Some(0.0)) {
            Some(d_r) => d_r,
            None => return Err("No shortest path after distance was found".to_string())
        };
        let mut elevation_result: DijkstraResult = match self.dijkstra(Dijkstra::Elevation, start, end, travel_type, use_distance, Some(std::f64::MAX)) {
            Some(d_r) => d_r,
            None => return Err("No shortest path after elevation was found".to_string())
        };

        if !all_paths && distance_result.ele_rise < max_elevation {
            // shortest path in ele range and only optimal route requested
            return Ok(vec![distance_result]);
        }

        let mut found_paths = Vec::<DijkstraResult>::new();
        //always return the shortest path as a reference to the user
        if elevation_result.eq(&distance_result) {
            // there is only one solution that's also perfect
            return Ok(found_paths);
        }
        if elevation_result.ele_rise < max_elevation {
            // add shortest path by elevation to results
            found_paths.push(elevation_result.clone());
        } else {
            return Err(format!("There is no solution, min elevation is {}", elevation_result.ele_rise).to_string());
        }

        if distance_result.ele_rise < max_elevation {
            // shortest path is in range, return path with lowest elevation and optimal path
            found_paths.push(distance_result.clone());
        }

        let mut multiplier: f64 = -1.0;
        let mut previous_multiplier;
        let  recommendation_multiplier_threshold = 0.01;
        loop {
            previous_multiplier = multiplier;
            multiplier = ((distance_result.distance - elevation_result.distance) as f64) / ((elevation_result.ele_rise - distance_result.ele_rise) as f64);
            let latest_result: DijkstraResult = match self.dijkstra(Dijkstra::Multiplier, start, end, travel_type, use_distance, Some(multiplier)) {
                Some(d_r) => d_r,
                None => break
            };

            if latest_result.multiplier == elevation_result.multiplier || latest_result.multiplier == distance_result.multiplier {
                // perfect multiplier found
                break;
            }
            if latest_result.ele_rise <= max_elevation {
                let multi_delta = (multiplier - previous_multiplier).abs();
                if multi_delta > recommendation_multiplier_threshold {
                    // add path as recommendation (optimal path for different weighting)
                    found_paths.push(latest_result.clone());
                }
                // update best path in max_elevation range
                elevation_result = latest_result.clone();
            } else {
                // update best path outside of max elevation range
                distance_result = latest_result.clone();
            }
        }

        return match all_paths {
            // return the optimal result (last one found)
            false => match found_paths.last() {
                Some(result) => Ok(vec![result.clone()]),
                None => return Err("Did not find an optimal path".to_string())
            }
            // return all paths found
            true => Ok(found_paths)
        };
    }

    fn dijkstra(&self, min_of: Dijkstra, start: usize, end: usize, travel_type: usize, use_distance: bool, multiplier: Option<f64>) -> Option<DijkstraResult> {
        let mut dist = vec![(MAX_F64, None); self.nodes.len()];

        let mut heap = BinaryHeap::new();
        dist[start] = (0.0, None);
        heap.push(State {
            node: start,
            cost: 0.0,
            distance: 0.0,
            ele_rise: 0.0,
        });

        while let Some(State { node, cost, distance, ele_rise }) = heap.pop() {
            if node == end {
                let mut path = Vec::with_capacity(dist.len() / 2);
                let mut current_dist = dist[end];
                path.push(end);
                while let Some(prev) = current_dist.1 {
                    path.push(prev);
                    current_dist = dist[prev];
                }
                path.reverse();
                return Some(DijkstraResult {
                    path,
                    distance,
                    ele_rise,
                    multiplier: multiplier,
                });
            }
            if cost > dist[node].0 as f64 {
                continue;
            }
            for edge in self.offset[node]..self.offset[node + 1] {
                let current_way: Way = self.ways[edge];
                // skip way, if the type does not match
                match travel_type {
                    0 => match current_way.travel_type {
                        0 | 1 | 5 => (),
                        _ => continue,
                    },
                    1 => match current_way.travel_type {
                        1 | 2 | 3 | 5 => (),
                        _ => continue,
                    },
                    2 => match current_way.travel_type {
                        3 | 4 | 5 => (),
                        _ => continue,
                    },
                    _ => unreachable!(),
                }
                // calculate costs
                let additional_distance = self.get_edge_distance(current_way, travel_type, use_distance);
                let additional_ele_rise = self.get_edge_elevation_rise(current_way);
                let next = State {
                    node: current_way.target,
                    cost: cost + match min_of {
                        Dijkstra::Elevation => additional_ele_rise,
                        Dijkstra::Multiplier => match multiplier {
                            Some(multiplier) => self.get_weight_with_multiplier(additional_distance, additional_ele_rise, multiplier),
                            None => panic!("Dijkstra was called in multiplier mode but no multiplier was provided")
                        }
                    },
                    distance: distance + additional_distance,
                    ele_rise: ele_rise + additional_ele_rise,
                };

                // add way to heap
                if next.cost < dist[next.node].0 as f64 {
                    dist[next.node] = (next.cost as f64, Some(node));
                    heap.push(next);
                }
            }
        }
        None
    }
}


/// get distance on earth surface using haversine formula
fn calc_distance(lat_1: f32, long_1: f32, lat_2: f32, long_2: f32) -> f32 {
    let r: f32 = 6371.0; // constant used for meters
    let d_lat: f32 = (lat_2 - lat_1).to_radians();
    let d_lon: f32 = (long_2 - long_1).to_radians();
    let lat1: f32 = (lat_1).to_radians();
    let lat2: f32 = (lat_2).to_radians();

    let a: f32 = ((d_lat / 2.0).sin()) * ((d_lat / 2.0).sin())
        + ((d_lon / 2.0).sin()) * ((d_lon / 2.0).sin()) * (lat1.cos()) * (lat2.cos());
    let c: f32 = 2.0 * ((a.sqrt()).atan2((1.0 - a).sqrt()));
    return r * c;
}

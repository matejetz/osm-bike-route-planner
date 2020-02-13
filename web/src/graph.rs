// based on https://rosettacode.org/wiki/Dijkstra%27s_algorithm#Rust
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::usize;

use Node;
use Way;

// 2^18
const COST_MULTIPLICATOR: usize = 262144;
// First three digits of coordinates are used for the grid hashing
const GRID_MULTIPLICATOR: usize = 100;

#[derive(Clone)]
pub struct Graph {
    nodes: Vec<Node>,
    ways: Vec<Way>,
    offset: Vec<usize>,
    grid: HashMap<(usize, usize), Vec<usize>>,
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct State {
    node: usize,
    cost: usize,
}

// Manually implement Ord so we get a min-heap instead of a max-heap
impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost)
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

    /// returns closes point of given long & lat
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
        return path.iter().map(|x| self.nodes[*x]).collect::<Vec<Node>>();
    }

    /// returns the edge weight from source to target
    fn get_edge_weight(&self, way: Way, travel_type: usize, use_distance: bool) -> usize {
        if use_distance {
            return way.distance;
        } else {
            if way.speed == 0 {
                return way.distance;
            }
            let speed = match travel_type {
                0 => way.speed,
                1 if way.speed <= 20 => way.speed,
                1 if way.speed >= 20 => 20,
                2 => 7,
                _ => unreachable!(),
            };
            return way.distance / speed;
        }
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

    /// executes dijkstra
    pub fn find_path(
        &self,
        start: usize,
        end: usize,
        travel_type: usize,
        use_distance: bool,
    ) -> Option<(Vec<usize>, f32)> {
        let mut dist = vec![(usize::MAX, None); self.nodes.len()];

        let mut heap = BinaryHeap::new();
        dist[start] = (0, None);
        heap.push(State {
            node: start,
            cost: 0,
        });

        while let Some(State { node, cost }) = heap.pop() {
            if node == end {
                let mut path = Vec::with_capacity(dist.len() / 2);
                let mut current_dist = dist[end];
                path.push(end);
                while let Some(prev) = current_dist.1 {
                    path.push(prev);
                    current_dist = dist[prev];
                }
                path.reverse();
                return Some((path, cost as f32 / COST_MULTIPLICATOR as f32));
            }
            if cost > dist[node].0 {
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
                let next = State {
                    node: current_way.target,
                    cost: cost + self.get_edge_weight(current_way, travel_type, use_distance),
                };
                // add way to heap
                if next.cost < dist[next.node].0 {
                    dist[next.node] = (next.cost, Some(node));
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

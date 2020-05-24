extern crate bincode;
extern crate osmpbfreader;
extern crate serde;
extern crate serde_json;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use bincode::serialize_into;
use osmpbfreader::{groups, primitive_block_from_blob};
use serde::Serialize;

use srtm::SRTM;

#[cfg(test)]
mod tests;
mod srtm;

// First three digits of coordinates are used for the grid hashing
const GRID_MULTIPLICATOR: usize = 100;

#[derive(Serialize, Debug, Clone)]
struct Way {
    source: usize,
    target: usize,
    speed: usize,
    distance: f32,
    travel_type: usize,
}

#[derive(Serialize, Debug, Clone)]
struct Node {
    latitude: f32,
    longitude: f32,
    elevation: f32,
}

#[derive(Serialize, Debug)]
struct Output {
    nodes: Vec<Node>,
    ways: Vec<Way>,
    offset: Vec<usize>,
    grid: HashMap<(usize, usize), Vec<usize>>,
}

fn parse_speed(max_speed: &str, highway: &str) -> usize {
    match max_speed.parse::<usize>() {
        Ok(ok) => return ok,
        Err(_e) => match resolve_max_speed(max_speed) {
            Ok(ok) => return ok,
            Err(_e) => {
                return aproximate_speed_limit(highway);
            }
        },
    }
}

pub fn parse_one_way(s: &str) -> (bool, bool) {
    return match s {
        "yes" => (true, false),
        "-1" => (true, true),
        _ => (false, false),
    };
}

/// resolves the int value from a dirty string that can't be resolved by default parsing
fn resolve_max_speed(s: &str) -> Result<usize, &str> {
    return match s {
        "DE:motorway" => Ok(120),
        "DE:rural" | "AT:rural" => Ok(100),
        "DE:urban" | "AT:urban" | "CZ:urban" => Ok(50),
        "maxspeed=50" => Ok(50),
        "50;" | "50b" => Ok(50),
        "DE:living_street" => Ok(30),
        "30 kph" => Ok(30),
        "zone:maxspeed=de:30" => Ok(30),
        "DE:zone:30" => Ok(30),
        "DE:zone30" => Ok(30),
        "30 mph" => Ok(30),
        "20:forward" => Ok(20),
        "10 mph" => Ok(10),
        "5 mph" => Ok(7),
        "DE:walk" | "walk" | "Schrittgeschwindigkeit" => Ok(7),
        _ => Err("none"),
    };
}

/// approximates the speed limit based on given highway type
// infos from https://wiki.openstreetmap.org/wiki/Key:highway
fn aproximate_speed_limit(s: &str) -> usize {
    return match s {
        "motorway" => 120,
        "motorway_link" => 60,
        "trunk" => 100,
        "trunk_link" => 50,
        "primary" => 60,
        "primary_link" => 50,
        "secondary" | "secondary_link" => 50,
        "tertiary" | "tertiary_link" => 50,
        "unclassified" => 40,
        "residential" => 30,
        "track" | "service" => 10,
        "living_street" => 7,
        "path" | "walk" | "footway" => 4,
        _ => 50,
    };
}

/// get what kind of street it is:
/* infos from https://wiki.openstreetmap.org/wiki/Key:highway
0 = car only
1 = car and bicycle
2 = bicycle
3 = bicycle and pedestrian
4 = pedestrian
5 = all
100 = skip
*/
fn get_street_type(s: &str, has_sidewalk: bool) -> usize {
    let mut result = match s {
        "motorway" | "motorway_link" => 0,
        "trunk" | "trunk_link" => 0,
        "raceway" | "services" | "rest_area" => 0,
        "primary" | "primary_link" => 1,
        "secondary" | "secondary_link" => 1,
        "tertiary" | "tertiary_link" => 1,
        "cycleway" => 2,
        "trail" | "track" | "path" => 3,
        "elevator" | "platform" | "corridor" => 4,
        "bus_stop" | "bridleway" | "steps" | "pedestrian" | "footway" => 4,
        "unclassified" => 5,
        "residential" | "living_street" => 5,
        "service" | "road" => 5,
        "razed" | "abandoned" | "disused" | "construction" | "proposed" => 100,
        _ => 5,
    };
    if has_sidewalk {
        result = match result {
            1 => 5,
            2 => 3,
            3 => 5,
            _ => result,
        }
    }
    return result;
}

fn main() {
    let mut ways = Vec::<Way>::new();
    let mut nodes = Vec::<Node>::new();
    let mut offset = Vec::<usize>::new();
    // stores node ids for a 2d grid e.g. (1,1) = [1,2,3,..]
    let mut grid = HashMap::<(usize, usize), Vec<usize>>::new();

    let mut amount_nodes = 0;

    // check if arguments are right
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} pbf_file", args[0]);
        return;
    }

    // read pbf file
    let filename = std::env::args_os().nth(1).unwrap();
    let path = Path::new(&filename);
    if !path.exists() {
        println!("{} not found", filename.into_string().unwrap());
        std::process::exit(1);
    }
    let r = File::open(&path).unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(r);

    // for storing mapping of own-ids and osm-ids
    let mut osm_id_mapping = HashMap::<i64, usize>::new();

    // first store all way-IDs that are having the "highway" tag. also store speed-limit
    for block in pbf.blobs().map(|b| primitive_block_from_blob(&b.unwrap())) {
        let block = block.unwrap();
        for group in block.get_primitivegroup().iter() {
            for way in groups::ways(&group, &block) {
                if way.tags.contains_key("highway") {
                    let highway = way.tags.get("highway").unwrap().trim();
                    let mut has_sidewalk: bool = false;
                    if way.tags.contains_key("sidewalk") {
                        has_sidewalk = match way.tags.get("sidewalk").unwrap().trim() {
                            "None" | "none" | "No" | "no" => false,
                            _ => true,
                        }
                    }
                    let travel_type = get_street_type(highway, has_sidewalk);
                    if travel_type == 100 {
                        continue;
                    }
                    let mut max_speed: &str = "";
                    if way.tags.contains_key("maxspeed") {
                        max_speed = way.tags.get("maxspeed").unwrap().trim();
                    }
                    let speed = parse_speed(max_speed, highway);

                    let mut one_way: &str = "";
                    if way.tags.contains_key("oneway") {
                        one_way = way.tags.get("oneway").unwrap().trim();
                    }
                    let (one_way, reverse_dir): (bool, bool) = parse_one_way(one_way);

                    // get all node IDs from ways without duplication
                    let mut prev_id: usize;
                    let osm_id = way.nodes[0].0;
                    if osm_id_mapping.contains_key(&osm_id) {
                        prev_id = *osm_id_mapping.get(&osm_id).unwrap();
                    } else {
                        osm_id_mapping.insert(osm_id, amount_nodes);
                        prev_id = amount_nodes;
                        amount_nodes += 1;
                    }
                    // iterate over nodes and add them
                    for node in way.nodes.iter().skip(1) {
                        let osm_id = node.0;
                        let id;
                        if osm_id_mapping.contains_key(&osm_id) {
                            id = *osm_id_mapping.get(&osm_id).unwrap();
                        } else {
                            osm_id_mapping.insert(osm_id, amount_nodes);
                            id = amount_nodes;
                            amount_nodes += 1;
                        }
                        if (!reverse_dir && one_way) || !one_way {
                            ways.push(Way {
                                source: prev_id,
                                target: id,
                                speed: speed,
                                distance: 0.0,
                                travel_type: travel_type,
                            });
                        }
                        if (reverse_dir && one_way) || !one_way {
                            ways.push(Way {
                                source: id,
                                target: prev_id,
                                speed: speed,
                                distance: 0.0,
                                travel_type: travel_type,
                            });
                        }
                        prev_id = id;
                    }
                }
            }
        }
    }

    // resize offset and nodes
    nodes.resize(
        amount_nodes,
        Node {
            latitude: 0.0,
            longitude: 0.0,
            elevation: 0.0,
        },
    );
    offset.resize(amount_nodes + 1, 0);

    // reset pbf reader
    match pbf.rewind() {
        Ok(_ok) => (),
        Err(_e) => panic!("rewind was not successfull"),
    }

    let mut srtm = SRTM::new();
    // store all geo-information about the nodes
    let mut latest_elevation_opt: Option<f32> = None;
    for block in pbf.blobs().map(|b| primitive_block_from_blob(&b.unwrap())) {
        let block = block.unwrap();
        for group in block.get_primitivegroup().iter() {
            for node in groups::dense_nodes(&group, &block) {
                // check if node in osm_id_mapping
                match osm_id_mapping.get(&node.id.0) {
                    Some(our_id) => {
                        let latitude = node.decimicro_lat as f32 / 10_000_000.0;
                        let longitude = node.decimicro_lon as f32 / 10_000_000.0;
                        let elevation = match srtm.get_elevation(latitude, longitude, true) {
                            Some(elevation) => {
                                latest_elevation_opt = Some(elevation);
                                elevation
                            },
                            None => {
                                match latest_elevation_opt {
                                    Some(latest_elevation) => latest_elevation,
                                    None => {
                                        panic!{"No way to set elevation of node: {}", node.id.0}
                                    }
                                }
                            }
                        };
                        nodes[*our_id] = Node {
                            // https://github.com/rust-lang/rfcs/blob/master/text/1682-field-init-shorthand.md
                            latitude,
                            longitude,
                            elevation,
                        };
                        let lat_grid = (latitude * GRID_MULTIPLICATOR as f32) as usize;
                        let lng_grid = (longitude * GRID_MULTIPLICATOR as f32) as usize;
                        match grid.get_mut(&(lat_grid, lng_grid)) {
                            Some(id_list) => {
                                id_list.push(*our_id);
                            }
                            None => {
                                let mut new_id_list = Vec::<usize>::new();
                                new_id_list.push(*our_id);
                                grid.insert((lat_grid as usize, lng_grid as usize), new_id_list);
                            }
                        }
                    }
                    None => continue,
                }
            }
        }
    }

    ways.sort_by(|a, b| a.source.cmp(&b.source));
    fill_offset(&ways, &mut offset);

    //let mut counter: usize = 0;

    for i in 0..ways.len() {
        ways[i].distance = calc_distance(
            nodes[ways[i].source].latitude,
            nodes[ways[i].source].longitude,
            nodes[ways[i].target].latitude,
            nodes[ways[i].target].longitude,
        );
        /*
        if ways[i].distance == 0 {
            counter += 1;
        }
        */
    }
    //println!("zero counter {:?}", counter);

    // serialize everything
    let result = Output {
        nodes,
        ways,
        offset,
        grid,
    };

    let output_file = format!("{}{}", filename.into_string().unwrap(), ".fmi");
    println!("everything gets written to {}", output_file);
    let mut f = BufWriter::new(File::create(output_file).unwrap());
    serialize_into(&mut f, &result).unwrap();
}

/// fill offset array
fn fill_offset(ways: &Vec<Way>, offset: &mut Vec<usize>) {
    for way in ways {
        offset[way.source + 1] += 1;
    }
    for i in 1..offset.len() {
        offset[i] += offset[i - 1];
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

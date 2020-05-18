extern crate actix_files;
extern crate actix_web;
extern crate bincode;
extern crate serde;
extern crate serde_json;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Instant;

use actix_files as fs;
use actix_web::{App, HttpServer, middleware, web};
use bincode::deserialize_from;
use serde::{Deserialize, Serialize};

use graph::Graph;

mod graph;

#[derive(Copy, Clone, Deserialize, Debug)]
pub struct Way {
    source: usize,
    target: usize,
    speed: usize,
    distance: f32,
    travel_type: usize,
}

#[derive(Copy, Clone, Deserialize, Serialize, Debug)]
pub struct Node {
    latitude: f32,
    longitude: f32,
    elevation: f32,
}

#[derive(Copy, Clone, Deserialize, Serialize, Debug)]
pub struct Position {
    latitude: f32,
    longitude: f32,
}


#[derive(Deserialize, Debug)]
struct MapData {
    nodes: Vec<Node>,
    ways: Vec<Way>,
    offset: Vec<usize>,
    grid: HashMap<(usize, usize), Vec<usize>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Query {
    start: Position,
    end: Position,
    travel_type: String,
    by_distance: bool,
    max_ele_rise: i32,
    all_paths: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct Response {
    path: Vec<Node>,
    distance: f32,
    distance_type: String,
    elevation: f32,
}

fn query(request: web::Json<Query>, dijkstra: web::Data<Graph>) -> web::Json<Vec<Response>> {
    let total_time = Instant::now();
    // extract points
    let start: &Position = &request.start;
    let end: &Position = &request.end;
    let travel_type = match request.travel_type.as_ref() {
        "car" => 0,
        "bicycle" => 1,
        "foot" => 2,
        _ => 0,
    };
    let by_distance: bool = request.by_distance;
    let max_elevation = request.max_ele_rise;
    let all_paths = request.all_paths;
    // println!("Start: {},{}", start.latitude, start.longitude);
    // println!("End: {},{}", end.latitude, end.longitude);
    // println!("travel_type: {}, by_distance: {}", travel_type, by_distance);

    // search for clicked points
    let timing_find = Instant::now();
    let start_id: usize = dijkstra.get_point_id(start.latitude, start.longitude, travel_type);
    let end_id: usize = dijkstra.get_point_id(end.latitude, end.longitude, travel_type);
    println!("### duration for get_point_id(): {:?}", timing_find.elapsed());

    let timing = Instant::now();

    let tmp = dijkstra.find_path(start_id, end_id, travel_type, by_distance, max_elevation as f32, all_paths);
    println!("### duration for find_path(): {:?}", timing.elapsed());

    let mut results = Vec::<Response>::new();
    match tmp {
        Ok(drs) => {
            for dr in drs {
                let result: Vec<Node>;
                let mut distance: f32;
                let mut distance_type: String = "".to_string();
                println!("elevation: {}", dr.ele_rise);
                println!("distance: {}", dr.distance);
                result = dijkstra.get_nodes(dr.path);
                match by_distance {
                    false => {
                        if dr.distance.trunc() >= 1.0 {
                            distance = dr.distance;
                            distance_type.push_str("h ");
                        }
                        distance = dr.distance.fract() * 60.0;
                        distance_type.push_str("min");
                    }
                    true => {
                        distance = dr.distance;
                        distance_type.push_str("km");
                    }
                };
                results.push(Response{
                    path: result,
                    distance,
                    distance_type,
                    elevation: dr.ele_rise
                })
            }
        }
        Err(e) => {
            println!("{}", e);
        }
    }

    println!("result size: {}", results.len());
    println!("### answered request in: {:?}", total_time.elapsed());
    return web::Json(results);
}

fn main() {
    // check if arguments are right
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} pbf.fmi_file", args[0]);
        return;
    }

    // check if file is right
    let filename = std::env::args_os().nth(1).unwrap();
    if !Path::new(&filename).exists() {
        println!("{} not found", filename.into_string().unwrap());
        std::process::exit(1);
    }

    // read file
    let mut f = BufReader::new(File::open(filename).unwrap());
    let input: MapData = deserialize_from(&mut f).unwrap();
    let d = Graph::new(input.nodes, input.ways, input.offset, input.grid);

    let graph = web::Data::new(d);

    // check for static-html folder
    if !Path::new("./static").exists() {
        eprintln!("./static/ directory not found");
        std::process::exit(1);
    }

    // start webserver
    println!("webserver started on http://localhost:8080");
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .data(web::JsonConfig::default().limit(1024))
            .register_data(graph.clone())
            .service(web::resource("/dijkstra").route(web::post().to(query)))
            .service(fs::Files::new("/", "./static/").index_file("index.html"))
    })
        .bind("localhost:8080")
        .unwrap()
        .run()
        .unwrap();
}

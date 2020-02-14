# OSM Bike Route Planner
by Simon Matejetz

This project was part of "Lab Course: Algorithms for OSM Data" and is forked from the previous group project with Felix Buehler.
It adds elevation data from the NASAs SRTM3 data-collection to the parsed graph and replaces the naive-dijkstra shortest-path algorithm with a constrained shortest path algorithm to find a suitable way (low upward slope as well as low distance) for bike tours (to some extend also usable for car and walking routes).

This repository consists of two programms:

## pre

this crate is for lazily downloading, accessing parsing and interpolating the SRTM3 elevation data (https://dds.cr.usgs.gov/srtm/version2_1/SRTM3/Eurasia/)
its also for parsing the `*.osm.pbf` file into a `*.osm.pbf.fmi` file, which then contains all nodes and their elevations, as well as edges between them and is the data basis for the `web`-program

### dependecies

- `osmpbfreader` = parsing the pbf file
- `serde` = serialization
- `bincode` = exporting serialization
- `reqwest` = downloading `.hgt.zip` srtm files
- `zip` = extracting downloaded `.hgt.zip` files to receive the `.hgt` file containing elevation data

### Compilation
`cargo build --release`

### Usage
`cargo run --release [PATH_TO_OSM_PBF_FILE]`

## web

is the webserver which provides the interface and executes the DUAL-RELAX-CSP (Lagrangian dual problem) algorithm for the chosen start and destination node. it needs the `.osm.pbf.fmi`-file from the `pre`-programm as an input to use as a data source.

### dependecies

- `actix-files` = serving static files
- `actix-web` = webserver
- `serde` = serialization
- `bincode` = exporting serialization
- `serde_json` = parsing json

### Compilation

`cargo build --release`

### Usage

`cargo run --release [PATH_TO_OSM_PBF_FMI_FILE]`

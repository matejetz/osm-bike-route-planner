extern crate reqwest;
extern crate zip;

use std::collections::HashMap;
use std::env::join_paths;
use std::fs::File;
use std::io::{Bytes, copy, Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::fs;

const ELEVATION_NULL_VALUE: i16 = -32768;
const SRTM_FOLDER_NAME: &str = "../strm_files";

pub struct SRTM {
    _files: HashMap<String, GeoElevationFile>,
}

struct GeoElevationFile {
    file_name: String,
    latitude: f32,
    longitude: f32,
    data: Vec<u8>,
    resolution: f64,
    square_side: i64,
}

enum LatitudeOrientation {
    North,
    South,
}

enum LongitudeOrientation {
    West,
    East,
}

impl SRTM {
    pub fn new() -> Self {
        SRTM {
            _files: HashMap::<String, GeoElevationFile>::new()
        }
    }

    pub fn get_elevation(&mut self, latitude: f32, longitude: f32, interpolate: bool) -> Option<f32> {
        let geo_file = self.get_file(latitude, longitude);
        if !interpolate {
            return match geo_file.get_elevation(latitude, longitude) {
                Ok(elevation) => {
                    if elevation == ELEVATION_NULL_VALUE { None } else { Some(elevation as f32) }
                }
                Err(error) => None
            };
        } else {
            return Some(geo_file.get_interpolated_elevation(latitude, longitude));
        }
    }

    fn get_file(&mut self, latitude: f32, longitude: f32) -> &GeoElevationFile {
        let file_name = SRTM::get_file_name(latitude, longitude);
        if !self._files.contains_key(file_name.as_str()) {
            let data = self.load_file_data(file_name.as_str());
            let geo_file = GeoElevationFile::new(file_name.clone(), latitude, longitude, data);
            self._files.insert(String::from(file_name.as_str()), geo_file);
        }
        return self._files.get(file_name.as_str()).unwrap();
    }

    fn load_file_data(&self, file_name: &str) -> Vec<u8> {
        let srtm_folder: &Path = Path::new(SRTM_FOLDER_NAME);
        if !srtm_folder.exists() {
            fs::create_dir_all(srtm_folder);
        }
        let file_path: String = SRTM::get_relative_strm_file_path(file_name);
        let path: &Path = Path::new(&file_path);
        if !path.exists() {
            println!("{} not found locally", path.display());
            SRTM::download_srtm_file(file_name);
        }
        let mut file: File = File::open(path).unwrap();
        let mut buf = Vec::<u8>::new();
        file.read_to_end(&mut buf);
        buf
    }

    fn get_relative_strm_file_path(file_name: &str) -> String {
        let path = format!("{}/{}.hgt", STRM_FOLDER_NAME, file_name);
        path
    }

    fn download_srtm_file(file_name: &str) -> Result<(), reqwest::Error> {
        // download zip file into response reader
        let url = format!("https://dds.cr.usgs.gov/srtm/version2_1/SRTM3/Eurasia/{}.hgt.zip", file_name);
        println!("downloading srtm file from: {}", url);
        let mut response = reqwest::blocking::get(url.as_str())?;

        // unzip file
        let mut unzipped_file = zip::read::read_zipfile_from_stream(&mut response).unwrap().unwrap();
        let mut file_buf: Vec<u8> = Vec::new();
        unzipped_file.read_to_end(&mut file_buf).map_err(|err|println!("{:?}", err)).err();

        // save unzipped file
        let mut file_location = SRTM::get_relative_strm_file_path(file_name);
        let mut dest = File::create(&file_location).unwrap();
        dest.write_all(&file_buf).map_err(|err| println!("{:?}", err)).err();
        println!("saved srtm file to: {}", file_location);
        Ok(())
    }

    fn get_latitude_orientation(latitude: f32) -> LatitudeOrientation {
        return if latitude >= 0.0 {
            LatitudeOrientation::North
        } else {
            LatitudeOrientation::South
        };
    }

    fn get_longitude_orientation(longitude: f32) -> LongitudeOrientation {
        return if longitude >= 0.0 {
            LongitudeOrientation::East
        } else {
            LongitudeOrientation::West
        };
    }

    fn get_file_name(latitude: f32, longitude: f32) -> String {
        let north_south = match SRTM::get_latitude_orientation(latitude) {
            LatitudeOrientation::North => "N",
            LatitudeOrientation::South => "S"
        };
        let east_west = match SRTM::get_longitude_orientation(longitude) {
            LongitudeOrientation::East => "E",
            LongitudeOrientation::West => "W"
        };
        return format!("{}{:0>2}{}{:0>3}", north_south, (latitude.floor() as i32).to_string(), east_west, (longitude.floor() as i32).to_string());
    }
}

impl GeoElevationFile {
    pub fn new(file_name: String, latitude: f32, longitude: f32, data: Vec<u8>) -> Self {
        let square_side = (data.len() as f64 / 2.0).sqrt();
        let resolution = 1.0 / (square_side - 1.0);
        let lat_filename = latitude.floor();
        let lon_filename = longitude.floor();
        let final_square_side = square_side as i64;
        let file = GeoElevationFile {
            file_name,
            latitude: lat_filename,
            longitude: lon_filename,
            data,
            resolution,
            square_side: final_square_side,
        };
        file
    }

    fn get_elevation(&self, latitude: f32, longitude: f32) -> Result<i16, String> {
        let (row, column) = self.get_row_and_column(latitude, longitude);
        return self.get_elevation_from_row_and_column(row, column);
    }

    fn get_row_and_column(&self, latitude: f32, longitude: f32) -> (i64, i64) {
        let row = ((self.latitude + 1.0 - latitude) * (self.square_side - 1) as f32).floor() as i64;
        let column = ((longitude - self.longitude) * (self.square_side - 1) as f32).floor() as i64;
        return (row, column);
    }

    /// returns the latitude and longitude represented by the position (row+column) in the file
    fn get_lat_and_lon(&self, row: i64, column: i64) -> (f32, f32) {
        return (self.latitude + 1.0 - row as f32 * self.resolution as f32, self.longitude + column as f32 * self.resolution as f32);
    }

    fn get_elevation_from_row_and_column(&self, row: i64, column: i64) -> Result<i16, String> {
        let i = row * self.square_side + column;
        if ((i * 2 + 1) as usize) > self.data.len() {
            return Err(format!("Index not in array for file {}", self.file_name));
        }
        let first_byte = self.data[(i * 2) as usize];
        let second_byte = self.data[((i * 2) + 1) as usize];
        let elevation = i16::from_be_bytes([first_byte, second_byte]);
        return Ok(elevation);
    }

    fn get_interpolated_elevation(&self, latitude: f32, longitude: f32) -> f32 {
        let mut ele_weight = self.get_elevation_weight_of_neighbors(latitude, longitude);
        println!("number of neighbors {}", ele_weight.len());
        // sum all weights in result
        let sum_weights: f32 = ele_weight.iter().map(|&e_w| e_w.1).sum();
        // return normalized sum
        return ele_weight.iter().map(|&e_w| (e_w.1 / sum_weights) * e_w.0 as f32).sum();
    }

    /// returns elevation and weighting of nearest neighbor and its neighbors interpolated by distance to actual node
    fn get_elevation_weight_of_neighbors(&self, latitude: f32, longitude: f32) -> Vec<(i16, f32)> {
        let (row, column) = self.get_row_and_column(latitude, longitude);
        let mut ele_weight = Vec::<(i16, f32)>::new();
        // nearest neighbor
        match self.get_elevation_from_row_and_column(row, column) {
            Ok(ele) => {
                let (lat_nearest, lon_nearest) = self.get_lat_and_lon(row, column);
                let weight_nearest = 1.0 / Self::distance(latitude, longitude, lat_nearest, lon_nearest);
                ele_weight.push((ele, weight_nearest));
            }
            Err(e) => ()
        }
        // nearest neighbors top neighbor
        match self.get_elevation_from_row_and_column(row, column - 1) {
            Ok(ele) => {
                let (lat_west, lon_west) = self.get_lat_and_lon(row, column - 1);
                let weight_west = 1.0 / Self::distance(latitude, longitude, lat_west, lon_west);
                ele_weight.push((ele, weight_west));
            }
            Err(e) => ()
        }
        // nearest neighbors bottom neighbor
        match self.get_elevation_from_row_and_column(row, column + 1) {
            Ok(ele) => {
                let (lat_east, lon_east) = self.get_lat_and_lon(row, column + 1);
                let weight_east = 1.0 / Self::distance(latitude, longitude, lat_east, lon_east);
                ele_weight.push((ele, weight_east));
            }
            Err(e) => ()
        }
        // nearest neighbors left neighbor
        match self.get_elevation_from_row_and_column(row - 1, column) {
            Ok(ele) => {
                let (lat_north, lon_north) = self.get_lat_and_lon(row - 1, column);
                let weight_north = 1.0 / Self::distance(latitude, longitude, lat_north, lon_north);
                ele_weight.push((ele, weight_north));
            }
            Err(e) => ()
        }
        // nearest neighbors right neighbor
        match self.get_elevation_from_row_and_column(row + 1, column) {
            Ok(ele) => {
                let (lat_south, lon_south) = self.get_lat_and_lon(row + 1, column);
                let weight_south = 1.0 / Self::distance(latitude, longitude, lat_south, lon_south);
                ele_weight.push((ele, weight_south));
            }
            Err(e) => ()
        }
        return ele_weight;
    }

    fn distance(lat1: f32, lon1: f32, lat2: f32, lon2: f32) -> f32 {
        let x = lat1 - lat2;
        let y = lon1 - lon2;
        return (x * x + y * y).sqrt();
    }
}


#[test]
fn test_get_file_name() {
    let mut name = SRTM::get_file_name(53.891374, 13.083872);
    assert_eq!(name, "N53E013");
    name = SRTM::get_file_name(46.178926, 7.639213);
    // Freiburg (SW)
    assert_eq!(name, "N47E007");
}

#[test]
fn test_srtm() {
    let mut srtm = SRTM::new();
    // netherlands, sea-level
    let sea_level_elevation = srtm.get_elevation(52.6028117, 5.2589886, false);
    let stuttgart_elevation = srtm.get_elevation(48.785631, 9.186167, false);
    let himalaya_elevation = srtm.get_elevation(30.3089602, 81.0986149, false);
    print!("{:?}", (sea_level_elevation.unwrap(), stuttgart_elevation.unwrap(), himalaya_elevation.unwrap()));
    assert!(sea_level_elevation.is_some());
    assert!(stuttgart_elevation.is_some());
    assert!(himalaya_elevation.is_some());
    assert!(sea_level_elevation.unwrap() < stuttgart_elevation.unwrap() && stuttgart_elevation.unwrap() < himalaya_elevation.unwrap());
}

#[test]
fn test_srtm_interpolate() {
    let mut srtm = SRTM::new();
    // netherlands, sea-level
    let sea_level_elevation = srtm.get_elevation(52.6028117, 5.2589886, true);
    let stuttgart_elevation = srtm.get_elevation(48.785631, 9.186167, true);
    let himalaya_elevation = srtm.get_elevation(30.3089602, 81.0986149, true);
    print!("{:?}", (sea_level_elevation.unwrap(), stuttgart_elevation.unwrap(), himalaya_elevation.unwrap()));
    assert!(sea_level_elevation.is_some());
    assert!(stuttgart_elevation.is_some());
    assert!(himalaya_elevation.is_some());
    assert!(sea_level_elevation.unwrap() < stuttgart_elevation.unwrap() && stuttgart_elevation.unwrap() < himalaya_elevation.unwrap());
}

#[test]
fn test_retrieve_srtm() {
    let name = SRTM::get_file_name(47.678926, 7.639213);
    assert_eq!(name, "N47E007");
    let result = SRTM::download_srtm_file(name.as_str()).unwrap();
}
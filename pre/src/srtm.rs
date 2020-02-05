extern crate reqwest;
extern crate tempfile;
extern crate zip;

use std::collections::HashMap;
use std::env::join_paths;
use std::fs::File;
use std::io::{Bytes, copy, Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use self::reqwest::Error;

const ELEVATION_NULL_VALUE: i16 = -32768;

pub struct SRTM {
    _files: HashMap<String, GeoElevationFile>,
}

struct GeoElevationFile {
    file_name: String,
    latitude: f32,
    longitude: f32,
    data: Vec<u8>,
    square_side: i64,
}

impl SRTM {
    pub fn new() -> Self {
        SRTM {
            _files: HashMap::<String, GeoElevationFile>::new()
        }
    }

    pub fn get_elevation(&mut self, latitude: f32, longitude: f32) -> Option<i16> {
        let geo_file = self.get_file(latitude, longitude);
        return match geo_file.get_elevation(latitude, longitude) {
            Ok(elevation) => {
                if elevation == ELEVATION_NULL_VALUE { None } else { Some(elevation) }
            }
            Err(error) => None
        };
    }

    fn get_file(&mut self, latitude: f32, longitude: f32) -> &GeoElevationFile {
        let file_name = SRTM::get_file_name(latitude, longitude);
        println!("{}", file_name);
        if !self._files.contains_key(file_name.as_str()) {
            let data = self.load_file_data(file_name.as_str());
            let geo_file = GeoElevationFile::new(file_name.clone(), latitude, longitude, data);
            self._files.insert(String::from(file_name.as_str()), geo_file);
        }
        return self._files.get(file_name.as_str()).unwrap();
    }

    fn load_file_data(&self, file_name: &str) -> Vec<u8> {
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
        let path = format!("../strm_files/{}.hgt", file_name);
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
        unzipped_file.read_to_end(&mut file_buf);

        // save unzipped file
        let mut file_location = SRTM::get_relative_strm_file_path(file_name);
        let mut dest = File::create(&file_location).unwrap();
        dest.write_all(&file_buf);
        println!("saved srtm file to: {}", file_location);
        Ok(())
    }


    fn get_file_name(latitude: f32, longitude: f32) -> String {
        let north_south;
        let east_west;
        if latitude >= 0.0 {
            north_south = "N";
        } else {
            north_south = "S";
        }
        if longitude >= 0.0 {
            east_west = "E";
        } else {
            east_west = "W";
        }

        return format!("{}{:0>2}{}{:0>3}", north_south, (latitude.floor() as i32).to_string(), east_west, (longitude.floor() as i32).to_string());
    }
}

impl GeoElevationFile {
    pub fn new(file_name: String, latitude: f32, longitude: f32, data: Vec<u8>) -> Self {
        let square_side = (data.len() as f64 / 2.0).sqrt();
        let resolution = 1.0 / (square_side - 1.0);
        let final_square_side = square_side as i64;
        let file = GeoElevationFile {
            file_name,
            latitude,
            longitude,
            data,
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

    fn get_elevation_from_row_and_column(&self, row: i64, column: i64) -> Result<i16, String> {
        let i = row * self.square_side + column;
        println!("i {}", i);
        if ((i * 2 + 1) as usize) > self.data.len() {
            return Err(format!("Index not in array for file {}", self.file_name));
        }
        let first_byte = self.data[(i * 2) as usize];
        let second_byte = self.data[(i * 2 + 1) as usize];
        let elevation = i16::from_be_bytes([first_byte, second_byte]);
        return Ok(elevation);
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
    let sea_level_elevation = srtm.get_elevation(52.6028117, 5.2589886);
    let stuttgart_elevation = srtm.get_elevation(48.7359657, 9.2466);
    let himalaya_elevation = srtm.get_elevation(30.3089602, 81.0986149);
    let lol_ele = srtm.get_elevation(48.66619, 9.251554);
    assert!(sea_level_elevation.is_some());
    assert!(stuttgart_elevation.is_some());
    assert!(himalaya_elevation.is_none());
    assert!(sea_level_elevation.unwrap() < stuttgart_elevation.unwrap());
}

#[test]
fn test_retrieve_srtm() {
    let name = SRTM::get_file_name(47.678926, 7.639213);
    assert_eq!(name, "N47E007");
    let result = SRTM::download_srtm_file(name.as_str()).unwrap();
    println!("{:?}", result);
    assert_eq!(result, ());
    assert_eq!(1, 0);
}
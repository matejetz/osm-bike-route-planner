extern crate reqwest;
extern crate zip;
extern crate tempfile;

use std::collections::HashMap;
use std::env::join_paths;
use std::fs::File;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use self::reqwest::Error;
use std::io::{Read, copy, Bytes, Write};
use self::zip::read::ZipFile;

struct SRTM {
    _files: HashMap<String, GeoElevationFile>,
}

struct GeoElevationFile {
    path: String,
    latitude: f64,
    longitude: f64,
}

impl SRTM {
    pub fn new() -> Self {
        SRTM {
            _files: HashMap::<String, GeoElevationFile>::new()
        }
    }

    fn get_elevation(&mut self, latitude: f64, longitude: f64) -> &GeoElevationFile {
        let geo_file = self.get_file(latitude, longitude);
        return geo_file;
    }

    fn get_file(&mut self, latitude: f64, longitude: f64) -> &GeoElevationFile {
        let filename = SRTM::get_file_name(latitude, longitude);
        println!("{}", filename);
        if !self._files.contains_key(filename.as_str()) {
            let file_found = self.load_file_data(filename.as_str());
            self._files.insert(String::from(filename.as_str()), file_found);
        }
        return self._files.get(filename.as_str()).unwrap();
    }

    fn load_file_data(&self, file_name: &str) -> GeoElevationFile {
        let path: String = SRTM::get_relative_strm_file_path(file_name);
        if Path::new(&path).exists() {
            SRTM::download_srtm_file(file_name);
        }
        return GeoElevationFile::new(path);
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
        let mut dest = File::create(file_location).unwrap();
        dest.write_all(&file_buf);
        println!("saved srtm file to: {}", file_location);

        Ok(())
    }


    fn get_file_name(latitude: f64, longitude: f64) -> String {
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
    pub fn new(
        path: String
    ) -> Self {
        GeoElevationFile {
            path,
            latitude: 0.00,
            longitude: 0.00,
        }
    }
}


#[test]
fn test_get_file_name() {
    let mut name = SRTM::get_file_name(53.891374, 13.083872);
    assert_eq!(name, "N53E013");
    name = SRTM::get_file_name(47.678926, 7.639213);
    // Freiburg (SW)
    assert_eq!(name, "N47E007");
}

#[test]
fn test_srtm() {
    let mut srtm = SRTM::new();
    let elevation = srtm.get_elevation(40.3, 7.2).longitude;
    println!("{}", elevation);
}

#[test]
fn test_retrieve_srtm() {
    let name = SRTM::get_file_name(47.678926, 7.639213);
    assert_eq!(name, "N47E007");
    let result = SRTM::download_srtm_file(name.as_str()).unwrap();
    println!("{:?}", result);
    assert_eq!(result,());
    assert_eq!(1, 0);
}
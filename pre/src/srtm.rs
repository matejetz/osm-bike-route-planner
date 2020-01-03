extern crate reqwest;
extern crate zip;

use std::collections::HashMap;
use std::env::join_paths;
use std::fs::File;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use self::reqwest::Error;
use std::io::{Read, copy};
use self::zip::read::ZipFile;

const SRTM_FOLDER: &str = "strm_files";
const SRTM_EXTENSION: &str = ".hgt";

struct SRTM {
    _files: HashMap<String, GeoElevationFile>,
}

struct GeoElevationFile {
    path: PathBuf,
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
        println!("hi elevation");
        return geo_file;
    }

    fn get_file(&mut self, latitude: f64, longitude: f64) -> &GeoElevationFile {
        let filename = SRTM::get_file_name(latitude, longitude);
        println!("hi file");
        println!("{}", filename);
        if !self._files.contains_key(filename.as_str()) {
            let file_found = self.retrieve_or_load_file_data(filename.as_str());
            self._files.insert(String::from(filename.as_str()), file_found);
        }
        return self._files.get(filename.as_str()).unwrap();
    }

    fn retrieve_or_load_file_data(&self, file_name: &str) -> GeoElevationFile {
        let path: PathBuf = SRTM::get_local_path(file_name);
        if !path.exists() {
            SRTM::retrieve_srtm_file(file_name);
        }
        return GeoElevationFile::new(path);
    }

    fn get_local_path(file_name: &str) -> PathBuf {
        let mut path = PathBuf::from(SRTM_FOLDER);
        path.push(file_name);
        path.set_extension(SRTM_EXTENSION);
        return path;
    }

    fn retrieve_srtm_file(file_name: &str) -> Result<(), reqwest::Error> {
        let url = format!("https://dds.cr.usgs.gov/srtm/version2_1/SRTM3/Eurasia/{}.hgt.zip", file_name);
        let mut response = reqwest::blocking::get(url.as_str())?;
        println!("{}", response.status());
        // let zipped_bytes = response.bytes();
        // let unzipped_bytes = SRTM::unzip_bytes(zipped_bytes);
        // println!("{:?}", unzipped_bytes);
        let mut file_location = String::from("../strm_files/");
        file_location.push_str(file_name);
        file_location.push_str(".hgt.zip");
        let mut dest = File::create(file_location).unwrap();


        copy(&mut response, &mut dest);

        Ok(())
    }

    fn unzip_bytes(zipped_bytes: &[u8]) -> Result<Vec<u8>, zip::result::ZipError> {
        let mut reader = std::io::Cursor::new(zipped_bytes);
        let mut zip = zip::ZipArchive::new(reader)?;
        let mut file = zip.by_index(0).unwrap();
        let mut unzipped_bytes: Vec<u8> = Vec::<u8>::new();
        for byte in file.bytes() {
            match byte {
                Ok(byte) => {unzipped_bytes.push(byte)},
                Err(err) => {},
            }
        }
        return Ok(unzipped_bytes);
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
        path: PathBuf
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
    let result = SRTM::retrieve_srtm_file(name.as_str()).unwrap();
    println!("{:?}", result);
    assert_eq!(result,());
    assert_eq!(1, 0);
}
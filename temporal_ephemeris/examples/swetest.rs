extern crate bincode;
extern crate temporal_ephemeris;
extern crate serde;

use temporal_ephemeris_sys::*;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::fs::File;
use std::io::{self, Write};

#[derive(Serialize, Deserialize)]
struct ChartData {
    date: (i32, i32, i32),
    time: (f64, f64),
    location: (f64, f64),
    planets: Vec<PlanetData>,
    ascendant: f64,
    midheaven: f64,
}

#[derive(Serialize, Deserialize)]
struct PlanetData {
    name: String,
    longitude: f64,
    latitude: f64,
    distance: f64,
    house: f64,
}

fn main() {
    unsafe {
        let null: *mut i8 = std::ptr::null_mut();
        let iflag: i32 = (SEFLG_TROPICAL | SEFLG_SIDEREAL).try_into().unwrap();
        swe_set_ephe_path(null);
        let gregorian_calendar_flag: i32 = SE_GREG_CAL.try_into().unwrap();

        // Input birth date
        let date = get_input_date(
            "Enter birth date (YYYY MM DD) or press Enter for default (1991 6 18):",
            (1991, 6, 18),
        );

        // Input birth time
        let time = get_input_time(
            "Enter birth time (HH MM) or press Enter for default (07 10):",
            (7.0, 10.0),
        );

        // Input birth location
        let location = get_input_location("Enter birth location (Latitude Longitude) or press Enter for default (40.7128 -74.0060):", (40.7128, -74.0060));

        let julian_day_ut = swe_julday(
            date.0,
            date.1,
            date.2,
            time.0 + time.1 / 60.0,
            gregorian_calendar_flag,
        );

        println!("\nCalculating Natal Chart...");

        let mut cusps: [f64; 13] = [0.0; 13];
        let mut ascmc: [f64; 10] = [0.0; 10];
        swe_houses(
            julian_day_ut,
            location.0,
            location.1,
            'P' as i32,
            cusps.as_mut_ptr(),
            ascmc.as_mut_ptr(),
        );

        let mut chart_data = ChartData {
            date,
            time,
            location,
            planets: Vec::new(),
            ascendant: ascmc[0],
            midheaven: ascmc[1],
        };

        println!("\nNatal Chart:");
        println!("Planet\tLongitude\tLatitude\tDistance\tHouse");

        for body in SE_SUN..=SE_PLUTO {
            if body == SE_EARTH {
                continue;
            }
            let body_signed: i32 = body.try_into().unwrap();
            let (planet_name, coordinates) =
                calculate_planet_position(julian_day_ut, body_signed, iflag);

            if let (Some(name), Some(coords)) = (planet_name, coordinates) {
                let longitude = coords[0];
                let latitude = coords[1];
                let distance = coords[2];
                let house = swe_house_pos(
                    julian_day_ut,
                    latitude,
                    longitude,
                    'P' as i32,
                    cusps.as_mut_ptr(),
                    std::ptr::null_mut(),
                );
                println!(
                    "{}\t{:.2}°\t{:.2}°\t{:.2}\t{:.2}",
                    name, longitude, latitude, distance, house
                );

                chart_data.planets.push(PlanetData {
                    name,
                    longitude,
                    latitude,
                    distance,
                    house,
                });
            }
        }

        // Print Ascendant and Midheaven
        println!("Ascendant\t{:.2}°", chart_data.ascendant);
        println!("Midheaven\t{:.2}°", chart_data.midheaven);

        // Save chart data
        save_chart(&chart_data);

        swe_close();
    }
}

fn get_input_date(prompt: &str, default: (i32, i32, i32)) -> (i32, i32, i32) {
    print!("{} ", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    if input.trim().is_empty() {
        println!("Using default date: {:?}", default);
        default
    } else {
        let parts: Vec<i32> = input
            .trim()
            .split_whitespace()
            .map(|s| s.parse().expect("Invalid input"))
            .collect();
        (parts[0], parts[1], parts[2])
    }
}

fn get_input_time(prompt: &str, default: (f64, f64)) -> (f64, f64) {
    print!("{} ", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    if input.trim().is_empty() {
        println!("Using default time: {:?}", default);
        default
    } else {
        let parts: Vec<f64> = input
            .trim()
            .split_whitespace()
            .map(|s| s.parse().expect("Invalid input"))
            .collect();
        (parts[0], parts[1])
    }
}

fn get_input_location(prompt: &str, default: (f64, f64)) -> (f64, f64) {
    print!("{} ", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    if input.trim().is_empty() {
        println!("Using default location: {:?}", default);
        default
    } else {
        let parts: Vec<f64> = input
            .trim()
            .split_whitespace()
            .map(|s| s.parse().expect("Invalid input"))
            .collect();
        (parts[0], parts[1])
    }
}

unsafe fn calculate_planet_position(
    julian_day_ut: f64,
    body: i32,
    iflag: i32,
) -> (Option<String>, Option<[f64; 6]>) {
    let mut coordinates: [f64; 6] = [0.0; 6];
    let mut name: [u8; 64] = [0; 64];
    let mut error_message: [u8; 256] = [0; 256];

    let return_flag = swe_calc_ut(
        julian_day_ut,
        body,
        iflag,
        coordinates.as_mut_ptr(),
        error_message.as_mut_ptr() as *mut i8,
    );

    if return_flag < 0 {
        let error_vec: Vec<u8> = error_message.to_vec();
        let error_string = String::from_utf8_unchecked(error_vec);
        eprintln!("Error: {}", error_string);
        (None, None)
    } else {
        swe_get_planet_name(body, name.as_mut_ptr() as *mut i8);
        let planet_name = String::from_utf8_unchecked(name.to_vec());
        (
            Some(planet_name.trim_matches(char::from(0)).to_string()),
            Some(coordinates),
        )
    }
}

fn save_chart(chart_data: &ChartData) {
    let file_name = format!(
        "chart_{}_{:02}_{:02}.bin",
        chart_data.date.0, chart_data.date.1, chart_data.date.2
    );
    let encoded: Vec<u8> = bincode::serialize(&chart_data).unwrap();
    let mut file = File::create(&file_name).unwrap();
    file.write_all(&encoded).unwrap();
    println!("\nChart data saved to {}", file_name);
}

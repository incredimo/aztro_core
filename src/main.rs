use aztro_core::{  generate_vedic_astrology_report, AstronomicalResult, BirthInfo,  CelestialBody, CoordinateSystem, Location};
use chrono::{TimeZone, Utc};
 

fn main() {

    // aghil mohan 18th june 1991 07:10 AM , calicut kerala india
    let birth_info = BirthInfo {
        date_time: Utc.with_ymd_and_hms(1991, 6, 18, 7, 10, 0).unwrap(),
        location: Location::kozhikode(),
    };



    match birth_info.generate_report() {
        Ok(report) => println!("{:#?}", report),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}

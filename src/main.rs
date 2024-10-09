use aztro_core::{  generate_vedic_astrology_report, AstronomicalResult, BirthInfo, CalculationFlag, CelestialBody, CoordinateSystem, SwissEph};
use chrono::{TimeZone, Utc};
 

fn main() {
    // Example usage
    let eph = SwissEph::new();
    // aghil mohan 18th june 1991 07:10 AM , calicut kerala india
    let birth_info = BirthInfo {
        date_time: Utc.ymd(1991, 6, 18).and_hms(7, 10, 0),
        latitude: 10.522,
        longitude: 76.172,
    };


    match generate_vedic_astrology_report(&eph, birth_info) {
        Ok(report) => println!("{:#?}", report),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}

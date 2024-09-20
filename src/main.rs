use aztro_core::{SwissEph, gregorian_to_julian_day, CoordinateSystem, CelestialBody, CalculationFlag, AstronomicalResult};

fn main() {
    let eph = SwissEph::new();
    let julian_day = gregorian_to_julian_day(2024, 4, 27, 15, 30, 0.0);

    // Calculate Sun position in Tropical coordinate system
    match eph.calculate(
        CoordinateSystem::Tropical,
        julian_day,
        CelestialBody::Sun,
        &[CalculationFlag::Speed],
    ) {
        Ok(AstronomicalResult::CelestialBody(info)) => {
            println!("Sun Position:");
            println!("Longitude: {}", info.longitude);
            println!("Latitude: {}", info.latitude);
            println!("Distance: {}", info.distance);
            println!("Speed Longitude: {}", info.speed_longitude);
            println!("Speed Latitude: {}", info.speed_latitude);
            println!("Speed Distance: {}", info.speed_distance);
        }
        Ok(_) => println!("Unexpected result type."),
        Err(e) => eprintln!("Error: {}", e.message),
    }

    // Calculate Houses
    let latitude = 28.6139;  // Example: New Delhi latitude
    let longitude = 77.2090; // Example: New Delhi longitude
    match eph.calculate_houses(CoordinateSystem::Tropical, julian_day, latitude, longitude) {
        Ok(houses) => {
            for house in houses {
                println!(
                    "House {:?}: {:?} at {} degrees",
                    house.house, house.sign, house.degree
                );
            }
        }
        Err(e) => eprintln!("Error calculating houses: {}", e.message),
    }
}

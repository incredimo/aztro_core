use aztro_core::*;

fn main() {
    let eph = SwissEph::new();

    let dates = [
        (2024, 1, 1),
        (1991, 1, 1),
        (1960, 1, 1),
        (1800, 1, 1),
        (1700, 1, 1),
    ];

    for (year, month, day) in dates.iter() {
        println!("Calculating positions of all planets on {}/{}/{} 00:00:00", month, day, year);
        let date = Timestamp::new(*year, *month, *day, 0, 0, 0);
        for body in [
            CelestialBody::Sun,
            CelestialBody::Moon,
            CelestialBody::Mercury,
            CelestialBody::Venus,
            CelestialBody::Mars,
            CelestialBody::Jupiter,
            CelestialBody::Saturn,
            CelestialBody::Uranus,
            CelestialBody::Neptune,
            CelestialBody::Pluto,
            CelestialBody::MeanNode,
            CelestialBody::TrueNode,
 
        ] {
            let result = eph.calculate(CoordinateSystem::Sidereal, &date, body, &[]).unwrap();
            println!("{:?}: {:?}", body, result);
        }
        println!();
    }
}
use aztro_core::{   AstronomicalResult, BirthInfo, CelestialBody, CoordinateSystem, Gender, Location, Report};
use chrono::{TimeZone, Utc};
 

fn main() {

 
    let name = "Aghil Mohan";

    let gender = Gender::Male;

    let birth_info = Location::kozhikode().born_at(1991, 6, 18, 7, 10, 0);

    let report = Report::calculate(name, birth_info, gender).unwrap();

    report.pretty_print();
}

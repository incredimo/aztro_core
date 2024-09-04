/*  cosmo_ephemeris-rs | Rust bindings for cosmo_ephemeris, the Swiss Ephemeris C library.
 *  Copyright (c) 2021 incredimo. All rights reserved.

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

extern crate cosmo_ephemeris;

use chrono::{TimeZone, Utc};
use cosmo_ephemeris::core::{Body, CalculationResult, Flag};

fn main() {
    cosmo_ephemeris::core::set_ephe_path(Option::None);
    let julian_day_ut = cosmo_ephemeris::core::julday(Utc.ymd(1991, 10, 13).and_hms(20, 0, 0));
    println!("Planet\tlon\tlat\tdist");
    let bodies = [
        Body::Sun,
        Body::Moon,
        Body::Mercury,
        Body::Venus,
        Body::Mars,
        Body::Jupiter,
        Body::Saturn,
        Body::Neptune,
        Body::Uranus,
        Body::Pluto,
    ];
    for body in bodies {
        if body == Body::Earth {
            continue;
        }
        let flag_set = [Flag::HighPrecSpeed];
        let calc_result = cosmo_ephemeris::core::calc_ut(julian_day_ut, body, &flag_set);
        match calc_result {
            Ok(calc) => match calc {
                CalculationResult::Body(body_result) => {
                    let name = cosmo_ephemeris::core::get_planet_name(body);
                    println!(
                        "{}\t{}\t{}\t{}",
                        name,
                        body_result.pos.get(0).unwrap(),
                        body_result.pos.get(1).unwrap(),
                        body_result.pos.get(2).unwrap()
                    );
                }
                _ => (),
            },
            Err(err) => eprintln!("{}", err),
        }
    }
    cosmo_ephemeris::core::close();
}

use super::*;

// Include the generated bindings for the Swiss Ephemeris C library
include!("../build/bindings.rs");

// Embed the ephemeris file into the binary
static EPHE_FILE: &[u8] = include_bytes!("../ephe/sepl_18.se1");
static INIT: Once = Once::new();
/// SwissEph provides methods to perform astronomical calculations using the Swiss Ephemeris.
pub struct SwissEph {
    _temp_file: NamedTempFile,
}

const SE_ASCMC_ARMC: usize = 0;
const SE_ASCMC_EQUASC: usize = 1;

impl SwissEph {
    /// Initializes a new instance of SwissEph, loading the ephemeris data.
    pub fn new() -> Self {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        std::io::copy(&mut Cursor::new(EPHE_FILE), &mut temp_file)
            .expect("Failed to write ephemeris data to temp file");

        INIT.call_once(|| {
            let file_path = temp_file
                .path()
                .to_str()
                .expect("Invalid ephemeris file path");
            let c_path = CString::new(file_path).expect("Failed to convert path to CString");
            unsafe {
                swe_set_ephe_path(c_path.as_ptr() as *mut c_char);
            }
            eprintln!("Ephemeris file path set to: {}", file_path);
        });

        SwissEph {
            _temp_file: temp_file,
        }
    }

    /// Calculates the house number for a given planet longitude.
    pub fn get_house(
        &self,
        julian_day: JulianDay,
        planet_longitude: f64,
        latitude: f64,
        longitude: f64,
        hsys: char,
    ) -> Result<House, CalculationError> {
        let mut cusps: [f64; 13] = [0.0; 13];
        let mut ascmc: [f64; 10] = [0.0; 10];

        let calc_result = unsafe {
            swe_houses(
                julian_day,
                latitude,
                longitude,
                hsys as i32,
                cusps.as_mut_ptr(),
                ascmc.as_mut_ptr(),
            )
        };

        if calc_result < 0 {
            return Err(CalculationError {
                code: calc_result,
                message: "Error calculating houses".to_string(),
            });
        }

        let armc = ascmc[SE_ASCMC_ARMC];
        let eps = ascmc[SE_ASCMC_EQUASC];

        let mut serr: [c_char; 256] = [0; 256];
        let house_position = unsafe {
            swe_house_pos(
                armc,
                latitude,
                eps,
                hsys as i32,
                &mut [planet_longitude, 0.0] as *mut f64,
                serr.as_mut_ptr(),
            )
        };

        if house_position < 0.0 {
            let error_message = unsafe { CStr::from_ptr(serr.as_ptr()) }
                .to_string_lossy()
                .into_owned();
            return Err(CalculationError {
                code: -1,
                message: error_message,
            });
        }

        let house_number = house_position.floor() as usize;
        let house = match house_number {
            1 => House::First,
            2 => House::Second,
            3 => House::Third,
            4 => House::Fourth,
            5 => House::Fifth,
            6 => House::Sixth,
            7 => House::Seventh,
            8 => House::Eighth,
            9 => House::Ninth,
            10 => House::Tenth,
            11 => House::Eleventh,
            12 => House::Twelfth,
            _ => House::First,
        };
        Ok(house)
    }

    /// Calculates the Navamsa position for a given longitude.
    pub fn calculate_navamsa(&self, longitude: f64) -> f64 {
        let normalized_longitude = longitude.rem_euclid(360.0);
        let sign_index = (normalized_longitude / 30.0).floor() as usize;
        let position_in_sign = normalized_longitude % 30.0;

        let navamsa_number = (position_in_sign / 3.3333333333333335).floor() as usize;
        let navamsa_sign = (sign_index * 9 + navamsa_number) % 12;
        let adjusted_longitude =
            navamsa_sign as f64 * 30.0 + (position_in_sign % 3.3333333333333335) * 9.0;

        adjusted_longitude
    }

    /// Determines Nakshatra information for a given longitude.
    pub fn calculate_nakshatra(&self, longitude: f64) -> NakshatraInfo {
        let normalized_longitude = longitude.rem_euclid(360.0);
        let nakshatra_length = 360.0 / 27.0; // 13.333... degrees per Nakshatra
        let nakshatra_index = (normalized_longitude / nakshatra_length).floor() as usize;
        let nakshatra = match nakshatra_index {
            0 => Nakshatra::Ashwini,
            1 => Nakshatra::Bharani,
            2 => Nakshatra::Krittika,
            3 => Nakshatra::Rohini,
            4 => Nakshatra::Mrigashira,
            5 => Nakshatra::Ardra,
            6 => Nakshatra::Punarvasu,
            7 => Nakshatra::Pushya,
            8 => Nakshatra::Ashlesha,
            9 => Nakshatra::Magha,
            10 => Nakshatra::PurvaPhalguni,
            11 => Nakshatra::UttaraPhalguni,
            12 => Nakshatra::Hasta,
            13 => Nakshatra::Chitra,
            14 => Nakshatra::Swati,
            15 => Nakshatra::Vishakha,
            16 => Nakshatra::Anuradha,
            17 => Nakshatra::Jyeshtha,
            18 => Nakshatra::Moola,
            19 => Nakshatra::PurvaAshadha,
            20 => Nakshatra::UttaraAshadha,
            21 => Nakshatra::Shravana,
            22 => Nakshatra::Dhanishta,
            23 => Nakshatra::Shatabhisha,
            24 => Nakshatra::PurvaBhadrapada,
            25 => Nakshatra::UttaraBhadrapada,
            26 => Nakshatra::Revati,
            _ => Nakshatra::Revati,
        };
        let pada_length = nakshatra_length / 4.0; // 3.333... degrees per Pada
        let pada = ((normalized_longitude % nakshatra_length) / pada_length).floor() as u8 + 1;
        let lord = self.get_nakshatra_lord(nakshatra);
        NakshatraInfo {
            nakshatra,
            pada,
            lord,
            degree: normalized_longitude,
        }
    }

    /// Determines the lord of a Nakshatra.
    pub fn get_nakshatra_lord(&self, nakshatra: Nakshatra) -> CelestialBody {
        let lords = [
            CelestialBody::Ketu,
            CelestialBody::Venus,
            CelestialBody::Sun,
            CelestialBody::Moon,
            CelestialBody::Mars,
            CelestialBody::Rahu,
            CelestialBody::Jupiter,
            CelestialBody::Saturn,
            CelestialBody::Mercury,
        ];
        lords[(nakshatra as usize) % 9]
    }

    /// Calculates Dasha periods (Maha, Antar, Pratyantar) using Vimshottari Dasha system.
    pub fn calculate_dasha(&self, birth_info: &BirthInfo) -> Result<DashaInfo, CalculationError> {
        let julian_day = date_to_julian_day(birth_info.date_time);
        let result = self.calculate(
            CoordinateSystem::Sidereal,
            julian_day,
            CelestialBody::Moon,
            &[],
        )?;
        let moon_longitude = match result {
            AstronomicalResult::CelestialBody(info) => info.longitude,
            _ => {
                return Err(CalculationError {
                    code: -1,
                    message: "Failed to calculate Moon position".to_string(),
                })
            }
        };

        // Calculate the Nakshatra
        let nakshatra_info = self.calculate_nakshatra(moon_longitude);
        let starting_dasha = match nakshatra_info.lord {
            CelestialBody::Sun => Dasha::Sun,
            CelestialBody::Moon => Dasha::Moon,
            CelestialBody::Mars => Dasha::Mars,
            CelestialBody::Mercury => Dasha::Mercury,
            CelestialBody::Jupiter => Dasha::Jupiter,
            CelestialBody::Venus => Dasha::Venus,
            CelestialBody::Saturn => Dasha::Saturn,
            CelestialBody::Rahu => Dasha::Rahu,
            CelestialBody::Ketu => Dasha::Ketu,
        };

        // Vimshottari Dasha years
        let dasha_years = [
            (Dasha::Ketu, 7.0),
            (Dasha::Venus, 20.0),
            (Dasha::Sun, 6.0),
            (Dasha::Moon, 10.0),
            (Dasha::Mars, 7.0),
            (Dasha::Rahu, 18.0),
            (Dasha::Jupiter, 16.0),
            (Dasha::Saturn, 19.0),
            (Dasha::Mercury, 17.0),
        ];

        // Calculate the balance of the Dasha at birth
        let position_in_nakshatra = moon_longitude % 13.333333333333334;
        let nakshatra_fraction = position_in_nakshatra / 13.333333333333334;

        let total_dasha_years = dasha_years
            .iter()
            .find(|&&(dasha, _)| dasha == starting_dasha)
            .map(|&(_, years)| years)
            .unwrap_or(0.0);

        let dasha_balance_years = total_dasha_years * (1.0 - nakshatra_fraction);

        // Generate Maha Dasha periods covering 120 years
        let mut maha_dasha_periods = Vec::new();
        let mut index = dasha_years
            .iter()
            .position(|&(dasha, _)| dasha == starting_dasha)
            .unwrap_or(0);

        let mut maha_dasha_start = birth_info.date_time;

        // First Dasha is partial, with duration dasha_balance_years
        let (current_dasha, _) = dasha_years[index];
        let maha_dasha_end = maha_dasha_start
            + chrono::Duration::seconds((dasha_balance_years * 365.25 * 86400.0) as i64);
        maha_dasha_periods.push((current_dasha, maha_dasha_start, maha_dasha_end));
        maha_dasha_start = maha_dasha_end;
        index = (index + 1) % 9;

        let mut total_years = dasha_balance_years;

        while total_years < 120.0 {
            let (current_dasha, years) = dasha_years[index];
            let maha_dasha_end =
                maha_dasha_start + chrono::Duration::seconds((years * 365.25 * 86400.0) as i64);
            maha_dasha_periods.push((current_dasha, maha_dasha_start, maha_dasha_end));
            maha_dasha_start = maha_dasha_end;

            total_years += years;
            index = (index + 1) % 9;
        }

        // Now, find the current Maha Dasha
        let now = Utc::now();
        let current_maha_dasha = maha_dasha_periods
            .iter()
            .find(|&&(_, start, end)| now >= start && now < end)
            .unwrap_or(&maha_dasha_periods[0]);

        let (maha_dasha, maha_dasha_start, maha_dasha_end) = *current_maha_dasha;
        let maha_dasha_duration_days =
            (maha_dasha_end - maha_dasha_start).num_seconds() as f64 / 86400.0;

        // Calculate Antar Dashas within the current Maha Dasha
        let mut antar_dasha_periods = Vec::new();
        let mut antar_dasha_start = maha_dasha_start;

        for &(antar_dasha, antar_dasha_years) in &dasha_years {
            let antar_dasha_duration_days = maha_dasha_duration_days * (antar_dasha_years / 120.0);
            let antar_dasha_end = antar_dasha_start
                + chrono::Duration::seconds((antar_dasha_duration_days * 86400.0) as i64);

            antar_dasha_periods.push((antar_dasha, antar_dasha_start, antar_dasha_end));

            antar_dasha_start = antar_dasha_end;
        }

        // Find the current Antar Dasha
        let current_antar_dasha = antar_dasha_periods
            .iter()
            .find(|&&(_, start, end)| now >= start && now < end)
            .unwrap_or(&antar_dasha_periods[0]);

        let (antar_dasha, antar_dasha_start, antar_dasha_end) = *current_antar_dasha;
        let antar_dasha_duration_days =
            (antar_dasha_end - antar_dasha_start).num_seconds() as f64 / 86400.0;

        // Calculate Pratyantar Dashas within the current Antar Dasha
        let mut pratyantar_dasha_periods = Vec::new();
        let mut pratyantar_dasha_start = antar_dasha_start;

        for &(pratyantar_dasha, pratyantar_dasha_years) in &dasha_years {
            let pratyantar_dasha_duration_days =
                antar_dasha_duration_days * (pratyantar_dasha_years / 120.0);
            let pratyantar_dasha_end = pratyantar_dasha_start
                + chrono::Duration::seconds((pratyantar_dasha_duration_days * 86400.0) as i64);

            pratyantar_dasha_periods.push((
                pratyantar_dasha,
                pratyantar_dasha_start,
                pratyantar_dasha_end,
            ));

            pratyantar_dasha_start = pratyantar_dasha_end;
        }

        // Find the current Pratyantar Dasha
        let current_pratyantar_dasha = pratyantar_dasha_periods
            .iter()
            .find(|&&(_, start, end)| now >= start && now < end)
            .unwrap_or(&pratyantar_dasha_periods[0]);

        let (pratyantar_dasha, pratyantar_dasha_start, pratyantar_dasha_end) =
            *current_pratyantar_dasha;

        // Return the DashaInfo
        Ok(DashaInfo {
            maha_dasha,
            maha_dasha_start,
            maha_dasha_end,
            antar_dasha,
            antar_dasha_start,
            antar_dasha_end,
            pratyantar_dasha,
            pratyantar_dasha_start,
            pratyantar_dasha_end,
        })
    }

    /// Extended Planetary States Calculation
    pub fn calculate_planetary_states(
        &self,
        chart_info: &ChartInfo,
    ) -> Result<HashMap<CelestialBody, PlanetaryState>, CalculationError> {
        let mut states = HashMap::new();

        // Exaltation and Debilitation degrees
        let exaltation_points = [
            (CelestialBody::Sun, ZodiacSign::Aries, 10.0),
            (CelestialBody::Moon, ZodiacSign::Taurus, 3.0),
            (CelestialBody::Mars, ZodiacSign::Capricorn, 28.0),
            (CelestialBody::Mercury, ZodiacSign::Virgo, 15.0),
            (CelestialBody::Jupiter, ZodiacSign::Cancer, 5.0),
            (CelestialBody::Venus, ZodiacSign::Pisces, 27.0),
            (CelestialBody::Saturn, ZodiacSign::Libra, 20.0),
            (CelestialBody::Rahu, ZodiacSign::Gemini, 20.0),
            (CelestialBody::Ketu, ZodiacSign::Sagittarius, 20.0),
        ];

        let debilitation_points = [
            (CelestialBody::Sun, ZodiacSign::Libra, 10.0),
            (CelestialBody::Moon, ZodiacSign::Scorpio, 3.0),
            (CelestialBody::Mars, ZodiacSign::Cancer, 28.0),
            (CelestialBody::Mercury, ZodiacSign::Pisces, 15.0),
            (CelestialBody::Jupiter, ZodiacSign::Capricorn, 5.0),
            (CelestialBody::Venus, ZodiacSign::Virgo, 27.0),
            (CelestialBody::Saturn, ZodiacSign::Aries, 20.0),
            (CelestialBody::Rahu, ZodiacSign::Sagittarius, 20.0),
            (CelestialBody::Ketu, ZodiacSign::Gemini, 20.0),
        ];

        let own_signs = [
            (CelestialBody::Sun, vec![ZodiacSign::Leo]),
            (CelestialBody::Moon, vec![ZodiacSign::Cancer]),
            (
                CelestialBody::Mars,
                vec![ZodiacSign::Aries, ZodiacSign::Scorpio],
            ),
            (
                CelestialBody::Mercury,
                vec![ZodiacSign::Gemini, ZodiacSign::Virgo],
            ),
            (
                CelestialBody::Jupiter,
                vec![ZodiacSign::Sagittarius, ZodiacSign::Pisces],
            ),
            (
                CelestialBody::Venus,
                vec![ZodiacSign::Taurus, ZodiacSign::Libra],
            ),
            (
                CelestialBody::Saturn,
                vec![ZodiacSign::Capricorn, ZodiacSign::Aquarius],
            ),
            (
                CelestialBody::Rahu,
                vec![ZodiacSign::Gemini, ZodiacSign::Virgo],
            ),
            (
                CelestialBody::Ketu,
                vec![ZodiacSign::Sagittarius, ZodiacSign::Pisces],
            ),
        ];

        for planet_position in &chart_info.planets {
            let planet = planet_position.planet;
            let sign = planet_position.sign;
            let longitude = planet_position.longitude % 30.0;

            // Determine exaltation
            let exalted = exaltation_points
                .iter()
                .find(|&&(p, s, _)| p == planet && s == sign)
                .map(|&(_, _, deg)| {
                    if (longitude - deg).abs() < 1.0 {
                        PlanetaryState::DeepExaltation
                    } else {
                        PlanetaryState::Exalted
                    }
                });

            // Determine debilitation
            let debilitated = debilitation_points
                .iter()
                .find(|&&(p, s, _)| p == planet && s == sign)
                .map(|&(_, _, deg)| {
                    if (longitude - deg).abs() < 1.0 {
                        PlanetaryState::DeepDebilitation
                    } else {
                        PlanetaryState::Debilitated
                    }
                });

            // Determine own sign
            let own_sign = own_signs
                .iter()
                .find(|&&(p, ref signs)| p == planet && signs.contains(&sign))
                .map(|_| PlanetaryState::OwnSign);

            // Determine friendly or malefic
            let friendly = match planet {
                CelestialBody::Jupiter
                | CelestialBody::Venus
                | CelestialBody::Mercury
                | CelestialBody::Moon
                | CelestialBody::Sun => true,
                CelestialBody::Saturn
                | CelestialBody::Mars
                | CelestialBody::Rahu
                | CelestialBody::Ketu => false,
            };

            // Determine state
            let state = if let Some(ex_state) = exalted {
                ex_state
            } else if let Some(deb_state) = debilitated {
                deb_state
            } else if let Some(own_state) = own_sign {
                own_state
            } else {
                if friendly {
                    PlanetaryState::Benefic
                } else {
                    PlanetaryState::Malefic
                }
            };

            // Check for retrograde motion
            let final_state = if planet_position.retrograde {
                PlanetaryState::Retrograde
            } else {
                state
            };

            states.insert(planet, final_state);
        }

        Ok(states)
    }

    /// Calculates the position of a celestial body or ecliptic obliquity.
    pub fn calculate(
        &self,
        coord_system: CoordinateSystem,
        julian_day: JulianDay,
        body: CelestialBody,
        flags: &[CalculationFlag],
    ) -> Result<AstronomicalResult, CalculationError> {
        // Set sidereal mode if needed
        match coord_system {
            CoordinateSystem::Sidereal => unsafe {
                swe_set_sid_mode(SE_SIDM_LAHIRI as i32, 0.0, 0.0);
            },
            CoordinateSystem::Tropical => unsafe {
                swe_set_sid_mode(SE_SIDM_FAGAN_BRADLEY as i32, 0.0, 0.0);
            },
        }

        // Combine flags
        let mut iflag: i32 = if coord_system == CoordinateSystem::Sidereal {
            SEFLG_SIDEREAL as i32
        } else {
            0
        };
        for flag in flags {
            iflag |= *flag as i32;
        }

        // Perform calculation based on the celestial body
        let result = match body {
            CelestialBody::Ketu => {
                // Calculate Rahu and subtract 180 degrees
                let rahu_result =
                    self.calculate(coord_system, julian_day, CelestialBody::Rahu, flags)?;
                let (
                    longitude,
                    latitude,
                    distance,
                    speed_longitude,
                    speed_latitude,
                    speed_distance,
                ) = match rahu_result {
                    AstronomicalResult::CelestialBody(info) => (
                        (info.longitude + 180.0) % 360.0,
                        -info.latitude,
                        info.distance,
                        info.speed_longitude,
                        info.speed_latitude,
                        info.speed_distance,
                    ),
                    _ => {
                        return Err(CalculationError {
                            code: -1,
                            message: "Failed to calculate Ketu".to_string(),
                        })
                    }
                };
                Ok(AstronomicalResult::CelestialBody(CelestialBodyInfo {
                    longitude,
                    latitude,
                    distance,
                    speed_longitude,
                    speed_latitude,
                    speed_distance,
                }))
            }
            _ => {
                let mut results: [f64; 6] = [0.0; 6];
                let mut error: [c_char; 256] = [0; 256];
                let calc_result = unsafe {
                    swe_calc_ut(
                        julian_day,
                        body as i32,
                        iflag,
                        results.as_mut_ptr(),
                        error.as_mut_ptr(),
                    )
                };
                if calc_result < 0 {
                    let error_message = unsafe { CStr::from_ptr(error.as_ptr()) }
                        .to_string_lossy()
                        .into_owned();
                    return Err(CalculationError {
                        code: calc_result,
                        message: error_message,
                    });
                }
                Ok(AstronomicalResult::CelestialBody(CelestialBodyInfo {
                    longitude: results[0],
                    latitude: results[1],
                    distance: results[2],
                    speed_longitude: results[3],
                    speed_latitude: results[4],
                    speed_distance: results[5],
                }))
            }
        };

        result
    }

    /// Retrieves the name of a celestial body.
    pub fn get_body_name(&self, body: CelestialBody) -> String {
        match body {
            CelestialBody::Ketu => "Ketu".to_string(),
            _ => {
                let mut name: [c_char; 256] = [0; 256];
                unsafe {
                    swe_get_planet_name(body as i32, name.as_mut_ptr());
                }
                unsafe { CStr::from_ptr(name.as_ptr()) }
                    .to_string_lossy()
                    .into_owned()
            }
        }
    }

    /// Calculates the positions of the astrological houses.
    pub fn calculate_houses(
        &self,
        coord_system: CoordinateSystem,
        julian_day: JulianDay,
        latitude: f64,
        longitude: f64,
    ) -> Result<Vec<HousePosition>, CalculationError> {
        // Set sidereal mode if needed
        if coord_system == CoordinateSystem::Sidereal {
            unsafe {
                swe_set_sid_mode(SE_SIDM_LAHIRI as i32, 0.0, 0.0);
            }
        }

        let mut cusps: [f64; 13] = [0.0; 13];
        let mut ascmc: [f64; 10] = [0.0; 10];
        let flag = if coord_system == CoordinateSystem::Sidereal {
            SEFLG_SIDEREAL as i32
        } else {
            0
        };

        let calc_result = unsafe {
            swe_houses_ex(
                julian_day,
                flag,
                latitude,
                longitude,
                'P' as i32, // Placidus house system
                cusps.as_mut_ptr(),
                ascmc.as_mut_ptr(),
            )
        };

        if calc_result < 0 {
            return Err(CalculationError {
                code: calc_result,
                message: "Error calculating houses".to_string(),
            });
        }

        let house_positions: Vec<HousePosition> = (1..=12)
            .map(|i| HousePosition {
                house: match i {
                    1 => House::First,
                    2 => House::Second,
                    3 => House::Third,
                    4 => House::Fourth,
                    5 => House::Fifth,
                    6 => House::Sixth,
                    7 => House::Seventh,
                    8 => House::Eighth,
                    9 => House::Ninth,
                    10 => House::Tenth,
                    11 => House::Eleventh,
                    12 => House::Twelfth,
                    _ => unreachable!(),
                },
                sign: Self::get_zodiac_sign(cusps[i]),
                degree: cusps[i] % 30.0,
            })
            .collect();

        Ok(house_positions)
    }

    /// Determines the ascendant based on the birth info.
    pub fn calculate_ascendant(
        &self,
        coord_system: CoordinateSystem,
        julian_day: JulianDay,
        latitude: f64,
        longitude: f64,
    ) -> Result<HousePosition, CalculationError> {
        let mut cusps: [f64; 13] = [0.0; 13];
        let mut ascmc: [f64; 10] = [0.0; 10];
        let flag = if coord_system == CoordinateSystem::Sidereal {
            SEFLG_SIDEREAL as i32
        } else {
            0
        };

        let calc_result = unsafe {
            swe_houses_ex(
                julian_day,
                flag,
                latitude,
                longitude,
                'P' as i32, // Placidus house system
                cusps.as_mut_ptr(),
                ascmc.as_mut_ptr(),
            )
        };

        if calc_result < 0 {
            return Err(CalculationError {
                code: calc_result,
                message: "Error calculating ascendant".to_string(),
            });
        }

        let ascendant_degree = ascmc[0]; // Ascendant is at index 0
        let sign = Self::get_zodiac_sign(ascendant_degree);
        Ok(HousePosition {
            house: House::First,
            sign,
            degree: ascendant_degree % 30.0,
        })
    }

    /// Converts a longitude to its corresponding zodiac sign.
    fn get_zodiac_sign(longitude: f64) -> ZodiacSign {
        let normalized_longitude = longitude.rem_euclid(360.0);
        match (normalized_longitude / 30.0).floor() as usize {
            0 => ZodiacSign::Aries,
            1 => ZodiacSign::Taurus,
            2 => ZodiacSign::Gemini,
            3 => ZodiacSign::Cancer,
            4 => ZodiacSign::Leo,
            5 => ZodiacSign::Virgo,
            6 => ZodiacSign::Libra,
            7 => ZodiacSign::Scorpio,
            8 => ZodiacSign::Sagittarius,
            9 => ZodiacSign::Capricorn,
            10 => ZodiacSign::Aquarius,
            11 => ZodiacSign::Pisces,
            _ => ZodiacSign::Pisces,
        }
    }

    /// Computes planet positions (longitude, latitude, speed).
    pub fn calculate_planet_positions(
        &self,
        coord_system: CoordinateSystem,
        julian_day: JulianDay,
        chart_type: ChartType,
        birth_info: &BirthInfo,
    ) -> Result<Vec<PlanetPosition>, CalculationError> {
        let planets = vec![
            CelestialBody::Sun,
            CelestialBody::Moon,
            CelestialBody::Mars,
            CelestialBody::Mercury,
            CelestialBody::Jupiter,
            CelestialBody::Venus,
            CelestialBody::Saturn,
            CelestialBody::Rahu,
            CelestialBody::Ketu,
        ];

        let mut positions = Vec::new();

        for planet in planets {
            let result =
                self.calculate(coord_system, julian_day, planet, &[CalculationFlag::Speed])?;
            let (longitude, latitude, speed) = match result {
                AstronomicalResult::CelestialBody(info) => {
                    (info.longitude, info.latitude, info.speed_longitude)
                }
                _ => continue,
            };

            let adjusted_longitude = match chart_type {
                ChartType::Rasi => longitude,
                ChartType::Navamsa => self.calculate_navamsa(longitude),
                _ => longitude, // For simplicity, other divisional charts are not implemented here
            };

            let sign = Self::get_zodiac_sign(adjusted_longitude);
            let house = self.get_house(
                julian_day,
                adjusted_longitude,
                birth_info.latitude,
                birth_info.longitude,
                'P',
            )?;

            let nakshatra = self.calculate_nakshatra(adjusted_longitude);

            let retrograde = speed < 0.0;

            positions.push(PlanetPosition {
                planet,
                longitude: adjusted_longitude,
                latitude,
                speed,
                sign,
                house,
                nakshatra,
                retrograde,
            });
        }

        Ok(positions)
    }

    /// Calculates special lagnas (Bhava, Hora, etc.).
    pub fn calculate_special_lagnas(&self, birth_info: &BirthInfo) -> HashMap<SpecialLagna, f64> {
        let mut lagnas = HashMap::new();
        let julian_day = date_to_julian_day(birth_info.date_time);

        // Calculate Ascendant (Lagna)
        let ascendant_house: HousePosition = self
            .calculate_ascendant(
                CoordinateSystem::Sidereal,
                julian_day,
                birth_info.latitude,
                birth_info.longitude,
            )
            .unwrap();

        let ascendant = ascendant_house.degree;

        // Calculate Sun's longitude
        let sun_info = self
            .calculate(
                CoordinateSystem::Sidereal,
                julian_day,
                CelestialBody::Sun,
                &[CalculationFlag::Speed],
            )
            .unwrap();
        let sun_longitude = match sun_info {
            AstronomicalResult::CelestialBody(info) => info.longitude,
            _ => 0.0, // Default value if calculation fails
        };

        // Calculate Moon's longitude
        let moon_info = self
            .calculate(
                CoordinateSystem::Sidereal,
                julian_day,
                CelestialBody::Moon,
                &[CalculationFlag::Speed],
            )
            .unwrap();
        let moon_longitude = match moon_info {
            AstronomicalResult::CelestialBody(info) => info.longitude,
            _ => 0.0, // Default value if calculation fails
        };

        // Bhava Lagna (same as Ascendant)
        lagnas.insert(SpecialLagna::Bhava, ascendant);

        // Hora Lagna
        let hora_lagna = (ascendant + (sun_longitude - moon_longitude)).rem_euclid(360.0);
        lagnas.insert(SpecialLagna::Hora, hora_lagna);

        // Ghati Lagna
        let ghati_lagna =
            (ascendant + 15.0 * (sun_longitude - ascendant) / 360.0).rem_euclid(360.0);
        lagnas.insert(SpecialLagna::Ghati, ghati_lagna);

        // Vighati Lagna
        let vighati_lagna =
            (ascendant + 4.0 * (sun_longitude - ascendant) / 360.0).rem_euclid(360.0);
        lagnas.insert(SpecialLagna::Vighati, vighati_lagna);

        // Pravesa Lagna
        let pravesa_lagna =
            (ascendant + 3.0 * (sun_longitude - ascendant) / 360.0).rem_euclid(360.0);
        lagnas.insert(SpecialLagna::Pravesa, pravesa_lagna);

        // Sree Lagna
        let sree_lagna = (moon_longitude + (ascendant - sun_longitude)).rem_euclid(360.0);
        lagnas.insert(SpecialLagna::Sree, sree_lagna);

        // Indu Lagna
        let indu_lagna = (moon_longitude + (moon_longitude - sun_longitude)).rem_euclid(360.0);
        lagnas.insert(SpecialLagna::Indu, indu_lagna);

        // Nirayana Lagna
        let nirayana_lagna = (ascendant + (moon_longitude - sun_longitude)).rem_euclid(360.0);
        lagnas.insert(SpecialLagna::Nirayana, nirayana_lagna);

        // Tri Lagna
        let tri_lagna = (3.0 * ascendant) % 360.0;
        lagnas.insert(SpecialLagna::Tri, tri_lagna);

        // Kala Lagna
        let kala_lagna = (sun_longitude + (moon_longitude - ascendant)).rem_euclid(360.0);
        lagnas.insert(SpecialLagna::Kala, kala_lagna);

        // Varnada Lagna
        let varnada_lagna = (ascendant + 2.0 * (sun_longitude - moon_longitude)).rem_euclid(360.0);
        lagnas.insert(SpecialLagna::Varnada, varnada_lagna);

        // Pada Lagna
        let pada_lagna = (ascendant + (sun_longitude - moon_longitude) / 3.0).rem_euclid(360.0);
        lagnas.insert(SpecialLagna::Pada, pada_lagna);

        // Arka Lagna
        let arka_lagna = (sun_longitude + (ascendant - moon_longitude)).rem_euclid(360.0);
        lagnas.insert(SpecialLagna::Arka, arka_lagna);

        // Sudasa Lagna
        let sudasa_lagna = (10.0 * ascendant) % 360.0;
        lagnas.insert(SpecialLagna::Sudasa, sudasa_lagna);

        // Sudarsa Lagna
        let sudarsa_lagna = (11.0 * ascendant) % 360.0;
        lagnas.insert(SpecialLagna::Sudarsa, sudarsa_lagna);

        // Yogardha Lagna
        let yogardha_lagna = ((ascendant + moon_longitude) / 2.0).rem_euclid(360.0);
        lagnas.insert(SpecialLagna::Yogardha, yogardha_lagna);

        lagnas
    }

    /// Calculates upagrahas (shadow planets) like Dhuma, Vyatipata, Parivesha, Indrachaapa, and Upaketu.
    pub fn calculate_upagrahas(&self, birth_info: &BirthInfo) -> HashMap<Upagraha, f64> {
        let mut upagrahas = HashMap::new();
        let julian_day = date_to_julian_day(birth_info.date_time);

        upagrahas
    }

    /// Calculates sensitive points (Gulika, Mandi).
    pub fn calculate_sensitive_points(
        &self,
        birth_info: &BirthInfo,
    ) -> HashMap<SensitivePoint, f64> {
        let mut points = HashMap::new();
        let julian_day = date_to_julian_day(birth_info.date_time);

        // Placeholder calculations; actual calculations require detailed steps
        let gulika = (julian_day + 180.0) % 360.0;
        points.insert(SensitivePoint::Gulika, gulika);

        points
    }

    /// Computes multiple divisional charts (D1 to D60).
    pub fn calculate_divisional_charts(
        &self,
        birth_info: &BirthInfo,
    ) -> HashMap<DivisionalChart, ChartInfo> {
        let mut charts = HashMap::new();

        // Example: Calculating Navamsa (D9) chart
        if let Ok(navamsa_chart) = calculate_chart(self, birth_info, ChartType::Navamsa) {
            charts.insert(DivisionalChart::D9, navamsa_chart);
        }

        // Additional divisional charts (D2 to D60) can be calculated similarly

        charts
    }

    /// Adjusts longitude values to fit the 0-360 degree range.
    pub fn normalize_longitude(longitude: f64) -> f64 {
        longitude.rem_euclid(360.0)
    }



    use super::*;
use std::f64::consts::PI;

impl SwissEph {
    // ... [Previous methods remain unchanged] ...

    /// Calculate Yogas (planetary combinations)
    pub fn calculate_yogas(&self, chart: &ChartInfo) -> Vec<YogaInfo> {
        let mut yogas = Vec::new();

        // Helper function to get planet by celestial body
        let get_planet = |body: CelestialBody| -> Option<&PlanetPosition> {
            chart.planets.iter().find(|p| p.planet == body)
        };

        // Raj Yoga: Lord of 9th and 10th house conjunction
        if let (Some(ninth_lord), Some(tenth_lord)) = (get_planet(CelestialBody::Jupiter), get_planet(CelestialBody::Saturn)) {
            if (ninth_lord.longitude - tenth_lord.longitude).abs() < 10.0 {
                yogas.push(YogaInfo {
                    yoga: Yoga::Raj,
                    strength: 1.0,
                    involved_planets: vec![CelestialBody::Jupiter, CelestialBody::Saturn],
                });
            }
        }

        // Dhana Yoga: Jupiter in 2nd, 5th, 9th, or 11th house
        if let Some(jupiter) = get_planet(CelestialBody::Jupiter) {
            if matches!(jupiter.house, House::Second | House::Fifth | House::Ninth | House::Eleventh) {
                yogas.push(YogaInfo {
                    yoga: Yoga::Gajakesari,
                    strength: 0.8,
                    involved_planets: vec![CelestialBody::Jupiter],
                });
            }
        }

        // Budhaditya Yoga: Sun and Mercury in same house
        if let (Some(sun), Some(mercury)) = (get_planet(CelestialBody::Sun), get_planet(CelestialBody::Mercury)) {
            if sun.house == mercury.house {
                yogas.push(YogaInfo {
                    yoga: Yoga::Budhaditya,
                    strength: 0.9,
                    involved_planets: vec![CelestialBody::Sun, CelestialBody::Mercury],
                });
            }
        }

        // Hamsa Yoga: Jupiter in Kendra from Moon
        if let (Some(jupiter), Some(moon)) = (get_planet(CelestialBody::Jupiter), get_planet(CelestialBody::Moon)) {
            let house_diff = (jupiter.house as i32 - moon.house as i32).abs() % 12;
            if house_diff == 1 || house_diff == 4 || house_diff == 7 || house_diff == 10 {
                yogas.push(YogaInfo {
                    yoga: Yoga::Hamsa,
                    strength: 0.85,
                    involved_planets: vec![CelestialBody::Jupiter, CelestialBody::Moon],
                });
            }
        }

        // Malavya Yoga: Venus in a Kendra house
        if let Some(venus) = get_planet(CelestialBody::Venus) {
            if matches!(venus.house, House::First | House::Fourth | House::Seventh | House::Tenth) {
                yogas.push(YogaInfo {
                    yoga: Yoga::Malavya,
                    strength: 0.75,
                    involved_planets: vec![CelestialBody::Venus],
                });
            }
        }

        yogas
    }

    /// Calculate aspects between planets
    pub fn calculate_aspects(&self, chart: &ChartInfo) -> Vec<AspectInfo> {
        let mut aspects = Vec::new();

        for (i, planet1) in chart.planets.iter().enumerate() {
            for planet2 in chart.planets.iter().skip(i + 1) {
                let angle = (planet2.longitude - planet1.longitude).abs() % 360.0;
                let aspect = match angle {
                    a if (0.0..=10.0).contains(&a) || (350.0..=360.0).contains(&a) => Some(Aspect::Conjunction),
                    a if (170.0..=190.0).contains(&a) => Some(Aspect::Opposition),
                    a if (115.0..=125.0).contains(&a) || (235.0..=245.0).contains(&a) => Some(Aspect::Trine),
                    a if (85.0..=95.0).contains(&a) || (265.0..=275.0).contains(&a) => Some(Aspect::Square),
                    a if (55.0..=65.0).contains(&a) || (295.0..=305.0).contains(&a) => Some(Aspect::Sextile),
                    _ => None,
                };

                if let Some(asp) = aspect {
                    aspects.push(AspectInfo {
                        aspect: asp,
                        planet1: planet1.planet,
                        planet2: planet2.planet,
                        orb: angle % 30.0,
                    });
                }
            }
        }

        aspects
    }

    /// Calculate planetary strengths (Shadbala and Ashtakavarga)
    pub fn calculate_strengths(&self, chart: &ChartInfo) -> HashMap<CelestialBody, StrengthInfo> {
        let mut strengths = HashMap::new();

        for planet in &chart.planets {
            let shad_bala = self.calculate_shadbala(planet, chart);
            let ashtaka_varga = self.calculate_ashtakavarga(planet, chart);

            strengths.insert(planet.planet, StrengthInfo {
                shad_bala,
                ashtaka_varga,
            });
        }

        strengths
    }

    /// Helper function to calculate Shadbala
    fn calculate_shadbala(&self, planet: &PlanetPosition, chart: &ChartInfo) -> f64 {
        let mut strength = 0.0;

        // Sthana Bala (Positional Strength)
        strength += match planet.house {
            House::First | House::Fourth | House::Seventh | House::Tenth => 60.0,
            House::Second | House::Fifth | House::Eighth | House::Eleventh => 30.0,
            _ => 15.0,
        };

        // Dig Bala (Directional Strength)
        strength += match (planet.planet, planet.house) {
            (CelestialBody::Sun, House::Tenth) | (CelestialBody::Mars, House::Tenth) => 60.0,
            (CelestialBody::Jupiter, House::First) | (CelestialBody::Mercury, House::First) => 60.0,
            (CelestialBody::Moon, House::Fourth) | (CelestialBody::Venus, House::Fourth) => 60.0,
            (CelestialBody::Saturn, House::Seventh) => 60.0,
            _ => 30.0,
        };

        // Kala Bala (Temporal Strength)
        if planet.retrograde {
            strength += 60.0;
        } else {
            strength += 30.0;
        }

        // Chesta Bala (Motional Strength)
        strength += if planet.speed > 1.0 { 60.0 } else { 30.0 };

        // Naisargika Bala (Natural Strength)
        strength += match planet.planet {
            CelestialBody::Saturn => 60.0,
            CelestialBody::Jupiter => 50.0,
            CelestialBody::Mars => 40.0,
            CelestialBody::Sun => 30.0,
            CelestialBody::Venus => 20.0,
            CelestialBody::Mercury => 10.0,
            CelestialBody::Moon => 0.0,
            _ => 0.0,
        };

        // Drik Bala (Aspectual Strength)
        strength += self.calculate_aspects(chart)
            .iter()
            .filter(|a| a.planet1 == planet.planet || a.planet2 == planet.planet)
            .count() as f64 * 10.0;

        strength
    }

    /// Helper function to calculate Ashtakavarga
    fn calculate_ashtakavarga(&self, planet: &PlanetPosition, chart: &ChartInfo) -> u32 {
        let mut points = 0;

        for house in 1..=12 {
            for other_planet in &chart.planets {
                if other_planet.planet == planet.planet {
                    continue;
                }

                let angle = ((other_planet.house as i32 - house as i32 + 12) % 12) as u32;
                
                points += match (planet.planet, other_planet.planet, angle) {
                    (CelestialBody::Sun, CelestialBody::Sun, 1 | 2 | 4 | 7 | 8 | 9 | 10 | 11) => 1,
                    (CelestialBody::Sun, CelestialBody::Moon, 3 | 6 | 10 | 11) => 1,
                    (CelestialBody::Sun, CelestialBody::Mars, 1 | 2 | 4 | 7 | 8 | 9 | 10 | 11) => 1,
                    (CelestialBody::Sun, CelestialBody::Mercury, 3 | 5 | 6 | 9 | 10 | 11 | 12) => 1,
                    (CelestialBody::Sun, CelestialBody::Jupiter, 5 | 6 | 9 | 11) => 1,
                    (CelestialBody::Sun, CelestialBody::Venus, 6 | 7 | 12) => 1,
                    (CelestialBody::Sun, CelestialBody::Saturn, 1 | 2 | 4 | 7 | 8 | 10 | 11) => 1,
                    // Add similar rules for other planets...
                    _ => 0,
                };
            }
        }

        points
    }

    /// Calculate planetary dignities
    pub fn calculate_dignities(&self, chart: &ChartInfo) -> HashMap<CelestialBody, DignityInfo> {
        let mut dignities = HashMap::new();

        for planet in &chart.planets {
            let dignity = DignityInfo {
                moolatrikona: self.is_moolatrikona(planet),
                own_sign: self.is_own_sign(planet),
                exalted: self.is_exalted(planet),
                debilitated: self.is_debilitated(planet),
            };
            dignities.insert(planet.planet, dignity);
        }

        dignities
    }

    /// Helper function to check if a planet is in its Moolatrikona sign
    fn is_moolatrikona(&self, planet: &PlanetPosition) -> bool {
        match (planet.planet, planet.sign) {
            (CelestialBody::Sun, ZodiacSign::Leo) => true,
            (CelestialBody::Moon, ZodiacSign::Taurus) => true,
            (CelestialBody::Mars, ZodiacSign::Aries) => true,
            (CelestialBody::Mercury, ZodiacSign::Virgo) => true,
            (CelestialBody::Jupiter, ZodiacSign::Sagittarius) => true,
            (CelestialBody::Venus, ZodiacSign::Libra) => true,
            (CelestialBody::Saturn, ZodiacSign::Aquarius) => true,
            _ => false,
        }
    }

    /// Helper function to check if a planet is in its own sign
    fn is_own_sign(&self, planet: &PlanetPosition) -> bool {
        match (planet.planet, planet.sign) {
            (CelestialBody::Sun, ZodiacSign::Leo) => true,
            (CelestialBody::Moon, ZodiacSign::Cancer) => true,
            (CelestialBody::Mars, ZodiacSign::Aries) | (CelestialBody::Mars, ZodiacSign::Scorpio) => true,
            (CelestialBody::Mercury, ZodiacSign::Gemini) | (CelestialBody::Mercury, ZodiacSign::Virgo) => true,
            (CelestialBody::Jupiter, ZodiacSign::Sagittarius) | (CelestialBody::Jupiter, ZodiacSign::Pisces) => true,
            (CelestialBody::Venus, ZodiacSign::Taurus) | (CelestialBody::Venus, ZodiacSign::Libra) => true,
            (CelestialBody::Saturn, ZodiacSign::Capricorn) | (CelestialBody::Saturn, ZodiacSign::Aquarius) => true,
            _ => false,
        }
    }

    /// Helper function to check if a planet is exalted
    fn is_exalted(&self, planet: &PlanetPosition) -> bool {
        match (planet.planet, planet.sign) {
            (CelestialBody::Sun, ZodiacSign::Aries) => true,
            (CelestialBody::Moon, ZodiacSign::Taurus) => true,
            (CelestialBody::Mars, ZodiacSign::Capricorn) => true,
            (CelestialBody::Mercury, ZodiacSign::Virgo) => true,
            (CelestialBody::Jupiter, ZodiacSign::Cancer) => true,
            (CelestialBody::Venus, ZodiacSign::Pisces) => true,
            (CelestialBody::Saturn, ZodiacSign::Libra) => true,
            _ => false,
        }
    }

    /// Helper function to check if a planet is debilitated
    fn is_debilitated(&self, planet: &PlanetPosition) -> bool {
        match (planet.planet, planet.sign) {
            (CelestialBody::Sun, ZodiacSign::Libra) => true,
            (CelestialBody::Moon, ZodiacSign::Scorpio) => true,
            (CelestialBody::Mars, ZodiacSign::Cancer) => true,
            (CelestialBody::Mercury, ZodiacSign::Pisces) => true,
            (CelestialBody::Jupiter, ZodiacSign::Capricorn) => true,
            (CelestialBody::Venus, ZodiacSign::Virgo) => true,
            (CelestialBody::Saturn, ZodiacSign::Aries) => true,
            _ => false,
        }
    }

    /// Calculate Bhava (house) information
    pub fn calculate_bhavas(&self, chart: &ChartInfo) -> Vec<BhavaInfo> {
        let mut bhavas = Vec::new();

        for (i, house) in chart.houses.iter().enumerate() {
            let bhava = House::from_index(i).unwrap();
            let lord = self.get_house_lord(house.sign);
            let planets = chart.planets.iter()
                .filter(|p| p.house == bhava)
                .map(|p| p.planet)
                .collect();

                bhavas.push(BhavaInfo {
                    bhava,
                    sign: house.sign,
                    degree: house.degree,
                    lord,
                    planets,
                });
            }
    
            bhavas
        }
    
        /// Helper function to get the lord of a zodiac sign
        fn get_house_lord(&self, sign: ZodiacSign) -> CelestialBody {
            match sign {
                ZodiacSign::Aries | ZodiacSign::Scorpio => CelestialBody::Mars,
                ZodiacSign::Taurus | ZodiacSign::Libra => CelestialBody::Venus,
                ZodiacSign::Gemini | ZodiacSign::Virgo => CelestialBody::Mercury,
                ZodiacSign::Cancer => CelestialBody::Moon,
                ZodiacSign::Leo => CelestialBody::Sun,
                ZodiacSign::Sagittarius | ZodiacSign::Pisces => CelestialBody::Jupiter,
                ZodiacSign::Capricorn | ZodiacSign::Aquarius => CelestialBody::Saturn,
            }
        }
    
        /// Calculate planetary transits
        pub fn calculate_transits(&self, birth_info: &BirthInfo, period: Duration) -> Vec<TransitInfo> {
            let mut transits = Vec::new();
            let start_jd = date_to_julian_day(birth_info.date_time);
            let end_jd = start_jd + period.num_days() as f64;
    
            let planets = vec![
                CelestialBody::Sun,
                CelestialBody::Moon,
                CelestialBody::Mars,
                CelestialBody::Mercury,
                CelestialBody::Jupiter,
                CelestialBody::Venus,
                CelestialBody::Saturn,
            ];
    
            for planet in planets {
                let mut current_jd = start_jd;
                let mut current_sign = self.get_zodiac_sign_for_planet(planet, current_jd);
    
                while current_jd < end_jd {
                    current_jd += 1.0;
                    let new_sign = self.get_zodiac_sign_for_planet(planet, current_jd);
    
                    if new_sign != current_sign {
                        transits.push(TransitInfo {
                            planet,
                            from_sign: current_sign,
                            to_sign: new_sign,
                            date: julian_day_to_date(current_jd),
                        });
                        current_sign = new_sign;
                    }
                }
            }
    
            transits
        }
    
        /// Helper function to get the zodiac sign for a planet at a given Julian day
        fn get_zodiac_sign_for_planet(&self, planet: CelestialBody, jd: JulianDay) -> ZodiacSign {
            let result = self.calculate(CoordinateSystem::Tropical, jd, planet, &[]);
            if let Ok(AstronomicalResult::CelestialBody(info)) = result {
                Self::get_zodiac_sign(info.longitude)
            } else {
                ZodiacSign::Aries // Default to Aries if calculation fails
            }
        }
    
        /// Calculate Varshaphal (annual chart)
        pub fn calculate_varshaphal(&self, birth_info: &BirthInfo, year: i32) -> Option<VarshaphalInfo> {
            let birth_jd = date_to_julian_day(birth_info.date_time);
            let solar_return_jd = self.find_solar_return(birth_jd, year);
    
            if let Some(return_jd) = solar_return_jd {
                let ascendant = self.calculate_ascendant(
                    CoordinateSystem::Tropical,
                    return_jd,
                    birth_info.latitude,
                    birth_info.longitude,
                ).ok()?;
    
                let planets = self.calculate_planet_positions(
                    CoordinateSystem::Tropical,
                    return_jd,
                    ChartType::Rasi,
                    birth_info,
                ).ok()?;
    
                Some(VarshaphalInfo {
                    year,
                    ascendant: ascendant.sign,
                    planets,
                })
            } else {
                None
            }
        }
    
        /// Helper function to find the solar return Julian day
        fn find_solar_return(&self, birth_jd: JulianDay, target_year: i32) -> Option<JulianDay> {
            let birth_sun_long = self.calculate(CoordinateSystem::Tropical, birth_jd, CelestialBody::Sun, &[])
                .ok()
                .and_then(|r| match r {
                    AstronomicalResult::CelestialBody(info) => Some(info.longitude),
                    _ => None,
                })?;
    
            let mut low = birth_jd + (target_year - birth_jd.floor() as i32) as f64 * 365.25;
            let mut high = low + 366.0;
    
            while high - low > 0.00001 {
                let mid = (low + high) / 2.0;
                let mid_sun_long = self.calculate(CoordinateSystem::Tropical, mid, CelestialBody::Sun, &[])
                    .ok()
                    .and_then(|r| match r {
                        AstronomicalResult::CelestialBody(info) => Some(info.longitude),
                        _ => None,
                    })?;
    
                if (mid_sun_long - birth_sun_long).abs() < 0.00001 {
                    return Some(mid);
                } else if mid_sun_long < birth_sun_long {
                    low = mid;
                } else {
                    high = mid;
                }
            }
    
            None
        }
    
        /// Calculate compatibility between two charts
        pub fn calculate_compatibility(&self, chart1: &ChartInfo, chart2: &ChartInfo) -> CompatibilityInfo {
            let kuta_points = self.calculate_kuta_points(chart1, chart2);
            let compatibility_score = self.calculate_compatibility_score(chart1, chart2);
    
            CompatibilityInfo {
                kuta_points,
                compatibility_score,
            }
        }
    
        /// Helper function to calculate Kuta points
        fn calculate_kuta_points(&self, chart1: &ChartInfo, chart2: &ChartInfo) -> u32 {
            let mut points = 0;
    
            // Varna Kuta (1 point)
            if self.check_varna_compatibility(chart1.ascendant, chart2.ascendant) {
                points += 1;
            }
    
            // Vasya Kuta (2 points)
            if self.check_vasya_compatibility(chart1.ascendant, chart2.ascendant) {
                points += 2;
            }
    
            // Tara Kuta (3 points)
            points += self.calculate_tara_kuta(chart1, chart2);
    
            // Yoni Kuta (4 points)
            points += self.calculate_yoni_kuta(chart1, chart2);
    
            // Graha Maitri (5 points)
            points += self.calculate_graha_maitri(chart1, chart2);
    
            // Gana Kuta (6 points)
            if self.check_gana_compatibility(chart1.ascendant, chart2.ascendant) {
                points += 6;
            }
    
            // Bhakut Kuta (7 points)
            if self.check_bhakut_compatibility(chart1.ascendant, chart2.ascendant) {
                points += 7;
            }
    
            // Nadi Kuta (8 points)
            if self.check_nadi_compatibility(chart1.ascendant, chart2.ascendant) {
                points += 8;
            }
    
            points
        }
    
        /// Helper function to calculate overall compatibility score
        fn calculate_compatibility_score(&self, chart1: &ChartInfo, chart2: &ChartInfo) -> f64 {
            let kuta_points = self.calculate_kuta_points(chart1, chart2) as f64;
            let max_points = 36.0; // Maximum possible Kuta points
    
            (kuta_points / max_points) * 100.0
        }
    
        // Helper functions for compatibility calculations
        fn check_varna_compatibility(&self, sign1: ZodiacSign, sign2: ZodiacSign) -> bool {
            let varna1 = self.get_varna(sign1);
            let varna2 = self.get_varna(sign2);
            varna1 >= varna2
        }
    
        fn get_varna(&self, sign: ZodiacSign) -> u32 {
            match sign {
                ZodiacSign::Leo | ZodiacSign::Aries | ZodiacSign::Sagittarius => 4, // Brahmin
                ZodiacSign::Cancer | ZodiacSign::Scorpio | ZodiacSign::Pisces => 3, // Kshatriya
                ZodiacSign::Gemini | ZodiacSign::Libra | ZodiacSign::Aquarius => 2, // Vaishya
                ZodiacSign::Taurus | ZodiacSign::Virgo | ZodiacSign::Capricorn => 1, // Shudra
            }
        }
    
        fn check_vasya_compatibility(&self, sign1: ZodiacSign, sign2: ZodiacSign) -> bool {
            let vasya_groups = vec![
                vec![ZodiacSign::Leo, ZodiacSign::Aries],
                vec![ZodiacSign::Cancer, ZodiacSign::Scorpio],
                vec![ZodiacSign::Gemini, ZodiacSign::Libra, ZodiacSign::Aquarius],
                vec![ZodiacSign::Taurus, ZodiacSign::Capricorn],
                vec![ZodiacSign::Virgo, ZodiacSign::Pisces],
                vec![ZodiacSign::Sagittarius],
            ];
    
            vasya_groups.iter().any(|group| group.contains(&sign1) && group.contains(&sign2))
        }
    
        fn calculate_tara_kuta(&self, chart1: &ChartInfo, chart2: &ChartInfo) -> u32 {
            let moon1 = chart1.planets.iter().find(|p| p.planet == CelestialBody::Moon).unwrap();
            let moon2 = chart2.planets.iter().find(|p| p.planet == CelestialBody::Moon).unwrap();
    
            let nakshatra1 = self.calculate_nakshatra(moon1.longitude);
            let nakshatra2 = self.calculate_nakshatra(moon2.longitude);
    
            let tara = ((nakshatra2.nakshatra as i32 - nakshatra1.nakshatra as i32 + 27) % 27) / 3;
    
            match tara {
                1 | 3 | 5 | 7 => 3,
                0 | 2 | 4 | 6 | 8 => 0,
                _ => unreachable!(),
            }
        }
    
        fn calculate_yoni_kuta(&self, chart1: &ChartInfo, chart2: &ChartInfo) -> u32 {
            let moon1 = chart1.planets.iter().find(|p| p.planet == CelestialBody::Moon).unwrap();
            let moon2 = chart2.planets.iter().find(|p| p.planet == CelestialBody::Moon).unwrap();
    
            let nakshatra1 = self.calculate_nakshatra(moon1.longitude);
            let nakshatra2 = self.calculate_nakshatra(moon2.longitude);
    
            let yoni1 = self.get_yoni(nakshatra1.nakshatra);
            let yoni2 = self.get_yoni(nakshatra2.nakshatra);
    
            if yoni1 == yoni2 {
                4
            } else if self.are_yonis_compatible(yoni1, yoni2) {
                2
            } else {
                0
            }
        }
    
        fn get_yoni(&self, nakshatra: Nakshatra) -> &'static str {
            match nakshatra {
                Nakshatra::Ashwini | Nakshatra::Shatabhisha => "Horse",
                Nakshatra::Bharani | Nakshatra::Revati => "Elephant",
                Nakshatra::Krittika | Nakshatra::Punarvasu => "Goat",
                Nakshatra::Rohini | Nakshatra::Uttara_Phalguni => "Snake",
                Nakshatra::Mrigashira | Nakshatra::Chitra => "Dog",
                Nakshatra::Ardra | Nakshatra::Shravana => "Cat",
                Nakshatra::Pushya | Nakshatra::Uttara_Ashadha => "Ram",
                Nakshatra::Ashlesha | Nakshatra::Jyeshtha => "Mongoose",
                Nakshatra::Magha | Nakshatra::Purva_Phalguni => "Rat",
                Nakshatra::Hasta | Nakshatra::Anuradha => "Buffalo",
                Nakshatra::Swati | Nakshatra::Dhanishta => "Tiger",
                Nakshatra::Vishakha | Nakshatra::Purva_Ashadha => "Deer",
                Nakshatra::Moola | Nakshatra::Purva_Bhadrapada => "Monkey",
                Nakshatra::Uttara_Bhadrapada => "Lion",
            }
        }
    
        fn are_yonis_compatible(&self, yoni1: &str, yoni2: &str) -> bool {
            let compatible_pairs = vec![
                ("Horse", "Mare"),
                ("Elephant", "Elephant"),
                ("Goat", "Goat"),
                ("Snake", "Snake"),
                ("Dog", "Bitch"),
                ("Cat", "Cat"),
                ("Ram", "Sheep"),
                ("Mongoose", "Mongoose"),
                ("Rat", "Rat"),
                ("Buffalo", "Buffalo"),
                ("Tiger", "Deer"),
                ("Deer", "Tiger"),
                ("Monkey", "Monkey"),
                ("Lion", "Lion"),
            ];
    
            compatible_pairs.contains(&(yoni1, yoni2)) || compatible_pairs.contains(&(yoni2, yoni1))
        }
    
        fn calculate_graha_maitri(&self, chart1: &ChartInfo, chart2: &ChartInfo) -> u32 {
            let moon1 = chart1.planets.iter().find(|p| p.planet == CelestialBody::Moon).unwrap();
            let moon2 = chart2.planets.iter().find(|p| p.planet == CelestialBody::Moon).unwrap();
    
            let lord1 = self.get_house_lord(moon1.sign);
            let lord2 = self.get_house_lord(moon2.sign);
    
            if self.are_planets_friends(lord1, lord2) {
                5
            } else if self.are_planets_neutral(lord1, lord2) {
                3
            } else {
                0
            }
        }
    
        fn are_planets_friends(&self, planet1: CelestialBody, planet2: CelestialBody) -> bool {
            let friendships = [
                (CelestialBody::Sun, vec![CelestialBody::Moon, CelestialBody::Mars, CelestialBody::Jupiter]),
                (CelestialBody::Moon, vec![CelestialBody::Sun, CelestialBody::Mercury]),
                (CelestialBody::Mars, vec![CelestialBody::Sun, CelestialBody::Moon, CelestialBody::Jupiter]),
                (CelestialBody::Mercury, vec![CelestialBody::Sun, CelestialBody::Venus]),
                (CelestialBody::Jupiter, vec![CelestialBody::Sun, CelestialBody::Moon, CelestialBody::Mars]),
                (CelestialBody::Venus, vec![CelestialBody::Mercury, CelestialBody::Saturn]),
                (CelestialBody::Saturn, vec![CelestialBody::Mercury, CelestialBody::Venus]),
            ];
    
            friendships.iter().any(|&(p, ref friends)| 
                (p == planet1 && friends.contains(&planet2)) || 
                (p == planet2 && friends.contains(&planet1))
            )
        }
    
        fn are_planets_neutral(&self, planet1: CelestialBody, planet2: CelestialBody) -> bool {
            let neutral_relations = [
                (CelestialBody::Sun, vec![CelestialBody::Mercury]),
                (CelestialBody::Moon, vec![CelestialBody::Mars, CelestialBody::Jupiter, CelestialBody::Venus, CelestialBody::Saturn]),
                (CelestialBody::Mars, vec![CelestialBody::Mercury, CelestialBody::Venus, CelestialBody::Saturn]),
                (CelestialBody::Mercury, vec![CelestialBody::Mars, CelestialBody::Jupiter, CelestialBody::Saturn]),
                (CelestialBody::Jupiter, vec![CelestialBody::Mercury, CelestialBody::Venus, CelestialBody::Saturn]),
                (CelestialBody::Venus, vec![CelestialBody::Mars, CelestialBody::Jupiter]),
                (CelestialBody::Saturn, vec![CelestialBody::Mars, CelestialBody::Jupiter]),
            ];
    
            neutral_relations.iter().any(|&(p, ref neutrals)| 
                (p == planet1 && neutrals.contains(&planet2)) || 
                (p == planet2 && neutrals.contains(&planet1))
            )
        }
    
        fn check_gana_compatibility(&self, sign1: ZodiacSign, sign2: ZodiacSign) -> bool {
            let gana1 = self.get_gana(sign1);
            let gana2 = self.get_gana(sign2);
    
            match (gana1, gana2) {
                ("Deva", "Deva") | ("Manushya", "Manushya") | ("Rakshasa", "Rakshasa") => true,
                ("Deva", "Manushya") | ("Manushya", "Deva") => true,
                _ => false,
            }
        }
    
        fn get_gana(&self, sign: ZodiacSign) -> &'static str {
            match sign {
                ZodiacSign::Aries | ZodiacSign::Leo | ZodiacSign::Sagittarius => "Deva",
                ZodiacSign::Taurus | ZodiacSign::Virgo | ZodiacSign::Capricorn => "Manushya",
                ZodiacSign::Gemini | ZodiacSign::Libra | ZodiacSign::Aquarius => "Deva",
                ZodiacSign::Cancer | ZodiacSign::Scorpio | ZodiacSign::Pisces => "Rakshasa",
            }
        }
    
        fn check_bhakut_compatibility(&self, sign1: ZodiacSign, sign2: ZodiacSign) -> bool {
            let diff = (sign2 as i32 - sign1 as i32 + 12) % 12;
            matches!(diff, 1 | 2 | 3 | 4 | 5 | 7 | 9 | 11)
        }
    
        fn check_nadi_compatibility(&self, sign1: ZodiacSign, sign2: ZodiacSign) -> bool {
            let nadi1 = self.get_nadi(sign1);
            let nadi2 = self.get_nadi(sign2);
            nadi1 != nadi2
        }
    
        fn get_nadi(&self, sign: ZodiacSign) -> &'static str {
            match sign {
                ZodiacSign::Aries | ZodiacSign::Cancer | ZodiacSign::Libra | ZodiacSign::Capricorn => "Aadi",
                ZodiacSign::Taurus | ZodiacSign::Leo | ZodiacSign::Scorpio | ZodiacSign::Aquarius => "Madhya",
                ZodiacSign::Gemini | ZodiacSign::Virgo | ZodiacSign::Sagittarius | ZodiacSign::Pisces => "Antya",
            }
        }
    
        /// Suggest remedial measures
        pub fn suggest_remedial_measures(&self, chart: &ChartInfo) -> Vec<RemedialMeasure> {
            let mut remedies = Vec::new();
    
            for planet in &chart.planets {
                if self.is_planet_weak(planet) {
                    let remedy = self.get_remedy_for_planet(planet.planet);
                    remedies.push(remedy);
                }
            }
    
            // Add general remedies
            remedies.push(RemedialMeasure {
                description: "Practice meditation daily for spiritual growth".to_string(),
                gemstone: None,
            });
    
            remedies.push(RemedialMeasure {
                description: "Perform charity on Saturdays to mitigate malefic influences".to_string(),
                gemstone: None,
            });
    
            remedies
        }
    
        fn is_planet_weak(&self, planet: &PlanetPosition) -> bool {
            planet.retrograde || self.is_debilitated(planet) || self.is_combust(planet)
        }
    
        fn is_combust(&self, planet: &PlanetPosition) -> bool {
            if planet.planet == CelestialBody::Sun {
                return false;
            }
    
            let sun_position = self.calculate(
                CoordinateSystem::Tropical,
                date_to_julian_day(Utc::now()),
                CelestialBody::Sun,
                &[],
            ).unwrap();
    
            if let AstronomicalResult::CelestialBody(sun_info) = sun_position {
                let angle_diff = (planet.longitude - sun_info.longitude).abs();
                match planet.planet {
                    CelestialBody::Moon => angle_diff <= 12.0,
                    CelestialBody::Mars => angle_diff <= 17.0,
                    CelestialBody::Mercury => angle_diff <= 14.0,
                    CelestialBody::Jupiter => angle_diff <= 11.0,
                    CelestialBody::Venus => angle_diff <= 10.0,
                    CelestialBody::Saturn => angle_diff <= 15.0,
                    _ => false,
                }
            } else {
                false
            }
        }
    
        fn get_remedy_for_planet(&self, planet: CelestialBody) -> RemedialMeasure {
            match planet {
                CelestialBody::Sun => RemedialMeasure {
                    description: "Offer water to the Sun every morning".to_string(),
                    gemstone: Some("Ruby".to_string()),
                },
                CelestialBody::Moon => RemedialMeasure {
                    description: "Wear white clothes on Mondays".to_string(),
                    gemstone: Some("Pearl".to_string()),
                },
                CelestialBody::Mars => RemedialMeasure {
                    description: "Recite Mars mantra on Tuesdays".to_string(),
                    gemstone: Some("Red Coral".to_string()),
                },
                CelestialBody::Mercury => RemedialMeasure {
                    description: "Feed green vegetables to cows on Wednesdays".to_string(),
                    gemstone: Some("Emerald".to_string()),
                },
                CelestialBody::Jupiter => RemedialMeasure {
                    description: "Donate yellow items on Thursdays".to_string(),
                    gemstone: Some("Yellow Sapphire".to_string()),
                },
                CelestialBody::Venus => RemedialMeasure {
                    description: "Offer white flowers to Venus on Fridays".to_string(),
                    gemstone: Some("Diamond".to_string()),
                },
                CelestialBody::Saturn => RemedialMeasure {
                    description: "Feed black sesame seeds to birds on Saturdays".to_string(),
                    gemstone: Some("Blue Sapphire".to_string()),
                },
                CelestialBody::Rahu => RemedialMeasure {
                    description: "Donate to orphanages on Saturdays".to_string(),
                    gemstone: Some("Hessonite".to_string()),
                },
                CelestialBody::Ketu => RemedialMeasure {
                    description: "Perform fire rituals on Tuesdays".to_string(),
                    gemstone: Some("Cat's Eye".to_string()),
                },
            }
        }
    
        /// Generate chart interpretation
        pub fn generate_interpretation(&self, report: &Report) -> String {
            let mut interpretation = String::new();
    
            interpretation.push_str(&format!("Birth Chart Interpretation for {}\n\n", 
                report.birth_info.date_time.format("%Y-%m-%d %H:%M:%S")));
    
            interpretation.push_str("Planetary Positions:\n");
            for planet in &report.charts[0].planets {
                interpretation.push_str(&format!("{}: {} in {}\n", 
                    self.get_body_name(planet.planet),
                    planet.sign,
                    planet.house));
            }
    
            interpretation.push_str("\nAscendant: ");
            interpretation.push_str(&format!("{}\n", report.charts[0].ascendant));
    
            interpretation.push_str("\nYogas:\n");
            for yoga in &report.yogas {
                interpretation.push_str(&format!("{:?} Yoga (Strength: {:.2})\n", 
                    yoga.yoga, yoga.strength));
            }
    
            interpretation.push_str("\nDasha Periods:\n");
            interpretation.push_str(&format!("Maha Dasha: {:?} ({} to {})\n",
                report.dashas.maha_dasha,
                report.dashas.maha_dasha_start.format("%Y-%m-%d"),
                report.dashas.maha_dasha_end.format("%Y-%m-%d")));
            interpretation.push_str(&format!("Antar Dasha: {:?} ({} to {})\n",
                report.dashas.antar_dasha,
                report.dashas.antar_dasha_start.format("%Y-%m-%d"),
                report.dashas.antar_dasha_end.format("%Y-%m-%d")));
    
            interpretation.push_str("\nPlanetary Strengths:\n");
            for (planet, strength) in &report.strengths {
                interpretation.push_str(&format!("{}: Shadbala = {:.2}, Ashtakavarga = {}\n",
                    self.get_body_name(*planet),
                    strength.shad_bala,
                    strength.ashtaka_varga));
            }
    
            interpretation.push_str("\nRemedial Measures:\n");
            for remedy in &report.remedial_measures {
                interpretation.push_str(&format!("- {}\n", remedy.description));
                if let Some(gemstone) = &remedy.gemstone {
                    interpretation.push_str(&format!("  Recommended Gemstone: {}\n", gemstone));
                }
            }
    
            interpretation
        }
    }
    
    /// Convert Julian Day to DateTime<Utc>
    pub fn julian_day_to_date(jd: JulianDay) -> DateTime<Utc> {
        let mut year = 0;
        let mut month = 0;
        let mut day = 0;
        let mut hour = 0;
        let mut minute = 0;
        let mut second = 0.0;
    
        unsafe {
            swe_jdut1_to_utc(
                jd,
                1, // calendar: Gregorian
                &mut year,
                &mut month,
                &mut day,
                &mut hour,
                &mut minute,
                &mut second,
            );
        }
    
        Utc.ymd(year, month as u32, day as u32)
            .and_hms_micro(
                hour as u32,
                minute as u32,
                second as u32,
                ((second.fract() * 1_000_000.0) as u32),
            )
        }

impl Drop for SwissEph {
    fn drop(&mut self) {
        unsafe {
            swe_close();
        }
    }
}

/// Converts a DateTime<Utc> to Julian Day.
pub fn date_to_julian_day(date_time: DateTime<Utc>) -> JulianDay {
    let year = date_time.year();
    let month = date_time.month();
    let day = date_time.day();
    let hour = date_time.hour();
    let minute = date_time.minute();
    let second =
        date_time.second() as f64 + (date_time.timestamp_subsec_micros() as f64 / 1_000_000.0);

    let mut tjd_ut: f64 = 0.0;
    let mut dret: [f64; 2] = [0.0; 2];
    unsafe {
        swe_utc_to_jd(
            year,
            month as i32,
            day as i32,
            hour as i32,
            minute as i32,
            second,
            SE_GREG_CAL as i32,
            dret.as_mut_ptr(),
            std::ptr::null_mut(),
        );
        tjd_ut = dret[1]; // Use UT
    }
    tjd_ut
}

/// Calculates ayanamsa value based on the Julian day.
pub fn calculate_ayanamsa(julian_day: JulianDay) -> AyanamsaInfo {
    let ayanamsa_value = unsafe { swe_get_ayanamsa_ut(julian_day) };
    let ayanamsa_name = "Lahiri".to_string(); // Assuming Lahiri ayanamsa
    AyanamsaInfo {
        ayanamsa_name,
        ayanamsa_value,
    }
}

/// Generates a chart (Rasi, Navamsa, etc.) based on birth info.
pub fn calculate_chart(
    eph: &SwissEph,
    birth_info: &BirthInfo,
    chart_type: ChartType,
) -> Result<ChartInfo, CalculationError> {
    let julian_day = date_to_julian_day(birth_info.date_time);
    let coord_system = CoordinateSystem::Sidereal;

    let ascendant_info = eph.calculate_ascendant(
        coord_system,
        julian_day,
        birth_info.latitude,
        birth_info.longitude,
    )?;

    let houses = eph.calculate_houses(
        coord_system,
        julian_day,
        birth_info.latitude,
        birth_info.longitude,
    )?;

    let planets =
        eph.calculate_planet_positions(coord_system, julian_day, chart_type, birth_info)?;

    Ok(ChartInfo {
        chart_type,
        ascendant: ascendant_info.sign,
        houses,
        planets,
    })
}

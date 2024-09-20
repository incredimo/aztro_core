// src/main.rs

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt;
use std::io::Cursor;
use std::os::raw::c_char;
use std::sync::Once;
use tempfile::NamedTempFile;

// Include the generated bindings for the Swiss Ephemeris C library
include!("../build/bindings.rs");

// Embed the ephemeris file into the binary
static EPHE_FILE: &[u8] = include_bytes!("../ephe/sepl_18.se1");
static INIT: Once = Once::new();

/// Represents the coordinate system used in calculations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoordinateSystem {
    Tropical,
    Sidereal,
}

/// Enumerates the celestial bodies supported by the Swiss Ephemeris.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum CelestialBody {
    EclipticNutation = -1,
    Sun = 0,
    Moon = 1,
    Mercury = 2,
    Venus = 3,
    Mars = 4,
    Jupiter = 5,
    Saturn = 6,
    Uranus = 7,
    Neptune = 8,
    Pluto = 9,
    MeanNode = 10,
    TrueNode = 11,
    MeanLunarApogee = 12,
    OsculatingLunarApogee = 13,
    Earth = 14,
    Chiron = 15,
    Pholus = 16,
    Ceres = 17,
    Pallas = 18,
    Juno = 19,
    Vesta = 20,
}

/// Flags to control the calculation behavior.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalculationFlag {
    Speed = SEFLG_SPEED as isize,
    NoGravitationalDeflection = SEFLG_NOGDEFL as isize,
    NoAnnualAberration = SEFLG_NOABERR as isize,
    Topocentric = SEFLG_TOPOCTR as isize,
    Equatorial = SEFLG_EQUATORIAL as isize,
    XYZ = SEFLG_XYZ as isize,
    Radians = SEFLG_RADIANS as isize,
    Barycentric = SEFLG_BARYCTR as isize,
    Heliocentric = SEFLG_HELCTR as isize,
}

/// Represents the twelve astrological houses.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum House {
    First,
    Second,
    Third,
    Fourth,
    Fifth,
    Sixth,
    Seventh,
    Eighth,
    Ninth,
    Tenth,
    Eleventh,
    Twelfth,
}

/// Represents the twelve zodiac signs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZodiacSign {
    Aries,
    Taurus,
    Gemini,
    Cancer,
    Leo,
    Virgo,
    Libra,
    Scorpio,
    Sagittarius,
    Capricorn,
    Aquarius,
    Pisces,
}

/// Contains information about a celestial body's position and speed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CelestialBodyInfo {
    pub longitude: f64,
    pub latitude: f64,
    pub distance: f64,
    pub speed_longitude: f64,
    pub speed_latitude: f64,
    pub speed_distance: f64,
}

/// Contains information about the ecliptic obliquity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EclipticObliquityInfo {
    pub ecliptic_true_obliquity: f64,
    pub ecliptic_mean_obliquity: f64,
    pub nutation_longitude: f64,
    pub nutation_obliquity: f64,
}

/// Represents the position of an astrological house.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HousePosition {
    pub house: House,
    pub sign: ZodiacSign,
    pub degree: f64,
}

/// Represents the result of an astronomical calculation.
#[derive(Debug)]
pub enum AstronomicalResult {
    CelestialBody(CelestialBodyInfo),
    EclipticObliquity(EclipticObliquityInfo),
}

/// Represents an error that can occur during calculations.
#[derive(Debug)]
pub struct CalculationError {
    pub code: i32,
    pub message: String,
}

impl fmt::Display for CalculationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CalculationError {}: {}", self.code, self.message)
    }
}

impl Error for CalculationError {}

/// Alias for Julian Day, simplifying the representation of dates.
pub type JulianDay = f64;

/// SwissEph provides methods to perform astronomical calculations using the Swiss Ephemeris.
pub struct SwissEph {
    _temp_file: NamedTempFile,
}

impl SwissEph {
    /// Initializes a new instance of SwissEph, loading the ephemeris data.
    pub fn new() -> Self {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        std::io::copy(&mut Cursor::new(EPHE_FILE), &mut temp_file)
            .expect("Failed to write ephemeris data to temp file");

        INIT.call_once(|| {
            let file_path = temp_file.path().to_str().expect("Invalid ephemeris file path");
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
            CoordinateSystem::Sidereal => {
                unsafe {
                    swe_set_sid_mode(SE_SIDM_LAHIRI as i32, 0.0, 0.0);
                }
            }
            CoordinateSystem::Tropical => {
                unsafe {
                    swe_set_sid_mode(SE_SIDM_FAGAN_BRADLEY as i32, 0.0, 0.0);
                }
            }
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
            CelestialBody::EclipticNutation => {
                let mut nut: [f64; 4] = [0.0; 4];
                let mut error: [c_char; 256] = [0; 256];
                let calc_result = unsafe {
                    swe_calc_ut(
                        julian_day,
                        SE_ECL_NUT as i32,
                        0,
                        nut.as_mut_ptr(),
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
                Ok(AstronomicalResult::EclipticObliquity(EclipticObliquityInfo {
                    ecliptic_true_obliquity: nut[0],
                    ecliptic_mean_obliquity: nut[1],
                    nutation_longitude: nut[2],
                    nutation_obliquity: nut[3],
                }))
            }
            _ => {
                let mut results: [f64; 6] = [0.0; 6];
                let mut error: [c_char; 256] = [0; 256];
                let calc_result =
                    unsafe { swe_calc_ut(julian_day, body as i32, iflag, results.as_mut_ptr(), error.as_mut_ptr()) };
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
        let mut name: [c_char; 256] = [0; 256];
        unsafe {
            swe_get_planet_name(body as i32, name.as_mut_ptr());
        }
        unsafe { CStr::from_ptr(name.as_ptr()) }
            .to_string_lossy()
            .into_owned()
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
}

impl Drop for SwissEph {
    fn drop(&mut self) {
        unsafe {
            swe_close();
        }
    }
}

/// Converts a Gregorian date and time to Julian Day.
pub fn gregorian_to_julian_day(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: f64,
) -> JulianDay {
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

/// Converts Julian Day to Gregorian date and time.
pub fn julian_day_to_gregorian(jd: JulianDay) -> (i32, u32, u32, u32, u32, f64) {
    let mut year: i32 = 0;
    let mut month: i32 = 0;
    let mut day: i32 = 0;
    let mut hour: i32 = 0;
    let mut minute: i32 = 0;
    let mut second: f64 = 0.0;
    unsafe {
        swe_jdut1_to_utc(
            jd,
            SE_GREG_CAL as i32,
            &mut year,
            &mut month,
            &mut day,
            &mut hour,
            &mut minute,
            &mut second,
        );
    }
    (
        year,
        month as u32,
        day as u32,
        hour as u32,
        minute as u32,
        second,
    )
}

/// Formats degree into DMS (Degrees, Minutes, Seconds)
pub fn format_degree(degree: f64) -> String {
    let deg = degree.floor() as u32;
    let minutes_float = (degree - deg as f64) * 60.0;
    let minutes = minutes_float.floor() as u32;
    let seconds = ((minutes_float - minutes as f64) * 60.0).floor() as u32;
    format!("{:02}-{:02}-{:02}", deg, minutes, seconds)
}

/// Parses degree from "DD-MM-SS" format
pub fn parse_degree(degree_str: &str) -> Result<f64, Box<dyn Error>> {
    let parts: Vec<&str> = degree_str.split('-').collect();
    if parts.len() != 3 {
        return Err("Degree string must be in 'DD-MM-SS' format".into());
    }
    let deg: f64 = parts[0].parse()?;
    let min: f64 = parts[1].parse()?;
    let sec: f64 = parts[2].parse()?;
    Ok(deg + min / 60.0 + sec / 3600.0)
}

/// Retrieves the opposite sign of a given sign.
pub fn get_opposite_sign(sign: &str) -> Result<String, Box<dyn Error>> {
    let signs = vec![
        "Aries", "Taurus", "Gemini", "Cancer", "Leo", "Virgo",
        "Libra", "Scorpio", "Sagittarius", "Capricorn",
        "Aquarius", "Pisces",
    ];
    let index = signs.iter().position(|&s| s == sign).ok_or("Invalid sign")?;
    let opposite_index = (index + 6) % 12;
    Ok(signs[opposite_index].to_string())
}


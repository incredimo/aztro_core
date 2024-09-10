#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]

use std::ffi::{CStr, CString};
use std::io::Cursor;
use std::os::raw::c_char;
use std::sync::Once;
use serde::{Serialize, Deserialize};
use tempfile::NamedTempFile;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

static INIT: Once = Once::new();
static EPHE_FILE: &[u8] = include_bytes!("../ephe/sepl_18.se1");

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UtcDateTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

 


impl UtcDateTime {
    pub fn new(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> Self {
        UtcDateTime { year, month, day, hour, minute, second }
    }


    pub fn to_julian_day(&self) -> f64 {
        let hour = self.hour as f64 + self.minute as f64 / 60.0 + self.second as f64 / 3600.0;
        unsafe {
            swe_julday(
                self.year,
                self.month as i32,
                self.day as i32,
                hour,
                SE_GREG_CAL as i32,
            )
        }
    }

    pub fn from_julian_day(julian_day: f64) -> Self {
        let mut year = 0;
        let mut month = 0;
        let mut day = 0;
        let mut hour = 0.0;

        unsafe {
            swe_revjul(julian_day, SE_GREG_CAL as i32, &mut year, &mut month, &mut day, &mut hour);
        }

        let total_seconds = (hour * 3600.0) as u32;
        let hour = total_seconds / 3600;
        let minute = (total_seconds % 3600) / 60;
        let second = total_seconds % 60;

        UtcDateTime { year, month: month as u32, day: day as u32, hour, minute, second }
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Body {
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

const SIDEREAL_MODE: i32 = SE_SIDM_FAGAN_BRADLEY as i32;
const SIDEREAL_MODE_TROPICAL: i32 = SE_SIDM_TRUE_CITRA as i32;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Flag {
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BodyInfo {
    pub longitude: f64,
    pub latitude: f64,
    pub distance: f64,
    pub speed_longitude: f64,
    pub speed_latitude: f64,
    pub speed_distance: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize )]
pub struct EclipticObliquityInfo {
    pub ecliptic_true_obliquity: f64,
    pub ecliptic_mean_obliquity: f64,
    pub nutation_longitude: f64,
    pub nutation_obliquity: f64,
}


#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize  )]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HousePosition {
    pub house: House,
    pub sign: ZodiacSign,
    pub degree: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug)]
pub enum CalculationResult {
    Body(BodyInfo),
    EclipticObliquity(EclipticObliquityInfo),
}

#[derive(Debug)]
pub struct CalculationError {
    pub code: i32,
    pub message: String,
}

pub struct SwissEph {
    _temp_file: NamedTempFile,
}

impl SwissEph {
    pub fn new() -> Self {
        let mut temp_file = NamedTempFile::new().unwrap();
        std::io::copy(&mut Cursor::new(EPHE_FILE), &mut temp_file).unwrap();
        
        INIT.call_once(|| {
            let c_path = CString::new(temp_file.path().to_str().unwrap()).unwrap();
            unsafe {
                swe_set_ephe_path(c_path.as_ptr() as *mut c_char);
            }
        });

        SwissEph { _temp_file: temp_file }
    }

    fn date_to_julian(&self, date: UtcDateTime) -> f64 {
        let hour = date.hour as f64 + date.minute as f64 / 60.0 + date.second as f64 / 3600.0;
        unsafe {
            swe_julday(
                date.year,
                date.month as i32,
                date.day as i32,
                hour,
                SE_GREG_CAL as i32,
            )
        }
    }

    fn calculate_internal(&self, date: UtcDateTime, body: Body, iflag: i32) -> Result<CalculationResult, CalculationError> {
        let julian_day = self.date_to_julian(date);
        
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
            let error_message = unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy().into_owned();
            return Err(CalculationError {
                code: calc_result,
                message: error_message,
            });
        }

        match body {
            Body::EclipticNutation => Ok(CalculationResult::EclipticObliquity(EclipticObliquityInfo {
                ecliptic_true_obliquity: results[0],
                ecliptic_mean_obliquity: results[1],
                nutation_longitude: results[2],
                nutation_obliquity: results[3],
            })),
            _ => Ok(CalculationResult::Body(BodyInfo {
                longitude: results[0],
                latitude: results[1],
                distance: results[2],
                speed_longitude: results[3],
                speed_latitude: results[4],
                speed_distance: results[5],
            })),
        }
    }

    pub fn calculate_tropical(&self, date: UtcDateTime, body: Body, flags: &[Flag]) -> Result<CalculationResult, CalculationError> {
        unsafe {
            swe_set_sid_mode(SIDEREAL_MODE_TROPICAL as i32, 0.0, 0.0);
        }
        
        let mut iflag: i32 = 0;
        for flag in flags {
            iflag |= *flag as i32;
        }
        self.calculate_internal(date, body, iflag)
    }

    pub fn calculate_sidereal(&self, date: UtcDateTime, body: Body, flags: &[Flag]) -> Result<CalculationResult, CalculationError> {
        unsafe {
            swe_set_sid_mode(SIDEREAL_MODE as i32, 0.0, 0.0);
        }
        
        let mut iflag: i32 = SEFLG_SIDEREAL as i32;
        for flag in flags {
            iflag |= *flag as i32;
        }
        self.calculate_internal(date, body, iflag)
    }

    pub fn get_body_name(&self, body: Body) -> String {
        let mut name: [c_char; 256] = [0; 256];
        unsafe {
            swe_get_planet_name(body as i32, name.as_mut_ptr());
        }
        unsafe { CStr::from_ptr(name.as_ptr()) }.to_string_lossy().into_owned()
    }


    pub fn calculate_houses_tropical(&self, date: UtcDateTime, latitude: f64, longitude: f64) -> Result<Vec<HousePosition>, CalculationError> {
        self.calculate_houses(date, latitude, longitude, 'P' as i32, false)
    }

    pub fn calculate_houses_sidereal(&self, date: UtcDateTime, latitude: f64, longitude: f64) -> Result<Vec<HousePosition>, CalculationError> {
        self.calculate_houses(date, latitude, longitude, 'P' as i32, true)
    }

    fn calculate_houses(&self, date: UtcDateTime, latitude: f64, longitude: f64, house_system: i32, is_sidereal: bool) -> Result<Vec<HousePosition>, CalculationError> {
        let julian_day = self.date_to_julian(date);
        let mut cusps: [f64; 13] = [0.0; 13];
        let mut ascmc: [f64; 10] = [0.0; 10];
        let mut error: [c_char; 256] = [0; 256];

        let calc_result = unsafe {
            if is_sidereal {
                swe_set_sid_mode(SIDEREAL_MODE, 0.0, 0.0);
                swe_houses_ex(
                    julian_day,
                    SEFLG_SIDEREAL as i32,
                    latitude,
                    longitude,
                    house_system,
                    cusps.as_mut_ptr(),
                    ascmc.as_mut_ptr(),
                )
            } else {
                swe_houses(
                    julian_day,
                    latitude,
                    longitude,
                    house_system,
                    cusps.as_mut_ptr(),
                    ascmc.as_mut_ptr(),
                )
            }
        };

        if calc_result < 0 {
            let error_message = unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy().into_owned();
            return Err(CalculationError {
                code: calc_result,
                message: error_message,
            });
        }

        let mut house_positions: Vec<HousePosition> = Vec::new();
        for i in 1..13 {
            let house = match i {
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
            };

            let longitude = cusps[i];
            let house_position = HousePosition {
                house,
                sign: self.get_sign_from_longitude(longitude),
                degree: longitude % 30.0,
            };

            house_positions.push(house_position);
        }

        Ok(house_positions)
    }

    fn get_sign_from_longitude(&self, longitude: f64) -> ZodiacSign {
        let normalized_longitude = longitude % 360.0;
        match (normalized_longitude / 30.0) as usize {
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
            _ => unreachable!(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;
    use approx::assert_relative_eq;

    fn create_date(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> UtcDateTime {
        UtcDateTime::new(year, month, day, hour, minute, second)
    }

    fn assert_longitude_in_range(longitude: f64) {
        assert!(longitude >= 0.0 && longitude < 360.0, "Longitude {} is out of range", longitude);
    }

    fn assert_latitude_in_range(latitude: f64) {
        assert!(latitude >= -90.0 && latitude <= 90.0, "Latitude {} is out of range", latitude);
    }

    fn deg_to_rad(deg: f64) -> f64 {
        deg * PI / 180.0
    }

    #[test]
    fn test_sun_position() {
        let eph = SwissEph::new();
        let date = create_date(2023, 5, 17, 12, 0, 0);
        
        let result = eph.calculate_tropical(date, Body::Sun, &[Flag::Speed]).unwrap();
        
        if let CalculationResult::Body(body_info) = result {
            assert_longitude_in_range(body_info.longitude);
            assert_latitude_in_range(body_info.latitude);
            assert!(body_info.distance > 0.9 && body_info.distance < 1.1, "Sun distance {} is out of expected range", body_info.distance);
            
            assert!(body_info.speed_longitude.abs() < 1.0, "Sun longitude speed {} seems unreasonable", body_info.speed_longitude);
            assert!(body_info.speed_latitude.abs() < 0.1, "Sun latitude speed {} seems unreasonable", body_info.speed_latitude);
            assert!(body_info.speed_distance.abs() < 0.01, "Sun distance speed {} seems unreasonable", body_info.speed_distance);
        } else {
            panic!("Expected BodyInfo for Sun");
        }
    }

    #[test]
    fn test_moon_position() {
        let eph = SwissEph::new();
        let date = create_date(2023, 5, 17, 12, 0, 0);
        
        let result = eph.calculate_tropical(date, Body::Moon, &[Flag::Speed]).unwrap();
        
        if let CalculationResult::Body(body_info) = result {
            assert_longitude_in_range(body_info.longitude);
            assert_latitude_in_range(body_info.latitude);
            assert!(body_info.distance > 0.002 && body_info.distance < 0.003, "Moon distance {} is out of expected range", body_info.distance);
            
            assert!(body_info.speed_longitude.abs() < 15.0, "Moon longitude speed {} seems unreasonable", body_info.speed_longitude);
            assert!(body_info.speed_latitude.abs() < 10.0, "Moon latitude speed {} seems unreasonable", body_info.speed_latitude);
            assert!(body_info.speed_distance.abs() < 0.01, "Moon distance speed {} seems unreasonable", body_info.speed_distance);
        } else {
            panic!("Expected BodyInfo for Moon");
        }
    }

    #[test]
    fn test_ecliptic_obliquity() {
        let eph = SwissEph::new();
        let date = create_date(2023, 5, 17, 12, 0, 0);
        
        let result = eph.calculate_tropical(date, Body::EclipticNutation, &[]).unwrap();
        
        if let CalculationResult::EclipticObliquity(info) = result {
            assert!(info.ecliptic_true_obliquity > 23.0 && info.ecliptic_true_obliquity < 24.0, 
                    "True obliquity {} is out of expected range", info.ecliptic_true_obliquity);
            assert!(info.ecliptic_mean_obliquity > 23.0 && info.ecliptic_mean_obliquity < 24.0, 
                    "Mean obliquity {} is out of expected range", info.ecliptic_mean_obliquity);
            assert!(info.nutation_longitude.abs() < 0.01, 
                    "Nutation in longitude {} seems unreasonable", info.nutation_longitude);
            assert!(info.nutation_obliquity.abs() < 0.01, 
                    "Nutation in obliquity {} seems unreasonable", info.nutation_obliquity);
        } else {
            panic!("Expected EclipticObliquityInfo");
        }
    }

    #[test]
    fn test_sidereal_calculation() {
        let eph = SwissEph::new();
        let date = create_date(2023, 5, 17, 12, 0, 0);
        
        let tropical_result = eph.calculate_tropical(date, Body::Sun, &[]).unwrap();
        let sidereal_result = eph.calculate_sidereal(date, Body::Sun, &[]).unwrap();
        
        if let (CalculationResult::Body(tropical), CalculationResult::Body(sidereal)) = (tropical_result, sidereal_result) {
            assert!(
                (tropical.longitude - sidereal.longitude).abs() > 20.0,
                "Tropical and sidereal longitudes should differ significantly. Tropical: {}, Sidereal: {}",
                tropical.longitude,
                sidereal.longitude
            );
            assert_relative_eq!(tropical.latitude, sidereal.latitude, epsilon = 1e-6);
            assert_relative_eq!(tropical.distance, sidereal.distance, epsilon = 1e-6);
        } else {
            panic!("Expected BodyInfo for both calculations");
        }
    }

    #[test]
    fn test_invalid_date() {
        let eph = SwissEph::new();
        let invalid_date = create_date(-5000000, 13, 32, 25, 61, 61);
        
        let result = eph.calculate_tropical(invalid_date, Body::Sun, &[]);
        
        match result {
            Ok(_) => panic!("Expected an error for invalid date, but calculation succeeded"),
            Err(e) => {
                assert!(
                    e.message.contains("invalid") || e.message.contains("error") || e.code < 0,
                    "Error should indicate an invalid date or have a negative error code. Got: {} with code {}",
                    e.message,
                    e.code
                );
            }
        }
    }

    #[test]
    fn test_date_to_julian() {
        let eph = SwissEph::new();
        let date = create_date(2023, 5, 17, 12, 0, 0);
        
        let julian_day = eph.date_to_julian(date);
        
        let expected_jd = 2460082.0;
        assert_relative_eq!(julian_day, expected_jd, epsilon = 1e-2);
    }
}
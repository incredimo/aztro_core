use serde::{Serialize, Deserialize};
use std::ffi::{CStr, CString};
use std::io::Cursor;
use std::os::raw::c_char;
use std::sync::Once;
use tempfile::NamedTempFile;

include!("../build/bindings.rs");

static EPHE_FILE: &[u8] = include_bytes!("../ephe/sepl_18.se1");
static INIT: Once = Once::new();

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoordinateSystem {
    Tropical,
    Sidereal,
}

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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum House {
    First, Second, Third, Fourth, Fifth, Sixth,
    Seventh, Eighth, Ninth, Tenth, Eleventh, Twelfth,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZodiacSign {
    Aries, Taurus, Gemini, Cancer, Leo, Virgo,
    Libra, Scorpio, Sagittarius, Capricorn, Aquarius, Pisces,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtcDateTime {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CelestialBodyInfo {
    pub longitude: f64,
    pub latitude: f64,
    pub distance: f64,
    pub speed_longitude: f64,
    pub speed_latitude: f64,
    pub speed_distance: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EclipticObliquityInfo {
    pub ecliptic_true_obliquity: f64,
    pub ecliptic_mean_obliquity: f64,
    pub nutation_longitude: f64,
    pub nutation_obliquity: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HousePosition {
    pub house: House,
    pub sign: ZodiacSign,
    pub degree: f64,
}

#[derive(Debug)]
pub enum AstronomicalResult {
    CelestialBody(CelestialBodyInfo),
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

impl UtcDateTime {
    pub fn new(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> Self {
        UtcDateTime { year, month, day, hour, minute, second }
    }

    pub fn to_julian_day(&self) -> f64 {
        let mut tjd: f64 = 0.0;
        let mut dret: [f64; 2] = [0.0; 2];
        unsafe {
            swe_utc_to_jd(
                self.year,
                self.month as i32,
                self.day as i32,
                self.hour as i32,
                self.minute as i32,
                self.second as f64,
                1, // gregorian calendar
                dret.as_mut_ptr(),
                std::ptr::null_mut(),
            );
            tjd = dret[1]; // Use TT (Terrestrial Time)
        }
        tjd
    }
    
    pub fn from_julian_day(jd: f64) -> Self {
        let mut year: i32 = 0;
        let mut month: i32 = 0;
        let mut day: i32 = 0;
        let mut hour: i32 = 0;
        let mut minute: i32 = 0;
        let mut second: f64 = 0.0;
        unsafe {
            swe_jdut1_to_utc(
                jd,
                1, // gregorian calendar
                &mut year,
                &mut month,
                &mut day,
                &mut hour,
                &mut minute,
                &mut second,
            );
        }
        UtcDateTime::new(year, month as u32, day as u32, hour as u32, minute as u32, second as u32)
    }

    pub fn add_minutes(&self, minutes: i32) -> Self {
        let mut jd = self.to_julian_day();
        jd += minutes as f64 / 1440.0; // 1440 minutes in a day
        UtcDateTime::from_julian_day(jd)
    }

    pub fn fractional_hour(&self) -> f64 {
        self.hour as f64 + self.minute as f64 / 60.0 + self.second as f64 / 3600.0
    }
}

impl SwissEph {
    pub fn new() -> Self {
        let mut temp_file = NamedTempFile::new().unwrap();
        std::io::copy(&mut Cursor::new(EPHE_FILE), &mut temp_file).unwrap();
        
        INIT.call_once(|| {
            let file_path = temp_file.path().to_str().unwrap();
            let c_path = CString::new(file_path).unwrap();
            unsafe {
                swe_set_ephe_path(c_path.as_ptr() as *mut c_char);
            }
            eprintln!("Ephemeris file path: {}", file_path);
        });

        SwissEph { _temp_file: temp_file }
    }

    pub fn calculate(&self, coord_system: CoordinateSystem, date: &UtcDateTime, body: CelestialBody, flags: &[CalculationFlag]) -> Result<AstronomicalResult, CalculationError> {
        let sidereal_mode = match coord_system {
            CoordinateSystem::Tropical => SE_SIDM_FAGAN_BRADLEY as i32,
            CoordinateSystem::Sidereal => SE_SIDM_LAHIRI as i32,
        };
    
        unsafe {
            swe_set_sid_mode(sidereal_mode, 0.0, 0.0);
        }
        
        let mut iflag: i32 = if coord_system == CoordinateSystem::Sidereal { SEFLG_SIDEREAL as i32 } else { 0 };
        for flag in flags {
            iflag |= *flag as i32;
        }

        let julian_day = date.to_julian_day();

        let mut results: [f64; 6] = [0.0; 6];
        let mut error: [c_char; 256] = [0; 256];

        let calc_result = match body {
            CelestialBody::EclipticNutation => {
                let mut nut: [f64; 4] = [0.0; 4];
                unsafe {
                    swe_calc_ut(julian_day, SE_ECL_NUT as i32, 0, nut.as_mut_ptr(), error.as_mut_ptr());
                }
                Ok(AstronomicalResult::EclipticObliquity(EclipticObliquityInfo {
                    ecliptic_true_obliquity: nut[0],
                    ecliptic_mean_obliquity: nut[1],
                    nutation_longitude: nut[2],
                    nutation_obliquity: nut[3],
                }))
            },
            _ => {
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

                Ok(AstronomicalResult::CelestialBody(CelestialBodyInfo {
                    longitude: results[0],
                    latitude: results[1],
                    distance: results[2],
                    speed_longitude: results[3],
                    speed_latitude: results[4],
                    speed_distance: results[5],
                }))
            },
        };

        calc_result
    }

    pub fn get_body_name(&self, body: CelestialBody) -> String {
        let mut name: [c_char; 256] = [0; 256];
        unsafe {
            swe_get_planet_name(body as i32, name.as_mut_ptr());
        }
        unsafe { CStr::from_ptr(name.as_ptr()) }.to_string_lossy().into_owned()
    }

    pub fn calculate_houses(&self, coord_system: CoordinateSystem, date: &UtcDateTime, latitude: f64, longitude: f64) -> Result<Vec<HousePosition>, CalculationError> {
        let julian_day = date.to_julian_day();

        let mut cusps: [f64; 13] = [0.0; 13];
        let mut ascmc: [f64; 10] = [0.0; 10];

        let calc_result = match coord_system {
            CoordinateSystem::Tropical => unsafe {
                swe_houses(
                    julian_day,
                    latitude,
                    longitude,
                    'P' as i32,
                    cusps.as_mut_ptr(),
                    ascmc.as_mut_ptr(),
                )
            },
            CoordinateSystem::Sidereal => unsafe {
                swe_set_sid_mode(SE_SIDM_LAHIRI as i32, 0.0, 0.0);
                swe_houses_ex(
                    julian_day,
                    SEFLG_SIDEREAL as i32,
                    latitude,
                    longitude,
                    'P' as i32,
                    cusps.as_mut_ptr(),
                    ascmc.as_mut_ptr(),
                )
            },
        };

        if calc_result < 0 {
            return Err(CalculationError {
                code: calc_result,
                message: "Error calculating houses".to_string(),
            });
        }

        let house_positions: Vec<HousePosition> = (1..=12).map(|i| {
            HousePosition {
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
            }
        }).collect();

        Ok(house_positions)
    }

    fn get_zodiac_sign(longitude: f64) -> ZodiacSign {
        let normalized_longitude = longitude.rem_euclid(360.0);
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
    use approx::assert_relative_eq;

    #[test]
    fn test_sun_position() {
        let eph = SwissEph::new();
        let date = UtcDateTime::new(2023, 5, 17, 12, 0, 0);
        
        let result = eph.calculate(CoordinateSystem::Tropical, &date, CelestialBody::Sun, &[CalculationFlag::Speed]).unwrap();
        
        if let AstronomicalResult::CelestialBody(body_info) = result {
            assert!(body_info.longitude >= 0.0 && body_info.longitude < 360.0, "Longitude {} is out of range", body_info.longitude);
            assert!(body_info.latitude >= -90.0 && body_info.latitude <= 90.0, "Latitude {} is out of range", body_info.latitude);
            assert!(body_info.distance > 0.9 && body_info.distance < 1.1, "Sun distance {} is out of expected range", body_info.distance);
            
            assert!(body_info.speed_longitude.abs() < 1.0, "Sun longitude speed {} seems unreasonable", body_info.speed_longitude);
        } else {
            panic!("Unexpected result type: {:?}", result);
        }
    }

    #[test]
    fn test_house_positions() {
        let eph = SwissEph::new();
        let date = UtcDateTime::new(2023, 5, 17, 12, 0, 0);
        let latitude = 40.0;
        let longitude = 20.0;
        
        let result = eph.calculate_houses(CoordinateSystem::Tropical, &date, latitude, longitude).unwrap();
        
        assert_eq!(result.len(), 12);
    }

   

#[test]
fn test_moon_position() {
    let eph = SwissEph::new();
    let date = UtcDateTime::new(2023, 5, 17, 12, 0, 0);
    
    let result = eph.calculate(CoordinateSystem::Tropical, &date, CelestialBody::Moon, &[CalculationFlag::Speed]).unwrap();
    
    if let AstronomicalResult::CelestialBody(body_info) = result {
        assert!(body_info.longitude >= 0.0 && body_info.longitude < 360.0, "Longitude {} is out of range", body_info.longitude);
        assert!(body_info.latitude >= -90.0 && body_info.latitude <= 90.0, "Latitude {} is out of range", body_info.latitude);
        assert!(body_info.distance > 0.002 && body_info.distance < 0.003, "Moon distance {} is out of expected range", body_info.distance);
        
        assert!(body_info.speed_longitude.abs() < 15.0, "Moon longitude speed {} seems unreasonable", body_info.speed_longitude);
        assert!(body_info.speed_latitude.abs() < 10.0, "Moon latitude speed {} seems unreasonable", body_info.speed_latitude);
        assert!(body_info.speed_distance.abs() < 0.01, "Moon distance speed {} seems unreasonable", body_info.speed_distance);
    } else {
        panic!("Expected CelestialBodyInfo for Moon");
    }
}

#[test]
fn test_ecliptic_obliquity() {
    let eph = SwissEph::new();
    let date = UtcDateTime::new(2023, 5, 17, 12, 0, 0);
    
    let result = eph.calculate(CoordinateSystem::Tropical, &date, CelestialBody::EclipticNutation, &[]).unwrap();
    
    if let AstronomicalResult::EclipticObliquity(info) = result {
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
    let date = UtcDateTime::new(2023, 5, 17, 12, 0, 0);
    
    let tropical_result = eph.calculate(CoordinateSystem::Tropical, &date, CelestialBody::Sun, &[]).unwrap();
    let sidereal_result = eph.calculate(CoordinateSystem::Sidereal, &date, CelestialBody::Sun, &[]).unwrap();
    
    if let (AstronomicalResult::CelestialBody(tropical), AstronomicalResult::CelestialBody(sidereal)) = (tropical_result, sidereal_result) {
        assert!(
            (tropical.longitude - sidereal.longitude).abs() > 20.0,
            "Tropical and sidereal longitudes should differ significantly. Tropical: {}, Sidereal: {}",
            tropical.longitude,
            sidereal.longitude
        );
        assert_relative_eq!(tropical.latitude, sidereal.latitude, epsilon = 1e-6);
        assert_relative_eq!(tropical.distance, sidereal.distance, epsilon = 1e-6);
    } else {
        panic!("Expected CelestialBodyInfo for both calculations");
    }
}

#[test]
fn test_house_calculation() {
    let eph = SwissEph::new();
    let date = UtcDateTime::new(2023, 5, 17, 12, 0, 0);
    let latitude = 40.7128; // New York City latitude
    let longitude = -74.0060; // New York City longitude

    let houses = eph.calculate_houses(CoordinateSystem::Tropical, &date, latitude, longitude).unwrap();

    assert_eq!(houses.len(), 12, "There should be 12 houses");

 

    // Check if degrees are within valid range
    for house in &houses {
        assert!(house.degree >= 0.0 && house.degree < 30.0, "House degree {} is out of range", house.degree);
    }
}

#[test]
fn test_utc_date_time() {
    let date = UtcDateTime::new(2023, 5, 17, 12, 30, 45);
    assert_eq!(date.year, 2023);
    assert_eq!(date.month, 5);
    assert_eq!(date.day, 17);
    assert_eq!(date.hour, 12);
    assert_eq!(date.minute, 30);
    assert_eq!(date.second, 45);

    let julian_day = date.to_julian_day();
    let reconstructed_date = UtcDateTime::from_julian_day(julian_day);
    assert_eq!(date.year, reconstructed_date.year);
    assert_eq!(date.month, reconstructed_date.month);
    assert_eq!(date.day, reconstructed_date.day);
    assert_eq!(date.hour, reconstructed_date.hour);
    assert_eq!(date.minute, reconstructed_date.minute);
    assert_eq!(date.second, reconstructed_date.second);
}

#[test]
fn test_get_body_name() {
    let eph = SwissEph::new();
    assert_eq!(eph.get_body_name(CelestialBody::Sun), "Sun");
    assert_eq!(eph.get_body_name(CelestialBody::Moon), "Moon");
    assert_eq!(eph.get_body_name(CelestialBody::Mercury), "Mercury");
}
}

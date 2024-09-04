use std::fmt;
use std::str;
use std::sync::Mutex;
use std::{path::Path, ptr::null_mut};
use std::env;

use minimo::*;

const MAXCH: usize = 256;
static EPHE_PATH: Mutex<Option<String>> = Mutex::new(None);
static CLOSED: Mutex<bool> = Mutex::new(false);

// Swiss Ephemeris functions
extern "C" {
    fn swe_set_ephe_path(path: *const i8);
    fn swe_close();
    fn swe_version(s: *mut i8) -> *const i8;
    fn swe_get_library_path(s: *mut i8) -> *const i8;
    fn swe_set_jpl_file(fname: *const i8);
    fn swe_get_current_file_data(ifno: i32, tfstart: *mut f64, tfend: *mut f64, denum: *mut i32) -> *const i8;
    fn swe_calc_ut(tjd_ut: f64, ipl: i32, iflag: i32, xx: *mut f64, serr: *mut i8) -> i32;
    fn swe_julday(year: i32, month: i32, day: i32, hour: f64, gregflag: i32) -> f64;
    fn swe_get_planet_name(ipl: i32, spname: *mut i8) -> *const i8;
}

const SE_GREG_CAL: i32 = 1;

macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name[..name.len() - 3]
    }};
}

fn assert_ephe_ready(fn_name: &str) {
    assert!(
        !*CLOSED.lock().unwrap(),
        "Invoked temporal_ephemeris function {} after closing the ephemeris files.",
        fn_name
    );
    assert!(
        EPHE_PATH.lock().unwrap().is_some(),
        "Invoked temporal_ephemeris function {} before calling set_ephe_path.",
        fn_name
    );
}

pub struct FileData {
    pub filepath: String,
    pub start_date: f64,
    pub end_date: f64,
    pub ephemeris_num: i32,
}

#[repr(i32)]
#[derive(Debug, PartialEq, Copy, Clone)]
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

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Flag {
    JPLEphemeris = 1,
    SwissEphemeris = 2,
    MoshierEphemeris = 4,
    HeliocentricPos = 8,
    TruePos = 16,
    NoPrecession = 32,
    NoNutation = 64,
    HighPrecSpeed = 256,
    NoGravDeflection = 512,
    NoAnnualAbberation = 1024,
    AstrometricPos = 1536,
    EquatorialPos = 2048,
    CartesianCoords = 4096,
    Radians = 8192,
    BarycentricPos = 16384,
    TopocentricPos = 32 * 1024,
    Sideral = 64 * 1024,
    ICRS = 128 * 1024,
    JPLHorizons = 256 * 1024,
    JPLHorizonsApprox = 512 * 1024,
    CenterOfBody = 1024 * 1024,
}

#[derive(Debug)]
pub struct BodyResult {
    pub pos: Vec<f64>,
    pub vel: Vec<f64>,
}

#[derive(Debug)]
pub struct EclipticAndNutationResult {
    pub ecliptic_true_obliquity: f64,
    pub ecliptic_mean_obliquity: f64,
    pub nutation_lng: f64,
    pub nutation_obliquity: f64,
}

#[derive(Debug)]
pub enum CalculationResult {
    Body(BodyResult),
    EclipticAndNutation(EclipticAndNutationResult),
}

#[derive(Debug)]
pub struct CalculationError {
    code: i32,
    msg: String,
}

impl fmt::Display for CalculationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CalculationError {{ code: {} message: {} }}",
            self.code, self.msg
        )
    }
}

pub fn set_ephe_path(path: Option<&str>) {
    let mut closed = CLOSED.lock().unwrap();
    if *closed {
        return;
    }

    let mut ephe_path = EPHE_PATH.lock().unwrap();
    if ephe_path.is_some() {
        return;
    }

    match path {
        Some(path_str) => {
            assert!(path_str.len() < MAXCH);
            let path_p = Path::new(path_str);
            
            #[cfg(not(test))]
            assert!(path_p.is_dir());

            let mpath = path_str.to_owned();
            unsafe {
                swe_set_ephe_path(mpath.as_ptr() as *const i8);
            }
            *ephe_path = Some(mpath);
        }
        None => unsafe {
            swe_set_ephe_path(null_mut());
            *ephe_path = Some(String::new());
        },
    }
}

pub fn set_jpl_file(filename: &str) {
    assert_ephe_ready(function!());

    let env_ephe_path = env::var("SE_EPHE_PATH").ok();
    assert!(filename.len() < MAXCH);
    let path = match env_ephe_path {
        Some(path_str) => Path::new(&path_str).join(filename),
        None => {
            let ephe_path = EPHE_PATH.lock().unwrap();
            Path::new(ephe_path.as_ref().unwrap()).join(filename)
        },
    };
    #[cfg(not(test))]
    assert!(path.is_file());
    let mfilename = filename.to_owned();
    unsafe {
        swe_set_jpl_file(mfilename.as_ptr() as *const i8);
    }
}

pub fn close() {
    let mut closed = CLOSED.lock().unwrap();
    if !*closed {
        unsafe { swe_close() };
        *closed = true;
    }
}

pub fn version() -> String {
    assert_ephe_ready(function!());
    let mut swe_vers_i: [u8; MAXCH] = [0; MAXCH];
    unsafe {
        swe_version(swe_vers_i.as_mut_ptr() as *mut i8);
    }
    String::from(str::from_utf8(&swe_vers_i).unwrap().trim_end_matches('\0'))
}

pub fn get_library_path() -> String {
    assert_ephe_ready(function!());
    let mut swe_lp_i: [u8; MAXCH] = [0; MAXCH];
    unsafe {
        swe_get_library_path(swe_lp_i.as_mut_ptr() as *mut i8);
    }
    String::from(str::from_utf8(&swe_lp_i).unwrap().trim_end_matches('\0'))
}

pub fn get_current_file_data(ifno: i32) -> FileData {
    assert_ephe_ready(function!());
    let mut tfstart: f64 = 0.0;
    let mut tfend: f64 = 0.0;
    let mut denum: i32 = 0;
    let mut filepath = String::with_capacity(MAXCH);

    let fp_i = unsafe {
        swe_get_current_file_data(
            ifno,
            &mut tfstart as *mut f64,
            &mut tfend as *mut f64,
            &mut denum as *mut i32,
        )
    } as *const u8;
    let mut fp_p = fp_i;
    let term = b'\0';
    while unsafe { *fp_p } != term {
        unsafe {
            let i = *fp_p;
            let i_slice = &[i as u8];
            let s = str::from_utf8(i_slice).unwrap();
            filepath.push_str(s);
            fp_p = fp_p.add(1);
        }
    }

    FileData {
        filepath,
        start_date: tfstart,
        end_date: tfend,
        ephemeris_num: denum,
    }
}

pub fn calc_ut(
    dt_with_location: &DateTimeWithLocation,
    body: Body,
    flag_set: &[Flag],
) -> Result<CalculationResult, CalculationError> {
    assert_ephe_ready(function!());
    
    let julian_day_ut = julday(&dt_with_location.datetime);
    
    let mut flags: i32 = 0;
    for f in flag_set.iter() {
        flags |= *f as i32;
    }
    let mut results: [f64; 6] = [0.0; 6];
    let mut error_i: [u8; MAXCH] = [0; MAXCH];
    let code = unsafe {
        swe_calc_ut(
            julian_day_ut,
            body as i32,
            flags,
            &mut results as *mut f64,
            error_i.as_mut_ptr() as *mut i8,
        )
    };
    let msg = String::from(str::from_utf8(&error_i).unwrap().trim_end_matches('\0'));
    if code < 0 {
        Err(CalculationError { code, msg })
    } else {
        match body {
            Body::EclipticNutation => Ok(CalculationResult::EclipticAndNutation(
                EclipticAndNutationResult {
                    ecliptic_true_obliquity: results[0],
                    ecliptic_mean_obliquity: results[1],
                    nutation_lng: results[2],
                    nutation_obliquity: results[3],
                },
            )),
            _ => Ok(CalculationResult::Body(BodyResult {
                pos: vec![results[0], results[1], results[2]],
                vel: vec![results[3], results[4], results[5]],
            })),
        }
    }
}

pub fn julday(dt: &DateTime) -> f64 {
    let hour = dt.hour() as f64 + dt.minute() as f64 / 60.0 + dt.second() as f64 / 3600.0;
    unsafe {
        swe_julday(
            dt.year(),
            dt.month() as i32,
            dt.day() as i32,
            hour,
            SE_GREG_CAL,
        )
    }
}

pub fn get_planet_name(body: Body) -> String {
    assert_ephe_ready(function!());
    let mut name: [u8; MAXCH] = [0; MAXCH];

    unsafe { swe_get_planet_name(body as i32, name.as_mut_ptr() as *mut i8) };
    String::from_utf8_lossy(&name).trim_end_matches('\0').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::fs;
    use std::path::PathBuf;

    fn setup() {
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_ephe");
        fs::create_dir_all(&test_dir).unwrap();
        set_ephe_path(Some(test_dir.to_str().unwrap()));
    }

    #[test]
    fn test_sun_position() {
        setup();
        let dt = DateTime::new(2023, 5, 17, 12, 0, 0);
        let location = Location::new("Test", 0.0, 0.0, 0);
        let dt_with_location = DateTimeWithLocation::new(dt, location);
        
        let result = calc_ut(&dt_with_location, Body::Sun, &[Flag::SwissEphemeris]).unwrap();
        
        if let CalculationResult::Body(body_result) = result {
            // These values are approximate and may need adjustment based on the exact ephemeris used
            assert_relative_eq!(body_result.pos[0], 56.3, epsilon = 0.1); // Longitude
            assert_relative_eq!(body_result.pos[1], 0.0, epsilon = 0.1);  // Latitude
            assert_relative_eq!(body_result.pos[2], 1.01, epsilon = 0.01); // Distance (AU)
        } else {
            panic!("Expected BodyResult for Sun");
        }
    }

    #[test]
    fn test_moon_position() {
        setup();
        let dt = DateTime::new(2023, 5, 17, 12, 0, 0);
        let location = Location::new("Test", 0.0, 0.0, 0);
        let dt_with_location = DateTimeWithLocation::new(dt, location);
        
        let result = calc_ut(&dt_with_location, Body::Moon, &[Flag::SwissEphemeris]).unwrap();
        
        if let CalculationResult::Body(body_result) = result {
            // These values are approximate and may need adjustment
            assert_relative_eq!(body_result.pos[0], 29.7, epsilon = 1.0); // Longitude
            assert_relative_eq!(body_result.pos[1], -0.3, epsilon = 1.0); // Latitude
            assert_relative_eq!(body_result.pos[2], 0.002, epsilon = 0.001); // Distance (AU)
        } else {
            panic!("Expected BodyResult for Moon");
        }
    }

    #[test]
    fn test_ecliptic_and_nutation() {
        setup();
        let dt = DateTime::new(2023, 5, 17, 12, 0, 0);
        let location = Location::new("Test", 0.0, 0.0, 0);
        let dt_with_location = DateTimeWithLocation::new(dt, location);
        
        let result = calc_ut(&dt_with_location, Body::EclipticNutation, &[Flag::SwissEphemeris]).unwrap();
        
        if let CalculationResult::EclipticAndNutation(ecliptic_result) = result {
            assert_relative_eq!(ecliptic_result.ecliptic_true_obliquity, 23.44, epsilon = 0.01);
            assert_relative_eq!(ecliptic_result.ecliptic_mean_obliquity, 23.44, epsilon = 0.01);
            // Nutation values are very small, so we use a smaller epsilon
            assert_relative_eq!(ecliptic_result.nutation_lng, -0.002, epsilon = 0.001);
            assert_relative_eq!(ecliptic_result.nutation_obliquity, 0.002, epsilon = 0.001);
        } else {
            panic!("Expected EclipticAndNutationResult");
        }
    }

    #[test]
    fn test_different_flags() {
        setup();
        let dt = DateTime::new(2023, 5, 17, 12, 0, 0);
        let location = Location::new("Test", 0.0, 0.0, 0);
        let dt_with_location = DateTimeWithLocation::new(dt, location);
        
        let result_swiss = calc_ut(&dt_with_location, Body::Mars, &[Flag::SwissEphemeris]).unwrap();
        let result_moshier = calc_ut(&dt_with_location, Body::Mars, &[Flag::MoshierEphemeris]).unwrap();
        
        // Results should be close but not identical
        if let (CalculationResult::Body(swiss), CalculationResult::Body(moshier)) = (result_swiss, result_moshier) {
            assert_relative_eq!(swiss.pos[0], moshier.pos[0], epsilon = 0.1);
            assert_relative_eq!(swiss.pos[1], moshier.pos[1], epsilon = 0.1);
            assert_relative_eq!(swiss.pos[2], moshier.pos[2], epsilon = 0.01);
        } else {
            panic!("Expected BodyResult for both calculations");
        }
    }

    #[test]
    fn test_heliocentric_position() {
        setup();
        let dt = DateTime::new(2023, 5, 17, 12, 0, 0);
        let location = Location::new("Test", 0.0, 0.0, 0);
        let dt_with_location = DateTimeWithLocation::new(dt, location);
        
        let result = calc_ut(&dt_with_location, Body::Earth, &[Flag::SwissEphemeris, Flag::HeliocentricPos]).unwrap();
        
        if let CalculationResult::Body(body_result) = result {
            // Earth's heliocentric position should be close to 1 AU from the Sun
            assert_relative_eq!(body_result.pos[2], 1.0, epsilon = 0.1);
        } else {
            panic!("Expected BodyResult for Earth");
        }
    }

 

    #[test]
    fn test_different_dates() {
        setup();
        let location = Location::new("Test", 0.0, 0.0, 0);
        
        let dt1 = DateTime::new(2000, 1, 1, 0, 0, 0);
        let dt2 = DateTime::new(2050, 12, 31, 23, 59, 59);
        
        let result1 = calc_ut(&DateTimeWithLocation::new(dt1, location.clone()), Body::Jupiter, &[Flag::SwissEphemeris]).unwrap();
        let result2 = calc_ut(&DateTimeWithLocation::new(dt2, location), Body::Jupiter, &[Flag::SwissEphemeris]).unwrap();
        
        if let (CalculationResult::Body(body1), CalculationResult::Body(body2)) = (result1, result2) {
            // Positions should be different due to the large time difference
            assert!((body1.pos[0] - body2.pos[0]).abs() > 1.0);
        } else {
            panic!("Expected BodyResult for both calculations");
        }
    }
}
// src/main.rs

use chrono::{DateTime, Datelike, Duration as ChronoDuration, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt;
use std::io::Cursor;
use std::os::raw::{c_char, c_double, c_int};
use std::sync::Once;
use tempfile::NamedTempFile;

// ---------------------------
// ## Enumerations
// ---------------------------

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CalculationFlag {
    Speed = SEFLG_SPEED as isize,
    NoGravitationalDeflection = 512,
    NoAnnualAberration = 1024,
    Topocentric = 32768,
    Equatorial = 2048,
    XYZ = 8192,
    Radians = 16384,
    Barycentric = 4,
    Heliocentric = 8,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CoordinateSystem {
    Tropical,
    Sidereal,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CelestialBody {
    Sun = 0,
    Moon = 1,
    Mercury = 2,
    Venus = 3,
    Mars = 4,
    Jupiter = 5,
    Saturn = 6,
    Rahu = 11,
    Ketu = 999,
}

impl CelestialBody {
    fn iter() -> impl Iterator<Item = CelestialBody> {
        [
            CelestialBody::Sun,
            CelestialBody::Moon,
            CelestialBody::Mercury,
            CelestialBody::Venus,
            CelestialBody::Mars,
            CelestialBody::Jupiter,
            CelestialBody::Saturn,
            CelestialBody::Rahu,
            CelestialBody::Ketu,
        ]
        .iter()
        .copied()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum House {
    First = 1,
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

impl House {
    pub fn from_index(index: usize) -> Option<House> {
        match index {
            1 => Some(House::First),
            2 => Some(House::Second),
            3 => Some(House::Third),
            4 => Some(House::Fourth),
            5 => Some(House::Fifth),
            6 => Some(House::Sixth),
            7 => Some(House::Seventh),
            8 => Some(House::Eighth),
            9 => Some(House::Ninth),
            10 => Some(House::Tenth),
            11 => Some(House::Eleventh),
            12 => Some(House::Twelfth),
            _ => None,
        }
    }
    
    pub fn all() -> impl Iterator<Item = House> {
        (1..=12).map(House::from_index).flatten()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ZodiacSign {
    Aries = 0,
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

impl ZodiacSign {
    pub fn from_longitude(longitude: f64) -> Self {
        let normalized_longitude = longitude.rem_euclid(360.0);
        let sign_index = (normalized_longitude / 30.0).floor() as usize;
        match sign_index {
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
            _ => ZodiacSign::Aries, // Fallback
        }
    }
}

impl fmt::Display for ZodiacSign {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sign_str = match self {
            ZodiacSign::Aries => "Aries",
            ZodiacSign::Taurus => "Taurus",
            ZodiacSign::Gemini => "Gemini",
            ZodiacSign::Cancer => "Cancer",
            ZodiacSign::Leo => "Leo",
            ZodiacSign::Virgo => "Virgo",
            ZodiacSign::Libra => "Libra",
            ZodiacSign::Scorpio => "Scorpio",
            ZodiacSign::Sagittarius => "Sagittarius",
            ZodiacSign::Capricorn => "Capricorn",
            ZodiacSign::Aquarius => "Aquarius",
            ZodiacSign::Pisces => "Pisces",
        };
        write!(f, "{}", sign_str)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Nakshatra {
    Ashwini,
    Bharani,
    Krittika,
    Rohini,
    Mrigashira,
    Ardra,
    Punarvasu,
    Pushya,
    Ashlesha,
    Magha,
    PurvaPhalguni,
    UttaraPhalguni,
    Hasta,
    Chitra,
    Swati,
    Vishakha,
    Anuradha,
    Jyeshtha,
    Moola,
    PurvaAshadha,
    UttaraAshadha,
    Shravana,
    Dhanishta,
    Shatabhisha,
    PurvaBhadrapada,
    UttaraBhadrapada,
    Revati,
}

impl Nakshatra {
    pub fn from_longitude(longitude: f64) -> Nakshatra {
        let normalized_longitude = longitude.rem_euclid(360.0);
        let nakshatra_index = (normalized_longitude / 13.333333333333334).floor() as usize;
        match nakshatra_index {
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
            _ => Nakshatra::Ashwini, // Fallback
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Dasha {
    Ketu,
    Venus,
    Sun,
    Moon,
    Mars,
    Rahu,
    Jupiter,
    Saturn,
    Mercury,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PlanetaryState {
    Exalted,
    DeepExaltation,
    Moolatrikona,
    OwnSign,
    GreatFriend,
    Friend,
    Neutral,
    Enemy,
    GreatEnemy,
    Debilitated,
    DeepDebilitation,
    Combust,
    Retrograde,
    Direct,
    Benefic,
    Malefic,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ChartType {
    Rasi,
    Navamsa,
    Hora,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SpecialLagna {
    Bhava,
    Hora,
    Ghati,
    Varnada,
    Sree,
    Pranapada,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Upagraha {
    Dhuma,
    Vyatipata,
    Parivesha,
    Indrachaapa,
    Upaketu,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SensitivePoint {
    Gulika,
    Mandi,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Aspect {
    Conjunction,
    Opposition,
    Trine,
    Square,
    Sextile,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Trait {
    Health,
    Wealth,
    Relationship,
    Career,
    Family,
    Children,
    Travel,
    Education,
    Communication,
    Spirituality,
    Creativity,
    Leadership,
    Intellect,
    Emotions,
    Longevity,
    Fame,
    Power,
    Luck,
    Happiness,
    Courage,
    Wisdom,
    Intuition,
    Ambition,
    Discipline,
    Adaptability,
    Independence,
    Compassion,
    Resilience,
    Honesty,
    Generosity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum On {
    Oneself,
    Spouse,
    Husband,
    Wife,
    Children,
    Son,
    Daughter,
    Parents,
    Father,
    Mother,
    InLaws,
    MotherInLaw,
    FatherInLaw,
    BrotherInLaw,
    SisterInLaw,
    Siblings,
    Brother,
    Sister,
    Friends,
    Enemies,
    Teacher,
    Student,
    Society,
}

// ---------------------------
// ## Structures
// ---------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CelestialCoordinates {
    pub longitude: f64,
    pub latitude: f64,
    pub distance: f64,
    pub speed_longitude: f64,
    pub speed_latitude: f64,
    pub speed_distance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HousePosition {
    pub house: House,
    pub sign: ZodiacSign,
    pub degree: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalculationError {
    pub code: i32,
    pub message: String,
}

pub type JulianDay = f64;

#[derive(Debug, Clone, PartialEq)]
pub struct NakshatraInfo {
    pub nakshatra: Nakshatra,
    pub pada: u8,
    pub lord: CelestialBody,
    pub degree: f64,
}

impl NakshatraInfo {
    pub fn from_longitude(longitude: f64) -> NakshatraInfo {
        let normalized_longitude = longitude.rem_euclid(360.0);
        let nakshatra = Nakshatra::from_longitude(normalized_longitude);
        let pada_length = 13.333333333333334 / 4.0;
        let pada = ((normalized_longitude % 13.333333333333334) / pada_length).floor() as u8 + 1;
        let lord = NakshatraInfo::get_nakshatra_lord(nakshatra);
        NakshatraInfo {
            nakshatra,
            pada,
            lord,
            degree: normalized_longitude,
        }
    }

    pub fn get_nakshatra_lord(nakshatra: Nakshatra) -> CelestialBody {
        match nakshatra {
            Nakshatra::Ashwini => CelestialBody::Ketu,
            Nakshatra::Bharani => CelestialBody::Venus,
            Nakshatra::Krittika => CelestialBody::Sun,
            Nakshatra::Rohini => CelestialBody::Moon,
            Nakshatra::Mrigashira => CelestialBody::Mars,
            Nakshatra::Ardra => CelestialBody::Saturn,
            Nakshatra::Punarvasu => CelestialBody::Mercury,
            Nakshatra::Pushya => CelestialBody::Venus,
            Nakshatra::Ashlesha => CelestialBody::Sun,
            Nakshatra::Magha => CelestialBody::Moon,
            Nakshatra::PurvaPhalguni => CelestialBody::Mars,
            Nakshatra::UttaraPhalguni => CelestialBody::Rahu,
            Nakshatra::Hasta => CelestialBody::Jupiter,
            Nakshatra::Chitra => CelestialBody::Saturn,
            Nakshatra::Swati => CelestialBody::Mercury,
            Nakshatra::Vishakha => CelestialBody::Venus,
            Nakshatra::Anuradha => CelestialBody::Sun,
            Nakshatra::Jyeshtha => CelestialBody::Moon,
            Nakshatra::Moola => CelestialBody::Mars,
            Nakshatra::PurvaAshadha => CelestialBody::Rahu,
            Nakshatra::UttaraAshadha => CelestialBody::Jupiter,
            Nakshatra::Shravana => CelestialBody::Saturn,
            Nakshatra::Dhanishta => CelestialBody::Mercury,
            Nakshatra::Shatabhisha => CelestialBody::Venus,
            Nakshatra::PurvaBhadrapada => CelestialBody::Rahu,
            Nakshatra::UttaraBhadrapada => CelestialBody::Jupiter,
            Nakshatra::Revati => CelestialBody::Saturn,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DashaInfo {
    pub maha_dasha: Dasha,
    pub maha_dasha_start: DateTime<Utc>,
    pub maha_dasha_end: DateTime<Utc>,
    pub antar_dasha: Dasha,
    pub antar_dasha_start: DateTime<Utc>,
    pub antar_dasha_end: DateTime<Utc>,
    pub pratyantar_dasha: Dasha,
    pub pratyantar_dasha_start: DateTime<Utc>,
    pub pratyantar_dasha_end: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Yoga {
    pub name: String,
    pub condition: Condition,
    pub effects: Effects,
    pub strength: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    pub description: String,
    pub check: fn(chart: &ChartInfo) -> bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Effects {
    pub description: String,
    pub apply: fn(chart: &ChartInfo) -> Impact,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Impact {
    Positive(On, Trait, f64),
    Negative(On, Trait, f64),
    Neutral(On, Trait, f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct YogaInfo {
    pub yoga: Yoga,
    pub strength: f64,
    pub involved_planets: Vec<CelestialBody>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AyanamsaInfo {
    pub ayanamsa_name: String,
    pub ayanamsa_value: f64,
}

impl AyanamsaInfo {
    pub fn calculate(julian_day: JulianDay) -> Self {
        // Actual calculation using FFI bindings
        calculate_ayanamsa(julian_day)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChartInfo {
    pub chart_type: ChartType,
    pub ascendant: HousePosition,
    pub houses: Vec<HousePosition>,
    pub planets: Vec<PlanetPosition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanetPosition {
    pub planet: CelestialBody,
    pub longitude: f64,
    pub latitude: f64,
    pub speed: f64,
    pub sign: ZodiacSign,
    pub house: House,
    pub nakshatra: NakshatraInfo,
    pub retrograde: bool,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub birth_info: BirthInfo,
    pub ayanamsa: AyanamsaInfo,
    pub charts: Vec<ChartInfo>,
    pub dashas: DashaInfo,
    pub yogas: Vec<YogaInfo>,
    pub nakshatras: Vec<NakshatraInfo>,
    pub planetary_states: HashMap<CelestialBody, PlanetaryState>,
    pub divisional_charts: Vec<DivisionalChart>,
    pub special_lagnas: HashMap<SpecialLagna, f64>,
    pub upagrahas: HashMap<Upagraha, f64>,
    pub sensitive_points: HashMap<SensitivePoint, f64>,
    pub strengths: HashMap<CelestialBody, StrengthInfo>,
    pub remedial_measures: Vec<RemedialMeasure>,
}

impl Report {
    pub fn calculate(birth_info: &BirthInfo, ephemeris: &SwissEph) -> Result<Self, CalculationError> {
        // Calculate the ayanamsa
        let ayanamsa = AyanamsaInfo::calculate(date_to_julian_day(birth_info.date_time));

        // Calculate the chart
        let chart = ephemeris.calculate_chart(birth_info)?;

        // Calculate the dashas
        let dashas = ephemeris.calculate_dasha(birth_info)?;

        // Calculate the yogas
        let yogas = ephemeris.calculate_yogas(&chart);

        // Calculate the nakshatras
        let nakshatras = ephemeris.calculate_nakshatras(&chart);

        // Calculate the planetary states
        let planetary_states = ephemeris.calculate_planetary_states(&chart)?;

        // Calculate the divisional charts
        let divisional_charts = ephemeris.calculate_divisional_charts(&chart);

        // Calculate special lagnas
        let special_lagnas = ephemeris.calculate_special_lagnas(&chart);

        // Calculate upagrahas
        // let upagrahas = ephemeris.calculate_upagrahas(&chart);
        let upagrahas = HashMap::new();

        // Calculate sensitive points
        // let sensitive_points = ephemeris.calculate_sensitive_points(&chart);
        let sensitive_points = HashMap::new();
        // Calculate strengths
        // let strengths = ephemeris.calculate_strengths(&chart);
        let strengths = HashMap::new();
        // Calculate remedial measures
        let remedial_measures = ephemeris.suggest_remedial_measures(&chart);

        Ok(Self {
            birth_info: birth_info.clone(),
            ayanamsa,
            charts: vec![chart],
            dashas,
            yogas,
            nakshatras,
            planetary_states,
            divisional_charts,
            special_lagnas,
            upagrahas,
            sensitive_points,
            strengths,
            remedial_measures,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BirthInfo {
    pub date_time: DateTime<Utc>,
    pub location: Location,
}

impl BirthInfo {
    pub fn generate_report(&self) -> Result<Report, CalculationError> {
        if let Ok(eph) = SwissEph::new() {
            Report::calculate(&self, &eph)
        } else {
            Err(CalculationError {
                code: -1,
                message: "Failed to initialize Swiss Ephemeris".to_string(),
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}

impl Location {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Location { latitude, longitude }
    }

    pub fn delhi() -> Self { Location { latitude: 28.6139, longitude: 77.2090 } }
    pub fn mumbai() -> Self { Location { latitude: 19.0760, longitude: 72.8777 } }
    pub fn bangalore() -> Self { Location { latitude: 12.9716, longitude: 77.5946 } }
    pub fn chennai() -> Self { Location { latitude: 13.0827, longitude: 80.2707 } }
    pub fn kannur() -> Self { Location { latitude: 11.8740, longitude: 75.3600 } }
    pub fn kolkata() -> Self { Location { latitude: 22.5052, longitude: 87.3616 } }
    pub fn abu_dhabi() -> Self { Location { latitude: 24.4667, longitude: 54.3667 } }
    pub fn dubai() -> Self { Location { latitude: 25.276987, longitude: 55.296234 } }
    pub fn sharjah() -> Self { Location { latitude: 25.3550, longitude: 55.4000 } }
    pub fn malappuram() -> Self { Location { latitude: 10.7900, longitude: 76.0700 } }
    pub fn kochi() -> Self { Location { latitude: 9.9312, longitude: 76.2673 } }
    pub fn kollam() -> Self { Location { latitude: 8.8857, longitude: 76.5881 } }
    pub fn thrissur() -> Self { Location { latitude: 10.522, longitude: 76.2100 } }
    pub fn kozhikode() -> Self { Location { latitude: 11.2588, longitude: 75.7804 } }
    pub fn wayanad() -> Self { Location { latitude: 11.6900, longitude: 75.8900 } }
    pub fn munnar() -> Self { Location { latitude: 10.0000, longitude: 77.0667 } }
    pub fn idukki() -> Self { Location { latitude: 10.0000, longitude: 77.0667 } }
    pub fn kottayam() -> Self { Location { latitude: 10.0000, longitude: 76.5000 } }
    pub fn alappuzha() -> Self { Location { latitude: 9.4900, longitude: 76.3200 } }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemedialMeasure {
    pub description: String,
    pub gemstone: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrengthInfo {
    pub shad_bala: f64,
    pub ashtaka_varga: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DignityInfo {
    pub moolatrikona: bool,
    pub own_sign: bool,
    pub exalted: bool,
    pub debilitated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BhavaInfo {
    pub bhava: House,
    pub sign: ZodiacSign,
    pub degree: f64,
    pub lord: CelestialBody,
    pub planets: Vec<CelestialBody>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransitInfo {
    pub planet: CelestialBody,
    pub from_sign: ZodiacSign,
    pub to_sign: ZodiacSign,
    pub date: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarshaphalInfo {
    pub year: i32,
    pub ascendant: ZodiacSign,
    pub planets: Vec<PlanetPosition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompatibilityInfo {
    pub kuta_points: u32,
    pub compatibility_score: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DivisionalChart {
    pub chart_type: ChartType,
    pub ascendant: ZodiacSign,
    pub houses: [ZodiacSign; 12],
    pub planets: Vec<PlanetPosition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AspectInfo {
    pub aspect: Aspect,
    pub planet1: CelestialBody,
    pub planet2: CelestialBody,
    pub orb: f64,
}

// ---------------------------
// ## Error Handling
// ---------------------------

#[derive(Debug)]
pub enum AstrologyError {
    CalculationError(CalculationError),
    EphemerisError(String),
    InvalidInput(String),
    UnknownError(String),
}

impl fmt::Display for AstrologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AstrologyError::CalculationError(err) => write!(f, "Calculation Error {}: {}", err.code, err.message),
            AstrologyError::EphemerisError(msg) => write!(f, "Ephemeris Error: {}", msg),
            AstrologyError::InvalidInput(msg) => write!(f, "Invalid Input: {}", msg),
            AstrologyError::UnknownError(msg) => write!(f, "Unknown Error: {}", msg),
        }
    }
}

impl Error for AstrologyError {}

// ---------------------------
// ## FFI Bindings for Swiss Ephemeris
// ---------------------------

mod bindings {
    use super::*;

    extern "C" {
        // Initialize and close Swiss Ephemeris
        pub fn swe_set_ephe_path(path: *const c_char) -> c_int;
        pub fn swe_set_sid_mode(
            sid_mode: c_int,
            t0: c_double,
            ayan_t0: c_double,
        ) -> c_int;
        pub fn swe_close();

        // Calculate planetary positions
        pub fn swe_calc_ut(
            tjd_ut: c_double,
            ipl: c_int,
            iflag: c_int,
            xx: *mut c_double,
            serr: *mut c_char,
        ) -> c_int;

        // House calculations
        pub fn swe_houses_ex(
            tjd_ut: c_double,
            iflag: c_int,
            geolat: c_double,
            geolon: c_double,
            hsys: c_int,
            cusps: *mut c_double,
            ascmc: *mut c_double,
        ) -> c_int;

        pub fn swe_house_pos(
            armc: c_double,
            geolat: c_double,
            eps: c_double,
            hsys: c_int,
            at: *mut c_double,
            serr: *mut c_char,
        ) -> c_double;

        // Get planet name
        pub fn swe_get_planet_name(ipl: c_int, name: *mut c_char);

        // Ayanamsa
        pub fn swe_get_ayanamsa_ut(tjd_ut: c_double) -> c_double;

        // Convert UTC to Julian Day
        pub fn swe_utc_to_jd(
            year: c_int,
            month: c_int,
            day: c_int,
            hour: c_int,
            minute: c_int,
            sec: c_double,
            gregflag: c_int,
            jdet: *mut c_double,
            jday: *mut c_double,
        ) -> c_int;

        // Convert Julian Day to UTC
        pub fn swe_jdut1_to_utc(
            tjd_ut: c_double,
            gregflag: c_int,
            year: *mut c_int,
            month: *mut c_int,
            day: *mut c_int,
            hour: *mut c_int,
            minute: *mut c_int,
            sec: *mut c_double,
        ) -> c_int;
    }
}

// Import bindings
use bindings::*;

// ---------------------------
// ## Constants for Swiss Ephemeris
// ---------------------------

pub const SE_GREG_CAL: c_int = 1;
pub const SE_SIDM_LAHIRI: c_int = 1;
pub const SE_SIDM_FAGAN_BRADLEY: c_int = 2;

// Flags for calculations
pub const SEFLG_SPEED: c_int = 256;
pub const SEFLG_SIDEREAL: c_int = 1 << 7;
pub const SEFLG_SWIEPH: c_int = 1 << 0;

// House system codes
pub const SE_HS_PLACIDUS: c_int = 0;
pub const SE_HS_KRISHNAMURTI: c_int = 10;
pub const SE_HS_BUCHAREST: c_int = 11;
pub const SE_HS_EQUATORIAL: c_int = 12;
pub const SE_HS_MERCURY: c_int = 13;
pub const SE_HS_CAMPANUS: c_int = 14;
pub const SE_HS_MORIN: c_int = 15;
pub const SE_HS_PORPHYRUS: c_int = 16;
pub const SE_HS_VEHRENBERG: c_int = 17;
pub const SE_HS_ALCABITUS: c_int = 18;
pub const SE_HS_TOPHRAS: c_int = 19;
pub const SE_HS_NAVAMSA: c_int = 20;
pub const SE_HS_HORA: c_int = 21;

// ---------------------------
// ## SwissEph Structure
// ---------------------------

pub struct SwissEph {
    _temp_file: NamedTempFile,
}

static EPHE_FILE: &[u8] = include_bytes!("../ephe/sepl_18.se1"); // Ensure the ephemeris file is in ../ephe/
static INIT: Once = Once::new();

impl SwissEph {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let mut temp_file = NamedTempFile::new()?;
        std::io::copy(&mut Cursor::new(EPHE_FILE), &mut temp_file)?;

        // Set ephemeris path once
        INIT.call_once(|| {
            let file_path = temp_file.path().to_str().unwrap();
            let c_path = CString::new(file_path).unwrap();
            unsafe {
                swe_set_ephe_path(c_path.as_ptr());
            }
            eprintln!("Ephemeris file path set to: {}", file_path);
        });

        Ok(SwissEph {
            _temp_file: temp_file,
        })
    }

    pub fn get_house(
        &self,
        julian_day: JulianDay,
        planet_longitude: f64,
        latitude: f64,
        longitude: f64,
        house_system: ChartType,
    ) -> Result<House, CalculationError> {
        let hsys = match house_system {
            ChartType::Rasi => SE_HS_PLACIDUS, // Placidus
            ChartType::Navamsa => SE_HS_NAVAMSA, // Navamsa
            ChartType::Hora => SE_HS_HORA, // Hora
            // Add other house systems as needed
        };

        let mut cusps: [c_double; 13] = [0.0; 13];
        let mut ascmc: [c_double; 10] = [0.0; 10];

        let hsys_code = hsys;

        let flag = 0; // Additional flags can be set here

        let calc_result = unsafe {
            swe_houses_ex(
                julian_day,
                flag,
                latitude,
                longitude,
                hsys_code,
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

        let armc = ascmc[0];
        let eps = ascmc[1];

        let mut serr: [c_char; 256] = [0; 256];
        let house_position = unsafe {
            swe_house_pos(
                armc,
                latitude,
                eps,
                hsys_code,
                &planet_longitude as *const f64 as *mut f64,
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
        Ok(match house_number {
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
        })
    }

    pub fn calculate_ayanamsa(&self, julian_day: JulianDay) -> f64 {
        unsafe { swe_get_ayanamsa_ut(julian_day) }
    }

    pub fn calculate_navamsa(&self, longitude: f64) -> f64 {
        let normalized_longitude = longitude.rem_euclid(360.0);
        let navamsa_longitude = (normalized_longitude / 3.0).rem_euclid(360.0);
        navamsa_longitude
    }

    pub fn calculate_nakshatra(&self, longitude: f64) -> NakshatraInfo {
        NakshatraInfo::from_longitude(longitude)
    }

    pub fn calculate_nakshatras(&self, chart_info: &ChartInfo) -> Vec<NakshatraInfo> {
        chart_info.planets.iter().map(|planet| self.calculate_nakshatra(planet.longitude)).collect()
    }

    pub fn get_nakshatra_lord(&self, nakshatra: Nakshatra) -> CelestialBody {
        NakshatraInfo::get_nakshatra_lord(nakshatra)
    }
    pub fn calculate_dasha(&self, birth_info: &BirthInfo) -> Result<DashaInfo, CalculationError> {
        let julian_day = date_to_julian_day(birth_info.date_time);
        let result = self.calculate(
            CoordinateSystem::Sidereal,
            julian_day,
            CelestialBody::Moon,
            &[CalculationFlag::Speed],
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

        let dasha_sequence = [
            Dasha::Ketu,
            Dasha::Venus,
            Dasha::Sun,
            Dasha::Moon,
            Dasha::Mars,
            Dasha::Rahu,
            Dasha::Jupiter,
            Dasha::Saturn,
            Dasha::Mercury,
        ];

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

        let position_in_nakshatra = moon_longitude % 13.333333333333334;
        let nakshatra_fraction = position_in_nakshatra / 13.333333333333334;

        let total_dasha_years = dasha_years
            .iter()
            .find(|&&(dasha, _)| dasha == starting_dasha)
            .map(|&(_, years)| years)
            .unwrap_or(0.0);

        let dasha_balance_years = total_dasha_years * (1.0 - nakshatra_fraction);

        let mut maha_dasha_periods = Vec::new();
        let mut index = dasha_sequence
            .iter()
            .position(|&dasha| dasha == starting_dasha)
            .unwrap_or(0);

        let mut maha_dasha_start = birth_info.date_time;

        let mut total_years = dasha_balance_years;

        // First Dasha
        {
            let current_dasha = dasha_sequence[index];
            let years = dasha_balance_years;
            let maha_dasha_end = maha_dasha_start
                + ChronoDuration::seconds((years * 365.25 * 86400.0) as i64);
            maha_dasha_periods.push((current_dasha, maha_dasha_start, maha_dasha_end));
            maha_dasha_start = maha_dasha_end;
            index = (index + 1) % dasha_sequence.len();
        }

        while total_years < 120.0 {
            let current_dasha = dasha_sequence[index];
            let years = dasha_years
                .iter()
                .find(|&&(dasha, _)| dasha == current_dasha)
                .map(|&(_, years)| years)
                .unwrap_or(0.0);

            let maha_dasha_end =
                maha_dasha_start + ChronoDuration::seconds((years * 365.25 * 86400.0) as i64);
            maha_dasha_periods.push((current_dasha, maha_dasha_start, maha_dasha_end));
            maha_dasha_start = maha_dasha_end;

            total_years += years;
            index = (index + 1) % dasha_sequence.len();
        }

        let now = Utc::now();
        let current_maha_dasha = maha_dasha_periods
            .iter()
            .find(|&&(_, start, end)| now >= start && now < end)
            .unwrap_or(&maha_dasha_periods[0]);

        let (maha_dasha, maha_dasha_start, maha_dasha_end) = *current_maha_dasha;

        // Antar Dasha Calculation
        let maha_dasha_duration = (maha_dasha_end - maha_dasha_start).num_seconds() as f64;

        let mut antar_dasha_start = maha_dasha_start;
        let mut antar_dasha_periods = Vec::new();

        for &antar_dasha in &dasha_sequence {
            let antar_dasha_years = dasha_years
                .iter()
                .find(|&&(dasha, _)| dasha == antar_dasha)
                .map(|&(_, years)| years)
                .unwrap_or(0.0);

            let antar_dasha_duration = maha_dasha_duration * (antar_dasha_years / 120.0);

            let antar_dasha_end = antar_dasha_start
                + ChronoDuration::seconds(antar_dasha_duration as i64);

            antar_dasha_periods.push((antar_dasha, antar_dasha_start, antar_dasha_end));

            antar_dasha_start = antar_dasha_end;
        }

        let current_antar_dasha = antar_dasha_periods
            .iter()
            .find(|&&(_, start, end)| now >= start && now < end)
            .unwrap_or(&antar_dasha_periods[0]);

        let (antar_dasha, antar_dasha_start, antar_dasha_end) = *current_antar_dasha;

        // Pratyantar Dasha Calculation
        let antar_dasha_duration = (antar_dasha_end - antar_dasha_start).num_seconds() as f64;

        let mut pratyantar_dasha_start = antar_dasha_start;
        let mut pratyantar_dasha_periods = Vec::new();

        for &pratyantar_dasha in &dasha_sequence {
            let pratyantar_dasha_years = dasha_years
                .iter()
                .find(|&&(dasha, _)| dasha == pratyantar_dasha)
                .map(|&(_, years)| years)
                .unwrap_or(0.0);

            let pratyantar_dasha_duration = antar_dasha_duration * (pratyantar_dasha_years / 120.0);

            let pratyantar_dasha_end = pratyantar_dasha_start
                + ChronoDuration::seconds(pratyantar_dasha_duration as i64);

            pratyantar_dasha_periods.push((pratyantar_dasha, pratyantar_dasha_start, pratyantar_dasha_end));

            pratyantar_dasha_start = pratyantar_dasha_end;
        }

        let current_pratyantar_dasha = pratyantar_dasha_periods
            .iter()
            .find(|&&(_, start, end)| now >= start && now < end)
            .unwrap_or(&pratyantar_dasha_periods[0]);

        let (pratyantar_dasha, pratyantar_dasha_start, pratyantar_dasha_end) = *current_pratyantar_dasha;

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

    
    pub fn calculate_planetary_states(
        &self,
        chart_info: &ChartInfo,
    ) -> Result<HashMap<CelestialBody, PlanetaryState>, CalculationError> {
        let mut states = HashMap::new();

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
            (CelestialBody::Mars, vec![ZodiacSign::Aries, ZodiacSign::Scorpio]),
            (CelestialBody::Mercury, vec![ZodiacSign::Gemini, ZodiacSign::Virgo]),
            (CelestialBody::Jupiter, vec![ZodiacSign::Sagittarius, ZodiacSign::Pisces]),
            (CelestialBody::Venus, vec![ZodiacSign::Taurus, ZodiacSign::Libra]),
            (CelestialBody::Saturn, vec![ZodiacSign::Capricorn, ZodiacSign::Aquarius]),
            (CelestialBody::Rahu, vec![ZodiacSign::Gemini, ZodiacSign::Virgo]),
            (CelestialBody::Ketu, vec![ZodiacSign::Sagittarius, ZodiacSign::Pisces]),
        ];

        for planet_position in &chart_info.planets {
            let planet = planet_position.planet;
            let sign = planet_position.sign;
            let longitude = planet_position.longitude % 30.0;

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

            let own_sign = own_signs
                .iter()
                .find(|&&(p, ref signs)| p == planet && signs.contains(&sign))
                .map(|_| PlanetaryState::OwnSign);

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

            let final_state = if planet_position.retrograde {
                PlanetaryState::Retrograde
            } else {
                state
            };

            states.insert(planet, final_state);
        }

        Ok(states)
    }

    pub fn calculate(
        &self,
        coord_system: CoordinateSystem,
        julian_day: JulianDay,
        body: CelestialBody,
        flags: &[CalculationFlag],
    ) -> Result<AstronomicalResult, CalculationError> {
        match coord_system {
            CoordinateSystem::Sidereal => unsafe {
                swe_set_sid_mode(SE_SIDM_LAHIRI, 0.0, 0.0);
            },
            CoordinateSystem::Tropical => unsafe {
                swe_set_sid_mode(SE_SIDM_FAGAN_BRADLEY, 0.0, 0.0);
            },
        }

        let mut iflag: c_int = if coord_system == CoordinateSystem::Sidereal {
            SEFLG_SIDEREAL
        } else {
            0
        };
        for flag in flags {
            iflag |= *flag as c_int;
        }

        let result = match body {
            CelestialBody::Ketu => {
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
                        -info.speed_latitude,
                        info.speed_distance,
                    ),
                    _ => {
                        return Err(CalculationError {
                            code: -1,
                            message: "Failed to calculate Ketu".to_string(),
                        })
                    }
                };
                Ok(AstronomicalResult::CelestialBody(CelestialCoordinates {
                    longitude,
                    latitude,
                    distance,
                    speed_longitude,
                    speed_latitude,
                    speed_distance,
                }))
            }
            _ => {
                let mut results: [c_double; 6] = [0.0; 6];
                let mut error: [c_char; 256] = [0; 256];
                let calc_result = unsafe {
                    swe_calc_ut(
                        julian_day,
                        body as c_int,
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
                Ok(AstronomicalResult::CelestialBody(CelestialCoordinates {
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

    pub fn get_body_name(&self, body: CelestialBody) -> String {
        match body {
            CelestialBody::Ketu => "Ketu".to_string(),
            _ => {
                let mut name: [c_char; 256] = [0; 256];
                unsafe {
                    swe_get_planet_name(body as c_int, name.as_mut_ptr());
                }
                unsafe { CStr::from_ptr(name.as_ptr()) }
                    .to_string_lossy()
                    .into_owned()
            }
        }
    }

    pub fn calculate_houses(
        &self,
        coord_system: CoordinateSystem,
        julian_day: JulianDay,
        latitude: f64,
        longitude: f64,
        house_system: ChartType,
    ) -> Result<Vec<HousePosition>, CalculationError> {
        let hsys = match house_system {
            ChartType::Rasi => SE_HS_PLACIDUS,
            ChartType::Navamsa => SE_HS_NAVAMSA,
            ChartType::Hora => SE_HS_HORA,
            // Add other house systems as needed
        };

        if coord_system == CoordinateSystem::Sidereal {
            unsafe {
                swe_set_sid_mode(SE_SIDM_LAHIRI, 0.0, 0.0);
            }
        }

        let flag = if coord_system == CoordinateSystem::Sidereal {
            SEFLG_SIDEREAL
        } else {
            0
        };

        let mut cusps: [c_double; 13] = [0.0; 13];
        let mut ascmc: [c_double; 10] = [0.0; 10];

        let calc_result = unsafe {
            swe_houses_ex(
                julian_day,
                flag,
                latitude,
                longitude,
                hsys,
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
                house: House::from_index(i).unwrap(),
                sign: Self::get_zodiac_sign(cusps[i]),
                degree: cusps[i] % 30.0,
            })
            .collect();

        Ok(house_positions)
    }

    pub fn calculate_ascendant(
        &self,
        coord_system: CoordinateSystem,
        julian_day: JulianDay,
        latitude: f64,
        longitude: f64,
        house_system: ChartType,
    ) -> Result<HousePosition, CalculationError> {
        let hsys = match house_system {
            ChartType::Rasi => SE_HS_PLACIDUS,
            ChartType::Navamsa => SE_HS_NAVAMSA,
            ChartType::Hora => SE_HS_HORA,
            // Add other house systems as needed
        };

        let flag = if coord_system == CoordinateSystem::Sidereal {
            SEFLG_SIDEREAL
        } else {
            0
        };

        let mut cusps: [c_double; 13] = [0.0; 13];
        let mut ascmc: [c_double; 10] = [0.0; 10];

        let calc_result = unsafe {
            swe_houses_ex(
                julian_day,
                flag,
                latitude,
                longitude,
                hsys,
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

        let ascendant_degree = ascmc[0];
        let sign = Self::get_zodiac_sign(ascendant_degree);
        Ok(HousePosition {
            house: House::First,
            sign,
            degree: ascendant_degree % 30.0,
        })
    }

    fn get_zodiac_sign(longitude: f64) -> ZodiacSign {
        ZodiacSign::from_longitude(longitude)
    }

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
                AstronomicalResult::CelestialBody(info) => (info.longitude, info.latitude, info.speed_longitude),
                _ => continue,
            };

            let adjusted_longitude = match chart_type {
                ChartType::Rasi => longitude,
                ChartType::Navamsa => self.calculate_navamsa(longitude),
                ChartType::Hora => (longitude * 2.0) % 360.0, // Example for Hora
                // Add more chart types as needed
            };

            let sign = Self::get_zodiac_sign(adjusted_longitude);
            let house = self.get_house(
                julian_day,
                adjusted_longitude,
                birth_info.location.latitude,
                birth_info.location.longitude,
                chart_type,
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

 
 

    // ---------------------------
    // ## Compatibility Calculations
    // ---------------------------

    pub fn calculate_compatibility(
        &self,
        chart1: &ChartInfo,
        chart2: &ChartInfo,
    ) -> CompatibilityInfo {
        let kuta_points = self.calculate_kuta_points(chart1, chart2);
        let compatibility_score = self.calculate_compatibility_score(chart1, chart2);

        CompatibilityInfo {
            kuta_points,
            compatibility_score,
        }
    }

    pub fn calculate_yogas(&self, chart: &ChartInfo) -> Vec<YogaInfo> {
        let mut yogas = Vec::new();

        let get_planet = |body: CelestialBody| -> Option<&PlanetPosition> {
            chart.planets.iter().find(|p| p.planet == body)
        };

        // Example Yoga 1: Raj Yoga - Lord of 9th and 10th house conjunction
        if let (Some(ninth_lord), Some(tenth_lord)) = (
            get_planet(CelestialBody::Jupiter),
            get_planet(CelestialBody::Saturn),
        ) {
            if (ninth_lord.longitude - tenth_lord.longitude).abs() < 10.0 {
                yogas.push(YogaInfo {
                    yoga: Yoga {
                        name: "Raj Yoga".to_string(),
                        condition: Condition {
                            description: "Conjunction of lords of 9th and 10th houses".to_string(),
                            check: |chart| {
                                let ninth_lord = chart.planets.iter().find(|p| p.house == House::Ninth).map(|p| p.planet);
                                let tenth_lord = chart.planets.iter().find(|p| p.house == House::Tenth).map(|p| p.planet);
                                match (ninth_lord, tenth_lord) {
                                    (Some(n), Some(t)) => {
                                        let p1 = chart.planets.iter().find(|p| p.planet == n).unwrap();
                                        let p2 = chart.planets.iter().find(|p| p.planet == t).unwrap();
                                        (p1.longitude - p2.longitude).abs() < 10.0
                                    }
                                    _ => false,
                                }
                            },
                        },
                        effects: Effects {
                            description: "Enhances authority and career prospects.".to_string(),
                            apply: |chart| Impact::Positive(On::Oneself, Trait::Career, 8.0),
                        },
                        strength: 1.0,
                    },
                    strength: 1.0,
                    involved_planets: vec![CelestialBody::Jupiter, CelestialBody::Saturn],
                });
            }
        }

        // Example Yoga 2: Gajakesari Yoga - Jupiter in a Kendra from Moon
        if let (Some(jupiter), Some(moon)) = (
            get_planet(CelestialBody::Jupiter),
            get_planet(CelestialBody::Moon),
        ) {
            let house_diff = (jupiter.house as i32 - moon.house as i32).abs() % 12;
            if house_diff == 4 || house_diff == 7 || house_diff == 10 || house_diff == 1 {
                yogas.push(YogaInfo {
                    yoga: Yoga {
                        name: "Gajakesari Yoga".to_string(),
                        condition: Condition {
                            description: "Jupiter in Kendra from Moon".to_string(),
                            check: |chart| {
                                let j = chart.planets.iter().find(|p| p.planet == CelestialBody::Jupiter).unwrap();
                                let m = chart.planets.iter().find(|p| p.planet == CelestialBody::Moon).unwrap();
                                let house_diff = (j.house as i32 - m.house as i32).abs() % 12;
                                house_diff == 4 || house_diff == 7 || house_diff == 10 || house_diff == 1
                            },
                        },
                        effects: Effects {
                            description: "Brings intelligence and prosperity.".to_string(),
                            apply: |chart| Impact::Positive(On::Oneself, Trait::Wealth, 7.0),
                        },
                        strength: 0.85,
                    },
                    strength: 0.85,
                    involved_planets: vec![CelestialBody::Jupiter, CelestialBody::Moon],
                });
            }
        }

        // Example Yoga 3: Budhaditya Yoga - Sun and Mercury in same house
        if let (Some(sun), Some(mercury)) = (
            get_planet(CelestialBody::Sun),
            get_planet(CelestialBody::Mercury),
        ) {
            if sun.house == mercury.house {
                yogas.push(YogaInfo {
                    yoga: Yoga {
                        name: "Budhaditya Yoga".to_string(),
                        condition: Condition {
                            description: "Sun and Mercury in the same house".to_string(),
                            check: |chart| {
                                let s = chart.planets.iter().find(|p| p.planet == CelestialBody::Sun).unwrap();
                                let m = chart.planets.iter().find(|p| p.planet == CelestialBody::Mercury).unwrap();
                                s.house == m.house
                            },
                        },
                        effects: Effects {
                            description: "Enhances communication and intelligence.".to_string(),
                            apply: |chart| Impact::Positive(On::Oneself, Trait::Communication, 8.0),
                        },
                        strength: 0.9,
                    },
                    strength: 0.9,
                    involved_planets: vec![CelestialBody::Sun, CelestialBody::Mercury],
                });
            }
        }

        // Example Yoga 4: Hamsa Yoga - Jupiter in Kendra from Moon
        if let (Some(jupiter), Some(moon)) = (
            get_planet(CelestialBody::Jupiter),
            get_planet(CelestialBody::Moon),
        ) {
            let house_diff = (jupiter.house as i32 - moon.house as i32).abs() % 12;
            if house_diff == 4 || house_diff == 7 || house_diff == 10 || house_diff == 1 {
                yogas.push(YogaInfo {
                    yoga: Yoga {
                        name: "Hamsa Yoga".to_string(),
                        condition: Condition {
                            description: "Jupiter in Kendra from Moon".to_string(),
                            check: |chart| {
                                let j = chart.planets.iter().find(|p| p.planet == CelestialBody::Jupiter).unwrap();
                                let m = chart.planets.iter().find(|p| p.planet == CelestialBody::Moon).unwrap();
                                let house_diff = (j.house as i32 - m.house as i32).abs() % 12;
                                house_diff == 4 || house_diff == 7 || house_diff == 10 || house_diff == 1
                            },
                        },
                        effects: Effects {
                            description: "Bestows wisdom and prosperity.".to_string(),
                            apply: |chart| Impact::Positive(On::Oneself, Trait::Wealth, 8.0),
                        },
                        strength: 0.8,
                    },
                    strength: 0.8,
                    involved_planets: vec![CelestialBody::Jupiter, CelestialBody::Moon],
                });
            }
        }

        // Example Yoga 5: Malavya Yoga - Venus in a Kendra house
        if let Some(venus) = get_planet(CelestialBody::Venus) {
            if matches!(
                venus.house,
                House::First | House::Fourth | House::Seventh | House::Tenth
            ) {
                yogas.push(YogaInfo {
                    yoga: Yoga {
                        name: "Malavya Yoga".to_string(),
                        condition: Condition {
                            description: "Venus in a Kendra house".to_string(),
                            check: |chart| {
                                let v = chart.planets.iter().find(|p| p.planet == CelestialBody::Venus).unwrap();
                                matches!(v.house, House::First | House::Fourth | House::Seventh | House::Tenth)
                            },
                        },
                        effects: Effects {
                            description: "Enhances love and artistic abilities.".to_string(),
                            apply: |chart| Impact::Positive(On::Oneself, Trait::Relationship, 7.0),
                        },
                        strength: 0.75,
                    },
                    strength: 0.75,
                    involved_planets: vec![CelestialBody::Venus],
                });
            }
        }

        yogas
    }

    pub fn calculate_special_lagnas(&self, chart: &ChartInfo) -> HashMap<SpecialLagna, f64> {
        let mut special_lagnas = HashMap::new();

        let ascendant_longitude = chart.ascendant.degree;
        let sun_longitude = chart.planets.iter().find(|p| p.planet == CelestialBody::Sun).unwrap().longitude;
        let moon_longitude = chart.planets.iter().find(|p| p.planet == CelestialBody::Moon).unwrap().longitude;

        // Calculate Hora Lagna
        let hora_lagna = (ascendant_longitude + (sun_longitude - moon_longitude)) % 360.0;
        special_lagnas.insert(SpecialLagna::Hora, hora_lagna);

        // Calculate Ghati Lagna
        let ghati_lagna = (ascendant_longitude + (moon_longitude - sun_longitude) * 5.0) % 360.0;
        special_lagnas.insert(SpecialLagna::Ghati, ghati_lagna);

        // Calculate Varnada Lagna
        let varnada_lagna = (ascendant_longitude + (sun_longitude - moon_longitude) * 3.0) % 360.0;
        special_lagnas.insert(SpecialLagna::Varnada, varnada_lagna);

        // Calculate Sree Lagna
        let sree_lagna = (ascendant_longitude + moon_longitude) % 360.0;
            special_lagnas.insert(SpecialLagna::Sree, sree_lagna);

        // Calculate Pranapada Lagna
            let pranapada_lagna = (ascendant_longitude + (sun_longitude - moon_longitude) * 7.0) % 360.0;
        special_lagnas.insert(SpecialLagna::Pranapada, pranapada_lagna);

        special_lagnas
    }


    fn calculate_kuta_points(&self, chart1: &ChartInfo, chart2: &ChartInfo) -> u32 {
        let mut points = 0;

        // Varna Kuta (1 point)
        if self.check_varna_compatibility(chart1.ascendant.sign, chart2.ascendant.sign) {
            points += 1;
        }

        // Vasya Kuta (2 points)
        if self.check_vasya_compatibility(chart1.ascendant.sign, chart2.ascendant.sign) {
            points += 2;
        }

        // Tara Kuta (3 points)
        points += self.calculate_tara_kuta(chart1, chart2);

        // Yoni Kuta (4 points)
        points += self.calculate_yoni_kuta(chart1, chart2);

        // Graha Maitri (5 points)
        points += self.calculate_graha_maitri(chart1, chart2);

        // Gana Kuta (6 points)
        if self.check_gana_compatibility(chart1.ascendant.sign, chart2.ascendant.sign) {
            points += 6;
        }

        // Bhakut Kuta (7 points)
        if self.check_bhakut_compatibility(chart1.ascendant.sign, chart2.ascendant.sign) {
            points += 7;
        }

        // Nadi Kuta (8 points)
        if self.check_nadi_compatibility(chart1.ascendant.sign, chart2.ascendant.sign) {
            points += 8;
        }

        points
    }

    fn calculate_compatibility_score(&self, chart1: &ChartInfo, chart2: &ChartInfo) -> f64 {
        let kuta_points = self.calculate_kuta_points(chart1, chart2) as f64;
        let max_points = 36.0; // Maximum possible Kuta points

        (kuta_points / max_points) * 100.0
    }

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

        vasya_groups
            .iter()
            .any(|group| group.contains(&sign1) && group.contains(&sign2))
    }

    fn calculate_tara_kuta(&self, chart1: &ChartInfo, chart2: &ChartInfo) -> u32 {
        let moon1 = chart1
            .planets
            .iter()
            .find(|p| p.planet == CelestialBody::Moon)
            .unwrap();
        let moon2 = chart2
            .planets
            .iter()
            .find(|p| p.planet == CelestialBody::Moon)
            .unwrap();

        let nakshatra1 = moon1.nakshatra.nakshatra as u32;
        let nakshatra2 = moon2.nakshatra.nakshatra as u32;

        let tara = ((nakshatra2 + 27) - nakshatra1) % 27 / 3;

        match tara {
            1 | 3 | 5 | 7 => 3,
            0 | 2 | 4 | 6 | 8 => 0,
            _ => 0,
        }
    }

    fn calculate_yoni_kuta(&self, chart1: &ChartInfo, chart2: &ChartInfo) -> u32 {
        let moon1 = chart1
            .planets
            .iter()
            .find(|p| p.planet == CelestialBody::Moon)
            .unwrap();
        let moon2 = chart2
            .planets
            .iter()
            .find(|p| p.planet == CelestialBody::Moon)
            .unwrap();

        let yoni1 = self.get_yoni(moon1.nakshatra.nakshatra);
        let yoni2 = self.get_yoni(moon2.nakshatra.nakshatra);

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
            Nakshatra::Rohini | Nakshatra::UttaraPhalguni => "Snake",
            Nakshatra::Mrigashira | Nakshatra::Chitra => "Dog",
            Nakshatra::Ardra | Nakshatra::Shravana => "Cat",
            Nakshatra::Pushya | Nakshatra::UttaraAshadha => "Ram",
            Nakshatra::Ashlesha | Nakshatra::Jyeshtha => "Mongoose",
            Nakshatra::Magha | Nakshatra::PurvaPhalguni => "Rat",
            Nakshatra::Hasta | Nakshatra::Anuradha => "Buffalo",
            Nakshatra::Swati | Nakshatra::Dhanishta => "Tiger",
            Nakshatra::Vishakha | Nakshatra::PurvaAshadha => "Deer",
            Nakshatra::Moola | Nakshatra::PurvaBhadrapada => "Monkey",
            Nakshatra::UttaraBhadrapada => "Lion",
        }
    }

    fn are_yonis_compatible(&self, yoni1: &str, yoni2: &str) -> bool {
        let compatible_pairs = vec![
            ("Horse", "Horse"),
            ("Elephant", "Elephant"),
            ("Goat", "Goat"),
            ("Snake", "Snake"),
            ("Dog", "Dog"),
            ("Cat", "Cat"),
            ("Ram", "Ram"),
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

    pub fn calculate_graha_maitri(&self, chart1: &ChartInfo, chart2: &ChartInfo) -> u32 {
        let lord1 = self.get_house_lord(chart1.ascendant.house);
        let lord2 = self.get_house_lord(chart2.ascendant.house);

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
            (
                CelestialBody::Sun,
                vec![
                    CelestialBody::Moon,
                    CelestialBody::Mars,
                    CelestialBody::Jupiter,
                ],
            ),
            (
                CelestialBody::Moon,
                vec![CelestialBody::Sun, CelestialBody::Mercury],
            ),
            (
                CelestialBody::Mars,
                vec![
                    CelestialBody::Sun,
                    CelestialBody::Moon,
                    CelestialBody::Jupiter,
                ],
            ),
            (
                CelestialBody::Mercury,
                vec![CelestialBody::Sun, CelestialBody::Venus],
            ),
            (
                CelestialBody::Jupiter,
                vec![
                    CelestialBody::Sun,
                    CelestialBody::Moon,
                    CelestialBody::Mars,
                ],
            ),
            (
                CelestialBody::Venus,
                vec![CelestialBody::Mercury, CelestialBody::Saturn],
            ),
            (
                CelestialBody::Saturn,
                vec![CelestialBody::Mercury, CelestialBody::Venus],
            ),
        ];

        friendships.iter().any(|&(p, ref friends)| {
            (p == planet1 && friends.contains(&planet2)) || (p == planet2 && friends.contains(&planet1))
        })
    }

    fn are_planets_neutral(&self, planet1: CelestialBody, planet2: CelestialBody) -> bool {
        let neutral_relations = [
            (CelestialBody::Sun, vec![CelestialBody::Mercury]),
            (
                CelestialBody::Moon,
                vec![
                    CelestialBody::Mars,
                    CelestialBody::Jupiter,
                    CelestialBody::Venus,
                    CelestialBody::Saturn,
                ],
            ),
            (
                CelestialBody::Mars,
                vec![
                    CelestialBody::Mercury,
                    CelestialBody::Venus,
                    CelestialBody::Saturn,
                ],
            ),
            (
                CelestialBody::Mercury,
                vec![
                    CelestialBody::Mars,
                    CelestialBody::Jupiter,
                    CelestialBody::Saturn,
                ],
            ),
            (
                CelestialBody::Jupiter,
                vec![
                    CelestialBody::Mercury,
                    CelestialBody::Venus,
                    CelestialBody::Saturn,
                ],
            ),
            (
                CelestialBody::Venus,
                vec![CelestialBody::Mars, CelestialBody::Jupiter],
            ),
            (
                CelestialBody::Saturn,
                vec![CelestialBody::Mars, CelestialBody::Jupiter],
            ),
        ];

        neutral_relations.iter().any(|&(p, ref neutrals)| {
            (p == planet1 && neutrals.contains(&planet2)) || (p == planet2 && neutrals.contains(&planet1))
        })
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
            ZodiacSign::Cancer | ZodiacSign::Scorpio | ZodiacSign::Pisces => "Rakshasa",
            ZodiacSign::Gemini | ZodiacSign::Libra | ZodiacSign::Aquarius => "Deva",
            ZodiacSign::Taurus | ZodiacSign::Virgo | ZodiacSign::Capricorn => "Manushya",
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
            ZodiacSign::Taurus | ZodiacSign::Virgo | ZodiacSign::Sagittarius | ZodiacSign::Pisces => "Madhya",
            ZodiacSign::Gemini | ZodiacSign::Libra | ZodiacSign::Aquarius => "Antya",
            _ => "Unknown",
        }
    }

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
        planet.retrograde || self.is_combust(planet)
    }

    fn is_combust(&self, planet: &PlanetPosition) -> bool {
        if planet.planet == CelestialBody::Sun {
            return false;
        }

        let current_julian_day = date_to_julian_day(Utc::now());
        let sun_position = self.calculate(CoordinateSystem::Tropical, current_julian_day, CelestialBody::Sun, &[])
            .unwrap_or(AstronomicalResult::CelestialBody(CelestialCoordinates {
                longitude: 0.0,
                latitude: 0.0,
                distance: 0.0,
                speed_longitude: 0.0,
                speed_latitude: 0.0,
                speed_distance: 0.0,
            }));

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

    pub fn generate_interpretation(&self, report: &Report) -> String {
        let mut interpretation = String::new();

        interpretation.push_str(&format!(
            "Birth Chart Interpretation for {}\n\n",
            report.birth_info.date_time.format("%Y-%m-%d %H:%M:%S")
        ));

        interpretation.push_str("Planetary Positions:\n");
        for planet in &report.charts[0].planets {
            interpretation.push_str(&format!(
                "{}: {}° in {:?} (House {:?})\n",
                self.get_body_name(planet.planet),
                planet.longitude,
                planet.nakshatra.nakshatra,
                planet.house
            ));
        }

        interpretation.push_str("\nAscendant: ");
        interpretation.push_str(&format!("{:?}\n", report.charts[0].ascendant));

        interpretation.push_str("\nYogas:\n");
        for yoga in &report.yogas {
            interpretation.push_str(&format!(
                "{} Yoga (Strength: {:.2})\n",
                yoga.yoga.name, yoga.strength
            ));
        }

        interpretation.push_str("\nDasha Periods:\n");
        interpretation.push_str(&format!(
            "Maha Dasha: {:?} ({} to {})\n",
            report.dashas.maha_dasha,
            report.dashas.maha_dasha_start.format("%Y-%m-%d"),
            report.dashas.maha_dasha_end.format("%Y-%m-%d")
        ));
        interpretation.push_str(&format!(
            "Antar Dasha: {:?} ({} to {})\n",
            report.dashas.antar_dasha,
            report.dashas.antar_dasha_start.format("%Y-%m-%d"),
            report.dashas.antar_dasha_end.format("%Y-%m-%d")
        ));
        interpretation.push_str(&format!(
            "Pratyantar Dasha: {:?} ({} to {})\n",
            report.dashas.pratyantar_dasha,
            report.dashas.pratyantar_dasha_start.format("%Y-%m-%d"),
            report.dashas.pratyantar_dasha_end.format("%Y-%m-%d")
        ));

        interpretation.push_str("\nPlanetary Strengths:\n");
        for (planet, strength) in &report.strengths {
            interpretation.push_str(&format!(
                "{}: Shadbala = {:.2}, Ashtakavarga = {}\n",
                self.get_body_name(*planet),
                strength.shad_bala,
                strength.ashtaka_varga
            ));
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

    pub fn calculate_divisional_charts(&self, chart: &ChartInfo) -> Vec<DivisionalChart> {
        let mut divisional_charts = Vec::new();

        // D1 chart (Rashi chart)
        divisional_charts.push(self.calculate_D1(chart));

        // D2 chart (Hora chart)
        divisional_charts.push(self.calculate_D2(chart));

        // Add more divisional charts as needed (D3, D4, D9, etc.)

        divisional_charts
    }

    fn calculate_D1(&self, chart: &ChartInfo) -> DivisionalChart {
        DivisionalChart {
            chart_type: ChartType::Rasi,
            ascendant: chart.ascendant.sign,
            houses: chart
                .houses
                .iter()
                .map(|house| house.sign)
                .collect::<Vec<ZodiacSign>>()
                .try_into()
                .unwrap(),
            planets: chart.planets.clone(),
        }
    }

    fn calculate_D2(&self, chart: &ChartInfo) -> DivisionalChart {
        let mut d2_planets = Vec::new();

        for planet in &chart.planets {
            let d2_longitude = (planet.longitude * 2.0) % 360.0;
            let d2_sign = ZodiacSign::from_longitude(d2_longitude);
            let d2_house = House::from_index(((d2_longitude / 30.0).floor() as usize) + 1).unwrap();

            d2_planets.push(PlanetPosition {
                planet: planet.planet,
                longitude: d2_longitude,
                latitude: planet.latitude,
                speed: planet.speed,
                sign: d2_sign,
                house: d2_house,
                retrograde: planet.retrograde,
                nakshatra: NakshatraInfo::from_longitude(d2_longitude),
            });
        }

        DivisionalChart {
            chart_type: ChartType::Hora,
            ascendant: ZodiacSign::from_longitude((chart.ascendant.degree * 2.0).rem_euclid(360.0)),
            houses: [ZodiacSign::Aries; 12], // Placeholder, actual calculation needed
            planets: d2_planets,
        }
    }

    pub fn calculate_chart(&self, birth_info: &BirthInfo) -> Result<ChartInfo, CalculationError> {
        let julian_day = date_to_julian_day(birth_info.date_time);
        let ayanamsa = self.calculate_ayanamsa(julian_day);
        let houses = self.calculate_houses(CoordinateSystem::Sidereal, julian_day, birth_info.location.latitude, birth_info.location.longitude, ChartType::Rasi)?;
        let planets = self.calculate_planet_positions(CoordinateSystem::Sidereal, julian_day, ChartType::Rasi, birth_info)?;

        let ascendant = houses.first().cloned().ok_or(CalculationError {
            code: -1,
            message: "Failed to calculate ascendant".to_string(),
        })?;

        Ok(ChartInfo {
            chart_type: ChartType::Rasi,
            ascendant,
            houses,
            planets,
        })
    }
 
    fn calculate_house(&self, julian_day: f64, latitude: f64, longitude: f64, chart_type: ChartType, planet_longitude: f64) -> Result<House, CalculationError> {
        let hsys = match chart_type {
            ChartType::Rasi => SE_HS_PLACIDUS,
            ChartType::Navamsa => SE_HS_NAVAMSA,
            ChartType::Hora => SE_HS_HORA,
        };

        let mut cusps: [c_double; 13] = [0.0; 13];
        let mut ascmc: [c_double; 10] = [0.0; 10];
        let mut planet_longitude = planet_longitude;

        unsafe {
            swe_houses_ex(
                julian_day,
                0, // flags
                latitude,
                longitude,
                hsys,
                cusps.as_mut_ptr(),
                ascmc.as_mut_ptr(),
            );
        }

        let house_position = unsafe {
            swe_house_pos(
                ascmc[2],
                latitude,
                ascmc[0],
                hsys,
                &mut planet_longitude as *mut f64,
                std::ptr::null_mut(),
            )
        };

        let house_number = house_position.floor() as usize;
        House::from_index(house_number).ok_or(CalculationError {
            code: -1,
            message: format!("Invalid house number: {}", house_number),
        })
    }


    pub fn is_house_compatible(&self, house1: House, house2: House) -> bool {
        let angle_diff = (house2 as i32 - house1 as i32 + 12) % 12;
        matches!(angle_diff, 1 | 2 | 3 | 4 | 5 | 7 | 9 | 11)
    }

    pub fn get_house_lord(&self, house: House) -> CelestialBody {
        match house {
            House::First => CelestialBody::Moon,
            House::Second => CelestialBody::Mars,
            House::Third => CelestialBody::Mercury,
            House::Fourth => CelestialBody::Jupiter,
            House::Fifth => CelestialBody::Venus,
            House::Sixth => CelestialBody::Saturn,
            House::Seventh => CelestialBody::Rahu,
            House::Eighth => CelestialBody::Ketu,
            House::Ninth => CelestialBody::Sun,
            House::Tenth => CelestialBody::Moon,
            House::Eleventh => CelestialBody::Mars,
            House::Twelfth => CelestialBody::Mercury,
            _ => CelestialBody::Sun,
        }
    }

    pub fn calculate_house_lord_strength(&self, house: House, planet: CelestialBody) -> f64 {
        let angle_diff = (planet as i32 - house as i32 + 12) % 12;
        let strength = match angle_diff {
            1 => 100.0,
            2 => 95.0,
            3 => 90.0,
            4 => 85.0,
            5 => 80.0,
            6 => 75.0,
            7 => 70.0,
            8 => 65.0,
            9 => 60.0,
            10 => 55.0,
            11 => 50.0,
            _ => 0.0,
        };

        strength
    }

    pub fn calculate_house_lord_compatibility(&self, chart1: &ChartInfo, chart2: &ChartInfo) -> u32 {
        let mut score = 0;

        for house in House::all() {
            let lord1 = self.get_house_lord(house);
            let lord2 = self.get_house_lord(house); 

            let strength1 = self.calculate_house_lord_strength(house, lord1);
            let strength2 = self.calculate_house_lord_strength(house, lord2);

            if self.is_house_compatible(house, house) {
                score += (strength1 + strength2) as u32;
            }
        }

        score
    }

    pub fn calculate_house_lord_compatibility_score(&self, chart1: &ChartInfo, chart2: &ChartInfo) -> u32 {
        let mut score = 0;

        for house in House::all() {
            let lord1 = self.get_house_lord(house);
            let lord2 = self.get_house_lord(house);

            let strength1 = self.calculate_house_lord_strength(house, lord1);
            let strength2 = self.calculate_house_lord_strength(house, lord2);

            if self.is_house_compatible(house, house) {
                score += (strength1 + strength2) as u32;
            }
        }

        score
    }

    
}

// ---------------------------
// ## Astronomical Result Enum
// ---------------------------

pub enum AstronomicalResult {
    CelestialBody(CelestialCoordinates),
    HousePosition(f64),
}

// ---------------------------
// ## Utility Functions
// ---------------------------

pub fn date_to_julian_day(date_time: DateTime<Utc>) -> JulianDay {
    let year = date_time.year();
    let month = date_time.month();
    let day = date_time.day();
    let hour = date_time.hour();
    let minute = date_time.minute();
    let second = date_time.second() as f64 + (date_time.nanosecond() as f64 / 1_000_000_000.0);

    let mut tjd_ut: c_double = 0.0;
    let mut dret: [c_double; 2] = [0.0; 2];
    let gregflag = SE_GREG_CAL;
    unsafe {
        swe_utc_to_jd(
            year,
            month as c_int,
            day as c_int,
            hour as c_int,
            minute as c_int,
            second,
            gregflag,
            &mut dret[0],
            &mut dret[1],
        );
        tjd_ut = dret[1]; // Use UT
    }
    tjd_ut
}

pub fn julian_day_to_date(jd: JulianDay) -> DateTime<Utc> {
    let mut year: c_int = 0;
    let mut month: c_int = 0;
    let mut day: c_int = 0;
    let mut hour: c_int = 0;
    let mut minute: c_int = 0;
    let mut second: c_double = 0.0;

    let gregflag = 1; // Gregorian calendar

    unsafe {
        swe_jdut1_to_utc(
            jd,
            gregflag,
            &mut year,
            &mut month,
            &mut day,
            &mut hour,
            &mut minute,
            &mut second,
        );
    }

    Utc.ymd(year as i32, month as u32, day as u32).and_hms_micro(
        hour as u32,
        minute as u32,
        second.floor() as u32,
        ((second.fract() * 1_000_000.0) as u32),
    )
}

pub fn calculate_ayanamsa(julian_day: JulianDay) -> AyanamsaInfo {
    let ayanamsa_value = unsafe { swe_get_ayanamsa_ut(julian_day) };
    let ayanamsa_name = "Lahiri".to_string(); // Assuming Lahiri ayanamsa
    AyanamsaInfo {
        ayanamsa_name,
        ayanamsa_value,
    }
}

 
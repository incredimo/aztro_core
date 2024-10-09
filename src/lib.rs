use chrono::{DateTime, Datelike, Duration as ChronoDuration, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::ffi::{CStr, CString};
use std::io::Cursor;
use std::os::raw::c_char;
use std::sync::Once;
use tempfile::NamedTempFile;

include!("../build/bindings.rs");

// Enums
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoordinateSystem {
    Tropical,
    Sidereal,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum CelestialBody {
    Sun = 0,
    Moon = 1,
    Mars = 4,
    Mercury = 2,
    Jupiter = 5,
    Venus = 3,
    Saturn = 6,
    Rahu = 11,
    Ketu = 999,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalculationFlag {
    Speed = 256,
    NoGravitationalDeflection = 512,
    NoAnnualAberration = 1024,
    Topocentric = 32768,
    Equatorial = 2048,
    XYZ = 8192,
    Radians = 16384,
    Barycentric = 4,
    Heliocentric = 8,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub fn from_longitude(longitude: f64) -> ZodiacSign {
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
            _ => ZodiacSign::Aries, // Default to Aries if out of bounds
        }
    }
}

impl std::fmt::Display for ZodiacSign {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
            _ => Nakshatra::Ashwini, // Default to Ashwini if out of bounds
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Yoga {
    Raj,
    Gajakesari,
    Budhaditya,
    Hamsa,
    Malavya,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChartType {
    Rasi,
    Navamsa,
    Hora, // Added Hora for D2
    // Add more as needed
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DivisionalChart {
    pub chart_type: ChartType,
    pub ascendant: ZodiacSign,
    pub houses: [ZodiacSign; 12],
    pub planets: Vec<PlanetPosition>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpecialLagna {
    Bhava,
    Hora,
    Ghati,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Upagraha {
    Dhuma,
    Vyatipata,
    Parivesha,
    Indrachaapa,
    Upaketu,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensitivePoint {
    Gulika,
    Mandi,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Aspect {
    Conjunction,
    Opposition,
    Trine,
    Square,
    Sextile,
}

// Structs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CelestialBodyInfo {
    pub longitude: f64,
    pub latitude: f64,
    pub distance: f64,
    pub speed_longitude: f64,
    pub speed_latitude: f64,
    pub speed_distance: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct HousePosition {
    pub house: House,
    pub sign: ZodiacSign,
    pub degree: f64,
}

#[derive(Debug)]
pub struct CalculationError {
    pub code: i32,
    pub message: String,
}

pub type JulianDay = f64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct YogaInfo {
    pub yoga: Yoga,
    pub strength: f64,
    pub involved_planets: Vec<CelestialBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AyanamsaInfo {
    pub ayanamsa_name: String,
    pub ayanamsa_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChartInfo {
    pub chart_type: ChartType,
    pub ascendant: ZodiacSign,
    pub houses: Vec<HousePosition>,
    pub planets: Vec<PlanetPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BirthInfo {
    pub date_time: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AspectInfo {
    pub aspect: Aspect,
    pub planet1: CelestialBody,
    pub planet2: CelestialBody,
    pub orb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrengthInfo {
    pub shad_bala: f64,
    pub ashtaka_varga: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DignityInfo {
    pub moolatrikona: bool,
    pub own_sign: bool,
    pub exalted: bool,
    pub debilitated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BhavaInfo {
    pub bhava: House,
    pub sign: ZodiacSign,
    pub degree: f64,
    pub lord: CelestialBody,
    pub planets: Vec<CelestialBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransitInfo {
    pub planet: CelestialBody,
    pub from_sign: ZodiacSign,
    pub to_sign: ZodiacSign,
    pub date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VarshaphalInfo {
    pub year: i32,
    pub ascendant: ZodiacSign,
    pub planets: Vec<PlanetPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompatibilityInfo {
    pub kuta_points: u32,
    pub compatibility_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemedialMeasure {
    pub description: String,
    pub gemstone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AstronomicalResult {
    CelestialBody(CelestialBodyInfo),
    House(HousePosition),
    Nakshatra(NakshatraInfo),
    Dasha(DashaInfo),
    Yoga(YogaInfo),
    Planet(PlanetPosition),
}

pub const SE_HS_PLACIDUS: i32 = 0;
pub const SE_HS_KRISHNAMURTI: i32 = 10;
pub const SE_HS_BUCHAREST: i32 = 11;
pub const SE_HS_EQUATORIAL: i32 = 12;
pub const SE_HS_MERCURY: i32 = 13;
pub const SE_HS_CAMPANUS: i32 = 14;
pub const SE_HS_MORIN: i32 = 15;
pub const SE_HS_PORPHYRUS: i32 = 16;
pub const SE_HS_VEHRENBERG: i32 = 17;
pub const SE_HS_ALCABITUS: i32 = 18;
pub const SE_HS_TOPHRAS: i32 = 19;  
pub const SE_HS_NAVAMSA: i32 = 20;
pub const SE_HS_HORA: i32 = 21;


// SwissEph Structure
pub struct SwissEph {
    _temp_file: NamedTempFile,
}

static EPHE_FILE: &[u8] = include_bytes!("../ephe/sepl_18.se1");
static INIT: Once = Once::new();

impl SwissEph {
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

    pub fn get_house(
        &self,
        julian_day: JulianDay,
        planet_longitude: f64,
        latitude: f64,
        longitude: f64,
        house_system: ChartType,
    ) -> Result<House, CalculationError> {
        let hsys = match house_system {
            ChartType::Rasi => 'P', // Placidus
            ChartType::Navamsa => 'N', // Navamsa
            ChartType::Hora => 'H', // Hora
            // Add other house systems as needed
        };

        let mut cusps: [f64; 13] = [0.0; 13];
        let mut ascmc: [f64; 10] = [0.0; 10];

        let hsys_code = match hsys {
            'P' => SE_HS_PLACIDUS,
            'K' => SE_HS_KRISHNAMURTI,
            'B' => SE_HS_BUCHAREST,
            'E' => SE_HS_EQUATORIAL,
            'X' => SE_HS_MERCURY,
            'C' => SE_HS_CAMPANUS,
            'M' => SE_HS_MORIN,
            'O' => SE_HS_PORPHYRUS,
            'V' => SE_HS_VEHRENBERG,
            'A' => SE_HS_ALCABITUS,
            'L' => SE_HS_TOPHRAS,
            'N' => SE_HS_NAVAMSA,
            'H' => SE_HS_HORA,
            _ => SE_HS_PLACIDUS, // Default to Placidus
        };

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

    pub fn calculate_navamsa(&self, longitude: f64) -> f64 {
        let normalized_longitude = longitude.rem_euclid(360.0);
        let navamsa_longitude = (normalized_longitude / 3.0).rem_euclid(360.0);
        navamsa_longitude
    }

    pub fn calculate_nakshatra(&self, longitude: f64) -> NakshatraInfo {
        NakshatraInfo::from_longitude(longitude)
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
        let mut index = dasha_years
            .iter()
            .position(|(dasha, _)| dasha == &starting_dasha)
            .unwrap_or(0);

        let mut maha_dasha_start = birth_info.date_time;

        let (current_dasha, _) = dasha_years[index];
        let maha_dasha_end = maha_dasha_start
            + ChronoDuration::seconds((dasha_balance_years * 365.25 * 86400.0) as i64);
        maha_dasha_periods.push((current_dasha, maha_dasha_start, maha_dasha_end));
        maha_dasha_start = maha_dasha_end;
        index = (index + 1) % dasha_years.len();

        let mut total_years = dasha_balance_years;

        while total_years < 120.0 {
            let (current_dasha, years) = dasha_years[index];
            let maha_dasha_end =
                maha_dasha_start + ChronoDuration::seconds((years * 365.25 * 86400.0) as i64);
            maha_dasha_periods.push((current_dasha, maha_dasha_start, maha_dasha_end));
            maha_dasha_start = maha_dasha_end;

            total_years += years;
            index = (index + 1) % dasha_years.len();
        }

        let now = Utc::now();
        let current_maha_dasha = maha_dasha_periods
            .iter()
            .find(|&&(_, start, end)| now >= start && now < end)
            .unwrap_or(&maha_dasha_periods[0]);

        let (maha_dasha, maha_dasha_start, maha_dasha_end) = *current_maha_dasha;

        // Antar Dasha Calculation
        let antar_dasha = starting_dasha;
        let antar_dasha_start = maha_dasha_start;
        let antar_dasha_end = maha_dasha_end;

        // Pratyantar Dasha Calculation
        let pratyantar_dasha = starting_dasha;
        let pratyantar_dasha_start = antar_dasha_start;
        let pratyantar_dasha_end = antar_dasha_end;

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
                swe_set_sid_mode(SE_SIDM_LAHIRI as i32, 0.0, 0.0);
            },
            CoordinateSystem::Tropical => unsafe {
                swe_set_sid_mode(SE_SIDM_FAGAN_BRADLEY as i32, 0.0, 0.0);
            },
        }

        let mut iflag: i32 = if coord_system == CoordinateSystem::Sidereal {
            SEFLG_SIDEREAL as i32
        } else {
            0
        };
        for flag in flags {
            iflag |= *flag as i32;
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
                swe_set_sid_mode(SE_SIDM_LAHIRI as i32, 0.0, 0.0);
            }
        }

        let flag = if coord_system == CoordinateSystem::Sidereal {
            SEFLG_SIDEREAL as i32
        } else {
            0
        };

        let mut cusps: [f64; 13] = [0.0; 13];
        let mut ascmc: [f64; 10] = [0.0; 10];

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
            SEFLG_SIDEREAL as i32
        } else {
            0
        };

        let mut cusps: [f64; 13] = [0.0; 13];
        let mut ascmc: [f64; 10] = [0.0; 10];

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
                birth_info.latitude,
                birth_info.longitude,
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

    pub fn calculate_yogas(&self, chart: &ChartInfo) -> Vec<YogaInfo> {
        let mut yogas = Vec::new();

        let get_planet = |body: CelestialBody| -> Option<&PlanetPosition> {
            chart.planets.iter().find(|p| p.planet == body)
        };

        // Raj Yoga: Lord of 9th and 10th house conjunction
        if let (Some(ninth_lord), Some(tenth_lord)) = (
            get_planet(CelestialBody::Jupiter),
            get_planet(CelestialBody::Saturn),
        ) {
            if (ninth_lord.longitude - tenth_lord.longitude).abs() < 10.0 {
                yogas.push(YogaInfo {
                    yoga: Yoga::Raj,
                    strength: 1.0,
                    involved_planets: vec![CelestialBody::Jupiter, CelestialBody::Saturn],
                });
            }
        }

        // Gajakesari Yoga: Jupiter in a Kendra from Moon
        if let (Some(jupiter), Some(moon)) = (
            get_planet(CelestialBody::Jupiter),
            get_planet(CelestialBody::Moon),
        ) {
            let house_diff = (jupiter.house as i32 - moon.house as i32).abs() % 12;
            if house_diff == 4 || house_diff == 7 || house_diff == 10 || house_diff == 1 {
                yogas.push(YogaInfo {
                    yoga: Yoga::Gajakesari,
                    strength: 0.85,
                    involved_planets: vec![CelestialBody::Jupiter, CelestialBody::Moon],
                });
            }
        }

        // Budhaditya Yoga: Sun and Mercury in same house
        if let (Some(sun), Some(mercury)) = (
            get_planet(CelestialBody::Sun),
            get_planet(CelestialBody::Mercury),
        ) {
            if sun.house == mercury.house {
                yogas.push(YogaInfo {
                    yoga: Yoga::Budhaditya,
                    strength: 0.9,
                    involved_planets: vec![CelestialBody::Sun, CelestialBody::Mercury],
                });
            }
        }

        // Hamsa Yoga: Jupiter in Kendra from Moon
        if let (Some(jupiter), Some(moon)) = (
            get_planet(CelestialBody::Jupiter),
            get_planet(CelestialBody::Moon),
        ) {
            let house_diff = (jupiter.house as i32 - moon.house as i32).abs() % 12;
            if house_diff == 4 || house_diff == 7 || house_diff == 10 || house_diff == 1 {
                yogas.push(YogaInfo {
                    yoga: Yoga::Hamsa,
                    strength: 0.8,
                    involved_planets: vec![CelestialBody::Jupiter, CelestialBody::Moon],
                });
            }
        }

        // Malavya Yoga: Venus in a Kendra house
        if let Some(venus) = get_planet(CelestialBody::Venus) {
            if matches!(
                venus.house,
                House::First | House::Fourth | House::Seventh | House::Tenth
            ) {
                yogas.push(YogaInfo {
                    yoga: Yoga::Malavya,
                    strength: 0.75,
                    involved_planets: vec![CelestialBody::Venus],
                });
            }
        }

        yogas
    }

    pub fn calculate_aspects(&self, chart: &ChartInfo) -> Vec<AspectInfo> {
        let mut aspects = Vec::new();

        for (i, planet1) in chart.planets.iter().enumerate() {
            for planet2 in chart.planets.iter().skip(i + 1) {
                let angle = (planet2.longitude - planet1.longitude).abs() % 360.0;
                let aspect = match angle {
                    a if (0.0..=10.0).contains(&a) || (350.0..=360.0).contains(&a) => {
                        Some(Aspect::Conjunction)
                    }
                    a if (170.0..=190.0).contains(&a) => Some(Aspect::Opposition),
                    a if (115.0..=125.0).contains(&a) => Some(Aspect::Trine),
                    a if (85.0..=95.0).contains(&a) => Some(Aspect::Square),
                    a if (55.0..=65.0).contains(&a) => Some(Aspect::Sextile),
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

    pub fn calculate_strengths(&self, chart: &ChartInfo) -> HashMap<CelestialBody, StrengthInfo> {
        let mut strengths = HashMap::new();

        for planet in &chart.planets {
            let shad_bala = self.calculate_shadbala(planet, chart);
            let ashtaka_varga = self.calculate_ashtakavarga(planet, chart);

            strengths.insert(
                planet.planet,
                StrengthInfo {
                    shad_bala,
                    ashtaka_varga,
                },
            );
        }

        strengths
    }

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
        strength += if planet.retrograde {
            60.0
        } else {
            30.0
        };

        // Chesta Bala (Motional Strength)
        strength += if planet.speed.abs() > 1.0 { 60.0 } else { 30.0 };

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
        strength += self
            .calculate_aspects(chart)
            .iter()
            .filter(|a| a.planet1 == planet.planet || a.planet2 == planet.planet)
            .count() as f64
            * 10.0;

        strength
    }

    fn calculate_ashtakavarga(&self, planet: &PlanetPosition, chart: &ChartInfo) -> u32 {
        let mut points = 0;

        for house in 1..=12 {
            for other_planet in &chart.planets {
                if other_planet.planet == planet.planet {
                    continue;
                }

                let angle_diff = ((other_planet.longitude - planet.longitude).abs() % 360.0).floor();
                let house_diff = ((angle_diff / 30.0).floor() as usize) % 12;

                points += match (planet.planet, other_planet.planet, house_diff) {
                    // Example rules for Ashtakavarga; needs detailed implementation
                    (CelestialBody::Sun, _, 0..=11) => 1,
                    (CelestialBody::Moon, _, 0..=11) => 1,
                    (CelestialBody::Mars, _, 0..=11) => 1,
                    (CelestialBody::Mercury, _, 0..=11) => 1,
                    (CelestialBody::Jupiter, _, 0..=11) => 1,
                    (CelestialBody::Venus, _, 0..=11) => 1,
                    (CelestialBody::Saturn, _, 0..=11) => 1,
                    (CelestialBody::Rahu, _, 0..=11) => 1,
                    (CelestialBody::Ketu, _, 0..=11) => 1,
                    _ => 0,
                };
            }
        }

        points
    }

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

    pub fn calculate_bhavas(&self, chart: &ChartInfo) -> Vec<BhavaInfo> {
        let mut bhavas = Vec::new();

        for (i, house) in chart.houses.iter().enumerate() {
            let bhava = House::from_index(i + 1).unwrap();
            let lord = self.get_house_lord(house.sign);
            let planets = chart
                .planets
                .iter()
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

    pub fn calculate_transits(&self, birth_info: &BirthInfo, period: ChronoDuration) -> Vec<TransitInfo> {
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

    fn get_zodiac_sign_for_planet(&self, planet: CelestialBody, jd: JulianDay) -> ZodiacSign {
        let result = self.calculate(CoordinateSystem::Tropical, jd, planet, &[]);
        if let Ok(AstronomicalResult::CelestialBody(info)) = result {
            Self::get_zodiac_sign(info.longitude)
        } else {
            ZodiacSign::Aries // Default to Aries if calculation fails
        }
    }

    pub fn calculate_varshaphal(
        &self,
        birth_info: &BirthInfo,
        year: i32,
    ) -> Option<VarshaphalInfo> {
        let birth_jd = date_to_julian_day(birth_info.date_time);
        let solar_return_jd = self.find_solar_return(birth_jd, year)?;

        let ascendant = self
            .calculate_ascendant(
                CoordinateSystem::Tropical,
                solar_return_jd,
                birth_info.latitude,
                birth_info.longitude,
                ChartType::Rasi,
            )
            .ok()?;

        let planets = self
            .calculate_planet_positions(
                CoordinateSystem::Tropical,
                solar_return_jd,
                ChartType::Rasi,
                birth_info,
            )
            .ok()?;

        Some(VarshaphalInfo {
            year,
            ascendant: ascendant.sign,
            planets,
        })
    }

    fn find_solar_return(&self, birth_jd: JulianDay, target_year: i32) -> Option<JulianDay> {
        let birth_sun_long = self
            .calculate(
                CoordinateSystem::Tropical,
                birth_jd,
                CelestialBody::Sun,
                &[],
            )
            .ok()
            .and_then(|r| match r {
                AstronomicalResult::CelestialBody(info) => Some(info.longitude),
                _ => None,
            })?;

        // Approximate the solar return by adding target_year years to birth_jd
        let mut low = birth_jd + (target_year as f64) * 365.25;
        let mut high = low + 366.0; // Maximum one solar year

        let mut solar_return_jd = None;

        while high - low > 0.0001 {
            let mid = (low + high) / 2.0;
            let mid_sun_long = self
                .calculate(CoordinateSystem::Tropical, mid, CelestialBody::Sun, &[])
                .ok()
                .and_then(|r| match r {
                    AstronomicalResult::CelestialBody(info) => Some(info.longitude),
                    _ => None,
                })?;

            let diff = (mid_sun_long - birth_sun_long).abs();
            if diff < 0.001 {
                solar_return_jd = Some(mid);
                break;
            } else if mid_sun_long < birth_sun_long {
                low = mid;
            } else {
                high = mid;
            }
        }

        solar_return_jd
    }

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
            .ok_or(CalculationError{
                code: 1,
                message: "Missing planet".to_string(),
            }).unwrap();
        let moon2 = chart2
            .planets
            .iter()
            .find(|p| p.planet == CelestialBody::Moon)
            .ok_or(CalculationError{
                code: 1,
                message: "Missing planet".to_string(),
            }).unwrap();

        let nakshatra1 = moon1.nakshatra.nakshatra as u32;
        let nakshatra2 = moon2.nakshatra.nakshatra as u32;

        let tara = ((nakshatra2 - nakshatra1 + 27) % 27) / 3;

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
            .ok_or(CalculationError{
                code: 1,
                message: "Missing planet".to_string(),
            }).unwrap();
        let moon2 = chart2
            .planets
            .iter()
            .find(|p| p.planet == CelestialBody::Moon)
            .ok_or(CalculationError{
                code: 1,
                message: "Missing planet".to_string(),
            }).unwrap();

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
        let lord1 = self.get_house_lord(chart1.ascendant);
        let lord2 = self.get_house_lord(chart2.ascendant);

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
                vec![CelestialBody::Sun, CelestialBody::Moon, CelestialBody::Mars],
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
            ZodiacSign::Aries | ZodiacSign::Cancer | ZodiacSign::Libra | ZodiacSign::Capricorn => {
                "Aadi"
            }
            ZodiacSign::Taurus | ZodiacSign::Leo | ZodiacSign::Scorpio | ZodiacSign::Aquarius => {
                "Madhya"
            }
            ZodiacSign::Gemini | ZodiacSign::Virgo | ZodiacSign::Sagittarius | ZodiacSign::Pisces => {
                "Antya"
            }
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
        planet.retrograde || self.is_debilitated(planet) || self.is_combust(planet)
    }

    fn is_combust(&self, planet: &PlanetPosition) -> bool {
        if planet.planet == CelestialBody::Sun {
            return false;
        }

        let sun_position = self
            .calculate(
                CoordinateSystem::Tropical,
                date_to_julian_day(Utc::now()),
                CelestialBody::Sun,
                &[],
            )
            .unwrap_or(AstronomicalResult::CelestialBody(CelestialBodyInfo {
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
                "{}: {:?} in {:?} (House {:?})\n",
                self.get_body_name(planet.planet),
                planet.sign,
                planet.nakshatra.nakshatra,
                planet.house
            ));
        }

        interpretation.push_str("\nAscendant: ");
        interpretation.push_str(&format!("{}\n", report.charts[0].ascendant));

        interpretation.push_str("\nYogas:\n");
        for yoga in &report.yogas {
            interpretation.push_str(&format!(
                "{:?} Yoga (Strength: {:.2})\n",
                yoga.yoga, yoga.strength
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

    fn calculate_divisional_charts(&self, chart: &ChartInfo) -> Vec<DivisionalChart> {
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
            ascendant: chart.ascendant,
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
            ascendant: ZodiacSign::from_longitude((chart.ascendant as u8 as f64 * 2.0) % 360.0),
            houses: [ZodiacSign::Aries; 12], // Placeholder, actual calculation needed
            planets: d2_planets,
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

// Utility Functions
pub fn date_to_julian_day(date_time: DateTime<Utc>) -> JulianDay {
    let year = date_time.year();
    let month = date_time.month();
    let day = date_time.day();
    let hour = date_time.hour();
    let minute = date_time.minute();
    let second = date_time.second() as f64 + (date_time.nanosecond() as f64 / 1_000_000_000.0);

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

    Utc.ymd(year, month as u32, day as u32).and_hms_micro(
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
        chart_type,
    )?;

    let houses = eph.calculate_houses(
        coord_system,
        julian_day,
        birth_info.latitude,
        birth_info.longitude,
        chart_type,
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

pub fn generate_vedic_astrology_report(
    eph: &SwissEph,
    birth_info: BirthInfo,
) -> Result<Report, CalculationError> {
    let julian_day = date_to_julian_day(birth_info.date_time);
    let ayanamsa = calculate_ayanamsa(julian_day);
    let rasi_chart = calculate_chart(eph, &birth_info, ChartType::Rasi)?;
    let dashas = eph.calculate_dasha(&birth_info)?;
    let planetary_states = eph.calculate_planetary_states(&rasi_chart)?;
    let divisional_charts = eph.calculate_divisional_charts(&rasi_chart);
    let yogas = eph.calculate_yogas(&rasi_chart);
    let strengths = eph.calculate_strengths(&rasi_chart);
    let remedial_measures = eph.suggest_remedial_measures(&rasi_chart);

    let nakshatras = rasi_chart.planets.iter().map(|p| p.nakshatra.clone()).collect();

    // Placeholder implementations for special_lagnas, upagrahas, and sensitive_points
    let special_lagnas = HashMap::new();
    let upagrahas = HashMap::new();
    let sensitive_points = HashMap::new();

    Ok(Report {
        birth_info,
        ayanamsa,
        charts: vec![rasi_chart],
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

 
use boa::{Context, JsValue};
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use tera::Tera;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::{CStr, CString};
use std::{fmt, fs};
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

/// Enumerates the celestial bodies used in Vedic astrology.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum CelestialBody {
    Sun = SE_SUN as i32,
    Moon = SE_MOON as i32,
    Mars = SE_MARS as i32,
    Mercury = SE_MERCURY as i32,
    Jupiter = SE_JUPITER as i32,
    Venus = SE_VENUS as i32,
    Saturn = SE_SATURN as i32,
    Rahu = SE_MEAN_NODE as i32,
    Ketu = 999, // Special handling for Ketu
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

/// Represents the twelve zodiac signs.
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
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(ZodiacSign::Aries),
            1 => Some(ZodiacSign::Taurus),
            2 => Some(ZodiacSign::Gemini),
            3 => Some(ZodiacSign::Cancer),
            4 => Some(ZodiacSign::Leo),
            5 => Some(ZodiacSign::Virgo),
            6 => Some(ZodiacSign::Libra),
            7 => Some(ZodiacSign::Scorpio),
            8 => Some(ZodiacSign::Sagittarius),
            9 => Some(ZodiacSign::Capricorn),
            10 => Some(ZodiacSign::Aquarius),
            11 => Some(ZodiacSign::Pisces),
            _ => None,
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            ZodiacSign::Aries => ZodiacSign::Libra,
            ZodiacSign::Taurus => ZodiacSign::Scorpio,
            ZodiacSign::Gemini => ZodiacSign::Sagittarius,
            ZodiacSign::Cancer => ZodiacSign::Capricorn,
            ZodiacSign::Leo => ZodiacSign::Aquarius,
            ZodiacSign::Virgo => ZodiacSign::Pisces,
            ZodiacSign::Libra => ZodiacSign::Aries,
            ZodiacSign::Scorpio => ZodiacSign::Taurus,
            ZodiacSign::Sagittarius => ZodiacSign::Gemini,
            ZodiacSign::Capricorn => ZodiacSign::Cancer,
            ZodiacSign::Aquarius => ZodiacSign::Leo,
            ZodiacSign::Pisces => ZodiacSign::Virgo,
        }
    }
}

/// Enumerates the 27 nakshatras (lunar constellations).
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

/// Enumerates the 9 dasha cycles (planetary periods).
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

/// Enumerates various types of yogas (combinations of planets).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Yoga {
    Raj,
    Gajakesari,
    Budhaditya,
    Adhi,
    Vesi,
    Vaidhritya,
    Sakata,
    Amala,
    Parvata,
    Kaal,
    Hamsa,
    Malavya,
    Shash,
    Ruchaka,
    Bhadra,
    Sreenatha,
    Aswini,
    Sunapha,
    Anapha,
    Duradhara,
    Kemadruma,
    Vallaki,
    Vayushakti,
    Akriti,
    Shiva,
    Siddha,
    Saraswati,
    Sarala,
    Vimala,
    Chandra,
}

/// Enumerates the possible states a planet can have.
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
    DeepFall,
    Combusted,
    Retrograde,
    Direct,
    Stationary,
    FastMoving,
    SlowMoving,
    Vargottama,
    AstangataBeginning,
    AstangataDeep,
    AstangataEnding,
    Benefic,
    Malefic,
    TemporaryBenefic,
    TemporaryMalefic,
    Pushkara,
    Deprived,
    Digbala,
    Chestabala,
    Ojayugmarasyamsa,
    Kendradipatya,
    Mrityu,
    Yuddha,
}

/// Enumerates different types of astrological charts.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChartType {
    Rasi,
    Navamsa,
    Drekkana,
    Chaturthaamsa,
    Saptamsa,
    Dasamsa,
    Dwadasamsa,
    Shodasamsa,
    Vimshamsa,
    Chaturvimshamsa,
    Saptavimshamsa,
    Trimshamsa,
    Khavedamsa,
    Akshavedamsa,
    Shashtiamsa,
}

/// Enumerates divisional charts (D1 to D60).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DivisionalChart {
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    D10,
    D11,
    D12,
    D13,
    D14,
    D15,
    D16,
    D17,
    D18,
    D19,
    D20,
    D21,
    D22,
    D23,
    D24,
    D25,
    D26,
    D27,
    D28,
    D29,
    D30,
    D31,
    D32,
    D33,
    D34,
    D35,
    D36,
    D37,
    D38,
    D39,
    D40,
    D41,
    D42,
    D43,
    D44,
    D45,
    D46,
    D47,
    D48,
    D49,
    D50,
    D51,
    D52,
    D53,
    D54,
    D55,
    D56,
    D57,
    D58,
    D59,
    D60,
}

/// Enumerates special lagnas (ascendant points).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpecialLagna {
    Bhava,
    Hora,
    Ghati,
    Vighati,
    Pravesa,
    Sree,
    Indu,
    Nirayana,
    Tri,
    Kala,
    Varnada,
    Pada,
    Arka,
    Sudasa,
    Sudarsa,
    Yogardha,
}

/// Enumerates upagrahas (shadow planets).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Upagraha {
    Dhuma,
    Vyatipata,
    Parivesha,
    Indrachaapa,
    Upaketu,
}

/// Enumerates sensitive points.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensitivePoint {
    Gulika,
    Mandi,
    YamakantakaA,
    YamakantakaB,
    Ardhaprahara,
    Kala,
    Mrityubhaga,
    Yamagantaka,
    Dhumadi,
    Vyatipata,
    Parivesha,
    Indrachapa,
    Upaketu,
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

/// Contains Nakshatra information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NakshatraInfo {
    pub nakshatra: Nakshatra,
    pub pada: u8,
    pub lord: CelestialBody,
    pub degree: f64,
}

/// Contains Dasha periods.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Contains Yoga information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YogaInfo {
    pub yoga: Yoga,
    pub strength: f64,
    pub involved_planets: Vec<CelestialBody>,
}

/// Contains Ayanamsa information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AyanamsaInfo {
    pub ayanamsa_name: String,
    pub ayanamsa_value: f64,
}

/// Contains Chart information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartInfo {
    pub chart_type: ChartType,
    pub ascendant: ZodiacSign,
    pub houses: Vec<HousePosition>,
    pub planets: Vec<PlanetPosition>,
}

/// Contains Planet Position information.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Contains Birth Information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirthInfo {
    pub date_time: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
}

/// Contains the complete Vedic Astrology Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub birth_info: BirthInfo,
    pub ayanamsa: AyanamsaInfo,
    pub charts: Vec<ChartInfo>,
    pub dashas: DashaInfo,
    pub yogas: Vec<YogaInfo>,
    pub nakshatras: Vec<NakshatraInfo>,
    pub planetary_states: HashMap<CelestialBody, PlanetaryState>,
    pub divisional_charts: HashMap<DivisionalChart, ChartInfo>,
    pub special_lagnas: HashMap<SpecialLagna, f64>,
    pub upagrahas: HashMap<Upagraha, f64>,
    pub sensitive_points: HashMap<SensitivePoint, f64>,
}

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
        let adjusted_longitude = navamsa_sign as f64 * 30.0
            + (position_in_sign % 3.3333333333333335) * 9.0;

        adjusted_longitude
    }

    /// Determines Nakshatra information for a given longitude.
    pub fn calculate_nakshatra(&self, longitude: f64) -> NakshatraInfo {
        let normalized_longitude = longitude.rem_euclid(360.0);
        let nakshatra_index = (normalized_longitude / 13.333333333333334).floor() as usize;
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
        let pada =
            ((normalized_longitude % 13.333333333333334) / 3.3333333333333335).floor() as u8 + 1;
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
    pub fn calculate_dasha(
        &self,
        birth_info: &BirthInfo,
    ) -> Result<DashaInfo, CalculationError> {
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
            let antar_dasha_duration_days =
                maha_dasha_duration_days * (antar_dasha_years / 120.0);
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
    ) -> HashMap<CelestialBody, PlanetaryState> {
        let mut states = HashMap::new();

        // Exaltation and Debilitation degrees
        let exaltation_points = [
            (CelestialBody::Sun, 10.0, ZodiacSign::Aries),
            (CelestialBody::Moon, 3.0, ZodiacSign::Taurus),
            (CelestialBody::Mars, 28.0, ZodiacSign::Capricorn),
            (CelestialBody::Mercury, 15.0, ZodiacSign::Virgo),
            (CelestialBody::Jupiter, 5.0, ZodiacSign::Cancer),
            (CelestialBody::Venus, 27.0, ZodiacSign::Pisces),
            (CelestialBody::Saturn, 20.0, ZodiacSign::Libra),
            (CelestialBody::Rahu, 20.0, ZodiacSign::Gemini),
            (CelestialBody::Ketu, 20.0, ZodiacSign::Sagittarius),
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

            // Determine if the planet is in exaltation, debilitation, etc.
            let state = if let Some(&(ex_planet, ex_degree, ex_sign)) =
                exaltation_points.iter().find(|&&(p, _, _)| p == planet)
            {
                if sign == ex_sign {
                    if (longitude - ex_degree).abs() < 1.0 {
                        PlanetaryState::DeepExaltation
                    } else {
                        PlanetaryState::Exalted
                    }
                } else if sign == ex_sign.opposite() {
                    if (longitude - ex_degree).abs() < 1.0 {
                        PlanetaryState::DeepDebilitation
                    } else {
                        PlanetaryState::Debilitated
                    }
                } else if let Some(own_sign_list) = own_signs
                    .iter()
                    .find(|&&(p, _)| p == planet)
                    .map(|&(_, ref signs)| signs)
                {
                    if own_sign_list.contains(&sign) {
                        PlanetaryState::OwnSign
                    } else {
                        PlanetaryState::Neutral
                    }
                } else {
                    PlanetaryState::Neutral
                }
            } else {
                PlanetaryState::Neutral
            };

            // Check for retrograde motion
            let state = if planet_position.retrograde {
                PlanetaryState::Retrograde
            } else {
                state
            };

            states.insert(planet, state);
        }

        states
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
    pub fn calculate_special_lagnas(
        &self,
        birth_info: &BirthInfo,
    ) -> HashMap<SpecialLagna, f64> {
        let mut lagnas = HashMap::new();
        let julian_day = date_to_julian_day(birth_info.date_time);

        // Calculate special lagnas based on Vedic astrology principles
        // For example, Hora Lagna (Hour Ascendant)
        // This is a placeholder implementation; actual calculations would be more complex
        let hora_lagna = (julian_day * 24.0) % 360.0;
        lagnas.insert(SpecialLagna::Hora, hora_lagna);

        lagnas
    }

    /// Calculates upagrahas like Dhuma, Vyatipata.
    pub fn calculate_upagrahas(&self, birth_info: &BirthInfo) -> HashMap<Upagraha, f64> {
        let mut upagrahas = HashMap::new();
        let julian_day = date_to_julian_day(birth_info.date_time);

        // Placeholder calculations; actual calculations require specific formulas
        let dhuma = (julian_day + 133.0 + 360.0) % 360.0;
        upagrahas.insert(Upagraha::Dhuma, dhuma);

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

use boa::{Context, JsValue, property::Attribute, object::ObjectInitializer};
use chrono::Utc;
use serde_json::Value;
use tera::Tera;
use std::{fs, path::Path};

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

    let planets = eph.calculate_planet_positions(coord_system, julian_day, chart_type, birth_info)?;

    Ok(ChartInfo {
        chart_type,
        ascendant: ascendant_info.sign,
        houses,
        planets,
    })
}

/// Generates the complete Vedic Astrology Report.
pub fn generate_vedic_astrology_report(
    eph: &SwissEph,
    birth_info: BirthInfo,
) -> Result<Report, CalculationError> {
    let julian_day = date_to_julian_day(birth_info.date_time);
    let ayanamsa = calculate_ayanamsa(julian_day);
    let rasi_chart = calculate_chart(eph, &birth_info, ChartType::Rasi)?;
    let dashas = eph.calculate_dasha(&birth_info)?;
    let planetary_states = eph.calculate_planetary_states(&rasi_chart);
    let divisional_charts = eph.calculate_divisional_charts(&birth_info);
    let special_lagnas = eph.calculate_special_lagnas(&birth_info);
    let upagrahas = eph.calculate_upagrahas(&birth_info);
    let sensitive_points = eph.calculate_sensitive_points(&birth_info);

    Ok(Report {
        birth_info,
        ayanamsa,
        charts: vec![rasi_chart],
        dashas,
        yogas: Vec::new(), // Will be filled by JS scripts
        nakshatras: Vec::new(), // Will be filled by JS scripts
        planetary_states,
        divisional_charts,
        special_lagnas,
        upagrahas,
        sensitive_points,
    })
}


#[op]
fn op_update_report(state: &mut OpState, updated_report: Value) -> Result<(), AnyError> {
    let report = state.borrow_mut::<Report>();
    *report = serde_json::from_value(updated_report)?;
    Ok(())
}

fn create_extension(report: Report) -> Extension {
    Extension::builder("vedic_astrology")
        .state(move |state| {
            state.put(report.clone());
            Ok(())
        })
        .op("updateReport", op_update_report)
        .build()
}

pub fn execute_js_scripts(report: &mut Report, scripts_folder: &str) -> Result<(), AnyError> {
    let runtime = Runtime::new()?;
    let extension = create_extension(report.clone());

    runtime.block_on(async {
        let options = BootstrapOptions {
            args: vec![],
            cpu_count: 1,
            debug_flag: false,
            enable_testing_features: false,
            location: None,
            no_color: false,
            is_tty: false,
            runtime_version: "x".to_string(),
            ts_version: "x".to_string(),
            unstable: false,
        };

        let module_loader = Rc::new(FsModuleLoader);
        let create_web_worker_cb = Arc::new(|_| {
            todo!("Web workers are not supported in this example");
        });

        let mut worker = MainWorker::bootstrap_from_options(
            module_loader,
            "file:///main.js".to_string(),
            Permissions::allow_all(),
            options,
            vec![extension],
            create_web_worker_cb,
        );

        let main_module = deno_core::resolve_path("file:///main.js")?;
        let result = worker.execute_main_module(&main_module).await?;
        worker.run_event_loop(false).await?;

        // Retrieve the updated report from the Deno runtime
        let updated_report: Report = worker
            .js_runtime
            .op_state()
            .borrow()
            .try_borrow::<Report>()
            .ok_or_else(|| AnyError::msg("Failed to retrieve updated report"))?
            .clone();

        *report = updated_report;

        Ok(result)
    })?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eph = SwissEph::new();
    let birth_info = BirthInfo {
        date_time: Utc.ymd(1991, 6, 18).and_hms(7, 10, 0),
        latitude: 11.2588,  // Latitude for Coimbatore, India
        longitude: 75.7804, // Longitude for Coimbatore, India
    };

    let mut report = generate_vedic_astrology_report(&eph, birth_info)?;

    execute_js_scripts(&mut report, "./rules")?;

    let tera = Tera::new("templates/**/*")?;
    let mut context = tera::Context::new();
    context.insert("report", &report);

    let html = tera.render("report.html", &context)?;
    println!("{}", html);

    Ok(())
}
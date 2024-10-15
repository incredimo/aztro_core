// src/main.rs

use chrono::{DateTime, Datelike, Duration as ChronoDuration, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::ffi::{CStr, CString};
use std::io::Cursor;
use std::os::raw::{c_char, c_double, c_int};
use std::sync::Once;
use tempfile::NamedTempFile;

mod ephemeris;
use ephemeris::*;
///! basic structs for astrological calculations

// Enum Definitions
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
    Hora, 
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
pub struct CelestialCoordinates {
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
    pub location: Location,
}

impl BirthInfo {
    pub fn generate_report(&self) -> Result<Report, CalculationError> {
        if let Ok(eph) = SwissEph::new() {
            generate_vedic_astrology_report(&eph, self.clone())
        } else {
            Err(CalculationError {
                code: -1,
                message: "Failed to initialize Swiss Ephemeris".to_string(),
            })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}

impl Location {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Location { latitude, longitude }
    }

    pub fn latitude(&self) -> f64 {
        self.latitude
    }

    pub fn longitude(&self) -> f64 {
        self.longitude
    }

 
    pub fn kannur() -> Self { Location { latitude: 11.8740, longitude: 75.3600 }  }
    pub fn delhi() -> Self { Location { latitude: 28.6139, longitude: 77.2090 }  }
    pub fn mumbai() -> Self { Location { latitude: 19.0760, longitude: 72.8777 }  }
    pub fn bangalore() -> Self { Location { latitude: 12.9716, longitude: 77.5946 }  }
    pub fn chennai() -> Self { Location { latitude: 13.0827, longitude: 80.2707 }  }
 
    pub fn kolkata() -> Self { Location { latitude: 22.5052, longitude: 87.3616 }  }
    pub fn abu_dhabi() -> Self { Location { latitude: 24.4667, longitude: 54.3667 }  }
    pub fn dubai() -> Self { Location { latitude: 25.276987, longitude: 55.296234 }  }
    pub fn sharjah() -> Self { Location { latitude: 25.3550, longitude: 55.4000 }  }
    pub fn malappuram() -> Self { Location { latitude: 10.7900, longitude: 76.0700 }  }
    pub fn kochi() -> Self { Location { latitude: 9.9312, longitude: 76.2673 }  }
    pub fn kollam() -> Self { Location { latitude: 8.8857, longitude: 76.5881 }  }
    pub fn thrissur() -> Self { Location { latitude: 10.522, longitude: 76.2100 }  }
    pub fn kozhikode() -> Self { Location { latitude: 11.2588, longitude: 75.7804 }  }
    pub fn wayanad() -> Self { Location { latitude: 11.6900, longitude: 75.8900 }  }
    pub fn munnar() -> Self { Location { latitude: 10.0000, longitude: 77.0667 }  }
    pub fn idukki() -> Self { Location { latitude: 10.0000, longitude: 77.0667 }  }
    pub fn kottayam() -> Self { Location { latitude: 10.0000, longitude: 76.5000 }  }
    pub fn alappuzha() -> Self { Location { latitude: 9.4900, longitude: 76.3200 }  }
 
 
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
    CelestialBody(CelestialCoordinates),
    House(HousePosition),
    Nakshatra(NakshatraInfo),
    Dasha(DashaInfo),
    Yoga(YogaInfo),
    Planet(PlanetPosition),
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
        birth_info.location.latitude,
        birth_info.location.longitude,
        chart_type,
    )?;

    let houses = eph.calculate_houses(
        coord_system,
        julian_day,
        birth_info.location.latitude,
        birth_info.location.longitude,
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

// Example Usage
fn main() -> Result<(), Box<dyn Error>> {
    // Initialize Swiss Ephemeris
    let eph = SwissEph::new()?;

    // Define Birth Information
    let birth_info = BirthInfo {
        date_time: Utc.ymd(1990, 5, 15).and_hms(10, 30, 0),
        location: Location {
            latitude: 28.6139,   // Example: New Delhi Latitude
            longitude: 77.2090,  // Example: New Delhi Longitude
        },
    };

    // Generate Astrology Report
    let report = generate_vedic_astrology_report(&eph, birth_info).unwrap();

    // Print Interpretation
    let interpretation = eph.generate_interpretation(&report);
    println!("{}", interpretation);

    Ok(())
}

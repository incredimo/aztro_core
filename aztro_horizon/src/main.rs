use chrono::{DateTime, Utc, TimeZone, NaiveDateTime};
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::error::Error;
use urlencoding::encode;

// Define a struct to represent each planet with its Horizons ID
struct Planet {
    name: &'static str,
    horizons_id: &'static str,
}

// List of planets and lunar nodes used in Vedic astrology with their corresponding Horizons IDs
// Note: The Horizons IDs for Rahu and Ketu are assumed as '-99' and '-100'. Verify with Horizons documentation.
const PLANETS: &[Planet] = &[
    Planet { name: "Sun", horizons_id: "'10'" },
    Planet { name: "Moon", horizons_id: "'301'" },
    Planet { name: "Mercury", horizons_id: "'199'" },
    Planet { name: "Venus", horizons_id: "'299'" },
    Planet { name: "Mars", horizons_id: "'499'" },
    Planet { name: "Jupiter", horizons_id: "'599'" },
    Planet { name: "Saturn", horizons_id: "'699'" },
    Planet { name: "Rahu", horizons_id: "'-99'" }, // Ascending Node
    Planet { name: "Ketu", horizons_id: "'-100'" }, // Descending Node
];

// Define Zodiac signs in sidereal astrology
const ZODIAC_SIGNS: &[&str] = &[
    "Aries", "Taurus", "Gemini", "Cancer", "Leo", "Virgo",
    "Libra", "Scorpio", "Sagittarius", "Capricorn", "Aquarius", "Pisces",
];

// Define Nakshatras in Vedic astrology
const NAKSHATRAS: &[&str] = &[
    "Ashwini", "Bharani", "Krittika", "Rohini", "Mrigashirsha",
    "Ardra", "Punarvasu", "Pushya", "Ashlesha", "Magha",
    "Purva Phalguni", "Uttara Phalguni", "Hasta", "Chitra",
    "Swati", "Vishakha", "Anuradha", "Jyeshtha", "Mula",
    "Purva Ashadha", "Uttara Ashadha", "Shravana", "Dhanistha",
    "Shatabhisha", "Purva Bhadrapada", "Uttara Bhadrapada", "Revati",
];

// Define the ayanamsa value (Lahiri) in degrees
// Note: Ayanamsa varies over time; for precise calculations, implement a dynamic method
const AYANAMSA: f64 = 23.856; // Approximate value

// Define the structure of the JSON response from Horizons API
#[derive(Debug, Deserialize)]
struct HorizonsResponse {
    signature: Signature,
    result: String,
}

#[derive(Debug, Deserialize)]
struct Signature {
    source: String,
    version: String,
}

// Structure to hold the ecliptic longitude
struct EclipticPosition {
    longitude: f64,    // in degrees
    declination: f64,  // in degrees
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Collect command-line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        eprintln!("Usage: {} <YYYY-MM-DD HH:MM:SS> <Latitude> <Longitude> <TimeZoneOffset>", args[0]);
        eprintln!("Example: {} \"2024-10-07 12:00:00\" 28.6139 77.2090 +05:30", args[0]);
        return Ok(());
    }

    // Parse input arguments
    let datetime_str = &args[1];
    let latitude: f64 = args[2].parse()?;
    let longitude: f64 = args[3].parse()?;
    let timezone_offset_str = &args[4];

    // Parse the datetime string with timezone offset
    let datetime_with_offset = format!("{} {}", datetime_str, timezone_offset_str);
    let local_datetime = DateTime::parse_from_str(&datetime_with_offset, "%Y-%m-%d %H:%M:%S %z")
        .map_err(|e| format!("Error parsing datetime: {}", e))?;
    
    // Convert to UTC
    let utc_datetime: DateTime<Utc> = local_datetime.with_timezone(&Utc);

    // Extract date and time components
    let date_str = utc_datetime.format("%Y-%m-%d").to_string();
    let time_str = utc_datetime.format("%H:%M:%S").to_string();

    // Initialize HTTP client
    let client = Client::new();

    println!("Natal Chart for:");
    println!("Date & Time (UTC): {}", utc_datetime);
    println!("Location: Latitude {}, Longitude {}", latitude, longitude);
    println!("----------------------------------------");

    // Iterate over each planet and fetch its position
    for planet in PLANETS {
        match fetch_planet_position(&client, planet, &date_str, &time_str).await {
            Ok(position) => {
                // Apply ayanamsa to get sidereal longitude
                let mut sidereal_longitude = position.longitude - AYANAMSA;
                if sidereal_longitude < 0.0 {
                    sidereal_longitude += 360.0;
                }

                // Determine zodiac sign
                let sign_index = (sidereal_longitude / 30.0).floor() as usize % 12;
                let sign = ZODIAC_SIGNS[sign_index];
                let degree = sidereal_longitude % 30.0;

                // Determine Nakshatra
                let nakshatra_index = (sidereal_longitude / (360.0 / 27.0)).floor() as usize % 27;
                let nakshatra = NAKSHATRAS[nakshatra_index];
                let degree_in_nakshatra = sidereal_longitude % (360.0 / 27.0);

                println!("{}:", planet.name);
                println!("  Sidereal Longitude: {:.2}°", sidereal_longitude);
                println!("  Zodiac Sign: {} ({:.2}° in {})", sign, degree, sign);
                println!("  Nakshatra: {} ({:.2}° in {})", nakshatra, degree_in_nakshatra, nakshatra);
                println!("");
            },
            Err(e) => {
                eprintln!("Error fetching position for {}: {}", planet.name, e);
            }
        }
    }

    Ok(())
}

// Function to fetch the ecliptic position of a planet from Horizons API
async fn fetch_planet_position(client: &Client, planet: &Planet, date: &str, time: &str) -> Result<EclipticPosition, Box<dyn Error>> {
    // Build the query parameters
    let command = planet.horizons_id;
    let format = "json";
    let obj_data = "YES";
    let make_ephem = "YES";
    let ephem_type = "OBSERVER";
    let center = "500@399"; // Geocentric
    let start_time = format!("{} {}", date, time);
    let stop_time = format!("{} {}", date, time);
    let step_size = "1m"; // 1 minute
    let quantities = "1,9,20,23,24,29"; // Specify desired quantities

    // Encode the COMMAND parameter
    let encoded_command = encode(command);

    // Build the full URL
    let url = format!(
        "https://ssd.jpl.nasa.gov/api/horizons.api?format={}&COMMAND={}&OBJ_DATA={}&MAKE_EPHEM={}&EPHEM_TYPE={}&CENTER={}&START_TIME='{}'&STOP_TIME='{}'&STEP_SIZE='{}'&QUANTITIES='{}'",
        format,
        encoded_command,
        obj_data,
        make_ephem,
        ephem_type,
        center,
        encode(&start_time),
        encode(&stop_time),
        encode(step_size),
        encode(quantities)
    );

    // Make the GET request
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(format!("HTTP request failed with status: {}", response.status()).into());
    }

    // Parse the JSON response
    let horizons_response: HorizonsResponse = response.json().await?;

    // Extract the ecliptic longitude and declination from the result string
    // Note: Horizons API returns ephemeris data as plain text within the JSON "result" field
    // We'll need to parse this text to extract the required information

    // Split the result into lines
    let lines: Vec<&str> = horizons_response.result.lines().collect();

    // Find the line that contains the ephemeris data
    // Typically, it's after the "$$SOE" marker
    let soe_index = lines.iter().position(|&line| line.contains("$$SOE"));
    let eoe_index = lines.iter().position(|&line| line.contains("$$EOE"));

    if let (Some(soe), Some(eoe)) = (soe_index, eoe_index) {
        // Iterate over the lines between $$SOE and $$EOE
        for line in &lines[soe + 1..eoe] {
            // Each line contains date, RA, DEC, etc.
            // Example line:
            // 2024-Oct-07 12:00     10 15 30.00  20 30 40.0    ... other columns
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 9 {
                continue; // Skip malformed lines
            }

            // Extract RA and DEC
            // RA is in parts[2], parts[3], parts[4]
            // DEC is in parts[5], parts[6], parts[7]
            let ra_h = parts[2];
            let ra_m = parts[3];
            let ra_s = parts[4];
            let dec_sign = parts[5].chars().next().unwrap_or('+');
            let dec_d = parts[5][1..].to_string();
            let dec_m = parts[6];
            let dec_s = parts[7];

            // Convert RA to degrees
            let ra_hours: f64 = ra_h.parse()?;
            let ra_minutes: f64 = ra_m.parse()?;
            let ra_seconds: f64 = ra_s.parse()?;
            let ra_degrees = (ra_hours + ra_minutes / 60.0 + ra_seconds / 3600.0) * 15.0;

            // Convert DEC to degrees
            let dec_degrees: f64 = dec_d.parse::<f64>()? + dec_m.parse::<f64>()? / 60.0 + dec_s.parse::<f64>()? / 3600.0;
            let dec_degrees = if dec_sign == '-' { -dec_degrees } else { dec_degrees };

            // For sidereal astrology, we need ecliptic longitude. Horizons provides equatorial coordinates (RA/DEC).
            // We need to convert RA/DEC to ecliptic longitude. This requires knowing the obliquity of the ecliptic.

            let obliquity_deg: f64 = 23.439291; // Obliquity of the ecliptic for J2000.0

            // Convert degrees to radians
            let ra_rad = ra_degrees.to_radians();
            let dec_rad = dec_degrees.to_radians();
            let obliquity_rad = obliquity_deg.to_radians();

            // Calculate ecliptic longitude using the formula:
            // tan(lambda) = (sin(alpha) * cos(epsilon) + tan(delta) * sin(epsilon)) / cos(alpha)
            let lambda_rad = (ra_rad.sin() * obliquity_rad.cos() + dec_rad.tan() * obliquity_rad.sin()).atan2(ra_rad.cos());

            // Convert ecliptic longitude to degrees
            let mut lambda_deg = lambda_rad.to_degrees();
            if lambda_deg < 0.0 {
                lambda_deg += 360.0;
            }

            // Return the ecliptic position
            return Ok(EclipticPosition {
                longitude: lambda_deg,
                declination: dec_degrees,
            });
        }

        Err("Failed to parse ephemeris data".into())
    } else {
        Err("Ephemeris data markers not found".into())
    }
}

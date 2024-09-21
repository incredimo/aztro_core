use std::sync::Arc;

use super::*;

impl SwissEph {
    /// Identifies and calculates yogas based on Vedic astrology principles.
    pub fn calculate_yogas(&self, chart_info: &ChartInfo) -> Vec<YogaInfo> {
        let mut yogas = Vec::new();

        // Helper function to determine if a planet is in Kendra (1, 4, 7, 10 houses)
        let is_in_kendra = |house: House| {
            matches!(
                house,
                House::First | House::Fourth | House::Seventh | House::Tenth
            )
        };

        // Helper function to determine if a planet is in own sign or exalted
        let is_in_own_sign_or_exalted = |planet: CelestialBody, sign: ZodiacSign| {
            let own_signs = match planet {
                CelestialBody::Sun => vec![ZodiacSign::Leo],
                CelestialBody::Moon => vec![ZodiacSign::Cancer],
                CelestialBody::Mars => vec![ZodiacSign::Aries, ZodiacSign::Scorpio],
                CelestialBody::Mercury => vec![ZodiacSign::Gemini, ZodiacSign::Virgo],
                CelestialBody::Jupiter => vec![ZodiacSign::Sagittarius, ZodiacSign::Pisces],
                CelestialBody::Venus => vec![ZodiacSign::Taurus, ZodiacSign::Libra],
                CelestialBody::Saturn => vec![ZodiacSign::Capricorn, ZodiacSign::Aquarius],
                _ => vec![],
            };
            let exalted_sign = match planet {
                CelestialBody::Sun => ZodiacSign::Aries,
                CelestialBody::Moon => ZodiacSign::Taurus,
                CelestialBody::Mars => ZodiacSign::Capricorn,
                CelestialBody::Mercury => ZodiacSign::Virgo,
                CelestialBody::Jupiter => ZodiacSign::Cancer,
                CelestialBody::Venus => ZodiacSign::Pisces,
                CelestialBody::Saturn => ZodiacSign::Libra,
                _ => ZodiacSign::Aries, // Default placeholder
            };
            own_signs.contains(&sign) || sign == exalted_sign
        };

        // Gajakesari Yoga: Jupiter in Kendra from Moon
        if let Some(moon) = chart_info
            .planets
            .iter()
            .find(|p| p.planet == CelestialBody::Moon)
        {
            if let Some(jupiter) = chart_info
                .planets
                .iter()
                .find(|p| p.planet == CelestialBody::Jupiter)
            {
                let moon_house_num = moon.house as u8;
                let jupiter_house_num = jupiter.house as u8;
                let diff = (12 + jupiter_house_num - moon_house_num) % 12;
                if diff == 0 || diff == 4 || diff == 7 || diff == 10 {
                    yogas.push(YogaInfo {
                        yoga: Yoga::Gajakesari,
                        strength: 1.0,
                        involved_planets: vec![CelestialBody::Moon, CelestialBody::Jupiter],
                    });
                }
            }
        }

        // Budhaditya Yoga: Sun and Mercury in the same house
        if let Some(sun) = chart_info
            .planets
            .iter()
            .find(|p| p.planet == CelestialBody::Sun)
        {
            if let Some(mercury) = chart_info
                .planets
                .iter()
                .find(|p| p.planet == CelestialBody::Mercury)
            {
                if sun.house == mercury.house {
                    yogas.push(YogaInfo {
                        yoga: Yoga::Budhaditya,
                        strength: 1.0,
                        involved_planets: vec![CelestialBody::Sun, CelestialBody::Mercury],
                    });
                }
            }
        }

        // Panch Mahapurusha Yogas
        for planet in &[
            CelestialBody::Mars,
            CelestialBody::Mercury,
            CelestialBody::Jupiter,
            CelestialBody::Venus,
            CelestialBody::Saturn,
        ] {
            if let Some(p) = chart_info.planets.iter().find(|p| p.planet == *planet) {
                if is_in_kendra(p.house) && is_in_own_sign_or_exalted(*planet, p.sign) {
                    let yoga = match planet {
                        CelestialBody::Mars => Yoga::Ruchaka,
                        CelestialBody::Mercury => Yoga::Bhadra,
                        CelestialBody::Jupiter => Yoga::Hamsa,
                        CelestialBody::Venus => Yoga::Malavya,
                        CelestialBody::Saturn => Yoga::Shash,
                        _ => continue,
                    };
                    yogas.push(YogaInfo {
                        yoga,
                        strength: 1.0,
                        involved_planets: vec![*planet],
                    });
                }
            }
        }

        // Adhi Yoga: Benefic planets in 6th, 7th, and 8th houses from Moon
        if let Some(moon) = chart_info
            .planets
            .iter()
            .find(|p| p.planet == CelestialBody::Moon)
        {
            let benefics = [
                CelestialBody::Jupiter,
                CelestialBody::Venus,
                CelestialBody::Mercury,
            ];
            let houses_from_moon = |offset: u8| ((moon.house as u8 + offset - 1) % 12) + 1;
            let mut benefic_planets_in_positions = vec![];

            for &benefic in &benefics {
                if let Some(planet) = chart_info.planets.iter().find(|p| p.planet == benefic) {
                    let planet_house_num = planet.house as u8;
                    if planet_house_num == houses_from_moon(6)
                        || planet_house_num == houses_from_moon(7)
                        || planet_house_num == houses_from_moon(8)
                    {
                        benefic_planets_in_positions.push(benefic);
                    }
                }
            }

            if !benefic_planets_in_positions.is_empty() {
                yogas.push(YogaInfo {
                    yoga: Yoga::Adhi,
                    strength: 1.0,
                    involved_planets: benefic_planets_in_positions,
                });
            }
        }

        // Chandra Mangala Yoga: Moon and Mars conjunction or mutual aspect
        if let Some(moon) = chart_info
            .planets
            .iter()
            .find(|p| p.planet == CelestialBody::Moon)
        {
            if let Some(mars) = chart_info
                .planets
                .iter()
                .find(|p| p.planet == CelestialBody::Mars)
            {
                if moon.house == mars.house {
                    yogas.push(YogaInfo {
                        yoga: Yoga::Chandra,
                        strength: 1.0,
                        involved_planets: vec![CelestialBody::Moon, CelestialBody::Mars],
                    });
                }
            }
        }

        // Sunapha Yoga: Planets (excluding Sun) in the 2nd house from Moon
        if let Some(moon) = chart_info
            .planets
            .iter()
            .find(|p| p.planet == CelestialBody::Moon)
        {
            let second_house_num = ((moon.house as u8) % 12) + 1;
            let mut planets_in_second = vec![];

            for planet in chart_info
                .planets
                .iter()
                .filter(|p| p.planet != CelestialBody::Sun)
            {
                if planet.house as u8 == second_house_num {
                    planets_in_second.push(planet.planet);
                }
            }

            if !planets_in_second.is_empty() {
                yogas.push(YogaInfo {
                    yoga: Yoga::Sunapha,
                    strength: 1.0,
                    involved_planets: planets_in_second,
                });
            }
        }

        // Anapha Yoga: Planets (excluding Sun) in the 12th house from Moon
        if let Some(moon) = chart_info
            .planets
            .iter()
            .find(|p| p.planet == CelestialBody::Moon)
        {
            let twelfth_house_num = ((moon.house as u8 + 10) % 12) + 1;
            let mut planets_in_twelfth = vec![];

            for planet in chart_info
                .planets
                .iter()
                .filter(|p| p.planet != CelestialBody::Sun)
            {
                if planet.house as u8 == twelfth_house_num {
                    planets_in_twelfth.push(planet.planet);
                }
            }

            if !planets_in_twelfth.is_empty() {
                yogas.push(YogaInfo {
                    yoga: Yoga::Anapha,
                    strength: 1.0,
                    involved_planets: planets_in_twelfth,
                });
            }
        }

        // Durudhara Yoga: Planets (excluding Sun) in both 2nd and 12th houses from Moon
        if yogas.iter().any(|y| y.yoga == Yoga::Sunapha)
            && yogas.iter().any(|y| y.yoga == Yoga::Anapha)
        {
            let involved_planets: Vec<_> = yogas
                .iter()
                .filter(|y| y.yoga == Yoga::Sunapha || y.yoga == Yoga::Anapha)
                .flat_map(|y| y.involved_planets.clone())
                .collect();

            yogas.push(YogaInfo {
                yoga: Yoga::Duradhara,
                strength: 1.0,
                involved_planets,
            });
        }

        // Kemadruma Yoga: No planets (excluding Sun, Rahu, Ketu) in 2nd and 12th houses from Moon
        if let Some(moon) = chart_info
            .planets
            .iter()
            .find(|p| p.planet == CelestialBody::Moon)
        {
            let second_house_num = ((moon.house as u8) % 12) + 1;
            let twelfth_house_num = ((moon.house as u8 + 10) % 12) + 1;

            let planets_in_second_or_twelfth = chart_info
                .planets
                .iter()
                .filter(|p| {
                    p.planet != CelestialBody::Sun
                        && p.planet != CelestialBody::Rahu
                        && p.planet != CelestialBody::Ketu
                })
                .any(|p| {
                    let house_num = p.house as u8;
                    house_num == second_house_num || house_num == twelfth_house_num
                });

            if !planets_in_second_or_twelfth {
                yogas.push(YogaInfo {
                    yoga: Yoga::Kemadruma,
                    strength: 1.0,
                    involved_planets: vec![CelestialBody::Moon],
                });
            }
        }

        // Parvata Yoga: Benefic planets in Kendras and malefic planets in upachaya houses (3, 6, 10, 11)
        let benefic_planets = [
            CelestialBody::Jupiter,
            CelestialBody::Venus,
            CelestialBody::Mercury,
        ];
        let malefic_planets = [
            CelestialBody::Saturn,
            CelestialBody::Mars,
            CelestialBody::Sun,
        ];

        let benefics_in_kendra = chart_info
            .planets
            .iter()
            .filter(|p| benefic_planets.contains(&p.planet) && is_in_kendra(p.house))
            .count();

        let malefics_in_upachaya = chart_info
            .planets
            .iter()
            .filter(|p| {
                malefic_planets.contains(&p.planet)
                    && matches!(
                        p.house,
                        House::Third | House::Sixth | House::Tenth | House::Eleventh
                    )
            })
            .count();

        if benefics_in_kendra > 0 && malefics_in_upachaya > 0 {
            yogas.push(YogaInfo {
                yoga: Yoga::Parvata,
                strength: 1.0,
                involved_planets: vec![], // Specific planets can be added if needed
            });
        }

        // Neechabhanga Raja Yoga: Cancellation of debilitation
        for planet in &chart_info.planets {
            if let Some(debilitated_sign) = match planet.planet {
                CelestialBody::Sun => Some(ZodiacSign::Libra),
                CelestialBody::Moon => Some(ZodiacSign::Scorpio),
                CelestialBody::Mars => Some(ZodiacSign::Cancer),
                CelestialBody::Mercury => Some(ZodiacSign::Pisces),
                CelestialBody::Jupiter => Some(ZodiacSign::Capricorn),
                CelestialBody::Venus => Some(ZodiacSign::Virgo),
                CelestialBody::Saturn => Some(ZodiacSign::Aries),
                _ => None,
            } {
                if planet.sign == debilitated_sign {
                    // Check for cancellation conditions
                    let cancellation = chart_info.planets.iter().any(|p| {
                        // Debilitation lord in Kendra from Lagna
                        let debilitation_lord = match planet.sign {
                            ZodiacSign::Libra => CelestialBody::Venus,
                            ZodiacSign::Scorpio => CelestialBody::Mars,
                            ZodiacSign::Cancer => CelestialBody::Moon,
                            ZodiacSign::Pisces => CelestialBody::Jupiter,
                            ZodiacSign::Capricorn => CelestialBody::Saturn,
                            ZodiacSign::Virgo => CelestialBody::Mercury,
                            ZodiacSign::Aries => CelestialBody::Mars,
                            _ => return false,
                        };
                        p.planet == debilitation_lord && is_in_kendra(p.house)
                    });

                    if cancellation {
                        yogas.push(YogaInfo {
                            yoga: Yoga::Raj,
                            strength: 1.0,
                            involved_planets: vec![planet.planet],
                        });
                    }
                }
            }
        }

        // Additional yogas can be calculated here following Vedic astrology principles

        yogas
    }
}


#[derive(Debug)]
pub struct CalculationError {
    pub message: String,
}
pub type CalculationResult<T> = Result<T, CalculationError>;

impl CalculationError {
    pub fn new(message: String) -> Self {
        CalculationError { message }
    }
}

impl std::fmt::Display for CalculationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CalculationError {
    fn description(&self) -> &str {
        &self.message
    }
}

 

impl<T> Into<CalculationResult<T>> for CalculationError {
    fn into(self) -> CalculationResult<T> {
        Err(self)
    }
}





pub type AstrologicalCalculation = Arc<dyn FnMut(&mut Report) -> CalculationResult<()>>;




#[derive(Clone)]
pub struct YogaDefinition {
    pub name: String,
    pub description: String, // explains when a yoga is considered to be active
    pub reference: String,
    pub calculation: AstrologicalCalculation,
    pub impacts: Impacts,
}

#[derive(Clone)]
pub struct Impacts(Vec<Impact>);

#[derive(Clone)]
pub struct Impact {
    pub realm: Realm,
    pub weight: i8, // -100 to 100
    pub description: String,
}

 


 macro_rules! yoga {
     ($name:ident, $description:expr, $reference:expr, $calculation:expr) => {
       const  $name: YogaDefinition =  YogaDefinition {
             name: stringify!($name).to_string(),
             description: $description.to_string(),
             reference: $reference.to_string(),
             calculation: $calculation,
         }
     };
 }


 yoga!(GajakesariYoga, 
    "As Jupiter is in Kendra from Moon, it is considered to be in a state of joy and prosperity. This yoga is considered to be very auspicious and is associated with good fortune and success in life.",
    "Brihat Jataka",
    |report: &mut Report| {
        let mut yogas = Vec::new();

        if let Some(jupiter) = report.charts[0].planets.iter().find(|p| p.planet == CelestialBody::Jupiter) {
            if let Some(moon) = report.charts[0].planets.iter().find(|p| p.planet == CelestialBody::Moon) {
                if jupiter.house == moon.house {
                    yogas.push(YogaInfo {
                        yoga: Yoga::Gajakesari, 
                        strength: 1.0,
                        involved_planets: vec![CelestialBody::Jupiter, CelestialBody::Moon],
                    });
                }
            }
        }

        Ok(())
    }
);








    












         
 
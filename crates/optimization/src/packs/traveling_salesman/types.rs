//! Types for Traveling Salesman pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct City {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TspInput {
    pub cities: Vec<City>,
}

impl TspInput {
    pub fn validate(&self) -> Result<()> {
        if self.cities.len() < 2 {
            return Err(converge_pack::GateError::invalid_input(
                "At least two cities required",
            ));
        }
        for (i, city) in self.cities.iter().enumerate() {
            if !city.x.is_finite() || !city.y.is_finite() {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "City {} has non-finite coordinates",
                    i
                )));
            }
        }
        Ok(())
    }

    pub fn distance(&self, a: usize, b: usize) -> f64 {
        let dx = self.cities[a].x - self.cities[b].x;
        let dy = self.cities[a].y - self.cities[b].y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TspOutput {
    pub tour: Vec<usize>,
    pub total_distance: f64,
}

impl TspOutput {
    pub fn summary(&self) -> String {
        format!(
            "Tour of {} cities with total distance {:.2}",
            self.tour.len(),
            self.total_distance
        )
    }
}

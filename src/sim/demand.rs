use crate::grid::{Grid, TileType};

/// RCI demand values. Positive = undersupplied (growth wanted).
/// Negative = oversupplied (potential abandonment).
#[derive(Clone, Debug, Default)]
pub struct RciDemand {
    pub residential: f32,
    pub commercial: f32,
    pub industrial: f32,
}

/// EMA smoothing factor. Higher = more reactive to changes.
const DEMAND_ALPHA: f32 = 0.3;

impl RciDemand {
    /// Compute raw demand from current grid state.
    ///
    /// R_demand = (C_jobs + I_jobs) - R_population
    /// C_demand = 0.3 * R_population - C_jobs
    /// I_demand = 0.2 * R_population - I_jobs
    pub fn compute_raw(grid: &Grid) -> Self {
        let population = grid.population() as f32;
        let (c_jobs, i_jobs) = grid.total_jobs();
        let total_jobs = (c_jobs + i_jobs) as f32;
        let c_count = grid.count_type(TileType::Commercial) as f32;
        let i_count = grid.count_type(TileType::Industrial) as f32;

        Self {
            residential: total_jobs - population,
            commercial: 0.3 * population - c_count * 5.0,
            industrial: 0.2 * population - i_count * 10.0,
        }
    }

    /// Apply EMA smoothing: new = prev * (1 - alpha) + raw * alpha
    pub fn smooth(&mut self, raw: &RciDemand) {
        self.residential = self.residential * (1.0 - DEMAND_ALPHA) + raw.residential * DEMAND_ALPHA;
        self.commercial = self.commercial * (1.0 - DEMAND_ALPHA) + raw.commercial * DEMAND_ALPHA;
        self.industrial = self.industrial * (1.0 - DEMAND_ALPHA) + raw.industrial * DEMAND_ALPHA;
    }

    /// Which zone type has the highest positive demand?
    #[allow(dead_code)]
    pub fn highest_demand(&self) -> Option<TileType> {
        let max = self.residential.max(self.commercial).max(self.industrial);
        if max <= 0.0 {
            return None;
        }
        if self.residential >= self.commercial && self.residential >= self.industrial {
            Some(TileType::ZonedResidential)
        } else if self.commercial >= self.industrial {
            Some(TileType::ZonedCommercial)
        } else {
            Some(TileType::ZonedIndustrial)
        }
    }

    /// Demand value for a specific building type.
    #[allow(dead_code)]
    pub fn for_tile(&self, tile: TileType) -> f32 {
        match tile {
            TileType::Residential | TileType::ZonedResidential => self.residential,
            TileType::Commercial | TileType::ZonedCommercial => self.commercial,
            TileType::Industrial | TileType::ZonedIndustrial => self.industrial,
            _ => 0.0,
        }
    }
}

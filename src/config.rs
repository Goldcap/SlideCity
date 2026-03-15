use serde::{Deserialize, Serialize};

/// Game difficulty presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Peaceful,
    Normal,
    Harsh,
}

impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Peaceful => "Peaceful",
            Difficulty::Normal => "Normal",
            Difficulty::Harsh => "Harsh",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Difficulty::Peaceful => "Relaxed pace, rare disasters, generous funds",
            Difficulty::Normal => "Balanced challenge, standard simulation",
            Difficulty::Harsh => "Frequent disasters, tight budget, aggressive decay",
        }
    }
}

/// Simulation parameters, derived from difficulty.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimConfig {
    pub difficulty: Difficulty,

    // Grid
    pub grid_width: usize,
    pub grid_height: usize,

    // Timing
    pub base_tick_ms: f32,
    pub ticks_per_year: u32,
    pub ticks_per_season: u32,
    pub utility_recompute_interval: u64,
    pub mayor_tick_interval: u64,
    pub audio_reeval_interval: u64,

    // Funds
    pub starting_funds: i64,
    pub tax_multiplier: f32,
    pub res_tax: i64,
    pub com_tax: i64,
    pub ind_tax: i64,

    // Fire
    pub fire_spread_prob: f32,
    pub random_fire_chance: f32,
    pub disaster_cooldown_secs: f32,

    // Decay
    pub decay_multiplier: f32,

    // Speed
    pub speed_multiplier: f32,
}

impl SimConfig {
    pub fn new(difficulty: Difficulty) -> Self {
        let (starting_funds, tax_mult, fire_spread, random_fire, decay_mult, cooldown) =
            match difficulty {
                Difficulty::Peaceful => (200_000, 1.5, 0.14, 0.0003, 0.5, 60.0),
                Difficulty::Normal => (150_000, 1.0, 0.28, 0.0006, 1.0, 30.0),
                Difficulty::Harsh => (100_000, 0.7, 0.42, 0.001, 2.0, 15.0),
            };

        Self {
            difficulty,
            grid_width: 128,
            grid_height: 128,
            base_tick_ms: 800.0,
            ticks_per_year: 200,
            ticks_per_season: 50,
            utility_recompute_interval: 5,
            mayor_tick_interval: 8,
            audio_reeval_interval: 10,
            starting_funds,
            tax_multiplier: tax_mult,
            res_tax: 3,
            com_tax: 9,
            ind_tax: 6,
            fire_spread_prob: fire_spread,
            random_fire_chance: random_fire,
            disaster_cooldown_secs: cooldown,
            decay_multiplier: decay_mult,
            speed_multiplier: 1.0,
        }
    }

    /// Effective tick duration in seconds, accounting for speed multiplier.
    pub fn tick_duration(&self) -> f32 {
        (self.base_tick_ms / 1000.0) / self.speed_multiplier
    }
}

impl Default for SimConfig {
    fn default() -> Self {
        Self::new(Difficulty::Normal)
    }
}

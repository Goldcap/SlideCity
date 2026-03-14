use ::rand::rngs::SmallRng;
use ::rand::Rng;
/// Mayor personality traits that drive all decisions.
#[derive(Clone, Debug)]
pub struct MayorPersonality {
    pub name: &'static str,
    pub emoji: &'static str,
    pub growth_aggression: f32,
    pub green_affinity: f32,
    pub industrial_bias: f32,
    pub risk_tolerance: f32,
    pub panic_threshold: f32,
    pub power: MayorPower,
    pub weakness: MayorWeakness,
}

/// Mechanical bonus unique to each archetype.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MayorPower {
    CheapRoads,           // Roads cost 50% less
    ParkFirebreak,        // Parks block fire spread
    IndustrialTaxDouble,  // Industrial tax income doubled
    AllCostDiscount,      // All costs -10%
    FastDisasterRecovery, // Disaster recovery 2x faster (rubble clears faster)
    CheapMonument,        // Monument cost halved, COM grows 50% faster
    IndustrialDouble,     // Industrial output doubled
    HappinessBoost,       // Baseline happiness +20%
}

/// Mechanical penalty unique to each archetype.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MayorWeakness {
    ParkDecayFast,         // Parks decay 2x faster
    IndustrialTaxHalved,   // Industrial tax income -50%
    ParkCostTriple,        // Park placement cost 3x
    NoSpecialBonus,        // Perfectly average (no weakness either)
    SlowExpansion,         // Expansion rate -50%
    EarlyFundDrain,        // Early-game funds drain 25% faster
    HappinessDecayFast,    // Happiness decays 2x faster
    SlowGrowth,            // Growth rate -30%
}

pub const ARCHETYPES: &[MayorPersonality] = &[
    MayorPersonality {
        name: "The Developer",
        emoji: "🏗️",
        growth_aggression: 0.9,
        green_affinity: 0.1,
        industrial_bias: 0.5,
        risk_tolerance: 0.8,
        panic_threshold: 0.2,
        power: MayorPower::CheapRoads,
        weakness: MayorWeakness::ParkDecayFast,
    },
    MayorPersonality {
        name: "The Environmentalist",
        emoji: "🌿",
        growth_aggression: 0.4,
        green_affinity: 0.9,
        industrial_bias: 0.1,
        risk_tolerance: 0.3,
        panic_threshold: 0.5,
        power: MayorPower::ParkFirebreak,
        weakness: MayorWeakness::IndustrialTaxHalved,
    },
    MayorPersonality {
        name: "The Baron",
        emoji: "🏭",
        growth_aggression: 0.8,
        green_affinity: 0.1,
        industrial_bias: 0.9,
        risk_tolerance: 0.6,
        panic_threshold: 0.3,
        power: MayorPower::IndustrialTaxDouble,
        weakness: MayorWeakness::ParkCostTriple,
    },
    MayorPersonality {
        name: "The Pragmatist",
        emoji: "📋",
        growth_aggression: 0.5,
        green_affinity: 0.5,
        industrial_bias: 0.5,
        risk_tolerance: 0.5,
        panic_threshold: 0.5,
        power: MayorPower::AllCostDiscount,
        weakness: MayorWeakness::NoSpecialBonus,
    },
    MayorPersonality {
        name: "The Nervous Mayor",
        emoji: "😰",
        growth_aggression: 0.3,
        green_affinity: 0.6,
        industrial_bias: 0.2,
        risk_tolerance: 0.1,
        panic_threshold: 0.9,
        power: MayorPower::FastDisasterRecovery,
        weakness: MayorWeakness::SlowExpansion,
    },
    MayorPersonality {
        name: "The Visionary",
        emoji: "✨",
        growth_aggression: 0.6,
        green_affinity: 0.8,
        industrial_bias: 0.3,
        risk_tolerance: 0.9,
        panic_threshold: 0.2,
        power: MayorPower::CheapMonument,
        weakness: MayorWeakness::EarlyFundDrain,
    },
    MayorPersonality {
        name: "The Machine",
        emoji: "🤖",
        growth_aggression: 1.0,
        green_affinity: 0.0,
        industrial_bias: 1.0,
        risk_tolerance: 1.0,
        panic_threshold: 0.0,
        power: MayorPower::IndustrialDouble,
        weakness: MayorWeakness::HappinessDecayFast,
    },
    MayorPersonality {
        name: "The Philosopher",
        emoji: "🧘",
        growth_aggression: 0.2,
        green_affinity: 0.7,
        industrial_bias: 0.2,
        risk_tolerance: 0.4,
        panic_threshold: 0.6,
        power: MayorPower::HappinessBoost,
        weakness: MayorWeakness::SlowGrowth,
    },
];

impl MayorPersonality {
    /// Pick a random archetype.
    pub fn random(rng: &mut SmallRng) -> &'static MayorPersonality {
        let idx = rng.gen_range(0..ARCHETYPES.len());
        &ARCHETYPES[idx]
    }

    /// Pick a specific archetype by index.
    pub fn by_index(idx: usize) -> &'static MayorPersonality {
        &ARCHETYPES[idx % ARCHETYPES.len()]
    }

    /// Apply cost modifier based on power/weakness.
    pub fn modify_cost(&self, base_cost: i64, is_road: bool, is_park: bool, is_monument: bool) -> i64 {
        let mut cost = base_cost as f64;

        // Power: CheapRoads
        if is_road && self.power == MayorPower::CheapRoads {
            cost *= 0.5;
        }

        // Power: AllCostDiscount
        if self.power == MayorPower::AllCostDiscount {
            cost *= 0.9;
        }

        // Power: CheapMonument
        if is_monument && self.power == MayorPower::CheapMonument {
            cost *= 0.5;
        }

        // Weakness: ParkCostTriple
        if is_park && self.weakness == MayorWeakness::ParkCostTriple {
            cost *= 3.0;
        }

        cost as i64
    }

    /// Tax modifier for industrial income.
    pub fn industrial_tax_modifier(&self) -> f32 {
        if self.power == MayorPower::IndustrialTaxDouble || self.power == MayorPower::IndustrialDouble {
            2.0
        } else if self.weakness == MayorWeakness::IndustrialTaxHalved {
            0.5
        } else {
            1.0
        }
    }

    /// Growth rate modifier (affects how often mayor places new zones).
    pub fn growth_rate_modifier(&self) -> f32 {
        if self.weakness == MayorWeakness::SlowExpansion {
            0.5
        } else if self.weakness == MayorWeakness::SlowGrowth {
            0.7
        } else {
            1.0
        }
    }
}

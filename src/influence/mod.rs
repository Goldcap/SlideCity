pub mod audience;
pub mod council;
pub mod suggestion;

use serde::{Deserialize, Serialize};

use crate::mayor::personality::MayorPersonality;

/// Action categories the player can suggest or vote on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionCategory {
    BuildPark,
    ZoneResidential,
    ZoneCommercial,
    ZoneIndustrial,
    ExtendPower,
    ExtendWater,
    BuildRoads,
}

impl ActionCategory {
    pub const ALL: &[ActionCategory] = &[
        ActionCategory::BuildPark,
        ActionCategory::ZoneResidential,
        ActionCategory::ZoneCommercial,
        ActionCategory::ZoneIndustrial,
        ActionCategory::ExtendPower,
        ActionCategory::ExtendWater,
        ActionCategory::BuildRoads,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ActionCategory::BuildPark => "Build a Park",
            ActionCategory::ZoneResidential => "Zone Residential",
            ActionCategory::ZoneCommercial => "Zone Commercial",
            ActionCategory::ZoneIndustrial => "Zone Industrial",
            ActionCategory::ExtendPower => "Extend Power Grid",
            ActionCategory::ExtendWater => "Extend Water Supply",
            ActionCategory::BuildRoads => "Build Roads",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            ActionCategory::BuildPark => "🌳",
            ActionCategory::ZoneResidential => "🏠",
            ActionCategory::ZoneCommercial => "🏬",
            ActionCategory::ZoneIndustrial => "🏭",
            ActionCategory::ExtendPower => "⚡",
            ActionCategory::ExtendWater => "💧",
            ActionCategory::BuildRoads => "🛤️",
        }
    }

    /// How well this action aligns with a mayor personality (0.0 - 1.0).
    pub fn alignment(self, p: &MayorPersonality) -> f32 {
        match self {
            ActionCategory::BuildPark => p.green_affinity,
            ActionCategory::ZoneResidential => p.growth_aggression,
            ActionCategory::ZoneCommercial => 0.5,
            ActionCategory::ZoneIndustrial => p.industrial_bias,
            ActionCategory::ExtendPower => 0.6, // Generally useful
            ActionCategory::ExtendWater => 0.6,
            ActionCategory::BuildRoads => p.growth_aggression * 0.8 + 0.2,
        }
    }
}

/// Mayor's response to a player influence attempt.
#[derive(Clone, Debug)]
pub enum MayorResponse {
    /// Mayor complies with the request.
    Comply(ActionCategory, String),
    /// Mayor ignores the request.
    Ignore(String),
    /// Mayor argues against the request.
    Argue(String),
    /// Mayor was overridden (council vote).
    Override(ActionCategory, String),
}

impl MayorResponse {
    pub fn text(&self) -> &str {
        match self {
            MayorResponse::Comply(_, t) => t,
            MayorResponse::Ignore(t) => t,
            MayorResponse::Argue(t) => t,
            MayorResponse::Override(_, t) => t,
        }
    }

    pub fn did_comply(&self) -> bool {
        matches!(self, MayorResponse::Comply(..))
    }

    pub fn action(&self) -> Option<ActionCategory> {
        match self {
            MayorResponse::Comply(a, _) => Some(*a),
            MayorResponse::Override(a, _) => Some(*a),
            _ => None,
        }
    }
}

/// Influence Points economy — core state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfluenceState {
    pub ip: u32,
    pub disaster_cooldown: f32,
    pub disasters_this_year: u32,
    pub last_milestone_pop: u32,
    pub last_year_earned: u32,
    pub last_phase_earned: Option<String>,
    /// Compliance boost: mayor is more likely to agree for the next N decisions.
    pub compliance_boost: u32,
}

impl InfluenceState {
    pub fn new() -> Self {
        Self {
            ip: 0,
            disaster_cooldown: 0.0,
            disasters_this_year: 0,
            last_milestone_pop: 0,
            last_year_earned: 0,
            last_phase_earned: None,
            compliance_boost: 0,
        }
    }

    /// Award IP for yearly passive income.
    pub fn yearly_tick(&mut self, year: u32) {
        if year > self.last_year_earned {
            self.ip += 1;
            self.last_year_earned = year;
            self.disasters_this_year = 0;
        }
    }

    /// Award IP for population milestones.
    pub fn check_milestones(&mut self, population: u32) {
        let milestones = [50, 100, 200, 350, 500];
        for &m in &milestones {
            if population >= m && self.last_milestone_pop < m {
                self.ip += 1;
                self.last_milestone_pop = m;
            }
        }
    }

    /// Award IP for triggering a disaster (max 2 per year).
    pub fn disaster_triggered(&mut self) -> bool {
        if self.disasters_this_year < 2 {
            self.ip += 2;
            self.disasters_this_year += 1;
            true
        } else {
            false
        }
    }

    /// Award IP for surviving a disaster (fire burned out without >50% city loss).
    pub fn disaster_survived(&mut self) {
        self.ip += 1;
    }

    /// Award IP for mayor phase transition.
    pub fn phase_transition(&mut self, phase_name: &str) {
        let phase_str = phase_name.to_string();
        if self.last_phase_earned.as_ref() != Some(&phase_str) {
            self.ip += 1;
            self.last_phase_earned = Some(phase_str);
        }
    }

    /// Buy IP with city funds. Returns true if purchase succeeded.
    pub fn buy_ip(&mut self, funds: &mut i64) -> bool {
        if *funds >= 5000 {
            *funds -= 5000;
            self.ip += 1;
            true
        } else {
            false
        }
    }

    /// Spend IP. Returns true if the player had enough.
    pub fn spend(&mut self, cost: u32) -> bool {
        if self.ip >= cost {
            self.ip -= cost;
            true
        } else {
            false
        }
    }

    /// Set compliance boost from a successful audience.
    pub fn set_compliance_boost(&mut self, decisions: u32) {
        self.compliance_boost = decisions;
    }

    /// Consume one compliance boost tick. Returns true if boosted.
    pub fn consume_boost(&mut self) -> bool {
        if self.compliance_boost > 0 {
            self.compliance_boost -= 1;
            true
        } else {
            false
        }
    }

    /// Update cooldown timer.
    pub fn update(&mut self, dt: f32) {
        if self.disaster_cooldown > 0.0 {
            self.disaster_cooldown -= dt;
            if self.disaster_cooldown < 0.0 {
                self.disaster_cooldown = 0.0;
            }
        }
    }

    /// Can afford a tier?
    pub fn can_afford(&self, cost: u32) -> bool {
        self.ip >= cost
    }
}

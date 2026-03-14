pub mod influence_ui;
pub mod mayor_log;
pub mod minimap;
pub mod start_screen;
pub mod stats;

use macroquad::prelude::*;

use crate::config::Difficulty;

/// Top-level game state machine.
#[derive(Clone, Debug)]
pub enum GameState {
    StartScreen(StartPhase),
    Playing,
}

/// Start screen sub-states.
#[derive(Clone, Debug)]
pub enum StartPhase {
    Title,
    MayorSelect,
    DifficultySelect,
}

/// Setup chosen by the player on the start screen.
#[derive(Clone, Debug)]
pub struct GameSetup {
    pub mayor_idx: usize,
    pub difficulty: Difficulty,
    pub speed_idx: usize,
    pub seed: u64,
}

impl Default for GameSetup {
    fn default() -> Self {
        Self {
            mayor_idx: 0,
            difficulty: Difficulty::Normal,
            speed_idx: 0,
            seed: 42,
        }
    }
}

/// Influence Points state (owned by main, drawn by influence_ui).
#[derive(Clone, Debug)]
pub struct InfluenceState {
    pub ip: u32,
    pub disaster_cooldown: f32,
    pub disasters_this_year: u32,
    pub last_milestone_pop: u32,
    pub last_year_earned: u32,
}

impl InfluenceState {
    pub fn new() -> Self {
        Self {
            ip: 0,
            disaster_cooldown: 0.0,
            disasters_this_year: 0,
            last_milestone_pop: 0,
            last_year_earned: 0,
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

    /// Update cooldown timer.
    pub fn update(&mut self, dt: f32) {
        if self.disaster_cooldown > 0.0 {
            self.disaster_cooldown -= dt;
            if self.disaster_cooldown < 0.0 {
                self.disaster_cooldown = 0.0;
            }
        }
    }
}

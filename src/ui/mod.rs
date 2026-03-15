pub mod influence_ui;
pub mod mayor_log;
pub mod minimap;
pub mod start_screen;
pub mod stats;

use crate::config::Difficulty;
use crate::influence::ActionCategory;

/// Top-level game state machine.
#[derive(Clone, Debug)]
pub enum GameState {
    StartScreen(StartPhase),
    Playing,
    Paused,
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

/// Active influence modal overlay.
#[derive(Clone, Debug)]
pub enum InfluenceModal {
    None,
    SuggestionBox,
    CouncilVote {
        candidates: [ActionCategory; 3],
    },
    Audience {
        input: String,
        response: Option<String>,
        waiting: bool,
    },
    BuyIP,
}

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::SimConfig;
use crate::grid::Grid;
use crate::influence::InfluenceState;
use crate::mayor::Mayor;

/// Serializable game save state.
#[derive(Serialize, Deserialize)]
pub struct SaveState {
    pub grid: Grid,
    pub mayor: Mayor,
    pub influence: InfluenceState,
    pub config: SimConfig,
    pub funds: i64,
    pub tick_count: u64,
    pub monument_sting_played: bool,
    pub speed_idx: usize,
    pub save_version: u32,
}

const SAVE_VERSION: u32 = 1;

impl SaveState {
    pub fn new(
        grid: &Grid,
        mayor: &Mayor,
        influence: &InfluenceState,
        config: &SimConfig,
        funds: i64,
        tick_count: u64,
        monument_sting_played: bool,
        speed_idx: usize,
    ) -> Self {
        Self {
            grid: grid.clone(),
            mayor: mayor.clone(),
            influence: influence.clone(),
            config: config.clone(),
            funds,
            tick_count,
            monument_sting_played,
            speed_idx,
            save_version: SAVE_VERSION,
        }
    }
}

/// Get the save file path.
fn save_path() -> PathBuf {
    // Use platform-appropriate save directory
    if let Some(dir) = dirs_fallback() {
        let save_dir = dir.join("SlideCity");
        let _ = std::fs::create_dir_all(&save_dir);
        save_dir.join("save.json")
    } else {
        PathBuf::from("slidecity_save.json")
    }
}

fn dirs_fallback() -> Option<PathBuf> {
    // Try XDG_DATA_HOME, then ~/.local/share, then current dir
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return Some(PathBuf::from(xdg));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join(".local").join("share"));
    }
    None
}

/// Save the game state to disk.
pub fn save_game(state: &SaveState) -> Result<(), String> {
    let path = save_path();
    let json = serde_json::to_string(state).map_err(|e| format!("Serialize error: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write error: {}: {}", path.display(), e))?;
    Ok(())
}

/// Load a saved game from disk.
pub fn load_game() -> Result<SaveState, String> {
    let path = save_path();
    if !path.exists() {
        return Err("No save file found".to_string());
    }
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Read error: {}: {}", path.display(), e))?;
    let state: SaveState =
        serde_json::from_str(&json).map_err(|e| format!("Parse error: {}", e))?;
    if state.save_version != SAVE_VERSION {
        return Err(format!(
            "Save version mismatch: got {}, expected {}",
            state.save_version, SAVE_VERSION
        ));
    }
    Ok(state)
}

/// Check if a save file exists.
pub fn save_exists() -> bool {
    save_path().exists()
}

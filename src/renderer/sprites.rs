use std::collections::HashMap;
use macroquad::prelude::*;

use crate::grid::{Cell, TileType, TerrainType};

/// Sprite key for looking up textures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SpriteKey {
    // Terrain
    Grass,
    GrassFlower,
    TreesDense,
    TreesSparse,
    Sand,
    Dirt,
    Rock,
    Snow,
    Water,
    // Roads
    RoadStraight,
    RoadCross,
    // Residential (stage 1-3, variant 1-4)
    Residential(u8, u8),
    // Commercial (stage 1-3, variant 1-4)
    Commercial(u8, u8),
    // Industrial (stage 1-3, variant 1-3)
    Industrial(u8, u8),
    // Infrastructure
    PowerPlant,
    PowerLine,
    WaterTower,
    WaterMain,
    Monument,
    Park,
    // Events
    Fire,
    Rubble,
}

/// Loaded sprite atlas — maps keys to macroquad Texture2D handles.
pub struct SpriteAtlas {
    textures: HashMap<SpriteKey, Texture2D>,
}

impl SpriteAtlas {
    /// Load all sprites from assets/sprites/. Missing files are silently skipped.
    pub async fn load() -> Self {
        let mut textures = HashMap::new();

        // Terrain
        try_load(&mut textures, SpriteKey::Grass, "assets/sprites/terrain/grass.png").await;
        try_load(&mut textures, SpriteKey::GrassFlower, "assets/sprites/terrain/grass_flower.png").await;
        try_load(&mut textures, SpriteKey::TreesDense, "assets/sprites/terrain/trees_dense.png").await;
        try_load(&mut textures, SpriteKey::TreesSparse, "assets/sprites/terrain/trees_sparse.png").await;
        try_load(&mut textures, SpriteKey::Sand, "assets/sprites/terrain/sand.png").await;
        try_load(&mut textures, SpriteKey::Dirt, "assets/sprites/terrain/dirt.png").await;
        try_load(&mut textures, SpriteKey::Rock, "assets/sprites/terrain/rock.png").await;
        try_load(&mut textures, SpriteKey::Snow, "assets/sprites/terrain/snow.png").await;
        try_load(&mut textures, SpriteKey::Water, "assets/sprites/terrain/water.png").await;

        // Roads
        try_load(&mut textures, SpriteKey::RoadStraight, "assets/sprites/roads/road_straight.png").await;
        try_load(&mut textures, SpriteKey::RoadCross, "assets/sprites/roads/road_cross.png").await;

        // Residential: 3 stages × 4 variants
        for stage in 1..=3u8 {
            for variant in 1..=4u8 {
                let path = format!("assets/sprites/residential/res_s{}_v{}.png", stage, variant);
                try_load(&mut textures, SpriteKey::Residential(stage, variant), &path).await;
            }
        }

        // Commercial: 3 stages × 4 variants
        for stage in 1..=3u8 {
            for variant in 1..=4u8 {
                let path = format!("assets/sprites/commercial/com_s{}_v{}.png", stage, variant);
                try_load(&mut textures, SpriteKey::Commercial(stage, variant), &path).await;
            }
        }

        // Industrial: 3 stages × 3 variants
        for stage in 1..=3u8 {
            for variant in 1..=3u8 {
                let path = format!("assets/sprites/industrial/ind_s{}_v{}.png", stage, variant);
                try_load(&mut textures, SpriteKey::Industrial(stage, variant), &path).await;
            }
        }

        // Infrastructure
        try_load(&mut textures, SpriteKey::PowerPlant, "assets/sprites/infrastructure/power_plant.png").await;
        try_load(&mut textures, SpriteKey::PowerLine, "assets/sprites/infrastructure/power_line.png").await;
        try_load(&mut textures, SpriteKey::WaterTower, "assets/sprites/infrastructure/water_tower.png").await;
        try_load(&mut textures, SpriteKey::WaterMain, "assets/sprites/infrastructure/water_main.png").await;
        try_load(&mut textures, SpriteKey::Monument, "assets/sprites/infrastructure/monument.png").await;
        try_load(&mut textures, SpriteKey::Park, "assets/sprites/infrastructure/park.png").await;

        // Events
        try_load(&mut textures, SpriteKey::Fire, "assets/sprites/events/fire.png").await;
        try_load(&mut textures, SpriteKey::Rubble, "assets/sprites/events/rubble.png").await;

        let count = textures.len();
        if count > 0 {
            eprintln!("[sprites] Loaded {} textures", count);
        } else {
            eprintln!("[sprites] No sprite files found — using colored rectangles");
        }

        Self { textures }
    }

    /// Get the texture for a cell, if available.
    pub fn get_for_cell(&self, cell: &Cell) -> Option<&Texture2D> {
        let key = cell_to_key(cell)?;
        self.textures.get(&key)
    }

    /// Whether any sprites were loaded.
    pub fn has_sprites(&self) -> bool {
        !self.textures.is_empty()
    }
}

/// Map a cell to its sprite key.
fn cell_to_key(cell: &Cell) -> Option<SpriteKey> {
    match cell.tile {
        TileType::Empty => {
            Some(match cell.terrain_type {
                TerrainType::Grass => SpriteKey::Grass,
                TerrainType::GrassFlower => SpriteKey::GrassFlower,
                TerrainType::Trees => SpriteKey::TreesDense,
                TerrainType::TreesSparse => SpriteKey::TreesSparse,
                TerrainType::Sand => SpriteKey::Sand,
                TerrainType::Dirt => SpriteKey::Dirt,
                TerrainType::Rock => SpriteKey::Rock,
                TerrainType::Snow => SpriteKey::Snow,
            })
        }
        TileType::WaterBody => Some(SpriteKey::Water),
        TileType::Road => {
            // TODO: detect straight vs cross based on neighbors
            Some(SpriteKey::RoadStraight)
        }
        TileType::Residential => {
            let (stage, variant) = building_stage_variant(cell);
            Some(SpriteKey::Residential(stage, variant))
        }
        TileType::Commercial => {
            let (stage, variant) = building_stage_variant(cell);
            Some(SpriteKey::Commercial(stage, variant))
        }
        TileType::Industrial => {
            let (stage, variant) = industrial_stage_variant(cell);
            Some(SpriteKey::Industrial(stage, variant))
        }
        TileType::PowerPlant => Some(SpriteKey::PowerPlant),
        TileType::PowerLine => Some(SpriteKey::PowerLine),
        TileType::WaterTower => Some(SpriteKey::WaterTower),
        TileType::WaterMain => Some(SpriteKey::WaterMain),
        TileType::Monument => Some(SpriteKey::Monument),
        TileType::Park => Some(SpriteKey::Park),
        TileType::Fire => Some(SpriteKey::Fire),
        TileType::Rubble => Some(SpriteKey::Rubble),
    }
}

/// Building stage (1-3) and variant (1-4) from cell age and style.
fn building_stage_variant(cell: &Cell) -> (u8, u8) {
    let stage = if cell.age < 16 { 1 } else if cell.age < 46 { 2 } else { 3 };
    let variant = (cell.style % 4) + 1;
    (stage, variant)
}

/// Industrial stage (1-3) and variant (1-3) from cell age and style.
fn industrial_stage_variant(cell: &Cell) -> (u8, u8) {
    let stage = if cell.age < 16 { 1 } else if cell.age < 46 { 2 } else { 3 };
    let variant = (cell.style % 3) + 1;
    (stage, variant)
}

/// Try to load a texture, silently skip if file doesn't exist.
async fn try_load(map: &mut HashMap<SpriteKey, Texture2D>, key: SpriteKey, path: &str) {
    match load_texture(path).await {
        Ok(tex) => {
            // Use nearest-neighbor filtering for pixel art crispness
            tex.set_filter(FilterMode::Nearest);
            map.insert(key, tex);
        }
        Err(_) => {
            // File doesn't exist — silently fall back to colored rects
        }
    }
}

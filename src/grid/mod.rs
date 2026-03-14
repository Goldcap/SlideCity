pub mod neighbors;
pub mod terrain;

use serde::{Deserialize, Serialize};

/// Tile types for each cell in the grid.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TileType {
    Empty,
    Road,
    Residential,
    Commercial,
    Industrial,
    Park,
    PowerPlant,
    PowerLine,
    WaterTower,
    WaterMain,
    Monument,
    Fire,
    Rubble,
    WaterBody,
}

impl TileType {
    /// Single-character label for fallback rendering.
    pub fn label(self) -> &'static str {
        match self {
            TileType::Empty => "",
            TileType::Road => "R",
            TileType::Residential => "H",
            TileType::Commercial => "C",
            TileType::Industrial => "I",
            TileType::Park => "P",
            TileType::PowerPlant => "E",
            TileType::PowerLine => "e",
            TileType::WaterTower => "W",
            TileType::WaterMain => "w",
            TileType::Monument => "M",
            TileType::Fire => "F",
            TileType::Rubble => "X",
            TileType::WaterBody => "~",
        }
    }

    /// Fallback color for colored-rect rendering (R, G, B).
    pub fn color(self) -> (f32, f32, f32) {
        match self {
            TileType::Empty => (0.3, 0.5, 0.2),        // grass green
            TileType::Road => (0.35, 0.35, 0.35),       // asphalt grey
            TileType::Residential => (0.2, 0.6, 0.2),   // green
            TileType::Commercial => (0.2, 0.3, 0.8),    // blue
            TileType::Industrial => (0.7, 0.6, 0.2),    // yellow-brown
            TileType::Park => (0.1, 0.7, 0.3),          // bright green
            TileType::PowerPlant => (0.8, 0.5, 0.1),    // orange
            TileType::PowerLine => (0.7, 0.5, 0.1),     // light orange
            TileType::WaterTower => (0.1, 0.4, 0.9),    // bright blue
            TileType::WaterMain => (0.2, 0.4, 0.7),     // medium blue
            TileType::Monument => (0.7, 0.3, 0.8),      // purple
            TileType::Fire => (0.9, 0.2, 0.0),          // red-orange
            TileType::Rubble => (0.4, 0.35, 0.3),       // dark brown
            TileType::WaterBody => (0.1, 0.3, 0.7),     // deep blue
        }
    }

    /// Building height in floors, used for isometric rendering.
    /// Stage is determined by cell age: stage1=0-15, stage2=16-45, stage3=46+.
    pub fn height_floors(self, age: u8) -> f32 {
        let stage = if age < 16 {
            1
        } else if age < 46 {
            2
        } else {
            3
        };
        match self {
            TileType::Empty | TileType::Road | TileType::Park | TileType::WaterBody => 0.0,
            TileType::Residential => match stage {
                1 => 1.0,
                2 => 2.0,
                _ => 4.0,
            },
            TileType::Commercial => match stage {
                1 => 1.0,
                2 => 4.0,
                _ => 10.0,
            },
            TileType::Industrial => match stage {
                1 => 2.0,
                2 => 3.0,
                _ => 5.0,
            },
            TileType::PowerPlant => 4.0,
            TileType::PowerLine => 0.0,
            TileType::WaterTower => 6.0,
            TileType::WaterMain => 0.0,
            TileType::Monument => 12.0,
            TileType::Fire => 0.0,
            TileType::Rubble => 0.0,
        }
    }

    /// Population per cell based on stage.
    pub fn population(self, age: u8) -> u32 {
        if self != TileType::Residential {
            return 0;
        }
        if age < 16 {
            2
        } else if age < 46 {
            6
        } else {
            12
        }
    }
}

/// A single cell in the grid.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub tile: TileType,
    pub age: u8,
    pub style: u8,
    pub has_power: bool,
    pub has_water: bool,
    pub terrain_height: f32,
}

impl Cell {
    pub fn empty(terrain_height: f32) -> Self {
        Self {
            tile: TileType::Empty,
            age: 0,
            style: 0,
            has_power: false,
            has_water: false,
            terrain_height,
        }
    }

    pub fn water(terrain_height: f32) -> Self {
        Self {
            tile: TileType::WaterBody,
            age: 0,
            style: 0,
            has_power: false,
            has_water: false,
            terrain_height,
        }
    }
}

/// The game grid: flat Vec storage with row * width + col indexing.
#[derive(Clone, Serialize, Deserialize)]
pub struct Grid {
    pub cells: Vec<Cell>,
    pub width: usize,
    pub height: usize,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![Cell::empty(0.0); width * height],
            width,
            height,
        }
    }

    #[inline]
    pub fn idx(&self, col: usize, row: usize) -> usize {
        row * self.width + col
    }

    #[inline]
    pub fn in_bounds(&self, col: usize, row: usize) -> bool {
        col < self.width && row < self.height
    }

    #[inline]
    pub fn get(&self, col: usize, row: usize) -> &Cell {
        &self.cells[self.idx(col, row)]
    }

    #[inline]
    pub fn get_mut(&mut self, col: usize, row: usize) -> &mut Cell {
        let idx = self.idx(col, row);
        &mut self.cells[idx]
    }

    /// Count total cells of a given tile type.
    pub fn count_type(&self, tile: TileType) -> usize {
        self.cells.iter().filter(|c| c.tile == tile).count()
    }

    /// Calculate total population.
    pub fn population(&self) -> u32 {
        self.cells.iter().map(|c| c.tile.population(c.age)).sum()
    }
}

use crate::grid::{Grid, TileType};

/// Aggregated city statistics, recomputed each tick.
#[derive(Clone, Debug, Default)]
pub struct CityStats {
    pub population: u32,
    pub res_count: u32,
    pub com_count: u32,
    pub ind_count: u32,
    pub fire_count: u32,
    pub park_count: u32,
    pub road_count: u32,
    pub rubble_count: u32,
    pub total_developed: u32,
    pub powered_zones: u32,
    pub total_zones: u32,
    pub watered_zones: u32,
    pub power_coverage: f32,
    pub water_coverage: f32,
    pub happiness: f32,
}

impl CityStats {
    /// Compute all stats from the current grid state.
    pub fn compute(grid: &Grid) -> Self {
        let mut stats = Self::default();

        let mut ind_adjacent_to_res = 0u32;

        for row in 0..grid.height {
            for col in 0..grid.width {
                let cell = grid.get(col, row);

                match cell.tile {
                    TileType::Residential => {
                        stats.res_count += 1;
                        stats.population += cell.tile.population(cell.age);
                        stats.total_zones += 1;
                        if cell.has_power {
                            stats.powered_zones += 1;
                        }
                        if cell.has_water {
                            stats.watered_zones += 1;
                        }

                        // Check for adjacent industrial (pollution)
                        let ind_nearby = grid.count_neighbors(col, row, 2, TileType::Industrial);
                        if ind_nearby > 0 {
                            ind_adjacent_to_res += 1;
                        }
                    }
                    TileType::Commercial => {
                        stats.com_count += 1;
                        stats.total_zones += 1;
                        if cell.has_power {
                            stats.powered_zones += 1;
                        }
                        if cell.has_water {
                            stats.watered_zones += 1;
                        }
                    }
                    TileType::Industrial => {
                        stats.ind_count += 1;
                        stats.total_zones += 1;
                        if cell.has_power {
                            stats.powered_zones += 1;
                        }
                        if cell.has_water {
                            stats.watered_zones += 1;
                        }
                    }
                    TileType::Fire => stats.fire_count += 1,
                    TileType::Park => stats.park_count += 1,
                    TileType::Road => stats.road_count += 1,
                    TileType::Rubble => stats.rubble_count += 1,
                    _ => {}
                }

                if cell.tile != TileType::Empty && cell.tile != TileType::WaterBody {
                    stats.total_developed += 1;
                }
            }
        }

        // Utility coverage
        if stats.total_zones > 0 {
            stats.power_coverage = stats.powered_zones as f32 / stats.total_zones as f32;
            stats.water_coverage = stats.watered_zones as f32 / stats.total_zones as f32;
        }

        // Happiness formula:
        // 0.25 * park_ratio + 0.25 * power_coverage + 0.20 * water_coverage
        // + 0.20 * (1.0 - pollution) + 0.10 * commercial_ratio
        let park_ratio = if stats.total_developed > 0 {
            (stats.park_count as f32 / stats.total_developed as f32).min(1.0)
        } else {
            0.0
        };

        let pollution = if stats.res_count > 0 {
            (ind_adjacent_to_res as f32 / stats.res_count as f32).min(1.0)
        } else {
            0.0
        };

        let com_ratio = if stats.res_count > 0 {
            // Ideal ratio is ~0.3 commercial per residential
            let raw = stats.com_count as f32 / stats.res_count as f32;
            (raw / 0.3).min(1.0)
        } else {
            0.0
        };

        stats.happiness = 0.25 * park_ratio
            + 0.25 * stats.power_coverage
            + 0.20 * stats.water_coverage
            + 0.20 * (1.0 - pollution)
            + 0.10 * com_ratio;

        // Clamp to 0-1
        stats.happiness = stats.happiness.clamp(0.0, 1.0);

        stats
    }
}

use ::rand::rngs::SmallRng;
use ::rand::Rng;

use crate::config::SimConfig;
use crate::grid::{Cell, Grid, TileType};

/// Apply all cellular automaton rules. Reads from `grid`, writes to `next`.
pub fn apply_all_rules(grid: &Grid, next: &mut Grid, config: &SimConfig, rng: &mut SmallRng) {
    for row in 0..grid.height {
        for col in 0..grid.width {
            let cell = grid.get(col, row);
            let new_tile = match cell.tile {
                TileType::Empty => rule_empty(grid, col, row, config, rng),
                TileType::Residential => rule_residential(grid, col, row, cell, config, rng),
                TileType::Commercial => rule_commercial(grid, col, row, cell, config, rng),
                TileType::Industrial => rule_industrial(grid, col, row, cell, config, rng),
                TileType::Fire => rule_fire(grid, col, row, cell, next, config, rng),
                TileType::Rubble => rule_rubble(cell, config, rng),
                TileType::Park => rule_park(grid, col, row, config, rng),
                // Road, PowerPlant, PowerLine, WaterTower, WaterMain, Monument, WaterBody: no rules
                _ => None,
            };

            if let Some(new_type) = new_tile {
                let next_cell = next.get_mut(col, row);
                next_cell.tile = new_type;
                next_cell.age = 0;
                if new_type == TileType::Fire {
                    next_cell.style = 0;
                } else {
                    next_cell.style = rng.gen_range(0..4);
                }
            }
        }
    }

    // Random fire ignition
    for row in 0..grid.height {
        for col in 0..grid.width {
            let cell = grid.get(col, row);
            let is_flammable = matches!(
                cell.tile,
                TileType::Residential | TileType::Commercial | TileType::Industrial
            );
            if is_flammable && rng.gen::<f32>() < config.random_fire_chance {
                let next_cell = next.get_mut(col, row);
                next_cell.tile = TileType::Fire;
                next_cell.age = 0;
            }
        }
    }
}

/// Empty cell: can seed zones if near roads and other development.
fn rule_empty(grid: &Grid, col: usize, row: usize, _config: &SimConfig, rng: &mut SmallRng) -> Option<TileType> {
    if !grid.has_road_neighbor(col, row) {
        return None;
    }

    let res_n = grid.count_neighbors(col, row, 2, TileType::Residential);
    let com_n = grid.count_neighbors(col, row, 2, TileType::Commercial);
    let ind_n = grid.count_neighbors(col, row, 2, TileType::Industrial);
    let density = grid.count_developed(col, row, 3);

    // Residential seeding
    if res_n >= 3 && rng.gen::<f32>() < 0.05 {
        return Some(TileType::Residential);
    }

    // Commercial seeding (needs nearby residential)
    let res_nearby = grid.count_neighbors(col, row, 5, TileType::Residential);
    if com_n >= 2 && res_nearby >= 8 && rng.gen::<f32>() < 0.03 {
        return Some(TileType::Commercial);
    }

    // Industrial seeding
    if ind_n >= 3 && rng.gen::<f32>() < 0.025 {
        return Some(TileType::Industrial);
    }

    // Park from high density
    if density >= 8 && rng.gen::<f32>() < 0.02 {
        return Some(TileType::Park);
    }

    // Road extension (aligned, low probability)
    let road_count = grid.count_neighbors(col, row, 1, TileType::Road);
    if road_count == 2 && grid.roads_aligned(col, row) && rng.gen::<f32>() < 0.008 {
        return Some(TileType::Road);
    }

    None
}

/// Residential: can decay from pollution, abandonment, or upzone to commercial.
fn rule_residential(
    grid: &Grid, col: usize, row: usize, cell: &Cell,
    config: &SimConfig, rng: &mut SmallRng,
) -> Option<TileType> {
    let ind_nearby = grid.count_neighbors(col, row, 3, TileType::Industrial);
    let park_nearby = grid.count_neighbors(col, row, 3, TileType::Park);
    let com_nearby = grid.count_neighbors(col, row, 3, TileType::Commercial);
    let decay = config.decay_multiplier;

    // Pollution decay (very rare — need heavy industrial AND no parks)
    if ind_nearby >= 5 && park_nearby == 0 && rng.gen::<f32>() < 0.004 * decay {
        return Some(TileType::Rubble);
    }

    // Power abandonment (slow — give mayor time to build power)
    if !cell.has_power && cell.age > 80 && rng.gen::<f32>() < 0.002 * decay {
        return Some(TileType::Rubble);
    }

    // Water abandonment (very slow)
    if !cell.has_water && cell.age > 100 && rng.gen::<f32>() < 0.001 * decay {
        return Some(TileType::Rubble);
    }

    // Upzone to commercial
    if cell.age > 50 && com_nearby >= 3 && cell.has_power && cell.has_water && rng.gen::<f32>() < 0.008 {
        return Some(TileType::Commercial);
    }

    None
}

/// Commercial: decays without customers or power.
fn rule_commercial(
    grid: &Grid, col: usize, row: usize, cell: &Cell,
    config: &SimConfig, rng: &mut SmallRng,
) -> Option<TileType> {
    let res_nearby = grid.count_neighbors(col, row, 4, TileType::Residential);
    let decay = config.decay_multiplier;

    // No customers (slower — give time for residential to grow nearby)
    if res_nearby < 2 && cell.age > 50 && rng.gen::<f32>() < 0.004 * decay {
        return Some(TileType::Rubble);
    }

    // No power
    if !cell.has_power && cell.age > 60 && rng.gen::<f32>() < 0.003 * decay {
        return Some(TileType::Rubble);
    }

    None
}

/// Industrial: can gentrify to commercial or decay without power.
fn rule_industrial(
    grid: &Grid, col: usize, row: usize, cell: &Cell,
    config: &SimConfig, rng: &mut SmallRng,
) -> Option<TileType> {
    let res_nearby = grid.count_neighbors(col, row, 4, TileType::Residential);
    let com_nearby = grid.count_neighbors(col, row, 3, TileType::Commercial);
    let decay = config.decay_multiplier;

    // Gentrification
    if res_nearby >= 6 && com_nearby >= 3 && cell.age > 35 && rng.gen::<f32>() < 0.008 {
        return Some(TileType::Commercial);
    }

    // No power (industrial is more tolerant)
    if !cell.has_power && cell.age > 60 && rng.gen::<f32>() < 0.003 * decay {
        return Some(TileType::Rubble);
    }

    None
}

/// Fire: spreads to adjacent flammable cells, burns out to rubble.
fn rule_fire(
    grid: &Grid, col: usize, row: usize, cell: &Cell,
    next: &mut Grid, config: &SimConfig, rng: &mut SmallRng,
) -> Option<TileType> {
    // Spread to neighbors
    let neighbors: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    for (dc, dr) in &neighbors {
        let nc = col as i32 + dc;
        let nr = row as i32 + dr;
        if nc >= 0 && nc < grid.width as i32 && nr >= 0 && nr < grid.height as i32 {
            let nc = nc as usize;
            let nr = nr as usize;
            let neighbor = grid.get(nc, nr);
            let is_flammable = matches!(
                neighbor.tile,
                TileType::Residential | TileType::Commercial | TileType::Industrial | TileType::Park
            );
            if is_flammable && rng.gen::<f32>() < config.fire_spread_prob {
                let n = next.get_mut(nc, nr);
                n.tile = TileType::Fire;
                n.age = 0;
            }
        }
    }

    // Burn out
    if cell.age > 10 {
        return Some(TileType::Rubble);
    }

    None
}

/// Rubble: eventually clears to empty (slow — abandoned buildings linger).
fn rule_rubble(cell: &Cell, config: &SimConfig, rng: &mut SmallRng) -> Option<TileType> {
    let decay = config.decay_multiplier;
    if cell.age > 60 && rng.gen::<f32>() < 0.015 / decay {
        return Some(TileType::Empty);
    }
    None
}

/// Park: can be encroached by nearby industrial.
fn rule_park(grid: &Grid, col: usize, row: usize, config: &SimConfig, rng: &mut SmallRng) -> Option<TileType> {
    let ind_nearby = grid.count_neighbors(col, row, 2, TileType::Industrial);
    let decay = config.decay_multiplier;

    if ind_nearby >= 4 && rng.gen::<f32>() < 0.015 * decay {
        return Some(TileType::Empty);
    }

    None
}

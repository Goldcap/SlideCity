use ::rand::rngs::SmallRng;
use ::rand::Rng;

use crate::config::SimConfig;
use crate::grid::{Grid, TileType};
use crate::sim::demand::RciDemand;
use crate::sim::desirability::DesirabilityGrid;

/// Apply all cellular automaton rules with demand-driven economics.
/// Reads from `grid`, writes to `next`. Uses demand and desirability for growth/decay.
pub fn apply_all_rules(
    grid: &Grid,
    next: &mut Grid,
    config: &SimConfig,
    rng: &mut SmallRng,
    demand: &RciDemand,
    desirability: &DesirabilityGrid,
) {
    for row in 0..grid.height {
        for col in 0..grid.width {
            let cell = grid.get(col, row);
            let desir = desirability.get(col, row);

            let new_tile = match cell.tile {
                // Zoned empty land → develop into buildings when demand + desirability allow
                TileType::ZonedResidential => {
                    rule_zoned_develop(TileType::Residential, demand.residential, desir, rng)
                }
                TileType::ZonedCommercial => {
                    rule_zoned_develop(TileType::Commercial, demand.commercial, desir, rng)
                }
                TileType::ZonedIndustrial => {
                    rule_zoned_develop(TileType::Industrial, demand.industrial, desir, rng)
                }

                // Existing buildings → upgrade stages or abandon based on demand
                TileType::Residential => {
                    rule_building(next, col, row, demand.residential, desir, rng)
                }
                TileType::Commercial => {
                    rule_building(next, col, row, demand.commercial, desir, rng)
                }
                TileType::Industrial => {
                    rule_building(next, col, row, demand.industrial, desir, rng)
                }

                // Abandoned buildings just sit there — mayor must bulldoze
                TileType::Abandoned => None,

                // Fire/rubble mechanics unchanged
                TileType::Fire => rule_fire(grid, col, row, cell, next, config, rng),
                TileType::Rubble => rule_rubble(cell, config, rng),
                TileType::Park => rule_park(grid, col, row, config, rng),

                // Everything else: no rules
                _ => None,
            };

            if let Some(new_type) = new_tile {
                let next_cell = next.get_mut(col, row);
                next_cell.tile = new_type;
                next_cell.age = 0;
                next_cell.abandon_timer = 0;
                if new_type != TileType::Fire {
                    next_cell.style = rng.gen_range(0..4);
                }
                if new_type == TileType::Residential || new_type == TileType::Commercial || new_type == TileType::Industrial {
                    next_cell.building_stage = 0; // New building starts at stage 0
                }
            }
        }
    }

    // Random fire ignition (unchanged)
    for row in 0..grid.height {
        for col in 0..grid.width {
            let cell = grid.get(col, row);
            let is_flammable = cell.tile.is_building();
            if is_flammable && rng.gen::<f32>() < config.random_fire_chance {
                let next_cell = next.get_mut(col, row);
                next_cell.tile = TileType::Fire;
                next_cell.age = 0;
            }
        }
    }
}

/// Zoned empty land develops into a building when demand and desirability are positive.
fn rule_zoned_develop(
    target: TileType,
    demand: f32,
    desirability: f32,
    rng: &mut SmallRng,
) -> Option<TileType> {
    if demand <= 0.0 || desirability <= 0.0 {
        return None;
    }

    // Development probability scales with demand and desirability
    let demand_factor = (demand / 50.0).clamp(0.0, 1.0);
    let desir_factor = (desirability / 30.0).clamp(0.0, 1.0);
    let prob = demand_factor * desir_factor * 0.08; // ~8% max per tick

    if rng.gen::<f32>() < prob {
        Some(target)
    } else {
        None
    }
}

/// Existing building: upgrade stage with good conditions, or start abandonment timer.
fn rule_building(
    next: &mut Grid,
    col: usize,
    row: usize,
    demand: f32,
    desirability: f32,
    rng: &mut SmallRng,
) -> Option<TileType> {
    let cell = next.get_mut(col, row);

    // Stage upgrade: demand > 10 AND desirability above stage threshold
    let stage_threshold = match cell.building_stage {
        0 => 15.0,  // stage 0 → 1 requires desirability > 15
        1 => 35.0,  // stage 1 → 2 requires desirability > 35
        _ => f32::MAX, // stage 2 is max for Phase 1
    };

    if demand > 10.0 && desirability > stage_threshold && cell.age > 30
        && rng.gen::<f32>() < 0.02
    {
        cell.building_stage = (cell.building_stage + 1).min(2);
    }

    // Abandonment: sustained negative demand AND low desirability
    // Young buildings (age < 100) are protected — gives the mayor time to
    // establish the demand loop (zone C/I for jobs) before buildings can abandon.
    if cell.age > 100 && demand < -20.0 && desirability < 0.0 {
        cell.abandon_timer = cell.abandon_timer.saturating_add(1);
        if cell.abandon_timer >= 30 {
            return Some(TileType::Abandoned);
        }
    } else {
        // Conditions improved or building is young — reset timer
        cell.abandon_timer = cell.abandon_timer.saturating_sub(1);
    }

    None
}

/// Fire: spreads to neighbors, burns out to rubble.
fn rule_fire(
    grid: &Grid, col: usize, row: usize, cell: &crate::grid::Cell,
    next: &mut Grid, config: &SimConfig, rng: &mut SmallRng,
) -> Option<TileType> {
    let neighbors: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    for (dc, dr) in &neighbors {
        let nc = col as i32 + dc;
        let nr = row as i32 + dr;
        if nc >= 0 && nc < grid.width as i32 && nr >= 0 && nr < grid.height as i32 {
            let nc = nc as usize;
            let nr = nr as usize;
            let neighbor = grid.get(nc, nr);
            let is_flammable = neighbor.tile.is_building() || neighbor.tile == TileType::Park;
            if is_flammable && rng.gen::<f32>() < config.fire_spread_prob {
                let n = next.get_mut(nc, nr);
                n.tile = TileType::Fire;
                n.age = 0;
            }
        }
    }

    if cell.age > 10 {
        return Some(TileType::Rubble);
    }

    None
}

/// Rubble: eventually clears.
fn rule_rubble(cell: &crate::grid::Cell, config: &SimConfig, rng: &mut SmallRng) -> Option<TileType> {
    let decay = config.decay_multiplier;
    if cell.age > 60 && rng.gen::<f32>() < 0.015 / decay {
        return Some(TileType::Empty);
    }
    None
}

/// Park: can be encroached by industrial.
fn rule_park(grid: &Grid, col: usize, row: usize, config: &SimConfig, rng: &mut SmallRng) -> Option<TileType> {
    let ind_nearby = grid.count_neighbors(col, row, 2, TileType::Industrial);
    let decay = config.decay_multiplier;

    if ind_nearby >= 4 && rng.gen::<f32>() < 0.015 * decay {
        return Some(TileType::Empty);
    }

    None
}

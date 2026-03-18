pub mod automaton;
pub mod demand;
pub mod desirability;
pub mod growth;
pub mod stats;
pub mod utilities;

use ::rand::rngs::SmallRng;

use crate::config::SimConfig;
use crate::grid::Grid;

/// Run one simulation tick: apply demand-driven automaton rules, age cells, collect taxes.
/// Uses double-buffering: reads from `grid`, writes to `next_grid`, then swaps.
pub fn tick(
    grid: &mut Grid,
    next_grid: &mut Grid,
    config: &SimConfig,
    rng: &mut SmallRng,
    funds: &mut i64,
    rci_demand: &demand::RciDemand,
    desirability_grid: &desirability::DesirabilityGrid,
) {
    // Copy current state as baseline
    next_grid.cells.copy_from_slice(&grid.cells);

    // Apply demand-driven automaton rules
    automaton::apply_all_rules(grid, next_grid, config, rng, rci_demand, desirability_grid);

    // Age all non-empty cells and collect taxes
    for cell in next_grid.cells.iter_mut() {
        use crate::grid::TileType;

        // Age increment (saturating at 255) — skip empty, water, and zoned-empty
        if cell.tile != TileType::Empty && cell.tile != TileType::WaterBody
            && !cell.tile.is_zoned_empty()
        {
            cell.age = cell.age.saturating_add(1);
        }

        // Tax collection
        let tax = match cell.tile {
            TileType::Residential => config.res_tax,
            TileType::Commercial => config.com_tax,
            TileType::Industrial => config.ind_tax,
            _ => 0,
        };
        *funds += (tax as f32 * config.tax_multiplier) as i64;
    }

    // Swap: next_grid becomes the active grid
    std::mem::swap(&mut grid.cells, &mut next_grid.cells);
}

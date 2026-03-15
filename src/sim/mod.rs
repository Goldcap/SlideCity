pub mod automaton;
pub mod growth;
pub mod stats;
pub mod utilities;

use ::rand::rngs::SmallRng;

use crate::config::SimConfig;
use crate::grid::Grid;

/// Run one simulation tick: apply automaton rules, age cells, collect taxes.
/// Uses double-buffering: reads from `grid`, writes to `next_grid`, then swaps.
pub fn tick(grid: &mut Grid, next_grid: &mut Grid, config: &SimConfig, rng: &mut SmallRng, funds: &mut i64) {
    // Copy current state as baseline
    next_grid.cells.copy_from_slice(&grid.cells);

    // Apply automaton rules (reads grid, writes next_grid)
    automaton::apply_all_rules(grid, next_grid, config, rng);

    // Age all non-empty cells and collect taxes
    for cell in next_grid.cells.iter_mut() {
        use crate::grid::TileType;

        // Age increment (saturating at 255)
        if cell.tile != TileType::Empty && cell.tile != TileType::WaterBody {
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

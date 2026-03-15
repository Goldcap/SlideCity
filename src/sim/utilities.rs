use std::collections::VecDeque;

use crate::grid::{Grid, TileType};

/// Recompute power and water networks via flood-fill.
/// PowerPlant: manhattan distance 12, PowerLine extends with radius 6.
/// WaterTower: manhattan distance 10, WaterMain extends with radius 5.
pub fn recompute_utilities(grid: &mut Grid) {
    // Reset all utility flags
    for cell in grid.cells.iter_mut() {
        cell.has_power = false;
        cell.has_water = false;
    }

    // Power network
    flood_fill_utility(
        grid,
        TileType::PowerPlant,
        12,
        TileType::PowerLine,
        6,
        true, // is_power
    );

    // Water network
    flood_fill_utility(
        grid,
        TileType::WaterTower,
        10,
        TileType::WaterMain,
        5,
        false, // is_water
    );
}

/// Generic utility flood-fill from source tiles, extended by line tiles.
fn flood_fill_utility(
    grid: &mut Grid,
    source_type: TileType,
    source_radius: i32,
    line_type: TileType,
    line_radius: i32,
    is_power: bool,
) {
    let width = grid.width;
    let height = grid.height;

    // Collect all source positions
    let mut sources: Vec<(usize, usize)> = Vec::new();
    for row in 0..height {
        for col in 0..width {
            if grid.get(col, row).tile == source_type {
                sources.push((col, row));
            }
        }
    }

    // BFS from each source
    let mut visited = vec![false; width * height];
    let mut queue: VecDeque<(usize, usize, i32)> = VecDeque::new();

    for &(col, row) in &sources {
        let idx = row * width + col;
        if !visited[idx] {
            visited[idx] = true;
            queue.push_back((col, row, source_radius));
            set_utility(grid, col, row, is_power);
        }
    }

    while let Some((col, row, remaining_reach)) = queue.pop_front() {
        if remaining_reach <= 0 {
            continue;
        }

        let neighbors: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for (dc, dr) in &neighbors {
            let nc = col as i32 + dc;
            let nr = row as i32 + dr;
            if nc < 0 || nc >= width as i32 || nr < 0 || nr >= height as i32 {
                continue;
            }
            let nc = nc as usize;
            let nr = nr as usize;
            let idx = nr * width + nc;

            if visited[idx] {
                continue;
            }
            visited[idx] = true;

            let tile = grid.get(nc, nr).tile;

            // Skip water bodies
            if tile == TileType::WaterBody {
                continue;
            }

            // Set utility on this cell
            set_utility(grid, nc, nr, is_power);

            // If this is a line tile, it acts as a new source with line_radius
            let next_reach = if tile == line_type {
                line_radius
            } else {
                remaining_reach - 1
            };

            if next_reach > 0 {
                queue.push_back((nc, nr, next_reach));
            }
        }
    }
}

fn set_utility(grid: &mut Grid, col: usize, row: usize, is_power: bool) {
    let cell = grid.get_mut(col, row);
    if is_power {
        cell.has_power = true;
    } else {
        cell.has_water = true;
    }
}

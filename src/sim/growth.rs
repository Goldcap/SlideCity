use std::collections::VecDeque;

use ::rand::rngs::SmallRng;
use ::rand::Rng;

use crate::grid::{Grid, TileType};

/// Grow a blob of tiles via BFS from a seed point.
/// Does not cross Road or WaterBody. Produces organic shapes via weighted random expansion.
pub fn grow_blob(
    grid: &mut Grid,
    start_col: usize,
    start_row: usize,
    tile: TileType,
    target_size: usize,
    rng: &mut SmallRng,
) -> usize {
    if !grid.in_bounds(start_col, start_row) {
        return 0;
    }

    // Can only grow on empty cells
    if grid.get(start_col, start_row).tile != TileType::Empty {
        return 0;
    }

    let mut placed = 0;
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
    let mut visited = vec![false; grid.width * grid.height];

    // Place seed
    let cell = grid.get_mut(start_col, start_row);
    cell.tile = tile;
    cell.age = 0;
    cell.style = rng.gen_range(0..4);
    placed += 1;

    let idx = grid.idx(start_col, start_row);
    visited[idx] = true;

    // Add neighbors to expansion queue
    add_neighbors(&mut queue, &mut visited, grid, start_col, start_row);

    // BFS expansion with randomized priority
    while placed < target_size {
        if queue.is_empty() {
            break;
        }

        // Pick a random candidate from the queue (organic shape)
        let pick_idx = rng.gen_range(0..queue.len());
        let (col, row) = queue[pick_idx];
        queue.swap_remove_back(pick_idx);

        let candidate = grid.get(col, row);

        // Only grow onto empty cells
        if candidate.tile != TileType::Empty {
            continue;
        }

        // Place the tile
        let cell = grid.get_mut(col, row);
        cell.tile = tile;
        cell.age = 0;
        cell.style = rng.gen_range(0..4);
        placed += 1;

        add_neighbors(&mut queue, &mut visited, grid, col, row);
    }

    placed
}

fn add_neighbors(
    queue: &mut VecDeque<(usize, usize)>,
    visited: &mut [bool],
    grid: &Grid,
    col: usize,
    row: usize,
) {
    let neighbors: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    for (dc, dr) in &neighbors {
        let nc = col as i32 + dc;
        let nr = row as i32 + dr;
        if nc < 0 || nc >= grid.width as i32 || nr < 0 || nr >= grid.height as i32 {
            continue;
        }
        let nc = nc as usize;
        let nr = nr as usize;
        let idx = grid.idx(nc, nr);

        if visited[idx] {
            continue;
        }
        visited[idx] = true;

        // Don't cross roads or water
        let tile = grid.get(nc, nr).tile;
        if tile == TileType::Road || tile == TileType::WaterBody {
            continue;
        }

        queue.push_back((nc, nr));
    }
}

/// Generate a random blob size for a given zone type.
pub fn blob_size(tile: TileType, rng: &mut SmallRng) -> usize {
    match tile {
        TileType::Residential => rng.gen_range(8..=28),
        TileType::Commercial => rng.gen_range(4..=16),
        TileType::Industrial => rng.gen_range(10..=32),
        TileType::Park => rng.gen_range(4..=14),
        _ => 1,
    }
}

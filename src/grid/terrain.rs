use ::rand::rngs::SmallRng;
use ::rand::Rng;
use std::collections::VecDeque;

use super::{Cell, Grid, TileType};

/// Generate terrain with organic hills and water bodies.
pub fn generate_terrain(width: usize, height: usize, rng: &mut SmallRng) -> Grid {
    let mut grid = Grid::new(width, height);

    // 1. Generate heightmap using layered sine waves + noise
    for row in 0..height {
        for col in 0..width {
            let r = row as f32;
            let c = col as f32;

            let h = (r * 0.18).sin() * 0.3
                + (c * 0.14).cos() * 0.3
                + (r * 0.4 + 1.0).sin() * 0.1
                + (c * 0.35 + 2.0).cos() * 0.1
                + rng.gen_range(-0.1..0.1);

            // Normalize to 0.0-1.0 range
            let h = (h + 0.8) / 1.6;
            let h = h.clamp(0.0, 1.0);

            let cell = grid.get_mut(col, row);
            cell.terrain_height = h;
        }
    }

    // 2. Place 1-2 water body regions
    let max_water_cells = (width * height * 6) / 100; // 6% max
    let num_bodies = rng.gen_range(1..=2);
    let mut total_water = 0;

    for _ in 0..num_bodies {
        if total_water >= max_water_cells {
            break;
        }

        // Find a low-elevation starting point
        let start = find_low_point(&grid, rng);
        if let Some((sc, sr)) = start {
            let budget = max_water_cells.saturating_sub(total_water);
            let placed = flood_fill_water(&mut grid, sc, sr, budget);
            total_water += placed;
        }
    }

    grid
}

/// Find a random low-elevation point on the grid.
fn find_low_point(grid: &Grid, rng: &mut SmallRng) -> Option<(usize, usize)> {
    // Collect cells below the 25th percentile height
    let mut heights: Vec<f32> = grid.cells.iter().map(|c| c.terrain_height).collect();
    heights.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let threshold = heights[heights.len() / 4];

    let candidates: Vec<(usize, usize)> = (0..grid.height)
        .flat_map(|row| (0..grid.width).map(move |col| (col, row)))
        .filter(|&(col, row)| grid.get(col, row).terrain_height <= threshold)
        .collect();

    if candidates.is_empty() {
        return None;
    }

    Some(candidates[rng.gen_range(0..candidates.len())])
}

/// Flood-fill water from a starting point to adjacent low cells.
fn flood_fill_water(grid: &mut Grid, start_col: usize, start_row: usize, budget: usize) -> usize {
    let start_height = grid.get(start_col, start_row).terrain_height;
    // Allow water to fill cells up to slightly above the start height
    let height_limit = start_height + 0.15;

    let mut queue = VecDeque::new();
    let mut visited = vec![false; grid.width * grid.height];
    let mut placed = 0;

    queue.push_back((start_col, start_row));
    visited[grid.idx(start_col, start_row)] = true;

    while let Some((col, row)) = queue.pop_front() {
        if placed >= budget {
            break;
        }

        let cell = grid.get(col, row);
        if cell.terrain_height > height_limit || cell.tile == TileType::WaterBody {
            continue;
        }

        *grid.get_mut(col, row) = Cell::water(cell.terrain_height);
        placed += 1;

        // Check 4-connected neighbors
        let neighbors: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for (dc, dr) in &neighbors {
            let nc = col as i32 + dc;
            let nr = row as i32 + dr;
            if nc >= 0 && nc < grid.width as i32 && nr >= 0 && nr < grid.height as i32 {
                let nc = nc as usize;
                let nr = nr as usize;
                let idx = grid.idx(nc, nr);
                if !visited[idx] {
                    visited[idx] = true;
                    queue.push_back((nc, nr));
                }
            }
        }
    }

    placed
}

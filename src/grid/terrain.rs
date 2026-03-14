use ::rand::rngs::SmallRng;
use ::rand::Rng;
use std::collections::VecDeque;

use super::{Cell, Grid, TerrainType, TileType};

/// Generate terrain with organic hills, water bodies, beaches, trees, and rock.
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

        let start = find_low_point(&grid, rng);
        if let Some((sc, sr)) = start {
            let budget = max_water_cells.saturating_sub(total_water);
            let placed = flood_fill_water(&mut grid, sc, sr, budget);
            total_water += placed;
        }
    }

    // 3. Assign terrain sub-types based on height and proximity to water
    assign_terrain_types(&mut grid, rng);

    grid
}

/// Assign visual terrain sub-types to all empty cells.
fn assign_terrain_types(grid: &mut Grid, rng: &mut SmallRng) {
    let width = grid.width;
    let height = grid.height;

    // First pass: mark beach cells (empty cells adjacent to water)
    let mut is_beach = vec![false; width * height];
    for row in 0..height {
        for col in 0..width {
            if grid.get(col, row).tile != TileType::Empty {
                continue;
            }
            // Check if adjacent to water (including diagonals)
            let neighbors: [(i32, i32); 8] = [
                (-1, -1), (0, -1), (1, -1),
                (-1, 0),           (1, 0),
                (-1, 1),  (0, 1),  (1, 1),
            ];
            for (dc, dr) in &neighbors {
                let nc = col as i32 + dc;
                let nr = row as i32 + dr;
                if nc >= 0 && nc < width as i32 && nr >= 0 && nr < height as i32 {
                    if grid.get(nc as usize, nr as usize).tile == TileType::WaterBody {
                        is_beach[row * width + col] = true;
                        break;
                    }
                }
            }
        }
    }

    // Second pass: assign terrain types
    for row in 0..height {
        for col in 0..width {
            if grid.get(col, row).tile != TileType::Empty {
                continue;
            }

            let h = grid.get(col, row).terrain_height;
            let idx = row * width + col;

            let terrain_type = if is_beach[idx] {
                // Beach near water — also extend 1 more cell for wider beaches
                if rng.gen::<f32>() < 0.8 {
                    TerrainType::Sand
                } else {
                    TerrainType::Dirt
                }
            } else if h > 0.9 {
                // Highest peaks: snow caps
                TerrainType::Snow
            } else if h > 0.78 {
                // High elevation: rocky
                if rng.gen::<f32>() < 0.7 {
                    TerrainType::Rock
                } else {
                    TerrainType::Dirt
                }
            } else if h > 0.6 {
                // Mid-high: sparse trees and grass
                match rng.gen_range(0..4) {
                    0 => TerrainType::TreesSparse,
                    1 => TerrainType::GrassFlower,
                    _ => TerrainType::Grass,
                }
            } else if h > 0.35 {
                // Mid elevation: lush trees
                match rng.gen_range(0..5) {
                    0 => TerrainType::Trees,
                    1 => TerrainType::Trees,
                    2 => TerrainType::TreesSparse,
                    3 => TerrainType::GrassFlower,
                    _ => TerrainType::Grass,
                }
            } else if h > 0.2 {
                // Low-mid: grass with some trees
                match rng.gen_range(0..4) {
                    0 => TerrainType::TreesSparse,
                    1 => TerrainType::GrassFlower,
                    _ => TerrainType::Grass,
                }
            } else {
                // Very low: dirt and sparse grass (near water level)
                if rng.gen::<f32>() < 0.3 {
                    TerrainType::Dirt
                } else {
                    TerrainType::Grass
                }
            };

            grid.get_mut(col, row).terrain_type = terrain_type;
        }
    }

    // Third pass: extend beaches slightly (2nd ring gets partial sand)
    let beach_copy = is_beach.clone();
    for row in 0..height {
        for col in 0..width {
            if grid.get(col, row).tile != TileType::Empty {
                continue;
            }
            let idx = row * width + col;
            if beach_copy[idx] {
                continue; // Already a beach
            }

            // Check if adjacent to a beach cell
            let neighbors: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
            let mut near_beach = false;
            for (dc, dr) in &neighbors {
                let nc = col as i32 + dc;
                let nr = row as i32 + dr;
                if nc >= 0 && nc < width as i32 && nr >= 0 && nr < height as i32 {
                    let nidx = nr as usize * width + nc as usize;
                    if beach_copy[nidx] {
                        near_beach = true;
                        break;
                    }
                }
            }

            if near_beach && rng.gen::<f32>() < 0.4 {
                grid.get_mut(col, row).terrain_type = if rng.gen::<f32>() < 0.5 {
                    TerrainType::Sand
                } else {
                    TerrainType::Dirt
                };
            }
        }
    }
}

/// Find a random low-elevation point on the grid.
fn find_low_point(grid: &Grid, rng: &mut SmallRng) -> Option<(usize, usize)> {
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

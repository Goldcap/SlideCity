pub mod camera;
pub mod iso;
pub mod lighting;
pub mod particles;
pub mod sprites;
pub mod tiles;

use macroquad::prelude::*;

use crate::grid::{Grid, TileType};
use camera::GameCamera;
use iso::grid_to_screen;
use lighting::DayNightCycle;
use particles::ParticleSystem;
use sprites::SpriteAtlas;

/// Draw the entire grid in isometric view with painter's algorithm.
pub fn draw_world(
    grid: &Grid,
    _camera: &GameCamera,
    day_night: &DayNightCycle,
    particles: &ParticleSystem,
    _tick_count: u64,
    sprites: &SpriteAtlas,
) {
    let tint = day_night.tint();

    // Build sorted draw order: (depth, row, col)
    let mut draw_order: Vec<(usize, usize, usize)> =
        Vec::with_capacity(grid.width * grid.height);
    for row in 0..grid.height {
        for col in 0..grid.width {
            draw_order.push((col + row, row, col));
        }
    }
    draw_order.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));

    for &(_, row, col) in &draw_order {
        let cell = grid.get(col, row);
        let height = cell.tile.height_floors(cell.age);
        let pos = grid_to_screen(col, row, height);

        // Utility dimming for unpowered/unwatered zone cells
        let utility_dim = if matches!(
            cell.tile,
            TileType::Residential | TileType::Commercial | TileType::Industrial
        ) && (!cell.has_power || !cell.has_water)
        {
            0.7
        } else {
            1.0
        };

        // Pop-in animation: scale 0→1 over first few ticks (age 0-1)
        let pop_in = if cell.age <= 1
            && matches!(
                cell.tile,
                TileType::Residential
                    | TileType::Commercial
                    | TileType::Industrial
                    | TileType::PowerPlant
                    | TileType::WaterTower
                    | TileType::Monument
            )
        {
            let progress = cell.age as f32 / 2.0;
            ease_out_back(progress.min(1.0))
        } else {
            1.0
        };

        tiles::draw_cell_tinted(cell, pos, utility_dim, pop_in, tint, sprites);
    }

    // Draw particles on top of world
    particles.draw();
}

/// Ease-out-back for pop-in animation: slight overshoot for bouncy feel.
fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

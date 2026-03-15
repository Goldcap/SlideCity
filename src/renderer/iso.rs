use macroquad::prelude::*;

/// Isometric tile dimensions.
pub const TILE_W: f32 = 64.0;
pub const TILE_H: f32 = 32.0;
pub const Z_SCALE: f32 = 8.0;

/// Convert grid (col, row) to world-space screen position.
pub fn grid_to_screen(col: usize, row: usize, height_floors: f32) -> Vec2 {
    let x = (col as f32 - row as f32) * TILE_W / 2.0;
    let y = (col as f32 + row as f32) * TILE_H / 2.0 - height_floors * Z_SCALE;
    vec2(x, y)
}

/// Convert screen-space position (after camera transform) back to grid coordinates.
/// Returns None if the position is outside the grid bounds.
pub fn screen_to_grid(world_pos: Vec2, grid_width: usize, grid_height: usize) -> Option<(usize, usize)> {
    let col_f = (world_pos.x / (TILE_W / 2.0) + world_pos.y / (TILE_H / 2.0)) / 2.0;
    let row_f = (world_pos.y / (TILE_H / 2.0) - world_pos.x / (TILE_W / 2.0)) / 2.0;

    let col = col_f.round() as i32;
    let row = row_f.round() as i32;

    if col >= 0 && col < grid_width as i32 && row >= 0 && row < grid_height as i32 {
        Some((col as usize, row as usize))
    } else {
        None
    }
}

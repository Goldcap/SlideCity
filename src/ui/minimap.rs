use macroquad::prelude::*;

use crate::grid::{Grid, TileType};
use crate::renderer::camera::GameCamera;
use crate::renderer::iso::{TILE_W, TILE_H};

/// Size of each cell on the minimap (in pixels).
const CELL_SIZE: f32 = 2.0;

/// Draw a minimap in the bottom-left corner.
/// Returns true if the player clicked on the minimap (with the target grid coords).
pub fn draw_minimap(grid: &Grid, camera: &GameCamera) -> Option<(usize, usize)> {
    let map_w = grid.width as f32 * CELL_SIZE;
    let map_h = grid.height as f32 * CELL_SIZE;
    let margin = 10.0;
    let map_x = margin;
    let map_y = screen_height() - map_h - margin - 20.0; // Above the controls hint

    // Background
    draw_rectangle(
        map_x - 2.0,
        map_y - 2.0,
        map_w + 4.0,
        map_h + 4.0,
        Color::new(0.0, 0.0, 0.0, 0.7),
    );

    // Draw cells
    for row in 0..grid.height {
        for col in 0..grid.width {
            let cell = grid.get(col, row);
            let px = map_x + col as f32 * CELL_SIZE;
            let py = map_y + row as f32 * CELL_SIZE;

            let (r, g, b) = if cell.tile == TileType::Empty {
                cell.terrain_type.color()
            } else {
                cell.tile.color()
            };

            draw_rectangle(px, py, CELL_SIZE, CELL_SIZE, Color::new(r, g, b, 1.0));
        }
    }

    // Camera viewport indicator
    // Convert camera world-space target to approximate grid position
    // The camera target is in isometric world space, so we need to reverse the projection
    let cam_target = camera.target;
    // Approximate: from isometric back to grid
    let approx_col = (cam_target.x / (TILE_W / 2.0) + cam_target.y / (TILE_H / 2.0)) / 2.0;
    let approx_row = (cam_target.y / (TILE_H / 2.0) - cam_target.x / (TILE_W / 2.0)) / 2.0;

    // Viewport size in grid cells (approximate based on zoom and screen size)
    let cells_visible_x = screen_width() / (TILE_W * camera.zoom);
    let cells_visible_y = screen_height() / (TILE_H * camera.zoom);

    let vp_x = map_x + (approx_col - cells_visible_x / 2.0).max(0.0) * CELL_SIZE;
    let vp_y = map_y + (approx_row - cells_visible_y / 2.0).max(0.0) * CELL_SIZE;
    let vp_w = (cells_visible_x * CELL_SIZE).min(map_w);
    let vp_h = (cells_visible_y * CELL_SIZE).min(map_h);

    draw_rectangle_lines(
        vp_x, vp_y, vp_w, vp_h,
        1.0,
        Color::new(1.0, 1.0, 1.0, 0.7),
    );

    // Click to pan
    let (mx, my) = mouse_position();
    if is_mouse_button_pressed(MouseButton::Left)
        && mx >= map_x && mx <= map_x + map_w
        && my >= map_y && my <= map_y + map_h
    {
        let click_col = ((mx - map_x) / CELL_SIZE) as usize;
        let click_row = ((my - map_y) / CELL_SIZE) as usize;
        if click_col < grid.width && click_row < grid.height {
            return Some((click_col, click_row));
        }
    }

    None
}

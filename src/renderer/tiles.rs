use macroquad::prelude::*;

use crate::grid::Cell;
use super::iso::{TILE_W, TILE_H, Z_SCALE};

/// Draw a cell with day/night tint applied.
pub fn draw_cell_tinted(
    cell: &Cell,
    screen_pos: Vec2,
    utility_dim: f32,
    pop_in_scale: f32,
    tint: Color,
) {
    draw_cell(cell, screen_pos, utility_dim, pop_in_scale, tint);
}

/// Draw a single cell as colored isometric diamond + building height.
fn draw_cell(
    cell: &Cell,
    screen_pos: Vec2,
    utility_dim: f32,
    pop_in_scale: f32,
    tint: Color,
) {
    let height = cell.tile.height_floors(cell.age);
    let (r, g, b) = cell.tile.color();

    // Shade by terrain height + day/night tint
    let shade = 0.7 + cell.terrain_height * 0.3;
    let color = Color::new(
        r * shade * utility_dim * tint.r,
        g * shade * utility_dim * tint.g,
        b * shade * utility_dim * tint.b,
        1.0,
    );

    let hw = TILE_W / 2.0;
    let hh = TILE_H / 2.0;
    let cx = screen_pos.x;
    let cy = screen_pos.y;

    // Draw isometric diamond (two triangles)
    let top = vec2(cx, cy - hh);
    let right = vec2(cx + hw, cy);
    let bottom = vec2(cx, cy + hh);
    let left = vec2(cx - hw, cy);

    draw_triangle(top, right, bottom, color);
    draw_triangle(top, left, bottom, color);

    // Draw building height
    if height > 0.0 {
        let building_h = height * Z_SCALE * pop_in_scale;
        let building_w = TILE_W * 0.4 * pop_in_scale;

        if building_h > 0.1 {
            // Building front face (slightly darker)
            draw_rectangle(
                cx - building_w / 2.0,
                cy - hh - building_h,
                building_w,
                building_h,
                Color::new(
                    r * 0.85 * utility_dim * tint.r,
                    g * 0.85 * utility_dim * tint.g,
                    b * 0.85 * utility_dim * tint.b,
                    1.0,
                ),
            );

            // Building top face (slightly lighter)
            draw_rectangle(
                cx - building_w / 2.0,
                cy - hh - building_h,
                building_w,
                3.0 * pop_in_scale,
                Color::new(
                    (r * 1.1 * tint.r).min(1.0),
                    (g * 1.1 * tint.g).min(1.0),
                    (b * 1.1 * tint.b).min(1.0),
                    1.0,
                ),
            );
        }
    }

    // Draw label
    let label = cell.tile.label();
    if !label.is_empty() {
        let label_y = if height > 0.0 {
            cy - hh - height * Z_SCALE * pop_in_scale + 12.0
        } else {
            cy + 4.0
        };
        draw_text(label, cx - 4.0, label_y, 14.0, WHITE);
    }
}

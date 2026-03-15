use macroquad::prelude::*;

use crate::grid::{Cell, TileType};
use super::iso::{TILE_W, TILE_H, Z_SCALE};
use super::sprites::SpriteAtlas;

/// Terrain elevation scale: pixels per 0.1 height unit.
const TERRAIN_ELEV_SCALE: f32 = 24.0;

/// Draw a cell with day/night tint applied.
pub fn draw_cell_tinted(
    cell: &Cell,
    screen_pos: Vec2,
    utility_dim: f32,
    pop_in_scale: f32,
    tint: Color,
    sprites: &SpriteAtlas,
) {
    let elev_offset = cell.terrain_height * TERRAIN_ELEV_SCALE;
    let elevated_pos = vec2(screen_pos.x, screen_pos.y - elev_offset);

    // Try sprite first, fall back to colored rects
    if let Some(tex) = sprites.get_for_cell(cell) {
        draw_cell_sprite(cell, elevated_pos, utility_dim, pop_in_scale, tint, tex);
    } else {
        draw_cell_fallback(cell, elevated_pos, utility_dim, pop_in_scale, tint);
    }
}

/// Draw a cell using a sprite texture.
fn draw_cell_sprite(
    cell: &Cell,
    screen_pos: Vec2,
    utility_dim: f32,
    pop_in_scale: f32,
    tint: Color,
    tex: &Texture2D,
) {
    let tex_w = tex.width();
    let tex_h = tex.height();

    // Apply tint + utility dimming to the sprite color
    let shade = 0.75 + cell.terrain_height * 0.25;
    let color = Color::new(
        (shade * utility_dim * tint.r).min(1.0),
        (shade * utility_dim * tint.g).min(1.0),
        (shade * utility_dim * tint.b).min(1.0),
        1.0,
    );

    let height = cell.tile.height_floors(cell.age);
    let scale = if height > 0.0 { pop_in_scale } else { 1.0 };

    // Constrain sprite to the isometric tile footprint (TILE_W wide).
    // Scale proportionally: width = TILE_W, height preserves aspect ratio.
    let aspect = tex_h / tex_w;
    let draw_w = TILE_W * scale;
    let draw_h = TILE_W * aspect * scale;

    // Center horizontally on the isometric diamond center.
    // Anchor bottom of sprite to the bottom of the diamond.
    let draw_x = screen_pos.x - draw_w / 2.0;
    let draw_y = screen_pos.y + TILE_H / 2.0 - draw_h;

    draw_texture_ex(
        tex,
        draw_x,
        draw_y,
        color,
        DrawTextureParams {
            dest_size: Some(vec2(draw_w, draw_h)),
            ..Default::default()
        },
    );
}

/// Fallback: draw a cell using colored geometric primitives (original renderer).
fn draw_cell_fallback(
    cell: &Cell,
    screen_pos: Vec2,
    utility_dim: f32,
    pop_in_scale: f32,
    tint: Color,
) {
    let height = cell.tile.height_floors(cell.age);

    let (r, g, b) = if cell.tile == TileType::Empty {
        cell.terrain_type.color()
    } else {
        cell.tile.color()
    };

    let shade = 0.75 + cell.terrain_height * 0.25;
    let color = Color::new(
        (r * shade * utility_dim * tint.r).min(1.0),
        (g * shade * utility_dim * tint.g).min(1.0),
        (b * shade * utility_dim * tint.b).min(1.0),
        1.0,
    );

    let hw = TILE_W / 2.0;
    let hh = TILE_H / 2.0;
    let cx = screen_pos.x;
    let cy = screen_pos.y;

    // Isometric diamond
    let top = vec2(cx, cy - hh);
    let right = vec2(cx + hw, cy);
    let bottom = vec2(cx, cy + hh);
    let left = vec2(cx - hw, cy);
    draw_triangle(top, right, bottom, color);
    draw_triangle(top, left, bottom, color);

    // Terrain elevation sides
    if cell.terrain_height > 0.3 && cell.tile == TileType::Empty {
        let side_h = (cell.terrain_height - 0.3) * TERRAIN_ELEV_SCALE * 0.5;
        if side_h > 1.0 {
            let side_color = Color::new(
                (r * 0.5 * tint.r).min(1.0),
                (g * 0.5 * tint.g).min(1.0),
                (b * 0.5 * tint.b).min(1.0),
                1.0,
            );
            let br = vec2(cx + hw, cy);
            let bb = vec2(cx, cy + hh);
            let br2 = vec2(cx + hw, cy + side_h);
            let bb2 = vec2(cx, cy + hh + side_h);
            draw_triangle(br, bb, bb2, side_color);
            draw_triangle(br, br2, bb2, side_color);

            let darker = Color::new(
                (r * 0.4 * tint.r).min(1.0),
                (g * 0.4 * tint.g).min(1.0),
                (b * 0.4 * tint.b).min(1.0),
                1.0,
            );
            let bl = vec2(cx - hw, cy);
            let bl2 = vec2(cx - hw, cy + side_h);
            draw_triangle(bl, bb, bb2, darker);
            draw_triangle(bl, bl2, bb2, darker);
        }
    }

    // Trees
    if cell.tile == TileType::Empty && cell.terrain_type.has_trees() {
        draw_trees(cx, cy - hh, cell.terrain_type.tree_density(), cell.style, tint);
    }

    // Water shimmer
    if cell.tile == TileType::WaterBody {
        let highlight = Color::new(
            (0.2 * tint.r).min(1.0),
            (0.45 * tint.g).min(1.0),
            (0.85 * tint.b).min(1.0),
            0.3,
        );
        let small_hw = hw * 0.3;
        let small_hh = hh * 0.3;
        let st = vec2(cx, cy - small_hh);
        let sr = vec2(cx + small_hw, cy);
        let sb = vec2(cx, cy + small_hh);
        let sl = vec2(cx - small_hw, cy);
        draw_triangle(st, sr, sb, highlight);
        draw_triangle(st, sl, sb, highlight);
    }

    // Building height
    if height > 0.0 {
        let building_h = height * Z_SCALE * pop_in_scale;
        let building_w = TILE_W * 0.4 * pop_in_scale;
        if building_h > 0.1 {
            draw_rectangle(
                cx - building_w / 2.0,
                cy - hh - building_h,
                building_w,
                building_h,
                Color::new(
                    (r * 0.85 * utility_dim * tint.r).min(1.0),
                    (g * 0.85 * utility_dim * tint.g).min(1.0),
                    (b * 0.85 * utility_dim * tint.b).min(1.0),
                    1.0,
                ),
            );
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

    // Label
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

/// Draw tree shapes (fallback).
fn draw_trees(cx: f32, top_y: f32, density: f32, style: u8, tint: Color) {
    let positions: &[(f32, f32)] = match style % 4 {
        0 => &[(0.0, 0.0), (-8.0, 4.0), (6.0, 3.0)],
        1 => &[(-5.0, 2.0), (7.0, 1.0), (0.0, 5.0)],
        2 => &[(3.0, 0.0), (-6.0, 3.0), (0.0, -2.0)],
        _ => &[(-3.0, 1.0), (5.0, 4.0), (-7.0, -1.0)],
    };
    let num_trees = if density > 0.6 { 3 } else { 2 };
    for &(dx, dy) in positions.iter().take(num_trees) {
        let tx = cx + dx;
        let ty = top_y + dy;
        let trunk_color = Color::new(
            (0.35 * tint.r).min(1.0),
            (0.25 * tint.g).min(1.0),
            (0.15 * tint.b).min(1.0),
            1.0,
        );
        draw_rectangle(tx - 1.0, ty - 4.0, 2.0, 6.0, trunk_color);
        let canopy_color = Color::new(
            (0.1 * tint.r).min(1.0),
            (0.45 * tint.g).min(1.0),
            (0.1 * tint.b).min(1.0),
            0.9,
        );
        draw_circle(tx, ty - 7.0, 4.5, canopy_color);
        let highlight = Color::new(
            (0.15 * tint.r).min(1.0),
            (0.55 * tint.g).min(1.0),
            (0.15 * tint.b).min(1.0),
            0.7,
        );
        draw_circle(tx - 1.0, ty - 8.0, 2.5, highlight);
    }
}

mod config;
mod grid;

use config::SimConfig;
use grid::terrain::generate_terrain;
use macroquad::prelude::*;
use ::rand::rngs::SmallRng;
use ::rand::SeedableRng;

/// Isometric tile dimensions.
const TILE_W: f32 = 64.0;
const TILE_H: f32 = 32.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "SlideCity".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        ..Default::default()
    }
}

/// Convert grid (col, row) to screen (x, y) in world space.
fn grid_to_screen(col: usize, row: usize, height_floors: f32) -> Vec2 {
    let x = (col as f32 - row as f32) * TILE_W / 2.0;
    let y = (col as f32 + row as f32) * TILE_H / 2.0 - height_floors * 8.0;
    vec2(x, y)
}

#[macroquad::main(window_conf)]
async fn main() {
    let config = SimConfig::default();
    let mut rng = SmallRng::seed_from_u64(42);
    let grid = generate_terrain(config.grid_width, config.grid_height, &mut rng);

    // Camera state
    let mut cam_target = vec2(0.0, (config.grid_height as f32) * TILE_H / 2.0);
    let mut cam_zoom = 0.75_f32;
    let mut last_mouse: Option<Vec2> = None;

    loop {
        let dt = get_frame_time();

        // --- Input: zoom ---
        let (_, wheel_y) = mouse_wheel();
        if wheel_y != 0.0 {
            let zoom_levels = [0.5_f32, 0.75, 1.0, 1.5, 2.0];
            let dir = wheel_y.signum();
            let cur_idx = zoom_levels
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    ((**a) - cam_zoom)
                        .abs()
                        .partial_cmp(&((**b) - cam_zoom).abs())
                        .unwrap()
                })
                .map(|(i, _)| i)
                .unwrap_or(2);
            let new_idx = if dir > 0.0 {
                (cur_idx + 1).min(zoom_levels.len() - 1)
            } else {
                cur_idx.saturating_sub(1)
            };
            cam_zoom = zoom_levels[new_idx];
        }

        // --- Input: pan ---
        let mouse = vec2(mouse_position().0, mouse_position().1);
        if is_mouse_button_down(MouseButton::Left) {
            if let Some(prev) = last_mouse {
                let delta = mouse - prev;
                cam_target.x -= delta.x / cam_zoom;
                cam_target.y -= delta.y / cam_zoom;
            }
            last_mouse = Some(mouse);
        } else {
            last_mouse = None;
        }

        // Arrow key pan
        let pan_speed = 400.0 / cam_zoom * dt;
        if is_key_down(KeyCode::Left) {
            cam_target.x -= pan_speed;
        }
        if is_key_down(KeyCode::Right) {
            cam_target.x += pan_speed;
        }
        if is_key_down(KeyCode::Up) {
            cam_target.y -= pan_speed;
        }
        if is_key_down(KeyCode::Down) {
            cam_target.y += pan_speed;
        }

        // --- Set camera ---
        let camera = Camera2D {
            target: cam_target,
            zoom: vec2(
                cam_zoom / screen_width() * 2.0,
                cam_zoom / screen_height() * 2.0,
            ),
            ..Default::default()
        };
        set_camera(&camera);

        // --- Draw world ---
        clear_background(Color::new(0.08, 0.10, 0.06, 1.0));

        // Painter's algorithm: sort by (row + col) ascending, secondary by row descending
        let mut draw_order: Vec<(usize, usize, usize)> = Vec::with_capacity(grid.width * grid.height);
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

            let (r, g, b) = cell.tile.color();
            // Shade by terrain height
            let shade = 0.7 + cell.terrain_height * 0.3;
            let color = Color::new(r * shade, g * shade, b * shade, 1.0);

            // Draw isometric diamond
            let hw = TILE_W / 2.0;
            let hh = TILE_H / 2.0;
            let cx = pos.x;
            let cy = pos.y;

            // Fill diamond as two triangles
            let top = vec2(cx, cy - hh);
            let right = vec2(cx + hw, cy);
            let bottom = vec2(cx, cy + hh);
            let left = vec2(cx - hw, cy);

            draw_triangle(top, right, bottom, color);
            draw_triangle(top, left, bottom, color);

            // Draw building height as a colored rect above the diamond
            if height > 0.0 {
                let building_h = height * 8.0;
                let building_w = TILE_W * 0.4;
                draw_rectangle(
                    cx - building_w / 2.0,
                    cy - hh - building_h,
                    building_w,
                    building_h,
                    Color::new(r * 0.9, g * 0.9, b * 0.9, 1.0),
                );
            }

            // Draw label
            let label = cell.tile.label();
            if !label.is_empty() {
                draw_text(label, cx - 4.0, cy + 4.0, 14.0, WHITE);
            }
        }

        // --- UI (screen space) ---
        set_default_camera();

        let pop = grid.population();
        let water_count = grid.count_type(grid::TileType::WaterBody);
        let total = grid.width * grid.height;

        draw_rectangle(0.0, 0.0, screen_width(), 32.0, Color::new(0.0, 0.0, 0.0, 0.7));
        draw_text(
            &format!(
                "SlideCity | Pop: {} | Grid: {}x{} | Water: {:.1}% | Zoom: {:.0}%",
                pop,
                grid.width,
                grid.height,
                water_count as f32 / total as f32 * 100.0,
                cam_zoom * 100.0,
            ),
            10.0,
            22.0,
            20.0,
            WHITE,
        );

        next_frame().await;
    }
}

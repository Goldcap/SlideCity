mod config;
mod grid;
mod sim;

use config::SimConfig;
use grid::terrain::generate_terrain;
use grid::TileType;
use macroquad::prelude::*;
use sim::stats::CityStats;
use ::rand::rngs::SmallRng;
use ::rand::SeedableRng;

/// Isometric tile dimensions.
const TILE_W: f32 = 64.0;
const TILE_H: f32 = 32.0;
const Z_SCALE: f32 = 8.0;

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
    let y = (col as f32 + row as f32) * TILE_H / 2.0 - height_floors * Z_SCALE;
    vec2(x, y)
}

/// Place initial test infrastructure so the simulation has something to grow from.
fn place_test_infrastructure(grid: &mut grid::Grid, rng: &mut SmallRng) {
    let cx = grid.width / 2;
    let cy = grid.height / 2;

    // Horizontal road across center
    for col in (cx - 15)..=(cx + 15) {
        if grid.in_bounds(col, cy) {
            let cell = grid.get_mut(col, cy);
            cell.tile = TileType::Road;
            cell.age = 0;
        }
    }

    // Vertical road crossing center
    for row in (cy - 10)..=(cy + 10) {
        if grid.in_bounds(cx, row) {
            let cell = grid.get_mut(cx, row);
            cell.tile = TileType::Road;
            cell.age = 0;
        }
    }

    // Seed a residential blob near the intersection
    sim::growth::grow_blob(grid, cx + 2, cy + 2, TileType::Residential, 16, rng);
    sim::growth::grow_blob(grid, cx - 3, cy - 3, TileType::Residential, 12, rng);

    // Power plant on the west edge
    let pp_col = cx - 18;
    let pp_row = cy;
    if grid.in_bounds(pp_col, pp_row) {
        let cell = grid.get_mut(pp_col, pp_row);
        cell.tile = TileType::PowerPlant;
        cell.age = 0;
    }

    // Power line from plant toward residential
    for col in (pp_col + 1)..=(cx - 16) {
        if grid.in_bounds(col, cy) {
            let cell = grid.get_mut(col, cy);
            if cell.tile == TileType::Empty {
                cell.tile = TileType::PowerLine;
                cell.age = 0;
            }
        }
    }

    // Water tower on the east edge
    let wt_col = cx + 18;
    let wt_row = cy;
    if grid.in_bounds(wt_col, wt_row) {
        let cell = grid.get_mut(wt_col, wt_row);
        cell.tile = TileType::WaterTower;
        cell.age = 0;
    }

    // Water main from tower toward residential
    for col in ((cx + 16)..wt_col).rev() {
        if grid.in_bounds(col, cy) {
            let cell = grid.get_mut(col, cy);
            if cell.tile == TileType::Empty {
                cell.tile = TileType::WaterMain;
                cell.age = 0;
            }
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let config = SimConfig::default();
    let mut rng = SmallRng::seed_from_u64(42);

    let mut grid = generate_terrain(config.grid_width, config.grid_height, &mut rng);
    let mut next_grid = grid.clone();

    // Place test infrastructure
    place_test_infrastructure(&mut grid, &mut rng);

    // Recompute utilities immediately
    sim::utilities::recompute_utilities(&mut grid);

    let mut funds: i64 = config.starting_funds;
    let mut tick_timer: f32 = 0.0;
    let mut tick_count: u64 = 0;
    let mut stats = CityStats::compute(&grid);

    // Camera state
    let mut cam_target = vec2(0.0, (config.grid_height as f32) * TILE_H / 2.0);
    let mut cam_zoom = 0.75_f32;
    let mut last_mouse: Option<Vec2> = None;

    // Speed control
    let speed_levels = [1.0_f32, 2.0, 4.0, 8.0];
    let mut speed_idx: usize = 0;

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
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            cam_target.x -= pan_speed;
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            cam_target.x += pan_speed;
        }
        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            cam_target.y -= pan_speed;
        }
        if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
            cam_target.y += pan_speed;
        }

        // Speed control: 1-4 keys
        if is_key_pressed(KeyCode::Key1) { speed_idx = 0; }
        if is_key_pressed(KeyCode::Key2) { speed_idx = 1; }
        if is_key_pressed(KeyCode::Key3) { speed_idx = 2; }
        if is_key_pressed(KeyCode::Key4) { speed_idx = 3; }
        let speed = speed_levels[speed_idx];

        // --- Simulation tick ---
        let tick_duration = config.base_tick_ms / 1000.0 / speed;
        tick_timer += dt;
        while tick_timer >= tick_duration {
            tick_timer -= tick_duration;
            tick_count += 1;

            sim::tick(&mut grid, &mut next_grid, &config, &mut rng, &mut funds);

            // Recompute utilities every 5 ticks
            if tick_count.is_multiple_of(config.utility_recompute_interval) {
                sim::utilities::recompute_utilities(&mut grid);
            }

            // Recompute stats
            stats = CityStats::compute(&grid);
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

            let (r, g, b) = cell.tile.color();
            // Shade by terrain height
            let shade = 0.7 + cell.terrain_height * 0.3;

            // Power/water indicator: slightly dim unpowered zone cells
            let utility_dim = if matches!(
                cell.tile,
                TileType::Residential | TileType::Commercial | TileType::Industrial
            ) && (!cell.has_power || !cell.has_water)
            {
                0.7
            } else {
                1.0
            };

            let color = Color::new(
                r * shade * utility_dim,
                g * shade * utility_dim,
                b * shade * utility_dim,
                1.0,
            );

            // Draw isometric diamond
            let hw = TILE_W / 2.0;
            let hh = TILE_H / 2.0;
            let cx = pos.x;
            let cy = pos.y;

            let top = vec2(cx, cy - hh);
            let right = vec2(cx + hw, cy);
            let bottom = vec2(cx, cy + hh);
            let left = vec2(cx - hw, cy);

            draw_triangle(top, right, bottom, color);
            draw_triangle(top, left, bottom, color);

            // Draw building height as a colored rect above the diamond
            if height > 0.0 {
                let building_h = height * Z_SCALE;
                let building_w = TILE_W * 0.4;

                // Building front face (slightly darker)
                draw_rectangle(
                    cx - building_w / 2.0,
                    cy - hh - building_h,
                    building_w,
                    building_h,
                    Color::new(r * 0.85 * utility_dim, g * 0.85 * utility_dim, b * 0.85 * utility_dim, 1.0),
                );

                // Building top face (slightly lighter)
                draw_rectangle(
                    cx - building_w / 2.0,
                    cy - hh - building_h,
                    building_w,
                    3.0,
                    Color::new(r * 1.1, g * 1.1, b * 1.1, 1.0),
                );
            }

            // Draw label
            let label = cell.tile.label();
            if !label.is_empty() {
                let label_y = if height > 0.0 {
                    cy - hh - height * Z_SCALE + 12.0
                } else {
                    cy + 4.0
                };
                draw_text(label, cx - 4.0, label_y, 14.0, WHITE);
            }
        }

        // --- UI (screen space) ---
        set_default_camera();

        let year = tick_count / config.ticks_per_year as u64;
        let season_tick = (tick_count % config.ticks_per_year as u64) / config.ticks_per_season as u64;
        let season = match season_tick {
            0 => "Spring",
            1 => "Summer",
            2 => "Fall",
            _ => "Winter",
        };

        // Top HUD bar
        draw_rectangle(0.0, 0.0, screen_width(), 52.0, Color::new(0.0, 0.0, 0.0, 0.8));

        draw_text(
            &format!(
                "Pop: {:>5}  R:{} C:{} I:{}  ${:>6}  Year {}, {}",
                stats.population,
                stats.res_count,
                stats.com_count,
                stats.ind_count,
                funds,
                year + 1,
                season,
            ),
            10.0,
            20.0,
            18.0,
            WHITE,
        );

        draw_text(
            &format!(
                "Happy: {:.0}%  Power: {:.0}%  Water: {:.0}%  Fire: {}  Speed: {}x  Tick: {}",
                stats.happiness * 100.0,
                stats.power_coverage * 100.0,
                stats.water_coverage * 100.0,
                stats.fire_count,
                speed as u32,
                tick_count,
            ),
            10.0,
            42.0,
            18.0,
            Color::new(0.7, 0.8, 0.7, 1.0),
        );

        // Controls hint
        draw_text(
            "Scroll: zoom | Drag: pan | 1-4: speed",
            screen_width() - 320.0,
            42.0,
            14.0,
            Color::new(0.5, 0.5, 0.5, 1.0),
        );

        next_frame().await;
    }
}

mod config;
mod grid;
mod renderer;
mod sim;

use config::SimConfig;
use grid::terrain::generate_terrain;
use grid::TileType;
use macroquad::prelude::*;
use renderer::camera::GameCamera;
use renderer::iso::TILE_H;
use renderer::lighting::DayNightCycle;
use renderer::particles::ParticleSystem;
use sim::stats::CityStats;
use ::rand::rngs::SmallRng;
use ::rand::SeedableRng;

fn window_conf() -> Conf {
    Conf {
        window_title: "SlideCity".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        ..Default::default()
    }
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

    // Seed residential blobs near the intersection
    sim::growth::grow_blob(grid, cx + 2, cy + 2, TileType::Residential, 16, rng);
    sim::growth::grow_blob(grid, cx - 3, cy - 3, TileType::Residential, 12, rng);

    // Power plant on the west edge
    let pp_col = cx - 18;
    if grid.in_bounds(pp_col, cy) {
        let cell = grid.get_mut(pp_col, cy);
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
    if grid.in_bounds(wt_col, cy) {
        let cell = grid.get_mut(wt_col, cy);
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

    place_test_infrastructure(&mut grid, &mut rng);
    sim::utilities::recompute_utilities(&mut grid);

    let mut funds: i64 = config.starting_funds;
    let mut tick_timer: f32 = 0.0;
    let mut tick_count: u64 = 0;
    let mut stats = CityStats::compute(&grid);

    // Renderer state
    let initial_center = vec2(0.0, (config.grid_height as f32) * TILE_H / 2.0);
    let mut camera = GameCamera::new(initial_center);
    let mut day_night = DayNightCycle::new();
    let mut particles = ParticleSystem::new();

    // Speed control
    let speed_levels = [1.0_f32, 2.0, 4.0, 8.0];
    let mut speed_idx: usize = 0;

    loop {
        let dt = get_frame_time();

        // --- Input ---
        camera.handle_input(dt);

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

            // Spawn particles from grid state
            particles.spawn_from_grid(&grid, &mut rng);

            // Recompute stats
            stats = CityStats::compute(&grid);
        }

        // --- Update renderer state (every frame) ---
        camera.update(dt);
        day_night.update(dt);
        particles.update(dt);

        // --- Set camera and draw world ---
        set_camera(&camera.to_macroquad_camera());
        clear_background(Color::new(0.08, 0.10, 0.06, 1.0));

        renderer::draw_world(&grid, &camera, &day_night, &particles, tick_count);

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
                "Happy: {:.0}%  Power: {:.0}%  Water: {:.0}%  Fire: {}  Speed: {}x  {}  Tick: {}",
                stats.happiness * 100.0,
                stats.power_coverage * 100.0,
                stats.water_coverage * 100.0,
                stats.fire_count,
                speed as u32,
                day_night.phase_label(),
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

        // FPS
        draw_text(
            &format!("FPS: {}", get_fps()),
            screen_width() - 100.0,
            20.0,
            14.0,
            Color::new(0.5, 0.5, 0.5, 1.0),
        );

        next_frame().await;
    }
}

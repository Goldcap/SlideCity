mod config;
mod grid;
mod mayor;
mod renderer;
mod sim;

use config::SimConfig;
use grid::terrain::generate_terrain;
use macroquad::prelude::*;
use mayor::Mayor;
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

#[macroquad::main(window_conf)]
async fn main() {
    let config = SimConfig::default();
    let mut rng = SmallRng::seed_from_u64(42);

    let mut grid = generate_terrain(config.grid_width, config.grid_height, &mut rng);
    let mut next_grid = grid.clone();

    // Mayor builds the city from scratch — no test infrastructure needed
    let mut mayor = Mayor::new(0); // The Developer (idx 0) — TODO: selection screen
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

    // Monument sting flag
    let mut monument_sting_played = false;

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

            // Mayor decision every 8 ticks
            if tick_count.is_multiple_of(config.mayor_tick_interval) {
                mayor.decide(&mut grid, &stats, &config, &mut funds, tick_count, &mut rng);

                // Process mayor camera requests
                if let Some((x, y)) = mayor.camera_request.take() {
                    camera.pan_to(vec2(x, y));
                }
                if let Some((x, y)) = mayor.shake_request.take() {
                    camera.shake_at(vec2(x, y), 5.0, 0.5);
                }
            }

            // Spawn particles
            particles.spawn_from_grid(&grid, &mut rng);

            // Recompute stats
            stats = CityStats::compute(&grid);

            // Monument sting detection
            if mayor.monument_built && !monument_sting_played {
                monument_sting_played = true;
                // TODO: play monument sting audio
                // Camera will have already been panned by the mayor
            }
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

        let year = tick_count / config.ticks_per_year as u64 + 1;
        let season = mayor::narration::season_name(tick_count, config.ticks_per_season);
        let p = mayor.personality();

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
                year,
                season,
            ),
            10.0,
            20.0,
            18.0,
            WHITE,
        );

        draw_text(
            &format!(
                "Happy: {:.0}%  Power: {:.0}%  Water: {:.0}%  Fire: {}  Speed: {}x  {}  {:?}",
                stats.happiness * 100.0,
                stats.power_coverage * 100.0,
                stats.water_coverage * 100.0,
                stats.fire_count,
                speed as u32,
                day_night.phase_label(),
                mayor.phase,
            ),
            10.0,
            42.0,
            18.0,
            Color::new(0.7, 0.8, 0.7, 1.0),
        );

        // --- Right panel: Mayor log ---
        let panel_x = screen_width() - 300.0;
        draw_rectangle(panel_x, 0.0, 300.0, screen_height(), Color::new(0.0, 0.0, 0.0, 0.7));

        // Mayor identity
        draw_text(
            &format!("{} {}", p.emoji, p.name),
            panel_x + 10.0,
            28.0,
            22.0,
            WHITE,
        );

        // Phase
        draw_text(
            &format!("{:?} | Mayor #{}", mayor.phase, mayor.mayor_number),
            panel_x + 10.0,
            48.0,
            14.0,
            Color::new(0.6, 0.7, 0.6, 1.0),
        );

        // Separator
        draw_line(panel_x + 10.0, 58.0, panel_x + 290.0, 58.0, 1.0, Color::new(0.3, 0.3, 0.3, 1.0));

        // Log entries (last 7, newest first)
        let entries = mayor.log.last_n(7);
        for (i, entry) in entries.iter().enumerate() {
            let y = 80.0 + i as f32 * 65.0;
            let opacity = if i == 0 { 1.0 } else { 0.8 - i as f32 * 0.08 };
            let color = Color::new(1.0, 1.0, 1.0, opacity.max(0.3));
            let header_color = Color::new(0.6, 0.8, 0.6, opacity.max(0.3));

            draw_text(
                &format!("{} Year {}, {}", entry.emoji, entry.year, entry.season),
                panel_x + 10.0,
                y,
                13.0,
                header_color,
            );

            // Word-wrap the text (simple: truncate at ~35 chars per line)
            let text = &entry.text;
            if text.len() > 38 {
                draw_text(&text[..38], panel_x + 10.0, y + 16.0, 14.0, color);
                let rest = if text.len() > 76 { &text[38..76] } else { &text[38..] };
                draw_text(rest, panel_x + 10.0, y + 30.0, 14.0, color);
            } else {
                draw_text(text, panel_x + 10.0, y + 16.0, 14.0, color);
            }
        }

        // Controls hint
        draw_text(
            "1-4: speed | Scroll: zoom | Drag: pan",
            10.0,
            screen_height() - 10.0,
            14.0,
            Color::new(0.4, 0.4, 0.4, 1.0),
        );

        // FPS
        draw_text(
            &format!("FPS: {}", get_fps()),
            panel_x + 220.0,
            screen_height() - 10.0,
            14.0,
            Color::new(0.4, 0.4, 0.4, 1.0),
        );

        next_frame().await;
    }
}

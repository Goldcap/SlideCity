mod audio;
mod config;
mod grid;
mod mayor;
mod renderer;
mod sim;
mod ui;

use audio::AudioManager;
use audio::mood;
use config::SimConfig;
use grid::terrain::generate_terrain;
use grid::TileType;
use macroquad::prelude::*;
use mayor::Mayor;
use renderer::camera::GameCamera;
use renderer::iso::{grid_to_screen, TILE_H};
use renderer::lighting::DayNightCycle;
use renderer::particles::ParticleSystem;
use sim::stats::CityStats;
use ui::{GameState, StartPhase, InfluenceState};
use ui::start_screen::StartScreenState;
use ::rand::rngs::SmallRng;
use ::rand::{Rng, SeedableRng};

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
    // Start screen state
    let mut game_state = GameState::StartScreen(StartPhase::Title);
    let mut start_screen = StartScreenState::new();

    // These are initialized when the game starts (after start screen)
    let mut config = SimConfig::default();
    let mut rng = SmallRng::seed_from_u64(42);
    let mut grid = grid::Grid::new(1, 1); // placeholder
    let mut next_grid = grid.clone();
    let mut mayor = Mayor::new(0);
    let mut funds: i64 = 0;
    let mut tick_timer: f32 = 0.0;
    let mut tick_count: u64 = 0;
    let mut stats = CityStats::default();
    let mut camera = GameCamera::new(vec2(0.0, 0.0));
    let mut day_night = DayNightCycle::new();
    let mut particles = ParticleSystem::new();
    let mut audio_mgr = AudioManager::new();
    let speed_levels = [1.0_f32, 2.0, 4.0, 8.0];
    let mut speed_idx: usize = 0;
    let mut monument_sting_played = false;
    let mut influence = InfluenceState::new();

    loop {
        let dt = get_frame_time();

        match &game_state {
            GameState::StartScreen(phase) => {
                match phase {
                    StartPhase::Title => {
                        if let Some(next) = ui::start_screen::draw_title(&mut start_screen, dt) {
                            game_state = GameState::StartScreen(next);
                        }
                    }
                    StartPhase::MayorSelect => {
                        if let Some(next) = ui::start_screen::draw_mayor_select(&mut start_screen, dt) {
                            game_state = GameState::StartScreen(next);
                        }
                        // ESC goes back to title
                        if is_key_pressed(KeyCode::Escape) {
                            game_state = GameState::StartScreen(StartPhase::Title);
                        }
                    }
                    StartPhase::DifficultySelect => {
                        let confirmed = ui::start_screen::draw_difficulty_select(&mut start_screen, dt);

                        // ESC goes back to mayor select
                        if is_key_pressed(KeyCode::Escape) {
                            game_state = GameState::StartScreen(StartPhase::MayorSelect);
                        } else if confirmed {
                            // Initialize the game with chosen settings
                            let setup = &start_screen.setup;
                            config = SimConfig::new(setup.difficulty);
                            rng = SmallRng::seed_from_u64(setup.seed);
                            grid = generate_terrain(config.grid_width, config.grid_height, &mut rng);
                            next_grid = grid.clone();
                            mayor = Mayor::new(setup.mayor_idx);
                            funds = config.starting_funds;
                            tick_timer = 0.0;
                            tick_count = 0;
                            stats = CityStats::compute(&grid);
                            speed_idx = setup.speed_idx;
                            monument_sting_played = false;
                            influence = InfluenceState::new();

                            let initial_center = vec2(0.0, (config.grid_height as f32) * TILE_H / 2.0);
                            camera = GameCamera::new(initial_center);
                            day_night = DayNightCycle::new();
                            particles = ParticleSystem::new();

                            // Load audio
                            audio_mgr = AudioManager::new();
                            audio_mgr.load_local_tracks().await;
                            if audio_mgr.spotify.available {
                                audio_mgr.backend = audio::AudioBackend::Spotify;
                            }

                            game_state = GameState::Playing;
                        }
                    }
                }
            }

            GameState::Playing => {
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

                    // Audio: re-evaluate mood every 10 ticks
                    if tick_count.is_multiple_of(config.audio_reeval_interval) {
                        let track = mood::select_track(&stats);
                        audio_mgr.transition_to(track);
                    }

                    // Monument sting detection
                    if mayor.monument_built && !monument_sting_played {
                        monument_sting_played = true;
                        audio_mgr.play_sting(mood::TrackId::Monument);
                    }

                    // Influence: yearly IP + milestones
                    let year = tick_count / config.ticks_per_year as u64 + 1;
                    influence.yearly_tick(year as u32);
                    influence.check_milestones(stats.population);
                }

                // --- Update renderer state (every frame) ---
                camera.update(dt);
                day_night.update(dt);
                particles.update(dt);
                audio_mgr.update(dt);
                influence.update(dt);

                // --- Set camera and draw world ---
                set_camera(&camera.to_macroquad_camera());
                clear_background(Color::new(0.08, 0.10, 0.06, 1.0));

                renderer::draw_world(&grid, &camera, &day_night, &particles, tick_count);

                // --- UI (screen space) ---
                set_default_camera();

                let year = tick_count / config.ticks_per_year as u64 + 1;
                let season = mayor::narration::season_name(tick_count, config.ticks_per_season);
                let speed = speed_levels[speed_idx];

                // Top HUD bar
                ui::stats::draw_hud(
                    &stats,
                    funds,
                    year,
                    season,
                    speed,
                    &day_night,
                    mayor.phase,
                    &audio_mgr.current_mood_label,
                );

                // Right panel: mayor log
                let panel_x = ui::mayor_log::draw_mayor_panel(&mayor);

                // Influence UI + disaster button
                let disaster_clicked = ui::influence_ui::draw_influence(&influence, panel_x);
                if disaster_clicked {
                    // Spawn fire on a random developed cell
                    if let Some((col, row)) = find_random_developed(&grid, &mut rng) {
                        grid.get_mut(col, row).tile = TileType::Fire;
                        grid.get_mut(col, row).age = 0;
                        influence.disaster_triggered();
                        influence.disaster_cooldown = config.disaster_cooldown_secs;

                        // Camera shake to disaster
                        let pos = grid_to_screen(col, row, 0.0);
                        camera.shake_at(vec2(pos.x, pos.y), 5.0, 0.5);
                    }
                }

                // Speed slider
                if let Some(new_idx) = ui::influence_ui::draw_speed_slider(speed_idx) {
                    speed_idx = new_idx;
                }

                // Minimap
                if let Some((col, row)) = ui::minimap::draw_minimap(&grid, &camera) {
                    let pos = grid_to_screen(col, row, 0.0);
                    camera.pan_to(vec2(pos.x, pos.y));
                }
            }
        }

        next_frame().await;
    }
}

/// Find a random developed (non-empty, non-water, non-fire) cell for disaster spawning.
fn find_random_developed(grid: &grid::Grid, rng: &mut SmallRng) -> Option<(usize, usize)> {
    let mut candidates = Vec::new();
    for row in 0..grid.height {
        for col in 0..grid.width {
            let tile = grid.get(col, row).tile;
            if matches!(
                tile,
                TileType::Residential | TileType::Commercial | TileType::Industrial
            ) {
                candidates.push((col, row));
            }
        }
    }
    if candidates.is_empty() {
        return None;
    }
    Some(candidates[rng.gen_range(0..candidates.len())])
}

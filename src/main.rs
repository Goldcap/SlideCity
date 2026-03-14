mod audio;
mod config;
mod grid;
mod influence;
mod mayor;
mod renderer;
mod sim;
mod ui;

use audio::AudioManager;
use audio::mood;
use config::SimConfig;
use grid::terrain::generate_terrain;
use grid::TileType;
use influence::{ActionCategory, InfluenceState, MayorResponse};
use macroquad::prelude::*;
use mayor::Mayor;
use renderer::camera::GameCamera;
use renderer::iso::{grid_to_screen, TILE_H};
use renderer::lighting::DayNightCycle;
use renderer::particles::ParticleSystem;
use sim::stats::CityStats;
use ui::{GameState, InfluenceModal, StartPhase};
use ui::influence_ui::InfluenceAction;
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

    // Game state (initialized when game starts)
    let mut config = SimConfig::default();
    let mut rng = SmallRng::seed_from_u64(42);
    let mut grid = grid::Grid::new(1, 1);
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
    let mut influence_state = InfluenceState::new();
    let mut influence_modal = InfluenceModal::None;
    let mut last_phase_str = String::new();

    // Response toast state
    let mut response_toast: Option<MayorResponse> = None;
    let mut response_timer: f32 = 0.0;

    // Council vote state (kept between frames)
    let mut council_candidates: Option<[ActionCategory; 3]> = None;

    // LLM state
    let mut llm_request: Option<mayor::llm::LlmRequest> = None;
    let mut conversation_history = mayor::llm::ConversationHistory::default();
    let mut pending_audience_text = String::new();

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
                        if is_key_pressed(KeyCode::Escape) {
                            game_state = GameState::StartScreen(StartPhase::Title);
                        }
                    }
                    StartPhase::DifficultySelect => {
                        let confirmed = ui::start_screen::draw_difficulty_select(&mut start_screen, dt);
                        if is_key_pressed(KeyCode::Escape) {
                            game_state = GameState::StartScreen(StartPhase::MayorSelect);
                        } else if confirmed {
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
                            influence_state = InfluenceState::new();
                            influence_modal = InfluenceModal::None;
                            response_toast = None;
                            response_timer = 0.0;
                            last_phase_str = String::new();
                            council_candidates = None;
                            llm_request = None;
                            conversation_history = mayor::llm::ConversationHistory::default();
                            pending_audience_text.clear();

                            let initial_center = vec2(0.0, (config.grid_height as f32) * TILE_H / 2.0);
                            camera = GameCamera::new(initial_center);
                            day_night = DayNightCycle::new();
                            particles = ParticleSystem::new();

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
                let modal_open = !matches!(influence_modal, InfluenceModal::None);

                // --- Input (only when no modal) ---
                if !modal_open {
                    camera.handle_input(dt);

                    // Speed control: 1-4 keys
                    if is_key_pressed(KeyCode::Key1) { speed_idx = 0; }
                    if is_key_pressed(KeyCode::Key2) { speed_idx = 1; }
                    if is_key_pressed(KeyCode::Key3) { speed_idx = 2; }
                    if is_key_pressed(KeyCode::Key4) { speed_idx = 3; }
                }

                let speed = speed_levels[speed_idx];

                // --- Simulation tick (paused during modals) ---
                if !modal_open {
                    let tick_duration = config.base_tick_ms / 1000.0 / speed;
                    tick_timer += dt;
                    while tick_timer >= tick_duration {
                        tick_timer -= tick_duration;
                        tick_count += 1;

                        sim::tick(&mut grid, &mut next_grid, &config, &mut rng, &mut funds);

                        if tick_count.is_multiple_of(config.utility_recompute_interval) {
                            sim::utilities::recompute_utilities(&mut grid);
                        }

                        if tick_count.is_multiple_of(config.mayor_tick_interval) {
                            // Consume compliance boost before mayor decision
                            let _boosted = influence_state.consume_boost();

                            mayor.decide(&mut grid, &stats, &config, &mut funds, tick_count, &mut rng);

                            if let Some((x, y)) = mayor.camera_request.take() {
                                camera.pan_to(vec2(x, y));
                            }
                            if let Some((x, y)) = mayor.shake_request.take() {
                                camera.shake_at(vec2(x, y), 5.0, 0.5);
                            }
                        }

                        particles.spawn_from_grid(&grid, &mut rng);
                        stats = CityStats::compute(&grid);

                        if tick_count.is_multiple_of(config.audio_reeval_interval) {
                            let track = mood::select_track(&stats);
                            audio_mgr.transition_to(track);
                        }

                        if mayor.monument_built && !monument_sting_played {
                            monument_sting_played = true;
                            audio_mgr.play_sting(mood::TrackId::Monument);
                        }

                        // IP: yearly + milestones + phase transitions
                        let year = tick_count / config.ticks_per_year as u64 + 1;
                        influence_state.yearly_tick(year as u32);
                        influence_state.check_milestones(stats.population);

                        let phase_str = format!("{:?}", mayor.phase);
                        if phase_str != last_phase_str {
                            if !last_phase_str.is_empty() {
                                influence_state.phase_transition(&phase_str);
                            }
                            last_phase_str = phase_str;
                        }
                    }
                }

                // --- Update renderer state (every frame, even during modals) ---
                camera.update(dt);
                day_night.update(dt);
                particles.update(dt);
                audio_mgr.update(dt);
                influence_state.update(dt);

                // Poll LLM request
                if let Some(ref req) = llm_request {
                    if let Some(result) = req.try_recv() {
                        let yr = (tick_count / config.ticks_per_year as u64 + 1) as u32;
                        let ssn = mayor::narration::season_name(tick_count, config.ticks_per_season);
                        let p = mayor.personality();

                        let response_text = match result {
                            mayor::llm::LlmResult::Success(text) => text,
                            mayor::llm::LlmResult::Error(err) => {
                                // Fall back to scripted on error
                                let fallback = influence::audience::process_audience(
                                    &pending_audience_text, mayor.personality(), &mut rng,
                                );
                                format!("{} ({})", fallback.response, err)
                            }
                        };

                        influence_state.set_compliance_boost(rng.gen_range(2..=3));
                        mayor.log.push(yr, ssn, p.emoji, format!("[AUDIENCE] {}", response_text));
                        conversation_history.push(
                            pending_audience_text.clone(),
                            response_text.clone(),
                        );

                        influence_modal = InfluenceModal::Audience {
                            input: pending_audience_text.clone(),
                            response: Some(response_text),
                            waiting: false,
                        };

                        llm_request = None;
                        pending_audience_text.clear();
                    }
                }

                // Response toast timer
                if response_timer > 0.0 {
                    response_timer -= dt;
                    if response_timer <= 0.0 {
                        response_toast = None;
                    }
                }

                // --- Draw world ---
                set_camera(&camera.to_macroquad_camera());
                clear_background(Color::new(0.08, 0.10, 0.06, 1.0));
                renderer::draw_world(&grid, &camera, &day_night, &particles, tick_count);

                // --- UI (screen space) ---
                set_default_camera();

                let year = tick_count / config.ticks_per_year as u64 + 1;
                let season = mayor::narration::season_name(tick_count, config.ticks_per_season);
                let speed = speed_levels[speed_idx];

                // Top HUD
                ui::stats::draw_hud(
                    &stats, funds, year, season, speed,
                    &day_night, mayor.phase, &audio_mgr.current_mood_label,
                );

                // Right panel: mayor log
                let panel_x = ui::mayor_log::draw_mayor_panel(&mayor);

                // Influence panel + buttons
                let panel_action = ui::influence_ui::draw_influence_panel(
                    &influence_state, &influence_modal, panel_x,
                );

                // Speed slider
                if let Some(new_idx) = ui::influence_ui::draw_speed_slider(speed_idx) {
                    speed_idx = new_idx;
                }

                // Minimap
                if !modal_open {
                    if let Some((col, row)) = ui::minimap::draw_minimap(&grid, &camera) {
                        let pos = grid_to_screen(col, row, 0.0);
                        camera.pan_to(vec2(pos.x, pos.y));
                    }
                }

                // Response toast
                if let Some(ref resp) = response_toast {
                    ui::influence_ui::draw_response_toast(resp, response_timer);
                }

                // --- Handle panel actions (open modals, disaster button) ---
                if !modal_open {
                    match panel_action {
                        InfluenceAction::DisasterButton => {
                            if let Some((col, row)) = find_random_developed(&grid, &mut rng) {
                                grid.get_mut(col, row).tile = TileType::Fire;
                                grid.get_mut(col, row).age = 0;
                                influence_state.disaster_triggered();
                                influence_state.disaster_cooldown = config.disaster_cooldown_secs;
                                let pos = grid_to_screen(col, row, 0.0);
                                camera.shake_at(vec2(pos.x, pos.y), 5.0, 0.5);
                            }
                        }
                        InfluenceAction::OpenSuggestion => {
                            if influence_state.can_afford(1) {
                                influence_modal = InfluenceModal::SuggestionBox;
                            }
                        }
                        InfluenceAction::OpenCouncil => {
                            if influence_state.can_afford(3) {
                                let candidates = influence::council::generate_candidates(
                                    mayor.personality(), &stats, &mut rng,
                                );
                                council_candidates = Some(candidates);
                                influence_modal = InfluenceModal::CouncilVote { candidates };
                            }
                        }
                        InfluenceAction::OpenAudience => {
                            if influence_state.can_afford(5) {
                                influence_modal = InfluenceModal::Audience {
                                    input: String::new(),
                                    response: None,
                                    waiting: false,
                                };
                            }
                        }
                        InfluenceAction::OpenBuyIP => {
                            influence_modal = InfluenceModal::BuyIP;
                        }
                        _ => {}
                    }
                }

                // --- Draw and handle modal ---
                if modal_open {
                    let modal_action = ui::influence_ui::draw_modal(&mut influence_modal);

                    match modal_action {
                        InfluenceAction::CloseModal => {
                            influence_modal = InfluenceModal::None;
                        }
                        InfluenceAction::SuggestionPick(action) => {
                            if influence_state.spend(1) {
                                let has_boost = influence_state.compliance_boost > 0;
                                let response = influence::suggestion::evaluate_suggestion(
                                    action, mayor.personality(), has_boost, &mut rng,
                                );

                                // If the mayor complied, execute the action
                                if let Some(exec_action) = response.action() {
                                    execute_influence_action(
                                        exec_action, &mut grid, &mut funds,
                                        &mayor, &mut camera, &mut rng,
                                    );
                                }

                                // Log the response
                                let p = mayor.personality();
                                let yr = (tick_count / config.ticks_per_year as u64 + 1) as u32;
                                let ssn = mayor::narration::season_name(tick_count, config.ticks_per_season);
                                mayor.log.push(yr, ssn, p.emoji, format!("[SUGGEST] {}", response.text()));

                                show_response(&mut response_toast, &mut response_timer, response);
                                influence_modal = InfluenceModal::None;
                            }
                        }
                        InfluenceAction::CouncilPick(idx) => {
                            if let Some(candidates) = &council_candidates {
                                if influence_state.spend(3) {
                                    let choice = candidates[idx];
                                    let has_boost = influence_state.compliance_boost > 0;
                                    let response = influence::council::execute_vote(
                                        choice, candidates, mayor.personality(), has_boost, &mut rng,
                                    );

                                    // Execute the resulting action
                                    if let Some(exec_action) = response.action() {
                                        execute_influence_action(
                                            exec_action, &mut grid, &mut funds,
                                            &mayor, &mut camera, &mut rng,
                                        );
                                    }

                                    let p = mayor.personality();
                                    let yr = (tick_count / config.ticks_per_year as u64 + 1) as u32;
                                    let ssn = mayor::narration::season_name(tick_count, config.ticks_per_season);
                                    mayor.log.push(yr, ssn, p.emoji, format!("[COUNCIL] {}", response.text()));

                                    show_response(&mut response_toast, &mut response_timer, response);
                                    influence_modal = InfluenceModal::None;
                                    council_candidates = None;
                                }
                            }
                        }
                        InfluenceAction::AudienceSubmit(text) => {
                            if influence_state.spend(5) {
                                // Try LLM first, fall back to scripted
                                let yr = (tick_count / config.ticks_per_year as u64 + 1) as u32;
                                let recent_log: Vec<_> = mayor.log.last_n(5)
                                    .into_iter().cloned().collect();

                                if let Some(req) = mayor::llm::send_audience_request(
                                    text.clone(),
                                    mayor.personality(),
                                    &stats,
                                    recent_log,
                                    &conversation_history,
                                    funds,
                                    yr,
                                ) {
                                    // LLM request sent — show waiting state
                                    llm_request = Some(req);
                                    pending_audience_text = text.clone();
                                    influence_modal = InfluenceModal::Audience {
                                        input: text,
                                        response: None,
                                        waiting: true,
                                    };
                                } else {
                                    // No API key — use scripted fallback
                                    let result = influence::audience::process_audience(
                                        &text, mayor.personality(), &mut rng,
                                    );
                                    influence_state.set_compliance_boost(result.compliance_boost);

                                    let p = mayor.personality();
                                    let ssn = mayor::narration::season_name(tick_count, config.ticks_per_season);
                                    mayor.log.push(yr, ssn, p.emoji, format!("[AUDIENCE] {}", result.response));

                                    conversation_history.push(text.clone(), result.response.clone());
                                    influence_modal = InfluenceModal::Audience {
                                        input: text,
                                        response: Some(result.response),
                                        waiting: false,
                                    };
                                }
                            }
                        }
                        InfluenceAction::BuyIPConfirm => {
                            if influence_state.buy_ip(&mut funds) {
                                let p = mayor.personality();
                                let yr = (tick_count / config.ticks_per_year as u64 + 1) as u32;
                                let ssn = mayor::narration::season_name(tick_count, config.ticks_per_season);
                                mayor.log.push(yr, ssn, p.emoji,
                                    "The treasury takes a hit... someone bought influence.".to_string());
                            }
                            influence_modal = InfluenceModal::None;
                        }
                        _ => {}
                    }
                }
            }
        }

        next_frame().await;
    }
}

/// Execute a player-influenced action on the grid.
fn execute_influence_action(
    action: ActionCategory,
    grid: &mut grid::Grid,
    funds: &mut i64,
    mayor: &Mayor,
    camera: &mut GameCamera,
    rng: &mut SmallRng,
) {
    let p = mayor.personality();

    match action {
        ActionCategory::BuildPark => {
            if let Some((col, row)) = find_empty_near_development(grid, rng) {
                let size = rng.gen_range(4..=10);
                let placed = sim::growth::grow_blob(grid, col, row, TileType::Park, size, rng);
                if placed > 0 {
                    *funds -= p.modify_cost(300, false, true, false);
                    pan_to(camera, col, row);
                }
            }
        }
        ActionCategory::ZoneResidential => {
            if let Some((col, row)) = find_empty_near_road(grid, rng) {
                let size = rng.gen_range(8..=20);
                let placed = sim::growth::grow_blob(grid, col, row, TileType::Residential, size, rng);
                if placed > 0 {
                    *funds -= p.modify_cost(200, false, false, false);
                    pan_to(camera, col, row);
                }
            }
        }
        ActionCategory::ZoneCommercial => {
            if let Some((col, row)) = find_empty_near_road(grid, rng) {
                let size = rng.gen_range(4..=12);
                let placed = sim::growth::grow_blob(grid, col, row, TileType::Commercial, size, rng);
                if placed > 0 {
                    *funds -= p.modify_cost(500, false, false, false);
                    pan_to(camera, col, row);
                }
            }
        }
        ActionCategory::ZoneIndustrial => {
            if let Some((col, row)) = find_empty_near_road(grid, rng) {
                let size = rng.gen_range(10..=24);
                let placed = sim::growth::grow_blob(grid, col, row, TileType::Industrial, size, rng);
                if placed > 0 {
                    *funds -= p.modify_cost(800, false, false, false);
                    pan_to(camera, col, row);
                }
            }
        }
        ActionCategory::ExtendPower => {
            extend_utility_player(grid, funds, true, p, rng, camera);
        }
        ActionCategory::ExtendWater => {
            extend_utility_player(grid, funds, false, p, rng, camera);
        }
        ActionCategory::BuildRoads => {
            if let Some((col, row)) = find_road_endpoint(grid, rng) {
                let road_cost = p.modify_cost(50, true, false, false);
                let dirs: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
                let (dc, dr) = dirs[rng.gen_range(0..4)];
                let mut c = col;
                let mut r = row;
                for _ in 0..rng.gen_range(3..8) {
                    let nc = (c as i32 + dc) as usize;
                    let nr = (r as i32 + dr) as usize;
                    if grid.in_bounds(nc, nr) && grid.get(nc, nr).tile == TileType::Empty {
                        grid.get_mut(nc, nr).tile = TileType::Road;
                        *funds -= road_cost;
                        c = nc;
                        r = nr;
                    } else {
                        break;
                    }
                }
                pan_to(camera, col, row);
            }
        }
    }
}

fn extend_utility_player(
    grid: &mut grid::Grid,
    funds: &mut i64,
    is_power: bool,
    p: &mayor::personality::MayorPersonality,
    _rng: &mut SmallRng,
    camera: &mut GameCamera,
) {
    // Find an unserviced zone and place utility lines toward it
    let line_type = if is_power { TileType::PowerLine } else { TileType::WaterMain };
    let cost_per = if is_power {
        p.modify_cost(100, false, false, false)
    } else {
        p.modify_cost(80, false, false, false)
    };

    for row in 0..grid.height {
        for col in 0..grid.width {
            let cell = grid.get(col, row);
            let is_zone = matches!(cell.tile, TileType::Residential | TileType::Commercial | TileType::Industrial);
            let needs_service = if is_power { !cell.has_power } else { !cell.has_water };
            if is_zone && needs_service {
                // Place a short line toward this zone
                let cx = grid.width / 2;
                let direction: i32 = if col > cx { 1 } else { -1 };
                let mut placed = 0;
                let mut c = col;
                for _ in 0..6 {
                    let nc = (c as i32 + direction) as usize;
                    if grid.in_bounds(nc, row) && grid.get(nc, row).tile == TileType::Empty {
                        grid.get_mut(nc, row).tile = line_type;
                        *funds -= cost_per;
                        placed += 1;
                        c = nc;
                    } else {
                        break;
                    }
                }
                if placed > 0 {
                    pan_to(camera, col, row);
                }
                return;
            }
        }
    }
}

fn pan_to(camera: &mut GameCamera, col: usize, row: usize) {
    let pos = grid_to_screen(col, row, 0.0);
    camera.pan_to(vec2(pos.x, pos.y));
}

fn show_response(
    toast: &mut Option<MayorResponse>,
    timer: &mut f32,
    response: MayorResponse,
) {
    *toast = Some(response);
    *timer = 4.0;
}

fn find_random_developed(grid: &grid::Grid, rng: &mut SmallRng) -> Option<(usize, usize)> {
    let mut candidates = Vec::new();
    for row in 0..grid.height {
        for col in 0..grid.width {
            let tile = grid.get(col, row).tile;
            if matches!(tile, TileType::Residential | TileType::Commercial | TileType::Industrial) {
                candidates.push((col, row));
            }
        }
    }
    if candidates.is_empty() { return None; }
    Some(candidates[rng.gen_range(0..candidates.len())])
}

fn find_empty_near_road(grid: &grid::Grid, rng: &mut SmallRng) -> Option<(usize, usize)> {
    let mut candidates = Vec::new();
    for row in 0..grid.height {
        for col in 0..grid.width {
            if grid.get(col, row).tile == TileType::Empty
                && grid.get(col, row).terrain_height < 0.8
                && grid.has_road_neighbor(col, row)
            {
                candidates.push((col, row));
            }
        }
    }
    if candidates.is_empty() { return None; }
    Some(candidates[rng.gen_range(0..candidates.len())])
}

fn find_empty_near_development(grid: &grid::Grid, rng: &mut SmallRng) -> Option<(usize, usize)> {
    let mut candidates = Vec::new();
    for row in 0..grid.height {
        for col in 0..grid.width {
            if grid.get(col, row).tile == TileType::Empty
                && grid.count_developed(col, row, 3) > 2
            {
                candidates.push((col, row));
            }
        }
    }
    if candidates.is_empty() { return None; }
    Some(candidates[rng.gen_range(0..candidates.len())])
}

fn find_road_endpoint(grid: &grid::Grid, rng: &mut SmallRng) -> Option<(usize, usize)> {
    let mut candidates = Vec::new();
    for row in 0..grid.height {
        for col in 0..grid.width {
            if grid.get(col, row).tile == TileType::Road {
                // Check if any adjacent cell is empty (road endpoint)
                let dirs: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
                for (dc, dr) in dirs {
                    let nc = (col as i32 + dc) as usize;
                    let nr = (row as i32 + dr) as usize;
                    if grid.in_bounds(nc, nr) && grid.get(nc, nr).tile == TileType::Empty {
                        candidates.push((col, row));
                        break;
                    }
                }
            }
        }
    }
    if candidates.is_empty() { return None; }
    Some(candidates[rng.gen_range(0..candidates.len())])
}

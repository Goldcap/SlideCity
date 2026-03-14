#![allow(clippy::too_many_arguments)]

pub mod llm;
pub mod narration;
pub mod personality;

use ::rand::rngs::SmallRng;
use ::rand::Rng;
use macroquad::prelude::Vec2;
use serde::{Deserialize, Serialize};

use crate::config::SimConfig;
use crate::grid::{Grid, TileType};
use crate::renderer::iso::grid_to_screen;
use crate::sim::growth;
use crate::sim::stats::CityStats;
use narration::{season_name, MayorLog, NarrationContext};
use personality::{MayorPersonality, ARCHETYPES};

/// Mayor phase state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MayorPhase {
    Founding,  // year 1-3
    Growth,    // year 3-10
    Maturity,  // year 10-25
    Evolution, // year 25+
}

/// Camera action request from the mayor.
#[derive(Clone, Debug)]
pub enum CameraRequest {
    PanTo(Vec2),
    ShakeAt(Vec2),
    None,
}

/// The Virtual Mayor — THE SOUL of the game.
#[derive(Clone, Serialize, Deserialize)]
pub struct Mayor {
    pub personality_idx: usize,
    pub phase: MayorPhase,
    pub log: MayorLog,
    pub founding_step: u8,
    pub monument_built: bool,
    pub mayor_number: u32, // How many mayors have served (for retirement)
    #[serde(skip)]
    pub camera_request: Option<(f32, f32)>, // world pos to pan to
    #[serde(skip)]
    pub shake_request: Option<(f32, f32)>,
}

impl Mayor {
    pub fn new(personality_idx: usize) -> Self {
        Self {
            personality_idx,
            phase: MayorPhase::Founding,
            log: MayorLog::new(),
            founding_step: 0,
            monument_built: false,
            mayor_number: 1,
            camera_request: None,
            shake_request: None,
        }
    }

    pub fn personality(&self) -> &'static MayorPersonality {
        &ARCHETYPES[self.personality_idx % ARCHETYPES.len()]
    }

    /// Run the mayor's decision loop. Called every mayor_tick_interval sim ticks.
    pub fn decide(
        &mut self,
        grid: &mut Grid,
        stats: &CityStats,
        config: &SimConfig,
        funds: &mut i64,
        tick_count: u64,
        rng: &mut SmallRng,
    ) {
        let year = (tick_count / config.ticks_per_year as u64) as u32 + 1;
        let season = season_name(tick_count, config.ticks_per_season);
        let p = self.personality();
        let emoji = p.emoji;

        // Clear camera requests
        self.camera_request = None;
        self.shake_request = None;

        // Update phase
        self.update_phase(year, stats);

        // CRITICAL: Fire response (always)
        if stats.fire_count > 0 {
            let text = narration::narrate(
                if stats.fire_count > 3 {
                    NarrationContext::FirePanic
                } else {
                    NarrationContext::FireWatching
                },
                rng,
            );
            self.log.push(year, season, emoji, text.to_string());

            // Find a fire cell and shake camera there
            if let Some((col, row)) = find_tile(grid, TileType::Fire) {
                let pos = grid_to_screen(col, row, 0.0);
                self.shake_request = Some((pos.x, pos.y));
            }
            return; // Don't do anything else during fire
        }

        // Phase-specific behavior
        match self.phase {
            MayorPhase::Founding => self.founding_tick(grid, stats, config, funds, year, season, rng),
            MayorPhase::Growth => self.growth_tick(grid, stats, config, funds, year, season, rng),
            MayorPhase::Maturity => self.maturity_tick(grid, stats, config, funds, year, season, rng),
            MayorPhase::Evolution => self.evolution_tick(grid, stats, config, funds, year, season, rng),
        }
    }

    fn update_phase(&mut self, year: u32, stats: &CityStats) {
        let relative_year = year; // TODO: track per-mayor year for successors
        self.phase = if relative_year <= 3 {
            MayorPhase::Founding
        } else if relative_year <= 10 {
            MayorPhase::Growth
        } else if relative_year <= 25 {
            MayorPhase::Maturity
        } else {
            MayorPhase::Evolution
        };

        // Override for successor mayors
        if self.mayor_number > 1 {
            if stats.population < 200 {
                self.phase = MayorPhase::Growth;
            } else if stats.population < 500 {
                self.phase = MayorPhase::Maturity;
            } else {
                self.phase = MayorPhase::Evolution;
            }
        }
    }

    // ===== FOUNDING PHASE =====

    fn founding_tick(
        &mut self, grid: &mut Grid, _stats: &CityStats, _config: &SimConfig,
        funds: &mut i64, year: u32, season: &'static str, rng: &mut SmallRng,
    ) {
        let p = self.personality();
        let emoji = p.emoji;
        let cx = grid.width / 2;
        let cy = grid.height / 2;

        match self.founding_step {
            0 => {
                // Place road spine
                let road_cost = p.modify_cost(50, true, false, false);
                for col in (cx.saturating_sub(15))..=(cx + 15).min(grid.width - 1) {
                    if grid.in_bounds(col, cy) && grid.get(col, cy).tile == TileType::Empty {
                        grid.get_mut(col, cy).tile = TileType::Road;
                        *funds -= road_cost;
                    }
                }
                for row in (cy.saturating_sub(10))..=(cy + 10).min(grid.height - 1) {
                    if grid.in_bounds(cx, row) && grid.get(cx, row).tile == TileType::Empty {
                        grid.get_mut(cx, row).tile = TileType::Road;
                        *funds -= road_cost;
                    }
                }
                self.log.push(year, season, emoji, narration::narrate(NarrationContext::RoadSpine, rng).to_string());
                self.pan_to_grid(cx, cy);
            }
            1 => {
                // First residential blob
                let size = rng.gen_range(12..=20);
                let placed = growth::grow_blob(grid, cx + 2, cy + 2, TileType::Residential, size, rng);
                if placed > 0 {
                    *funds -= p.modify_cost(200, false, false, false);
                    self.log.push(year, season, emoji, narration::narrate(NarrationContext::FirstResidential, rng).to_string());
                    self.pan_to_grid(cx + 2, cy + 2);
                }
            }
            2 => {
                // Power plant
                let pp_col = cx.saturating_sub(18);
                if grid.in_bounds(pp_col, cy) {
                    grid.get_mut(pp_col, cy).tile = TileType::PowerPlant;
                    grid.get_mut(pp_col, cy).age = 0;
                    *funds -= p.modify_cost(5000, false, false, false);
                    self.log.push(year, season, emoji, narration::narrate(NarrationContext::PowerPlantBuilt, rng).to_string());
                    self.pan_to_grid(pp_col, cy);
                }
            }
            3 => {
                // Power line toward residential
                let pp_col = cx.saturating_sub(18);
                let line_cost = p.modify_cost(100, false, false, false);
                for col in (pp_col + 1)..cx.saturating_sub(2) {
                    if grid.in_bounds(col, cy) && grid.get(col, cy).tile == TileType::Empty {
                        grid.get_mut(col, cy).tile = TileType::PowerLine;
                        *funds -= line_cost;
                    }
                }
                self.log.push(year, season, emoji, narration::narrate(NarrationContext::PowerLineRun, rng).to_string());
            }
            4 => {
                // Water tower
                let wt_col = (cx + 18).min(grid.width - 1);
                if grid.in_bounds(wt_col, cy) {
                    grid.get_mut(wt_col, cy).tile = TileType::WaterTower;
                    grid.get_mut(wt_col, cy).age = 0;
                    *funds -= p.modify_cost(4000, false, false, false);
                    self.log.push(year, season, emoji, narration::narrate(NarrationContext::WaterTowerBuilt, rng).to_string());
                    self.pan_to_grid(wt_col, cy);
                }
            }
            5 => {
                // Water main toward residential
                let wt_col = (cx + 18).min(grid.width - 1);
                let main_cost = p.modify_cost(80, false, false, false);
                for col in ((cx + 2)..wt_col).rev() {
                    if grid.in_bounds(col, cy) && grid.get(col, cy).tile == TileType::Empty {
                        grid.get_mut(col, cy).tile = TileType::WaterMain;
                        *funds -= main_cost;
                    }
                }
                self.log.push(year, season, emoji, narration::narrate(NarrationContext::WaterMainRun, rng).to_string());
            }
            _ => {
                // Founding done, stay here until phase transition
            }
        }
        self.founding_step = self.founding_step.saturating_add(1);
    }

    // ===== GROWTH PHASE =====

    fn growth_tick(
        &mut self, grid: &mut Grid, stats: &CityStats, _config: &SimConfig,
        funds: &mut i64, year: u32, season: &'static str, rng: &mut SmallRng,
    ) {
        let p = self.personality();
        let emoji = p.emoji;

        // Check growth rate modifier
        if rng.gen::<f32>() > p.growth_rate_modifier() {
            return; // Skip this tick (slow growth/expansion weakness)
        }

        // Fund check
        if *funds < 3000 {
            self.log.push(year, season, emoji, narration::narrate(NarrationContext::FundsTight, rng).to_string());
            return;
        }

        // CRITICAL: utility coverage
        if stats.power_coverage < 0.50 {
            self.extend_utility(grid, funds, true, year, season, rng);
            return;
        }
        if stats.water_coverage < 0.50 {
            self.extend_utility(grid, funds, false, year, season, rng);
            return;
        }

        // HIGH: residential demand
        let target_res = (20.0 + stats.population as f32 * 0.3 * p.growth_aggression) as u32;
        if stats.res_count < target_res && *funds > 10000 {
            if let Some((col, row)) = find_empty_near_road(grid, rng) {
                let size = rng.gen_range(8..=20);
                let placed = growth::grow_blob(grid, col, row, TileType::Residential, size, rng);
                if placed > 0 {
                    *funds -= p.modify_cost(200, false, false, false);
                    self.log.push(year, season, emoji, narration::narrate(NarrationContext::ResidentialPlaced, rng).to_string());
                    self.pan_to_grid(col, row);
                }
            }
            return;
        }

        // HIGH: commercial demand
        let r_to_c = if stats.com_count > 0 {
            stats.res_count as f32 / stats.com_count as f32
        } else {
            999.0
        };
        if r_to_c > 5.0 && *funds > 10000 {
            if let Some((col, row)) = find_empty_near_road(grid, rng) {
                let size = rng.gen_range(4..=12);
                let placed = growth::grow_blob(grid, col, row, TileType::Commercial, size, rng);
                if placed > 0 {
                    *funds -= p.modify_cost(500, false, false, false);
                    self.log.push(year, season, emoji, narration::narrate(NarrationContext::CommercialPlaced, rng).to_string());
                    self.pan_to_grid(col, row);
                }
            }
            return;
        }

        // HIGH: industrial (based on personality)
        if p.industrial_bias > 0.4 && stats.ind_count < stats.res_count / 4 && *funds > 10000 {
            if let Some((col, row)) = find_empty_near_road(grid, rng) {
                let size = rng.gen_range(10..=24);
                let placed = growth::grow_blob(grid, col, row, TileType::Industrial, size, rng);
                if placed > 0 {
                    *funds -= p.modify_cost(800, false, false, false);
                    self.log.push(year, season, emoji, narration::narrate(NarrationContext::IndustrialPlaced, rng).to_string());
                    self.pan_to_grid(col, row);
                }
            }
            return;
        }

        // MEDIUM: happiness - park
        if stats.happiness < 0.65 && p.green_affinity > 0.3 {
            if let Some((col, row)) = find_empty_near_development(grid, rng) {
                let size = rng.gen_range(4..=10);
                let placed = growth::grow_blob(grid, col, row, TileType::Park, size, rng);
                if placed > 0 {
                    let cost = p.modify_cost(300, false, true, false);
                    *funds -= cost;
                    self.log.push(year, season, emoji, narration::narrate(NarrationContext::ParkPlaced, rng).to_string());
                    self.pan_to_grid(col, row);
                }
            }
            return;
        }

        // MEDIUM: extend road network
        if *funds > 20000 && rng.gen::<f32>() < 0.3 {
            self.extend_roads(grid, funds, year, season, rng);
        }
    }

    // ===== MATURITY PHASE =====

    fn maturity_tick(
        &mut self, grid: &mut Grid, stats: &CityStats, config: &SimConfig,
        funds: &mut i64, year: u32, season: &'static str, rng: &mut SmallRng,
    ) {
        let p = self.personality();
        let emoji = p.emoji;

        // Monument!
        if stats.population > 500 && !self.monument_built {
            if let Some((col, row)) = find_empty_near_center(grid) {
                grid.get_mut(col, row).tile = TileType::Monument;
                grid.get_mut(col, row).age = 0;
                let cost = p.modify_cost(8000, false, false, true);
                *funds -= cost;
                self.monument_built = true;
                self.log.push(year, season, emoji, narration::narrate(NarrationContext::MonumentApproved, rng).to_string());
                self.pan_to_grid(col, row);
                return;
            }
        }

        // Otherwise run growth logic with maturity adjustments
        // CRITICAL: utilities first
        if stats.power_coverage < 0.60 {
            self.extend_utility(grid, funds, true, year, season, rng);
            return;
        }
        if stats.water_coverage < 0.60 {
            self.extend_utility(grid, funds, false, year, season, rng);
            return;
        }

        // Density management - parks
        if stats.happiness < 0.55 && p.green_affinity > 0.2 {
            if let Some((col, row)) = find_empty_near_development(grid, rng) {
                let size = rng.gen_range(4..=10);
                let placed = growth::grow_blob(grid, col, row, TileType::Park, size, rng);
                if placed > 0 {
                    *funds -= p.modify_cost(300, false, true, false);
                    self.log.push(year, season, emoji, narration::narrate(NarrationContext::DensityManagement, rng).to_string());
                    self.pan_to_grid(col, row);
                }
            }
            return;
        }

        // Continue growth at reduced pace
        if rng.gen::<f32>() < 0.5 {
            self.growth_tick(grid, stats, config, funds, year, season, rng);
        } else if rng.gen::<f32>() < 0.1 {
            // Occasional proud narration
            self.log.push(year, season, emoji, narration::narrate(NarrationContext::CityAlive, rng).to_string());
        }
    }

    // ===== EVOLUTION PHASE =====

    fn evolution_tick(
        &mut self, grid: &mut Grid, stats: &CityStats, config: &SimConfig,
        funds: &mut i64, year: u32, season: &'static str, rng: &mut SmallRng,
    ) {
        let p = self.personality();
        let emoji = p.emoji;

        // Mayor retirement at year 30-40
        if year >= 30 + self.mayor_number * 10 && rng.gen::<f32>() < 0.05 {
            self.log.push(year, season, emoji, narration::narrate(NarrationContext::Retirement, rng).to_string());

            // Roll new mayor
            let new_idx = rng.gen_range(0..ARCHETYPES.len());
            self.personality_idx = new_idx;
            self.mayor_number += 1;
            self.founding_step = 255; // Skip founding for successors

            let new_p = self.personality();
            self.log.push(year, season, new_p.emoji,
                format!("New mayor: {} {}. Ready to lead.", new_p.emoji, new_p.name));
            return;
        }

        // Run maturity logic
        self.maturity_tick(grid, stats, config, funds, year, season, rng);
    }

    // ===== HELPER METHODS =====

    fn extend_utility(
        &mut self, grid: &mut Grid, funds: &mut i64, is_power: bool,
        year: u32, season: &'static str, rng: &mut SmallRng,
    ) {
        let p = self.personality();
        let emoji = p.emoji;

        // Find an unpowered/unwatered zone cell and trace toward it
        let target = find_unserviced_zone(grid, is_power);
        if let Some((col, row)) = target {
            let line_type = if is_power { TileType::PowerLine } else { TileType::WaterMain };
            let cost_per = if is_power {
                p.modify_cost(100, false, false, false)
            } else {
                p.modify_cost(80, false, false, false)
            };

            // Place a short line segment toward the target
            let cx = grid.width / 2;
            let direction: i32 = if col > cx { 1 } else { -1 };
            let mut placed = 0;
            let mut c = col;
            for _ in 0..8 {
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
                let ctx = if is_power {
                    NarrationContext::PowerExtended
                } else {
                    NarrationContext::WaterExtended
                };
                self.log.push(year, season, emoji, narration::narrate(ctx, rng).to_string());
                self.pan_to_grid(col, row);
            }
        } else if *funds > 8000 {
            // No unserviced zones but coverage still low — build new source
            if let Some((col, row)) = find_empty_near_development(grid, rng) {
                let source_type = if is_power { TileType::PowerPlant } else { TileType::WaterTower };
                let cost = if is_power {
                    p.modify_cost(5000, false, false, false)
                } else {
                    p.modify_cost(4000, false, false, false)
                };
                grid.get_mut(col, row).tile = source_type;
                grid.get_mut(col, row).age = 0;
                *funds -= cost;

                let ctx = if is_power {
                    NarrationContext::PowerPlantBuilt
                } else {
                    NarrationContext::WaterTowerBuilt
                };
                self.log.push(year, season, emoji, narration::narrate(ctx, rng).to_string());
                self.pan_to_grid(col, row);
            }
        }
    }

    fn extend_roads(
        &mut self, grid: &mut Grid, funds: &mut i64,
        year: u32, season: &'static str, rng: &mut SmallRng,
    ) {
        let p = self.personality();
        let emoji = p.emoji;
        let road_cost = p.modify_cost(50, true, false, false);

        // Find end of existing road and extend it
        for _ in 0..20 {
            let col = rng.gen_range(1..grid.width - 1);
            let row = rng.gen_range(1..grid.height - 1);
            if grid.get(col, row).tile == TileType::Road {
                // Try to extend in a random direction
                let dirs: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
                let (dc, dr) = dirs[rng.gen_range(0..4)];
                let nc = (col as i32 + dc) as usize;
                let nr = (row as i32 + dr) as usize;
                if grid.in_bounds(nc, nr) && grid.get(nc, nr).tile == TileType::Empty {
                    grid.get_mut(nc, nr).tile = TileType::Road;
                    *funds -= road_cost;
                    // Extend a few more in the same direction
                    let mut c = nc;
                    let mut r = nr;
                    for _ in 0..rng.gen_range(3..8) {
                        let nc2 = (c as i32 + dc) as usize;
                        let nr2 = (r as i32 + dr) as usize;
                        if grid.in_bounds(nc2, nr2) && grid.get(nc2, nr2).tile == TileType::Empty {
                            grid.get_mut(nc2, nr2).tile = TileType::Road;
                            *funds -= road_cost;
                            c = nc2;
                            r = nr2;
                        } else {
                            break;
                        }
                    }
                    self.log.push(year, season, emoji, narration::narrate(NarrationContext::RoadExpansion, rng).to_string());
                    self.pan_to_grid(nc, nr);
                    return;
                }
            }
        }
    }

    fn pan_to_grid(&mut self, col: usize, row: usize) {
        let pos = grid_to_screen(col, row, 0.0);
        self.camera_request = Some((pos.x, pos.y));
    }
}

// ===== GRID SEARCH HELPERS =====

fn find_tile(grid: &Grid, tile: TileType) -> Option<(usize, usize)> {
    for row in 0..grid.height {
        for col in 0..grid.width {
            if grid.get(col, row).tile == tile {
                return Some((col, row));
            }
        }
    }
    None
}

fn find_empty_near_road(grid: &Grid, rng: &mut SmallRng) -> Option<(usize, usize)> {
    let mut candidates = Vec::new();
    for row in 0..grid.height {
        for col in 0..grid.width {
            if grid.get(col, row).tile == TileType::Empty
                && grid.get(col, row).terrain_height < 0.8 // Not on steep terrain
                && grid.has_road_neighbor(col, row)
            {
                candidates.push((col, row));
            }
        }
    }
    if candidates.is_empty() {
        return None;
    }
    Some(candidates[rng.gen_range(0..candidates.len())])
}

fn find_empty_near_development(grid: &Grid, rng: &mut SmallRng) -> Option<(usize, usize)> {
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
    if candidates.is_empty() {
        return None;
    }
    Some(candidates[rng.gen_range(0..candidates.len())])
}

fn find_empty_near_center(grid: &Grid) -> Option<(usize, usize)> {
    let cx = grid.width / 2;
    let cy = grid.height / 2;

    // Spiral outward from center
    for radius in 0..20 {
        for dr in -(radius as i32)..=(radius as i32) {
            for dc in -(radius as i32)..=(radius as i32) {
                if dr.abs() + dc.abs() != radius as i32 {
                    continue;
                }
                let col = (cx as i32 + dc) as usize;
                let row = (cy as i32 + dr) as usize;
                if grid.in_bounds(col, row) && grid.get(col, row).tile == TileType::Empty {
                    return Some((col, row));
                }
            }
        }
    }
    None
}

fn find_unserviced_zone(grid: &Grid, is_power: bool) -> Option<(usize, usize)> {
    for row in 0..grid.height {
        for col in 0..grid.width {
            let cell = grid.get(col, row);
            let is_zone = matches!(
                cell.tile,
                TileType::Residential | TileType::Commercial | TileType::Industrial
            );
            if is_zone {
                if is_power && !cell.has_power {
                    return Some((col, row));
                }
                if !is_power && !cell.has_water {
                    return Some((col, row));
                }
            }
        }
    }
    None
}

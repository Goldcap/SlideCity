---
title: "feat: SC4-Style RCI Demand Economics (Phase 1)"
type: feat
status: active
date: 2026-03-17
origin: docs/brainstorms/2026-03-17-rci-demand-economics-brainstorm.md
---

# feat: SC4-Style RCI Demand Economics (Phase 1)

## Overview

Replace SlideCity's random age-based growth/decay simulation with SimCity 4-style demand-driven economics. Buildings grow because there's demand for them and abandon because economic conditions deteriorate. This is Phase 1: core RCI demand loop with 3 zone types, basic desirability, zone-then-develop architecture, and demand-driven abandonment.

(see brainstorm: docs/brainstorms/2026-03-17-rci-demand-economics-brainstorm.md)

## Problem Statement

The current simulation uses cellular automaton rules with random probability checks. Buildings appear when enough neighbors exist and vanish based on random chance + age thresholds. This creates a city that feels arbitrary — zones appear and disappear for no understandable reason. Players (and the mayor AI) cannot form a mental model of WHY the city is growing or declining.

SC4's genius was the economic feedback loop: people move in because there are jobs, shops open because there are customers, factories open because there are workers. Each drives the other, creating organic city growth that responds to player (or mayor) decisions.

## Technical Approach

### Architecture

**Files modified:**
- `src/grid/mod.rs` — Add ZonedR/ZonedC/ZonedI to TileType, add building_stage/desirability to Cell
- `src/sim/mod.rs` — Add demand computation step to tick loop
- `src/sim/automaton.rs` — Replace random rules with demand-gated growth + abandonment
- `src/sim/stats.rs` — Add demand tracking, job counts, employment
- `src/sim/growth.rs` — Growth now reads demand + desirability (spatial targeting changes)
- `src/mayor/mod.rs` — Mayor reads RCI demand graph, zones accordingly, bulldozes
- `src/config.rs` — Add demand/desirability tuning parameters

**New files:**
- `src/sim/demand.rs` — RCI demand calculation engine
- `src/sim/desirability.rs` — Per-cell desirability grid computation

**Files unchanged:**
- `src/sim/utilities.rs` — Power/water networks stay as-is
- `src/main.rs` — Rendering systems unchanged (dirty-cell tracking already handles visual updates)

### Implementation Phases

#### Step 1: Data Model Changes

**Add new TileType variants** in `src/grid/mod.rs`:
```rust
pub enum TileType {
    Empty,
    ZonedResidential,  // NEW: zoned but no building yet
    ZonedCommercial,   // NEW
    ZonedIndustrial,   // NEW
    Residential,       // has building
    Commercial,        // has building
    Industrial,        // has building
    Abandoned,         // NEW: replaces Rubble for demand-abandoned buildings
    Road, Park, PowerPlant, PowerLine, WaterTower, WaterMain,
    Monument, Fire, Rubble, WaterBody,
}
```

**Extend Cell struct:**
```rust
pub struct Cell {
    pub tile: TileType,
    pub age: u8,
    pub style: u8,
    pub has_power: bool,
    pub has_water: bool,
    pub terrain_height: f32,
    pub terrain_type: TerrainType,
    // NEW fields:
    #[serde(default)]
    pub building_stage: u8,      // 0-2 for Phase 1 (0-7 for Phase 2)
    #[serde(default)]
    pub abandon_timer: u8,       // ticks of sustained negative conditions (0-255)
}
```

**Save migration:**
- Bump `SAVE_VERSION` to 2
- `#[serde(default)]` on new fields handles v1 saves automatically
- Existing Residential → Residential (keep as-is), existing Rubble → Abandoned

**Success criteria:**
- [ ] New TileType variants compile
- [ ] Existing saves load with default values for new fields
- [ ] Building rendering handles new tile types (ZonedR/C/I show empty land, Abandoned shows brown building)

---

#### Step 2: RCI Demand Engine (`src/sim/demand.rs`)

The heart of the new system. Computes demand for each zone type.

**Demand struct:**
```rust
pub struct RciDemand {
    pub residential: f32,  // positive = people want to move in
    pub commercial: f32,   // positive = businesses want to open
    pub industrial: f32,   // positive = factories want to build
}
```

**Demand formula (per tick):**
```
R_demand_raw = (C_jobs + I_jobs) - R_population
C_demand_raw = (0.3 × R_population) - C_count
I_demand_raw = (0.2 × R_population) - I_count
```

Where:
- `R_population` = sum of population across all Residential cells
- `C_jobs` = C_count × 5 (each commercial cell provides ~5 jobs)
- `I_jobs` = I_count × 10 (each industrial cell provides ~10 jobs)
- `C_count` = number of Commercial cells
- `I_count` = number of Industrial cells

**Demand smoothing (EMA):**
```
demand = demand_prev × (1 - alpha) + demand_raw × alpha
alpha = 0.1  (strong smoothing — demand changes slowly)
```

This prevents oscillation: demand shifts gradually, not in sudden spikes.

**Demand as Bevy Resource:**
```rust
#[derive(Resource, Default)]
pub struct RciDemand {
    pub residential: f32,
    pub commercial: f32,
    pub industrial: f32,
}
```

Computed in `GameSet::Sim` after the tick loop, before dirty-cell computation.

**Success criteria:**
- [ ] Demand values update each tick based on R/C/I counts and population
- [ ] Demand is smoothed via EMA (no wild swings)
- [ ] Demand is positive when zone type is undersupplied, negative when oversupplied

---

#### Step 3: Desirability Grid (`src/sim/desirability.rs`)

Per-cell desirability score determining WHERE buildings can grow.

**Desirability Resource:**
```rust
#[derive(Resource)]
pub struct DesirabilityGrid {
    pub values: Vec<f32>,   // one per cell, -100.0 to +100.0
    pub width: usize,
    pub height: usize,
}
```

**Phase 1 factors (additive scoring):**

| Factor | Score | Radius | Condition |
|--------|-------|--------|-----------|
| Road access | +20 | 1 cell | Has road neighbor |
| Park nearby | +15 | 5 cells | Diminishing: +15/+10/+7/+4/+2 by distance |
| Industrial pollution | -20 | 3 cells | Per industrial cell: -20/-12/-5 by distance |
| Abandoned blight | -10 | 2 cells | Per abandoned building: -10/-5 |
| Water proximity | +8 | 3 cells | Near WaterBody cells |
| Elevation bonus | +5 | n/a | terrain_height > 0.6 |
| Has power | +10 | n/a | Cell has power from network |
| Has water | +5 | n/a | Cell has water from network |

**Computation strategy:**
- Cache as a flat `Vec<f32>` parallel to grid cells
- Recompute only when grid changes (piggyback on dirty-cell tracking)
- Full recompute on game start, then incremental: recompute desirability for dirty cells + their neighbors within max factor radius (5 cells)

**Growth thresholds:**
- Building can grow on zoned land when: `desirability > 0.0` AND `demand > 0.0`
- Higher desirability = faster growth (probability scaled by desirability)
- Building stage upgrade requires `desirability > 20.0` for stage 1→2, `desirability > 40.0` for stage 2→3

**Success criteria:**
- [ ] Desirability grid computed per cell
- [ ] Parks increase nearby desirability
- [ ] Industrial decreases nearby residential desirability
- [ ] Abandoned buildings create blight (negative desirability)
- [ ] Road access is required for non-zero desirability

---

#### Step 4: Zone-Then-Develop Architecture

Replace direct building placement with SC4-style zoning.

**Mayor zones land** → cells become `ZonedResidential/ZonedCommercial/ZonedIndustrial`
**Automaton grows buildings** → when demand > 0 AND desirability > 0, zoned cells develop into full buildings

**Automaton changes in `src/sim/automaton.rs`:**

Replace `rule_empty` with:
```rust
fn rule_zoned_residential(cell, demand, desirability) -> Option<TileType> {
    if demand.residential > 0.0 && desirability > 0.0 {
        // Probability of development scales with demand + desirability
        let prob = (demand.residential / 100.0).clamp(0.0, 1.0) * (desirability / 50.0).clamp(0.0, 1.0);
        if rng.gen::<f32>() < prob * 0.05 {
            return Some(TileType::Residential);
        }
    }
    None
}
```

Similar rules for `rule_zoned_commercial` and `rule_zoned_industrial`.

**Existing building rules** — replace random decay with demand-driven abandonment:
```rust
fn rule_residential(cell, demand, desirability) -> Option<TileType> {
    // Upgrade stage if conditions are excellent
    if demand.residential > 20.0 && desirability > threshold_for_next_stage {
        cell.building_stage = (cell.building_stage + 1).min(2);
    }

    // Abandon if sustained negative conditions
    if demand.residential < -20.0 && desirability < 0.0 {
        cell.abandon_timer = cell.abandon_timer.saturating_add(1);
        if cell.abandon_timer > 20 { // ~20 ticks of bad conditions
            return Some(TileType::Abandoned);
        }
    } else {
        // Conditions improved — reset timer
        cell.abandon_timer = cell.abandon_timer.saturating_sub(1);
    }

    None
}
```

**Job matching within commute radius:**
- Residential cells check for C/I jobs within 10 cells
- If no jobs in radius AND demand is negative → accelerate abandonment
- If many jobs nearby AND demand positive → accelerate stage upgrade

**Success criteria:**
- [ ] Mayor zones empty land → cells show as "zoned" (colored ground, no building)
- [ ] Zoned cells develop into buildings when demand + desirability are positive
- [ ] Development probability scales with demand and desirability
- [ ] Buildings upgrade stages when conditions are excellent
- [ ] Buildings abandon after sustained (20+ ticks) negative demand + low desirability
- [ ] Abandon timer resets when conditions improve (recovery possible before threshold)

---

#### Step 5: Mayor AI Reads Demand

Rewrite the mayor's decision-making to respond to RCI demand.

**Key changes to `src/mayor/mod.rs`:**

1. **Zone based on demand** (replaces hardcoded ratio checks):
```rust
// In growth_tick:
if demand.residential > 10.0 && funds > zone_cost {
    // Find suitable empty land near roads
    // Zone it as ZonedResidential (NOT place buildings)
    grow_blob(grid, col, row, TileType::ZonedResidential, size, rng);
}
```

2. **Bulldoze abandoned buildings:**
```rust
// NEW: scan for abandoned buildings and bulldoze them
fn bulldoze_abandoned(&mut self, grid: &mut Grid, rng: &mut SmallRng) {
    // Find first abandoned building
    // Play SFX, pan camera, demolish → revert to Empty
    // Log narration: "Mayor demolishes abandoned building at..."
}
```

3. **Respond to demand balance:**
- If R demand high but C/I low → zone more C/I (people need jobs)
- If C demand high but R low → zone more R (shops need customers)
- If all demand negative → stop zoning, focus on infrastructure

4. **Founding phase** → largely unchanged (still places initial roads, power, water)
5. **Growth phase** → replaced with demand-responsive zoning
6. **Maturity/Evolution** → same progression but decisions driven by demand

**Bulldozer SFX:**
- Use existing `audio/sfx/demolish.ogg`
- SimEvents tracks `demolished` count → already wired to audio system

**Success criteria:**
- [ ] Mayor zones land based on positive RCI demand
- [ ] Mayor prioritizes the zone type with highest demand
- [ ] Mayor bulldozes abandoned buildings (with SFX)
- [ ] Mayor stops zoning when demand is negative
- [ ] Founding phase still works (initial city bootstrap)

---

#### Step 6: Rendering Updates

**Zoned-but-empty cells:**
- `ZonedResidential` → terrain texture with green tint/overlay (zoning indicator)
- `ZonedCommercial` → terrain with blue tint
- `ZonedIndustrial` → terrain with yellow tint
- Add these to `zone_atlas_index()` and the terrain atlas

**Abandoned buildings:**
- Already have brown `colormap_abandoned.png` texture
- `TileType::Abandoned` → render with brown cube at building height (existing code)

**Building stages:**
- Stage 0: scale 0.35 (small), Stage 1: scale 0.55 (medium), Stage 2: scale 0.80 (large)
- Use `cell.building_stage` instead of `age_stage(cell.age)` for model scale selection

**Success criteria:**
- [ ] Zoned empty land shows colored overlay on terrain
- [ ] Abandoned buildings show brown-tinted model
- [ ] Building scale reflects stage (not age)

---

#### Step 7: Save Migration & Integration

- Bump `SAVE_VERSION` to 2
- Add `#[serde(default)]` on `building_stage` and `abandon_timer`
- Existing `Rubble` cells → `Abandoned` on load
- Existing `Residential`/`Commercial`/`Industrial` → keep, set `building_stage` from age

**Success criteria:**
- [ ] V1 saves load successfully with defaults
- [ ] New saves include demand/desirability state
- [ ] Game plays correctly from both fresh start and loaded save

---

## System-Wide Impact

### Interaction Graph

```
Tick Loop:
  sim::tick()
    → automaton::apply_all_rules() [reads demand + desirability]
    → demand::compute_rci_demand() [reads grid state]
    → desirability::update_grid() [reads grid, writes desirability cache]
    → age & tax collection

  mayor::decide()
    → reads RciDemand resource
    → zones land (ZonedR/C/I) OR bulldozes abandoned
    → narrates actions

  compute_dirty_cells()
    → diffs grid state (including new tile types)
    → triggers terrain mesh + building + road + tree updates

  update_buildings()
    → reads building_stage for model scale (not age_stage)
    → renders Abandoned with brown material
    → renders ZonedR/C/I as empty land (no building)
```

### State Lifecycle Risks

- **Demand oscillation:** Mitigated by EMA smoothing (alpha=0.1). Demand changes 10% per tick max.
- **Blight cascade:** Abandoned buildings lower desirability, which could trigger more abandonment. Circuit breaker: abandon_timer requires 20+ consecutive bad ticks AND blight penalty is capped at -10 per abandoned cell.
- **Cold start:** New city has zero population, zero demand. Founding phase bypasses demand system to bootstrap.

### API Surface Parity

- Save/load: New fields with `#[serde(default)]` — backward compatible
- Rendering: Dirty-cell tracking handles all visual updates — no rendering changes needed beyond new tile type handling
- Audio: SimEvents already tracks building/demolition events — no audio changes needed

## Acceptance Criteria

### Functional Requirements

- [ ] RCI demand computed each tick from population, jobs, zone counts
- [ ] Demand smoothed via EMA (no wild oscillation)
- [ ] Per-cell desirability grid with pollution, parks, road, utility factors
- [ ] Mayor zones empty land as ZonedR/C/I based on demand
- [ ] Zoned land develops into buildings when demand > 0 AND desirability > 0
- [ ] Buildings upgrade through 3 stages based on sustained positive conditions
- [ ] Buildings abandon after 20+ ticks of negative demand + low desirability
- [ ] Abandoned buildings show brown tint, lower neighbor desirability
- [ ] Mayor bulldozes abandoned buildings (with SFX)
- [ ] Founding phase bootstraps city without demand system
- [ ] City grows organically — feels like SC4, not random noise

### Non-Functional Requirements

- [ ] Demand computation < 1ms per tick (simple arithmetic over grid counts)
- [ ] Desirability grid update < 5ms (incremental via dirty cells)
- [ ] No demand oscillation death spirals
- [ ] Save backward compatibility (v1 saves load with defaults)

### Quality Gates

- [ ] `cargo clippy` passes
- [ ] Game runs stably for 30+ minutes at 8x speed without crashes or death spirals
- [ ] City grows from 0 to 500+ population organically
- [ ] Mayor makes sensible zoning decisions visible to the player

## Dependencies & Prerequisites

- No new crate dependencies
- Bevy 0.15 stays as-is
- Existing rendering/audio systems handle new tile types via dirty-cell tracking
- Kenney building models already loaded — just need stage-based selection

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Demand oscillation / boom-bust | Medium | High | EMA smoothing (alpha=0.1), abandon timer cooldown |
| Blight cascade destroying neighborhoods | Medium | Medium | Cap blight penalty at -10, require 20-tick timer |
| Mayor AI too complex for demand-driven decisions | Low | Medium | Phase 1 is simple ratio-based; same decision tree structure |
| Cold start deadlock (no demand, no growth) | Low | High | Founding phase bypasses demand system |
| Save compatibility breakage | Low | Medium | #[serde(default)] + version check |
| Performance (desirability grid) | Low | Low | Cached grid, incremental updates via dirty cells |

## Future Considerations (Phase 2+)

- **Wealth tiers** — R$/R$$/R$$$, C$/C$$/C$$$, I-Dirty/I-Mfg/I-HiTech
- **8 building stages** — population-gated density (1,114 for medium, 25,952 for high)
- **Civic buildings** — schools, hospitals, police (mayor auto-places)
- **Tax system** — mayor adjusts rates, affects demand
- **Crime/education/health** desirability factors
- **RCI demand bar UI** — visual feedback on demand state

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-17-rci-demand-economics-brainstorm.md](docs/brainstorms/2026-03-17-rci-demand-economics-brainstorm.md)
- Key decisions: zone-then-develop architecture, EMA demand smoothing, 10-cell commute radius, phased implementation, mayor reads demand + bulldozes

### Internal References

- Automaton rules: `src/sim/automaton.rs` (being replaced)
- Mayor AI: `src/mayor/mod.rs` (being rewritten for demand-driven decisions)
- Grid types: `src/grid/mod.rs` (adding new TileType variants)
- Stats: `src/sim/stats.rs` (adding demand tracking)
- Growth: `src/sim/growth.rs` (targeting changes for demand-driven placement)

### External References

- [SimCity 4 RCI Mechanics — StrategyWiki](https://strategywiki.org/wiki/SimCity_4/Zoning_and_Demand)
- [SC4 Demand & Abandonment — Simtropolis](https://community.simtropolis.com/omnibus/simcity-4/reference/demand-desirability-and-abandonment-r31/)
- [SC4 Stage Caps — The Infinite Zenith](https://infinitemirai.wordpress.com/2012/01/05/sim-city-4-stage-caps/)

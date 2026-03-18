# Brainstorm: SC4-Style RCI Demand Economics

**Date:** 2026-03-17
**Status:** Complete
**Next step:** `/ce:plan`

## What We're Building

A complete rewrite of SlideCity's simulation core to replace the current random age-based growth/decay system with SimCity 4-style demand-driven economics. Buildings grow because there's demand for them, and abandon because economic conditions deteriorate — not because of random dice rolls.

### Core Systems

1. **RCI Demand Tracking** — R demand from available jobs, C demand from population, I demand from population. Ratio targets for mayor zoning + job matching for individual building growth.
2. **Three Wealth Tiers** — R$/R$$/R$$$, C$/C$$/C$$$, I-Dirty/I-Manufacturing/I-HiTech (9 sub-categories like SC4)
3. **8 Building Stages** — Stages 1-3 low density, 4-6 medium density (pop >1,114), 7-8 high density (pop >25,952). Each stage mapped to different Kenney models.
4. **Full Desirability System** — Per-cell land value driven by pollution, crime, parks, education, health, transit, water proximity, elevation. Drives where buildings grow.
5. **Demand-Driven Abandonment** — Buildings abandon due to sustained negative demand + low desirability + lack of utilities. Turn brown, never recover, mayor must bulldoze.
6. **Mayor AI Reads Demand** — Mayor zones based on RCI demand graph, bulldozes abandoned buildings, responds to economic conditions.
7. **Bulldozer** — Mayor-driven demolition with sound effects and visual feedback.

## Why This Approach

The current system feels arbitrary — buildings appear and vanish randomly. SC4's genius was the economic feedback loop:
- People move in because there are jobs (R demand from C+I)
- Shops open because there are customers (C demand from R)
- Factories open because there are workers (I demand from R)
- Each drives the other, creating organic city growth

Without this loop, the simulation is just random noise. With it, the city feels alive and the player (or mayor AI) can understand WHY things happen.

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Wealth tiers | 3 tiers ($/$$/$$$) | Full SC4 complexity — 9 zone sub-types |
| Desirability | Full system | Pollution, crime, parks, education, health, transit, water, elevation |
| Building stages | 8 stages (SC4 model) | 1-3 low, 4-6 medium, 7-8 high density |
| Demand formula | Ratio targets + job matching | Ratios guide mayor, jobs drive individual growth |
| Mayor AI | Reads RCI demand | Zones based on demand, bulldozes abandoned |
| Abandonment | SC4-faithful | Brown buildings, never recover, mayor bulldozes |
| Population thresholds | SC4 values | 1,114 for medium density, 25,952 for high density |
| Bulldozer | Mayor-driven with SFX | Sound effects, visual feedback (dust cloud?) |

## The RCI Feedback Loop

```
Population (R) ──creates──> Commercial Demand (C)
     │                           │
     │                      creates jobs
     │                           │
     └──────needs jobs──────────┘

Population (R) ──creates──> Industrial Demand (I)
     │                           │
     │                      creates jobs
     │                           │
     └──────needs jobs──────────┘

Industrial ──pollution──> Lowers R desirability nearby
Parks/Schools ──────────> Raises R desirability nearby
```

## Desirability Factors

| Factor | Affects | Radius | Effect |
|--------|---------|--------|--------|
| Park | R, C | 5 cells | +desirability |
| Industrial pollution | R | 3 cells | -desirability |
| Crime (from density) | R, C | 3 cells | -desirability |
| Education (schools) | R | 5 cells | +desirability, +wealth |
| Health (hospitals) | R | 5 cells | +desirability |
| Road access | R, C, I | 1 cell | Required for growth |
| Water proximity | R | 3 cells | +desirability |
| Elevation | R | n/a | Higher = +desirability |
| Abandoned buildings | R, C | 2 cells | -desirability (blight) |
| Power/Water utilities | All | n/a | Required for medium+ density |

## Resolved Questions

1. **Wealth tiers** — Full 3 tiers like SC4 (9 sub-categories)
2. **Desirability** — Full system with all SC4 factors
3. **Building stages** — 8 stages with population-gated density
4. **Demand formula** — Ratio targets for mayor + job matching for buildings
5. **Abandonment** — SC4-faithful (brown, permanent, mayor bulldozes)
6. **Mayor AI** — Reads demand, zones accordingly, bulldozes abandoned
7. **Bulldozer** — With sound effects and visual animation

## Resolved Questions (continued)

8. **Civic buildings** — Mayor auto-places education + safety buildings at population thresholds (PHASE 3).
9. **Commute simulation** — Simple radius check: jobs within 10 cells count. No pathfinding.
10. **Tax system** — Mayor adjusts taxes dynamically (PHASE 2 or 3).
11. **Phasing** — Phase 1: basic RCI demand (3 types, no wealth tiers) + desirability + zone-then-develop + abandonment. Phase 2: wealth tiers + taxes. Phase 3: civics + advanced desirability.
12. **Zone-then-develop** — SC4 model. Mayor zones empty land. Buildings appear organically based on demand + desirability. New "zoned but empty" cell states needed (ZonedR, ZonedC, ZonedI).
13. **Commute radius** — 10 cells. Allows 2-3 blocks of separation.
14. **Demand damping** — Exponential moving average (alpha=0.1) on demand values to prevent oscillation. 20-tick cooldown before abandonment. Blight cascade capped.
15. **Desirability** — Additive scoring, cached per-cell grid, recalculated via dirty-cell tracking. Thresholds: growth requires desirability > 0. Higher stages require higher desirability.
16. **Save compatibility** — Bump to save_version 2. Add #[serde(default)] on new Cell fields. Migration function for v1 saves.

## Phase 1 Scope (this implementation pass)

**IN scope:**
- RCI demand tracking (3 types: R, C, I — no wealth tiers)
- Demand formula: R_demand = (C_jobs + I_jobs) - R_population; C_demand = 0.3 * R_pop - C_count; I_demand = 0.2 * R_pop - I_count
- Job counts per building: C provides ~5 jobs per cell, I provides ~10 jobs per cell
- Zone-then-develop architecture (ZonedR/ZonedC/ZonedI cell states)
- Basic desirability grid (pollution, parks, road access, utilities, abandoned blight)
- Demand-gated building growth (buildings only appear on zoned land when demand > 0 and desirability > threshold)
- Demand-driven abandonment (sustained negative demand + low desirability = brown abandoned building)
- Mayor AI reads demand, zones accordingly, bulldozes abandoned
- Bulldozer with SFX
- Demand damping (EMA smoothing + abandonment cooldown)
- 3 building stages (mapped to scale for now — not 8 stages yet)
- Save migration v1 → v2

**OUT of scope (Phase 2+):**
- Wealth tiers (9 sub-categories)
- 8 building stages
- Civic buildings (schools, hospitals, police)
- Tax system
- Crime, education, health desirability factors
- Advanced commute distance calculation

## Open Questions

None — all key decisions resolved for Phase 1.

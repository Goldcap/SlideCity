---
title: "feat: SlideCity — Autonomous Isometric City Simulation"
type: feat
status: active
date: 2026-03-14
origin: docs/brainstorms/2026-03-14-slidecity-design-brainstorm.md
---

# SlideCity — Autonomous Isometric City Simulation

## Overview

Build a fully autonomous isometric city simulation in the spirit of SimCity 2000, where a Virtual Mayor AI builds and manages the city. The player is primarily a spectator who can influence the mayor through a tiered political interaction system powered by Influence Points. Built in Rust with Macroquad 0.4, featuring cellular automaton rules, dynamic contextual music, LLM-powered mayor conversations, and pixel art sprites.

## Problem Statement / Motivation

There's no modern game that captures the joy of watching a SimCity 2000 city come alive — the organic growth, the personality, the feeling of a living system. SlideCity aims to recreate that experience with modern tech: AI-driven decision making, LLM-powered character interactions, and procedurally generated art and music. The player watches, nudges, and occasionally argues with an AI mayor who has opinions.

## Proposed Solution

A Rust application using Macroquad 0.4 with the following core systems:

1. **Grid Simulation** — 80×52 cell grid with Conway/SimCity cellular automaton rules
2. **Virtual Mayor** — AI agent with personality archetypes, state machine, and LLM-powered conversations
3. **Isometric Renderer** — Painter's algorithm with visual juice (particles, animations, day/night)
4. **Dynamic Audio** — 8 contextual tracks with crossfading based on city state
5. **Player Influence** — IP economy with 3 tiers of mayor interaction
6. **Art Pipeline** — Replicate-generated pixel art with colored-rect fallbacks

## Technical Approach

### Architecture

Plain Rust structs (no ECS). Top-level `GameState` owns all subsystems. Fixed-timestep simulation tick (800ms default, adjustable via speed slider). Double-buffered grid for simulation correctness.

**Key Macroquad 0.4 patterns:**
- `Camera2D` for world rendering, `set_default_camera()` for UI overlays
- `build_textures_atlas()` called once after all textures loaded
- `FilterMode::Nearest` on all textures for pixel art
- Audio opt-in via `features = ["audio"]`
- `rand = { version = "0.8", features = ["small_rng"] }` (not `rand_small` — that crate doesn't exist)

### Core Formulas (from spec-flow analysis)

These are load-bearing values referenced across multiple systems. Defined here as the authoritative source.

**Time:**
- 1 game year = 200 ticks (160 seconds at 1x speed with 800ms base tick)
- 1 season = 50 ticks (Spring/Summer/Fall/Winter, cosmetic labels only)
- Seasons do NOT affect simulation rules

**Population:**
```
population = sum over all residential cells of:
  stage1 (age 0-15):  2 per cell
  stage2 (age 16-45): 6 per cell
  stage3 (age 46+):  12 per cell
```

**Happiness (0.0 - 1.0):**
```
happiness = weighted average of:
  0.25 × park_ratio        (parks / total developed cells, clamped 0-1)
  0.25 × power_coverage    (powered cells / total zone cells)
  0.20 × water_coverage    (watered cells / total zone cells)
  0.20 × (1.0 - pollution) (pollution = industrial_adjacent_to_residential / total_residential)
  0.10 × commercial_ratio  (commercial / residential, clamped at 0.3 = ideal)
```

**Difficulty Multipliers:**

| Parameter | Peaceful | Normal | Harsh |
|-----------|----------|--------|-------|
| Fire spread prob | 0.14 | 0.28 | 0.42 |
| Decay/abandon rates | 0.5× | 1.0× | 2.0× |
| Starting funds | §100,000 | §75,000 | §50,000 |
| Tax income | 1.5× | 1.0× | 0.7× |
| Random fire chance | 0.0003 | 0.0006 | 0.001 |
| Disaster button cooldown | 60s | 30s | 15s |

**Influence Points:**
- Suggestion Box: 1 IP
- Council Vote: 3 IP
- Direct Audience: 5 IP
- Passive earning: +1 IP per game year
- Pop milestones: +1 IP at 50, 100, 200, 350, 500
- Surviving disaster: +1 IP (fire burns out without >50% city loss)
- Phase transition: +1 IP
- Disaster triggered: +2 IP (max 2 per game year to prevent griefing loop)
- Treasury purchase: §5,000 per IP

**Utility Reach:**
- PowerPlant: manhattan distance 12 from plant cell
- PowerLine: extends reach — each PowerLine cell acts as a new source with radius 6
- WaterTower: manhattan distance 10 from tower cell
- WaterMain: extends reach — each WaterMain cell acts as a new source with radius 5

**Speed Slider Behavior:**
- Simulation tick rate scales (800ms / speed_multiplier)
- Mayor tick rate scales with simulation (still every 8th tick)
- Camera animations do NOT scale (always real-time smooth)
- Audio crossfade does NOT scale (always 3.0s real-time)
- Particle effects do NOT scale (always real-time)

**Mayor Powers (all 8 archetypes):**

| Archetype | Power | Weakness |
|-----------|-------|----------|
| The Developer | Roads cost 50% less | Parks decay 2× faster |
| The Environmentalist | Parks block fire spread | Industrial tax income -50% |
| The Baron | Industrial tax income doubled | Park placement cost 3× |
| The Pragmatist | All costs -10% | No special bonus (perfectly average) |
| The Nervous Mayor | Disaster recovery 2× faster | Expansion rate -50% |
| The Visionary | Monument cost halved, COM grows 50% faster | Early-game funds drain 25% faster |
| The Machine | Industrial output doubled | Happiness decays 2× faster |
| The Philosopher | Baseline happiness +20% | Growth rate -30% |

**Mayor Succession:**
- Player chooses successor (selection screen reappears)
- IP is preserved across mayor transitions
- Successor starts in Growth if pop < 200, Maturity if pop < 500, Evolution otherwise
- Successor inherits city state but gets a fresh narration log

**Game State Machine:**
```
MainMenu → MayorSelect → DifficultySelect → Loading → Playing ⇄ Paused → (optional: GameOver)
```

**Other Resolved Gaps:**
- City death: simulation continues, mayor gets no income at pop 0, city can recover from rubble
- Minimap: clickable to pan camera
- Debug mode: `--debug` CLI flag or F12 key toggle
- Day/night: purely cosmetic, 2-minute real-time cycle
- Road pathing: A* with WaterBody and existing buildings as obstacles
- LLM chat UI: modal overlay that pauses simulation
- Save: auto-save every 60s + manual via Esc menu, versioned JSON, platform-specific save dir
- Disaster during mayor retirement: mayor completes current crisis response before retiring

### Project Structure

```
slidecity/
├── Cargo.toml
├── SPEC/
│   ├── SPEC.md
│   └── CLAUDE.md
├── docs/
│   ├── brainstorms/
│   └── plans/
├── .env.example
├── assets/
│   ├── sprites/          ← Generated PNGs (or empty — fallbacks work)
│   ├── audio/            ← Suno OGGs
│   └── fonts/
├── scripts/
│   └── generate_assets.py
└── src/
    ├── main.rs
    ├── game.rs              ← Main game loop, state machine
    ├── config.rs            ← Difficulty, game settings
    ├── grid/
    │   ├── mod.rs           ← Grid type, Cell, flat Vec storage
    │   ├── terrain.rs       ← Heightmap + water body generation
    │   └── neighbors.rs     ← Neighbor counting utilities
    ├── sim/
    │   ├── mod.rs           ← Simulation tick orchestration
    │   ├── automaton.rs     ← Conway/SimCity cell rules
    │   ├── utilities.rs     ← Power/water flood-fill networks
    │   └── growth.rs        ← Zone blob seeding + expansion
    ├── mayor/
    │   ├── mod.rs           ← Mayor brain, phase machine, decisions
    │   ├── personality.rs   ← Archetypes, powers, weaknesses
    │   ├── narration.rs     ← Log strings by phase/mood/personality
    │   └── llm.rs           ← Claude API integration for Direct Audience
    ├── influence/
    │   ├── mod.rs           ← IP economy, tier definitions
    │   ├── suggestion.rs    ← Suggestion box logic
    │   ├── council.rs       ← Council vote system
    │   └── audience.rs      ← Direct audience (LLM chat)
    ├── renderer/
    │   ├── mod.rs           ← Draw loop orchestration
    │   ├── iso.rs           ← Isometric projection math
    │   ├── tiles.rs         ← Sprite lookup + colored rect fallbacks
    │   ├── camera.rs        ← Zoom, pan, smooth follow, shake
    │   ├── atlas.rs         ← Texture atlas management
    │   ├── particles.rs     ← Smoke, sparks, building pop-in
    │   └── lighting.rs      ← Day/night cycle tinting
    ├── audio/
    │   └── mod.rs           ← Music layer selection + crossfade
    └── ui/
        ├── mod.rs
        ├── stats.rs         ← HUD: pop, funds, happiness, year
        ├── mayor_log.rs     ← Scrolling thought log panel
        ├── minimap.rs       ← Minimap canvas
        ├── start_screen.rs  ← Mayor selection + difficulty picker
        └── influence_ui.rs  ← IP display, interaction buttons
```

### Implementation Phases

---

#### Phase 1: Foundation (Grid & Project Setup)

**Goal:** Window opens, colored grid renders in isometric projection.

**Files:**
- `Cargo.toml`
- `.gitignore`
- `src/main.rs`
- `src/grid/mod.rs`
- `src/grid/terrain.rs`
- `src/grid/neighbors.rs`
- `src/config.rs`

**Tasks:**
- [ ] Create `Cargo.toml` with dependencies: `macroquad = { version = "0.4", features = ["audio"] }`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `rand = { version = "0.8", features = ["small_rng"] }`, `reqwest = { version = "0.12", features = ["json"] }`
- [ ] Implement `Grid` struct with flat `Vec<Cell>` storage, `idx(col, row)`, `get()`, `get_mut()`
- [ ] Implement `Cell` struct and `TileType` enum (14 variants) with `Clone`, `Copy`, `PartialEq`
- [ ] Implement terrain generation: layered sine heightmap, water body flood-fill (1-2 regions, max 6% of cells)
- [ ] Implement neighbor counting utilities (radius-based counts by TileType)
- [ ] Create `config.rs` with difficulty presets (Easy/Normal/Hard) affecting simulation parameters
- [ ] Verify: `cargo build` succeeds, grid initializes with terrain

**Success criteria:** Grid struct compiles, terrain generates reproducibly with a given seed.

---

#### Phase 2: Simulation Core

**Goal:** Cellular automaton runs, zones grow and decay, utilities propagate.

**Files:**
- `src/sim/mod.rs`
- `src/sim/automaton.rs`
- `src/sim/utilities.rs`
- `src/sim/growth.rs`

**Tasks:**
- [ ] Implement double-buffered simulation tick: read from `grid`, write to `next_grid`, `std::mem::swap`
- [ ] Implement all automaton rules from spec: Empty→zone seeding, Residential decay/upzone, Commercial decay, Industrial gentrification, Fire spread (difficulty-adjusted), Rubble clearing, Park encroachment
- [ ] Implement power network flood-fill from PowerPlant through PowerLine (manhattan distance 12)
- [ ] Implement water network flood-fill from WaterTower through WaterMain (radius 10)
- [ ] Recompute utilities every 5 ticks
- [ ] Implement blob growth via BFS with weighted random direction bias — does not cross Road or WaterBody
- [ ] Target blob sizes: RES 8-28, COM 4-16, IND 10-32, Park 4-14
- [ ] Implement tax income per tick: RES §3, COM §9, IND §6
- [ ] Implement population calculation (sum of residential cells, weighted by age/stage)
- [ ] Implement happiness calculation based on park proximity, utility coverage, industrial pollution
- [ ] Thread single `SmallRng` instance through all simulation functions

**Success criteria:** Starting from a manually-placed road + RES blob + PowerPlant + WaterTower, the simulation runs and zones grow/decay organically over 100+ ticks without crashing or stalling.

---

#### Phase 3: Isometric Renderer

**Goal:** City renders beautifully in isometric view with colored-rect fallbacks and visual juice.

**Files:**
- `src/renderer/mod.rs`
- `src/renderer/iso.rs`
- `src/renderer/tiles.rs`
- `src/renderer/camera.rs`
- `src/renderer/atlas.rs`
- `src/renderer/particles.rs`
- `src/renderer/lighting.rs`

**Tasks:**
- [ ] Implement isometric projection: `grid_to_screen(col, row, height_floors, camera)` and inverse `screen_to_grid`
- [ ] Implement painter's algorithm: sort draw calls by `(row + col)` ascending, secondary sort by `row` descending for same-depth stability
- [ ] Implement colored-rect fallback renderer: distinct colors per TileType, white text initial letter, building height represented by stacked rects
- [ ] Implement sprite lookup: try `load_texture` for each tile type, graceful fallback to colored rect if missing
- [ ] Implement `build_textures_atlas()` call after all textures loaded, `FilterMode::Nearest` on all textures
- [ ] Implement tall sprite anchoring: draw at `screen_y - (sprite_height - TILE_H)`
- [ ] Implement camera: `Camera2D` with smooth lerp toward target, 5 zoom levels `[0.5, 0.75, 1.0, 1.5, 2.0]`
- [ ] Implement camera controls: scroll wheel zoom (normalize for browser), click-drag pan, arrow key pan
- [ ] Implement mayor action auto-pan: smooth lerp to action location (0.8s)
- [ ] Implement disaster snap-pan with screen shake (brief sinusoidal offset)
- [ ] Implement viewport frustum culling (skip cells outside camera view)
- [ ] Implement particle system: smoke puffs from industrial (age > 30), sparks from fire, dust from construction
- [ ] Implement building pop-in animation: scale from 0 to 1 over 0.3s when cell.age == 0
- [ ] Implement day/night cycle tinting: color overlay that shifts warm→cool over game years

**Success criteria:** 80×52 grid renders at 60fps with colored rects, painter's sort is visually correct (tall buildings occlude correctly), camera zoom/pan is smooth, particles visible on fire and industrial tiles.

---

#### Phase 4: The Virtual Mayor

**Goal:** Mayor builds the city autonomously from nothing to monument. Feels like a character.

**Files:**
- `src/mayor/mod.rs`
- `src/mayor/personality.rs`
- `src/mayor/narration.rs`

**Tasks:**
- [ ] Implement `MayorPersonality` struct with 5 trait floats + name + emoji
- [ ] Implement 8 archetypes with distinct **powers and weaknesses** (see brainstorm):
  - The Developer — roads cost 50% less, parks decay 2x faster
  - The Environmentalist — parks block fire spread, industrial tax -50%
  - The Baron — industrial output doubled, park placement cost 3x
  - The Pragmatist — all costs -10%, no special bonuses (balanced)
  - The Nervous Mayor — disaster recovery 2x faster, expansion 50% slower
  - The Visionary — monument cost halved, early-game funds drain faster
  - The Machine — industrial output doubled, happiness decays 2x faster
  - The Philosopher — happiness bonus +20% baseline, growth rate -30%
- [ ] Implement `MayorPhase` state machine: Founding → Growth → Maturity → Evolution
- [ ] Implement phase transitions based on year thresholds (1-3, 3-10, 10-25, 25+)
- [ ] Implement mayor decision loop (every 8 sim ticks):
  1. ASSESS: count zones, population, happiness, utility coverage, fires, rubble, funds
  2. PRIORITIZE: evaluate urgency × personality weights (critical/high/medium/low)
  3. ACT: place 1-3 items, deduct funds, apply personality power/weakness modifiers
  4. NARRATE: push log entry with personality-flavored text
  5. CAMERA: request smooth pan to action location
- [ ] Implement Founding phase sequence: road spine → first RES blob → PowerPlant → PowerLine → WaterTower → WaterMain
- [ ] Implement Growth phase: expand RES, add COM when R:C > 4:1, add IND based on industrial_bias, extend roads and utilities, first park when happiness < 0.65
- [ ] Implement Maturity phase: density management, monument at pop > 500, disaster recovery, utility extension to struggling areas
- [ ] Implement Evolution phase: gentrification waves (IND → COM), urban renewal, mayor retirement at year 30-40 with new personality roll
- [ ] Implement fund management behavior: pause expansion below §10k, halt non-critical below §3k, aggressive above §50k
- [ ] Implement ~100 narration strings tagged by phase + mood + personality (see spec for samples)
- [ ] Implement the **monument moment**: when pop > 500 and no monument exists, mayor builds Monument, camera pans, log entry "The people deserve something permanent"

**Success criteria:** Starting from an empty terrain, the mayor builds a functioning city that progresses through all 4 phases. Different personality archetypes produce visibly different cities. The monument moment triggers correctly.

---

#### Phase 5: Dynamic Audio

**Goal:** City mood is told through sound. Music crossfades contextually.

**Files:**
- `src/audio/mod.rs`

**Tasks:**
- [ ] Implement `AudioManager` with track loading (`load_sound`), current/next track state, fade timer
- [ ] Implement crossfade: fade out current track, fade in next over 3.0 seconds using `set_sound_volume`
- [ ] Implement track selection logic (re-evaluated every 10 ticks):
  - fire_count > 2 → Disaster
  - happiness < 0.30 → Decline
  - population == 0 → EmptyLand
  - population < 60 → FirstStreets
  - population < 350 → GrowingCity
  - happiness < 0.50 → Recovery
  - else → BoomTown
- [ ] Implement monument sting: one-shot play over current music, then resume
- [ ] Graceful handling when audio files are missing (silent mode, no panics)
- [ ] Placeholder support: system works with any subset of tracks present

**Success criteria:** Music changes perceptibly as city grows from empty to thriving. Monument sting plays at the right moment. No audio-related crashes when files are missing.

---

#### Phase 6: UI & Start Screen

**Goal:** Full HUD, mayor log, minimap, and a rich start screen with mayor selection and difficulty picker.

**Files:**
- `src/ui/mod.rs`
- `src/ui/stats.rs`
- `src/ui/mayor_log.rs`
- `src/ui/minimap.rs`
- `src/ui/start_screen.rs`
- `src/ui/influence_ui.rs`

**Tasks:**
- [ ] Implement start screen state machine: Title → Mayor Select → Difficulty Select → Game
- [ ] Implement mayor selection: display all 8 archetypes with emoji + name (no stats), animated highlight on hover, map seed input field
- [ ] Implement difficulty selector: Easy / Normal / Hard with brief description of what changes
- [ ] Implement game speed selector: 1x / 2x / 4x / 8x default
- [ ] Implement top HUD bar (screen space via `set_default_camera()`): Pop, R/C/I counts, funds, year+season, happiness %, power %, water %
- [ ] Implement right panel: mayor emoji + name + archetype title, scrolling log (last 7 entries, newest first, older entries fade to 60% opacity)
- [ ] Implement log entry format: `[emoji] Year X, [Season] — [text]`
- [ ] Implement minimap: colored 2×2 pixel rects per cell type, camera viewport indicator rect
- [ ] Implement disaster button: [FIRE] button in right panel, spawns fire on random developed cell, earns IP
- [ ] Implement IP display: current IP count, tier cost indicators
- [ ] Implement speed slider (visible in-game, not just debug mode)

**Success criteria:** Start screen flows cleanly to gameplay. HUD is readable at all zoom levels. Mayor log updates in real-time with personality-flavored narration. Minimap accurately reflects city state.

---

#### Phase 7: Player Influence System

**Goal:** Players earn and spend IP to influence the mayor across three tiers.

**Files:**
- `src/influence/mod.rs`
- `src/influence/suggestion.rs`
- `src/influence/council.rs`
- `src/influence/audience.rs`

**Tasks:**
- [ ] Implement `InfluenceState`: current IP balance, earning history, spending history
- [ ] Implement IP earning:
  - +1 IP per game year (passive)
  - +1 IP at population milestones (50, 100, 200, 350, 500)
  - +1 IP for surviving disasters (fire burns out without total collapse)
  - +1 IP for phase transitions
  - Buyable with city funds (§5,000 per IP — drains mayor's budget)
  - +2 IP for triggering disasters (moral hazard incentive)
- [ ] Implement Suggestion Box (cost: 1 IP):
  - Player selects an action category (build park, zone residential, extend power, etc.)
  - Mayor evaluates against personality: compliance probability = 0.3 + (0.5 × alignment with personality)
  - Mayor logs response: comply ("Fine, I'll build your park."), ignore ("Not a priority."), or argue ("Parks are wasted land.")
- [ ] Implement Council Vote (cost: 3 IP):
  - Mayor generates 3 candidate actions based on current assessment
  - Player picks one
  - Mayor executes chosen action but may log reluctance if it conflicts with personality
  - Small chance (15%) mayor overrides player choice anyway ("I know what this city needs.")
- [ ] Implement Direct Audience UI (cost: 5 IP):
  - Free-text input field
  - Shows "Requesting audience with the mayor..." loading state
  - Displays mayor response in the log panel with special formatting
  - Response influences next 2-3 mayor decisions (compliance boost)

**Success criteria:** IP accumulates through gameplay. All three tiers produce distinct mayor responses. The disaster-for-IP loop creates interesting player decisions. Mayor personality visibly affects response probability.

---

#### Phase 8: LLM Integration (Claude API)

**Goal:** Direct Audience tier produces genuinely in-character mayor responses via Claude API.

**Files:**
- `src/mayor/llm.rs`

**Tasks:**
- [ ] Implement Claude API client using `reqwest`:
  - POST to `https://api.anthropic.com/v1/messages`
  - API key from `.env` or environment variable `ANTHROPIC_API_KEY`
  - Model: `claude-haiku-4-5-20251001` (fast, cheap, sufficient for character responses)
- [ ] Implement mayor system prompt builder:
  - Personality traits and archetype description
  - Current city state (pop, happiness, funds, active crises, recent builds)
  - Mayor's recent log entries (last 5) for conversational continuity
  - Instruction: respond in character, 1-3 sentences, may argue/deflect/agree
- [ ] Implement rate-limited free-tier proxy (optional, for distribution):
  - Simple HTTP proxy that adds API key server-side
  - Rate limit: 3 requests per game session
  - Fallback: if proxy unavailable and no local API key, show "The mayor is unavailable" message
- [ ] Implement response parsing and integration:
  - Parse Claude response, truncate to 200 chars if needed
  - Push to mayor log with special [AUDIENCE] tag
  - Set compliance boost flag for next 2-3 mayor decisions
- [ ] Implement conversation history (last 3 exchanges) for multi-turn coherence
- [ ] Handle errors gracefully: timeout → "The mayor is busy", rate limit → "The mayor needs rest", API error → "Communication breakdown"

**Success criteria:** Player types a message, mayor responds in character within 2-3 seconds. Personality is clearly expressed in responses. Errors degrade gracefully to log messages, never crashes.

---

#### Phase 9: Art & Audio Pipeline

**Goal:** Generate sprite assets via Replicate and music via Suno.

**Files:**
- `scripts/generate_assets.py`
- `.env.example`

**Tasks:**
- [ ] Create `.env.example` with `REPLICATE_API_TOKEN=your_token_here` and `ANTHROPIC_API_KEY=your_key_here`
- [ ] Implement `generate_assets.py`:
  - Static sprites via `retro-diffusion/rd-plus` with `style=isometric_asset`, `remove_bg=True`
  - Animated sprites (fire, water, smoke) via `retro-diffusion/rd-animation`
  - ~40 sprites total across terrain, roads, residential (3 stages × 4 variants), commercial (3×4), industrial (3×3), infrastructure, events
  - Correct dimensions: ground 64×32, buildings 64×96, tall 64×128, infra 64×80, events 64×64
  - Download and save to `assets/sprites/` subdirectories
- [ ] Test sprite consistency — if retro-diffusion produces inconsistent results, document alternatives
- [ ] Document Suno prompts for all 8 tracks (provided in spec) + monument sting
- [ ] Create `assets/audio/` directory structure

**Success criteria:** Running the script generates a complete sprite set. Game renders with real sprites when available, colored rects when not.

---

#### Phase 10: Game Loop & Polish

**Goal:** Wire everything together into a polished, complete game loop.

**Files:**
- `src/game.rs`
- `src/main.rs`

**Tasks:**
- [ ] Implement `GameState` struct owning all subsystems
- [ ] Implement game state machine: StartScreen → Playing → Paused
- [ ] Implement main update loop:
  1. Handle input (camera, UI interactions, influence actions)
  2. Advance simulation tick (fixed timestep, respecting speed multiplier)
  3. Update camera smooth lerp
  4. Update audio crossfade
  5. Update particles
  6. Update day/night cycle
- [ ] Implement save/load via serde_json:
  - Serialize: grid state, mayor state, influence state, funds, tick count, year, conversation history
  - Deserialize and resume from any point
  - Auto-save every 60 seconds
- [ ] Implement year/season cycle: 1 year = 50 ticks, seasons rotate every 12.5 ticks
- [ ] Implement `?debug=true` mode: tick rate slider, step button, grid overlay, cell inspector
- [ ] Performance verification: 60fps at all zoom levels with full 80×52 developed grid
- [ ] Final integration test: start new game → watch mayor build from nothing → reach monument

**Success criteria:** The monument moment works end-to-end: population hits 500, mayor builds monument, camera pans, sting plays, log says "The people deserve something permanent." The entire flow from start screen to monument feels polished and alive.

---

## Alternative Approaches Considered

### ECS (Bevy/Hecs) vs Plain Structs
Rejected. 4,160 cells with simple state transitions don't benefit from ECS overhead. Plain structs with a flat `Vec<Cell>` are simpler, faster to compile, and easier to reason about. (see brainstorm: docs/brainstorms/2026-03-14-slidecity-design-brainstorm.md)

### Raylib-rs vs Macroquad
Rejected per spec. Macroquad provides built-in texture atlas, audio, and WASM target. Camera/transform system is simpler for isometric. Claude Code knows it well.

### Scripted Mayor Responses vs LLM
LLM chosen for Direct Audience tier. Response trees would feel canned and break immersion. Claude API (Haiku) is fast and cheap enough for 3-5 responses per game session. Free-tier proxy makes it accessible without an API key. (see brainstorm: docs/brainstorms/2026-03-14-slidecity-design-brainstorm.md)

### Pure Spectator vs Player Influence
Player influence chosen via brainstorm. The IP economy with tiered interactions adds strategic depth without undermining the autonomous feel. The disaster-for-IP moral hazard creates emergent storytelling. (see brainstorm: docs/brainstorms/2026-03-14-slidecity-design-brainstorm.md)

## System-Wide Impact

### Interaction Graph

```
Game Loop Tick
  → sim::tick() reads grid, writes next_grid, swaps
    → automaton rules transform cells
    → fire spread damages neighbors
  → sim::recompute_utilities() (every 5 ticks)
    → flood-fill power/water networks
    → update has_power/has_water on all cells
  → mayor::decide() (every 8 ticks)
    → assess city stats
    → prioritize actions × personality
    → place blobs/infrastructure → modifies grid
    → push narration → updates mayor log
    → request camera pan → updates camera target
  → audio::select_track() (every 10 ticks)
    → evaluates city state → triggers crossfade
  → influence actions (on player input)
    → deduct IP → evaluate against personality
    → suggestion: probabilistic compliance
    → council: present options, execute choice
    → audience: Claude API call → response → compliance boost
```

### Error Propagation

- Missing sprites → colored rect fallback (never crashes)
- Missing audio files → silent mode (never crashes)
- Claude API timeout → graceful log message ("The mayor is busy")
- Claude API key missing → Direct Audience tier unavailable, other tiers work
- Simulation edge cases (all cells fire) → mayor logs crisis, recovery rules eventually clear rubble
- Save file corrupt → start new game, log warning

### State Lifecycle Risks

- **Double buffer swap**: `std::mem::swap` is atomic from the grid's perspective — no partial state
- **Mayor decision mid-tick**: Mayor runs on tick multiples, not mid-simulation — no race conditions
- **LLM response delay**: Async request, game continues running, response arrives and is queued for next frame
- **Save during tick**: Save at end of tick, never mid-simulation — consistent state guaranteed

### API Surface Parity

- All gameplay is autonomous — player influence is optional layer on top
- All visual features have fallback (sprites → rects, audio → silence, LLM → log message)
- Debug mode exposes same simulation as release, just with additional controls

## Acceptance Criteria

### Functional Requirements

- [ ] City grows autonomously from empty terrain to 500+ population without player input
- [ ] 8 distinct mayor personalities produce visibly different cities
- [ ] Each mayor archetype has unique mechanical powers and weaknesses
- [ ] Player can earn and spend IP through suggestion box, council vote, and direct audience
- [ ] Triggering disasters earns IP (2 IP per disaster triggered)
- [ ] Claude API integration works for Direct Audience with graceful fallback
- [ ] Dynamic music crossfades based on city state
- [ ] Monument moment triggers correctly at pop 500: build, camera pan, sting, narration
- [ ] Start screen allows mayor selection and difficulty choice
- [ ] Game is fully playable with zero art/audio assets (colored rect + silent mode)

### Non-Functional Requirements

- [ ] 60fps at all zoom levels with fully developed 80×52 grid
- [ ] Simulation tick completes in < 5ms for 4,160 cells
- [ ] Claude API response time < 5 seconds for Direct Audience
- [ ] Memory usage < 200MB including loaded textures and audio
- [ ] `cargo build --release` completes in < 60 seconds

### Quality Gates

- [ ] All colored-rect fallbacks visually distinguish every TileType
- [ ] Painter's algorithm sort is visually correct (no Z-fighting or flickering)
- [ ] Camera zoom/pan feels smooth (lerp, not snappy)
- [ ] Mayor narration reads naturally, personality is expressed
- [ ] Fire does not permanently kill a city on Normal difficulty (recovery rules work)

## Success Metrics

- **Monument moment within 15-25 minutes** on Normal difficulty at 1x speed
- **Distinct visual signature** per mayor archetype (The Machine's city looks different from The Environmentalist's)
- **Player engagement loop**: IP earning → spending → mayor response creates "one more turn" feeling
- **Zero-asset playability**: First-time `cargo run` produces a complete, playable experience

## Dependencies & Prerequisites

- Rust 1.90+ stable (available on this machine)
- Macroquad 0.4 (cargo dependency)
- Claude API key for LLM features (optional, game works without it)
- Replicate API key for sprite generation (optional, colored rects work)
- Suno account for music generation (manual process)
- Python 3 + `replicate` SDK for asset generation script

## Risk Analysis & Mitigation

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Retro-diffusion sprites inconsistent | Medium | Medium | Colored rect fallbacks always work; can switch to asset packs |
| Mayor AI feels mechanical | Medium | High | Hard divergence + LLM conversations + ~100 narration strings |
| Simulation balance issues | High | Medium | Difficulty selector + speed slider + tunable parameters |
| Claude API costs for free tier | Low | Low | Rate limit to 3 per session; Haiku is cheap (~$0.001/request) |
| Macroquad 0.4 bugs/limitations | Low | Medium | Well-documented crate; community support; WASM deferred |
| Fire dominates gameplay | Medium | Medium | Difficulty controls fire spread probability; parks as firebreaks for Environmentalist |

## Future Considerations

- **WASM build** for browser-based sharing (nice-to-have, deferred)
- **Multiple save slots** for comparing different mayor runs
- **Replay system** recording decisions for shareable city stories
- **Multiplayer spectator** mode (watch friend's city)
- **More mayor archetypes** unlockable through achievements
- **Seasonal events** (storms, festivals) tied to the year cycle

## Documentation Plan

- README.md with build instructions, gameplay overview, screenshot
- CLAUDE.md at project root (copy from SPEC/CLAUDE.md, update as needed)
- In-game `?debug=true` mode serves as living documentation of simulation parameters

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-14-slidecity-design-brainstorm.md](../brainstorms/2026-03-14-slidecity-design-brainstorm.md) — Key decisions carried forward: hard personality divergence, tiered IP influence system, LLM-powered mayor chat, juicy visuals from day one, dynamic music as core feature.

### Internal References

- Full design spec: `SPEC/SPEC.md`
- Build instructions: `SPEC/CLAUDE.md`

### External References

- [Macroquad 0.4 API Docs](https://docs.rs/macroquad/0.4.5/macroquad/)
- [Macroquad 0.4 Changelog](https://macroquad.rs/articles/macroquad-0-4/)
- [SmallRng documentation](https://docs.rs/rand/latest/rand/rngs/struct.SmallRng.html)
- [Isometric depth sorting reference](https://shaunlebron.github.io/IsometricBlocks/)
- [Claude API Messages endpoint](https://docs.anthropic.com/en/api/messages)
- [Retro Diffusion on Replicate](https://replicate.com/retro-diffusion/rd-plus)

### Related Work

- Linear project: [SlideCity](https://linear.app/techno87/project/slidecity-d56e43b7cacb)
- Git remote: `git@github.com:Goldcap/SlideCity.git`

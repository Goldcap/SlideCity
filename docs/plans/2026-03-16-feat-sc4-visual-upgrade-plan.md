---
title: "feat: SC4-Level Visual Upgrade"
type: feat
status: active
date: 2026-03-16
origin: docs/brainstorms/2026-03-16-sc4-visual-upgrade-brainstorm.md
---

# feat: SC4-Level Visual Upgrade

## Overview

Transform SlideCity from vertex-colored cubes to SimCity 4-level visual quality. Replace flat-colored terrain with a UV-mapped texture atlas (with transition tiles), swap cube buildings for GLTF 3D models, add instanced GLTF trees with LOD, generate a full road network mesh with all intersection types, add middle-click camera pan, and upgrade to 1920x1080.

This is the largest single upgrade since the Bevy migration. It touches terrain rendering, building spawning, a new vegetation system, road mesh generation, camera input, window config, and introduces an external asset pipeline.

## Problem Statement

The game "still just looks like colored boxes" (user feedback). The Bevy 3D migration gave us a 3D engine, shadows, and an orbit camera, but the visual content is placeholder-quality: vertex-colored flat quads for terrain, `Cuboid` primitives for buildings, zero vegetation, and no road geometry. The camera also lacks mouse-only pan control, and the 1280x720 window is undersized for a modern city builder.

## Technical Approach

### Architecture

All rendering code currently lives in `src/main.rs` (~497 lines). Before implementation begins, extract into modules to prevent a 1500+ line monolith:

```
src/
  main.rs              — App setup, plugin registration
  camera.rs            — OrbitCamera component + camera_controls system
  terrain_mesh.rs      — build_terrain_mesh, update_terrain_mesh, atlas UV logic
  building_system.rs   — BuildingModelPool, update_buildings, dirty-cell tracking
  tree_system.rs       — TreeModelPool, tree spawning, LOD system
  road_mesh.rs         — RoadMesh, road network generation, bitmask classification
  asset_loading.rs     — Asset probing, fallback detection, loading state
```

The legacy Macroquad renderer in `src/renderer/` is dead code and will not be touched. The CLAUDE.md contract ("game must work with ZERO art assets") is preserved — every visual system has a fallback path that works without external assets.

**New resource types:**
- `BuildingModelPool` — pre-loaded `Handle<Scene>` for each building type/stage/variant
- `TreeMeshPool` — extracted `Handle<Mesh>` + `Handle<StandardMaterial>` for tree variants (NOT SceneRoot — see Phase 4)
- `RoadMesh` — single combined mesh for the entire road network, with reusable vertex Vecs
- `TerrainAtlasHandle` — `Handle<Image>` for the terrain texture atlas
- `PreviousCellState` — snapshot of visual-relevant cell fields for dirty-cell diffing (shared across terrain, buildings, roads)
- `CubeFallbackHandles` — pre-allocated shared mesh+material handles for cube fallback (one per TileType+stage)

**Bevy SystemSet ordering (CRITICAL):**

Systems must execute in a defined order to avoid one-frame-lag visual artifacts:
```rust
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum GameSet {
    Sim,            // simulation_tick (sim + mayor + utilities)
    DirtyCompute,   // compute dirty cells from PreviousCellState diff
    TerrainRender,  // update_terrain_mesh
    BuildingRender, // update_buildings
    RoadRender,     // update_road_mesh
    TreeRender,     // update_trees + tree_lod_update
}

// In app setup:
app.configure_sets(Update, (
    GameSet::Sim,
    GameSet::DirtyCompute.after(GameSet::Sim),
    GameSet::TerrainRender.after(GameSet::DirtyCompute),
    GameSet::BuildingRender.after(GameSet::DirtyCompute),
    GameSet::RoadRender.after(GameSet::DirtyCompute),
    GameSet::TreeRender.after(GameSet::DirtyCompute),
));
```

**Shared dirty-cell tracking:**

A single `PreviousCellState` resource stores visual-relevant fields per cell. After ALL tick mutations (sim + mayor + utilities), diff against current grid. Only fields that affect rendering are compared — NOT raw `age` (which increments every tick and would mark all cells dirty):

```rust
#[derive(Resource)]
struct PreviousCellState {
    /// (tile, age_stage, style) per cell — only these affect visual output
    cells: Vec<(TileType, u8, u8)>,
    /// Cells that changed this tick — consumed by terrain, building, road systems
    dirty: HashSet<(usize, usize)>,
}
```

Age stage thresholds: 0-15 (stage 0), 16-45 (stage 1), 46+ (stage 2). A cell is visually dirty only when `tile`, `style`, or age STAGE changes.

The diff runs AFTER `simulation_tick` (which includes sim::tick swap + mayor mutations + utility recomputation) to capture all changes.

**Fallback contract (Bevy 3D):**
| Asset Category | Fallback (no assets) | With assets |
|---------------|---------------------|-------------|
| Terrain | Vertex colors (current) | UV-mapped texture atlas |
| Buildings | Colored cubes (current) | GLTF models per type/stage/variant |
| Trees | Nothing (current) | Instanced GLTF models with LOD |
| Roads | Flat colored terrain cells (current) | Combined road network mesh with texture |

### Implementation Phases

#### Phase 1: Window + Mouse Controls (Quick Wins)

**Window resize** — Change `resolution: (1280.0, 720.0).into()` to `(1920.0, 1080.0).into()` in `src/main.rs:57`.

**Middle-click pan** — Add to `camera_controls` system in `src/main.rs:181`:
```rust
if mouse_button.pressed(MouseButton::Middle) {
    let forward = Vec3::new(-orbit.yaw.sin(), 0.0, -orbit.yaw.cos());
    let right = Vec3::new(orbit.yaw.cos(), 0.0, -orbit.yaw.sin());
    let sensitivity = orbit.distance * 0.003; // scale pan speed with zoom
    orbit.target += right * (-delta.x * sensitivity) + forward * (delta.y * sensitivity);
}
```

**Camera target clamping** — Clamp `orbit.target` to grid bounds:
```rust
let max_x = grid.width as f32 * TILE_SIZE;
let max_z = grid.height as f32 * TILE_SIZE;
orbit.target.x = orbit.target.x.clamp(0.0, max_x);
orbit.target.z = orbit.target.z.clamp(0.0, max_z);
```

**Input priority:** Any manual camera input (middle-click, WASD, right-click, scroll) cancels mayor auto-pan.

**Files changed:** `src/main.rs`

**Success criteria:**
- [x] Window opens at 1920x1080
- [x] Middle-click drag pans camera in world space, scaled by zoom distance
- [x] Camera cannot pan beyond grid boundaries
- [x] Middle-click pan + WASD both work, no conflicts
- [x] Right-click rotate still works independently
- [ ] Mayor auto-pan canceled by any manual input

---

#### Phase 2: Terrain Texture Atlas

The biggest visual improvement. Replace vertex colors with UV-mapped textures.

**Atlas layout:** 2048x2048 texture, 128x128 pixels per tile = 16x16 tile grid = 256 slots.

**Tile allocation in atlas:**

| Row | Tiles | Count |
|-----|-------|-------|
| 0 | Base terrain: Grass, GrassFlower, Trees, TreesSparse, Sand, Dirt, Rock, Snow | 8 |
| 1-4 | Transition tiles (2-type blends): Grass-Sand, Grass-Dirt, Grass-Rock, Sand-Rock, Sand-Dirt, Dirt-Rock, Rock-Snow, Snow-Grass, etc. | ~48 |
| 5-6 | Zone overlays: Residential, Commercial, Industrial, Park, PowerPlant, WaterTower, Monument, Road base | ~16 |
| 7+ | Reserved for future | — |

**Transition tile selection algorithm:**

For each cell, examine 4 cardinal neighbors. Use a **priority system** for terrain types:
```
Snow(7) > Rock(6) > Sand(5) > Dirt(4) > Trees(3) > TreesSparse(2) > GrassFlower(1) > Grass(0)
```

Each cell edge that borders a different terrain type gets a transition tile. For the 3+ type corner problem, the highest-priority adjacent type wins.

**Transition UV rotation:** Use a 4-bit edge mask (same pattern as road bitmask in Phase 5) to determine which edges border different terrain types. The mask selects both the atlas tile AND its UV rotation:
- 1 edge different: transition tile rotated to face that edge
- 2 adjacent edges: corner transition
- 2 opposite edges: strip transition
- 3-4 edges: isolated cell, use base tile with blended border

Extract the bitmask-to-configuration logic into a shared utility (`src/bitmask_config.rs`) reused by both terrain transitions and road classification.

**UV mapping per cell:**

Each cell's 4 vertices get UVs that index into the atlas. For a tile at atlas position (col, row):
```rust
let u_min = col as f32 / 16.0;
let v_min = row as f32 / 16.0;
let u_max = u_min + 1.0 / 16.0;
let v_max = v_min + 1.0 / 16.0;
// TL, TR, BL, BR
uvs: [[u_min, v_min], [u_max, v_min], [u_min, v_max], [u_max, v_max]]
```

**Mesh changes in `build_terrain_mesh`:**
1. Add `ATTRIBUTE_UV_0` (Vec<[f32; 2]>) alongside existing position/normal/color
2. Compute UVs per cell based on terrain type + neighbor transitions
3. Keep `ATTRIBUTE_COLOR` as fallback tint (set to white when atlas is loaded)

**Material changes:**
- When atlas is loaded: `StandardMaterial { base_color_texture: Some(atlas_handle), perceptual_roughness: 0.9, ..default() }`
- When no atlas: Keep current vertex-color material (white base + ATTRIBUTE_COLOR)

**Update path changes in `update_terrain_mesh`:**
- Must now update both `ATTRIBUTE_UV_0` and `ATTRIBUTE_COLOR` when grid changes
- UVs change when a cell's tile type changes (e.g., empty -> residential)

**Atlas image creation:**
- Provide a Python/Rust script to compose the atlas from individual tile PNGs
- Tile PNGs sourced externally (Kenney, OpenGameArt) or hand-painted
- Script outputs `assets/textures/terrain_atlas.png`

**Files changed:** `src/main.rs` (build_terrain_mesh, update_terrain_mesh, setup, terrain material)

**New files:** `assets/textures/terrain_atlas.png` (downloaded), `scripts/build_atlas.py` (committed)

**Success criteria:**
- [x] Terrain renders with textures when atlas exists in `assets/textures/`
- [x] Terrain falls back to vertex colors when atlas is missing
- [x] Transition tiles appear at terrain type boundaries
- [x] 3+ type corners resolve cleanly via priority system
- [x] Zone placement updates terrain UVs correctly
- [ ] No visible seams between tiles (seamless atlas tiles)
- [ ] Performance: terrain mesh rebuild < 5ms on 128x128 grid

---

#### Phase 3: GLTF Building Models

Replace `Cuboid` primitives with proper 3D models.

**Asset directory structure:**
```
assets/models/buildings/
  residential/
    s1_v1.glb  s1_v2.glb  s1_v3.glb  s1_v4.glb  s1_v5.glb  s1_v6.glb
    s2_v1.glb  s2_v2.glb  s2_v3.glb  s2_v4.glb  s2_v5.glb  s2_v6.glb
    s3_v1.glb  s3_v2.glb  s3_v3.glb  s3_v4.glb  s3_v5.glb  s3_v6.glb
  commercial/
    s1_v1.glb ... s3_v6.glb
  industrial/
    s1_v1.glb ... s3_v6.glb
  infrastructure/
    power_plant.glb
    water_tower.glb
    monument.glb
```

**Stage mapping:** s1 = age 0-15 (low density), s2 = age 16-45 (medium), s3 = age 46+ (high density). Variant = `cell.style % variant_count`.

**Model conventions:**
- Origin at ground center of footprint
- Y-up, facing +Z
- Scale: 1 unit = 1 TILE_SIZE (models designed for ~0.7x0.7 footprint)
- Materials embedded in GLB (from source asset packs)
- **Asset budget per model:** max 2,000 triangles, max 512x512 texture (keeps ~54 models under 50 MB VRAM total)

**Pre-loading (setup system):**
```rust
#[derive(Resource)]
struct BuildingModelPool {
    models: HashMap<(TileType, u8, u8), Handle<Scene>>,  // (type, stage, variant)
    loaded: bool,
}
```
Load all GLBs at startup via `AssetServer::load()`. Mark `loaded = true` once all handles report ready. Until loaded, fall back to cubes.

**Incremental building updates (replacing full despawn/respawn):**

Use the shared `PreviousCellState.dirty` set (computed once per tick in `GameSet::DirtyCompute`). Only despawn/respawn buildings for dirty cells. This reduces per-tick entity operations from O(grid_size) to O(changed_cells).

**CRITICAL: Pre-allocate cube fallback handles.** The current code creates unique `meshes.add()` and `materials.add()` per building per tick — leaking ~10,000 asset handles per tick. Fix by pre-creating shared handles at startup:
```rust
#[derive(Resource)]
struct CubeFallbackHandles {
    meshes: HashMap<(TileType, u8), Handle<Mesh>>,      // (type, stage) -> shared cuboid
    materials: HashMap<TileType, Handle<StandardMaterial>>, // type -> shared material
}
```
All cube fallback buildings reuse these handles, enabling Bevy auto-batching.

**Building entity structure:**
```rust
commands.spawn((
    SceneRoot(model_handle.clone()),
    Transform::from_xyz(world_x, terrain_y, world_z)
        .with_scale(Vec3::splat(scale)),
    BuildingMarker { col, row },
));
```

**Shadow optimization:** Use Bevy's built-in `DirectionalLight` cascade shadow distance configuration instead of per-entity shadow toggling. Per-entity component add/remove causes expensive archetype thrashing in Bevy's ECS. Configure cascade distances in the light setup:
```rust
DirectionalLight {
    shadows_enabled: true,
    shadow_depth_bias: 0.02,
    // Bevy's cascaded shadow maps automatically cull beyond cascade distance
    ..default()
}
```

**Files changed:** `src/building_system.rs` (new), `src/asset_loading.rs` (new), `src/main.rs` (setup)

**Success criteria:**
- [ ] GLTF models load and display for each building type/stage
- [x] Falls back to colored cubes when GLBs are missing
- [x] Correct model selected based on TileType + age stage + style variant
- [x] Incremental updates: only changed cells despawn/respawn
- [x] Models positioned correctly on terrain (Y follows terrain height)
- [ ] Performance: building update < 2ms per sim tick with ~5,000 buildings
- [ ] Shadow distance culling works

---

#### Phase 4: Tree Models + Instancing

Populate forest terrain cells with 3D tree models.

**Asset structure:**
```
assets/models/trees/
  tree_v1.glb  tree_v2.glb  tree_v3.glb  tree_v4.glb
  tree_v1_lod1.glb  tree_v2_lod1.glb  (simplified versions)
```

**Spawning strategy:**
- Each `TerrainType::Trees` cell spawns 2-3 tree entities at randomized positions within the cell
- Each `TerrainType::TreesSparse` cell spawns 1 tree
- Use `SmallRng` seeded by `(col, row)` for deterministic placement
- Variant selected by `(col * 7 + row * 13) % variant_count`

**GPU instancing (IMPORTANT — do NOT use SceneRoot for trees):**

Bevy 0.15 auto-instances entities sharing the same `Handle<Mesh>` + `Handle<StandardMaterial>`. However, `SceneRoot` spawns child entity hierarchies where instancing is unreliable. Instead, **extract mesh and material handles from loaded GLTFs at startup** and spawn trees as flat entities:

```rust
#[derive(Resource)]
struct TreeMeshPool {
    /// Extracted from GLTF at load time — NOT SceneRoot handles
    variants: Vec<(Handle<Mesh>, Handle<StandardMaterial>)>,       // LOD0 (full)
    variants_lod1: Vec<(Handle<Mesh>, Handle<StandardMaterial>)>,  // LOD1 (simplified)
    loaded: bool,
}

// Tree spawn — flat entity, guaranteed instancing:
commands.spawn((
    Mesh3d(tree_mesh.clone()),
    MeshMaterial3d(tree_material.clone()),
    Transform::from_xyz(x, y, z),
    TreeMarker { col, row },
));
```

This guarantees Bevy batches all trees of the same variant into a single draw call. With 4 variants x 2 LOD levels = ~8 draw calls for ALL trees.

**Manual LOD system:**
```rust
#[derive(Component)]
struct TreeLod {
    full_mesh: Handle<Mesh>,
    full_mat: Handle<StandardMaterial>,
    lod1_mesh: Handle<Mesh>,
    lod1_mat: Handle<StandardMaterial>,
    threshold: f32,       // 80.0 units
    despawn_distance: f32, // 150.0 units
}
```

LOD update system runs every 500ms via `Timer` resource with `TimerMode::Repeating`:
- Distance < 80: swap to full mesh/material handles
- Distance 80-150: swap to LOD1 mesh/material handles
- Distance > 150: `Visibility::Hidden`

LOD switching is a simple component write (`Mesh3d` handle swap), not a scene hierarchy teardown.

**Tree entity tracking:** Trees are tied to terrain type, not tile type. They persist as long as the cell's terrain type is Trees/TreesSparse. Track with `TreeMarker { col, row }` component. Only respawn when terrain type changes (rare — mostly at game start).

**Fallback:** When tree GLBs are missing, spawn nothing (current behavior). Trees are decorative, not gameplay-critical.

**Files changed:** `src/tree_system.rs` (new), `src/main.rs` (setup)

**Success criteria:**
- [ ] Trees spawn on Trees/TreesSparse terrain cells
- [ ] Multiple trees per cell with randomized positions
- [ ] LOD switches at distance thresholds
- [ ] Trees hidden beyond 150 units from camera
- [ ] Bevy auto-instances identical tree meshes (verify with diagnostics)
- [ ] Performance: < 1ms per frame for tree LOD updates
- [ ] Trees despawn when terrain type changes (zone placement on forest)

---

#### Phase 5: Road Network Mesh

The most complex geometry system. Generate a continuous road mesh from the grid.

**Road cell classification:**

Use a 4-bit bitmask from cardinal neighbors: `N(8) | E(4) | S(2) | W(1)`:

| Bitmask | Config | Description |
|---------|--------|-------------|
| 0000 | Isolated | Single road cell, no neighbors |
| 0001-0008 | Dead-end | One neighbor (4 rotations) |
| 0011, 0110, 1100, 1001 | Curve | Two adjacent neighbors (4 rotations) |
| 0101, 1010 | Straight | Two opposite neighbors (2 rotations) |
| 0111, 1011, 1101, 1110 | T-junction | Three neighbors (4 rotations) |
| 1111 | 4-way | All four neighbors |

= 16 configurations, each mapping to a mesh piece.

**Mesh generation approach:** Single combined mesh (like terrain). Full rebuild from scratch when any road cell changes (NOT partial mesh patching — Bevy can't partially update mesh buffers, and full rebuild of 2K cells is <1ms).

```rust
#[derive(Resource)]
struct RoadMesh {
    handle: Handle<Mesh>,
    entity: Entity,
    /// Reusable buffers to avoid per-rebuild allocation
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}
```

**Road mesh is an additive overlay:** The terrain mesh ALWAYS colors road cells with the road color (current behavior is preserved). The road mesh entity sits on top as a visual enhancement. If the road mesh system fails or is disabled, roads remain visible as colored terrain cells.

**Road mesh pieces:** Pre-define vertex data for each of the 16 configurations as arrays. For each road cell:
1. Determine bitmask from neighbors
2. Select the corresponding mesh piece
3. Rotate vertices based on orientation
4. Offset to cell world position
5. Set Y to `cell.terrain_height * HEIGHT_SCALE + 0.05` (slightly above terrain)

**Road-terrain interaction:**
- Roads sit 0.05 units above terrain surface (no z-fighting)
- Terrain height is NOT flattened under roads (keeps terrain variation visible)
- Road mesh follows terrain height at each cell

**Road texture:**
- Single road material with `base_color_texture` (asphalt + lane markings baked into texture)
- UV coordinates per mesh piece map to road atlas (straight, curve, intersection sections)
- Fallback: dark gray `StandardMaterial` with no texture

**Edge cases:**
- Isolated road cell: renders as a small platform/roundabout
- Roads at grid edge: dead-end cap facing inward
- Adjacent to water: normal road, no bridge (future feature)
- Diagonal adjacency: no visual connection (only cardinal)

**Files changed:** `src/road_mesh.rs` (new), `src/bitmask_config.rs` (shared with terrain transitions), `src/main.rs` (setup)

**Success criteria:**
- [ ] All 16 road configurations render correctly
- [ ] Road mesh rebuilds incrementally when roads are placed/removed
- [ ] Roads sit slightly above terrain (no z-fighting)
- [ ] Road texture applied when available, dark gray fallback otherwise
- [ ] T-junctions, 4-way intersections, and curves look correct
- [ ] Performance: road mesh rebuild < 3ms for ~2,000 road cells
- [ ] Dead-ends and isolated roads have visual caps

---

### Asset Pipeline

**Download script:** `scripts/download_assets.sh` (committed to repo)

```bash
#!/bin/bash
# Downloads game assets from release hosting
ASSET_URL="https://github.com/Goldcap/SlideCity/releases/download/assets-v1"
ASSET_DIR="assets"
mkdir -p "$ASSET_DIR/models/buildings" "$ASSET_DIR/models/trees" "$ASSET_DIR/textures"
# Download and extract asset pack
curl -L "$ASSET_URL/slidecity-assets-v1.tar.gz" | tar xz -C "$ASSET_DIR"
echo "Assets downloaded to $ASSET_DIR/"
```

**Asset versioning:** Tag asset releases separately from code releases (e.g., `assets-v1`). The game checks for asset presence at startup and logs which assets are available.

**`.gitignore` additions:**
```
assets/models/
assets/textures/terrain_atlas.png
```

## Alternative Approaches Considered

(see brainstorm: docs/brainstorms/2026-03-16-sc4-visual-upgrade-brainstorm.md)

| Alternative | Why Rejected |
|------------|-------------|
| Procedural terrain textures (shaders) | Doesn't give the hand-crafted SC4 tile aesthetic |
| Procedural building geometry | Can't match the quality of real 3D models |
| Billboard trees | Looks flat up close, doesn't match GLTF building quality |
| Simple road strips | SC4's road network was a defining visual feature |
| Assets in git repo | Binary bloat; separate download keeps repo lean |
| Per-cell building entities | Current approach, but must switch to incremental updates |

## System-Wide Impact

### Interaction Graph

1. Grid change (sim tick) -> `update_terrain_mesh` (updates UVs + colors) -> `update_buildings` (dirty-cell diff) -> `update_road_mesh` (dirty road cells) -> `update_trees` (only on terrain type change)
2. Asset loading (startup) -> `AssetServer::load()` async -> `BuildingModelPool.loaded = true` -> switch from cube fallback to GLTF models
3. Camera input (every frame) -> `camera_controls` reads middle-click + existing inputs -> updates `OrbitCamera` transform -> `tree_lod_update` reads camera position

### State Lifecycle Risks

- **Partial asset loading:** GLTF handles may be `Loading` for several frames after startup. The `BuildingModelPool.loaded` flag prevents spawning scenes from incomplete handles.
- **Grid diff accuracy:** The dirty-cell tracking must capture ALL cell changes per tick, including indirect changes (fire spread, growth). The sim already mutates `Grid` as a `Resource`, so `grid_res.is_changed()` catches everything — but dirty tracking needs to compare old vs new cell state, not just detect resource mutation.

### API Surface Parity

- Save/load: No schema changes needed. Building models are selected from `TileType + age + style` which are already saved. Road mesh is regenerated from grid state on load.
- The `style` field on `Cell` (u8, 0-3) was designed for variant selection and is already persisted.

## Acceptance Criteria

### Functional Requirements

- [x] **P1:** Window opens at 1920x1080
- [x] **P1:** Middle-click drag pans camera, works alongside WASD/right-drag/scroll
- [x] **P1:** Camera target clamped to grid bounds
- [x] **P2:** Terrain renders with texture atlas when available, vertex colors as fallback
- [x] **P2:** Terrain transition tiles at type boundaries
- [x] **P3:** GLTF building models per type/stage/variant, cube fallback
- [x] **P3:** Incremental building updates (no full despawn/respawn)
- [ ] **P4:** GLTF trees on forest cells with LOD
- [ ] **P5:** Full road network mesh with all 16 configurations
- [ ] **P5:** Road texture when available, gray fallback

### Non-Functional Requirements

- [ ] Terrain mesh rebuild < 5ms on 128x128 grid
- [ ] Building update < 2ms per sim tick with ~5,000 buildings
- [ ] Tree LOD update < 1ms per cycle
- [ ] Road mesh rebuild < 3ms for ~2,000 road cells
- [ ] 60fps at 1920x1080 with full assets on mid-range GPU
- [ ] Game launches and plays with zero external assets

### Quality Gates

- [ ] `cargo clippy` passes with no warnings
- [ ] Game runs on Linux, Windows, macOS (CI builds)
- [ ] Save/load works correctly with new visual systems
- [ ] No visual artifacts: z-fighting, seams, T-junctions, pop-in

## Dependencies & Prerequisites

- **Bevy 0.15** (already in Cargo.toml)
- **GLTF model assets** from Kenney CC0 + OpenGameArt/itch.io (curated for style)
- **Terrain tile textures** (sourced or hand-painted, composed into atlas)
- **Road texture** (asphalt + lane markings, sourced or painted)
- No new crate dependencies needed — Bevy 0.15 handles GLTF, textures, instancing natively

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| GLTF models inconsistent style across packs | High | Medium | Curate carefully, normalize scale/origin before use. Asset budget: 2K tris, 512x512 tex max |
| Building despawn/respawn performance with GLTF | High | Critical | Shared dirty-cell tracking + pre-allocated fallback handles |
| Tree SceneRoot instancing failure | High | High | **MITIGATED:** Use flat Mesh3d entities with extracted handles, not SceneRoot |
| System ordering race conditions | High | Critical | **MITIGATED:** Explicit SystemSet with .before()/.after() constraints |
| Age increment marks all cells dirty | High | Critical | **MITIGATED:** Diff only visual fields (tile, style, age_stage), not raw age |
| Tree entity count exceeds perf budget | Medium | High | Visibility culling + LOD + cap at ~15K entities |
| Terrain transition tile combos exponential | Medium | Medium | Priority system + shared bitmask utility for UV rotation |
| Road mesh generation complexity | Medium | Medium | 16-config lookup table, full rebuild (<1ms for 2K cells) |
| Asset download UX friction | Medium | Low | Game works without assets; download is optional enhancement |
| Shadow archetype thrashing | Medium | Medium | **MITIGATED:** Use cascade shadow distance, not per-entity toggling |

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-16-sc4-visual-upgrade-brainstorm.md](docs/brainstorms/2026-03-16-sc4-visual-upgrade-brainstorm.md)
- Key decisions carried forward: texture atlas with transitions, GLTF models from Kenney+OpenGameArt, full road network mesh, assets outside git

### Internal References

- Terrain mesh: `src/main.rs:247` (`build_terrain_mesh`)
- Building system: `src/main.rs:449` (`update_buildings`)
- Camera controls: `src/main.rs:181` (`camera_controls`)
- Grid types: `src/grid/mod.rs` (TileType, TerrainType, Cell)
- Legacy sprite mapping: `src/renderer/sprites.rs` (reference for tile-to-asset mapping)
- CLAUDE.md: `SPEC/CLAUDE.md` (zero-asset fallback requirement)

### External References

- [Bevy 0.15 GLTF loading](https://bevy-cheatbook.github.io/3d/gltf)
- [Bevy 0.15 custom mesh generation](https://bevy.org/examples/3d-rendering/generate-custom-mesh/)
- [Bevy auto-instancing](https://bevy.org/examples/shaders/automatic-instancing/)
- [Bevy pan-orbit camera cookbook](https://bevy-cheatbook.github.io/cookbook/pan-orbit-camera)
- [Kenney City Kit](https://kenney.nl/assets/city-kit-suburban) (CC0 building models)
- [Kenney Nature Kit](https://kenney.nl/assets/nature-kit) (CC0 tree models)

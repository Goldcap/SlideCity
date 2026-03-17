mod config;
mod grid;
mod influence;
mod mayor;
mod sim;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

use config::SimConfig;
use grid::terrain::generate_terrain;
use grid::{Grid, TileType, TerrainType};
use mayor::Mayor;
use sim::stats::CityStats;
use ::rand::rngs::SmallRng;
use ::rand::SeedableRng;

// ===== RESOURCES =====

#[derive(Resource)]
struct GameGrid {
    grid: Grid,
    next_grid: Grid,
}

#[derive(Resource)]
struct GameState {
    config: SimConfig,
    rng: SmallRng,
    mayor: Mayor,
    funds: i64,
    tick_count: u64,
    tick_timer: f32,
    stats: CityStats,
    speed_idx: usize,
    speed_levels: [f32; 4],
}

#[derive(Resource)]
struct TerrainMeshHandle(Handle<Mesh>);

/// Whether the terrain texture atlas is loaded and active.
#[derive(Resource)]
struct TerrainAtlas {
    loaded: bool,
}

/// Snapshot of visual-relevant cell fields for dirty-cell diffing.
/// Shared across terrain, building, and road update systems.
#[derive(Resource)]
struct PreviousCellState {
    /// (tile, age_stage, style) per cell — only these affect visual output
    cells: Vec<(TileType, u8, u8)>,
    /// Cells that changed this tick — consumed by rendering systems
    dirty: HashSet<(usize, usize)>,
}

impl PreviousCellState {
    fn from_grid(grid: &Grid) -> Self {
        let cells = grid.cells.iter().map(|c| {
            (c.tile, age_stage(c.age), c.style)
        }).collect();
        Self { cells, dirty: HashSet::new() }
    }
}

/// Map age to visual stage: 0-15 -> 0, 16-45 -> 1, 46+ -> 2
fn age_stage(age: u8) -> u8 {
    if age < 16 { 0 } else if age < 46 { 1 } else { 2 }
}

/// Pre-allocated shared mesh and material handles for cube fallback buildings.
/// Eliminates the 10K+ handle-per-tick leak in the old update_buildings.
#[derive(Resource)]
struct CubeFallbackHandles {
    /// (tile_type, stage) -> shared cuboid mesh handle
    meshes: HashMap<(TileType, u8), Handle<Mesh>>,
    /// tile_type -> shared material handle
    materials: HashMap<TileType, Handle<StandardMaterial>>,
}

/// Pre-loaded GLTF scene handles for building models.
/// Models are organized by zone type, with indices picked by cell style/stage.
#[derive(Resource)]
struct BuildingModelPool {
    /// Zone type -> list of scene handles (small buildings first, large last)
    residential: Vec<Handle<Scene>>,
    commercial: Vec<Handle<Scene>>,
    commercial_large: Vec<Handle<Scene>>, // skyscrapers for stage 2
    industrial: Vec<Handle<Scene>>,
    /// True if any model files were found at startup
    has_models: bool,
    /// Set to true once models are confirmed loaded — triggers a full building rebuild
    models_ready: bool,
}

/// Road network mesh entity + reusable buffers.
#[allow(dead_code)]
#[derive(Resource)]
struct RoadMeshRes {
    handle: Handle<Mesh>,
    entity: Entity,
}

/// Pre-loaded tree scene handles. Uses Kenney GLTF models when available,
/// falls back to procedural cone meshes.
#[derive(Resource)]
struct TreeModelPool {
    /// Scene handles for GLTF tree models (SceneRoot spawning)
    scenes: Vec<Handle<Scene>>,
    /// Fallback: procedural cone mesh+material handles (Mesh3d spawning)
    procedural: Vec<(Handle<Mesh>, Handle<StandardMaterial>)>,
    /// True when using GLTF models
    use_gltf: bool,
}

#[derive(Component)]
struct TreeMarker {
    col: usize,
    row: usize,
}

/// Timer for LOD updates (not every frame).
#[derive(Resource)]
struct TreeLodTimer(Timer);

// ===== SYSTEM SETS =====

/// Execution order for game systems to prevent one-frame-lag visual artifacts.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum GameSet {
    Sim,
    DirtyCompute,
    Render,
}

// ===== CONSTANTS =====

const TILE_SIZE: f32 = 1.0; // World units per grid cell
const HEIGHT_SCALE: f32 = 3.0; // Terrain height exaggeration
const ATLAS_TILE_UV: f32 = 1.0 / 16.0; // UV size of one tile (16x16 grid)

// ===== ATLAS TILE MAPPING =====

/// Returns the (col, row) position in the atlas for a terrain type (row 0).
fn terrain_atlas_index(terrain: TerrainType) -> (u32, u32) {
    let col = match terrain {
        TerrainType::Grass => 0,
        TerrainType::GrassFlower => 1,
        TerrainType::Trees => 2,
        TerrainType::TreesSparse => 3,
        TerrainType::Sand => 4,
        TerrainType::Dirt => 5,
        TerrainType::Rock => 6,
        TerrainType::Snow => 7,
    };
    (col, 0)
}

/// Returns the (col, row) position in the atlas for a zone tile type (row 5).
fn zone_atlas_index(tile: TileType) -> Option<(u32, u32)> {
    let col = match tile {
        TileType::Residential => 0,
        TileType::Commercial => 1,
        TileType::Industrial => 2,
        TileType::Park => 3,
        TileType::PowerPlant => 4,
        TileType::WaterTower => 5,
        TileType::Monument => 6,
        TileType::Road => 7,
        _ => return None,
    };
    Some((col, 5))
}

/// Terrain type priority for transition tile selection (higher = wins).
fn terrain_priority(t: TerrainType) -> u8 {
    match t {
        TerrainType::Grass => 0,
        TerrainType::GrassFlower => 1,
        TerrainType::TreesSparse => 2,
        TerrainType::Trees => 3,
        TerrainType::Dirt => 4,
        TerrainType::Sand => 5,
        TerrainType::Rock => 6,
        TerrainType::Snow => 7,
    }
}

/// Transition pair index in the atlas. Returns (pair_index, direction_offset).
/// Transition tiles are laid out in rows 1-4, 4 tiles per pair (N/E/S/W).
fn transition_pair_index(a: TerrainType, b: TerrainType) -> Option<usize> {
    // Order: low-priority type is "base", high-priority is "overlay"
    let (base, overlay) = if terrain_priority(a) < terrain_priority(b) {
        (a, b)
    } else {
        (b, a)
    };
    // Match against the known transition pairs (must match Python generator order)
    let pairs: &[(TerrainType, TerrainType)] = &[
        (TerrainType::Grass, TerrainType::Sand),
        (TerrainType::Grass, TerrainType::Dirt),
        (TerrainType::Grass, TerrainType::Rock),
        (TerrainType::Sand, TerrainType::Dirt),
        (TerrainType::Sand, TerrainType::Rock),
        (TerrainType::Dirt, TerrainType::Rock),
        (TerrainType::Rock, TerrainType::Snow),
        (TerrainType::Snow, TerrainType::Grass),
        (TerrainType::Grass, TerrainType::Trees),
        (TerrainType::Trees, TerrainType::Dirt),
        (TerrainType::Sand, TerrainType::Snow),
        (TerrainType::Dirt, TerrainType::Snow),
    ];
    pairs.iter().position(|&(pa, pb)| {
        (pa == base && pb == overlay) || (pb == base && pa == overlay)
    })
}

/// Compute the atlas tile (col, row) for a cell, considering neighbors for transitions.
#[allow(unused_assignments)]
fn cell_atlas_tile(grid: &Grid, col: usize, row: usize) -> (u32, u32) {
    let cell = grid.get(col, row);

    // WaterBody cells don't use the atlas
    if cell.tile == TileType::WaterBody {
        return (0, 0); // Will be hidden by water plane anyway
    }

    // Developed cells use zone overlay tiles
    if cell.tile != TileType::Empty {
        if let Some(idx) = zone_atlas_index(cell.tile) {
            return idx;
        }
        // Fallback for tiles without a zone atlas entry (PowerLine, WaterMain, Fire, Rubble)
        return terrain_atlas_index(cell.terrain_type);
    }

    // Empty cells: check neighbors for transitions
    let my_terrain = cell.terrain_type;
    let my_priority = terrain_priority(my_terrain);

    // Check cardinal neighbors for different terrain types
    let mut highest_neighbor: Option<TerrainType> = None;
    let mut highest_priority = my_priority;
    let mut transition_edge: u8 = 0; // bitmask: N=8, E=4, S=2, W=1

    // North
    if row > 0 {
        let n = grid.get(col, row - 1);
        if n.tile == TileType::Empty && n.terrain_type != my_terrain {
            let p = terrain_priority(n.terrain_type);
            if p > highest_priority {
                highest_priority = p;
                highest_neighbor = Some(n.terrain_type);
            }
            transition_edge |= 8;
        }
    }
    // East
    if col + 1 < grid.width {
        let n = grid.get(col + 1, row);
        if n.tile == TileType::Empty && n.terrain_type != my_terrain {
            let p = terrain_priority(n.terrain_type);
            if p > highest_priority {
                highest_priority = p;
                highest_neighbor = Some(n.terrain_type);
            }
            transition_edge |= 4;
        }
    }
    // South
    if row + 1 < grid.height {
        let n = grid.get(col, row + 1);
        if n.tile == TileType::Empty && n.terrain_type != my_terrain {
            let p = terrain_priority(n.terrain_type);
            if p > highest_priority {
                highest_priority = p;
                highest_neighbor = Some(n.terrain_type);
            }
            transition_edge |= 2;
        }
    }
    // West
    if col > 0 {
        let n = grid.get(col - 1, row);
        if n.tile == TileType::Empty && n.terrain_type != my_terrain {
            let p = terrain_priority(n.terrain_type);
            if p > highest_priority {
                highest_priority = p;
                highest_neighbor = Some(n.terrain_type);
            }
            transition_edge |= 1;
        }
    }

    // If no significant neighbor transition, use base tile
    if highest_neighbor.is_none() || transition_edge == 0 {
        return terrain_atlas_index(my_terrain);
    }

    let neighbor = highest_neighbor.unwrap();

    // Find the transition pair in the atlas
    if let Some(pair_idx) = transition_pair_index(my_terrain, neighbor) {
        // Direction offset: N=0, E=1, S=2, W=3
        // Pick the primary transition direction (first set bit)
        let dir_offset = if transition_edge & 8 != 0 {
            0 // North
        } else if transition_edge & 4 != 0 {
            1 // East
        } else if transition_edge & 2 != 0 {
            2 // South
        } else {
            3 // West
        };

        // Transition tiles start at row 1, 4 tiles per pair
        let flat_idx = pair_idx * 4 + dir_offset;
        let atlas_col = (flat_idx % 16) as u32;
        let atlas_row = 1 + (flat_idx / 16) as u32;
        return (atlas_col, atlas_row);
    }

    // No matching transition pair in atlas — use base tile
    terrain_atlas_index(my_terrain)
}

/// Convert an atlas tile (col, row) to UV coordinates for a cell's 4 vertices.
/// Returns [TL, TR, BL, BR] UV pairs.
fn atlas_uvs(atlas_col: u32, atlas_row: u32) -> [[f32; 2]; 4] {
    let u_min = atlas_col as f32 * ATLAS_TILE_UV;
    let v_min = atlas_row as f32 * ATLAS_TILE_UV;
    let u_max = u_min + ATLAS_TILE_UV;
    let v_max = v_min + ATLAS_TILE_UV;
    [
        [u_min, v_min], // TL
        [u_max, v_min], // TR
        [u_min, v_max], // BL
        [u_max, v_max], // BR
    ]
}

// ===== MAIN =====

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "SlideCity".to_string(),
                resolution: (1920.0, 1080.0).into(),
                ..default()
            }),
            ..default()
        }))
        .configure_sets(Update, (
            GameSet::Sim,
            GameSet::DirtyCompute.after(GameSet::Sim),
            GameSet::Render.after(GameSet::DirtyCompute),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (
            camera_controls,
            simulation_tick.in_set(GameSet::Sim),
            compute_dirty_cells.in_set(GameSet::DirtyCompute),
            update_terrain_mesh.in_set(GameSet::Render),
            check_models_loaded.in_set(GameSet::Render),
            update_buildings.in_set(GameSet::Render).after(check_models_loaded),
            spawn_trees.in_set(GameSet::Render),
            update_road_mesh.in_set(GameSet::Render),
            tree_lod_update,
        ))
        .run();
}

// ===== SETUP =====

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let config = SimConfig::default();
    let mut rng = SmallRng::seed_from_u64(42);
    let grid = generate_terrain(config.grid_width, config.grid_height, &mut rng);
    let next_grid = grid.clone();
    let stats = CityStats::compute(&grid);
    let mayor = Mayor::new(0);
    let funds = config.starting_funds;

    // Check for terrain atlas texture
    let atlas_path = "textures/terrain_atlas.png";
    let atlas_exists = Path::new("assets").join(atlas_path).exists();
    let use_atlas = atlas_exists;

    if atlas_exists {
        info!("Terrain atlas found — using textured terrain");
    } else {
        info!("No terrain atlas at assets/{} — using vertex color fallback", atlas_path);
    }

    // Generate terrain mesh (with UVs if atlas exists)
    let terrain_mesh = build_terrain_mesh(&grid, use_atlas);
    let mesh_handle = meshes.add(terrain_mesh);

    // Terrain material: textured if atlas available, vertex-colored otherwise
    let terrain_material = if use_atlas {
        let texture_handle: Handle<Image> = asset_server.load(atlas_path);
        materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(texture_handle),
            perceptual_roughness: 0.9,
            metallic: 0.0,
            ..default()
        })
    } else {
        materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.9,
            metallic: 0.0,
            ..default()
        })
    };

    // Terrain entity
    commands.spawn((
        Mesh3d(mesh_handle.clone()),
        MeshMaterial3d(terrain_material),
        Transform::default(),
    ));

    // Water plane (at sea level)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(
            config.grid_width as f32 * TILE_SIZE,
            config.grid_height as f32 * TILE_SIZE,
        ))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.1, 0.3, 0.7, 0.7),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.3,
            metallic: 0.1,
            ..default()
        })),
        Transform::from_xyz(
            config.grid_width as f32 * TILE_SIZE / 2.0,
            0.15, // Slightly above zero
            config.grid_height as f32 * TILE_SIZE / 2.0,
        ),
    ));

    // Sun (directional light)
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(50.0, 80.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ambient light
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.9, 0.95, 1.0),
        brightness: 300.0,
    });

    // Camera — orbit style, looking at center of map
    let center = Vec3::new(
        config.grid_width as f32 * TILE_SIZE / 2.0,
        0.0,
        config.grid_height as f32 * TILE_SIZE / 2.0,
    );
    let camera_pos = center + Vec3::new(40.0, 60.0, 40.0);
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(camera_pos).looking_at(center, Vec3::Y),
        OrbitCamera {
            target: center,
            distance: 80.0,
            pitch: 0.8, // ~45 degrees
            yaw: std::f32::consts::FRAC_PI_4,
        },
    ));

    // Pre-allocate shared cube mesh/material handles for building fallback
    // This fixes the 10K+ handle leak in the old per-building allocation
    let mut cube_meshes: HashMap<(TileType, u8), Handle<Mesh>> = HashMap::new();
    let mut cube_materials: HashMap<TileType, Handle<StandardMaterial>> = HashMap::new();

    let building_types = [
        TileType::Residential, TileType::Commercial, TileType::Industrial,
        TileType::PowerPlant, TileType::WaterTower, TileType::Monument,
    ];
    for &tile_type in &building_types {
        let (r, g, b) = tile_type.color();
        cube_materials.insert(tile_type, materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            perceptual_roughness: 0.6,
            ..default()
        }));
        // Create a cube mesh per stage (different heights)
        for stage in 0u8..3 {
            let age = match stage { 0 => 0, 1 => 20, _ => 50 };
            let height = tile_type.height_floors(age);
            if height > 0.0 {
                let h = height * 0.4;
                cube_meshes.insert(
                    (tile_type, stage),
                    meshes.add(Cuboid::new(0.7, h, 0.7)),
                );
            }
        }
    }

    // Load GLTF building models by scanning directories for GLB files
    let load_glbs = |dir: &str, filter: &str, asset_server: &AssetServer| -> Vec<Handle<Scene>> {
        let dir_path = Path::new("assets/models/buildings").join(dir);
        let mut handles = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir_path) {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.ends_with(".glb")
                        && name.contains(filter)
                        && !name.starts_with("detail-")
                        && !name.starts_with("road-")
                        && !name.starts_with("floor-")
                })
                .collect();
            files.sort_by_key(|e| e.file_name());
            for entry in &files {
                let rel_path = format!("models/buildings/{}/{}", dir, entry.file_name().to_string_lossy());
                let scene_path = format!("{}#Scene0", rel_path);
                let handle: Handle<Scene> = asset_server.load(&scene_path);
                info!("  Loaded {}", rel_path);
                handles.push(handle);
            }
        }
        handles
    };

    let residential = load_glbs("residential", "building", &asset_server);
    let commercial = load_glbs("commercial", "building-", &asset_server);
    // Separate skyscrapers for high-density commercial
    let commercial_large = load_glbs("commercial", "skyscraper", &asset_server);
    let industrial = load_glbs("industrial", "building", &asset_server);

    let has_models = !residential.is_empty() || !commercial.is_empty() || !industrial.is_empty();
    if has_models {
        eprintln!("[SlideCity] Building models found: {} residential, {} commercial ({} skyscrapers), {} industrial",
            residential.len(), commercial.len(), commercial_large.len(), industrial.len());
    } else {
        eprintln!("[SlideCity] No building models found in assets/models/buildings/ — using cube fallback");
    }

    // Store resources
    let prev_state = PreviousCellState::from_grid(&grid);
    commands.insert_resource(GameGrid { grid, next_grid });
    commands.insert_resource(GameState {
        config,
        rng,
        mayor,
        funds,
        tick_count: 0,
        tick_timer: 0.0,
        stats,
        speed_idx: 0,
        speed_levels: [1.0, 2.0, 4.0, 8.0],
    });
    commands.insert_resource(TerrainMeshHandle(mesh_handle));
    commands.insert_resource(TerrainAtlas { loaded: use_atlas });
    commands.insert_resource(prev_state);
    commands.insert_resource(CubeFallbackHandles {
        meshes: cube_meshes,
        materials: cube_materials,
    });
    commands.insert_resource(BuildingModelPool {
        residential,
        commercial,
        commercial_large,
        industrial,
        has_models,
        models_ready: false,
    });

    // Load Kenney tree models or fall back to procedural cones
    let tree_dir = Path::new("assets/models/trees");
    let mut tree_scenes = Vec::new();
    if tree_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(tree_dir) {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().ends_with(".glb"))
                .collect();
            files.sort_by_key(|e| e.file_name());
            for entry in &files {
                let path = format!("models/trees/{}#Scene0", entry.file_name().to_string_lossy());
                let handle: Handle<Scene> = asset_server.load(&path);
                eprintln!("[SlideCity] Loaded tree model: {}", entry.file_name().to_string_lossy());
                tree_scenes.push(handle);
            }
        }
    }

    let use_gltf_trees = !tree_scenes.is_empty();
    let procedural_trees = if use_gltf_trees {
        eprintln!("[SlideCity] Using {} Kenney tree models", tree_scenes.len());
        Vec::new()
    } else {
        eprintln!("[SlideCity] No tree models found — using procedural cones");
        create_tree_variants(&mut meshes, &mut materials)
    };

    commands.insert_resource(TreeModelPool {
        scenes: tree_scenes,
        procedural: procedural_trees,
        use_gltf: use_gltf_trees,
    });
    commands.insert_resource(TreeLodTimer(Timer::from_seconds(0.5, TimerMode::Repeating)));

    // Road network mesh (starts empty, rebuilt when roads are placed)
    let road_mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new())
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new())
        .with_inserted_indices(Indices::U32(Vec::new()));
    let road_mesh_handle = meshes.add(road_mesh);
    let road_entity = commands.spawn((
        Mesh3d(road_mesh_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.3, 0.3),
            perceptual_roughness: 0.95,
            metallic: 0.0,
            ..default()
        })),
        Transform::default(),
    )).id();
    commands.insert_resource(RoadMeshRes {
        handle: road_mesh_handle,
        entity: road_entity,
    });
}

// ===== ORBIT CAMERA =====

#[derive(Component)]
struct OrbitCamera {
    target: Vec3,
    distance: f32,
    pitch: f32, // radians, 0 = horizontal, PI/2 = top-down
    yaw: f32,   // radians, rotation around Y axis
}

fn camera_controls(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<bevy::input::mouse::MouseMotion>,
    mut scroll: EventReader<bevy::input::mouse::MouseWheel>,
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
    grid_res: Res<GameGrid>,
) {
    let (mut orbit, mut transform) = query.single_mut();

    // Accumulate mouse motion for this frame
    let mut delta = Vec2::ZERO;
    for ev in mouse_motion.read() {
        delta += ev.delta;
    }

    // Q/E rotate
    let rotate_speed = 1.5 * time.delta_secs();
    if keys.pressed(KeyCode::KeyQ) {
        orbit.yaw -= rotate_speed;
    }
    if keys.pressed(KeyCode::KeyE) {
        orbit.yaw += rotate_speed;
    }

    // Right-click drag to rotate
    if mouse_button.pressed(MouseButton::Right) {
        orbit.yaw += delta.x * 0.005;
        orbit.pitch = (orbit.pitch - delta.y * 0.005).clamp(0.2, 1.4);
    }

    // Middle-click drag to pan
    if mouse_button.pressed(MouseButton::Middle) {
        let forward = Vec3::new(-orbit.yaw.sin(), 0.0, -orbit.yaw.cos());
        let right = Vec3::new(orbit.yaw.cos(), 0.0, -orbit.yaw.sin());
        let sensitivity = orbit.distance * 0.003; // scale pan speed with zoom
        orbit.target += right * (-delta.x * sensitivity) + forward * (delta.y * sensitivity);
    }

    // Scroll to zoom
    for ev in scroll.read() {
        orbit.distance = (orbit.distance - ev.y * 5.0).clamp(20.0, 200.0);
    }

    // WASD to pan target
    let pan_speed = 30.0 * time.delta_secs();
    let forward = Vec3::new(-orbit.yaw.sin(), 0.0, -orbit.yaw.cos());
    let right = Vec3::new(orbit.yaw.cos(), 0.0, -orbit.yaw.sin());
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        orbit.target += forward * pan_speed;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        orbit.target -= forward * pan_speed;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        orbit.target -= right * pan_speed;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        orbit.target += right * pan_speed;
    }

    // Clamp camera target to grid bounds
    let max_x = grid_res.grid.width as f32 * TILE_SIZE;
    let max_z = grid_res.grid.height as f32 * TILE_SIZE;
    orbit.target.x = orbit.target.x.clamp(0.0, max_x);
    orbit.target.z = orbit.target.z.clamp(0.0, max_z);

    // Apply orbit transform
    let offset = Vec3::new(
        orbit.yaw.sin() * orbit.pitch.cos() * orbit.distance,
        orbit.pitch.sin() * orbit.distance,
        orbit.yaw.cos() * orbit.pitch.cos() * orbit.distance,
    );
    transform.translation = orbit.target + offset;
    transform.look_at(orbit.target, Vec3::Y);
}

// ===== TERRAIN MESH =====

fn build_terrain_mesh(grid: &Grid, use_atlas: bool) -> Mesh {
    let w = grid.width;
    let h = grid.height;
    // Each cell gets its own 4 vertices (no sharing) for crisp per-cell colors/UVs
    let num_cells = w * h;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(num_cells * 4);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(num_cells * 4);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(num_cells * 4);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(num_cells * 4);
    let mut indices: Vec<u32> = Vec::with_capacity(num_cells * 6);

    for row in 0..h {
        for col in 0..w {
            let cell = grid.get(col, row);
            let base_idx = (row * w + col) as u32 * 4;

            // Sample neighbor heights for corners (average with adjacent cells)
            let h_tl = corner_height(grid, col, row, HEIGHT_SCALE);
            let h_tr = corner_height(grid, col + 1, row, HEIGHT_SCALE);
            let h_bl = corner_height(grid, col, row + 1, HEIGHT_SCALE);
            let h_br = corner_height(grid, col + 1, row + 1, HEIGHT_SCALE);

            let x = col as f32 * TILE_SIZE;
            let z = row as f32 * TILE_SIZE;

            // 4 corners: TL, TR, BL, BR
            positions.push([x, h_tl, z]);
            positions.push([x + TILE_SIZE, h_tr, z]);
            positions.push([x, h_bl, z + TILE_SIZE]);
            positions.push([x + TILE_SIZE, h_br, z + TILE_SIZE]);

            // Flat normal for the cell (computed from the quad)
            let p0 = Vec3::new(x, h_tl, z);
            let p1 = Vec3::new(x + TILE_SIZE, h_tr, z);
            let p2 = Vec3::new(x, h_bl, z + TILE_SIZE);
            let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
            let n = [normal.x, normal.y, normal.z];
            normals.push(n);
            normals.push(n);
            normals.push(n);
            normals.push(n);

            // UV coordinates: map to atlas tile based on terrain/tile type
            let (atlas_col, atlas_row) = cell_atlas_tile(grid, col, row);
            let cell_uvs = atlas_uvs(atlas_col, atlas_row);
            uvs.push(cell_uvs[0]);
            uvs.push(cell_uvs[1]);
            uvs.push(cell_uvs[2]);
            uvs.push(cell_uvs[3]);

            // Vertex color: white when atlas is active (texture provides color),
            // terrain color when no atlas (fallback)
            let color = if use_atlas {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                let (r, g, b) = terrain_color(cell);
                [r, g, b, 1.0]
            };
            colors.push(color);
            colors.push(color);
            colors.push(color);
            colors.push(color);

            // Two triangles: TL-BL-TR, TR-BL-BR
            indices.push(base_idx);
            indices.push(base_idx + 2);
            indices.push(base_idx + 1);

            indices.push(base_idx + 1);
            indices.push(base_idx + 2);
            indices.push(base_idx + 3);
        }
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Average height at a grid corner (shared by up to 4 cells).
/// If any adjacent cell is a road, flatten the corner to match the road's smoothed height
/// so terrain seamlessly meets the road surface (SC4-style contouring).
fn corner_height(grid: &Grid, col: usize, row: usize, scale: f32) -> f32 {
    let mut sum = 0.0;
    let mut count = 0.0;
    let mut has_road = false;
    let mut road_height_sum = 0.0;
    let mut road_count = 0.0;

    // Sample up to 4 adjacent cells
    for dr in [0usize.wrapping_sub(1), 0] {
        for dc in [0usize.wrapping_sub(1), 0] {
            let c = col.wrapping_add(dc);
            let r = row.wrapping_add(dr);
            if c < grid.width && r < grid.height {
                let cell = grid.get(c, r);
                if cell.tile == TileType::WaterBody {
                    sum += 0.0;
                } else if cell.tile == TileType::Road {
                    // Road cell: use smoothed road height for flattening
                    has_road = true;
                    let rh = smoothed_road_height(grid, c, r);
                    road_height_sum += rh;
                    road_count += 1.0;
                    sum += rh;
                } else {
                    sum += cell.terrain_height * scale;
                }
                count += 1.0;
            }
        }
    }

    if has_road {
        // Flatten: terrain corners touching roads snap to the road height.
        // This creates SC4-style smooth road contouring.
        road_height_sum / road_count
    } else if count > 0.0 {
        sum / count
    } else {
        0.0
    }
}

fn terrain_color(cell: &grid::Cell) -> (f32, f32, f32) {
    if cell.tile == TileType::WaterBody {
        return (0.1, 0.3, 0.7);
    }
    if cell.tile != TileType::Empty {
        // Developed cells — use tile color for now
        return cell.tile.color();
    }
    match cell.terrain_type {
        TerrainType::Grass => (0.3, 0.55, 0.2),
        TerrainType::GrassFlower => (0.35, 0.58, 0.25),
        TerrainType::Trees => (0.15, 0.42, 0.12),
        TerrainType::TreesSparse => (0.22, 0.48, 0.16),
        TerrainType::Sand => (0.76, 0.70, 0.50),
        TerrainType::Dirt => (0.45, 0.38, 0.25),
        TerrainType::Rock => (0.50, 0.48, 0.45),
        TerrainType::Snow => (0.85, 0.88, 0.92),
    }
}

// ===== SIMULATION =====

fn simulation_tick(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<GameState>,
    mut grid_res: ResMut<GameGrid>,
) {
    // Speed control
    if keys.just_pressed(KeyCode::Digit1) { game.speed_idx = 0; }
    if keys.just_pressed(KeyCode::Digit2) { game.speed_idx = 1; }
    if keys.just_pressed(KeyCode::Digit3) { game.speed_idx = 2; }
    if keys.just_pressed(KeyCode::Digit4) { game.speed_idx = 3; }

    let speed = game.speed_levels[game.speed_idx];
    let tick_duration = game.config.base_tick_ms / 1000.0 / speed;
    game.tick_timer += time.delta_secs();

    let GameGrid { ref mut grid, ref mut next_grid } = *grid_res;
    let GameState {
        ref config, ref mut rng, ref mut mayor, ref mut funds,
        ref mut tick_count, ref mut tick_timer, ref mut stats, ..
    } = *game;

    while *tick_timer >= tick_duration {
        *tick_timer -= tick_duration;
        *tick_count += 1;

        sim::tick(grid, next_grid, config, rng, funds);

        if tick_count.is_multiple_of(config.utility_recompute_interval) {
            sim::utilities::recompute_utilities(grid);
        }

        if tick_count.is_multiple_of(config.mayor_tick_interval) {
            let tc = *tick_count;
            let s = stats.clone();
            mayor.decide(grid, &s, config, funds, tc, rng);
        }

        *stats = CityStats::compute(grid);
    }
}

// ===== UPDATE TERRAIN MESH =====

fn update_terrain_mesh(
    grid_res: Res<GameGrid>,
    terrain_handle: Res<TerrainMeshHandle>,
    atlas: Res<TerrainAtlas>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if !grid_res.is_changed() {
        return;
    }

    if let Some(mesh) = meshes.get_mut(&terrain_handle.0) {
        let grid = &grid_res.grid;
        let w = grid.width;
        let h = grid.height;
        let num_verts = w * h * 4;
        let use_atlas = atlas.loaded;

        let mut colors: Vec<[f32; 4]> = Vec::with_capacity(num_verts);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(num_verts);

        for row in 0..h {
            for col in 0..w {
                let cell = grid.get(col, row);

                // UV coordinates
                let (atlas_col, atlas_row) = cell_atlas_tile(grid, col, row);
                let cell_uvs = atlas_uvs(atlas_col, atlas_row);
                uvs.push(cell_uvs[0]);
                uvs.push(cell_uvs[1]);
                uvs.push(cell_uvs[2]);
                uvs.push(cell_uvs[3]);

                // Vertex color
                let color = if use_atlas {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    let (r, g, b) = terrain_color(cell);
                    [r, g, b, 1.0]
                };
                colors.push(color);
                colors.push(color);
                colors.push(color);
                colors.push(color);
            }
        }

        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }
}

// ===== DIRTY CELL TRACKING =====

/// Computes which cells changed visually since last frame.
/// Runs after simulation_tick (which includes sim + mayor + utilities).
fn compute_dirty_cells(
    grid_res: Res<GameGrid>,
    mut prev_state: ResMut<PreviousCellState>,
) {
    prev_state.dirty.clear();

    if !grid_res.is_changed() {
        return;
    }

    let grid = &grid_res.grid;
    for row in 0..grid.height {
        for col in 0..grid.width {
            let idx = row * grid.width + col;
            let cell = &grid.cells[idx];
            let new_state = (cell.tile, age_stage(cell.age), cell.style);
            if prev_state.cells[idx] != new_state {
                prev_state.dirty.insert((col, row));
                prev_state.cells[idx] = new_state;
            }
        }
    }
}

// ===== BUILDINGS =====

#[derive(Component)]
struct BuildingMarker {
    col: usize,
    row: usize,
}

/// Incremental building update: only despawn/respawn buildings for dirty cells.
/// Uses shared CubeFallbackHandles to avoid per-entity asset allocation.
/// Will use GLTF models from BuildingModelPool when available.
/// Check if GLTF models finished loading; if so, despawn all cubes and rebuild with models.
fn check_models_loaded(
    mut model_pool: ResMut<BuildingModelPool>,
    asset_server: Res<AssetServer>,
    mut prev_state: ResMut<PreviousCellState>,
    grid_res: Res<GameGrid>,
    mut frame_count: Local<u32>,
) {
    if !model_pool.has_models || model_pool.models_ready {
        return;
    }

    *frame_count += 1;

    // Check all model handles for loading status
    let all_handles: Vec<_> = model_pool.residential.iter()
        .chain(model_pool.commercial.iter())
        .chain(model_pool.commercial_large.iter())
        .chain(model_pool.industrial.iter())
        .collect();

    if all_handles.is_empty() {
        return;
    }

    let loaded_count = all_handles.iter()
        .filter(|h| asset_server.is_loaded_with_dependencies(h.id()))
        .count();

    // Log progress every 60 frames (~1 second)
    if *frame_count % 60 == 0 {
        eprintln!("[SlideCity] Model loading: {}/{} loaded (frame {})",
            loaded_count, all_handles.len(), *frame_count);
    }

    // Consider ready once at least half are loaded (don't wait for stragglers)
    if loaded_count > all_handles.len() / 2 {
        eprintln!("[SlideCity] Building models ready ({}/{}) — rebuilding all buildings",
            loaded_count, all_handles.len());
        model_pool.models_ready = true;
        // Mark ALL building cells as dirty to force a full rebuild
        let grid = &grid_res.grid;
        for row in 0..grid.height {
            for col in 0..grid.width {
                let cell = grid.get(col, row);
                if cell.tile.height_floors(cell.age) > 0.0 {
                    prev_state.dirty.insert((col, row));
                }
            }
        }
    }
}

fn update_buildings(
    mut commands: Commands,
    grid_res: Res<GameGrid>,
    prev_state: Res<PreviousCellState>,
    cube_handles: Res<CubeFallbackHandles>,
    model_pool: Res<BuildingModelPool>,
    existing: Query<(Entity, &BuildingMarker)>,
) {
    if prev_state.dirty.is_empty() {
        return;
    }

    let grid = &grid_res.grid;

    // Despawn only buildings on dirty cells
    for (entity, marker) in &existing {
        if prev_state.dirty.contains(&(marker.col, marker.row)) {
            commands.entity(entity).despawn();
        }
    }

    // Spawn new buildings for dirty cells that now have buildings
    for &(col, row) in &prev_state.dirty {
        let cell = grid.get(col, row);
        let height = cell.tile.height_floors(cell.age);
        if height <= 0.0 {
            continue;
        }

        let stage = age_stage(cell.age);
        let building_h = height * 0.4;
        let base_y = cell.terrain_height * HEIGHT_SCALE;
        let world_x = col as f32 * TILE_SIZE + TILE_SIZE / 2.0;
        let world_z = row as f32 * TILE_SIZE + TILE_SIZE / 2.0;

        // Try GLTF model first — pick from the right pool based on tile type
        if model_pool.models_ready {
            let models = match cell.tile {
                TileType::Residential => &model_pool.residential,
                TileType::Commercial if stage >= 2 && !model_pool.commercial_large.is_empty() => {
                    &model_pool.commercial_large
                }
                TileType::Commercial => &model_pool.commercial,
                TileType::Industrial => &model_pool.industrial,
                _ => &model_pool.residential, // infrastructure fallback
            };

            if !models.is_empty() {
                // Pick model by hashing col+row+style for variety
                let idx = (col * 31 + row * 17 + cell.style as usize) % models.len();
                let scene_handle = &models[idx];
                // Scale based on building stage
                let model_scale = match stage {
                    0 => 0.35,
                    1 => 0.55,
                    _ => 0.8,
                };
                commands.spawn((
                    SceneRoot(scene_handle.clone()),
                    Transform::from_xyz(world_x, base_y, world_z)
                        .with_scale(Vec3::splat(model_scale)),
                    BuildingMarker { col, row },
                ));
                continue;
            }
        }

        // Cube fallback: use pre-allocated shared handles
        let mesh = cube_handles.meshes.get(&(cell.tile, stage))
            .or_else(|| cube_handles.meshes.get(&(cell.tile, 0)));
        let material = cube_handles.materials.get(&cell.tile);

        if let (Some(mesh_handle), Some(mat_handle)) = (mesh, material) {
            commands.spawn((
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(mat_handle.clone()),
                Transform::from_xyz(world_x, base_y + building_h / 2.0, world_z),
                BuildingMarker { col, row },
            ));
        }
    }
}

// ===== TREES =====

/// Generate a simple cone mesh for tree foliage (procedural placeholder).
fn make_cone_mesh(radius: f32, height: f32, segments: u32) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    // Tip vertex
    positions.push([0.0, height, 0.0]);
    normals.push([0.0, 1.0, 0.0]);

    // Base ring
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;
        positions.push([x, 0.0, z]);
        // Approximate outward normal
        let ny = radius / height;
        let n = Vec3::new(angle.cos(), ny, angle.sin()).normalize();
        normals.push([n.x, n.y, n.z]);
    }

    // Side triangles (tip to base ring)
    for i in 0..segments {
        let next = (i + 1) % segments;
        indices.push(0); // tip
        indices.push(1 + i);
        indices.push(1 + next);
    }

    // Base center
    let base_center = positions.len() as u32;
    positions.push([0.0, 0.0, 0.0]);
    normals.push([0.0, -1.0, 0.0]);

    // Base triangles
    for i in 0..segments {
        let next = (i + 1) % segments;
        indices.push(base_center);
        indices.push(1 + next);
        indices.push(1 + i);
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(Indices::U32(indices))
}

/// Create procedural tree variants (different sizes/colors) as shared mesh handles.
fn create_tree_variants(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Vec<(Handle<Mesh>, Handle<StandardMaterial>)> {
    let variants = [
        // (foliage_radius, foliage_height, trunk_height, color)
        (0.25, 0.5, 0.15, Color::srgb(0.12, 0.40, 0.10)),  // small dark green
        (0.30, 0.6, 0.20, Color::srgb(0.18, 0.45, 0.12)),  // medium green
        (0.22, 0.55, 0.18, Color::srgb(0.15, 0.38, 0.08)),  // slim dark
        (0.35, 0.7, 0.15, Color::srgb(0.20, 0.50, 0.15)),  // large bright
    ];

    variants.iter().map(|&(radius, height, trunk_h, color)| {
        // Combine trunk cylinder + foliage cone into a single mesh
        // For simplicity, just use the cone (trunk is tiny at this scale)
        let mut cone = make_cone_mesh(radius, height, 8);

        // Offset cone up by trunk height
        if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(ref mut pos)) =
            cone.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            for p in pos.iter_mut() {
                p[1] += trunk_h;
            }
        }

        let mesh_handle = meshes.add(cone);
        let mat_handle = materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.8,
            ..default()
        });

        (mesh_handle, mat_handle)
    }).collect()
}

/// Spawn trees on forest terrain cells. Only runs when terrain types change.
fn spawn_trees(
    mut commands: Commands,
    grid_res: Res<GameGrid>,
    prev_state: Res<PreviousCellState>,
    tree_pool: Res<TreeModelPool>,
    existing: Query<(Entity, &TreeMarker)>,
    mut initial_spawn_done: Local<bool>,
) {
    // On first frame, spawn all trees. After that, only update dirty cells.
    let full_spawn = !*initial_spawn_done;
    if !full_spawn && prev_state.dirty.is_empty() {
        return;
    }

    let grid = &grid_res.grid;
    let num_variants = if tree_pool.use_gltf {
        tree_pool.scenes.len()
    } else {
        tree_pool.procedural.len()
    };
    if num_variants == 0 {
        return;
    }

    if full_spawn {
        *initial_spawn_done = true;
        for row in 0..grid.height {
            for col in 0..grid.width {
                spawn_trees_for_cell(&mut commands, grid, col, row, &tree_pool, num_variants);
            }
        }
    } else {
        for (entity, marker) in &existing {
            if prev_state.dirty.contains(&(marker.col, marker.row)) {
                commands.entity(entity).despawn();
            }
        }
        for &(col, row) in &prev_state.dirty {
            spawn_trees_for_cell(&mut commands, grid, col, row, &tree_pool, num_variants);
        }
    }
}

fn spawn_trees_for_cell(
    commands: &mut Commands,
    grid: &Grid,
    col: usize,
    row: usize,
    tree_pool: &TreeModelPool,
    num_variants: usize,
) {
    let cell = grid.get(col, row);

    if cell.tile != TileType::Empty {
        return;
    }

    let tree_count = match cell.terrain_type {
        TerrainType::Trees => 3,
        TerrainType::TreesSparse => 1,
        _ => return,
    };

    let base_y = cell.terrain_height * HEIGHT_SCALE;

    for i in 0..tree_count {
        let seed = (col * 7919 + row * 6271 + i * 3571) as u32;
        let fx = ((seed * 2654435761) & 0xFFFF) as f32 / 65535.0;
        let fz = ((seed * 2246822519) & 0xFFFF) as f32 / 65535.0;

        let margin = 0.15;
        let x = col as f32 * TILE_SIZE + margin + fx * (TILE_SIZE - 2.0 * margin);
        let z = row as f32 * TILE_SIZE + margin + fz * (TILE_SIZE - 2.0 * margin);

        let variant_idx = (col * 7 + row * 13 + i) % num_variants;

        let scale_seed = ((seed * 1103515245 + 12345) & 0xFFFF) as f32 / 65535.0;
        let scale = 0.25 + scale_seed * 0.25; // 0.25 to 0.5 (Kenney models are large)

        if tree_pool.use_gltf {
            let scene = &tree_pool.scenes[variant_idx];
            commands.spawn((
                SceneRoot(scene.clone()),
                Transform::from_xyz(x, base_y, z)
                    .with_scale(Vec3::splat(scale)),
                TreeMarker { col, row },
            ));
        } else {
            let (ref mesh, ref mat) = tree_pool.procedural[variant_idx];
            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x, base_y, z)
                    .with_scale(Vec3::splat(scale * 3.0)),
                TreeMarker { col, row },
            ));
        }
    }
}

/// LOD system: hide trees beyond a distance threshold.
#[allow(clippy::type_complexity)]
fn tree_lod_update(
    time: Res<Time>,
    mut lod_timer: ResMut<TreeLodTimer>,
    camera_query: Query<&Transform, With<OrbitCamera>>,
    mut tree_query: Query<(&Transform, &mut Visibility), (With<TreeMarker>, Without<OrbitCamera>)>,
) {
    lod_timer.0.tick(time.delta());
    if !lod_timer.0.just_finished() {
        return;
    }

    let camera_transform = camera_query.single();
    let cam_pos = camera_transform.translation;

    for (tree_transform, mut visibility) in &mut tree_query {
        let dist_sq = cam_pos.distance_squared(tree_transform.translation);
        // Hide trees beyond 150 units (150^2 = 22500)
        if dist_sq > 22500.0 {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
        }
    }
}

// ===== ROAD NETWORK MESH =====

/// Road surface offset above terrain to prevent z-fighting.
const ROAD_Y_OFFSET: f32 = 0.02;

/// Compute smoothed road height: average this road cell's height with connected
/// road neighbors. This prevents "steps" between adjacent road cells on uneven terrain.
fn smoothed_road_height(grid: &Grid, col: usize, row: usize) -> f32 {
    let cell = grid.get(col, row);
    let mut sum = cell.terrain_height;
    let mut count = 1.0;

    // Average with cardinal road neighbors
    let neighbors: [(isize, isize); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    for (dc, dr) in neighbors {
        let nc = col as isize + dc;
        let nr = row as isize + dr;
        if nc >= 0 && nc < grid.width as isize && nr >= 0 && nr < grid.height as isize {
            let n = grid.get(nc as usize, nr as usize);
            if n.tile == TileType::Road {
                sum += n.terrain_height;
                count += 1.0;
            }
        }
    }

    (sum / count) * HEIGHT_SCALE + ROAD_Y_OFFSET
}

/// Compute a 4-bit bitmask for a road cell based on cardinal road neighbors.
/// N=8, E=4, S=2, W=1
fn road_neighbor_mask(grid: &Grid, col: usize, row: usize) -> u8 {
    let mut mask = 0u8;
    if row > 0 && grid.get(col, row - 1).tile == TileType::Road {
        mask |= 8; // North
    }
    if col + 1 < grid.width && grid.get(col + 1, row).tile == TileType::Road {
        mask |= 4; // East
    }
    if row + 1 < grid.height && grid.get(col, row + 1).tile == TileType::Road {
        mask |= 2; // South
    }
    if col > 0 && grid.get(col - 1, row).tile == TileType::Road {
        mask |= 1; // West
    }
    mask
}

/// Generate road mesh geometry for a single cell.
/// Uses a full-cell-width quad with neighbor-matched edge heights for seamless connections.
fn emit_road_cell(
    grid: &Grid,
    col: usize,
    row: usize,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let x0 = col as f32 * TILE_SIZE;
    let z0 = row as f32 * TILE_SIZE;
    let x1 = x0 + TILE_SIZE;
    let z1 = z0 + TILE_SIZE;

    // Use smoothed height for this cell
    let y = smoothed_road_height(grid, col, row);

    // Match corner heights to neighbors for seamless edges:
    // Each corner averages the smoothed heights of the up-to-4 road cells that share it.
    let y_tl = road_corner_height(grid, col, row, 0, 0); // top-left
    let y_tr = road_corner_height(grid, col, row, 1, 0); // top-right
    let y_bl = road_corner_height(grid, col, row, 0, 1); // bottom-left
    let y_br = road_corner_height(grid, col, row, 1, 1); // bottom-right

    let up = [0.0f32, 1.0, 0.0];
    let road_color = [0.30, 0.30, 0.32, 1.0]; // Uniform asphalt

    // Full-cell road quad
    let base_idx = positions.len() as u32;
    positions.push([x0, y_tl, z0]); // TL
    positions.push([x1, y_tr, z0]); // TR
    positions.push([x0, y_bl, z1]); // BL
    positions.push([x1, y_br, z1]); // BR
    for _ in 0..4 { normals.push(up); colors.push(road_color); }
    indices.extend_from_slice(&[base_idx, base_idx+2, base_idx+1, base_idx+1, base_idx+2, base_idx+3]);

    // Center line markings (yellow dashed)
    let cx = x0 + TILE_SIZE / 2.0;
    let cz = z0 + TILE_SIZE / 2.0;
    let mask = road_neighbor_mask(grid, col, row);
    let marking_color = [0.75, 0.72, 0.55, 1.0];
    let lw = 0.025; // line width
    let my = y + 0.002; // slightly above road surface

    // Vertical center line (N-S roads)
    if mask & 0b1010 != 0 { // has N or S neighbor
        let idx = positions.len() as u32;
        positions.push([cx - lw, my, z0]);
        positions.push([cx + lw, my, z0]);
        positions.push([cx - lw, my, z1]);
        positions.push([cx + lw, my, z1]);
        for _ in 0..4 { normals.push(up); colors.push(marking_color); }
        indices.extend_from_slice(&[idx, idx+2, idx+1, idx+1, idx+2, idx+3]);
    }

    // Horizontal center line (E-W roads)
    if mask & 0b0101 != 0 { // has E or W neighbor
        let idx = positions.len() as u32;
        positions.push([x0, my, cz - lw]);
        positions.push([x1, my, cz - lw]);
        positions.push([x0, my, cz + lw]);
        positions.push([x1, my, cz + lw]);
        for _ in 0..4 { normals.push(up); colors.push(marking_color); }
        indices.extend_from_slice(&[idx, idx+2, idx+1, idx+1, idx+2, idx+3]);
    }
}

/// Compute road corner height by averaging smoothed heights of road cells sharing this corner.
/// corner_dx/corner_dy: 0=left/top, 1=right/bottom
fn road_corner_height(grid: &Grid, col: usize, row: usize, corner_dx: usize, corner_dy: usize) -> f32 {
    let mut sum = 0.0;
    let mut count = 0.0;

    // The 4 cells that share this corner
    for dr in [0usize.wrapping_sub(1), 0] {
        for dc in [0usize.wrapping_sub(1), 0] {
            let c = (col + corner_dx).wrapping_add(dc);
            let r = (row + corner_dy).wrapping_add(dr);
            if c < grid.width && r < grid.height {
                let cell = grid.get(c, r);
                if cell.tile == TileType::Road {
                    sum += smoothed_road_height(grid, c, r);
                    count += 1.0;
                } else {
                    // Non-road neighbors: use the base terrain height so road
                    // edges blend with terrain
                    sum += cell.terrain_height * HEIGHT_SCALE + ROAD_Y_OFFSET;
                    count += 1.0;
                }
            }
        }
    }
    if count > 0.0 { sum / count } else { smoothed_road_height(grid, col, row) }
}

/// Full rebuild of the road network mesh from grid state.
fn build_road_mesh(grid: &Grid) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for row in 0..grid.height {
        for col in 0..grid.width {
            if grid.get(col, row).tile != TileType::Road {
                continue;
            }
            emit_road_cell(grid, col, row, &mut positions, &mut normals, &mut colors, &mut indices);
        }
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(indices))
}

/// Update the road mesh when road cells change.
fn update_road_mesh(
    grid_res: Res<GameGrid>,
    prev_state: Res<PreviousCellState>,
    road_res: Res<RoadMeshRes>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Only rebuild if any dirty cell involves a road
    if prev_state.dirty.is_empty() {
        return;
    }

    let grid = &grid_res.grid;

    // Conservative: rebuild road mesh on any dirty cell change.
    // Road neighbors affect adjacent cells' mesh pieces, so any tile change
    // near a road could affect the road mesh.

    // Full rebuild of road mesh
    let new_mesh = build_road_mesh(grid);

    if let Some(mesh) = meshes.get_mut(&road_res.handle) {
        // Copy attributes from new mesh
        if let Some(pos) = new_mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos.clone());
        }
        if let Some(norm) = new_mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, norm.clone());
        }
        if let Some(col) = new_mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, col.clone());
        }
        if let Some(idx) = new_mesh.indices() {
            mesh.insert_indices(idx.clone());
        }
    }
}

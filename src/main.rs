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
#[derive(Resource)]
#[allow(dead_code)]
struct BuildingModelPool {
    /// (tile_type, stage, variant) -> scene handle
    models: HashMap<(TileType, u8, u8), Handle<Scene>>,
    /// True once all handles are confirmed loaded (future: async loading check)
    loaded: bool,
    /// True if any model files were found at startup
    has_models: bool,
}

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
            update_buildings.in_set(GameSet::Render),
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

    // Check for GLTF building models
    let models_dir = Path::new("assets/models/buildings");
    let has_models = models_dir.exists();
    let mut model_pool = HashMap::new();

    if has_models {
        info!("Building models directory found — loading GLTFs");
        let zone_dirs = [
            ("residential", TileType::Residential),
            ("commercial", TileType::Commercial),
            ("industrial", TileType::Industrial),
        ];
        for (dir_name, tile_type) in &zone_dirs {
            for stage in 1u8..=3 {
                for variant in 1u8..=6 {
                    let path = format!("models/buildings/{}/s{}_v{}.glb", dir_name, stage, variant);
                    if Path::new("assets").join(&path).exists() {
                        // Use #Scene0 label to load the first scene from the GLB
                        let scene_path = format!("{}#Scene0", path);
                        let handle: Handle<Scene> = asset_server.load(&scene_path);
                        model_pool.insert((*tile_type, stage - 1, variant - 1), handle);
                        info!("  Loaded {}", path);
                    }
                }
            }
        }
        // Infrastructure models
        let infra = [
            ("models/buildings/infrastructure/power_plant.glb", TileType::PowerPlant),
            ("models/buildings/infrastructure/water_tower.glb", TileType::WaterTower),
            ("models/buildings/infrastructure/monument.glb", TileType::Monument),
        ];
        for (path, tile_type) in &infra {
            if Path::new("assets").join(path).exists() {
                let scene_path = format!("{}#Scene0", path);
                let handle: Handle<Scene> = asset_server.load(&scene_path);
                model_pool.insert((*tile_type, 0, 0), handle);
                info!("  Loaded {}", path);
            }
        }
    } else {
        info!("No building models at assets/models/buildings/ — using cube fallback");
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
        models: model_pool,
        loaded: false,
        has_models,
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
fn corner_height(grid: &Grid, col: usize, row: usize, scale: f32) -> f32 {
    let mut sum = 0.0;
    let mut count = 0.0;
    // Sample up to 4 adjacent cells
    for dr in [0usize.wrapping_sub(1), 0] {
        for dc in [0usize.wrapping_sub(1), 0] {
            let c = col.wrapping_add(dc);
            let r = row.wrapping_add(dr);
            if c < grid.width && r < grid.height {
                let cell = grid.get(c, r);
                if cell.tile == TileType::WaterBody {
                    sum += 0.0;
                } else {
                    sum += cell.terrain_height * scale;
                }
                count += 1.0;
            }
        }
    }
    if count > 0.0 { sum / count } else { 0.0 }
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

        // Try GLTF model first
        let variant = cell.style;
        if model_pool.has_models {
            // Try exact variant, then fall back to variant 0
            let model = model_pool.models.get(&(cell.tile, stage, variant))
                .or_else(|| model_pool.models.get(&(cell.tile, stage, 0)))
                .or_else(|| model_pool.models.get(&(cell.tile, 0, 0)));

            if let Some(scene_handle) = model {
                commands.spawn((
                    SceneRoot(scene_handle.clone()),
                    Transform::from_xyz(world_x, base_y, world_z),
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

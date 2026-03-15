mod config;
mod grid;
mod influence;
mod mayor;
mod sim;

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

// ===== CONSTANTS =====

const TILE_SIZE: f32 = 1.0; // World units per grid cell
const HEIGHT_SCALE: f32 = 3.0; // Terrain height exaggeration

// ===== MAIN =====

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "SlideCity".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (
            camera_controls,
            simulation_tick,
            update_terrain_mesh,
            update_buildings,
        ))
        .run();
}

// ===== SETUP =====

fn setup(
    mut commands: Commands,
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

    // Generate terrain mesh
    let terrain_mesh = build_terrain_mesh(&grid);
    let mesh_handle = meshes.add(terrain_mesh);

    // Terrain entity
    commands.spawn((
        Mesh3d(mesh_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.9,
            metallic: 0.0,
            ..default()
        })),
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

    // Store resources
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
) {
    let (mut orbit, mut transform) = query.single_mut();

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
        for ev in mouse_motion.read() {
            orbit.yaw += ev.delta.x * 0.005;
            orbit.pitch = (orbit.pitch - ev.delta.y * 0.005).clamp(0.2, 1.4);
        }
    } else {
        mouse_motion.clear();
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

    // Speed control
    // (handled in simulation_tick)

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

fn build_terrain_mesh(grid: &Grid) -> Mesh {
    let w = grid.width;
    let h = grid.height;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity((w + 1) * (h + 1));
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity((w + 1) * (h + 1));
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity((w + 1) * (h + 1));
    let mut indices: Vec<u32> = Vec::with_capacity(w * h * 6);

    // Generate vertices at cell corners (w+1 x h+1)
    for row in 0..=h {
        for col in 0..=w {
            // Sample height from nearest cell
            let sample_col = col.min(w - 1);
            let sample_row = row.min(h - 1);
            let cell = grid.get(sample_col, sample_row);

            let height = if cell.tile == TileType::WaterBody {
                0.0 // Water at sea level
            } else {
                cell.terrain_height * HEIGHT_SCALE
            };

            positions.push([
                col as f32 * TILE_SIZE,
                height,
                row as f32 * TILE_SIZE,
            ]);

            normals.push([0.0, 1.0, 0.0]); // Will recompute

            // Vertex color from terrain type
            let (r, g, b) = terrain_color(cell);
            colors.push([r, g, b, 1.0]);
        }
    }

    // Generate triangles (two per cell)
    let stride = (w + 1) as u32;
    for row in 0..h as u32 {
        for col in 0..w as u32 {
            let tl = row * stride + col;
            let tr = tl + 1;
            let bl = (row + 1) * stride + col;
            let br = bl + 1;

            // Two triangles per quad
            indices.push(tl);
            indices.push(bl);
            indices.push(tr);

            indices.push(tr);
            indices.push(bl);
            indices.push(br);
        }
    }

    // Compute smooth normals
    let mut normal_accum: Vec<Vec3> = vec![Vec3::ZERO; positions.len()];
    for tri in indices.chunks(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        let p0 = Vec3::from(positions[i0]);
        let p1 = Vec3::from(positions[i1]);
        let p2 = Vec3::from(positions[i2]);
        let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
        normal_accum[i0] += normal;
        normal_accum[i1] += normal;
        normal_accum[i2] += normal;
    }
    for (i, n) in normal_accum.iter().enumerate() {
        let normalized = n.normalize_or_zero();
        normals[i] = [normalized.x, normalized.y, normalized.z];
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(indices))
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
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if !grid_res.is_changed() {
        return;
    }

    if let Some(mesh) = meshes.get_mut(&terrain_handle.0) {
        // Update vertex colors to reflect new tile states
        let grid = &grid_res.grid;
        let w = grid.width;
        let h = grid.height;
        let mut colors: Vec<[f32; 4]> = Vec::with_capacity((w + 1) * (h + 1));

        for row in 0..=h {
            for col in 0..=w {
                let sample_col = col.min(w - 1);
                let sample_row = row.min(h - 1);
                let cell = grid.get(sample_col, sample_row);
                let (r, g, b) = terrain_color(cell);
                colors.push([r, g, b, 1.0]);
            }
        }

        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
}

// ===== BUILDINGS (placeholder — spawn 3D cubes) =====

#[derive(Component)]
struct BuildingMarker {
    col: usize,
    row: usize,
}

fn update_buildings(
    mut commands: Commands,
    grid_res: Res<GameGrid>,
    game: Res<GameState>,
    existing: Query<(Entity, &BuildingMarker)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !grid_res.is_changed() {
        return;
    }

    // Despawn old buildings
    for (entity, _) in &existing {
        commands.entity(entity).despawn();
    }

    let grid = &grid_res.grid;

    for row in 0..grid.height {
        for col in 0..grid.width {
            let cell = grid.get(col, row);
            let height = cell.tile.height_floors(cell.age);
            if height <= 0.0 {
                continue;
            }

            let (r, g, b) = cell.tile.color();
            let building_h = height * 0.4;
            let base_y = cell.terrain_height * HEIGHT_SCALE;

            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.7, building_h, 0.7))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(r, g, b),
                    perceptual_roughness: 0.6,
                    ..default()
                })),
                Transform::from_xyz(
                    col as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                    base_y + building_h / 2.0,
                    row as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                ),
                BuildingMarker { col, row },
            ));
        }
    }
}

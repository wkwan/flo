use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::asset::{Asset};
use bevy::pbr::{MaterialPlugin, Material};

const WATER_GRID_LEN: usize = 64;
const GRAVITY: f32 = 10.;
const FRICTION: f32 = 1.5;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MaterialPlugin::<WaterMaterial>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, water_sim)
        .run();
}

fn water_sim(
    time: Res<Time>,
    mut query: Query<&mut WaterData>
) {
    let delta_time = time.delta_secs();
    
    for mut water_data in query.iter_mut() {

        for i in 0..WATER_GRID_LEN {
            water_data.flow_x[0][i] = 0.;
            water_data.flow_x[WATER_GRID_LEN-1][i] = 0.;
            water_data.flow_y[i][0] = 0.;
            water_data.flow_y[i][WATER_GRID_LEN-1] = 0.;
        }

        for x in 1..WATER_GRID_LEN {
            for y in 1..WATER_GRID_LEN {
                water_data.flow_x[x][y] = water_data.flow_x[x][y] * FRICTION.powf(delta_time) + (water_data.height[x-1][y] - water_data.height[x][y]) * GRAVITY * delta_time;
                water_data.flow_y[x][y] = water_data.flow_y[x][y] * FRICTION.powf(delta_time) + (water_data.height[x][y-1] - water_data.height[x][y]) * GRAVITY * delta_time;
            }
        }

        for x in 0..WATER_GRID_LEN-1 {
            for y in 0..WATER_GRID_LEN-1 {
                let mut total_outflow = 0.;
                total_outflow += 0.0f32.max(-water_data.flow_x[x][y]);
                total_outflow += 0.0f32.max(-water_data.flow_y[x][y]);
                total_outflow += 0.0f32.max(water_data.flow_x[x+1][y]);
                total_outflow += 0.0f32.max(water_data.flow_y[x][y+1]);

                let max_outflow = water_data.height[x][y] / delta_time;

                if total_outflow > 0. {
                    let scale = 1.0f32.min(max_outflow / total_outflow);
                    if water_data.flow_x[x][y] < 0. {
                        water_data.flow_x[x][y] *= scale;
                    } 
                    if water_data.flow_y[x][y] < 0. {
                        water_data.flow_y[x][y] *= scale;
                    }
                    if water_data.flow_x[x+1][y] > 0. {
                        water_data.flow_x[x+1][y] *= scale
                    }
                    if water_data.flow_y[x][y+1] > 0. {
                        water_data.flow_y[x][y+1] *= scale
                    }
                }
            }
        }

        for x in 1..WATER_GRID_LEN-1 {
            for y in 1..WATER_GRID_LEN-1 {
                water_data.height[x][y] += (water_data.flow_x[x][y] + water_data.flow_y[x][y] - water_data.flow_x[x+1][y] - water_data.flow_y[x][y+1]) * delta_time;
            }
        }

        // Print simulation data
        println!("\n=== Water Simulation Data ===");
        println!("Sample at (32, 32):");
        println!("  Height: {:.4}", water_data.height[32][32]);
        println!("  Flow X: {:.4}", water_data.flow_x[32][32]);
        println!("  Flow Y: {:.4}", water_data.flow_y[32][32]);
        
        // Calculate averages
        let mut avg_height = 0.0;
        let mut avg_flow_x = 0.0;
        let mut avg_flow_y = 0.0;
        for x in 0..WATER_GRID_LEN {
            for y in 0..WATER_GRID_LEN {
                avg_height += water_data.height[x][y];
                avg_flow_x += water_data.flow_x[x][y];
                avg_flow_y += water_data.flow_y[x][y];
            }
        }
        let total_cells = (WATER_GRID_LEN * WATER_GRID_LEN) as f32;
        avg_height /= total_cells;
        avg_flow_x /= total_cells;
        avg_flow_y /= total_cells;
        
        println!("\nAverages:");
        println!("  Height: {:.4}", avg_height);
        println!("  Flow X: {:.4}", avg_flow_x);
        println!("  Flow Y: {:.4}", avg_flow_y);
        println!("============================\n");
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Camera - positioned like in Unity screenshot
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 8.0, 8.0).looking_at(Vec3::new(0.0, 0.0, -2.0), Vec3::Y),
    ));

    // Directional light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 10000.0,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 8.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            ..default()
        },
    ));

    // Load stone wall textures
    let base_color_texture = asset_server.load("Stone Wall/Stone_Wall_basecolor.jpg");
    let normal_texture = asset_server.load("Stone Wall/Stone_Wall_normal.jpg");
    let roughness_texture = asset_server.load("Stone Wall/Stone_Wall_roughness.jpg");
    let ao_texture = asset_server.load("Stone Wall/Stone_Wall_ambientOcclusion.jpg");

    // Stone wall material with textures
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.6, 0.4),
        base_color_texture: Some(base_color_texture),
        normal_map_texture: Some(normal_texture),
        metallic_roughness_texture: Some(roughness_texture),
        occlusion_texture: Some(ao_texture),
        ..default()
    });

    // Water plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(4.0)))),
        MeshMaterial3d(water_materials.add(WaterMaterial {
            color: Vec4::new(0.1, 0.3, 0.8, 0.2),
        })),
        Transform::default(),
        WaterData::default(),
    ));

    // Create stone walls
    let wall_height = 6.0;
    let wall_thickness = 1.0;
    let water_size = 8.0;
    let half_water = water_size * 0.5;

    // Left wall (X = -half_water)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(wall_thickness, wall_height, water_size))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-half_water - wall_thickness * 0.5, wall_height * 0.5 - 2.0, 0.0),
    ));

    // Right wall (X = +half_water)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(wall_thickness, wall_height, water_size))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(half_water + wall_thickness * 0.5, wall_height * 0.5 - 2.0, 0.0),
    ));

    // Back wall (Z = -half_water)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(water_size + wall_thickness * 2.0, wall_height, wall_thickness))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(0.0, wall_height * 0.5 - 2.0, -half_water - wall_thickness * 0.5),
    ));

    // Bottom wall
    let bottom_wall_y = -2.0;
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(water_size + wall_thickness * 2.0, wall_thickness, water_size + wall_thickness * 2.0))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(0.0, bottom_wall_y, 0.0),
    ));
}

#[derive(Component)]
struct WaterData {
    height: [[f32; WATER_GRID_LEN]; WATER_GRID_LEN],
    flow_x: [[f32; WATER_GRID_LEN]; WATER_GRID_LEN],
    flow_y: [[f32; WATER_GRID_LEN]; WATER_GRID_LEN],
}

impl Default for WaterData {
    fn default() -> Self {
        Self {
            height: [[1.0; WATER_GRID_LEN]; WATER_GRID_LEN],
            flow_x: [[0.0; WATER_GRID_LEN]; WATER_GRID_LEN],
            flow_y: [[0.0; WATER_GRID_LEN]; WATER_GRID_LEN],
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct WaterMaterial {
    #[uniform(0)]
    color: Vec4,
}

impl Material for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/water_material.wgsl".into()
    }
    
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
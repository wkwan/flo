use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::asset::{Asset, RenderAssetUsages};
use bevy::pbr::{MaterialPlugin, Material, wireframe::{WireframePlugin, Wireframe}};

const WATER_GRID_LEN: usize = 64;
const GRAVITY: f32 = 10.;
const FRICTION: f32 = 1.5;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WireframePlugin::default())
        .add_plugins(MaterialPlugin::<WaterMaterial>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (water_sim, animate_water_mesh))
        .run();
}

fn animate_water_mesh(
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<&WaterMesh>,
) {
    let elapsed = time.elapsed_secs();
    
    for water_mesh in query.iter() {
        if let Some(mesh) = meshes.get_mut(&water_mesh.handle) {
            if let Some(vertex_attr) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
                if let bevy::render::mesh::VertexAttributeValues::Float32x3(positions) = vertex_attr {
                    let grid_size = 64;
                    let size = 8.0;
                    let step = size / grid_size as f32;
                    
                    // Only print once per second to avoid spam
                    if elapsed as u32 % 60 == 0 && elapsed.fract() < 0.016 {
                        println!("Animating water mesh at time: {:.2}", elapsed);
                    }
                    
                    for (i, pos) in positions.iter_mut().enumerate() {
                        let x = i % (grid_size + 1);
                        let y = i / (grid_size + 1);
                        
                        let x_pos = (x as f32 * step) - (size / 2.0);
                        let z_pos = (y as f32 * step) - (size / 2.0);
                        
                        // Animate with time-varying sine waves
                        let freq1 = 0.5;
                        let freq2 = 1.3;
                        let freq3 = 2.1;
                        
                        let height = 0.1 * (x_pos * freq1 + elapsed * 2.0).sin() * (z_pos * freq1).cos()
                                   + 0.05 * (x_pos * freq2 + elapsed * 3.0).sin() * (z_pos * freq2 + elapsed * 1.5).cos()
                                   + 0.03 * (x_pos * freq3 - elapsed * 4.0).sin() * (z_pos * freq3 + elapsed * 2.5).cos();
                        
                        pos[1] = height;
                    }
                }
            }
        }
    }
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

fn create_water_mesh(size: f32, grid_size: u32) -> Mesh {
    let vertices_per_side = grid_size + 1;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let step = size / grid_size as f32;

    // Generate vertices (flat initially, will be animated)
    for y in 0..vertices_per_side {
        for x in 0..vertices_per_side {
            let x_pos = (x as f32 * step) - (size / 2.0);
            let z_pos = (y as f32 * step) - (size / 2.0);
            
            positions.push([x_pos, 0.0, z_pos]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / grid_size as f32, y as f32 / grid_size as f32]);
        }
    }

    // Generate indices for triangles
    for y in 0..grid_size {
        for x in 0..grid_size {
            let base = y * vertices_per_side + x;
            
            // First triangle
            indices.push(base);
            indices.push(base + vertices_per_side);
            indices.push(base + 1);
            
            // Second triangle
            indices.push(base + 1);
            indices.push(base + vertices_per_side);
            indices.push(base + vertices_per_side + 1);
        }
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
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

    // Water plane with 64x64 grid
    let water_mesh_handle = meshes.add(create_water_mesh(8.0, 64));
    commands.spawn((
        Mesh3d(water_mesh_handle.clone()),
        MeshMaterial3d(water_materials.add(WaterMaterial {
            color: Vec4::new(0.1, 0.3, 0.8, 0.2),
        })),
        Transform::default(),
        WaterData::default(),
        WaterMesh { handle: water_mesh_handle },
        Wireframe,
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

#[derive(Component)]
struct WaterMesh {
    handle: Handle<Mesh>,
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
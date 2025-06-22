use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::asset::{Asset, RenderAssetUsages};
use bevy::pbr::{MaterialPlugin, Material, wireframe::{WireframePlugin, Wireframe}};

const WATER_GRID_LEN: usize = 64;
const GRAVITY: f32 = 10.;
const FRICTION: f32 = 0.6;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WireframePlugin::default())
        .add_plugins(MaterialPlugin::<WaterMaterial>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (water_sim, animate_water_mesh, handle_mouse_clicks))
        .run();
}

fn animate_water_mesh(
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<(&WaterMesh, &WaterData)>,
) {
    for (water_mesh, water_data) in query.iter() {
        if let Some(mesh) = meshes.get_mut(&water_mesh.handle) {
            if let Some(vertex_attr) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
                if let bevy::render::mesh::VertexAttributeValues::Float32x3(positions) = vertex_attr {
                    for (i, pos) in positions.iter_mut().enumerate() {
                        let x = i % (WATER_GRID_LEN + 1);
                        let y = i / (WATER_GRID_LEN + 1);
                        
                        // Clamp to grid bounds and use water simulation height
                        let grid_x = x.min(WATER_GRID_LEN - 1);
                        let grid_y = y.min(WATER_GRID_LEN - 1);
                        
                        pos[1] = water_data.height[grid_x][grid_y] - 1.0;
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

        // Print entire grid
        // println!("\n=== Water Height Grid ===");
        // for y in 0..WATER_GRID_LEN {
        //     for x in 0..WATER_GRID_LEN {
        //         print!("{:5.2} ", water_data.height[x][y]);
        //     }
        //     println!();
        // }
        // println!("========================\n");
    }
}

fn handle_mouse_clicks(
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut water_query: Query<&mut WaterData>,
) {
    if mouse_button.pressed(MouseButton::Left) {
        if let Ok((camera, camera_transform)) = camera_query.single() {
            if let Ok(window) = windows.single() {
                if let Some(cursor_position) = window.cursor_position() {
                    // Create a ray from the camera through the cursor
                    if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
                        // Calculate intersection with water plane (y = 0)
                        let t = -ray.origin.y / ray.direction.y;
                        if t > 0.0 {
                            let hit_point = ray.origin + ray.direction * t;
                            
                            // Convert world position to grid coordinates
                            let grid_x = ((hit_point.x + 4.0) / 8.0 * WATER_GRID_LEN as f32) as usize;
                            let grid_y = ((hit_point.z + 4.0) / 8.0 * WATER_GRID_LEN as f32) as usize;
                            
                            // Apply disturbance if within bounds
                            if grid_x < WATER_GRID_LEN && grid_y < WATER_GRID_LEN {
                                for mut water_data in water_query.iter_mut() {
                                    // Only disturb if we moved to a new grid cell
                                    let should_disturb = match water_data.last_disturbed_pos {
                                        Some((last_x, last_y)) => last_x != grid_x || last_y != grid_y,
                                        None => true,
                                    };
                                    
                                    if should_disturb {
                                        water_data.height[grid_x][grid_y] += 1.0;
                                        water_data.last_disturbed_pos = Some((grid_x, grid_y));
                                        // println!("Disturbing at grid position: ({}, {})", grid_x, grid_y);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Reset last position when mouse is released
        for mut water_data in water_query.iter_mut() {
            water_data.last_disturbed_pos = None;
        }
    }
}

fn create_scaled_uv_cuboid(width: f32, height: f32, depth: f32) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    
    let hw = width * 0.5;
    let hh = height * 0.5;
    let hd = depth * 0.5;
    
    // Calculate UV scales based on face dimensions
    let front_back_u_scale = width / height.max(width).max(depth);
    let front_back_v_scale = height / height.max(width).max(depth);
    let left_right_u_scale = depth / height.max(width).max(depth);
    let left_right_v_scale = height / height.max(width).max(depth);
    let top_bottom_u_scale = width / height.max(width).max(depth);
    let top_bottom_v_scale = depth / height.max(width).max(depth);
    
    // Front face (+Z)
    positions.extend_from_slice(&[
        [-hw, -hh, hd], [hw, -hh, hd], [hw, hh, hd], [-hw, hh, hd]
    ]);
    normals.extend_from_slice(&[[0.0, 0.0, 1.0]; 4]);
    uvs.extend_from_slice(&[
        [0.0, 0.0], [front_back_u_scale, 0.0], 
        [front_back_u_scale, front_back_v_scale], [0.0, front_back_v_scale]
    ]);
    indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    
    // Back face (-Z)
    positions.extend_from_slice(&[
        [hw, -hh, -hd], [-hw, -hh, -hd], [-hw, hh, -hd], [hw, hh, -hd]
    ]);
    normals.extend_from_slice(&[[0.0, 0.0, -1.0]; 4]);
    uvs.extend_from_slice(&[
        [0.0, 0.0], [front_back_u_scale, 0.0], 
        [front_back_u_scale, front_back_v_scale], [0.0, front_back_v_scale]
    ]);
    indices.extend_from_slice(&[4, 5, 6, 4, 6, 7]);
    
    // Left face (-X)
    positions.extend_from_slice(&[
        [-hw, -hh, -hd], [-hw, -hh, hd], [-hw, hh, hd], [-hw, hh, -hd]
    ]);
    normals.extend_from_slice(&[[-1.0, 0.0, 0.0]; 4]);
    uvs.extend_from_slice(&[
        [0.0, 0.0], [left_right_u_scale, 0.0], 
        [left_right_u_scale, left_right_v_scale], [0.0, left_right_v_scale]
    ]);
    indices.extend_from_slice(&[8, 9, 10, 8, 10, 11]);
    
    // Right face (+X)
    positions.extend_from_slice(&[
        [hw, -hh, hd], [hw, -hh, -hd], [hw, hh, -hd], [hw, hh, hd]
    ]);
    normals.extend_from_slice(&[[1.0, 0.0, 0.0]; 4]);
    uvs.extend_from_slice(&[
        [0.0, 0.0], [left_right_u_scale, 0.0], 
        [left_right_u_scale, left_right_v_scale], [0.0, left_right_v_scale]
    ]);
    indices.extend_from_slice(&[12, 13, 14, 12, 14, 15]);
    
    // Top face (+Y)
    positions.extend_from_slice(&[
        [-hw, hh, hd], [hw, hh, hd], [hw, hh, -hd], [-hw, hh, -hd]
    ]);
    normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);
    uvs.extend_from_slice(&[
        [0.0, 0.0], [top_bottom_u_scale, 0.0], 
        [top_bottom_u_scale, top_bottom_v_scale], [0.0, top_bottom_v_scale]
    ]);
    indices.extend_from_slice(&[16, 17, 18, 16, 18, 19]);
    
    // Bottom face (-Y)
    positions.extend_from_slice(&[
        [-hw, -hh, -hd], [hw, -hh, -hd], [hw, -hh, hd], [-hw, -hh, hd]
    ]);
    normals.extend_from_slice(&[[0.0, -1.0, 0.0]; 4]);
    uvs.extend_from_slice(&[
        [0.0, 0.0], [top_bottom_u_scale, 0.0], 
        [top_bottom_u_scale, top_bottom_v_scale], [0.0, top_bottom_v_scale]
    ]);
    indices.extend_from_slice(&[20, 21, 22, 20, 22, 23]);
    
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
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
    // Camera - positioned to show all walls and water plane
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 12.0).looking_at(Vec3::new(0.0, 0.0, -2.0), Vec3::Y),
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
        Mesh3d(meshes.add(create_scaled_uv_cuboid(wall_thickness, wall_height, water_size))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-half_water - wall_thickness * 0.5, wall_height * 0.5 - 2.0, 0.0),
    ));

    // Right wall (X = +half_water)
    commands.spawn((
        Mesh3d(meshes.add(create_scaled_uv_cuboid(wall_thickness, wall_height, water_size))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(half_water + wall_thickness * 0.5, wall_height * 0.5 - 2.0, 0.0),
    ));

    // Back wall (Z = -half_water)
    commands.spawn((
        Mesh3d(meshes.add(create_scaled_uv_cuboid(water_size + wall_thickness * 2.0, wall_height, wall_thickness))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(0.0, wall_height * 0.5 - 2.0, -half_water - wall_thickness * 0.5),
    ));

    // Bottom wall
    let bottom_wall_y = -2.0;
    commands.spawn((
        Mesh3d(meshes.add(create_scaled_uv_cuboid(water_size + wall_thickness * 2.0, wall_thickness, water_size + wall_thickness * 2.0))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(0.0, bottom_wall_y, 0.0),
    ));
}

#[derive(Component)]
struct WaterData {
    height: [[f32; WATER_GRID_LEN]; WATER_GRID_LEN],
    flow_x: [[f32; WATER_GRID_LEN]; WATER_GRID_LEN],
    flow_y: [[f32; WATER_GRID_LEN]; WATER_GRID_LEN],
    last_disturbed_pos: Option<(usize, usize)>,
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
            last_disturbed_pos: None,
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
use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::asset::{Asset, RenderAssetUsages};
use bevy::pbr::{MaterialPlugin, Material, wireframe::WireframePlugin};
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, DiagnosticsStore};

const WATER_GRID_LEN: usize = 64;
const GRAVITY: f32 = 10.;
const FRICTION: f32 = 0.6;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WireframePlugin::default())
        .add_plugins(MaterialPlugin::<WaterMaterial>::default())
        .add_plugins(MaterialPlugin::<SkyMaterial>::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_systems(Startup, (setup, setup_diagnostics_ui))
        .add_systems(Update, (water_sim, animate_water_mesh, update_water_material, update_sky_material, handle_mouse_clicks, update_diagnostics_text))
        .run();
}

fn animate_water_mesh(
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<(&WaterMesh, &WaterData)>,
) {
    for (water_mesh, water_data) in query.iter() {
        if let Some(mesh) = meshes.get_mut(&water_mesh.handle) {
            // Update positions
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
            
            // Calculate and update normals based on height differences
            // First, we need to get the positions to calculate normals
            let positions_copy = if let Some(pos_attr) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                if let bevy::render::mesh::VertexAttributeValues::Float32x3(positions) = pos_attr {
                    Some(positions.clone())
                } else {
                    None
                }
            } else {
                None
            };
            
            if let Some(positions) = positions_copy {
                if let Some(norm_attr) = mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL) {
                    if let bevy::render::mesh::VertexAttributeValues::Float32x3(normals) = norm_attr {
                        let grid_size = WATER_GRID_LEN + 1;
                        let grid_scale = 8.0 / WATER_GRID_LEN as f32; // Same scale as used for mesh creation
                        
                        for i in 0..normals.len() {
                            let x = i % grid_size;
                            let y = i / grid_size;
                            
                            // Calculate gradients using neighboring heights
                            let mut dx = 0.0;
                            let mut dy = 0.0;
                            
                            // X gradient
                            if x > 0 && x < grid_size - 1 {
                                let h_left = positions[i - 1][1];
                                let h_right = positions[i + 1][1];
                                dx = (h_right - h_left) / (2.0 * grid_scale);
                            }
                            
                            // Y gradient
                            if y > 0 && y < grid_size - 1 {
                                let h_up = positions[i - grid_size][1];
                                let h_down = positions[i + grid_size][1];
                                dy = (h_down - h_up) / (2.0 * grid_scale);
                            }
                            
                            // Normal = (-dx, 1, -dy) normalized
                            let normal = Vec3::new(-dx, 1.0, -dy).normalize();
                            normals[i] = [normal.x, normal.y, normal.z];
                        }
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

        for x in 0..WATER_GRID_LEN {
            for y in 0..WATER_GRID_LEN {
                // Calculate flow_x (horizontal flow from left to right)
                if x > 0 {
                    let source_has_wall = water_data.wall_mask[x-1][y];
                    let dest_has_wall = water_data.wall_mask[x][y];
                    let height_diff = water_data.height[x-1][y] - water_data.height[x][y];
                    
                    // Allow flow only if both source and destination have no walls
                    if !source_has_wall && !dest_has_wall {
                        let new_flow = water_data.flow_x[x][y] * FRICTION.powf(delta_time) + 
                            height_diff * GRAVITY * delta_time;
                        
                        water_data.flow_x[x][y] = new_flow;
                    } else {
                        // One or both cells have walls - no flow
                        water_data.flow_x[x][y] = 0.0;
                    }
                } else {
                    water_data.flow_x[x][y] = 0.0;
                }
                
                // Calculate flow_y (vertical flow from top to bottom)
                if y > 0 {
                    let source_has_wall = water_data.wall_mask[x][y-1];
                    let dest_has_wall = water_data.wall_mask[x][y];
                    let height_diff = water_data.height[x][y-1] - water_data.height[x][y];
                    
                    // Allow flow only if both source and destination have no walls
                    if !source_has_wall && !dest_has_wall {
                        let new_flow = water_data.flow_y[x][y] * FRICTION.powf(delta_time) + 
                            height_diff * GRAVITY * delta_time;
                        
                        water_data.flow_y[x][y] = new_flow;
                    } else {
                        // One or both cells have walls - no flow
                        water_data.flow_y[x][y] = 0.0;
                    }
                } else {
                    water_data.flow_y[x][y] = 0.0;
                }
            }
        }

        // Prevent water from flowing faster than available
        for x in 0..WATER_GRID_LEN {
            for y in 0..WATER_GRID_LEN {
                if water_data.wall_mask[x][y] {
                    continue;
                }

                let mut total_outflow = 0.;
                total_outflow += 0.0f32.max(-water_data.flow_x[x][y]);
                total_outflow += 0.0f32.max(-water_data.flow_y[x][y]);
                
                if x < WATER_GRID_LEN - 1 {
                    total_outflow += 0.0f32.max(water_data.flow_x[x+1][y]);
                }
                if y < WATER_GRID_LEN - 1 {
                    total_outflow += 0.0f32.max(water_data.flow_y[x][y+1]);
                }

                let max_outflow = water_data.height[x][y] / delta_time;

                if total_outflow > 0. {
                    let scale = 1.0f32.min(max_outflow / total_outflow);
                    if water_data.flow_x[x][y] < 0. {
                        water_data.flow_x[x][y] *= scale;
                    } 
                    if water_data.flow_y[x][y] < 0. {
                        water_data.flow_y[x][y] *= scale;
                    }
                    if x < WATER_GRID_LEN - 1 && water_data.flow_x[x+1][y] > 0. {
                        water_data.flow_x[x+1][y] *= scale;
                    }
                    if y < WATER_GRID_LEN - 1 && water_data.flow_y[x][y+1] > 0. {
                        water_data.flow_y[x][y+1] *= scale;
                    }
                }
            }
        }

        // Update heights based on flows, with proper wall handling
        for x in 0..WATER_GRID_LEN {
            for y in 0..WATER_GRID_LEN {
                let mut height_change = 0.0;
                
                // Inflow from left (blocked if current cell has a wall)
                let can_receive_from_left = x > 0 && !water_data.wall_mask[x-1][y] && !water_data.wall_mask[x][y];
                if can_receive_from_left {
                    height_change += water_data.flow_x[x][y];
                }
                
                // Inflow from top (blocked if current cell has a wall)
                let can_receive_from_top = y > 0 && !water_data.wall_mask[x][y-1] && !water_data.wall_mask[x][y];
                if can_receive_from_top {
                    height_change += water_data.flow_y[x][y];
                } 
                
                // Outflow to right (allow outflow from walls, but not into walls)
                let can_flow_right = x < WATER_GRID_LEN - 1 && !water_data.wall_mask[x+1][y];
                if can_flow_right {
                    height_change -= water_data.flow_x[x+1][y];
                }
                
                // Outflow to bottom (allow outflow from walls, but not into walls)
                let can_flow_bottom = y < WATER_GRID_LEN - 1 && !water_data.wall_mask[x][y+1];
                if can_flow_bottom {
                    height_change -= water_data.flow_y[x][y+1];
                }
                
                water_data.height[x][y] += height_change * delta_time;
                
                // Ensure water height stays positive
                water_data.height[x][y] = water_data.height[x][y].max(0.1);
                
                // Force wall cells to have minimal water height
                if water_data.wall_mask[x][y] {
                    water_data.height[x][y] = 0.1;
                }
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

fn update_water_material(
    time: Res<Time>,
    camera_query: Query<&Transform, With<Camera3d>>,
    windows: Query<&Window>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    water_query: Query<&MeshMaterial3d<WaterMaterial>>,
) {
    // Get camera position
    let camera_position = if let Ok(camera_transform) = camera_query.single() {
        camera_transform.translation
    } else {
        Vec3::ZERO
    };
    
    // Get window resolution
    let resolution = if let Ok(window) = windows.single() {
        Vec2::new(window.width(), window.height())
    } else {
        Vec2::new(1920.0, 1080.0)
    };
    
    // Update all water materials
    for material_handle in water_query.iter() {
        if let Some(material) = water_materials.get_mut(&material_handle.0) {
            material.time = time.elapsed_secs();
            material.camera_position = camera_position;
            material.resolution = resolution;
        }
    }
}

fn update_sky_material(
    camera_query: Query<&Transform, With<Camera3d>>,
    mut sky_materials: ResMut<Assets<SkyMaterial>>,
    sky_query: Query<&MeshMaterial3d<SkyMaterial>, With<SkyDome>>,
) {
    // Get camera position
    let camera_position = if let Ok(camera_transform) = camera_query.single() {
        camera_transform.translation
    } else {
        Vec3::ZERO
    };
    
    // Update all sky materials
    for material_handle in sky_query.iter() {
        if let Some(material) = sky_materials.get_mut(&material_handle.0) {
            material.camera_position = camera_position;
        }
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
                                    // Skip displacement for wall cells
                                    if water_data.wall_mask[grid_x][grid_y] {
                                        continue;
                                    }
                                    
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

fn create_sky_dome() -> Mesh {
    // Create a large sphere that surrounds the scene
    let radius = 100.0;
    let rings = 16;
    let sectors = 32;
    
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    
    // Generate vertices
    for i in 0..=rings {
        let theta = i as f32 * std::f32::consts::PI / rings as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        
        for j in 0..=sectors {
            let phi = j as f32 * 2.0 * std::f32::consts::PI / sectors as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();
            
            let x = radius * sin_theta * cos_phi;
            let y = radius * cos_theta;
            let z = radius * sin_theta * sin_phi;
            
            positions.push([x, y, z]);
            // Normals point inward for a sky dome
            normals.push([-x / radius, -y / radius, -z / radius]);
            uvs.push([j as f32 / sectors as f32, i as f32 / rings as f32]);
        }
    }
    
    // Generate indices
    for i in 0..rings {
        for j in 0..sectors {
            let first = i * (sectors + 1) + j;
            let second = first + sectors + 1;
            
            indices.push(first as u32);
            indices.push(second as u32);
            indices.push((first + 1) as u32);
            
            indices.push(second as u32);
            indices.push((second + 1) as u32);
            indices.push((first + 1) as u32);
        }
    }
    
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
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
    mut sky_materials: ResMut<Assets<SkyMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Camera - positioned to show all walls and water plane
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 12.0).looking_at(Vec3::new(0.0, 0.0, -2.0), Vec3::Y),
    ));

    // Sky dome with gradient - positioned far away
    let sky_mesh_handle = meshes.add(create_sky_dome());
    let sky_material_handle = sky_materials.add(SkyMaterial::new());
    commands.spawn((
        Mesh3d(sky_mesh_handle),
        MeshMaterial3d(sky_material_handle),
        Transform::from_scale(Vec3::splat(0.95)), // Make slightly smaller to avoid z-fighting
        SkyDome,
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
    let ao_texture: Handle<Image> = asset_server.load("Stone Wall/Stone_Wall_ambientOcclusion.jpg");

    // Stone wall material with textures
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.3, 0.0),
        base_color_texture: Some(base_color_texture),
        normal_map_texture: Some(normal_texture),
        metallic_roughness_texture: Some(roughness_texture),
        occlusion_texture: Some(ao_texture),
        ..default()
    });

    // Water plane with 64x64 grid
    let water_mesh_handle = meshes.add(create_water_mesh(8.0, 64));
    let water_material_handle = water_materials.add(WaterMaterial::new(Color::srgba(0.1, 0.3, 0.8, 0.8)));
    
    // Initialize water data with wall boundaries
    let mut water_data = WaterData::default();
    
    // Set wall mask for boundary cells (edges of the water plane)
    for i in 0..WATER_GRID_LEN {
        // Top and bottom edges
        water_data.wall_mask[i][0] = true;
        water_data.wall_mask[i][WATER_GRID_LEN - 1] = true;
        
        // Left and right edges
        water_data.wall_mask[0][i] = true;
        water_data.wall_mask[WATER_GRID_LEN - 1][i] = true;
    }
    
    commands.spawn((
        Mesh3d(water_mesh_handle.clone()),
        MeshMaterial3d(water_material_handle),
        Transform::default(),
        water_data,
        WaterMesh { handle: water_mesh_handle },
        // Wireframe, // enable wireframe for debugging
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
    wall_mask: [[bool; WATER_GRID_LEN]; WATER_GRID_LEN], // Track where walls are placed
}

#[derive(Component)]
struct WaterMesh {
    handle: Handle<Mesh>,
}

#[derive(Component)]
struct SkyDome;

impl Default for WaterData {
    fn default() -> Self {
        Self {
            height: [[1.0; WATER_GRID_LEN]; WATER_GRID_LEN],
            flow_x: [[0.0; WATER_GRID_LEN]; WATER_GRID_LEN],
            flow_y: [[0.0; WATER_GRID_LEN]; WATER_GRID_LEN],
            last_disturbed_pos: None,
            wall_mask: [[false; WATER_GRID_LEN]; WATER_GRID_LEN], // No walls initially
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct WaterMaterial {
    #[uniform(0)]
    color: Vec4,
    #[uniform(0)]
    time: f32,
    #[uniform(0)]
    camera_position: Vec3,
    #[uniform(0)]
    resolution: Vec2,
    #[uniform(0)]
    water_level: f32,
    #[uniform(0)]
    grid_scale: f32,
}

impl WaterMaterial {
    fn new(color: Color) -> Self {
        Self {
            color: Vec4::new(color.to_linear().red, color.to_linear().green, color.to_linear().blue, color.to_linear().alpha),
            time: 0.0,
            camera_position: Vec3::ZERO,
            resolution: Vec2::new(1920.0, 1080.0), // Default resolution
            water_level: 0.0,
            grid_scale: 8.0 / WATER_GRID_LEN as f32, // Scale based on water size
        }
    }
}

impl Material for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/water_material.wgsl".into()
    }
    
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct SkyMaterial {
    #[uniform(0)]
    camera_position: Vec3,
    #[uniform(0)]
    _padding: f32, // Pad to 16 bytes for proper alignment
}

impl SkyMaterial {
    fn new() -> Self {
        Self {
            camera_position: Vec3::ZERO,
            _padding: 0.0,
        }
    }
}

impl Material for SkyMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sky_gradient.wgsl".into()
    }
    
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
    
    // Ensure sky renders behind everything else  
    fn depth_bias(&self) -> f32 {
        1000.0 // Render far back
    }
}

#[derive(Component)]
struct DiagnosticsText;

fn setup_diagnostics_ui(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
        DiagnosticsText,
    ));
}

fn update_diagnostics_text(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<DiagnosticsText>>,
) {
    for mut text in query.iter_mut() {
        text.0.clear();
        
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                text.0.push_str(&format!("FPS: {:.1}\n", value));
            }
        }
        
        if let Some(frame_time) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME) {
            if let Some(value) = frame_time.smoothed() {
                text.0.push_str(&format!("Frame Time: {:.2}ms\n", value));
            }
        }
    }
}
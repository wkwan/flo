use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapperHolder};
use bevy::input::mouse::MouseButton;
use std::sync::{Arc, Mutex};

use vulkan_bevy_renderer::{
    setup_bevy_app,
    vulkan_renderer_unified::{VulkanRenderer, PushConstants},
    mesh::{MeshData, Vertex},
    fps_logger::FpsLogger,
};

const WATER_GRID_LEN: usize = 64;
const WATER_SIZE: f32 = 8.0;
const WATER_HALF_SIZE: f32 = 4.0; // WATER_SIZE * 0.5
const GRAVITY: f32 = 10.;
const FRICTION: f32 = 0.6;

fn main() {
    let mut app = setup_bevy_app();
    
    app.insert_resource(WaterSimData::default())
        .add_systems(PostStartup, setup_vulkan_renderer)
        .add_systems(
            Update,
            (
                water_sim,
                handle_mouse_clicks,
                render_frame,
            ).run_if(resource_exists::<VulkanContext>),
        )
        .run();
}

#[derive(Resource)]
struct VulkanContext {
    renderer: Arc<Mutex<Option<VulkanRenderer>>>,
    water_mesh_index: Option<usize>,
}

#[derive(Resource, Clone)]
struct WaterSimData {
    height: [[f32; WATER_GRID_LEN]; WATER_GRID_LEN],
    flow_x: [[f32; WATER_GRID_LEN]; WATER_GRID_LEN],
    flow_y: [[f32; WATER_GRID_LEN]; WATER_GRID_LEN],
    last_disturbed_pos: Option<(usize, usize)>,
    wall_mask: [[bool; WATER_GRID_LEN]; WATER_GRID_LEN],
    grid_screen_positions: Vec<Vec<[f32; 2]>>, // Screen positions for each grid vertex
}

impl Default for WaterSimData {
    fn default() -> Self {
        let mut water_data = Self {
            height: [[1.0; WATER_GRID_LEN]; WATER_GRID_LEN],
            flow_x: [[0.0; WATER_GRID_LEN]; WATER_GRID_LEN],
            flow_y: [[0.0; WATER_GRID_LEN]; WATER_GRID_LEN],
            last_disturbed_pos: None,
            wall_mask: [[false; WATER_GRID_LEN]; WATER_GRID_LEN],
            grid_screen_positions: vec![vec![[0.0, 0.0]; WATER_GRID_LEN + 1]; WATER_GRID_LEN + 1],
        };
        
        // Set wall mask for boundary cells
        for i in 0..WATER_GRID_LEN {
            water_data.wall_mask[i][0] = true;
            water_data.wall_mask[i][WATER_GRID_LEN - 1] = true;
            water_data.wall_mask[0][i] = true;
            water_data.wall_mask[WATER_GRID_LEN - 1][i] = true;
        }
        
        water_data
    }
}

fn setup_vulkan_renderer(
    mut commands: Commands,
    windows: Query<(Entity, &RawHandleWrapperHolder, &Window), With<PrimaryWindow>>,
    mut water_data: ResMut<WaterSimData>,
) {
    let (_entity, handle_wrapper, window) = windows.single().expect("Failed to get primary window");
    
    // Compute initial screen positions for the water grid
    compute_grid_screen_positions(&mut water_data, window.width(), window.height());
    
    // Create a basic mesh for initializing the renderer (won't be rendered)
    let basic_mesh = MeshData {
        vertices: vec![
            Vertex { position: [0.0, 0.0, 0.0], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color: [1.0, 1.0, 1.0, 1.0] },
            Vertex { position: [1.0, 0.0, 0.0], normal: [0.0, 1.0, 0.0], uv: [1.0, 0.0], color: [1.0, 1.0, 1.0, 1.0] },
            Vertex { position: [0.0, 0.0, 1.0], normal: [0.0, 1.0, 0.0], uv: [0.0, 1.0], color: [1.0, 1.0, 1.0, 1.0] },
        ],
        indices: vec![0, 1, 2],
    };
    
    let vulkan_renderer = Arc::new(Mutex::new(None));
    let vulkan_renderer_clone = vulkan_renderer.clone();
    
    // Create the renderer with a basic mesh
    match VulkanRenderer::new_from_mesh_data(
        handle_wrapper,
        "shaders/mesh.vert.spv",
        "shaders/mesh.frag.spv",
        &basic_mesh,
        1,
    ) {
        Ok(mut renderer) => {
            // Add sky pipeline (rendered first for background)
            if let Err(e) = renderer.add_fluid_pipeline("sky", "shaders/sky.vert.spv", "shaders/sky.frag.spv") {
                eprintln!("Failed to add sky pipeline: {}", e);
                return;
            }
            
            // Add fluid rendering pipelines
            if let Err(e) = renderer.add_fluid_pipeline("water", "shaders/water.vert.spv", "shaders/water.frag.spv") {
                eprintln!("Failed to add water pipeline: {}", e);
                return;
            }
            
            // Try to add wall pipeline with actual stone wall textures
            match renderer.add_wall_pipeline_with_textures() {
                Ok(_) => {
                    println!("Successfully loaded wall pipeline with stone textures");
                }
                Err(e) => {
                    eprintln!("Failed to load textured wall pipeline: {}", e);
                    return;
                }
            }
            
            // Create and add water mesh
            let water_mesh_data = create_water_mesh();
            let water_mesh_index;
            
            println!("Water mesh has {} vertices and {} indices", water_mesh_data.vertices.len(), water_mesh_data.indices.len());
            match renderer.add_mesh(&water_mesh_data) {
                Ok(water_index) => {
                    // Set transform for the mesh so it gets rendered
                    renderer.update_mesh_transforms(water_index, vec![bevy::math::Mat4::IDENTITY]);
                    // Set the mesh to use the water pipeline
                    renderer.set_mesh_pipeline(water_index, "water");
                    water_mesh_index = Some(water_index);
                    println!("Added water mesh at index {} with water pipeline", water_index);
                }
                Err(e) => {
                    eprintln!("Failed to add water mesh: {}", e);
                    return;
                }
            }
            
            // Create and add wall mesh
            let wall_mesh_data = create_wall_mesh();
            
            println!("Wall mesh has {} vertices and {} indices", wall_mesh_data.vertices.len(), wall_mesh_data.indices.len());
            match renderer.add_mesh(&wall_mesh_data) {
                Ok(wall_index) => {
                    // Set transform for the mesh so it gets rendered
                    renderer.update_mesh_transforms(wall_index, vec![bevy::math::Mat4::IDENTITY]);
                    // Set the mesh to use the wall pipeline
                    renderer.set_mesh_pipeline(wall_index, "wall");
                    println!("Added wall mesh at index {} with wall pipeline", wall_index);
                }
                Err(e) => {
                    eprintln!("Failed to add wall mesh: {}", e);
                    return;
                }
            }
            
            // Store the renderer and mesh indices
            *vulkan_renderer_clone.lock().unwrap() = Some(renderer);
            
            commands.insert_resource(VulkanContext { 
                renderer: vulkan_renderer,
                water_mesh_index,
            });
            
            println!("Vulkan renderer created with fluid simulation pipelines");
        }
        Err(e) => {
            eprintln!("Failed to create Vulkan renderer: {}", e);
        }
    }
}

fn compute_grid_screen_positions(water_data: &mut WaterSimData, window_width: f32, window_height: f32) {
    // Build the same view and projection matrices as used in the shader
    // View matrix - camera positioned at (0, 6, 8) looking down
    // Matrix is in column-major format like GLSL
    let view = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, -6.0, -8.0, 1.0],
    ];
    
    // Projection matrix with Y-axis flipped for Vulkan
    // Use the actual window aspect ratio
    let aspect_ratio = window_width / window_height;
    let projection = [
        [1.0 / aspect_ratio, 0.0, 0.0, 0.0],
        [0.0, -1.0, 0.0, 0.0],
        [0.0, 0.0, -1.0, -1.0],
        [0.0, 0.0, -0.2, 0.0],
    ];
    
    let vertices_per_side = WATER_GRID_LEN + 1;
    let size = WATER_SIZE;
    let step = size / WATER_GRID_LEN as f32;
    
    // Compute screen position for each grid vertex
    for y in 0..vertices_per_side {
        for x in 0..vertices_per_side {
            // World position of this grid vertex
            // Water at rest (height=1.0) is at Y=0 in the mesh
            let x_pos = (x as f32 * step) - (size / 2.0);
            let z_pos = (y as f32 * step) - (size / 2.0);
            let world_pos = [x_pos, 0.0, z_pos, 1.0];
            
            // Apply view transformation
            let view_pos = mat4_mul_vec4(&view, &world_pos);
            
            // Apply projection transformation
            let clip_pos = mat4_mul_vec4(&projection, &view_pos);
            
            // Perspective divide to get NDC coordinates
            let ndc_x = clip_pos[0] / clip_pos[3];
            let ndc_y = clip_pos[1] / clip_pos[3];
            
            // Convert NDC (-1 to 1) to screen coordinates (0 to window size)
            let screen_x = (ndc_x + 1.0) * 0.5 * window_width;
            let screen_y = (ndc_y + 1.0) * 0.5 * window_height;
            
            water_data.grid_screen_positions[y][x] = [screen_x, screen_y];
        }
    }
}

fn mat4_mul_vec4(mat: &[[f32; 4]; 4], vec: &[f32; 4]) -> [f32; 4] {
    [
        mat[0][0] * vec[0] + mat[1][0] * vec[1] + mat[2][0] * vec[2] + mat[3][0] * vec[3],
        mat[0][1] * vec[0] + mat[1][1] * vec[1] + mat[2][1] * vec[2] + mat[3][1] * vec[3],
        mat[0][2] * vec[0] + mat[1][2] * vec[1] + mat[2][2] * vec[2] + mat[3][2] * vec[3],
        mat[0][3] * vec[0] + mat[1][3] * vec[1] + mat[2][3] * vec[2] + mat[3][3] * vec[3],
    ]
}

fn water_sim(
    time: Res<Time>,
    mut water_data: ResMut<WaterSimData>,
) {
    let delta_time = time.delta_secs();
    
    // Clear boundary flows
    for i in 0..WATER_GRID_LEN {
        water_data.flow_x[0][i] = 0.;
        water_data.flow_x[WATER_GRID_LEN-1][i] = 0.;
        water_data.flow_y[i][0] = 0.;
        water_data.flow_y[i][WATER_GRID_LEN-1] = 0.;
    }

    // Calculate flows
    for x in 0..WATER_GRID_LEN {
        for y in 0..WATER_GRID_LEN {
            // Calculate flow_x
            if x > 0 {
                let source_has_wall = water_data.wall_mask[x-1][y];
                let dest_has_wall = water_data.wall_mask[x][y];
                let height_diff = water_data.height[x-1][y] - water_data.height[x][y];
                
                if !source_has_wall && !dest_has_wall {
                    let new_flow = water_data.flow_x[x][y] * FRICTION.powf(delta_time) + 
                        height_diff * GRAVITY * delta_time;
                    water_data.flow_x[x][y] = new_flow;
                } else {
                    water_data.flow_x[x][y] = 0.0;
                }
            } else {
                water_data.flow_x[x][y] = 0.0;
            }
            
            // Calculate flow_y
            if y > 0 {
                let source_has_wall = water_data.wall_mask[x][y-1];
                let dest_has_wall = water_data.wall_mask[x][y];
                let height_diff = water_data.height[x][y-1] - water_data.height[x][y];
                
                if !source_has_wall && !dest_has_wall {
                    let new_flow = water_data.flow_y[x][y] * FRICTION.powf(delta_time) + 
                        height_diff * GRAVITY * delta_time;
                    water_data.flow_y[x][y] = new_flow;
                } else {
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

    // Update heights based on flows
    for x in 0..WATER_GRID_LEN {
        for y in 0..WATER_GRID_LEN {
            let mut height_change = 0.0;
            
            let can_receive_from_left = x > 0 && !water_data.wall_mask[x-1][y] && !water_data.wall_mask[x][y];
            if can_receive_from_left {
                height_change += water_data.flow_x[x][y];
            }
            
            let can_receive_from_top = y > 0 && !water_data.wall_mask[x][y-1] && !water_data.wall_mask[x][y];
            if can_receive_from_top {
                height_change += water_data.flow_y[x][y];
            } 
            
            let can_flow_right = x < WATER_GRID_LEN - 1 && !water_data.wall_mask[x+1][y];
            if can_flow_right {
                height_change -= water_data.flow_x[x+1][y];
            }
            
            let can_flow_bottom = y < WATER_GRID_LEN - 1 && !water_data.wall_mask[x][y+1];
            if can_flow_bottom {
                height_change -= water_data.flow_y[x][y+1];
            }
            
            water_data.height[x][y] += height_change * delta_time;
            water_data.height[x][y] = water_data.height[x][y].max(0.1);
            
            if water_data.wall_mask[x][y] {
                water_data.height[x][y] = 0.1;
            }
        }
    }
}

fn handle_mouse_clicks(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut water_data: ResMut<WaterSimData>,
) {
    if mouse_button.pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            // Update screen positions in case window was resized
            compute_grid_screen_positions(&mut water_data, window.width(), window.height());
            
            if let Some(cursor_position) = window.cursor_position() {
                // Find the closest grid cell to the mouse position using pre-computed screen positions
                let mut best_grid_x = 0;
                let mut best_grid_z = 0;
                let mut best_distance_sq = f32::MAX;
                
                // Search through grid cells (not vertices) to find the one containing the cursor
                for grid_z in 0..WATER_GRID_LEN {
                    for grid_x in 0..WATER_GRID_LEN {
                        // Get the four corners of this grid cell
                        let top_left = water_data.grid_screen_positions[grid_z][grid_x];
                        let top_right = water_data.grid_screen_positions[grid_z][grid_x + 1];
                        let bottom_left = water_data.grid_screen_positions[grid_z + 1][grid_x];
                        let bottom_right = water_data.grid_screen_positions[grid_z + 1][grid_x + 1];
                        
                        // Calculate center of the cell (more accurate than using corners)
                        let center_x = (top_left[0] + top_right[0] + bottom_left[0] + bottom_right[0]) / 4.0;
                        let center_y = (top_left[1] + top_right[1] + bottom_left[1] + bottom_right[1]) / 4.0;
                        
                        // Check if cursor is within the cell bounds (approximate check)
                        let min_x = top_left[0].min(top_right[0]).min(bottom_left[0]).min(bottom_right[0]);
                        let max_x = top_left[0].max(top_right[0]).max(bottom_left[0]).max(bottom_right[0]);
                        let min_y = top_left[1].min(top_right[1]).min(bottom_left[1]).min(bottom_right[1]);
                        let max_y = top_left[1].max(top_right[1]).max(bottom_left[1]).max(bottom_right[1]);
                        
                        if cursor_position.x >= min_x && cursor_position.x <= max_x &&
                           cursor_position.y >= min_y && cursor_position.y <= max_y {
                            // Cursor is within this cell's bounds
                            best_grid_x = grid_x;
                            best_grid_z = grid_z;
                            best_distance_sq = 0.0;
                            break;
                        }
                        
                        // Also track the closest cell center as fallback
                        let dx = cursor_position.x - center_x;
                        let dy = cursor_position.y - center_y;
                        let distance_sq = dx * dx + dy * dy;
                        
                        if distance_sq < best_distance_sq {
                            best_distance_sq = distance_sq;
                            best_grid_x = grid_x;
                            best_grid_z = grid_z;
                        }
                    }
                    
                    if best_distance_sq == 0.0 {
                        break; // Found exact cell containing cursor
                    }
                }
                
                if best_grid_x < WATER_GRID_LEN && best_grid_z < WATER_GRID_LEN {
                    if !water_data.wall_mask[best_grid_x][best_grid_z] {
                        let should_disturb = match water_data.last_disturbed_pos {
                            Some((last_x, last_z)) => last_x != best_grid_x || last_z != best_grid_z,
                            None => true,
                        };
                        
                        if should_disturb {
                            water_data.height[best_grid_x][best_grid_z] += 1.0;
                            water_data.last_disturbed_pos = Some((best_grid_x, best_grid_z));
                        }
                    }
                }
            }
        }
    } else {
        water_data.last_disturbed_pos = None;
    }
}

fn render_frame(
    vulkan: Res<VulkanContext>,
    water_data: Res<WaterSimData>,
    mut fps_logger: Local<FpsLogger>,
    time: Res<Time>,
    _windows: Query<&Window>,
) {
    fps_logger.update(&time);
    
    // Update water mesh with current heights
    if let Ok(mut renderer_guard) = vulkan.renderer.lock() {
        if let Some(ref mut renderer) = *renderer_guard {
            if let Some(water_index) = vulkan.water_mesh_index {
                update_water_mesh_heights(renderer, water_index, &water_data.height);
            }
            
            // Get window resolution
            let resolution = if let Ok(window) = _windows.single() {
                [window.width(), window.height()]
            } else {
                [800.0, 600.0]
            };
            
            // Create push constants with camera position matching the original hardcoded values
            let camera_pos = [0.0, 6.0, 8.0];
            let push_constants = PushConstants {
                time: time.elapsed_secs(),
                camera_position_x: camera_pos[0],
                camera_position_y: camera_pos[1], 
                camera_position_z: camera_pos[2],
                resolution,
                water_level: 0.0,
                grid_scale: WATER_SIZE / WATER_GRID_LEN as f32,
            };
            
            // Use the fluid rendering method
            let view = bevy::math::Mat4::IDENTITY;
            let proj = bevy::math::Mat4::IDENTITY;
            renderer.render_frame_fluid(view, proj, &push_constants);
        }
    }
}

fn update_water_mesh_heights(renderer: &mut VulkanRenderer, mesh_index: usize, heights: &[[f32; WATER_GRID_LEN]; WATER_GRID_LEN]) {
    let vertices_per_side = WATER_GRID_LEN + 1;
    let grid_scale = WATER_SIZE / WATER_GRID_LEN as f32;
    let mut new_vertices = Vec::new();
    
    // Generate vertices with updated heights
    for y_idx in 0..vertices_per_side {
        for x_idx in 0..vertices_per_side {
            let x = -WATER_HALF_SIZE + (x_idx as f32 / WATER_GRID_LEN as f32) * WATER_SIZE;
            let z = -WATER_HALF_SIZE + (y_idx as f32 / WATER_GRID_LEN as f32) * WATER_SIZE;
            
            // Get the height for this vertex
            let grid_x = x_idx.min(WATER_GRID_LEN - 1);
            let grid_y = y_idx.min(WATER_GRID_LEN - 1);
            let height = heights[grid_x][grid_y];
            
            // Calculate normal based on neighboring heights
            let mut dx = 0.0;
            let mut dy = 0.0;
            
            if x_idx > 0 && x_idx < vertices_per_side - 1 {
                let h_left = if x_idx > 0 && grid_x > 0 { heights[grid_x - 1][grid_y] } else { heights[grid_x][grid_y] };
                let h_right = if x_idx < WATER_GRID_LEN - 1 { heights[grid_x + 1][grid_y] } else { heights[grid_x][grid_y] };
                dx = (h_right - h_left) / (2.0 * grid_scale);
            }
            
            if y_idx > 0 && y_idx < vertices_per_side - 1 {
                let h_up = if y_idx > 0 && grid_y > 0 { heights[grid_x][grid_y - 1] } else { heights[grid_x][grid_y] };
                let h_down = if y_idx < WATER_GRID_LEN - 1 { heights[grid_x][grid_y + 1] } else { heights[grid_x][grid_y] };
                dy = (h_down - h_up) / (2.0 * grid_scale);
            }
            
            // Normal = (-dx, 1, -dy) normalized
            let normal_len = (dx * dx + 1.0 + dy * dy).sqrt();
            let normal = [-dx / normal_len, 1.0 / normal_len, -dy / normal_len];
            
            let u = x_idx as f32 / WATER_GRID_LEN as f32;
            let v = y_idx as f32 / WATER_GRID_LEN as f32;
            
            new_vertices.push(Vertex {
                position: [x, height - 1.0, z],
                normal,
                uv: [u, v],
                color: [1.0, 1.0, 1.0, 1.0],
            });
        }
    }
    
    // Update the mesh vertices in the renderer
    renderer.update_mesh_vertices_full(mesh_index, &new_vertices);
}

fn create_water_mesh() -> MeshData {
    let vertices_per_side = WATER_GRID_LEN + 1;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    
    // Generate vertices
    for y_idx in 0..vertices_per_side {
        for x_idx in 0..vertices_per_side {
            let x = -WATER_HALF_SIZE + (x_idx as f32 / WATER_GRID_LEN as f32) * WATER_SIZE;
            let z = -WATER_HALF_SIZE + (y_idx as f32 / WATER_GRID_LEN as f32) * WATER_SIZE;
            let u = x_idx as f32 / WATER_GRID_LEN as f32;
            let v = y_idx as f32 / WATER_GRID_LEN as f32;
            
            vertices.push(Vertex {
                position: [x, 0.0, z],
                normal: [0.0, 1.0, 0.0],
                uv: [u, v],
                color: [1.0, 1.0, 1.0, 1.0],
            });
        }
    }
    
    // Generate indices for triangles
    for y in 0..WATER_GRID_LEN {
        for x in 0..WATER_GRID_LEN {
            let top_left = y * vertices_per_side + x;
            let top_right = top_left + 1;
            let bottom_left = (y + 1) * vertices_per_side + x;
            let bottom_right = bottom_left + 1;
            
            // First triangle
            indices.push(top_left as u32);
            indices.push(bottom_left as u32);
            indices.push(top_right as u32);
            
            // Second triangle
            indices.push(top_right as u32);
            indices.push(bottom_left as u32);
            indices.push(bottom_right as u32);
        }
    }
    
    MeshData { vertices, indices }
}

fn create_wall_mesh() -> MeshData {
    let half_size = WATER_HALF_SIZE;
    let wall_height = 2.0;
    let wall_bottom = -1.5; // Extend below water plane (water is at y=-1.0 when at rest)
    
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut vertex_offset = 0;
    
    // Front wall (along -Z) - only render the inner face touching water
    vertices.extend_from_slice(&[
        // Inner face (touching water at -half_size)
        Vertex { position: [-half_size, wall_bottom, -half_size], normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0], color: [1.0, 1.0, 1.0, 1.0] },
        Vertex { position: [half_size, wall_bottom, -half_size], normal: [0.0, 0.0, 1.0], uv: [1.0, 0.0], color: [1.0, 1.0, 1.0, 1.0] },
        Vertex { position: [half_size, wall_height, -half_size], normal: [0.0, 0.0, 1.0], uv: [1.0, 1.0], color: [1.0, 1.0, 1.0, 1.0] },
        Vertex { position: [-half_size, wall_height, -half_size], normal: [0.0, 0.0, 1.0], uv: [0.0, 1.0], color: [1.0, 1.0, 1.0, 1.0] },
    ]);
    
    // Add indices for front wall face
    indices.extend_from_slice(&[
        vertex_offset, vertex_offset + 1, vertex_offset + 2,
        vertex_offset, vertex_offset + 2, vertex_offset + 3,
    ]);
    vertex_offset += 4;
    
    // Back wall (along +Z) - inner face touches water boundary at +half_size
    vertices.extend_from_slice(&[
        // Inner face (touching water at +half_size) 
        Vertex { position: [-half_size, wall_bottom, half_size], normal: [0.0, 0.0, -1.0], uv: [0.0, 0.0], color: [1.0, 1.0, 1.0, 1.0] },
        Vertex { position: [half_size, wall_bottom, half_size], normal: [0.0, 0.0, -1.0], uv: [1.0, 0.0], color: [1.0, 1.0, 1.0, 1.0] },
        Vertex { position: [half_size, wall_height, half_size], normal: [0.0, 0.0, -1.0], uv: [1.0, 1.0], color: [1.0, 1.0, 1.0, 1.0] },
        Vertex { position: [-half_size, wall_height, half_size], normal: [0.0, 0.0, -1.0], uv: [0.0, 1.0], color: [1.0, 1.0, 1.0, 1.0] },
    ]);
    
    // Add indices for back wall
    indices.extend_from_slice(&[
        vertex_offset, vertex_offset + 2, vertex_offset + 1,
        vertex_offset, vertex_offset + 3, vertex_offset + 2,
    ]);
    vertex_offset += 4;
    
    // Left wall (along -X) - inner face touches water boundary at -half_size
    vertices.extend_from_slice(&[
        // Inner face (touching water)
        Vertex { position: [-half_size, wall_bottom, -half_size], normal: [1.0, 0.0, 0.0], uv: [0.0, 0.0], color: [1.0, 1.0, 1.0, 1.0] },
        Vertex { position: [-half_size, wall_bottom, half_size], normal: [1.0, 0.0, 0.0], uv: [1.0, 0.0], color: [1.0, 1.0, 1.0, 1.0] },
        Vertex { position: [-half_size, wall_height, half_size], normal: [1.0, 0.0, 0.0], uv: [1.0, 1.0], color: [1.0, 1.0, 1.0, 1.0] },
        Vertex { position: [-half_size, wall_height, -half_size], normal: [1.0, 0.0, 0.0], uv: [0.0, 1.0], color: [1.0, 1.0, 1.0, 1.0] },
    ]);
    
    // Add indices for left wall (reversed winding for correct facing)
    indices.extend_from_slice(&[
        vertex_offset, vertex_offset + 2, vertex_offset + 1,
        vertex_offset, vertex_offset + 3, vertex_offset + 2,
    ]);
    vertex_offset += 4;
    
    // Right wall (along +X) - inner face touches water boundary at +half_size
    vertices.extend_from_slice(&[
        // Inner face (touching water)
        Vertex { position: [half_size, wall_bottom, -half_size], normal: [-1.0, 0.0, 0.0], uv: [0.0, 0.0], color: [1.0, 1.0, 1.0, 1.0] },
        Vertex { position: [half_size, wall_bottom, half_size], normal: [-1.0, 0.0, 0.0], uv: [1.0, 0.0], color: [1.0, 1.0, 1.0, 1.0] },
        Vertex { position: [half_size, wall_height, half_size], normal: [-1.0, 0.0, 0.0], uv: [1.0, 1.0], color: [1.0, 1.0, 1.0, 1.0] },
        Vertex { position: [half_size, wall_height, -half_size], normal: [-1.0, 0.0, 0.0], uv: [0.0, 1.0], color: [1.0, 1.0, 1.0, 1.0] },
    ]);
    
    // Add indices for right wall (normal winding)
    indices.extend_from_slice(&[
        vertex_offset, vertex_offset + 1, vertex_offset + 2,
        vertex_offset, vertex_offset + 2, vertex_offset + 3,
    ]);
    
    MeshData { vertices, indices }
}
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapperHolder};
use vulkan_bevy_renderer::{setup_bevy_app, vulkan_renderer_unified::VulkanRenderer, mesh::{Vertex, MeshData}, fps_logger::FpsLogger};

fn main() {
    let mut app = setup_bevy_app();
    
    app.add_systems(PostStartup, setup_vulkan_renderer)
        .add_systems(
            Update,
            render_frame.run_if(resource_exists::<VulkanContext>),
        )
        .run();
}

#[derive(Resource)]
struct VulkanContext {
    renderer: VulkanRenderer,
    fps_logger: FpsLogger,
}

fn setup_vulkan_renderer(
    mut commands: Commands,
    windows: Query<(Entity, &RawHandleWrapperHolder, &Window), With<PrimaryWindow>>,
) {
    let (_entity, handle_wrapper, window) = windows.single().expect("Failed to get primary window");
    
    println!("=== Wireframe Cube Example ===");
    println!("Window size: {:?}x{:?}", window.width(), window.height());
    
    // Create wireframe cube mesh
    let (vertices, indices) = create_wireframe_cube_mesh();
    
    // Convert lines to thin triangles (since Vulkan renderer uses TRIANGLE_LIST)
    let triangle_mesh = convert_lines_to_triangles(&vertices, &indices);
    
    println!("Created wireframe mesh with {} vertices and {} indices", 
             triangle_mesh.0.len() / 3, triangle_mesh.1.len());
    
    // Convert to MeshData format
    let mesh_data = create_mesh_data_from_triangles(&triangle_mesh.0, &triangle_mesh.1);
    
    // Create a Vulkan renderer with the wireframe mesh
    let renderer = VulkanRenderer::new_from_mesh_data(
        handle_wrapper,
        "shaders/mesh.vert.spv",  // Use standard mesh shaders for now
        "shaders/mesh.frag.spv",
        &mesh_data,
        1, // Single instance
    ).expect("Failed to create Vulkan renderer");
    
    commands.insert_resource(VulkanContext { 
        renderer,
        fps_logger: FpsLogger::new(),
    });
}

fn create_mesh_data_from_triangles(vertices: &[f32], indices: &[u32]) -> MeshData {
    let mut mesh_vertices = Vec::new();
    
    // Convert flat vertex array to Vertex structs
    for i in (0..vertices.len()).step_by(3) {
        let position = [vertices[i], vertices[i + 1], vertices[i + 2]];
        let normal = [0.0, 1.0, 0.0]; // Simple up normal
        let uv = [0.0, 0.0];
        let color = [0.0, 0.5, 1.0, 1.0]; // Blue wireframe
        
        mesh_vertices.push(Vertex::with_color(position, normal, uv, color));
    }
    
    MeshData::new(mesh_vertices, indices.to_vec())
}

fn create_wireframe_cube_mesh() -> (Vec<f32>, Vec<u32>) {
    let half_size = 1.0;
    
    // Define the 8 vertices of a cube (position only)
    let vertices = vec![
        // Bottom face vertices
        -half_size, -half_size, -half_size,
         half_size, -half_size, -half_size,
         half_size, -half_size,  half_size,
        -half_size, -half_size,  half_size,
        // Top face vertices
        -half_size,  half_size, -half_size,
         half_size,  half_size, -half_size,
         half_size,  half_size,  half_size,
        -half_size,  half_size,  half_size,
    ];
    
    // Define edges as line indices (each pair forms a line)
    let indices = vec![
        // Bottom face edges
        0, 1,  1, 2,  2, 3,  3, 0,
        // Top face edges
        4, 5,  5, 6,  6, 7,  7, 4,
        // Vertical edges connecting bottom to top
        0, 4,  1, 5,  2, 6,  3, 7,
    ];
    
    (vertices, indices)
}

fn convert_lines_to_triangles(vertices: &[f32], line_indices: &[u32]) -> (Vec<f32>, Vec<u32>) {
    let mut triangle_vertices = Vec::new();
    let mut triangle_indices = Vec::new();
    
    let line_thickness = 0.02; // Thickness of the lines
    
    // Convert each line segment to a thin box (12 triangles per line)
    for i in (0..line_indices.len()).step_by(2) {
        let idx0 = (line_indices[i] * 3) as usize;
        let idx1 = (line_indices[i + 1] * 3) as usize;
        
        let v0 = Vec3::new(vertices[idx0], vertices[idx0 + 1], vertices[idx0 + 2]);
        let v1 = Vec3::new(vertices[idx1], vertices[idx1 + 1], vertices[idx1 + 2]);
        
        // Calculate perpendicular vectors for the box
        let dir = (v1 - v0).normalize();
        let perp1 = if dir.x.abs() < 0.9 {
            Vec3::X.cross(dir).normalize()
        } else {
            Vec3::Y.cross(dir).normalize()
        };
        let perp2 = dir.cross(perp1).normalize();
        
        let offset1 = perp1 * line_thickness;
        let offset2 = perp2 * line_thickness;
        
        let base_idx = (triangle_vertices.len() / 3) as u32;
        
        // Create 8 vertices for the box
        // Start point vertices
        add_vertex(&mut triangle_vertices, v0 - offset1 - offset2);
        add_vertex(&mut triangle_vertices, v0 + offset1 - offset2);
        add_vertex(&mut triangle_vertices, v0 + offset1 + offset2);
        add_vertex(&mut triangle_vertices, v0 - offset1 + offset2);
        // End point vertices
        add_vertex(&mut triangle_vertices, v1 - offset1 - offset2);
        add_vertex(&mut triangle_vertices, v1 + offset1 - offset2);
        add_vertex(&mut triangle_vertices, v1 + offset1 + offset2);
        add_vertex(&mut triangle_vertices, v1 - offset1 + offset2);
        
        // Create 12 triangles (2 per face, 6 faces)
        // Front face
        triangle_indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 5]);
        triangle_indices.extend_from_slice(&[base_idx, base_idx + 5, base_idx + 4]);
        // Back face
        triangle_indices.extend_from_slice(&[base_idx + 3, base_idx + 7, base_idx + 6]);
        triangle_indices.extend_from_slice(&[base_idx + 3, base_idx + 6, base_idx + 2]);
        // Top face
        triangle_indices.extend_from_slice(&[base_idx + 2, base_idx + 6, base_idx + 5]);
        triangle_indices.extend_from_slice(&[base_idx + 2, base_idx + 5, base_idx + 1]);
        // Bottom face
        triangle_indices.extend_from_slice(&[base_idx, base_idx + 4, base_idx + 7]);
        triangle_indices.extend_from_slice(&[base_idx, base_idx + 7, base_idx + 3]);
        // Left face
        triangle_indices.extend_from_slice(&[base_idx, base_idx + 3, base_idx + 2]);
        triangle_indices.extend_from_slice(&[base_idx, base_idx + 2, base_idx + 1]);
        // Right face
        triangle_indices.extend_from_slice(&[base_idx + 4, base_idx + 5, base_idx + 6]);
        triangle_indices.extend_from_slice(&[base_idx + 4, base_idx + 6, base_idx + 7]);
    }
    
    (triangle_vertices, triangle_indices)
}

fn add_vertex(vertices: &mut Vec<f32>, pos: Vec3) {
    vertices.push(pos.x);
    vertices.push(pos.y);
    vertices.push(pos.z);
}

fn render_frame(
    mut vulkan: ResMut<VulkanContext>,
    time: Res<Time>,
) {
    vulkan.fps_logger.update(&time);
    
    // Update camera rotation based on time
    let elapsed = time.elapsed_secs();
    
    // Calculate view matrix (rotating camera)
    let eye_x = 3.0 * elapsed.cos();
    let eye_z = 3.0 * elapsed.sin();
    let view = Mat4::look_at_rh(
        Vec3::new(eye_x, 2.0, eye_z),
        Vec3::ZERO,
        Vec3::Y,
    );
    
    // Calculate projection matrix
    let proj = Mat4::perspective_rh_gl(
        45.0_f32.to_radians(),
        16.0 / 9.0,
        0.1,
        100.0,
    );
    
    // Render the frame
    vulkan.renderer.render_frame_with_camera(view, proj);
}
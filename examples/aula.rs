use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapperHolder};
use std::collections::HashMap;

use vulkan_bevy_renderer::{
    setup_bevy_app,
    vulkan_renderer_unified::VulkanRenderer,
    mesh_textured::{TexturedMeshData, TexturedVertex},
    texture::TextureData,
    camera_controller::{CameraController, CameraControllerPlugin},
    fps_logger::FpsLogger,
};

fn process_node(
    node: &gltf::Node,
    parent_transform: &Mat4,
    combined_vertices: &mut Vec<TexturedVertex>,
    combined_indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
    material_to_texture: &HashMap<usize, u32>,
    buffers: &[gltf::buffer::Data],
    min_bounds: &mut Vec3,
    max_bounds: &mut Vec3,
    mesh_ranges: &mut Vec<(String, usize, usize, usize, usize)>,
) {
    // Get node transform
    let node_transform = Mat4::from_cols_array_2d(&node.transform().matrix());
    let world_transform = *parent_transform * node_transform;
    
    // Process mesh if present
    if let Some(mesh) = node.mesh() {
        println!("  Node {:?} has mesh {:?} with transform", node.name(), mesh.name());
        let vertices_before = combined_vertices.len();
        let indices_before = combined_indices.len();
        
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
            
            // Get the texture index for this primitive's material
            let texture_index = if let Some(mat_idx) = primitive.material().index() {
                *material_to_texture.get(&mat_idx).unwrap_or(&0)
            } else {
                0
            };
            
            // Read positions and transform them
            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .expect("Mesh should have positions")
                .collect();
            
            // Read normals (or use default)
            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|iter| iter.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
            
            // Read UVs (or use default)
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|iter| iter.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
            
            let _mesh_start_vertex = combined_vertices.len();
            
            // Create vertices with transformed positions
            for i in 0..positions.len() {
                let pos = Vec3::from(positions[i]);
                let transformed_pos = world_transform.transform_point3(pos);
                
                // Transform normal
                let normal = Vec3::from(normals[i]);
                let transformed_normal = world_transform.transform_vector3(normal).normalize();
                
                // Don't flip coordinates - keep original orientation
                
                // Update bounds using original positions
                min_bounds.x = min_bounds.x.min(transformed_pos.x);
                min_bounds.y = min_bounds.y.min(transformed_pos.y);
                min_bounds.z = min_bounds.z.min(transformed_pos.z);
                max_bounds.x = max_bounds.x.max(transformed_pos.x);
                max_bounds.y = max_bounds.y.max(transformed_pos.y);
                max_bounds.z = max_bounds.z.max(transformed_pos.z);
                
                combined_vertices.push(TexturedVertex::new(
                    transformed_pos.into(),
                    transformed_normal.into(),
                    uvs[i],
                    texture_index,
                ));
            }
            
            // Read and add indices
            if let Some(indices_reader) = reader.read_indices() {
                let indices: Vec<u32> = indices_reader.into_u32().collect();
                for triangle in indices.chunks(3) {
                    if triangle.len() == 3 {
                        combined_indices.push(triangle[0] + *vertex_offset);
                        combined_indices.push(triangle[1] + *vertex_offset);
                        combined_indices.push(triangle[2] + *vertex_offset);
                    }
                }
            }
            
            *vertex_offset += positions.len() as u32;
        }
        
        let vertices_added = combined_vertices.len() - vertices_before;
        let indices_added = combined_indices.len() - indices_before;
        println!("    Added {} vertices and {} indices for mesh {:?}", 
                 vertices_added, indices_added, mesh.name());
    }
    
    // Process children
    for child in node.children() {
        process_node(&child, &world_transform, combined_vertices, combined_indices,
                    vertex_offset, material_to_texture, buffers, 
                    min_bounds, max_bounds, mesh_ranges);
    }
}

fn main() {
    let mut app = setup_bevy_app();
    
    app.add_plugins(CameraControllerPlugin)
        .add_systems(PostStartup, setup_vulkan_renderer)
        .add_systems(
            Update,
            render_frame.run_if(resource_exists::<VulkanContext>),
        )
        .run();
}

#[derive(Resource)]
struct VulkanContext(VulkanRenderer);

fn setup_vulkan_renderer(
    mut commands: Commands,
    windows: Query<(Entity, &RawHandleWrapperHolder, &Window), With<PrimaryWindow>>,
) {
    let (_entity, handle_wrapper, window) = windows.single().expect("Failed to get primary window");
    
    println!("Window size: {:?}x{:?}", window.width(), window.height());
    
    // Load the GLB file manually using gltf crate
    let (document, buffers, images) = gltf::import("assets/Aula.glb")
        .expect("Failed to load GLB file");
    
    println!("Loading Computer model with {} meshes and {} textures", 
             document.meshes().count(), images.len());
    
    // First, add a white placeholder texture at index 0
    let mut textures = Vec::new();
    textures.push(TextureData::placeholder());
    println!("Added placeholder texture at index 0");
    
    // Build a mapping from texture index to image index first
    let mut texture_to_image: HashMap<usize, usize> = HashMap::new();
    for (tex_idx, texture) in document.textures().enumerate() {
        let image_idx = texture.source().index();
        texture_to_image.insert(tex_idx, image_idx);
        println!("Texture {} points to Image {}", tex_idx, image_idx);
    }
    
    // Convert all images to TextureData, but in the order of texture indices
    // First add placeholder images for all possible texture slots
    let max_texture_index = document.textures().count();
    for _ in 0..max_texture_index {
        textures.push(TextureData::placeholder());
    }
    
    // Now replace with actual textures
    for (tex_idx, texture) in document.textures().enumerate() {
        let image_idx = texture.source().index();
        let image = &images[image_idx];
        
        println!("Processing texture {} (from image {}): {}x{}, format: {:?}", 
                 tex_idx, image_idx, image.width, image.height, image.format);
        
        // Convert to RGBA if needed
        let rgba_pixels = match image.format {
            gltf::image::Format::R8G8B8A8 => {
                println!("  Format is already RGBA");
                image.pixels.clone()
            },
            gltf::image::Format::R8G8B8 => {
                println!("  Converting RGB to RGBA");
                // Convert RGB to RGBA
                let mut rgba = Vec::with_capacity(image.pixels.len() * 4 / 3);
                for chunk in image.pixels.chunks(3) {
                    rgba.push(chunk[0]);
                    rgba.push(chunk[1]);
                    rgba.push(chunk[2]);
                    rgba.push(255);
                }
                rgba
            },
            _ => {
                println!("  Unsupported texture format, using placeholder");
                TextureData::placeholder().pixels
            }
        };
        
        // Debug: Check if the texture has reasonable color values
        if rgba_pixels.len() >= 4 {
            println!("  First pixel RGBA: [{}, {}, {}, {}]", 
                     rgba_pixels[0], rgba_pixels[1], rgba_pixels[2], rgba_pixels[3]);
        }
        
        // Place at correct index (tex_idx + 1 because we have placeholder at 0)
        textures[tex_idx + 1] = TextureData::new(rgba_pixels, image.width, image.height);
    }
    
    // Create solid color textures for materials without textures
    let mut next_texture_idx = textures.len();
    
    // Create a mapping from material index to texture index
    let mut material_to_texture: HashMap<usize, u32> = HashMap::new();
    for (mat_idx, material) in document.materials().enumerate() {
        if let Some(base_color_texture) = material.pbr_metallic_roughness().base_color_texture() {
            let texture_idx = base_color_texture.texture().index();
            // Add 1 because we inserted placeholder at index 0
            material_to_texture.insert(mat_idx, (texture_idx + 1) as u32);
            println!("Material {} ({:?}) maps to texture {} (array index {})", 
                     mat_idx, material.name(), texture_idx, texture_idx + 1);
        } else {
            // Create a solid color texture for this material
            let base_color = material.pbr_metallic_roughness().base_color_factor();
            let color_r = (base_color[0] * 255.0) as u8;
            let color_g = (base_color[1] * 255.0) as u8;
            let color_b = (base_color[2] * 255.0) as u8;
            let color_a = (base_color[3] * 255.0) as u8;
            
            // Create a 1x1 texture with the material's base color
            let solid_color_pixels = vec![color_r, color_g, color_b, color_a];
            textures.push(TextureData::new(solid_color_pixels, 1, 1));
            
            material_to_texture.insert(mat_idx, next_texture_idx as u32);
            println!("Material {} ({:?}) uses solid color [{}, {}, {}, {}] (array index {})", 
                     mat_idx, material.name(), color_r, color_g, color_b, color_a, next_texture_idx);
            next_texture_idx += 1;
        }
    }
    
    // Process all meshes and combine them
    let mut combined_vertices = Vec::new();
    let mut combined_indices = Vec::new();
    let mut vertex_offset = 0u32;
    
    // Track bounds for camera positioning
    let mut min_bounds = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max_bounds = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
    
    let mut mesh_count = 0;
    let mut primitive_count = 0;
    
    // Track which mesh contributes to which index range for debugging
    let mut mesh_ranges = Vec::new();
    
    // Process nodes to get transforms for each mesh instance
    println!("\nProcessing scene nodes to apply transforms...");
    for scene in document.scenes() {
        for node in scene.nodes() {
            process_node(&node, &Mat4::IDENTITY, &mut combined_vertices, &mut combined_indices, 
                        &mut vertex_offset, &material_to_texture, &buffers, 
                        &mut min_bounds, &mut max_bounds, &mut mesh_ranges);
        }
    }
    
    // If no scenes, process meshes directly (fallback)
    if document.scenes().len() == 0 {
        println!("No scenes found, processing meshes directly...");
        for mesh in document.meshes() {
        let mesh_start_index = combined_indices.len();
        let mesh_start_vertex = combined_vertices.len();
        println!("Processing mesh {}: {:?}", mesh_count, mesh.name());
        mesh_count += 1;
        
        for primitive in mesh.primitives() {
            primitive_count += 1;
            // Check primitive mode
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                println!("  WARNING: Mesh {:?} uses primitive mode {:?}, not triangles!", 
                         mesh.name(), primitive.mode());
            }
            
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
            
            // Get the texture index for this primitive's material
            let texture_index = if let Some(mat_idx) = primitive.material().index() {
                let tex_idx = *material_to_texture.get(&mat_idx).unwrap_or(&0);
                println!("  Primitive with {} vertices uses material {} -> texture {}", 
                         reader.read_positions().map(|p| p.count()).unwrap_or(0), 
                         mat_idx, tex_idx);
                tex_idx
            } else {
                println!("  Primitive with no material, using texture 0");
                0
            };
            
            // Read positions
            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .expect("Mesh should have positions")
                .collect();
            
            // Read normals (or use default)
            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|iter| iter.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
            
            // Read UVs (or use default)
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|iter| iter.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
            
            // Create vertices with texture index
            let _start_vertex_count = combined_vertices.len();
            for i in 0..positions.len() {
                let pos = positions[i];
                
                // Don't flip Y - projection matrix in renderer handles it
                
                // Update bounds
                min_bounds.x = min_bounds.x.min(pos[0]);
                min_bounds.y = min_bounds.y.min(pos[1]);
                min_bounds.z = min_bounds.z.min(pos[2]);
                max_bounds.x = max_bounds.x.max(pos[0]);
                max_bounds.y = max_bounds.y.max(pos[1]);
                max_bounds.z = max_bounds.z.max(pos[2]);
                
                combined_vertices.push(TexturedVertex::new(
                    pos,
                    normals[i],
                    uvs[i],
                    texture_index,
                ));
            }
            println!("    Added {} vertices with texture index {} (total vertices: {})", 
                     positions.len(), texture_index, combined_vertices.len());
            
            // Read indices and fix winding order by reversing each triangle
            if let Some(indices_reader) = reader.read_indices() {
                let indices: Vec<u32> = indices_reader.into_u32().collect();
                let indices_before = combined_indices.len();
                // Process triangles - reverse winding order for Y-flip
                for triangle in indices.chunks(3) {
                    if triangle.len() == 3 {
                        combined_indices.push(triangle[0] + vertex_offset);
                        combined_indices.push(triangle[1] + vertex_offset);
                        combined_indices.push(triangle[2] + vertex_offset);
                    }
                }
                println!("    Added {} indices (total indices: {})", 
                         combined_indices.len() - indices_before, combined_indices.len());
            } else {
                // No indices provided, generate triangle list
                println!("  Warning: No indices for mesh {:?}, {} vertices", mesh.name(), positions.len());
                for i in (0..positions.len() as u32).step_by(3) {
                    if i + 2 < positions.len() as u32 {
                        combined_indices.push(i + vertex_offset);
                        combined_indices.push(i + 1 + vertex_offset);
                        combined_indices.push(i + 2 + vertex_offset);
                    }
                }
            }
            
            let prev_offset = vertex_offset;
            vertex_offset += positions.len() as u32;
            println!("    Vertex offset: {} -> {}", prev_offset, vertex_offset);
        }
        
        let mesh_end_index = combined_indices.len();
        let mesh_end_vertex = combined_vertices.len();
        if mesh_end_index > mesh_start_index {
            mesh_ranges.push((
                mesh.name().unwrap_or("unnamed").to_string(),
                mesh_start_index,
                mesh_end_index,
                mesh_start_vertex,
                mesh_end_vertex,
            ));
        }
    }
    } // Close the if statement for no scenes
    
    println!("\nFinal statistics:");
    println!("  Meshes processed: {}", mesh_count);
    println!("  Primitives processed: {}", primitive_count);
    println!("  Total vertices: {}", combined_vertices.len());
    println!("  Total indices: {} (forming {} triangles)", 
             combined_indices.len(), combined_indices.len() / 3);
    println!("  Vertex offset at end: {}", vertex_offset);
    
    println!("\nMesh index ranges:");
    for (name, start_idx, end_idx, start_vtx, end_vtx) in &mesh_ranges {
        println!("  {}: indices[{}..{}] ({} indices), vertices[{}..{}] ({} vertices)", 
                 name, start_idx, end_idx, end_idx - start_idx, 
                 start_vtx, end_vtx, end_vtx - start_vtx);
    }
    
    if combined_vertices.is_empty() {
        panic!("No mesh data found in GLB file");
    }
    
    // Debug: Check if any indices are out of bounds
    let max_index = *combined_indices.iter().max().unwrap_or(&0);
    let min_index = *combined_indices.iter().min().unwrap_or(&0);
    println!("\nIndex range check:");
    println!("  Min index: {}", min_index);
    println!("  Max index: {}", max_index);
    println!("  Vertex count: {}", combined_vertices.len());
    if max_index >= combined_vertices.len() as u32 {
        println!("  WARNING: Max index {} >= vertex count {}!", max_index, combined_vertices.len());
    }
    
    // Try rendering only the first few meshes to see if it's a size issue
    let test_limit = false; // Set to true to test with limited geometry
    let (final_vertices, final_indices) = if test_limit {
        // Only use first 5000 indices as a test
        let limit = 5000.min(combined_indices.len());
        println!("  TESTING: Limiting to first {} indices", limit);
        (combined_vertices.clone(), combined_indices[..limit].to_vec())
    } else {
        (combined_vertices, combined_indices)
    };
    
    let mesh_data = TexturedMeshData::new(final_vertices, final_indices);
    
    println!("Creating unified renderer with texture array support for {} textures", textures.len());
    
    let renderer = VulkanRenderer::new_texture_array(
        handle_wrapper,
        "shaders/texture_array.vert.spv",
        "shaders/texture_array.frag.spv",
        &mesh_data,
        &textures,
    ).expect("Failed to create unified Vulkan renderer with texture array");
    
    // Calculate model center and extents
    let center = (min_bounds + max_bounds) * 0.5;
    let extents = max_bounds - min_bounds;
    
    println!("Model bounds: center {:?}, extents {:?}", center, extents);
    
    commands.insert_resource(VulkanContext(renderer));
    
    // Spawn camera with controller at the center of the model
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(center),
        CameraController::default().print_controls(),
    ));
    
    println!("Texture array Vulkan renderer created successfully!");
}

fn render_frame(
    mut vulkan: ResMut<VulkanContext>,
    mut fps_logger: Local<FpsLogger>,
    time: Res<Time>,
    camera_query: Query<&Transform, With<Camera>>,
) {
    fps_logger.update(&time);
    
    // Use the camera transform from the camera entity
    if let Ok(camera_transform) = camera_query.single() {
        // Calculate view-projection matrix
        let view_matrix = camera_transform.compute_matrix().inverse();
        let aspect_ratio = 1920.0 / 1080.0; // TODO: Get from window
        let fov = std::f32::consts::PI / 3.0;
        let near = 0.1;
        let far = 1000.0;
        
        // Create projection matrix and flip Y to correct for Vulkan's coordinate system
        let mut proj_matrix = Mat4::perspective_rh(fov, aspect_ratio, near, far);
        proj_matrix.y_axis.y = -proj_matrix.y_axis.y; // Flip Y axis
        let view_proj = proj_matrix * view_matrix;
        
        let _ = vulkan.0.render_frame_with_view_proj(view_proj);
    }
}
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapperHolder};

use vulkan_bevy_renderer::{
    setup_bevy_app,
    vulkan_renderer_unified::VulkanRenderer,
    gltf_loader::GltfData,
    fps_logger::FpsLogger,
    mesh::Vertex,
    ash,
};

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
}

fn setup_vulkan_renderer(
    mut commands: Commands,
    windows: Query<(Entity, &RawHandleWrapperHolder, &Window), With<PrimaryWindow>>,
) {
    let (_entity, handle_wrapper, window) = windows.single().expect("Failed to get primary window");
    
    println!("Window size: {:?}x{:?}", window.width(), window.height());
    
    // Load the GLB file using the consolidated loader
    let gltf_data = GltfData::load_from_file("assets/red_grapes_wjbgdiz_low.glb")
        .expect("Failed to load GLB file");
    
    let mesh_data = gltf_data.mesh_data;
    let texture_data = gltf_data.texture_data;
    
    // Save texture data to temporary file if available
    let texture_path = if let Some(ref texture_data) = texture_data {
        let temp_path = "/tmp/grape_texture.png";
        
        // Save texture data as PNG
        use image::{ImageBuffer, Rgba};
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(
            texture_data.width,
            texture_data.height,
            texture_data.pixels.clone(),
        ).expect("Failed to create image buffer");
        
        img.save(temp_path).expect("Failed to save texture");
        println!("Saved grape texture to {}", temp_path);
        temp_path
    } else {
        "assets/Stone Wall/Stone_Wall_basecolor.jpg" // Fallback
    };
    
    // Use textured renderer with extracted grape texture
    println!("Creating textured renderer using unified renderer with grape texture");
    
    let renderer = VulkanRenderer::new_textured_with_winding(
        handle_wrapper,
        "shaders/mesh_textured.vert.spv",
        "shaders/mesh_textured.frag.spv",
        &mesh_data.vertices,
        &mesh_data.indices,
        vec![Vertex::get_binding_description()],
        Vertex::get_attribute_descriptions(),
        texture_path,
        1, // Single instance
        Some(ash::vk::FrontFace::COUNTER_CLOCKWISE), // Use counter-clockwise winding for grapes
    ).expect("Failed to create textured Vulkan renderer");
    
    commands.insert_resource(VulkanContext { renderer });
    println!("Unified Vulkan renderer created successfully!");
}

fn render_frame(
    mut vulkan: ResMut<VulkanContext>,
    mut fps_logger: Local<FpsLogger>,
    time: Res<Time>,
) {
    fps_logger.update(&time);
    vulkan.renderer.render_frame();
}
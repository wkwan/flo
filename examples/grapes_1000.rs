use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapperHolder};
use rand::Rng;

use vulkan_bevy_renderer::{
    setup_bevy_app,
    vulkan_renderer_unified::VulkanRenderer,
    gltf_loader::GltfData,
    fps_logger::FpsLogger,
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
struct VulkanContext(VulkanRenderer);

fn setup_vulkan_renderer(
    mut commands: Commands,
    windows: Query<(Entity, &RawHandleWrapperHolder, &Window), With<PrimaryWindow>>,
) {
    let (_entity, handle_wrapper, window) = windows.single().expect("Failed to get primary window");
    
    println!("Window size: {:?}x{:?}", window.width(), window.height());
    
    // Generate random positions for 1000 instances in a larger volume
    let mut rng = rand::thread_rng();
    let mut instance_positions = Vec::new();
    for _ in 0..1000 {
        instance_positions.push([
            rng.gen_range(-10.0..10.0),
            rng.gen_range(-7.0..7.0),
            rng.gen_range(-5.0..5.0),
        ]);
    }
    println!("Generated {} random positions", instance_positions.len());
    
    // Load the GLB file using the consolidated loader
    let gltf_data = GltfData::load_from_file("assets/red_grapes_wjbgdiz_low.glb")
        .expect("Failed to load GLB file");
    
    let mesh_data = gltf_data.mesh_data;
    let texture_data = gltf_data.texture_data;
    
    // Use textured renderer if texture data is available
    let renderer = if let Some(texture_data) = texture_data {
        println!("Creating instanced textured renderer with {}x{} texture and {} instances", 
                 texture_data.width, texture_data.height, instance_positions.len());
        
        // Save texture to a temporary file for the unified renderer
        let temp_path = "/tmp/grapes_texture.png";
        let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = image::ImageBuffer::from_raw(
            texture_data.width,
            texture_data.height,
            texture_data.pixels.clone(),
        ).expect("Failed to create image buffer");
        img.save(temp_path).expect("Failed to save texture");
        
        VulkanRenderer::new_textured_instanced_with_winding(
            handle_wrapper,
            "shaders/mesh_textured_instanced.vert.spv",
            "shaders/mesh_textured.frag.spv",
            &mesh_data,
            Some(temp_path),
            &instance_positions,
            Some(ash::vk::FrontFace::CLOCKWISE), // Use clockwise winding for grapes
        ).expect("Failed to create instanced textured Vulkan renderer")
    } else {
        println!("Creating instanced non-textured renderer with {} instances", instance_positions.len());
        
        VulkanRenderer::new_textured_instanced_with_winding(
            handle_wrapper,
            "shaders/mesh_instanced.vert.spv",
            "shaders/mesh.frag.spv",
            &mesh_data,
            None,
            &instance_positions,
            Some(ash::vk::FrontFace::CLOCKWISE), // Use clockwise winding for grapes
        ).expect("Failed to create instanced Vulkan renderer")
    };
    
    commands.insert_resource(VulkanContext(renderer));
    println!("Instanced Vulkan renderer created successfully!");
}

fn render_frame(
    mut vulkan: ResMut<VulkanContext>,
    mut fps_logger: Local<FpsLogger>,
    time: Res<Time>,
) {
    fps_logger.update(&time);
    
    // Render using true GPU instanced rendering
    vulkan.0.render_frame_instanced();
}
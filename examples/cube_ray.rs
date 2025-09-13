use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapperHolder};

use vulkan_bevy_renderer::{setup_bevy_app, vulkan_renderer_unified::VulkanRenderer, fps_logger::FpsLogger};

//still having crazy problems with shapes inside the cube, hoping for some help with this.

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
    println!("Window visible: {:?}", window.visible);
    
    let renderer = VulkanRenderer::new_simple(
        handle_wrapper,
        "shaders/cube_ray.vert.spv",
        "shaders/cube_ray.frag.spv",
        42,  // Updated from 36 to 42 (36 cube + 6 plane)
    ).expect("Failed to create Vulkan renderer");
    
    commands.insert_resource(VulkanContext { renderer });
}

fn render_frame(
    mut vulkan: ResMut<VulkanContext>,
    mut frame_count: Local<u32>,
    mut fps_logger: Local<FpsLogger>,
    time: Res<Time>,
) {
    fps_logger.update(&time);

    *frame_count += 1;
    if *frame_count == 1 {
        println!("Starting render loop...");
    }
    if *frame_count % 60 == 0 {
        println!("Frame count: {}", *frame_count);
    }
    
    vulkan.renderer.render_frame();
}
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapperHolder};

use vulkan_bevy_renderer::{setup_bevy_app, vulkan_renderer_unified::VulkanRenderer, fps_logger::FpsLogger};

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
    
    println!("Window size: {:?}x{:?}", window.width(), window.height());
    println!("Window visible: {:?}", window.visible);
    
    let renderer = VulkanRenderer::new_simple(
        handle_wrapper,
        "shaders/triangle.vert.spv",
        "shaders/triangle.frag.spv",
        3,
    ).expect("Failed to create Vulkan renderer");
    
    commands.insert_resource(VulkanContext { 
        renderer,
        fps_logger: FpsLogger::new(),
    });
}

fn render_frame(
    mut vulkan: ResMut<VulkanContext>,
    time: Res<Time>,
) {
    vulkan.fps_logger.update(&time);
    vulkan.renderer.render_frame();
}
use bevy::prelude::*;
use vulkan_bevy_renderer::fps_logger::FpsLogger;
use bevy::asset::{AssetPlugin, RenderAssetUsages};
use bevy::render::RenderPlugin;
use bevy::render::mesh::{PrimitiveTopology, Indices};
use bevy::pbr::PbrPlugin;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::winit::{WinitPlugin, WakeUp};
use bevy::render::texture::ImagePlugin;
use bevy::window::{WindowPlugin, Window, PresentMode};
use bevy::a11y::AccessibilityPlugin;
use bevy::scene::ScenePlugin;
use bevy::transform::TransformPlugin;
use bevy::input::InputPlugin;

fn main() {
    App::new()
        .add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::Immediate, // Disable vsync
                    ..default()
                }),
                ..default()
            },
            AccessibilityPlugin,
            InputPlugin::default(),
            WinitPlugin::<WakeUp>::default(),
            TransformPlugin,
            RenderPlugin::default(),
            ImagePlugin::default(),
            CorePipelinePlugin::default(),
            ScenePlugin,
            PbrPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, log_fps)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("=== Wireframe Cube Example (Bevy) ===");
    println!("Setting up wireframe cube scene...");
    
    // Create a wireframe cube mesh using lines
    let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::RENDER_WORLD);
    
    let half_size = 1.0;
    
    // Define the 8 vertices of the cube
    let vertices = vec![
        // Bottom face vertices (0-3)
        [-half_size, -half_size, -half_size],
        [ half_size, -half_size, -half_size],
        [ half_size, -half_size,  half_size],
        [-half_size, -half_size,  half_size],
        // Top face vertices (4-7)
        [-half_size,  half_size, -half_size],
        [ half_size,  half_size, -half_size],
        [ half_size,  half_size,  half_size],
        [-half_size,  half_size,  half_size],
    ];
    
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    
    // Define the edges as line indices (pairs of vertices)
    let indices = vec![
        // Bottom face edges
        0, 1,  1, 2,  2, 3,  3, 0,
        // Top face edges
        4, 5,  5, 6,  6, 7,  7, 4,
        // Vertical edges
        0, 4,  1, 5,  2, 6,  3, 7,
    ];
    
    mesh.insert_indices(Indices::U32(indices));
    
    // Spawn the wireframe cube
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 0.5, 1.0), // Blue wireframe
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    
    // Add a static camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 2.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    
    println!("Wireframe cube scene setup complete");
}

fn log_fps(
    mut fps_logger: Local<FpsLogger>,
    time: Res<Time>,
) {
    fps_logger.update(&time);
}
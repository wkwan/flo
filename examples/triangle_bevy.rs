use bevy::prelude::*;
use vulkan_bevy_renderer::fps_logger::FpsLogger;
use bevy::asset::{AssetPlugin, RenderAssetUsages};
use bevy::render::RenderPlugin;
use bevy::render::texture::ImagePlugin;
use bevy::pbr::PbrPlugin;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::winit::{WinitPlugin, WakeUp};
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
    println!("Setting up triangle scene...");
    
    // Create a custom triangle mesh
    let mut mesh = Mesh::new(bevy::render::mesh::PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    
    // Define triangle vertices
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [0.0, 0.5, 0.0],   // Top vertex
            [-0.5, -0.5, 0.0], // Bottom left
            [0.5, -0.5, 0.0],  // Bottom right
        ],
    );
    
    // Add normals
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0, 0.0, 1.0]; 3],
    );
    
    // Add UVs
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.5, 0.0], [0.0, 1.0], [1.0, 1.0]],
    );
    
    // Spawn the triangle (static, no rotation)
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 0.0),
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    
    // Add a 3D camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    
    println!("Triangle scene setup complete");
}

fn log_fps(
    mut fps_logger: Local<FpsLogger>,
    time: Res<Time>,
) {
    fps_logger.update(&time);
}
use bevy::prelude::*;
use vulkan_bevy_renderer::fps_logger::FpsLogger;
use bevy::asset::AssetPlugin;
use bevy::render::RenderPlugin;
use bevy::pbr::PbrPlugin;
use bevy::gltf::GltfPlugin;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::winit::{WinitPlugin, WakeUp};
use bevy::render::texture::ImagePlugin;
use bevy::window::{WindowPlugin, Window, PresentMode};
use bevy::a11y::AccessibilityPlugin;
use bevy::scene::ScenePlugin;
use bevy::transform::TransformPlugin;
use bevy::input::InputPlugin;
use bevy::input::keyboard::KeyboardFocusLost;

use vulkan_bevy_renderer::camera_controller::{CameraController, CameraControllerPlugin};

fn main() {
    App::new()
        .add_event::<KeyboardFocusLost>()
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
            GltfPlugin::default(),
            CameraControllerPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, log_fps)
        .run();
}

#[derive(Component)]
struct Aula;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    println!("Setting up Aula scene...");
    
    // Spawn the GLB file as a scene
    commands.spawn((
        SceneRoot(asset_server.load("Aula.glb#Scene0")),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Aula,
    ));
    println!("Spawned Aula scene");
    
    // Add a camera with controller inside the classroom
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-1.2, 3.0, 0.5).looking_at(Vec3::new(-1.2, 2.0, -5.0), Vec3::Y),
        CameraController::default().print_controls(),
    ));
    println!("Camera positioned with controller");
    
    // Add strong directional lighting
    commands.spawn((
        DirectionalLight {
            illuminance: 30000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -45.0_f32.to_radians(), 45.0_f32.to_radians(), 0.0)),
    ));
    
    // Add ambient light for better visibility
    commands.insert_resource(AmbientLight {
        color: Color::srgb(1.0, 1.0, 1.0),
        brightness: 200.0,
        affects_lightmapped_meshes: false,
    });
    
    // Add additional point lights for better illumination
    commands.spawn((
        PointLight {
            intensity: 2000000.0,
            range: 100.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 0.0),
    ));
    
    commands.spawn((
        PointLight {
            intensity: 1500000.0,
            range: 100.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-5.0, 5.0, 5.0),
    ));
}

fn log_fps(
    mut fps_logger: Local<FpsLogger>,
    time: Res<Time>,
) {
    fps_logger.update(&time);
}
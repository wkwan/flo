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
use rand::Rng;

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
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate_grapes, log_fps))
        .run();
}

#[derive(Component)]
struct Grapes;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    println!("Setting up scene with 1000 grapes...");
    
    let mut rng = rand::thread_rng();
    
    // Spawn 1000 grape models at random positions
    for _ in 0..1000 {
        // Keep them close together for a dense cloud
        let x = rng.gen_range(-1.0..1.0);
        let y = rng.gen_range(-0.5..0.5);
        let z = rng.gen_range(-0.5..0.5);
        
        commands.spawn((
            SceneRoot(asset_server.load("red_grapes_wjbgdiz_low.glb#Scene0")),
            Transform::from_xyz(x, y, z)
                .with_scale(Vec3::splat(1.0)), // Much larger scale
            Grapes,
        ));
    }
    println!("Spawned 1000 grape scenes");
    
    // Add a camera positioned to see all the grapes
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    println!("Camera positioned at (0, 0, 3)");
    
    // Add strong lighting
    commands.spawn((
        DirectionalLight {
            illuminance: 50000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -45.0_f32.to_radians(), 45.0_f32.to_radians(), 0.0)),
    ));
    
    // Add ambient light for better visibility
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 800.0,
        affects_lightmapped_meshes: false,
    });
}

fn rotate_grapes(
    mut query: Query<&mut Transform, With<Grapes>>,
    time: Res<Time>,
) {
    for mut transform in query.iter_mut() {
        // Rotate at the same speed as the Vulkan renderer (0.5 radians per second)
        transform.rotation = Quat::from_rotation_y(time.elapsed_secs() * 0.5);
    }
}

fn log_fps(
    mut fps_logger: Local<FpsLogger>,
    time: Res<Time>,
) {
    fps_logger.update(&time);
}
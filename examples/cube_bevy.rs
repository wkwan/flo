use bevy::prelude::*;
use vulkan_bevy_renderer::fps_logger::FpsLogger;
use bevy::asset::AssetPlugin;
use bevy::render::RenderPlugin;
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
        .add_systems(Update, (rotate_cube, log_fps))
        .run();
}

#[derive(Component)]
struct RotatingCube;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("Setting up cube scene...");
    
    // Spawn the cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.5, 1.0),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        RotatingCube,
    ));
    
    // Add a camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    
    // Add lighting
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -45.0_f32.to_radians(), 45.0_f32.to_radians(), 0.0)),
    ));
    
    // Add ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
        affects_lightmapped_meshes: false,
    });
    
    println!("Cube scene setup complete");
}

fn rotate_cube(
    mut query: Query<&mut Transform, With<RotatingCube>>,
    time: Res<Time>,
) {
    for mut transform in query.iter_mut() {
        // Rotate at the same speed as the Vulkan renderer
        transform.rotation = Quat::from_rotation_y(time.elapsed_secs()) * Quat::from_rotation_x(time.elapsed_secs() * 0.5);
    }
}

fn log_fps(
    mut fps_logger: Local<FpsLogger>,
    time: Res<Time>,
) {
    fps_logger.update(&time);
}
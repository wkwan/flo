// Set this to true to render multiple instanced mannequins, false for single mannequin
const SHOW_MULTIPLE: bool = true;

use bevy::prelude::*;
use bevy::window::{WindowPlugin, Window, PresentMode};
use bevy::gltf::Gltf;
use bevy::animation::{AnimationPlayer, graph::{AnimationGraph, AnimationGraphHandle, AnimationNodeIndex}};
use bevy::pbr::DirectionalLightShadowMap;
use bevy::scene::SceneInstanceReady;

use vulkan_bevy_renderer::fps_logger::FpsLogger;

// Resource to track if we've spawned the scene
#[derive(Resource, Default)]
struct SceneState {
    handle: Handle<Gltf>,
    loaded: bool,
    scene_spawned: bool,
}

// Resource to store animations
#[derive(Resource)]
struct Animations {
    graph: Handle<AnimationGraph>,
    node_index: AnimationNodeIndex,
}

// Component to mark mannequin entities
#[derive(Component)]
struct Mannequin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Mannequin Animation Example (Bevy Renderer)".into(),
                resolution: (800., 600.).into(),
                present_mode: PresentMode::Immediate, // Disable vsync
                ..default()
            }),
            ..default()
        }))
        .insert_resource(DirectionalLightShadowMap { size: 2048 })
        .insert_resource(SceneState::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (
            check_gltf_loaded,
            log_fps,
        ))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut scene_state: ResMut<SceneState>,
) {
    // Load the GLTF file
    scene_state.handle = asset_server.load("mannequin.glb");
    
    // Setup camera - positioned to match the Vulkan example
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(1.0, 0.5, 5.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y), // Use Y up instead of NEG_Y
        Projection::Perspective(PerspectiveProjection {
            fov: 60.0_f32.to_radians(),
            near: 0.1,
            far: 1000.0,
            ..default()
        }),
    ));
    
    // Add lighting to match the Vulkan renderer's appearance
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::PI / 4.0)),
    ));
    
    // Add some ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 500.0,
        affects_lightmapped_meshes: false,
    });
    
    println!("Setup complete - loading mannequin.glb");
}

fn check_gltf_loaded(
    mut scene_state: ResMut<SceneState>,
    gltf_assets: Res<Assets<Gltf>>,
    mut commands: Commands,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
) {
    if scene_state.scene_spawned {
        return;
    }
    
    // Check if the GLTF is loaded
    if let Some(gltf) = gltf_assets.get(&scene_state.handle) {
        if !scene_state.loaded {
            scene_state.loaded = true;
            
            // Log available animations
            if !gltf.animations.is_empty() {
                println!("Found {} animations", gltf.animations.len());
                for (i, _) in gltf.animations.iter().enumerate() {
                    println!("  Animation {}", i);
                }
                
                // Create animation graph from the first animation
                let mut graph = AnimationGraph::new();
                let node_index = graph.add_clip(
                    gltf.animations[0].clone(),
                    1.0,
                    graph.root,
                );
                let graph_handle = animation_graphs.add(graph);
                
                // Store the animations resource
                commands.insert_resource(Animations {
                    graph: graph_handle,
                    node_index,
                });
            }
        }
        
        if !scene_state.scene_spawned {
            // Spawn the mannequin scene
            if !gltf.scenes.is_empty() {
                if SHOW_MULTIPLE {
                    // Spawn multiple instances
                    let positions = [
                        Vec3::new(-2.0, 0.0, 0.0),
                        Vec3::new(-1.0, 0.0, 0.0),
                        Vec3::new(0.0, 0.0, 0.0),
                        Vec3::new(1.0, 0.0, 0.0),
                        Vec3::new(2.0, 0.0, 0.0),
                    ];
                    
                    for (i, pos) in positions.iter().enumerate() {
                        commands.spawn((
                            SceneRoot(gltf.scenes[0].clone()),
                            Transform::from_translation(*pos),
                            Mannequin,
                            Name::new(format!("Mannequin_{}", i)),
                        ))
                        .observe(setup_animation);
                    }
                    println!("Spawned {} mannequin instances", positions.len());
                } else {
                    // Spawn single mannequin at origin
                    commands.spawn((
                        SceneRoot(gltf.scenes[0].clone()),
                        Transform::default(),
                        Mannequin,
                        Name::new("Mannequin"),
                    ))
                    .observe(setup_animation);
                    println!("Spawned single mannequin scene");
                }
                
                scene_state.scene_spawned = true;
            }
        }
    }
}

fn setup_animation(
    trigger: Trigger<SceneInstanceReady>,
    animations: Res<Animations>,
    mut commands: Commands,
    children_query: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
) {
    let entity = trigger.target();
    println!("Scene instance ready for entity {:?}", entity);
    
    // Iterate through all descendants of the spawned scene
    for child in children_query.iter_descendants(entity) {
        if let Ok(mut player) = players.get_mut(child) {
            // Add the animation graph handle and play the animation
            commands
                .entity(child)
                .insert(AnimationGraphHandle(animations.graph.clone()));
            
            player.play(animations.node_index).repeat();
            println!("Started animation playback for entity {:?}", child);
        }
    }
}

fn log_fps(
    time: Res<Time>,
    mut fps_logger: Local<FpsLogger>,
) {
    fps_logger.update(&time);
}
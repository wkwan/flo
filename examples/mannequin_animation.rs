// Set this to true to render multiple instanced mannequins, false for single mannequin
const SHOW_MULTIPLE: bool = true;

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapperHolder, WindowPlugin, Window};
use bevy::asset::{AssetPlugin, LoadState};
use bevy::gltf::{Gltf, GltfPlugin};
use bevy::render::RenderPlugin;
use bevy::render::texture::ImagePlugin;
use bevy::render::settings::WgpuSettings;
use bevy::pbr::PbrPlugin;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::scene::ScenePlugin;
use bevy::transform::TransformPlugin;
use bevy::animation::{AnimationPlugin, AnimationPlayer};
use bevy::winit::{WinitPlugin, WakeUp};
use bevy::a11y::AccessibilityPlugin;
use bevy::input::InputPlugin;
use bevy::input::keyboard::KeyboardFocusLost;

use vulkan_bevy_renderer::{
    vulkan_renderer_unified::VulkanRenderer,
    skinned_mesh::SkinnedMeshData,
    utils,
    fps_logger::FpsLogger
};

use std::sync::{Arc, Mutex};

// Resource to hold the Vulkan renderer and mesh index
#[derive(Resource)]
struct VulkanRendererResource {
    renderer: Arc<Mutex<Option<VulkanRenderer>>>,
    mannequin_mesh_index: Option<usize>,
    fps_logger: FpsLogger,
}

fn main() {
    App::new()
        .add_event::<KeyboardFocusLost>()
        .add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            WindowPlugin {
                primary_window: Some(Window {
                    title: "Mannequin Animation Example".into(),
                    resolution: (800., 600.).into(),
                    ..default()
                }),
                ..default()
            },
            AccessibilityPlugin,
            InputPlugin::default(),
            WinitPlugin::<WakeUp>::default(),
            TransformPlugin,
            RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            },
            ImagePlugin::default(),
            CorePipelinePlugin::default(),
            PbrPlugin::default(),
            ScenePlugin,
            GltfPlugin::default(),
            AnimationPlugin,
        ))
        .insert_resource(utils::MeshGltf {
            handle: Handle::default(),
            loaded: false,
            scene_spawned: false,
        })
        .insert_resource(utils::ExtractedMeshData::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (
            check_gltf_loaded,
            utils::extract_mesh_data,
            render_with_vulkan,
            utils::animate_joints,
        ))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut mannequin_gltf: ResMut<utils::MeshGltf>,
) {
    // Load the GLTF file
    mannequin_gltf.handle = asset_server.load("mannequin.glb");
    
    // Initialize empty Vulkan renderer resource (will be created when mesh is loaded)
    commands.insert_resource(VulkanRendererResource {
        renderer: Arc::new(Mutex::new(None)),
        mannequin_mesh_index: None,
        fps_logger: FpsLogger::new(),
    });
    
    println!("Setup complete - loading mannequin.glb");
}

fn check_gltf_loaded(
    mut mannequin_gltf: ResMut<utils::MeshGltf>,
    asset_server: Res<AssetServer>,
    gltf_assets: Res<Assets<Gltf>>,
    mut commands: Commands,
) {
    if mannequin_gltf.scene_spawned {
        return;
    }
    
    // Check if the GLTF is loaded
    if let Some(LoadState::Loaded) = asset_server.get_load_state(&mannequin_gltf.handle) {
        if let Some(gltf) = gltf_assets.get(&mannequin_gltf.handle) {
            mannequin_gltf.loaded = true;
            
            if !mannequin_gltf.scene_spawned {
                // Spawn the first scene which contains the skinned mesh
                if !gltf.scenes.is_empty() {
                    commands.spawn((
                        SceneRoot(gltf.scenes[0].clone()),
                        AnimationPlayer::default(),
                    ));
                    mannequin_gltf.scene_spawned = true;
                    println!("Spawned mannequin scene with AnimationPlayer");
                    
                    // Log available animations
                    if !gltf.animations.is_empty() {
                        println!("Found {} animations", gltf.animations.len());
                        for (i, _anim) in gltf.animations.iter().enumerate() {
                            println!("  Animation {}", i);
                        }
                    }
                }
            }
        }
    }
}

fn render_with_vulkan(
    mut vulkan_renderer: Option<ResMut<VulkanRendererResource>>,
    extracted_data: Res<utils::ExtractedMeshData>,
    window_query: Query<&RawHandleWrapperHolder, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    // Skip if no renderer resource
    let Some(ref mut vulkan_renderer) = vulkan_renderer else {
        return;
    };
    
    // Skip if no mesh data extracted yet
    let Some(ref mesh_data) = extracted_data.mesh_data else {
        return;
    };
    
    // Initialize renderer if not yet created
    let needs_init = vulkan_renderer.renderer.lock()
        .map(|guard| guard.is_none())
        .unwrap_or(false);
    
    if needs_init {
        if let Ok(window_handle) = window_query.single() {
            match create_vulkan_renderer_for_mesh(window_handle, mesh_data) {
                Ok((renderer, mesh_index)) => {
                    vulkan_renderer.mannequin_mesh_index = Some(mesh_index);
                    if let Ok(mut renderer_guard) = vulkan_renderer.renderer.lock() {
                        *renderer_guard = Some(renderer);
                        println!("Vulkan renderer created with mannequin mesh at index {}", mesh_index);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create Vulkan renderer: {}", e);
                    return;
                }
            }
        }
    }
    
    // Update FPS logger
    vulkan_renderer.fps_logger.update(&time);
    
    // Render frame
    if let Ok(mut renderer_guard) = vulkan_renderer.renderer.lock() {
        if let Some(ref mut renderer) = *renderer_guard {
            // Update joint matrices for the mannequin mesh
            if let (Some(mesh_index), Some(ref mesh_data)) = (vulkan_renderer.mannequin_mesh_index, &extracted_data.mesh_data) {
                // Log matrix values being sent to GPU
                static mut LAST_GPU_LOG: f32 = 0.0;
                let current_time = time.elapsed_secs();
                
                unsafe {
                    if current_time - LAST_GPU_LOG >= 1.0 {
                        println!("=== GPU Matrix Update ===");
                        for i in 0..10 {  // Log first 10 matrices to include joint 5
                            let mat = &mesh_data.joint_matrices[i];
                            println!("GPU Joint {} matrix: {:.6}, {:.6}, {:.6}, {:.6}", 
                                i, mat.x_axis.x, mat.y_axis.y, mat.z_axis.z, mat.w_axis.w);
                        }
                        println!("=========================");
                        LAST_GPU_LOG = current_time;
                    }
                }
                
                // Update joint matrices for the specific mesh
                renderer.update_mesh_joint_matrices(mesh_index, &mesh_data.joint_matrices);
            }
            render_frame(renderer, mesh_data, time.elapsed_secs());
        }
    }; // Add semicolon to drop the temporary
}

fn create_vulkan_renderer_for_mesh(
    window_handle: &RawHandleWrapperHolder,
    mesh_data: &SkinnedMeshData,
) -> Result<(VulkanRenderer, usize), Box<dyn std::error::Error>> {
    // First create a basic renderer with new_from_mesh_data
    // Convert SkinnedMeshData to regular MeshData for initial setup
    use vulkan_bevy_renderer::mesh::{MeshData, Vertex};
    
    // Create a simple sphere mesh for the base renderer
    let vertices = vec![
        Vertex {
            position: [0.0, 0.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        },
    ];
    
    let basic_mesh_data = MeshData {
        vertices,
        indices: vec![0],
    };
    
    // Create base renderer with a simple shader
    let mut renderer = VulkanRenderer::new_from_mesh_data(
        window_handle,
        "shaders/mesh.vert.spv",
        "shaders/mesh.frag.spv",
        &basic_mesh_data,
        1,
    )?;
    
    // Add the appropriate skinned pipeline with correct descriptor set layout
    if SHOW_MULTIPLE {
        // Add instanced skinning pipeline
        renderer.add_skinned_pipeline(
            "skinned_instanced",
            "shaders/skinned_instanced.vert.spv",
            "shaders/mesh.frag.spv",
            true, // use_instancing
        )?;
    } else {
        // Add single skinned mesh pipeline
        renderer.add_skinned_pipeline(
            "skinned",
            "shaders/skinned_full.vert.spv",
            "shaders/mesh.frag.spv",
            false, // use_instancing
        )?;
    }
    
    // Now add the mannequin mesh using the multi-mesh system
    let mesh_index = if SHOW_MULTIPLE {
        let instance_positions = vec![
            [-2.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
        ];
        
        renderer.add_skinned_mesh_instanced(
            mesh_data,
            &instance_positions,
            Some("skinned_instanced".to_string()),
        )?
    } else {
        // For single mannequin, add with one instance
        renderer.add_skinned_mesh_instanced(
            mesh_data,
            &[[0.0, 0.0, 0.0]],
            Some("skinned".to_string()),
        )?
    };
    
    Ok((renderer, mesh_index))
}

fn render_frame(renderer: &mut VulkanRenderer, _mesh_data: &SkinnedMeshData, _time: f32) {
    // Camera positioned to view the model at original scale
    let view = bevy::math::Mat4::look_at_rh(
        bevy::math::Vec3::new(1.0, 0.5, 5.0),  // Much closer eye position
        bevy::math::Vec3::new(0.0, 1.0, 0.0),  // Look at center of model 
        bevy::math::Vec3::NEG_Y,               // Flip up vector to correct orientation
    );
    let proj = bevy::math::Mat4::perspective_rh(
        60.0_f32.to_radians(),  // FOV
        800.0 / 600.0,
        0.1,    // Normal near plane
        1000.0, // Normal far plane
    );
    
    renderer.render_frame_with_camera_multi(view, proj);
}
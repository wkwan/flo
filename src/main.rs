use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_graph::{self, RenderGraph, RenderLabel};
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderContext, RenderDevice};
use bevy::render::{Render, RenderApp, RenderSet};
use bevy::asset::{Asset, AssetApp};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WaveSimulationPlugin)
        .add_systems(Startup, (setup, setup_wave_textures).chain())
        .add_systems(Update, (handle_mouse_input, clear_wave_input, log_wave_simulation_status).chain())
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Camera - positioned like in Unity screenshot
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 8.0, 8.0).looking_at(Vec3::new(0.0, 0.0, -2.0), Vec3::Y),
        GlobalTransform::default(),
        Visibility::default(),
    ));

    // Directional light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 10000.0,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 8.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            ..default()
        },
        GlobalTransform::default(),
        Visibility::default(),
    ));

    // Load stone wall textures now that JPEG support is enabled
    let base_color_texture = asset_server.load("Stone Wall/Stone_Wall_basecolor.jpg");
    let normal_texture = asset_server.load("Stone Wall/Stone_Wall_normal.jpg");
    let roughness_texture = asset_server.load("Stone Wall/Stone_Wall_roughness.jpg");
    let ao_texture = asset_server.load("Stone Wall/Stone_Wall_ambientOcclusion.jpg");

    // Stone wall material with textures and fallback color
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.6, 0.4), // Sandy fallback color
        base_color_texture: Some(base_color_texture),
        normal_map_texture: Some(normal_texture),
        metallic_roughness_texture: Some(roughness_texture),
        occlusion_texture: Some(ao_texture),
        ..default()
    });

    // Water plane - subdivided quad for wave displacement (centered at origin)
    let water_mesh = create_subdivided_plane(64, 64, 8.0);
    commands.spawn((
        Name::new("WaterPlane"),
        Mesh3d(meshes.add(water_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.1, 0.3, 0.8, 0.8),
            metallic: 0.0,
            perceptual_roughness: 0.1,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
    ));

    // Create stone walls around the water plane to match Unity layout
    let wall_height = 6.0;
    let wall_thickness = 1.0;
    let water_size = 8.0;
    let half_water = water_size * 0.5;

    // Left wall (X = -half_water)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(wall_thickness, wall_height, water_size))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-half_water - wall_thickness * 0.5, wall_height * 0.5, 0.0),
        GlobalTransform::default(),
        Visibility::default(),
    ));

    // Right wall (X = +half_water)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(wall_thickness, wall_height, water_size))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(half_water + wall_thickness * 0.5, wall_height * 0.5, 0.0),
        GlobalTransform::default(),
        Visibility::default(),
    ));

    // Back wall (Z = -half_water)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(water_size + wall_thickness * 2.0, wall_height, wall_thickness))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(0.0, wall_height * 0.5, -half_water - wall_thickness * 0.5),
        GlobalTransform::default(),
        Visibility::default(),
    ));

    // Bottom wall - positioned lower to create a gap with water plane
    let bottom_wall_y = -2.0; // Much lower than water plane (at Y=0)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(water_size + wall_thickness * 2.0, wall_thickness, water_size + wall_thickness * 2.0))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(0.0, bottom_wall_y, 0.0),
        GlobalTransform::default(),
        Visibility::default(),
    ));
}

fn create_subdivided_plane(width_subdivisions: u32, height_subdivisions: u32, size: f32) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let half_size = size * 0.5;
    let width_step = size / width_subdivisions as f32;
    let height_step = size / height_subdivisions as f32;

    // Generate vertices
    for z in 0..=height_subdivisions {
        for x in 0..=width_subdivisions {
            let pos_x = -half_size + x as f32 * width_step;
            let pos_z = -half_size + z as f32 * height_step;
            
            positions.push([pos_x, 0.0, pos_z]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / width_subdivisions as f32, z as f32 / height_subdivisions as f32]);
        }
    }

    // Generate indices
    for z in 0..height_subdivisions {
        for x in 0..width_subdivisions {
            let i = z * (width_subdivisions + 1) + x;
            let next_row = i + width_subdivisions + 1;

            // First triangle
            indices.push(i);
            indices.push(next_row);
            indices.push(i + 1);

            // Second triangle  
            indices.push(i + 1);
            indices.push(next_row);
            indices.push(next_row + 1);
        }
    }

    Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD | bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(bevy::render::mesh::Indices::U32(indices))
}

fn create_wall_mesh(width: f32, height: f32, thickness: f32) -> Mesh {
    let half_width = width * 0.5;
    let half_height = height * 0.5;
    let half_thickness = thickness * 0.5;

    let positions = vec![
        // Front face
        [-half_width, -half_height, half_thickness],
        [half_width, -half_height, half_thickness],
        [half_width, half_height, half_thickness],
        [-half_width, half_height, half_thickness],
        // Back face
        [half_width, -half_height, -half_thickness],
        [-half_width, -half_height, -half_thickness],
        [-half_width, half_height, -half_thickness],
        [half_width, half_height, -half_thickness],
    ];

    let normals = vec![
        // Front face
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        // Back face
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
    ];

    let uvs = vec![
        // Front face
        [0.0, 0.0],
        [2.0, 0.0], // Repeat texture
        [2.0, 1.0],
        [0.0, 1.0],
        // Back face
        [0.0, 0.0],
        [2.0, 0.0],
        [2.0, 1.0],
        [0.0, 1.0],
    ];

    let indices = vec![
        // Front face
        0, 1, 2, 2, 3, 0,
        // Back face
        4, 5, 6, 6, 7, 4,
        // Left face
        5, 0, 3, 3, 6, 5,
        // Right face
        1, 4, 7, 7, 2, 1,
        // Top face
        3, 2, 7, 7, 6, 3,
        // Bottom face
        5, 4, 1, 1, 0, 5,
    ];

    Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD | bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(bevy::render::mesh::Indices::U32(indices))
}

#[derive(Resource, Clone, ExtractResource, ShaderType)]
struct WaveSimulationParams {
    dampening: f32,
    input_x: f32,
    input_y: f32,
    input_size: f32,
    min_input_size: f32,
    got_input: f32,
    input_push: f32,
    resolution: Vec2,
}

impl Default for WaveSimulationParams {
    fn default() -> Self {
        Self {
            dampening: 0.99,
            input_x: 0.0,
            input_y: 0.0,
            input_size: 20.0,
            min_input_size: 5.0,
            got_input: 0.0,
            input_push: 0.0,
            resolution: Vec2::new(512.0, 512.0),
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct WaveComputeShader {
    #[storage_texture(0, image_format = Rg32Float, access = ReadWrite)]
    texture_a: Handle<Image>,
    #[storage_texture(1, image_format = Rg32Float, access = ReadWrite)]
    texture_b: Handle<Image>,
    #[uniform(2)]
    params: WaveSimulationParams,
}

#[derive(Resource, Clone, ExtractResource)]
struct WaveTextures {
    texture_a: Handle<Image>,
    texture_b: Handle<Image>,
    current_texture: bool, // false = texture_a, true = texture_b
}

struct WaveSimulationPlugin;

impl Plugin for WaveSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<WaveComputeShader>()
            .init_resource::<WaveSimulationParams>()
            .add_plugins(ExtractResourcePlugin::<WaveSimulationParams>::default())
            .add_plugins(ExtractResourcePlugin::<WaveTextures>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(Render, queue_wave_simulation.in_set(RenderSet::Queue));
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(WaveSimulationLabel, WaveSimulationNode::default());
        render_graph.add_node_edge(WaveSimulationLabel, bevy::render::graph::CameraDriverLabel);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct WaveSimulationLabel;

#[derive(Default)]
struct WaveSimulationNode {
    initialized: std::sync::atomic::AtomicBool,
}

impl render_graph::Node for WaveSimulationNode {
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        _render_context: &mut RenderContext,
        world: &bevy::ecs::world::World,
    ) -> Result<(), render_graph::NodeRunError> {
        // Log when first initialized
        use std::sync::atomic::Ordering;
        if !self.initialized.load(Ordering::Relaxed) {
            info!("Wave simulation render node initialized");
            self.initialized.store(true, Ordering::Relaxed);
        }
        
        // Check if resources exist and run compute shader
        let Some(_wave_textures) = world.get_resource::<WaveTextures>() else {
            return Ok(());
        };
        
        let Some(params) = world.get_resource::<WaveSimulationParams>() else {
            return Ok(());
        };
        
        let Some(_pipeline_cache) = world.get_resource::<PipelineCache>() else {
            return Ok(());
        };
        
        let Some(_render_device) = world.get_resource::<RenderDevice>() else {
            return Ok(());
        };
        
        // TODO: Create compute pipeline and dispatch
        // For now, just log when we have input
        if params.got_input > 0.5 {
            trace!("Wave simulation would dispatch at UV ({:.3}, {:.3})", 
                params.input_x, params.input_y);
        }
        
        Ok(())
    }
}

fn setup_wave_textures(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    info!("Setting up wave simulation textures");
    // Create 512x512 RG32Float textures for double buffering
    let size = Extent3d {
        width: 512,
        height: 512,
        depth_or_array_layers: 1,
    };

    let mut texture_a = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("wave_texture_a"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rg32Float,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        data: Some(vec![0u8; (512 * 512 * 8) as usize]), // RG32Float = 8 bytes per pixel
        ..default()
    };

    let mut texture_b = texture_a.clone();
    texture_b.texture_descriptor.label = Some("wave_texture_b");
    
    // Initialize with neutral values (0.5, 0.5) representing no displacement
    let neutral_data: Vec<u8> = (0..512 * 512)
        .flat_map(|_| {
            let val = 0.5f32;
            [val.to_le_bytes(), val.to_le_bytes()].concat()
        })
        .collect();
    
    texture_a.data = Some(neutral_data.clone());
    texture_b.data = Some(neutral_data);

    let texture_a_handle = images.add(texture_a);
    let texture_b_handle = images.add(texture_b);

    commands.insert_resource(WaveTextures {
        texture_a: texture_a_handle.clone(),
        texture_b: texture_b_handle.clone(),
        current_texture: false,
    });
    
    info!("Wave textures initialized - A: {:?}, B: {:?}", texture_a_handle, texture_b_handle);
}

fn log_wave_simulation_status(
    time: Res<Time>,
    wave_textures: Option<Res<WaveTextures>>,
    params: Option<Res<WaveSimulationParams>>,
) {
    // Log every 2 seconds
    let elapsed = time.elapsed_secs();
    if (elapsed as u32) % 2 == 0 && (elapsed * 10.0) as u32 % 10 == 0 {
        if let (Some(textures), Some(params)) = (wave_textures, params) {
            info!("Wave simulation status - Time: {:.1}s, Dampening: {}, Current buffer: {}",
                elapsed, params.dampening, if textures.current_texture { "B" } else { "A" });
        } else {
            warn!("Wave simulation resources not found at {:.1}s", elapsed);
        }
    }
}

fn queue_wave_simulation(
    wave_textures: Res<WaveTextures>,
    _params: Res<WaveSimulationParams>,
) {
    // Log that we're queuing wave simulation
    trace!("Queuing wave simulation - current buffer: {}", 
        if wave_textures.current_texture { "B" } else { "A" });
}

fn handle_mouse_input(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    water_planes: Query<&Transform, With<Name>>,
    mut wave_params: ResMut<WaveSimulationParams>,
) {
    // Check for left mouse click
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    // Get the primary window
    let Ok(window) = windows.single() else {
        return;
    };

    // Get cursor position
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Get camera
    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };

    // Calculate ray from camera through cursor
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    // Find water plane (at Y=0)
    let water_y = 0.0;
    
    // Ray-plane intersection
    let t = (water_y - ray.origin.y) / ray.direction.y;
    if t < 0.0 {
        return; // Ray points away from plane
    }

    // Calculate intersection point
    let world_pos = ray.origin + ray.direction * t;
    
    // Convert world position to UV coordinates (water plane is 8x8 centered at origin)
    let water_size = 8.0;
    let half_size = water_size * 0.5;
    
    // Map from world space (-4, -4) to (4, 4) to UV space (0, 0) to (1, 1)
    let uv_x = (world_pos.x + half_size) / water_size;
    let uv_y = (world_pos.z + half_size) / water_size;
    
    // Clamp to valid UV range
    let uv_x = uv_x.clamp(0.0, 1.0);
    let uv_y = uv_y.clamp(0.0, 1.0);
    
    // Update wave parameters
    wave_params.input_x = uv_x;
    wave_params.input_y = uv_y;
    wave_params.got_input = 1.0;
    wave_params.input_push = 0.0; // 0 = push down (create wave), 1 = push up
    
    info!("Mouse click at world ({:.2}, {:.2}, {:.2}) -> UV ({:.3}, {:.3})", 
        world_pos.x, world_pos.y, world_pos.z, uv_x, uv_y);
}

fn clear_wave_input(
    mut wave_params: ResMut<WaveSimulationParams>,
) {
    // Clear input after one frame
    if wave_params.got_input > 0.5 {
        wave_params.got_input = 0.0;
    }
}

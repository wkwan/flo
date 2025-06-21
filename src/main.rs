use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_graph::{self, RenderGraph, RenderLabel};
use bevy::render::render_resource::*;
use bevy::render::renderer::RenderContext;
use bevy::render::{Render, RenderApp, RenderSet};
use bevy::asset::{Asset, AssetApp};
use bevy::pbr::{MaterialPlugin, Material};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MaterialPlugin::<WaterMaterial>::default())
        .add_systems(Startup, (setup, setup_wave_textures, setup_water_material).chain())
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

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct WaveComputeShader {
    #[storage_texture(0, image_format = Rg32Float, access = ReadWrite)]
    texture_a: Handle<Image>,
    #[storage_texture(1, image_format = Rg32Float, access = ReadWrite)]
    texture_b: Handle<Image>
}

#[derive(Resource, Clone, ExtractResource)]
struct WaveTextures {
    texture_a: Handle<Image>,
    texture_b: Handle<Image>,
    current_texture: bool, // false = texture_a, true = texture_b
}


#[derive(ShaderType, Clone)]
struct WaterMaterialUniform {
    wave_amplitude: f32,
    color: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct WaterMaterial {
    #[uniform(0)]
    uniform: WaterMaterialUniform,
    #[texture(1)]
    #[sampler(2)]
    wave_texture: Handle<Image>,
}

impl Material for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/water_material.wgsl".into()
    }
    
    fn vertex_shader() -> ShaderRef {
        "shaders/water_material.wgsl".into()
    }
    
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}

impl Default for WaterMaterial {
    fn default() -> Self {
        Self {
            uniform: WaterMaterialUniform {
                wave_amplitude: 1.0,
                color: Vec4::new(0.1, 0.3, 0.8, 1.0),
            },
            wave_texture: Handle::default(),
        }
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

fn setup_water_material(
    mut commands: Commands,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    wave_textures: Res<WaveTextures>,
    mut water_plane_query: Query<Entity, (With<Name>, Without<MeshMaterial3d<WaterMaterial>>)>,
) {
    // Create water material with wave texture
    let water_material = water_materials.add(WaterMaterial {
        uniform: WaterMaterialUniform {
            wave_amplitude: 2.0, // Make waves more visible
            color: Vec4::new(0.1, 0.3, 0.8, 1.0), // Blue water color
        },
        wave_texture: wave_textures.texture_a.clone(),
    });
    
    // Add material to water plane
    for entity in water_plane_query.iter_mut() {
        commands.entity(entity).insert(MeshMaterial3d(water_material.clone()));
        info!("Water material applied to water plane");
        break; // Only apply to first water plane
    }
}

fn handle_mouse_input(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    _water_planes: Query<&Transform, With<Name>>
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
        
    info!("Mouse click at world ({:.2}, {:.2}, {:.2}) -> UV ({:.3}, {:.3})", 
        world_pos.x, world_pos.y, world_pos.z, uv_x, uv_y);
}

fn update_water_material(
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    wave_textures: Option<Res<WaveTextures>>,
    water_planes: Query<&MeshMaterial3d<WaterMaterial>, With<Name>>,
) {
    // Update water material with current wave texture
    if let Some(wave_textures) = wave_textures {
        for material_handle in water_planes.iter() {
            if let Some(material) = water_materials.get_mut(&material_handle.0) {
                // Use the current active texture based on double buffering
                let current_texture = if wave_textures.current_texture {
                    &wave_textures.texture_b
                } else {
                    &wave_textures.texture_a
                };
                material.wave_texture = current_texture.clone();
            }
        }
    }
}

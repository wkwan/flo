use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
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

fn create_floor_mesh(size: f32, thickness: f32) -> Mesh {
    let half_size = size * 0.5;
    let half_thickness = thickness * 0.5;

    let positions = vec![
        // Top face
        [-half_size, half_thickness, -half_size],
        [half_size, half_thickness, -half_size],
        [half_size, half_thickness, half_size],
        [-half_size, half_thickness, half_size],
        // Bottom face
        [-half_size, -half_thickness, half_size],
        [half_size, -half_thickness, half_size],
        [half_size, -half_thickness, -half_size],
        [-half_size, -half_thickness, -half_size],
    ];

    let normals = vec![
        // Top face
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        // Bottom face
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
    ];

    let uvs = vec![
        // Top face - repeat texture
        [0.0, 0.0],
        [2.0, 0.0],
        [2.0, 2.0],
        [0.0, 2.0],
        // Bottom face
        [0.0, 0.0],
        [2.0, 0.0],
        [2.0, 2.0],
        [0.0, 2.0],
    ];

    let indices = vec![
        // Top face
        0, 1, 2, 2, 3, 0,
        // Bottom face
        4, 5, 6, 6, 7, 4,
        // Side faces
        7, 6, 1, 1, 0, 7,
        6, 5, 2, 2, 1, 6,
        5, 4, 3, 3, 2, 5,
        4, 7, 0, 0, 3, 4,
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

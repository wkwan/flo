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
) {
    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Directional light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            ..default()
        },
        ..default()
    });

    // Water plane - subdivided quad for wave displacement
    let water_mesh = create_subdivided_plane(64, 64, 10.0);
    commands.spawn(PbrBundle {
        mesh: meshes.add(water_mesh),
        material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.1, 0.3, 0.8, 0.8),
            metallic: 0.0,
            roughness: 0.1,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        ..default()
    });
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

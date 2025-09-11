use bevy::{
    prelude::*, 
    render::mesh::{
        VertexAttributeValues,
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        Indices,
    },
    animation::{
        AnimationPlayer, 
        graph::{AnimationGraph, AnimationGraphHandle}
    }
};

use crate::skinned_mesh::{SkinnedVertex, SkinnedMeshData};

// Resource to store extracted mesh data
#[derive(Resource, Default)]
pub struct ExtractedMeshData {
    pub mesh_data: Option<SkinnedMeshData>,
}

// Resource to track loaded GLTF
#[derive(Resource)]
pub struct MeshGltf {
    pub handle: Handle<Gltf>,
    pub loaded: bool,
    pub scene_spawned: bool,
}

pub fn extract_mesh_data(
    mut extracted_data: ResMut<ExtractedMeshData>,
    mesh_query: Query<(&Mesh3d, Option<&SkinnedMesh>)>,
    mesh_assets: Res<Assets<Mesh>>,
    colonist_gltf: Res<MeshGltf>,
    transform_query: Query<&GlobalTransform>,
    inverse_bindposes_assets: Res<Assets<SkinnedMeshInverseBindposes>>,
) {
    if !colonist_gltf.loaded || !colonist_gltf.scene_spawned {
        return;
    }
    
    // Extract mesh data on first load
    if extracted_data.mesh_data.is_none() {
        // Extract mesh data from Bevy and convert to Vulkan format
        // Look for the largest skinned mesh (likely to be the colonist)
        let mut best_mesh = None;
        let mut best_vertex_count = 0;
        
        for (mesh_handle, skinned_mesh) in mesh_query.iter() {
            if let Some(mesh) = mesh_assets.get(&mesh_handle.0) {
                // Only consider meshes with skinning data
                if skinned_mesh.is_some() {
                    let vertex_count = mesh.count_vertices();
                    println!("Found skinned mesh with {} vertices", vertex_count);
                    
                    if vertex_count > best_vertex_count {
                        best_mesh = Some((mesh, skinned_mesh));
                        best_vertex_count = vertex_count;
                    }
                }
            }
        }
        
        if let Some((mesh, skinned_mesh)) = best_mesh {
            println!("Using best skinned mesh with {} vertices", best_vertex_count);
            // Convert Bevy mesh to our Vulkan mesh format
            match convert_bevy_mesh_to_vulkan(mesh, skinned_mesh) {
                Ok(vulkan_mesh_data) => {
                    println!("Successfully extracted mesh with {} vertices and {} indices",
                        vulkan_mesh_data.vertices.len(),
                        vulkan_mesh_data.indices.len());
                    
                    if skinned_mesh.is_some() {
                        println!("Mesh has skinning data with {} joints", 
                            vulkan_mesh_data.joint_matrices.len());
                    }
                    
                    // Log first vertex and some random vertices for debugging
                    if !vulkan_mesh_data.vertices.is_empty() {
                        let v = &vulkan_mesh_data.vertices[0];
                        println!("First vertex: pos={:?}, joints={:?}, weights={:?}", 
                            v.position, v.joint_indices, v.joint_weights);
                        
                        // Check for vertices with all zero weights or unusual patterns
                        let mut zero_weight_count = 0;
                        let mut single_joint_count = 0;
                        for (i, v) in vulkan_mesh_data.vertices.iter().enumerate() {
                            let total_weight = v.joint_weights[0] + v.joint_weights[1] + 
                                             v.joint_weights[2] + v.joint_weights[3];
                            if total_weight < 0.001 {
                                zero_weight_count += 1;
                                if i < 10 {
                                    println!("Vertex {} has zero weights: pos={:?}", i, v.position);
                                }
                            } else if v.joint_weights[1] < 0.001 && v.joint_weights[2] < 0.001 && v.joint_weights[3] < 0.001 {
                                single_joint_count += 1;
                            }
                        }
                        println!("Found {} vertices with zero weights", zero_weight_count);
                        println!("Found {} vertices bound to single joint", single_joint_count);
                    }
                    
                    extracted_data.mesh_data = Some(vulkan_mesh_data);
                }
                Err(e) => {
                    eprintln!("Failed to convert mesh: {}", e);
                }
            }
        } else {
            println!("No skinned meshes found!");
        }
    }
    
    // Always update joint matrices if we have skinned mesh data
    if extracted_data.mesh_data.is_some() {
        static mut FIRST_LOG: bool = true;
        
        for (_mesh_handle, skinned_mesh) in mesh_query.iter() {
            if let Some(skinned) = skinned_mesh {
                if let Some(ref mut mesh_data) = extracted_data.mesh_data {
                    // Get the inverse bind poses
                    let inverse_bindposes = inverse_bindposes_assets.get(&skinned.inverse_bindposes);
                    
                    for (i, joint_entity) in skinned.joints.iter().enumerate() {
                        if let Ok(transform) = transform_query.get(*joint_entity) {
                            if i < mesh_data.joint_matrices.len() {
                                let world_transform = transform.compute_matrix();
                                
                                // Apply inverse bind pose if available
                                let final_matrix = if let Some(inverse_bindposes) = inverse_bindposes {
                                    if i < inverse_bindposes.len() {
                                        // Correct skinning formula: world_transform * inverse_bind_pose
                                        world_transform * inverse_bindposes[i]
                                    } else {
                                        world_transform
                                    }
                                } else {
                                    world_transform
                                };
                                
                                mesh_data.joint_matrices[i] = final_matrix;
                                
                                // Check if matrices are changing
                                // unsafe {
                                //     if i == 5 {  // Check joint 5 which should be animated
                                //         if let Some(last) = LAST_MATRIX_0 {
                                //             if last != final_matrix {
                                //                 println!("Joint 5 is animating! New matrix: {:?}", final_matrix);
                                //             }
                                //         }
                                //         LAST_MATRIX_0 = Some(final_matrix);
                                //     }
                                    
                                //     if FIRST_LOG && i < 3 {
                                //         println!("Joint {} initial matrix (with inverse bind pose): {:?}", i, mesh_data.joint_matrices[i]);
                                //     }
                                // }
                            }
                        }
                    }
                    unsafe { FIRST_LOG = false; }
                }
                break; // Only process first skinned mesh
            }
        }
    }
}

fn convert_bevy_mesh_to_vulkan(
    mesh: &Mesh,
    skinned_mesh: Option<&SkinnedMesh>,
) -> Result<SkinnedMeshData, String> {
    // Extract positions
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .ok_or("Mesh missing position attribute")?
        .as_float3()
        .ok_or("Position attribute has wrong format")?;
    
    // Extract normals
    let default_normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .and_then(|attr| attr.as_float3())
        .unwrap_or(&default_normals);
    
    // Extract UVs - need to handle the proper conversion
    let default_uvs = vec![[0.0, 0.0]; positions.len()];
    let uvs = if let Some(uv_attr) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        match uv_attr {
            VertexAttributeValues::Float32x2(values) => values.as_slice(),
            _ => &default_uvs,
        }
    } else {
        &default_uvs
    };
    
    // Extract colors (if available)
    let colors: Vec<[f32; 4]> = if let Some(attr) = mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
        match attr {
            VertexAttributeValues::Float32x4(values) => values.clone(),
            VertexAttributeValues::Float32x3(values) => {
                values.iter().map(|v| [v[0], v[1], v[2], 1.0]).collect()
            }
            _ => vec![[1.0, 1.0, 1.0, 1.0]; positions.len()],
        }
    } else {
        vec![[0.8, 0.8, 0.8, 1.0]; positions.len()]  // Default gray color
    };
    
    // Extract joint indices and weights if this is a skinned mesh
    let joint_indices = if skinned_mesh.is_some() {
        mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
            .and_then(|attr| match attr {
                VertexAttributeValues::Uint16x4(values) => Some(values.clone()),
                _ => None,
            })
    } else {
        None
    };
    
    let joint_weights = if skinned_mesh.is_some() {
        mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
            .and_then(|attr| match attr {
                VertexAttributeValues::Float32x4(values) => Some(values.clone()),
                _ => None,
            })
    } else {
        None
    };
    
    // Don't apply skinning on CPU - it will be done in the vertex shader
    // Don't flip Y here - keep consistent coordinate system
    let final_positions: Vec<[f32; 3]> = positions.to_vec();
    
    // Convert to Skinned Vulkan vertices
    let mut vertices = Vec::new();
    for i in 0..final_positions.len() {
        // Get joint indices and weights for this vertex
        let indices = if let Some(ref ji) = joint_indices {
            if i < ji.len() {
                [ji[i][0] as u32, ji[i][1] as u32, ji[i][2] as u32, ji[i][3] as u32]
            } else {
                [0, 0, 0, 0]
            }
        } else {
            [0, 0, 0, 0]
        };
        
        let weights = if let Some(ref jw) = joint_weights {
            if i < jw.len() {
                jw[i]
            } else {
                [0.0, 0.0, 0.0, 0.0]
            }
        } else {
            [0.0, 0.0, 0.0, 0.0]
        };
        
        vertices.push(SkinnedVertex::new(
            final_positions[i],
            normals[i],
            uvs[i],
            colors[i],
            indices,
            weights,
        ));
    }
    
    // Extract indices
    let indices = match mesh.indices() {
        Some(Indices::U32(indices)) => indices.clone(),
        Some(Indices::U16(indices)) => indices.iter().map(|&i| i as u32).collect(),
        None => {
            // Generate default indices for triangle list
            (0..vertices.len() as u32).collect()
        }
    };
    
    // Initialize joint matrices - will be updated each frame
    let joint_matrices = if let Some(skinned) = skinned_mesh {
        println!("Initializing {} joint matrices", skinned.joints.len());
        vec![Mat4::IDENTITY; skinned.joints.len()]
    } else {
        println!("No skinned mesh, using single identity matrix");
        vec![Mat4::IDENTITY; 128]  // Full set of identity matrices
    };
    
    Ok(SkinnedMeshData::new(vertices, indices, joint_matrices))
}

pub fn animate_joints(
    _time: Res<Time>,
    gltf_assets: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    colonist_gltf: Res<MeshGltf>,
    mut animation_players: Query<(Entity, &mut AnimationPlayer)>,
    mut commands: Commands,
) {
    static mut ANIMATION_STARTED: bool = false;
    
    unsafe {
        if ANIMATION_STARTED {
            return;
        }
    }
    
    // Get the GLTF asset
    if let Some(gltf) = gltf_assets.get(&colonist_gltf.handle) {
        if !gltf.animations.is_empty() {
            // Create animation graph with the first animation
            let (graph, node_index) = AnimationGraph::from_clip(gltf.animations[0].clone());
            let graph_handle = graphs.add(graph);
            
            for (entity, mut player) in animation_players.iter_mut() {
                // Add the graph handle component to the entity
                commands.entity(entity).insert(AnimationGraphHandle(graph_handle.clone()));
                
                // Play the animation
                player.play(node_index).repeat();
                unsafe {
                    ANIMATION_STARTED = true;
                }
                println!("Started playing animation from GLB file");
            }
        }
    }
}
use crate::{mesh::MeshData, texture::TextureData, mesh::Vertex};
use gltf;
use std::path::Path;

pub struct GltfData {
    pub mesh_data: MeshData,
    pub texture_data: Option<TextureData>,
}

impl GltfData {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();
        
        let (document, buffers, images) = gltf::import(path)
            .map_err(|e| format!("Failed to load GLB file: {}", e))?;
        
        let texture_data = Self::extract_texture(&images);
        let mesh_data = Self::extract_mesh(&document, &buffers)?;
        
        Ok(GltfData {
            mesh_data,
            texture_data,
        })
    }
    
    fn extract_texture(images: &[gltf::image::Data]) -> Option<TextureData> {
        if images.is_empty() {
            println!("No textures found in GLB file, using default color");
            return None;
        }
        
        println!("Found {} textures in GLB file", images.len());
        
        let image = images.first()?;
        println!("Using texture with dimensions {}x{}, format: {:?}", 
                 image.width, image.height, image.format);
        
        let rgba_pixels = match image.format {
            gltf::image::Format::R8G8B8A8 => image.pixels.clone(),
            gltf::image::Format::R8G8B8 => {
                let mut rgba = Vec::with_capacity(image.pixels.len() * 4 / 3);
                for chunk in image.pixels.chunks(3) {
                    rgba.push(chunk[0]);
                    rgba.push(chunk[1]);
                    rgba.push(chunk[2]);
                    rgba.push(255);
                }
                rgba
            },
            _ => {
                println!("Unsupported texture format, using placeholder");
                return Some(TextureData::placeholder());
            }
        };
        
        Some(TextureData::new(rgba_pixels, image.width, image.height))
    }
    
    fn extract_mesh(document: &gltf::Document, buffers: &[gltf::buffer::Data]) -> Result<MeshData, String> {
        let mut combined_vertices = Vec::new();
        let mut combined_indices = Vec::new();
        let mut vertex_offset = 0u32;
        
        for mesh in document.meshes() {
            println!("Processing mesh: {:?}", mesh.name());
            
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                
                // Get material color if available
                let material = primitive.material();
                let material_color = material.pbr_metallic_roughness().base_color_factor();
                
                let positions: Vec<[f32; 3]> = reader
                    .read_positions()
                    .ok_or("Mesh should have positions")?
                    .collect();
                
                let normals: Vec<[f32; 3]> = reader
                    .read_normals()
                    .map(|iter| iter.collect())
                    .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
                
                let uvs: Vec<[f32; 2]> = reader
                    .read_tex_coords(0)
                    .map(|iter| iter.into_f32().collect())
                    .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
                
                // Check if vertex colors exist, otherwise use material color
                let colors: Vec<[f32; 4]> = reader
                    .read_colors(0)
                    .map(|iter| iter.into_rgba_f32().collect())
                    .unwrap_or_else(|| vec![material_color; positions.len()]);
                
                for i in 0..positions.len() {
                    combined_vertices.push(Vertex::with_color(
                        positions[i],
                        normals[i],
                        uvs[i],
                        colors[i],
                    ));
                }
                
                if let Some(indices_reader) = reader.read_indices() {
                    let indices: Vec<u32> = indices_reader.into_u32().collect();
                    for triangle in indices.chunks(3) {
                        if triangle.len() == 3 {
                            combined_indices.push(triangle[0] + vertex_offset);
                            combined_indices.push(triangle[2] + vertex_offset);
                            combined_indices.push(triangle[1] + vertex_offset);
                        }
                    }
                }
                
                vertex_offset += positions.len() as u32;
            }
        }
        
        println!("Loaded mesh with {} vertices and {} indices", 
                 combined_vertices.len(), combined_indices.len());
        
        if combined_vertices.is_empty() {
            return Err("No mesh data found in GLB file".to_string());
        }
        
        Ok(MeshData::new(combined_vertices, combined_indices))
    }
}
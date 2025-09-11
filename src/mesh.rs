#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            uv,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
    
    pub fn with_color(position: [f32; 3], normal: [f32; 3], uv: [f32; 2], color: [f32; 4]) -> Self {
        Self {
            position,
            normal,
            uv,
            color,
        }
    }
    
    pub fn get_binding_description() -> ash::vk::VertexInputBindingDescription {
        ash::vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(ash::vk::VertexInputRate::VERTEX)
    }
    
    pub fn get_attribute_descriptions() -> Vec<ash::vk::VertexInputAttributeDescription> {
        vec![
            // Position
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .offset(0),
            // Normal
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .offset(12),
            // UV
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(ash::vk::Format::R32G32_SFLOAT)
                .offset(24),
            // Color
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(3)
                .format(ash::vk::Format::R32G32B32A32_SFLOAT)
                .offset(32),
        ]
    }
}

pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Self { vertices, indices }
    }
    
    pub fn from_bevy_mesh(mesh: &bevy::render::mesh::Mesh) -> Option<Self> {
        use bevy::render::mesh::VertexAttributeValues;
        
        let positions = mesh.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_POSITION)?;
        let normals = mesh.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_NORMAL);
        let uvs = mesh.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_UV_0);
        let colors = mesh.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_COLOR);
        
        let positions = match positions {
            VertexAttributeValues::Float32x3(values) => values,
            _ => return None,
        };
        
        let normals = normals.and_then(|n| match n {
            VertexAttributeValues::Float32x3(values) => Some(values),
            _ => None,
        });
        
        let uvs = uvs.and_then(|u| match u {
            VertexAttributeValues::Float32x2(values) => Some(values),
            _ => None,
        });
        
        let colors = colors.and_then(|c| match c {
            VertexAttributeValues::Float32x4(values) => Some(values),
            _ => None,
        });
        
        let mut vertices = Vec::with_capacity(positions.len());
        for i in 0..positions.len() {
            let position = positions[i];
            let normal = normals.map(|n| n[i]).unwrap_or([0.0, 1.0, 0.0]);
            let uv = uvs.map(|u| u[i]).unwrap_or([0.0, 0.0]);
            let color = colors.map(|c| c[i]).unwrap_or([1.0, 1.0, 1.0, 1.0]);
            
            vertices.push(Vertex::with_color(position, normal, uv, color));
        }
        
        let indices = mesh.indices()?.iter().map(|i| i as u32).collect();
        
        Some(MeshData::new(vertices, indices))
    }
}
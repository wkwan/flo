#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TexturedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub texture_index: u32,
}

impl TexturedVertex {
    pub fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2], texture_index: u32) -> Self {
        Self {
            position,
            normal,
            uv,
            texture_index,
        }
    }
    
    pub fn get_binding_description() -> ash::vk::VertexInputBindingDescription {
        ash::vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<TexturedVertex>() as u32)
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
            // Texture Index
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(3)
                .format(ash::vk::Format::R32_UINT)
                .offset(32),
        ]
    }
}

pub struct TexturedMeshData {
    pub vertices: Vec<TexturedVertex>,
    pub indices: Vec<u32>,
}

impl TexturedMeshData {
    pub fn new(vertices: Vec<TexturedVertex>, indices: Vec<u32>) -> Self {
        Self { vertices, indices }
    }
}
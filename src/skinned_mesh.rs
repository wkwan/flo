use bevy::math::Mat4;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkinnedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub joint_indices: [u32; 4],  // Up to 4 joints per vertex
    pub joint_weights: [f32; 4],  // Corresponding weights
}

impl SkinnedVertex {
    pub fn new(
        position: [f32; 3],
        normal: [f32; 3],
        uv: [f32; 2],
        color: [f32; 4],
        joint_indices: [u32; 4],
        joint_weights: [f32; 4],
    ) -> Self {
        Self {
            position,
            normal,
            uv,
            color,
            joint_indices,
            joint_weights,
        }
    }
    
    pub fn get_binding_description() -> ash::vk::VertexInputBindingDescription {
        ash::vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<SkinnedVertex>() as u32)
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
            // Joint Indices
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(4)
                .format(ash::vk::Format::R32G32B32A32_UINT)
                .offset(48),
            // Joint Weights
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(5)
                .format(ash::vk::Format::R32G32B32A32_SFLOAT)
                .offset(64),
        ]
    }
}

pub struct SkinnedMeshData {
    pub vertices: Vec<SkinnedVertex>,
    pub indices: Vec<u32>,
    pub joint_matrices: Vec<Mat4>,
}

impl SkinnedMeshData {
    pub fn new(vertices: Vec<SkinnedVertex>, indices: Vec<u32>, joint_matrices: Vec<Mat4>) -> Self {
        Self {
            vertices,
            indices,
            joint_matrices,
        }
    }
}
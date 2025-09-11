use ash::vk;
use bevy::prelude::*;
use bevy::window::RawHandleWrapperHolder;
use bevy::math::{Mat4, Vec3};
use std::mem;
use memoffset::offset_of;
use crate::vulkan_common::*;
use crate::constants::*;
use crate::mesh::{Vertex, MeshData};
use crate::skinned_mesh::{SkinnedVertex, SkinnedMeshData};
use crate::mesh_textured::{TexturedMeshData, TexturedVertex};
use crate::texture::{TextureData, Texture};
use crate::egui_integration::EguiIntegration;
use crate::memory_pool::{MemoryPoolManager, MemoryBlock};

// Optional resources for different renderer configurations
pub struct BufferResources {
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: Option<vk::Buffer>,
    pub index_buffer_memory: Option<vk::DeviceMemory>,
    pub index_count: u32,
    pub instance_buffer: Option<vk::Buffer>,
    pub instance_buffer_memory: Option<vk::DeviceMemory>,
}

pub struct TextureResources {
    pub image: vk::Image,
    pub image_memory: vk::DeviceMemory,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
}

pub struct TextureArrayResources {
    pub texture_array: vk::Image,
    pub texture_array_memory: vk::DeviceMemory,
    pub texture_array_view: vk::ImageView,
    pub texture_sampler: vk::Sampler,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_set: vk::DescriptorSet,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CameraUniforms {
    view: [f32; 16],
    proj: [f32; 16],
}

pub struct SkinnedMeshResources {
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub index_count: u32,
    pub joint_uniform_buffer: vk::Buffer,
    pub joint_uniform_memory: vk::DeviceMemory,
    pub camera_uniform_buffer: vk::Buffer,
    pub camera_uniform_memory: vk::DeviceMemory,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    // Instance buffer fields for GPU instancing
    pub instance_buffer: Option<vk::Buffer>,
    pub instance_buffer_memory: Option<vk::DeviceMemory>,
    pub instance_count: u32,
    pub use_instancing: bool,
}

// Structure to hold mesh data for multi-mesh rendering
pub struct MeshEntry {
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: Option<vk::DeviceMemory>,  // None if using memory pool
    pub vertex_memory_block: Option<MemoryBlock>,  // Some if using memory pool
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: Option<vk::DeviceMemory>,  // None if using memory pool
    pub index_memory_block: Option<MemoryBlock>,  // Some if using memory pool
    pub index_count: u32,
    pub transforms: Vec<Mat4>,  // Transform matrices for instances of this mesh
    pub pipeline_name: Option<String>,  // Optional pipeline name for this mesh
    pub texture_resources: Option<TextureResources>,  // Optional texture for this mesh
    // Instance buffer for GPU instancing (optional)
    pub instance_buffer: Option<vk::Buffer>,
    pub instance_buffer_memory: Option<vk::DeviceMemory>,
    pub instance_memory_block: Option<MemoryBlock>,  // Some if using memory pool
    pub instance_count: u32,
    pub use_instancing: bool,  // If true, use GPU instancing instead of iterating transforms
    pub base_color: [f32; 4],  // Base color for this mesh (used in shaders via push constants)
    // Skinned mesh support - joint matrices for skeletal animation
    pub joint_matrices: Option<Vec<Mat4>>,  // Joint matrices for this mesh (if skinned)
    pub joint_buffer: Option<vk::Buffer>,  // Buffer to store joint matrices on GPU
    pub joint_buffer_memory: Option<vk::DeviceMemory>,
    pub is_skinned: bool,  // Whether this mesh uses skeletal animation
    // Descriptor sets for skinned mesh (joint matrices + camera uniforms)
    pub skinned_descriptor_pool: Option<vk::DescriptorPool>,
    pub skinned_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    pub skinned_descriptor_sets: Option<Vec<vk::DescriptorSet>>,
    pub camera_uniform_buffer: Option<vk::Buffer>,
    pub camera_uniform_memory: Option<vk::DeviceMemory>
}

// Structure to hold a pipeline and its layout
pub struct Pipeline {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
}

// Structure to hold textured pipeline resources
struct TexturedPipelineResources {
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
}

#[repr(C, align(4))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PushConstants {
    pub time: f32,               // offset 0, size 4
    pub camera_position_x: f32,  // offset 4, size 4  
    pub camera_position_y: f32,  // offset 8, size 4
    pub camera_position_z: f32,  // offset 12, size 4
    pub resolution: [f32; 2],    // offset 16, size 8
    pub water_level: f32,        // offset 24, size 4
    pub grid_scale: f32,         // offset 28, size 4
}

pub struct VulkanRenderer {
    pub(crate) core: VulkanCore,
    pipeline_layout: vk::PipelineLayout,  // Default pipeline layout (for compatibility)
    graphics_pipeline: vk::Pipeline,       // Default pipeline (for compatibility)
    
    // Multiple pipelines support
    pipelines: std::collections::HashMap<String, Pipeline>,
    current_pipeline: String,
    
    // Configuration
    vertex_count: u32,  // For simple non-buffer rendering
    instance_count: u32,
    has_depth: bool,
    
    // Optional resources
    buffers: Option<BufferResources>,
    textures: Option<TextureResources>,
    texture_arrays: Option<TextureArrayResources>,
    skinned_mesh: Option<SkinnedMeshResources>,
    
    // Multi-mesh support
    meshes: Vec<MeshEntry>,
    
    // Memory pool manager
    memory_pool: MemoryPoolManager,
    
    // Egui integration
    pub egui_integration: Option<EguiIntegration>,
    
    // Water rendering push constants
    water_push_constants: Option<PushConstants>,
    
    // Textured pipelines (for multi-texture support)
    textured_pipelines: std::collections::HashMap<String, TexturedPipelineResources>,
}

impl VulkanRenderer {
    // Helper constructor for MeshData
    pub fn new_from_mesh_data(
        window_handle: &RawHandleWrapperHolder,
        vert_shader_path: &str,
        frag_shader_path: &str,
        mesh_data: &MeshData,
        instance_count: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_from_mesh_data_with_winding(window_handle, vert_shader_path, frag_shader_path, mesh_data, instance_count, None)
    }

    pub fn new_from_mesh_data_with_winding(
        window_handle: &RawHandleWrapperHolder,
        vert_shader_path: &str,
        frag_shader_path: &str,
        mesh_data: &MeshData,
        instance_count: u32,
        front_face: Option<vk::FrontFace>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_mesh_with_winding(
            window_handle,
            vert_shader_path,
            frag_shader_path,
            &mesh_data.vertices,
            &mesh_data.indices,
            vec![Vertex::get_binding_description()],
            Vertex::get_attribute_descriptions(),
            instance_count,
            front_face,
        )
    }
    
    // Constructor for simple triangle/cube (no buffers)
    pub fn new_simple(
        window_handle: &RawHandleWrapperHolder,
        vert_shader_path: &str,
        frag_shader_path: &str,
        vertex_count: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let with_depth = vertex_count == 36; // Cube needs depth
        let core = VulkanCore::new(window_handle, with_depth)?;
        
        let push_constants = if vert_shader_path.contains("cube") {
            vec![vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .offset(0)
                .size(4)]
        } else {
            Vec::new()
        };
        
        let (graphics_pipeline, pipeline_layout) = PipelineBuilder::new(
            core.device.clone(),
            vert_shader_path,
            frag_shader_path,
            core.swapchain_extent,
            core.render_pass,
        )?
        .with_vertex_input(Vec::new(), Vec::new())
        .with_push_constants(push_constants)
        .with_depth_test(with_depth)
        .with_cull_mode(vk::CullModeFlags::NONE)
        .with_front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .build()?;
        
        let mut pipelines = std::collections::HashMap::new();
        pipelines.insert("default".to_string(), Pipeline {
            pipeline: graphics_pipeline,
            layout: pipeline_layout,
        });
        
        let memory_pool = MemoryPoolManager::new(core.device.clone());
        
        Ok(Self {
            core,
            pipeline_layout,
            graphics_pipeline,
            pipelines,
            current_pipeline: "default".to_string(),
            vertex_count,
            instance_count: 1,
            has_depth: with_depth,
            buffers: None,
            textures: None,
            texture_arrays: None,
            skinned_mesh: None,
            meshes: Vec::new(),
            memory_pool,
            egui_integration: None,
            water_push_constants: None,
            textured_pipelines: std::collections::HashMap::new(),
        })
    }
    
    // Constructor for mesh rendering (with buffers)
    pub fn new_mesh<T: Copy>(
        window_handle: &RawHandleWrapperHolder,
        vert_shader_path: &str,
        frag_shader_path: &str,
        vertices: &[T],
        indices: &[u32],
        binding_descriptions: Vec<vk::VertexInputBindingDescription>,
        attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
        instance_count: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_mesh_with_winding(window_handle, vert_shader_path, frag_shader_path, vertices, indices, binding_descriptions, attribute_descriptions, instance_count, None)
    }

    pub fn new_mesh_with_winding<T: Copy>(
        window_handle: &RawHandleWrapperHolder,
        vert_shader_path: &str,
        frag_shader_path: &str,
        vertices: &[T],
        indices: &[u32],
        binding_descriptions: Vec<vk::VertexInputBindingDescription>,
        attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
        instance_count: u32,
        _front_face: Option<vk::FrontFace>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let core = VulkanCore::new(window_handle, true)?;
        
        // Create memory pool first
        let memory_pool = MemoryPoolManager::new(core.device.clone());
        
        // Create buffers - still use regular allocation for initial buffer
        // since BufferResources expects DeviceMemory not MemoryBlock
        let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            vertices,
        )?;
        
        let (index_buffer, index_buffer_memory) = create_index_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            indices,
        )?;
        
        // Configure push constants for MVP matrices
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(mem::size_of::<[f32; 16]>() as u32 * 3);
        
        let (graphics_pipeline, pipeline_layout) = PipelineBuilder::new(
            core.device.clone(),
            vert_shader_path,
            frag_shader_path,
            core.swapchain_extent,
            core.render_pass,
        )?
        .with_vertex_input(binding_descriptions, attribute_descriptions)
        .with_push_constants(vec![push_constant_range])
        .with_depth_test(true)
        .with_cull_mode(vk::CullModeFlags::BACK)
        .with_front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .build()?;
        
        let buffers = BufferResources {
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer: Some(index_buffer),
            index_buffer_memory: Some(index_buffer_memory),
            index_count: indices.len() as u32,
            instance_buffer: None,
            instance_buffer_memory: None,
        };
        
        let mut pipelines = std::collections::HashMap::new();
        pipelines.insert("default".to_string(), Pipeline {
            pipeline: graphics_pipeline,
            layout: pipeline_layout,
        });
        
        Ok(Self {
            core,
            pipeline_layout,
            graphics_pipeline,
            pipelines,
            current_pipeline: "default".to_string(),
            vertex_count: 0,
            instance_count,
            has_depth: true,
            buffers: Some(buffers),
            textures: None,
            texture_arrays: None,
            skinned_mesh: None,
            meshes: Vec::new(),
            memory_pool,
            egui_integration: None,
            water_push_constants: None,
            textured_pipelines: std::collections::HashMap::new(),
        })
    }
    
    // Constructor for textured rendering
    // Constructor for texture array rendering
    pub fn new_texture_array(
        window_handle: &RawHandleWrapperHolder,
        vert_shader_path: &str,
        frag_shader_path: &str,
        mesh_data: &TexturedMeshData,
        textures: &[TextureData],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let core = VulkanCore::new(window_handle, true)?;
        
        // Create vertex buffer
        let (vertex_buffer, vertex_buffer_memory) = create_textured_vertex_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            &mesh_data.vertices,
        )?;
        
        // Create index buffer
        let (index_buffer, index_buffer_memory) = create_index_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            &mesh_data.indices,
        )?;
        
        // Create texture array
        const MAX_TEXTURE_SIZE: u32 = 512;
        let max_width = textures.iter().map(|t| t.width.min(MAX_TEXTURE_SIZE)).max().unwrap_or(1);
        let max_height = textures.iter().map(|t| t.height.min(MAX_TEXTURE_SIZE)).max().unwrap_or(1);
        let layer_count = textures.len().min(256) as u32;
        
        let (texture_array, texture_array_memory) = create_texture_array(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            textures,
            max_width,
            max_height,
        )?;
        
        let texture_array_view = create_texture_array_view(&core.device, texture_array, layer_count)?;
        let texture_sampler = crate::vulkan_common::create_texture_sampler(&core.instance, &core.device, core.physical_device)?;
        
        // Create descriptor resources
        let binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);
        
        let descriptor_set_layout = create_descriptor_set_layout(&core.device, &[binding])?;
        
        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1);
        
        let descriptor_pool = create_descriptor_pool(&core.device, 1, &[pool_size])?;
        
        let layouts = vec![descriptor_set_layout];
        let descriptor_sets = allocate_descriptor_sets(&core.device, descriptor_pool, &layouts)?;
        let descriptor_set = descriptor_sets[0];
        
        // Update descriptor set
        update_descriptor_sets_texture(&core.device, descriptor_set, texture_array_view, texture_sampler, 0);
        
        // Configure pipeline with push constants for view-proj matrix
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(mem::size_of::<[f32; 16]>() as u32);
        
        let binding_descriptions = vec![TexturedVertex::get_binding_description()];
        let attribute_descriptions = TexturedVertex::get_attribute_descriptions();
        
        let (graphics_pipeline, pipeline_layout) = PipelineBuilder::new(
            core.device.clone(),
            vert_shader_path,
            frag_shader_path,
            core.swapchain_extent,
            core.render_pass,
        )?
        .with_vertex_input(binding_descriptions, attribute_descriptions)
        .with_push_constants(vec![push_constant_range])
        .with_descriptor_sets(vec![descriptor_set_layout])
        .with_depth_test(true)
        .with_cull_mode(vk::CullModeFlags::BACK)
        .with_front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .build()?;
        
        let buffers = BufferResources {
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer: Some(index_buffer),
            index_buffer_memory: Some(index_buffer_memory),
            index_count: mesh_data.indices.len() as u32,
            instance_buffer: None,
            instance_buffer_memory: None,
        };
        
        let texture_arrays = TextureArrayResources {
            texture_array,
            texture_array_memory,
            texture_array_view,
            texture_sampler,
            descriptor_pool,
            descriptor_set_layout,
            descriptor_set,
        };
        
        let mut pipelines = std::collections::HashMap::new();
        pipelines.insert("default".to_string(), Pipeline {
            pipeline: graphics_pipeline,
            layout: pipeline_layout,
        });
        
        let memory_pool = MemoryPoolManager::new(core.device.clone());
        
        Ok(Self {
            core,
            pipeline_layout,
            graphics_pipeline,
            pipelines,
            current_pipeline: "default".to_string(),
            vertex_count: 0,
            instance_count: 1,
            has_depth: true,
            buffers: Some(buffers),
            textures: None,
            texture_arrays: Some(texture_arrays),
            skinned_mesh: None,
            meshes: Vec::new(),
            memory_pool,
            egui_integration: None,
            water_push_constants: None,
            textured_pipelines: std::collections::HashMap::new(),
        })
    }
    
    // Constructor for instanced textured rendering
    pub fn new_textured_instanced(
        window_handle: &RawHandleWrapperHolder,
        vert_shader_path: &str,
        frag_shader_path: &str,
        mesh_data: &MeshData,
        texture_path: Option<&str>,
        instance_positions: &[[f32; 3]],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let core = VulkanCore::new(window_handle, true)?;
        
        // Create vertex buffer for mesh data
        let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            &mesh_data.vertices,
        )?;
        
        // Create index buffer
        let (index_buffer, index_buffer_memory) = create_index_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            &mesh_data.indices,
        )?;
        
        // Create instance buffer
        let (instance_buffer, instance_buffer_memory) = create_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            (instance_positions.len() * std::mem::size_of::<[f32; 3]>()) as vk::DeviceSize,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Copy instance data to buffer
        unsafe {
            let data = core.device.map_memory(
                instance_buffer_memory,
                0,
                (instance_positions.len() * std::mem::size_of::<[f32; 3]>()) as vk::DeviceSize,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy_nonoverlapping(
                instance_positions.as_ptr() as *const u8,
                data as *mut u8,
                instance_positions.len() * std::mem::size_of::<[f32; 3]>(),
            );
            core.device.unmap_memory(instance_buffer_memory);
        }
        
        // Vertex input configuration - need both per-vertex and per-instance attributes
        let binding_descriptions = vec![
            // Per-vertex data
            vk::VertexInputBindingDescription::default()
                .binding(0)
                .stride(std::mem::size_of::<Vertex>() as u32)
                .input_rate(vk::VertexInputRate::VERTEX),
            // Per-instance data
            vk::VertexInputBindingDescription::default()
                .binding(1)
                .stride(std::mem::size_of::<[f32; 3]>() as u32)
                .input_rate(vk::VertexInputRate::INSTANCE),
        ];
        
        let mut attribute_descriptions = Vertex::get_attribute_descriptions();
        
        // Add instance position attribute (location depends on whether we have UVs)
        let instance_location = if texture_path.is_some() { 3 } else { 2 };
        attribute_descriptions.push(
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(instance_location)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
        );
        
        // Create texture resources if path provided
        let (textures, descriptor_set_layout) = if let Some(path) = texture_path {
            // Create texture resources
            let (texture_image, texture_image_memory) = crate::vulkan_common::create_texture_image(
                &core.instance,
                &core.device,
                core.physical_device,
                core.command_pool,
                core.graphics_queue,
                path,
            )?;
            
            let texture_image_view = crate::vulkan_common::create_texture_image_view(&core.device, texture_image)?;
            let texture_sampler = crate::vulkan_common::create_texture_sampler(&core.instance, &core.device, core.physical_device)?;
            
            // Create descriptor resources
            let binding = vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT);
            
            let descriptor_set_layout = create_descriptor_set_layout(&core.device, &[binding])?;
            
            let pool_size = vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(core.swapchain_images.len() as u32);
            
            let descriptor_pool = create_descriptor_pool(&core.device, core.swapchain_images.len() as u32, &[pool_size])?;
            
            let layouts = vec![descriptor_set_layout; core.swapchain_images.len()];
            let descriptor_sets = allocate_descriptor_sets(&core.device, descriptor_pool, &layouts)?;
            
            // Update descriptor sets
            for &descriptor_set in &descriptor_sets {
                update_descriptor_sets_texture(&core.device, descriptor_set, texture_image_view, texture_sampler, 0);
            }
            
            let textures = TextureResources {
                image: texture_image,
                image_memory: texture_image_memory,
                image_view: texture_image_view,
                sampler: texture_sampler,
                descriptor_pool,
                descriptor_set_layout,
                descriptor_sets,
            };
            
            (Some(textures), Some(descriptor_set_layout))
        } else {
            (None, None)
        };
        
        // Configure push constants for time
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<f32>() as u32);
        
        // Build pipeline
        let mut pipeline_builder = PipelineBuilder::new(
            core.device.clone(),
            vert_shader_path,
            frag_shader_path,
            core.swapchain_extent,
            core.render_pass,
        )?
        .with_vertex_input(binding_descriptions, attribute_descriptions)
        .with_push_constants(vec![push_constant_range])
        .with_depth_test(true)
        .with_cull_mode(vk::CullModeFlags::BACK)
        .with_front_face(vk::FrontFace::COUNTER_CLOCKWISE);
        
        if let Some(layout) = descriptor_set_layout {
            pipeline_builder = pipeline_builder.with_descriptor_sets(vec![layout]);
        }
        
        let (graphics_pipeline, pipeline_layout) = pipeline_builder.build()?;
        
        let buffers = BufferResources {
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer: Some(index_buffer),
            index_buffer_memory: Some(index_buffer_memory),
            index_count: mesh_data.indices.len() as u32,
            instance_buffer: Some(instance_buffer),
            instance_buffer_memory: Some(instance_buffer_memory),
        };
        
        let mut pipelines = std::collections::HashMap::new();
        pipelines.insert("default".to_string(), Pipeline {
            pipeline: graphics_pipeline,
            layout: pipeline_layout,
        });
        
        let memory_pool = MemoryPoolManager::new(core.device.clone());
        
        Ok(Self {
            core,
            pipeline_layout,
            graphics_pipeline,
            pipelines,
            current_pipeline: "default".to_string(),
            vertex_count: 0,
            instance_count: instance_positions.len() as u32,
            has_depth: true,
            buffers: Some(buffers),
            textures,
            texture_arrays: None,
            skinned_mesh: None,
            meshes: Vec::new(),
            memory_pool,
            egui_integration: None,
            water_push_constants: None,
            textured_pipelines: std::collections::HashMap::new(),
        })
    }
    
    pub fn new_textured_instanced_with_winding(
        window_handle: &RawHandleWrapperHolder,
        vert_shader_path: &str,
        frag_shader_path: &str,
        mesh_data: &MeshData,
        texture_path: Option<&str>,
        instance_positions: &[[f32; 3]],
        front_face: Option<vk::FrontFace>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let core = VulkanCore::new(window_handle, true)?;
        
        // Create vertex buffer for mesh data
        let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            &mesh_data.vertices,
        )?;
        
        // Create index buffer
        let (index_buffer, index_buffer_memory) = create_index_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            &mesh_data.indices,
        )?;
        
        // Create instance buffer
        let (instance_buffer, instance_buffer_memory) = create_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            (instance_positions.len() * std::mem::size_of::<[f32; 3]>()) as vk::DeviceSize,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Copy instance data to buffer
        unsafe {
            let data = core.device.map_memory(
                instance_buffer_memory,
                0,
                (instance_positions.len() * std::mem::size_of::<[f32; 3]>()) as vk::DeviceSize,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy_nonoverlapping(
                instance_positions.as_ptr() as *const u8,
                data as *mut u8,
                instance_positions.len() * std::mem::size_of::<[f32; 3]>(),
            );
            core.device.unmap_memory(instance_buffer_memory);
        }
        
        // Vertex input configuration - need both per-vertex and per-instance attributes
        let binding_descriptions = vec![
            // Per-vertex data
            vk::VertexInputBindingDescription::default()
                .binding(0)
                .stride(std::mem::size_of::<Vertex>() as u32)
                .input_rate(vk::VertexInputRate::VERTEX),
            // Per-instance data
            vk::VertexInputBindingDescription::default()
                .binding(1)
                .stride(std::mem::size_of::<[f32; 3]>() as u32)
                .input_rate(vk::VertexInputRate::INSTANCE),
        ];
        
        let mut attribute_descriptions = Vertex::get_attribute_descriptions();
        
        // Add instance position attribute (location depends on whether we have UVs)
        let instance_location = if texture_path.is_some() { 3 } else { 2 };
        attribute_descriptions.push(
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(instance_location)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
        );
        
        // Create texture resources if path provided
        let (textures, descriptor_set_layout) = if let Some(path) = texture_path {
            // Create texture resources
            let (texture_image, texture_image_memory) = crate::vulkan_common::create_texture_image(
                &core.instance,
                &core.device,
                core.physical_device,
                core.command_pool,
                core.graphics_queue,
                path,
            )?;
            
            let texture_image_view = crate::vulkan_common::create_texture_image_view(&core.device, texture_image)?;
            let texture_sampler = crate::vulkan_common::create_texture_sampler(&core.instance, &core.device, core.physical_device)?;
            
            // Create descriptor resources
            let binding = vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT);
            
            let descriptor_set_layout = create_descriptor_set_layout(&core.device, &[binding])?;
            
            let pool_size = vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(core.swapchain_images.len() as u32);
            
            let descriptor_pool = create_descriptor_pool(&core.device, core.swapchain_images.len() as u32, &[pool_size])?;
            
            let layouts = vec![descriptor_set_layout; core.swapchain_images.len()];
            let descriptor_sets = allocate_descriptor_sets(&core.device, descriptor_pool, &layouts)?;
            
            // Update descriptor sets
            for &descriptor_set in &descriptor_sets {
                update_descriptor_sets_texture(&core.device, descriptor_set, texture_image_view, texture_sampler, 0);
            }
            
            let textures = TextureResources {
                image: texture_image,
                image_memory: texture_image_memory,
                image_view: texture_image_view,
                sampler: texture_sampler,
                descriptor_pool,
                descriptor_set_layout,
                descriptor_sets,
            };
            
            (Some(textures), Some(descriptor_set_layout))
        } else {
            (None, None)
        };
        
        // Configure push constants for time
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<f32>() as u32);
        
        // Build pipeline
        let mut pipeline_builder = PipelineBuilder::new(
            core.device.clone(),
            vert_shader_path,
            frag_shader_path,
            core.swapchain_extent,
            core.render_pass,
        )?
        .with_vertex_input(binding_descriptions, attribute_descriptions)
        .with_push_constants(vec![push_constant_range])
        .with_depth_test(true)
        .with_cull_mode(vk::CullModeFlags::BACK)
        .with_front_face(if let Some(face) = front_face { face } else { vk::FrontFace::COUNTER_CLOCKWISE });
        
        if let Some(layout) = descriptor_set_layout {
            pipeline_builder = pipeline_builder.with_descriptor_sets(vec![layout]);
        }
        
        let (graphics_pipeline, pipeline_layout) = pipeline_builder.build()?;
        
        let buffers = BufferResources {
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer: Some(index_buffer),
            index_buffer_memory: Some(index_buffer_memory),
            index_count: mesh_data.indices.len() as u32,
            instance_buffer: Some(instance_buffer),
            instance_buffer_memory: Some(instance_buffer_memory),
        };
        
        let mut pipelines = std::collections::HashMap::new();
        pipelines.insert("default".to_string(), Pipeline {
            pipeline: graphics_pipeline,
            layout: pipeline_layout,
        });
        
        let memory_pool = MemoryPoolManager::new(core.device.clone());
        
        Ok(Self {
            core,
            pipeline_layout,
            graphics_pipeline,
            pipelines,
            current_pipeline: "default".to_string(),
            vertex_count: 0,
            instance_count: instance_positions.len() as u32,
            has_depth: true,
            buffers: Some(buffers),
            textures,
            texture_arrays: None,
            skinned_mesh: None,
            meshes: Vec::new(),
            memory_pool,
            egui_integration: None,
            water_push_constants: None,
            textured_pipelines: std::collections::HashMap::new(),
        })
    }
    
    pub fn new_textured<T: Copy>(
        window_handle: &RawHandleWrapperHolder,
        vert_shader_path: &str,
        frag_shader_path: &str,
        vertices: &[T],
        indices: &[u32],
        binding_descriptions: Vec<vk::VertexInputBindingDescription>,
        attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
        texture_path: &str,
        instance_count: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let core = VulkanCore::new(window_handle, true)?;
        
        // Create buffers
        let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            vertices,
        )?;
        
        let (index_buffer, index_buffer_memory) = create_index_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            indices,
        )?;
        
        // Create texture resources
        let (texture_image, texture_image_memory) = crate::vulkan_common::create_texture_image(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            texture_path,
        )?;
        
        let texture_image_view = crate::vulkan_common::create_texture_image_view(&core.device, texture_image)?;
        let texture_sampler = crate::vulkan_common::create_texture_sampler(&core.instance, &core.device, core.physical_device)?;
        
        // Create descriptor resources
        let binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);
        
        let descriptor_set_layout = create_descriptor_set_layout(&core.device, &[binding])?;
        
        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(core.swapchain_images.len() as u32);
        
        let descriptor_pool = create_descriptor_pool(&core.device, core.swapchain_images.len() as u32, &[pool_size])?;
        
        let layouts = vec![descriptor_set_layout; core.swapchain_images.len()];
        let descriptor_sets = allocate_descriptor_sets(&core.device, descriptor_pool, &layouts)?;
        
        // Update descriptor sets
        for &descriptor_set in &descriptor_sets {
            update_descriptor_sets_texture(&core.device, descriptor_set, texture_image_view, texture_sampler, 0);
        }
        
        // Configure pipeline
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(mem::size_of::<[f32; 16]>() as u32 * 3);
        
        let (graphics_pipeline, pipeline_layout) = PipelineBuilder::new(
            core.device.clone(),
            vert_shader_path,
            frag_shader_path,
            core.swapchain_extent,
            core.render_pass,
        )?
        .with_vertex_input(binding_descriptions, attribute_descriptions)
        .with_push_constants(vec![push_constant_range])
        .with_descriptor_sets(vec![descriptor_set_layout])
        .with_depth_test(true)
        .with_cull_mode(vk::CullModeFlags::BACK)
        .with_front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .build()?;
        
        let buffers = BufferResources {
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer: Some(index_buffer),
            index_buffer_memory: Some(index_buffer_memory),
            index_count: indices.len() as u32,
            instance_buffer: None,
            instance_buffer_memory: None,
        };
        
        let textures = TextureResources {
            image: texture_image,
            image_memory: texture_image_memory,
            image_view: texture_image_view,
            sampler: texture_sampler,
            descriptor_pool,
            descriptor_set_layout,
            descriptor_sets,
        };
        
        let mut pipelines = std::collections::HashMap::new();
        pipelines.insert("default".to_string(), Pipeline {
            pipeline: graphics_pipeline,
            layout: pipeline_layout,
        });
        
        let memory_pool = MemoryPoolManager::new(core.device.clone());
        
        Ok(Self {
            core,
            pipeline_layout,
            graphics_pipeline,
            pipelines,
            current_pipeline: "default".to_string(),
            vertex_count: 0,
            instance_count,
            has_depth: true,
            buffers: Some(buffers),
            textures: Some(textures),
            texture_arrays: None,
            skinned_mesh: None,
            meshes: Vec::new(),
            memory_pool,
            egui_integration: None,
            water_push_constants: None,
            textured_pipelines: std::collections::HashMap::new(),
        })
    }
    
    pub fn new_textured_with_winding<T: Copy>(
        window_handle: &RawHandleWrapperHolder,
        vert_shader_path: &str,
        frag_shader_path: &str,
        vertices: &[T],
        indices: &[u32],
        binding_descriptions: Vec<vk::VertexInputBindingDescription>,
        attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
        texture_path: &str,
        instance_count: u32,
        front_face: Option<vk::FrontFace>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let core = VulkanCore::new(window_handle, true)?;
        
        // Create buffers
        let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            vertices,
        )?;
        
        let (index_buffer, index_buffer_memory) = create_index_buffer(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            indices,
        )?;
        
        // Create texture resources
        let (texture_image, texture_image_memory) = crate::vulkan_common::create_texture_image(
            &core.instance,
            &core.device,
            core.physical_device,
            core.command_pool,
            core.graphics_queue,
            texture_path,
        )?;
        
        let texture_image_view = crate::vulkan_common::create_texture_image_view(&core.device, texture_image)?;
        let texture_sampler = crate::vulkan_common::create_texture_sampler(&core.instance, &core.device, core.physical_device)?;
        
        // Create descriptor resources
        let binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);
        
        let descriptor_set_layout = create_descriptor_set_layout(&core.device, &[binding])?;
        
        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(core.swapchain_images.len() as u32);
        
        let descriptor_pool = create_descriptor_pool(&core.device, core.swapchain_images.len() as u32, &[pool_size])?;
        
        let layouts = vec![descriptor_set_layout; core.swapchain_images.len()];
        let descriptor_sets = allocate_descriptor_sets(&core.device, descriptor_pool, &layouts)?;
        
        // Update descriptor sets
        for &descriptor_set in &descriptor_sets {
            update_descriptor_sets_texture(&core.device, descriptor_set, texture_image_view, texture_sampler, 0);
        }
        
        // Configure pipeline
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(mem::size_of::<[f32; 16]>() as u32 * 3);
        
        let (graphics_pipeline, pipeline_layout) = PipelineBuilder::new(
            core.device.clone(),
            vert_shader_path,
            frag_shader_path,
            core.swapchain_extent,
            core.render_pass,
        )?
        .with_vertex_input(binding_descriptions, attribute_descriptions)
        .with_push_constants(vec![push_constant_range])
        .with_descriptor_sets(vec![descriptor_set_layout])
        .with_depth_test(true)
        .with_cull_mode(vk::CullModeFlags::BACK)
        .with_front_face(if let Some(face) = front_face { face } else { vk::FrontFace::COUNTER_CLOCKWISE })
        .build()?;
        
        let buffers = BufferResources {
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer: Some(index_buffer),
            index_buffer_memory: Some(index_buffer_memory),
            index_count: indices.len() as u32,
            instance_buffer: None,
            instance_buffer_memory: None,
        };
        
        let textures = TextureResources {
            image: texture_image,
            image_memory: texture_image_memory,
            image_view: texture_image_view,
            sampler: texture_sampler,
            descriptor_pool,
            descriptor_set_layout,
            descriptor_sets,
        };
        
        let mut pipelines = std::collections::HashMap::new();
        pipelines.insert("default".to_string(), Pipeline {
            pipeline: graphics_pipeline,
            layout: pipeline_layout,
        });
        
        let memory_pool = MemoryPoolManager::new(core.device.clone());
        
        Ok(Self {
            core,
            pipeline_layout,
            graphics_pipeline,
            pipelines,
            current_pipeline: "default".to_string(),
            vertex_count: 0,
            instance_count,
            has_depth: true,
            buffers: Some(buffers),
            textures: Some(textures),
            texture_arrays: None,
            skinned_mesh: None,
            meshes: Vec::new(),
            memory_pool,
            egui_integration: None,
            water_push_constants: None,
            textured_pipelines: std::collections::HashMap::new(),
        })
    }
    
    // Constructor for multi-mesh rendering
    pub fn new_multi_mesh(
        window_handle: &RawHandleWrapperHolder,
        vert_shader_path: &str,
        frag_shader_path: &str,
        meshes_data: Vec<(&MeshData, Vec<[f32; 3]>)>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let core = VulkanCore::new(window_handle, true)?;
        
        // Create mesh entries
        let mut meshes = Vec::new();
        for (mesh_idx, (mesh_data, positions)) in meshes_data.into_iter().enumerate() {
            let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(
                &core.instance,
                &core.device,
                core.physical_device,
                core.command_pool,
                core.graphics_queue,
                &mesh_data.vertices,
            )?;
            
            let (index_buffer, index_buffer_memory) = create_index_buffer(
                &core.instance,
                &core.device,
                core.physical_device,
                core.command_pool,
                core.graphics_queue,
                &mesh_data.indices,
            )?;
            
            let transforms = positions.iter()
                .map(|pos| Mat4::from_translation(Vec3::new(pos[0], pos[1], pos[2])))
                .collect();
            
            meshes.push(MeshEntry {
                vertex_buffer,
                vertex_buffer_memory: Some(vertex_buffer_memory),
                vertex_memory_block: None,
                index_buffer,
                index_buffer_memory: Some(index_buffer_memory),
                index_memory_block: None,
                index_count: mesh_data.indices.len() as u32,
                transforms,
                pipeline_name: None,
                texture_resources: None,
                instance_buffer: None,
                instance_buffer_memory: None,
                instance_memory_block: None,
                is_skinned: false,
                skinned_descriptor_pool: None,
                skinned_descriptor_set_layout: None,
                skinned_descriptor_sets: None,
                camera_uniform_buffer: None,
                camera_uniform_memory: None,
                instance_count: 0,
                use_instancing: false,
                base_color: [mesh_idx as f32, 0.0, 0.0, 1.0], // Store mesh index in first component
                joint_matrices: None,
                joint_buffer: None,
                joint_buffer_memory: None,
            });
        }
        
        // Configure push constants for MVP matrices and mesh ID
        // view (64) + proj (64) + model (64) + mesh_id_vec4 (16) = 208 bytes
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(208);
        
        let (graphics_pipeline, pipeline_layout) = PipelineBuilder::new(
            core.device.clone(),
            vert_shader_path,
            frag_shader_path,
            core.swapchain_extent,
            core.render_pass,
        )?
        .with_vertex_input(vec![Vertex::get_binding_description()], Vertex::get_attribute_descriptions())
        .with_push_constants(vec![push_constant_range])
        .with_depth_test(true)
        .with_cull_mode(vk::CullModeFlags::BACK)
        .with_front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .build()?;
        
        let mut pipelines = std::collections::HashMap::new();
        pipelines.insert("default".to_string(), Pipeline {
            pipeline: graphics_pipeline,
            layout: pipeline_layout,
        });
        
        let memory_pool = MemoryPoolManager::new(core.device.clone());
        
        Ok(Self {
            core,
            pipeline_layout,
            graphics_pipeline,
            pipelines,
            current_pipeline: "default".to_string(),
            vertex_count: 0,
            instance_count: 1,
            has_depth: true,
            buffers: None,
            textures: None,
            texture_arrays: None,
            skinned_mesh: None,
            meshes,
            memory_pool,
            egui_integration: None,
            water_push_constants: None,
            textured_pipelines: std::collections::HashMap::new(),
        })
    }
    
    // Add a new mesh to the renderer
    pub fn add_mesh(&mut self, mesh_data: &MeshData) -> Result<usize, Box<dyn std::error::Error>> {
        let (vertex_buffer, vertex_memory_block) = create_vertex_buffer_pooled(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            &mut self.memory_pool,
            &mesh_data.vertices,
        )?;
        
        let (index_buffer, index_memory_block) = create_index_buffer_pooled(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            &mut self.memory_pool,
            &mesh_data.indices,
        )?;
        
        let mesh_entry = MeshEntry {
            vertex_buffer,
            vertex_buffer_memory: None,
            vertex_memory_block: Some(vertex_memory_block),
            index_buffer,
            index_buffer_memory: None,
            index_memory_block: Some(index_memory_block),
            index_count: mesh_data.indices.len() as u32,
            transforms: Vec::new(),
            pipeline_name: None,
            texture_resources: None,
            instance_buffer: None,
            instance_buffer_memory: None,
            instance_memory_block: None,
            is_skinned: false,
            skinned_descriptor_pool: None,
            skinned_descriptor_set_layout: None,
            skinned_descriptor_sets: None,
            camera_uniform_buffer: None,
            camera_uniform_memory: None,
            instance_count: 0,
            use_instancing: false,
            base_color: [1.0, 1.0, 1.0, 1.0], // Default white
            joint_matrices: None,
            joint_buffer: None,
            joint_buffer_memory: None,
        };
        
        self.meshes.push(mesh_entry);
        Ok(self.meshes.len() - 1) // Return the index of the new mesh
    }
    
    // Add a skinned mesh with instancing to the multi-mesh system
    pub fn add_skinned_mesh_instanced(&mut self, 
        mesh_data: &SkinnedMeshData,
        instance_positions: &[[f32; 3]],
        pipeline_name: Option<String>,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        // Create vertex buffer for skinned mesh
        let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            &mesh_data.vertices,
        )?;
        
        // Create index buffer
        let (index_buffer, index_buffer_memory) = create_index_buffer(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            &mesh_data.indices,
        )?;
        
        // Create instance buffer
        let (instance_buffer, instance_buffer_memory) = create_buffer(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            (instance_positions.len() * std::mem::size_of::<[f32; 3]>()) as vk::DeviceSize,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Copy instance data to buffer
        unsafe {
            let data = self.core.device.map_memory(
                instance_buffer_memory,
                0,
                (instance_positions.len() * std::mem::size_of::<[f32; 3]>()) as vk::DeviceSize,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy_nonoverlapping(
                instance_positions.as_ptr() as *const u8,
                data as *mut u8,
                instance_positions.len() * std::mem::size_of::<[f32; 3]>(),
            );
            self.core.device.unmap_memory(instance_buffer_memory);
        }
        
        // Debug: Log what we're sending to GPU
        println!("Sending {} instance positions to GPU for skinned mesh:", instance_positions.len());
        for (i, pos) in instance_positions.iter().take(3).enumerate() {
            println!("  GPU Instance {}: [{:.2}, {:.2}, {:.2}]", i, pos[0], pos[1], pos[2]);
        }
        
        // Create joint buffer for skinned animation
        let joint_buffer_size = (std::mem::size_of::<Mat4>() * 128) as vk::DeviceSize; // Support up to 128 joints
        let (joint_buffer, joint_buffer_memory) = create_buffer(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            joint_buffer_size,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Copy initial joint matrices to buffer
        unsafe {
            let data = self.core.device.map_memory(
                joint_buffer_memory,
                0,
                joint_buffer_size,
                vk::MemoryMapFlags::empty(),
            )? as *mut Mat4;
            
            for (i, mat) in mesh_data.joint_matrices.iter().enumerate() {
                if i >= 128 { break; }
                data.add(i).write(*mat);
            }
            
            self.core.device.unmap_memory(joint_buffer_memory);
        }
        
        // Create camera uniform buffer for skinned mesh
        let camera_buffer_size = (std::mem::size_of::<Mat4>() * 2) as vk::DeviceSize; // view + proj matrices
        let (camera_uniform_buffer, camera_uniform_memory) = create_buffer(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            camera_buffer_size,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Create descriptor pool for skinned mesh
        let pool_sizes = vec![
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: self.core.swapchain_images.len() as u32 * 2, // joints + camera
            },
        ];
        
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(self.core.swapchain_images.len() as u32);
        
        let descriptor_pool = unsafe {
            self.core.device.create_descriptor_pool(&pool_info, None)?
        };
        
        // Create descriptor set layout for skinned mesh
        let bindings = vec![
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
        ];
        
        let descriptor_set_layout = create_descriptor_set_layout(&self.core.device, &bindings)?;
        
        // Create descriptor sets
        let layouts = vec![descriptor_set_layout; self.core.swapchain_images.len()];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);
        
        let descriptor_sets = unsafe {
            self.core.device.allocate_descriptor_sets(&alloc_info)?
        };
        
        // Update descriptor sets with buffer info
        for set in &descriptor_sets {
            let joint_buffer_info = vk::DescriptorBufferInfo::default()
                .buffer(joint_buffer)
                .offset(0)
                .range(joint_buffer_size);
            
            let camera_buffer_info = vk::DescriptorBufferInfo::default()
                .buffer(camera_uniform_buffer)
                .offset(0)
                .range(camera_buffer_size);
            
            let descriptor_writes = vec![
                vk::WriteDescriptorSet::default()
                    .dst_set(*set)
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(std::slice::from_ref(&joint_buffer_info)),
                vk::WriteDescriptorSet::default()
                    .dst_set(*set)
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(std::slice::from_ref(&camera_buffer_info)),
            ];
            
            unsafe {
                self.core.device.update_descriptor_sets(&descriptor_writes, &[]);
            }
        }
        
        let mesh_entry = MeshEntry {
            vertex_buffer,
            vertex_buffer_memory: Some(vertex_buffer_memory),
            vertex_memory_block: None,
            index_buffer,
            index_buffer_memory: Some(index_buffer_memory),
            index_memory_block: None,
            index_count: mesh_data.indices.len() as u32,
            transforms: Vec::new(), // Not used when instancing
            pipeline_name,
            texture_resources: None,
            instance_buffer: Some(instance_buffer),
            instance_buffer_memory: Some(instance_buffer_memory),
            instance_memory_block: None,
            instance_count: instance_positions.len() as u32,
            use_instancing: true,
            base_color: [1.0, 1.0, 1.0, 1.0],
            joint_matrices: Some(mesh_data.joint_matrices.clone()),
            joint_buffer: Some(joint_buffer),
            joint_buffer_memory: Some(joint_buffer_memory),
            is_skinned: true,
            skinned_descriptor_pool: Some(descriptor_pool),
            skinned_descriptor_set_layout: Some(descriptor_set_layout),
            skinned_descriptor_sets: Some(descriptor_sets),
            camera_uniform_buffer: Some(camera_uniform_buffer),
            camera_uniform_memory: Some(camera_uniform_memory),
        };
        
        let mesh_index = self.meshes.len();
        self.meshes.push(mesh_entry);
        println!("Added skinned mesh at index {} with is_skinned=true, instance_count={}", 
                 mesh_index, instance_positions.len());
        Ok(mesh_index)
    }
    
    // Update joint matrices for a specific skinned mesh
    pub fn update_mesh_joint_matrices(&mut self, mesh_index: usize, joint_matrices: &[Mat4]) {
        if mesh_index >= self.meshes.len() {
            return;
        }
        
        let mesh = &mut self.meshes[mesh_index];
        if !mesh.is_skinned || mesh.joint_buffer.is_none() || mesh.joint_buffer_memory.is_none() {
            return;
        }
        
        // Update the stored matrices
        if let Some(ref mut stored_matrices) = mesh.joint_matrices {
            for (i, mat) in joint_matrices.iter().enumerate() {
                if i < stored_matrices.len() {
                    stored_matrices[i] = *mat;
                }
            }
        }
        
        // Update the GPU buffer
        if let (Some(buffer_memory), Some(_buffer)) = (mesh.joint_buffer_memory, mesh.joint_buffer) {
            unsafe {
                let joint_buffer_size = (std::mem::size_of::<Mat4>() * 128) as vk::DeviceSize;
                if let Ok(data) = self.core.device.map_memory(
                    buffer_memory,
                    0,
                    joint_buffer_size,
                    vk::MemoryMapFlags::empty(),
                ) {
                    let data_ptr = data as *mut Mat4;
                    for (i, mat) in joint_matrices.iter().enumerate() {
                        if i >= 128 { break; }
                        data_ptr.add(i).write(*mat);
                    }
                    self.core.device.unmap_memory(buffer_memory);
                }
            }
        }
    }
    
    pub fn replace_mesh(&mut self, mesh_index: usize, mesh_data: &MeshData) -> Result<(), Box<dyn std::error::Error>> {
        if mesh_index >= self.meshes.len() {
            return Err(format!("Mesh index {} out of bounds", mesh_index).into());
        }
        
        // Take the old mesh entry to move its resources
        let old_mesh = std::mem::replace(&mut self.meshes[mesh_index], MeshEntry {
            vertex_buffer: vk::Buffer::null(),
            vertex_buffer_memory: None,
            vertex_memory_block: None,
            index_buffer: vk::Buffer::null(),
            index_buffer_memory: None,
            index_memory_block: None,
            index_count: 0,
            transforms: Vec::new(),
            pipeline_name: None,
            texture_resources: None,
            instance_buffer: None,
            instance_buffer_memory: None,
            instance_memory_block: None,
            instance_count: 0,
            use_instancing: false,
            base_color: [1.0, 1.0, 1.0, 1.0],
            joint_matrices: None,
            joint_buffer: None,
            joint_buffer_memory: None,
            is_skinned: false,
            skinned_descriptor_pool: None,
            skinned_descriptor_set_layout: None,
            skinned_descriptor_sets: None,
            camera_uniform_buffer: None,
            camera_uniform_memory: None,
        });
        
        unsafe {
            // Wait for GPU to finish using the old buffers
            self.core.device.device_wait_idle().map_err(|e| format!("Failed to wait for device idle: {:?}", e))?;
            
            // Destroy old vertex and index buffers
            self.core.device.destroy_buffer(old_mesh.vertex_buffer, None);
            
            // Free memory - check if using memory pool or direct allocation
            if let Some(memory) = old_mesh.vertex_buffer_memory {
                self.core.device.free_memory(memory, None);
            } else if let Some(block) = old_mesh.vertex_memory_block {
                self.memory_pool.free_buffer(block);
            }
            
            self.core.device.destroy_buffer(old_mesh.index_buffer, None);
            
            if let Some(memory) = old_mesh.index_buffer_memory {
                self.core.device.free_memory(memory, None);
            } else if let Some(block) = old_mesh.index_memory_block {
                self.memory_pool.free_buffer(block);
            }
        }
        
        // Create new buffers
        let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            &mesh_data.vertices,
        )?;
        
        let (index_buffer, index_buffer_memory) = create_index_buffer(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            &mesh_data.indices,
        )?;
        
        // Replace the mesh entry with preserved properties
        self.meshes[mesh_index] = MeshEntry {
            vertex_buffer,
            vertex_buffer_memory: Some(vertex_buffer_memory),
            vertex_memory_block: None,
            index_buffer,
            index_buffer_memory: Some(index_buffer_memory),
            index_memory_block: None,
            index_count: mesh_data.indices.len() as u32,
            transforms: old_mesh.transforms,
            pipeline_name: old_mesh.pipeline_name,
            texture_resources: old_mesh.texture_resources,
            instance_buffer: old_mesh.instance_buffer,
            instance_buffer_memory: old_mesh.instance_buffer_memory,
            instance_memory_block: old_mesh.instance_memory_block,
            instance_count: old_mesh.instance_count,
            use_instancing: old_mesh.use_instancing,
            base_color: old_mesh.base_color,
            joint_matrices: old_mesh.joint_matrices,
            joint_buffer: old_mesh.joint_buffer,
            joint_buffer_memory: old_mesh.joint_buffer_memory,
            is_skinned: old_mesh.is_skinned,
            skinned_descriptor_pool: old_mesh.skinned_descriptor_pool,
            skinned_descriptor_set_layout: old_mesh.skinned_descriptor_set_layout,
            skinned_descriptor_sets: old_mesh.skinned_descriptor_sets,
            camera_uniform_buffer: old_mesh.camera_uniform_buffer,
            camera_uniform_memory: old_mesh.camera_uniform_memory,
        };
        
        println!("Replaced mesh at index {} with {} vertices and {} indices", 
                 mesh_index, mesh_data.vertices.len(), mesh_data.indices.len());
        
        Ok(())
    }
    
    // Add mesh with GPU instancing support
    pub fn add_mesh_instanced(
        &mut self, 
        mesh_data: &MeshData, 
        instance_positions: Vec<[f32; 3]>,
        texture_path: Option<String>,
        pipeline_name: Option<String>,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        // Create vertex buffer using memory pool
        let (vertex_buffer, vertex_memory_block) = create_vertex_buffer_pooled(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            &mut self.memory_pool,
            &mesh_data.vertices,
        )?;
        
        // Create index buffer using memory pool
        let (index_buffer, index_memory_block) = create_index_buffer_pooled(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            &mut self.memory_pool,
            &mesh_data.indices,
        )?;
        
        // Create instance buffer using memory pool
        let instance_count = instance_positions.len() as u32;
        let (instance_buffer, instance_memory_block) = create_buffer_pooled(
            &self.core.device,
            self.core.physical_device,
            &self.core.instance,
            &mut self.memory_pool,
            (instance_positions.len() * std::mem::size_of::<[f32; 3]>()) as vk::DeviceSize,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Copy instance data to buffer
        unsafe {
            let data = self.core.device.map_memory(
                instance_memory_block.memory,
                instance_memory_block.offset,
                instance_memory_block.size,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy_nonoverlapping(
                instance_positions.as_ptr() as *const u8,
                data as *mut u8,
                instance_positions.len() * std::mem::size_of::<[f32; 3]>(),
            );
            self.core.device.unmap_memory(instance_memory_block.memory);
        }
        
        // Create texture resources if path provided
        let texture_resources = if let Some(_path) = texture_path {
            // Load and create texture (simplified - you may want to add proper error handling)
            None // TODO: Implement texture loading
        } else {
            None
        };
        
        let mesh_entry = MeshEntry {
            vertex_buffer,
            vertex_buffer_memory: None,
            vertex_memory_block: Some(vertex_memory_block),
            index_buffer,
            index_buffer_memory: None,
            index_memory_block: Some(index_memory_block),
            index_count: mesh_data.indices.len() as u32,
            transforms: Vec::new(),
            pipeline_name,
            texture_resources,
            instance_buffer: Some(instance_buffer),
            instance_buffer_memory: None,
            instance_memory_block: Some(instance_memory_block),
            instance_count,
            use_instancing: true,
            base_color: [1.0, 1.0, 1.0, 1.0], // Default white
            joint_matrices: None,
            joint_buffer: None,
            joint_buffer_memory: None,
            is_skinned: false,
            skinned_descriptor_pool: None,
            skinned_descriptor_set_layout: None,
            skinned_descriptor_sets: None,
            camera_uniform_buffer: None,
            camera_uniform_memory: None,
        };
        
        self.meshes.push(mesh_entry);
        Ok(self.meshes.len() - 1)
    }
    
    // Update instance buffer for a specific mesh
    pub fn update_mesh_instance_buffer(
        &mut self, 
        mesh_index: usize, 
        instance_positions: Vec<[f32; 3]>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if mesh_index >= self.meshes.len() {
            return Err("Invalid mesh index".into());
        }
        
        let mesh = &mut self.meshes[mesh_index];
        
        // Update instance count
        mesh.instance_count = instance_positions.len() as u32;
        
        // Update instance buffer data
        if let Some(instance_buffer_memory) = mesh.instance_buffer_memory {
            unsafe {
                let data = self.core.device.map_memory(
                    instance_buffer_memory,
                    0,
                    (instance_positions.len() * std::mem::size_of::<[f32; 3]>()) as vk::DeviceSize,
                    vk::MemoryMapFlags::empty(),
                )?;
                std::ptr::copy_nonoverlapping(
                    instance_positions.as_ptr() as *const u8,
                    data as *mut u8,
                    instance_positions.len() * std::mem::size_of::<[f32; 3]>(),
                );
                self.core.device.unmap_memory(instance_buffer_memory);
            }
        }
        
        Ok(())
    }
    
    // Update transforms for a specific mesh
    pub fn update_mesh_transforms(&mut self, mesh_index: usize, transforms: Vec<Mat4>) {
        if mesh_index < self.meshes.len() {
            self.meshes[mesh_index].transforms = transforms;
        }
    }
    
    // Remove a mesh from the renderer and free its resources
    pub fn remove_mesh(&mut self, mesh_index: usize) {
        if mesh_index >= self.meshes.len() {
            return;
        }
        
        let mesh = &self.meshes[mesh_index];
        
        unsafe {
            // Destroy buffers
            self.core.device.destroy_buffer(mesh.vertex_buffer, None);
            self.core.device.destroy_buffer(mesh.index_buffer, None);
            
            // Free memory if not using memory pool
            if let Some(vertex_memory) = mesh.vertex_buffer_memory {
                self.core.device.free_memory(vertex_memory, None);
            }
            if let Some(index_memory) = mesh.index_buffer_memory {
                self.core.device.free_memory(index_memory, None);
            }
            
            // Free memory pool blocks if using memory pool
            if let Some(vertex_block) = &mesh.vertex_memory_block {
                self.memory_pool.free_buffer(vertex_block.clone());
            }
            if let Some(index_block) = &mesh.index_memory_block {
                self.memory_pool.free_buffer(index_block.clone());
            }
            
            // Destroy instance buffer if present
            if let Some(instance_buffer) = mesh.instance_buffer {
                self.core.device.destroy_buffer(instance_buffer, None);
            }
            if let Some(instance_memory) = mesh.instance_buffer_memory {
                self.core.device.free_memory(instance_memory, None);
            }
            if let Some(instance_block) = &mesh.instance_memory_block {
                self.memory_pool.free_buffer(instance_block.clone());
            }
            
            // Clean up texture resources if present
            if let Some(texture_resources) = &mesh.texture_resources {
                self.core.device.destroy_image_view(texture_resources.image_view, None);
                self.core.device.destroy_image(texture_resources.image, None);
                self.core.device.free_memory(texture_resources.image_memory, None);
                self.core.device.destroy_sampler(texture_resources.sampler, None);
                self.core.device.destroy_descriptor_pool(texture_resources.descriptor_pool, None);
                self.core.device.destroy_descriptor_set_layout(texture_resources.descriptor_set_layout, None);
            }
            
            // Clean up skinned mesh resources if present
            if let Some(joint_buffer) = mesh.joint_buffer {
                self.core.device.destroy_buffer(joint_buffer, None);
            }
            if let Some(joint_memory) = mesh.joint_buffer_memory {
                self.core.device.free_memory(joint_memory, None);
            }
            
            // Clean up descriptor sets for skinned meshes
            if let Some(descriptor_pool) = mesh.skinned_descriptor_pool {
                self.core.device.destroy_descriptor_pool(descriptor_pool, None);
            }
            if let Some(descriptor_set_layout) = mesh.skinned_descriptor_set_layout {
                self.core.device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            }
            if let Some(camera_buffer) = mesh.camera_uniform_buffer {
                self.core.device.destroy_buffer(camera_buffer, None);
            }
            if let Some(camera_memory) = mesh.camera_uniform_memory {
                self.core.device.free_memory(camera_memory, None);
            }
        }
        
        // Mark the mesh slot as invalid by clearing it
        // We don't actually remove from the vector to preserve indices
        // Instead, we'll mark it as invalid by setting vertex count to 0
        self.meshes[mesh_index] = MeshEntry {
            vertex_buffer: vk::Buffer::null(),
            vertex_buffer_memory: None,
            vertex_memory_block: None,
            index_buffer: vk::Buffer::null(),
            index_buffer_memory: None,
            index_memory_block: None,
            index_count: 0,
            transforms: Vec::new(),
            pipeline_name: None,
            texture_resources: None,
            instance_buffer: None,
            instance_buffer_memory: None,
            instance_memory_block: None,
            instance_count: 0,
            use_instancing: false,
            base_color: [1.0, 1.0, 1.0, 1.0],
            is_skinned: false,
            joint_matrices: None,
            joint_buffer: None,
            joint_buffer_memory: None,
            skinned_descriptor_pool: None,
            skinned_descriptor_set_layout: None,
            skinned_descriptor_sets: None,
            camera_uniform_buffer: None,
            camera_uniform_memory: None,
        };
    }
    
    // Update joint matrices for a specific skinned mesh
    pub fn update_skinned_mesh_joints(&mut self, mesh_index: usize, joint_matrices: Vec<Mat4>) -> Result<(), Box<dyn std::error::Error>> {
        if mesh_index >= self.meshes.len() {
            return Err("Invalid mesh index".into());
        }
        
        let mesh = &mut self.meshes[mesh_index];
        
        // Store the joint matrices
        mesh.joint_matrices = Some(joint_matrices.clone());
        mesh.is_skinned = true;
        
        // Create or update the joint buffer if needed
        if mesh.joint_buffer.is_none() {
            // Create joint buffer for this mesh
            let buffer_size = (std::mem::size_of::<Mat4>() * 128) as vk::DeviceSize;
            
            let (joint_buffer, joint_buffer_memory) = crate::vulkan_common::create_buffer(
                &self.core.instance,
                &self.core.device,
                self.core.physical_device,
                buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            
            mesh.joint_buffer = Some(joint_buffer);
            mesh.joint_buffer_memory = Some(joint_buffer_memory);
        }
        
        // Update the joint buffer with new matrices
        if let Some(joint_buffer_memory) = mesh.joint_buffer_memory {
            unsafe {
                let data = self.core.device.map_memory(
                    joint_buffer_memory,
                    0,
                    (std::mem::size_of::<Mat4>() * 128) as vk::DeviceSize,
                    vk::MemoryMapFlags::empty(),
                )? as *mut Mat4;
                
                // Copy joint matrices to buffer
                for (i, mat) in joint_matrices.iter().enumerate() {
                    if i >= 128 { break; }
                    data.add(i).write(*mat);
                }
                
                self.core.device.unmap_memory(joint_buffer_memory);
            }
        }
        
        Ok(())
    }
    
    pub fn set_mesh_pipeline(&mut self, mesh_index: usize, pipeline_name: &str) {
        if mesh_index < self.meshes.len() {
            self.meshes[mesh_index].pipeline_name = Some(pipeline_name.to_string());
        }
    }
    
    // Set water push constants for fluid rendering
    pub fn set_water_push_constants(&mut self, push_constants: PushConstants) {
        self.water_push_constants = Some(push_constants);
    }
    
    pub fn set_mesh_color(&mut self, mesh_index: usize, color: [f32; 4]) {
        if mesh_index < self.meshes.len() {
            self.meshes[mesh_index].base_color = color;
        }
    }
    
    // Add texture to a specific mesh from a file path
    pub fn set_mesh_texture_from_file(&mut self, mesh_index: usize, texture_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if mesh_index >= self.meshes.len() {
            return Err("Invalid mesh index".into());
        }
        
        // Create texture resources
        let (texture_image, texture_image_memory) = crate::vulkan_common::create_texture_image(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            texture_path,
        )?;
        
        let texture_image_view = crate::vulkan_common::create_texture_image_view(&self.core.device, texture_image)?;
        let texture_sampler = crate::vulkan_common::create_texture_sampler(&self.core.instance, &self.core.device, self.core.physical_device)?;
        
        // Create descriptor resources
        let binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);
        
        let descriptor_set_layout = create_descriptor_set_layout(&self.core.device, &[binding])?;
        
        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(self.core.swapchain_images.len() as u32);
        
        let descriptor_pool = create_descriptor_pool(&self.core.device, self.core.swapchain_images.len() as u32, &[pool_size])?;
        
        let layouts = vec![descriptor_set_layout; self.core.swapchain_images.len()];
        let descriptor_sets = allocate_descriptor_sets(&self.core.device, descriptor_pool, &layouts)?;
        
        // Update descriptor sets
        for &descriptor_set in &descriptor_sets {
            update_descriptor_sets_texture(&self.core.device, descriptor_set, texture_image_view, texture_sampler, 0);
        }
        
        let textures = TextureResources {
            image: texture_image,
            image_memory: texture_image_memory,
            image_view: texture_image_view,
            sampler: texture_sampler,
            descriptor_pool,
            descriptor_set_layout,
            descriptor_sets,
        };
        
        self.meshes[mesh_index].texture_resources = Some(textures);
        Ok(())
    }
    
    // Update instance positions for a specific mesh (convenience method)
    pub fn update_mesh_instances(&mut self, mesh_index: usize, positions: Vec<[f32; 3]>) {
        if mesh_index < self.meshes.len() {
            self.meshes[mesh_index].transforms = positions.iter()
                .map(|pos| Mat4::from_translation(Vec3::new(pos[0], pos[1], pos[2])))
                .collect();
        } else {
            println!("ERROR: mesh_index {} out of bounds (meshes.len = {})", mesh_index, self.meshes.len());
        }
    }
    
    // Get the total number of meshes in the renderer
    pub fn get_mesh_count(&self) -> usize {
        self.meshes.len()
    }
    
    // Update mesh vertices dynamically (for fluid simulation)
    pub fn update_mesh_vertices(&mut self, mesh_index: usize, _new_positions: &[[f32; 3]]) {
        if mesh_index >= self.meshes.len() {
            eprintln!("ERROR: mesh_index {} out of bounds (meshes.len = {})", mesh_index, self.meshes.len());
            return;
        }
        
        // TODO: Implement proper vertex buffer update
        // For now, this is a placeholder - we need to:
        // 1. Store the original vertex data in MeshEntry
        // 2. Update the vertex buffer on the GPU
        // 3. Handle synchronization properly
        
        // This would require significant changes to MeshEntry structure
        // to store vertex data and handle dynamic updates
    }
    
    pub fn update_mesh_vertices_full(&mut self, mesh_index: usize, new_vertices: &[Vertex]) {
        if mesh_index >= self.meshes.len() {
            eprintln!("ERROR: mesh_index {} out of bounds (meshes.len = {})", mesh_index, self.meshes.len());
            return;
        }
        
        let mesh = &mut self.meshes[mesh_index];
        let vertex_data = bytemuck::cast_slice(new_vertices);
        let vertex_size = vertex_data.len() as u64;
        
        // Get a reusable staging buffer from the memory pool
        let staging_buffer = self.memory_pool.get_staging_buffer(
            &self.core.instance,
            self.core.physical_device,
            vertex_size
        ).expect("Failed to get staging buffer");
        
        unsafe {
            // Map the staging buffer and copy vertex data
            let ptr = self.core.device
                .map_memory(staging_buffer.1, 0, vertex_size, vk::MemoryMapFlags::empty())
                .expect("Failed to map staging buffer memory");
            
            std::ptr::copy_nonoverlapping(vertex_data.as_ptr(), ptr as *mut u8, vertex_data.len());
            
            self.core.device.unmap_memory(staging_buffer.1);
            
            // Copy from staging buffer to vertex buffer using command pool
            let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(self.core.command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            
            let command_buffers = self.core.device
                .allocate_command_buffers(&command_buffer_alloc_info)
                .expect("Failed to allocate command buffer");
            let command_buffer = command_buffers[0];
            
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            
            self.core.device
                .begin_command_buffer(command_buffer, &begin_info)
                .expect("Failed to begin command buffer");
            
            let copy_region = vk::BufferCopy::default()
                .src_offset(0)
                .dst_offset(0)
                .size(vertex_size);
            
            self.core.device.cmd_copy_buffer(
                command_buffer,
                staging_buffer.0,
                mesh.vertex_buffer,
                &[copy_region],
            );
            
            self.core.device
                .end_command_buffer(command_buffer)
                .expect("Failed to end command buffer");
            
            let command_buffers_to_submit = [command_buffer];
            let submit_info = vk::SubmitInfo::default()
                .command_buffers(&command_buffers_to_submit);
            
            self.core.device
                .queue_submit(self.core.graphics_queue, &[submit_info], vk::Fence::null())
                .expect("Failed to submit command buffer");
            
            self.core.device.queue_wait_idle(self.core.graphics_queue)
                .expect("Failed to wait for queue");
            
            self.core.device.free_command_buffers(self.core.command_pool, &[command_buffer]);
        }
    }
    
    // Add a new pipeline with a given name
    pub fn get_memory_stats(&self) -> String {
        self.memory_pool.get_stats()
    }
    
    pub fn add_pipeline(&mut self, name: &str, vert_shader_path: &str, frag_shader_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.add_pipeline_with_texture(name, vert_shader_path, frag_shader_path, false)
    }
    
    // Add a new pipeline with optional texture support
    pub fn add_pipeline_with_texture(&mut self, name: &str, vert_shader_path: &str, frag_shader_path: &str, has_texture: bool) -> Result<(), Box<dyn std::error::Error>> {
        // Use default COUNTER_CLOCKWISE for compatibility
        self.add_pipeline_with_texture_and_winding(name, vert_shader_path, frag_shader_path, has_texture, vk::FrontFace::COUNTER_CLOCKWISE)
    }
    
    // Add a new pipeline with optional texture support and custom winding order
    pub fn add_pipeline_with_texture_and_winding(&mut self, name: &str, vert_shader_path: &str, frag_shader_path: &str, has_texture: bool, front_face: vk::FrontFace) -> Result<(), Box<dyn std::error::Error>> {
        // Configure push constants for MVP matrices
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(208); // view (64) + proj (64) + model (64) + base_color (16)
        
        // Create descriptor set layout for texture if needed
        let descriptor_set_layout = if has_texture {
            let binding = vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT);
            
            Some(create_descriptor_set_layout(&self.core.device, &[binding])?)
        } else {
            None
        };
        
        let mut builder = PipelineBuilder::new(
            self.core.device.clone(),
            vert_shader_path,
            frag_shader_path,
            self.core.swapchain_extent,
            self.core.render_pass,
        )?
        .with_vertex_input(vec![Vertex::get_binding_description()], Vertex::get_attribute_descriptions())
        .with_push_constants(vec![push_constant_range])
        .with_depth_test(self.has_depth)
        .with_cull_mode(vk::CullModeFlags::BACK)
        .with_front_face(front_face);
        
        // Add descriptor set layout if we have texture
        if let Some(layout) = descriptor_set_layout {
            builder = builder.with_descriptor_sets(vec![layout]);
        }
        
        let (graphics_pipeline, pipeline_layout) = builder.build()?;
        
        self.pipelines.insert(name.to_string(), Pipeline {
            pipeline: graphics_pipeline,
            layout: pipeline_layout,
        });
        
        Ok(())
    }
    
    // Add a skinned mesh pipeline (single instance)
    pub fn add_skinned_mesh_pipeline(
        &mut self, 
        name: &str, 
        vert_shader_path: &str, 
        frag_shader_path: &str,
        mesh_data: &SkinnedMeshData,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Store the skinned mesh data
        self.setup_skinned_mesh_resources(mesh_data, None)?;
        
        // Create pipeline for skinned rendering
        self.create_skinned_pipeline(name, vert_shader_path, frag_shader_path, false)?;
        
        Ok(())
    }
    
    // Add a skinned mesh pipeline with instancing
    pub fn add_skinned_mesh_pipeline_instanced(
        &mut self, 
        name: &str, 
        vert_shader_path: &str, 
        frag_shader_path: &str,
        mesh_data: &SkinnedMeshData,
        instance_positions: &[[f32; 3]],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Store the skinned mesh data with instancing
        self.setup_skinned_mesh_resources(mesh_data, Some(instance_positions))?;
        
        // Create pipeline for instanced skinned rendering
        self.create_skinned_pipeline(name, vert_shader_path, frag_shader_path, true)?;
        
        Ok(())
    }
    
    // Add a fluid rendering pipeline with custom push constants
    pub fn add_fluid_pipeline(
        &mut self,
        name: &str,
        vert_shader_path: &str,
        frag_shader_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        
        // Configure push constants for fluid rendering
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<PushConstants>() as u32);
        
        // Build the pipeline with fluid-specific configuration
        let mut builder = PipelineBuilder::new(
            self.core.device.clone(),
            vert_shader_path,
            frag_shader_path,
            self.core.swapchain_extent,
            self.core.render_pass,
        )?;
        
        // Configure vertex input for basic water/wall meshes
        let binding_description = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX);
        
        let attribute_descriptions = vec![
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, position) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, normal) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Vertex, uv) as u32),
        ];
        
        builder = builder
            .with_vertex_input(vec![binding_description], attribute_descriptions)
            .with_push_constants(vec![push_constant_range])
            .with_depth_test(true)
            .with_cull_mode(vk::CullModeFlags::NONE) // No culling for water
            .with_alpha_blending(name == "water"); // Enable blending for water pipeline
        
        let (pipeline, layout) = builder.build()?;
        
        // Store the pipeline
        self.pipelines.insert(
            name.to_string(),
            Pipeline {
                pipeline,
                layout,
            },
        );
        
        Ok(())
    }
    
    // Add a wall pipeline with stone wall textures
    pub fn add_wall_pipeline_with_textures(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        
        // Load the stone wall textures
        println!("Loading wall textures...");
        let wall_base_color = Texture::from_file(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            "assets/Stone Wall/Stone_Wall_basecolor.jpg",
        )?;
        
        let wall_normal = Texture::from_file(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            "assets/Stone Wall/Stone_Wall_normal.jpg",
        )?;
        
        let wall_roughness = Texture::from_file(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            "assets/Stone Wall/Stone_Wall_roughness.jpg",
        )?;
        
        let wall_ao = Texture::from_file(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            "assets/Stone Wall/Stone_Wall_ambientOcclusion.jpg",
        )?;
        
        let sampler = crate::texture::create_texture_sampler(&self.core.device)?;
        
        let textures = vec![&wall_base_color, &wall_normal, &wall_roughness, &wall_ao];
        
        self.add_fluid_pipeline_with_textures(
            "wall",
            "shaders/wall.vert.spv",
            "shaders/wall.frag.spv",
            textures,
            sampler,
        )
    }
    
    // Add a fluid rendering pipeline with support for multiple textures (for wall rendering)
    fn add_fluid_pipeline_with_textures(
        &mut self,
        name: &str,
        vert_shader_path: &str,
        frag_shader_path: &str,
        textures: Vec<&Texture>,
        sampler: vk::Sampler,
    ) -> Result<(), Box<dyn std::error::Error>> {
        
        // Configure push constants for fluid rendering
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<PushConstants>() as u32);
        
        // Create descriptor set layout for textures
        let mut bindings = Vec::new();
        for i in 0..textures.len() {
            bindings.push(
                vk::DescriptorSetLayoutBinding::default()
                    .binding(i as u32)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            );
        }
        
        let descriptor_set_layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&bindings);
        
        let descriptor_set_layout = unsafe {
            self.core.device.create_descriptor_set_layout(&descriptor_set_layout_info, None)?
        };
        
        // Create descriptor pool
        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count((self.core.swapchain_images.len() * textures.len()) as u32);
        
        let pool_sizes = [pool_size];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(self.core.swapchain_images.len() as u32);
        
        let descriptor_pool = unsafe {
            self.core.device.create_descriptor_pool(&pool_info, None)?
        };
        
        // Create descriptor sets
        let layouts = vec![descriptor_set_layout; self.core.swapchain_images.len()];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);
        
        let descriptor_sets = unsafe {
            self.core.device.allocate_descriptor_sets(&alloc_info)?
        };
        
        // Update descriptor sets with texture bindings
        for &descriptor_set in &descriptor_sets {
            let mut image_infos = Vec::new();
            for texture in &textures {
                image_infos.push(
                    vk::DescriptorImageInfo::default()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(texture.view)
                        .sampler(sampler)
                );
            }
            
            let mut writes = Vec::new();
            for (i, image_info) in image_infos.iter().enumerate() {
                writes.push(
                    vk::WriteDescriptorSet::default()
                        .dst_set(descriptor_set)
                        .dst_binding(i as u32)
                        .dst_array_element(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(std::slice::from_ref(image_info))
                );
            }
            
            unsafe {
                self.core.device.update_descriptor_sets(&writes, &[]);
            }
        }
        
        // Build the pipeline with fluid-specific configuration
        let mut builder = PipelineBuilder::new(
            self.core.device.clone(),
            vert_shader_path,
            frag_shader_path,
            self.core.swapchain_extent,
            self.core.render_pass,
        )?;
        
        // Configure vertex input for basic water/wall meshes
        let binding_description = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX);
        
        let attribute_descriptions = vec![
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, position) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, normal) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Vertex, uv) as u32),
        ];
        
        builder = builder
            .with_vertex_input(vec![binding_description], attribute_descriptions)
            .with_push_constants(vec![push_constant_range])
            .with_descriptor_sets(vec![descriptor_set_layout])
            .with_depth_test(true)
            .with_cull_mode(vk::CullModeFlags::NONE); // No culling for walls to see all sides
        
        let (pipeline, layout) = builder.build()?;
        
        // Store the pipeline with descriptor resources
        self.pipelines.insert(
            name.to_string(),
            Pipeline {
                pipeline,
                layout,
            },
        );
        
        // Store descriptor resources for cleanup
        self.textured_pipelines.insert(
            name.to_string(),
            TexturedPipelineResources {
                descriptor_set_layout,
                descriptor_pool,
                descriptor_sets,
            },
        );
        
        Ok(())
    }
    
    // Add a skinned pipeline with the correct descriptor set layout for joint and camera uniforms
    pub fn add_skinned_pipeline(
        &mut self,
        name: &str,
        vert_shader_path: &str,
        frag_shader_path: &str,
        use_instancing: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create descriptor set layout for skinned meshes
        // Binding 0: Joint matrices uniform buffer
        // Binding 1: Camera matrices uniform buffer
        let bindings = vec![
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
        ];
        
        let descriptor_set_layout = create_descriptor_set_layout(&self.core.device, &bindings)?;
        
        // Create pipeline with skinned vertex format
        let mut builder = PipelineBuilder::new(
            self.core.device.clone(),
            vert_shader_path,
            frag_shader_path,
            self.core.swapchain_extent,
            self.core.render_pass,
        )?;
        
        // Configure for skinned vertex format with instancing
        if use_instancing {
            builder = builder
                .with_vertex_input(
                    vec![
                        // Binding 0: Vertex data
                        vk::VertexInputBindingDescription::default()
                            .binding(0)
                            .stride(std::mem::size_of::<SkinnedVertex>() as u32)
                            .input_rate(vk::VertexInputRate::VERTEX),
                        // Binding 1: Instance data
                        vk::VertexInputBindingDescription::default()
                            .binding(1)
                            .stride(std::mem::size_of::<[f32; 3]>() as u32)
                            .input_rate(vk::VertexInputRate::INSTANCE),
                    ],
                    vec![
                        // Vertex attributes
                        vk::VertexInputAttributeDescription::default()
                            .binding(0)
                            .location(0)
                            .format(vk::Format::R32G32B32_SFLOAT)
                            .offset(offset_of!(SkinnedVertex, position) as u32),
                        vk::VertexInputAttributeDescription::default()
                            .binding(0)
                            .location(1)
                            .format(vk::Format::R32G32B32_SFLOAT)
                            .offset(offset_of!(SkinnedVertex, normal) as u32),
                        vk::VertexInputAttributeDescription::default()
                            .binding(0)
                            .location(2)
                            .format(vk::Format::R32G32_SFLOAT)
                            .offset(offset_of!(SkinnedVertex, uv) as u32),
                        vk::VertexInputAttributeDescription::default()
                            .binding(0)
                            .location(3)
                            .format(vk::Format::R32G32B32A32_SFLOAT)
                            .offset(offset_of!(SkinnedVertex, color) as u32),
                        vk::VertexInputAttributeDescription::default()
                            .binding(0)
                            .location(4)
                            .format(vk::Format::R32G32B32A32_UINT)
                            .offset(offset_of!(SkinnedVertex, joint_indices) as u32),
                        vk::VertexInputAttributeDescription::default()
                            .binding(0)
                            .location(5)
                            .format(vk::Format::R32G32B32A32_SFLOAT)
                            .offset(offset_of!(SkinnedVertex, joint_weights) as u32),
                        // Instance position
                        vk::VertexInputAttributeDescription::default()
                            .binding(1)
                            .location(6)
                            .format(vk::Format::R32G32B32_SFLOAT)
                            .offset(0),
                    ],
                );
        } else {
            builder = builder
                .with_vertex_input(
                    vec![
                        vk::VertexInputBindingDescription::default()
                            .binding(0)
                            .stride(std::mem::size_of::<SkinnedVertex>() as u32)
                            .input_rate(vk::VertexInputRate::VERTEX),
                    ],
                    vec![
                        vk::VertexInputAttributeDescription::default()
                            .binding(0)
                            .location(0)
                            .format(vk::Format::R32G32B32_SFLOAT)
                            .offset(offset_of!(SkinnedVertex, position) as u32),
                        vk::VertexInputAttributeDescription::default()
                            .binding(0)
                            .location(1)
                            .format(vk::Format::R32G32B32_SFLOAT)
                            .offset(offset_of!(SkinnedVertex, normal) as u32),
                        vk::VertexInputAttributeDescription::default()
                            .binding(0)
                            .location(2)
                            .format(vk::Format::R32G32_SFLOAT)
                            .offset(offset_of!(SkinnedVertex, uv) as u32),
                        vk::VertexInputAttributeDescription::default()
                            .binding(0)
                            .location(3)
                            .format(vk::Format::R32G32B32A32_SFLOAT)
                            .offset(offset_of!(SkinnedVertex, color) as u32),
                        vk::VertexInputAttributeDescription::default()
                            .binding(0)
                            .location(4)
                            .format(vk::Format::R32G32B32A32_UINT)
                            .offset(offset_of!(SkinnedVertex, joint_indices) as u32),
                        vk::VertexInputAttributeDescription::default()
                            .binding(0)
                            .location(5)
                            .format(vk::Format::R32G32B32A32_SFLOAT)
                            .offset(offset_of!(SkinnedVertex, joint_weights) as u32),
                    ],
                );
        }
        
        // Configure push constants for time
        builder = builder.with_push_constants(vec![
            vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .offset(0)
                .size(4), // Just a float for time
        ]);
        
        // Set descriptor set layout
        builder = builder.with_descriptor_sets(vec![descriptor_set_layout])
            .with_depth_test(true)
            .with_cull_mode(vk::CullModeFlags::BACK)
            .with_front_face(vk::FrontFace::COUNTER_CLOCKWISE);
        
        // Build the pipeline
        let (pipeline, layout) = builder.build()?;
        
        // Store the pipeline
        self.pipelines.insert(
            name.to_string(),
            Pipeline {
                pipeline,
                layout,
            },
        );
        
        Ok(())
    }
    
    // Helper function to set up skinned mesh resources
    fn setup_skinned_mesh_resources(
        &mut self,
        mesh_data: &SkinnedMeshData,
        instance_positions: Option<&[[f32; 3]]>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create vertex buffer for skinned mesh
        let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            &mesh_data.vertices,
        )?;
        
        // Create index buffer
        let (index_buffer, index_buffer_memory) = create_index_buffer(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            self.core.command_pool,
            self.core.graphics_queue,
            &mesh_data.indices,
        )?;
        
        // Create uniform buffers for joints and camera
        let joint_buffer_size = (std::mem::size_of::<Mat4>() * 128) as vk::DeviceSize; // Support up to 128 joints
        let (joint_uniform_buffer, joint_uniform_memory) = create_buffer(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            joint_buffer_size,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        let camera_buffer_size = (std::mem::size_of::<Mat4>() * 2) as vk::DeviceSize; // view + proj matrices
        let (camera_uniform_buffer, camera_uniform_memory) = create_buffer(
            &self.core.instance,
            &self.core.device,
            self.core.physical_device,
            camera_buffer_size,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Create instance buffer if needed
        let (instance_buffer, instance_buffer_memory, instance_count) = if let Some(positions) = instance_positions {
            let (buffer, memory) = create_buffer(
                &self.core.instance,
                &self.core.device,
                self.core.physical_device,
                (positions.len() * std::mem::size_of::<[f32; 3]>()) as vk::DeviceSize,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            
            // Copy instance data to buffer
            unsafe {
                let data = self.core.device.map_memory(
                    memory,
                    0,
                    (positions.len() * std::mem::size_of::<[f32; 3]>()) as vk::DeviceSize,
                    vk::MemoryMapFlags::empty(),
                )?;
                std::ptr::copy_nonoverlapping(
                    positions.as_ptr() as *const u8,
                    data as *mut u8,
                    positions.len() * std::mem::size_of::<[f32; 3]>(),
                );
                self.core.device.unmap_memory(memory);
            }
            
            (Some(buffer), Some(memory), positions.len() as u32)
        } else {
            (None, None, 1)
        };
        
        // Create descriptor pool and sets for uniforms
        let pool_sizes = vec![
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: self.core.swapchain_images.len() as u32 * 2, // joints + camera
            },
        ];
        
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(self.core.swapchain_images.len() as u32);
        
        let descriptor_pool = unsafe {
            self.core.device.create_descriptor_pool(&pool_info, None)?
        };
        
        // Create descriptor set layout
        let bindings = vec![
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
        ];
        
        let descriptor_set_layout = create_descriptor_set_layout(&self.core.device, &bindings)?;
        
        // Create descriptor sets
        let layouts = vec![descriptor_set_layout; self.core.swapchain_images.len()];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);
        
        let descriptor_sets = unsafe {
            self.core.device.allocate_descriptor_sets(&alloc_info)?
        };
        
        // Update descriptor sets
        for set in &descriptor_sets {
            let joint_buffer_info = vk::DescriptorBufferInfo::default()
                .buffer(joint_uniform_buffer)
                .offset(0)
                .range(joint_buffer_size);
            
            let camera_buffer_info = vk::DescriptorBufferInfo::default()
                .buffer(camera_uniform_buffer)
                .offset(0)
                .range(camera_buffer_size);
            
            let descriptor_writes = vec![
                vk::WriteDescriptorSet::default()
                    .dst_set(*set)
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(std::slice::from_ref(&joint_buffer_info)),
                vk::WriteDescriptorSet::default()
                    .dst_set(*set)
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(std::slice::from_ref(&camera_buffer_info)),
            ];
            
            unsafe {
                self.core.device.update_descriptor_sets(&descriptor_writes, &[]);
            }
        }
        
        // Store the resources
        self.skinned_mesh = Some(SkinnedMeshResources {
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            index_count: mesh_data.indices.len() as u32,
            joint_uniform_buffer,
            joint_uniform_memory,
            camera_uniform_buffer,
            camera_uniform_memory,
            descriptor_pool,
            descriptor_set_layout,
            descriptor_sets,
            instance_buffer,
            instance_buffer_memory,
            instance_count,
            use_instancing: instance_positions.is_some(),
        });
        
        // Update the initial joint matrices
        self.update_joint_matrices(&mesh_data.joint_matrices);
        
        Ok(())
    }
    
    // Helper function to create a skinned mesh pipeline
    fn create_skinned_pipeline(
        &mut self,
        name: &str,
        vert_shader_path: &str,
        frag_shader_path: &str,
        use_instancing: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get the descriptor set layout from skinned mesh resources
        let descriptor_set_layout = self.skinned_mesh
            .as_ref()
            .ok_or("Skinned mesh resources not initialized")?
            .descriptor_set_layout;
        
        // Configure push constants for model matrix only (view/proj in uniforms)
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(64); // model matrix only
        
        let mut builder = PipelineBuilder::new(
            self.core.device.clone(),
            vert_shader_path,
            frag_shader_path,
            self.core.swapchain_extent,
            self.core.render_pass,
        )?
        .with_vertex_input(vec![SkinnedVertex::get_binding_description()], SkinnedVertex::get_attribute_descriptions())
        .with_push_constants(vec![push_constant_range])
        .with_descriptor_sets(vec![descriptor_set_layout])
        .with_depth_test(self.has_depth)
        .with_cull_mode(vk::CullModeFlags::BACK)
        .with_front_face(vk::FrontFace::COUNTER_CLOCKWISE);
        
        // Add instance data if using instancing
        if use_instancing {
            // Instance buffer binding (binding 1 for instance data)
            let instance_binding = vk::VertexInputBindingDescription::default()
                .binding(1)
                .stride(std::mem::size_of::<[f32; 3]>() as u32)
                .input_rate(vk::VertexInputRate::INSTANCE);
            
            // Instance position attribute
            let instance_attribute = vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(8) // After skinned vertex attributes
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0);
            
            // Get existing vertex bindings and attributes
            let existing_bindings = vec![SkinnedVertex::get_binding_description()];
            let existing_attributes = SkinnedVertex::get_attribute_descriptions();
            
            // Combine with instance data
            let mut all_bindings = existing_bindings;
            all_bindings.push(instance_binding);
            
            let mut all_attributes = existing_attributes;
            all_attributes.push(instance_attribute);
            
            // Recreate builder with combined input
            builder = PipelineBuilder::new(
                self.core.device.clone(),
                vert_shader_path,
                frag_shader_path,
                self.core.swapchain_extent,
                self.core.render_pass,
            )?
            .with_vertex_input(all_bindings, all_attributes)
            .with_push_constants(vec![push_constant_range])
            .with_descriptor_sets(vec![descriptor_set_layout])
            .with_depth_test(self.has_depth)
            .with_cull_mode(vk::CullModeFlags::BACK)
            .with_front_face(vk::FrontFace::COUNTER_CLOCKWISE);
        }
        
        let (graphics_pipeline, pipeline_layout) = builder.build()?;
        
        self.pipelines.insert(name.to_string(), Pipeline {
            pipeline: graphics_pipeline,
            layout: pipeline_layout,
        });
        
        Ok(())
    }
    
    // Switch to a different pipeline (alias for set_current_pipeline for compatibility)
    pub fn set_active_pipeline(&mut self, name: &str) -> Result<(), String> {
        self.set_current_pipeline(name)
    }
    
    // Switch to a different pipeline
    pub fn set_current_pipeline(&mut self, name: &str) -> Result<(), String> {
        if self.pipelines.contains_key(name) {
            self.current_pipeline = name.to_string();
            // Update the compatibility fields
            if let Some(pipeline) = self.pipelines.get(name) {
                self.graphics_pipeline = pipeline.pipeline;
                self.pipeline_layout = pipeline.layout;
            }
            Ok(())
        } else {
            Err(format!("Pipeline '{}' not found", name))
        }
    }
    
    
    // Render frame with multi-mesh support
    pub fn render_frame_with_camera_multi(&mut self, view: Mat4, proj: Mat4) {
        let image_index = match self.core.begin_frame() {
            Ok(index) => index,
            Err(e) => {
                eprintln!("Failed to begin frame: {}", e);
                return;
            }
        };
        
        self.record_command_buffer_multi_mesh_with_egui(image_index, view, proj, None);
        
        if let Err(e) = self.core.end_frame(image_index) {
            eprintln!("Failed to end frame: {}", e);
        }
    }
    
    pub fn render_frame_instanced(&mut self) {
        let image_index = match self.core.begin_frame() {
            Ok(index) => index,
            Err(e) => {
                eprintln!("Failed to begin frame: {}", e);
                return;
            }
        };
        
        self.record_command_buffer_instanced(image_index);
        
        if let Err(e) = self.core.end_frame(image_index) {
            eprintln!("Failed to end frame: {}", e);
        }
    }
    
    pub fn render_frame_multi_instance(&mut self, instance_positions: &[[f32; 3]]) {
        let image_index = match self.core.begin_frame() {
            Ok(index) => index,
            Err(e) => {
                eprintln!("Failed to begin frame: {}", e);
                return;
            }
        };
        
        self.record_command_buffer_multi_instance(image_index, instance_positions);
        
        if let Err(e) = self.core.end_frame(image_index) {
            eprintln!("Failed to end frame: {}", e);
        }
    }
    
    pub fn render_frame_with_view_proj(&mut self, view_proj: Mat4) {
        let image_index = match self.core.begin_frame() {
            Ok(index) => index,
            Err(e) => {
                eprintln!("Failed to begin frame: {}", e);
                return;
            }
        };
        
        self.record_command_buffer_with_view_proj(image_index, view_proj);
        
        if let Err(e) = self.core.end_frame(image_index) {
            eprintln!("Failed to end frame: {}", e);
        }
    }
    
    pub fn render_frame(&mut self) {
        let image_index = match self.core.begin_frame() {
            Ok(index) => index,
            Err(e) => {
                eprintln!("Failed to begin frame: {}", e);
                return;
            }
        };
        
        self.record_command_buffer(image_index);
        
        if let Err(e) = self.core.end_frame(image_index) {
            eprintln!("Failed to end frame: {}", e);
        }
    }
    
    // Render frame with fluid simulation push constants
    pub fn render_frame_fluid(
        &mut self, 
        view: Mat4, 
        proj: Mat4,
        push_constants: &PushConstants,
    ) {
        let image_index = match self.core.begin_frame() {
            Ok(index) => index,
            Err(e) => {
                eprintln!("Failed to begin frame: {:?}", e);
                return;
            }
        };
        
        // Record command buffer with fluid push constants
        self.record_command_buffer_fluid(image_index, view, proj, push_constants);
        
        let _ = self.core.end_frame(image_index);
    }
    
    pub fn render_frame_with_camera(&mut self, view: Mat4, proj: Mat4) {
        let image_index = match self.core.begin_frame() {
            Ok(index) => index,
            Err(e) => {
                eprintln!("Failed to begin frame: {}", e);
                return;
            }
        };
        
        // If this is a skinned mesh, update camera matrices and render accordingly
        if let Some(ref _skinned) = self.skinned_mesh {
            self.update_camera_matrices(view, proj);
            self.record_command_buffer_skinned(image_index);
        } else {
            self.record_command_buffer_with_push_data(image_index, view, proj);
        }
        
        if let Err(e) = self.core.end_frame(image_index) {
            eprintln!("Failed to end frame: {}", e);
        }
    }
    
    // Update joint matrices for skinned mesh
    pub fn update_joint_matrices(&mut self, joint_matrices: &[Mat4]) {
        if let Some(ref skinned) = self.skinned_mesh {
            unsafe {
                let buffer_size = (std::mem::size_of::<Mat4>() * 128) as u64;
                let data_ptr = self.core.device.map_memory(
                    skinned.joint_uniform_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                ).unwrap() as *mut Mat4;
                
                // Copy joint matrices to buffer
                for (i, mat) in joint_matrices.iter().enumerate() {
                    if i >= 128 { break; }
                    data_ptr.add(i).write(*mat);
                }
                
                self.core.device.unmap_memory(skinned.joint_uniform_memory);
            }
        }
    }
    
    // Update camera matrices for skinned mesh
    fn update_camera_matrices(&mut self, view: Mat4, proj: Mat4) {
        if let Some(ref skinned) = self.skinned_mesh {
            // Camera matrices being updated
            unsafe {
                let buffer_size = (std::mem::size_of::<Mat4>() * 2) as u64;
                let data_ptr = self.core.device.map_memory(
                    skinned.camera_uniform_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                ).unwrap() as *mut Mat4;
                
                // Write view and projection matrices
                data_ptr.write(view);
                data_ptr.add(1).write(proj);
                
                self.core.device.unmap_memory(skinned.camera_uniform_memory);
            }
        }
    }
    
    // Record command buffer for skinned mesh rendering
    fn record_command_buffer_skinned(&self, image_index: u32) {
        let command_buffer = self.core.command_buffers[image_index as usize];
        let framebuffer = self.core.framebuffers[image_index as usize];
        
        if let Some(ref skinned) = self.skinned_mesh {
            let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            
            unsafe {
                self.core.device
                    .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                    .expect("Failed to begin recording command buffer");
                
                let clear_values = [
                    vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.529, 0.0, 0.69, 1.0], // Magenta clear color
                        },
                    },
                    vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 1.0,
                            stencil: 0,
                        },
                    },
                ];
                
                let render_pass_begin_info = vk::RenderPassBeginInfo::default()
                    .render_pass(self.core.render_pass)
                    .framebuffer(framebuffer)
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.core.swapchain_extent,
                    })
                    .clear_values(&clear_values);
                
                self.core.device.cmd_begin_render_pass(
                    command_buffer,
                    &render_pass_begin_info,
                    vk::SubpassContents::INLINE,
                );
                
                // Choose appropriate pipeline based on instancing
                let pipeline_name = if skinned.use_instancing {
                    "skinned_instanced"
                } else {
                    "skinned"
                };
                
                // Bind the appropriate pipeline
                if let Some(pipeline) = self.pipelines.get(pipeline_name) {
                    self.core.device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline.pipeline,
                    );
                    
                    // Bind descriptor sets
                    let descriptor_set = skinned.descriptor_sets[self.core.current_frame];
                    self.core.device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline.layout,
                        0,
                        &[descriptor_set],
                        &[],
                    );
                    
                    // Bind vertex buffers - include instance buffer if using instancing
                    if skinned.use_instancing {
                        if let Some(instance_buffer) = skinned.instance_buffer {
                            // Bind both vertex buffer (binding 0) and instance buffer (binding 1)
                            let vertex_buffers = [skinned.vertex_buffer, instance_buffer];
                            let offsets = [0, 0];
                            self.core.device.cmd_bind_vertex_buffers(
                                command_buffer,
                                0,
                                &vertex_buffers,
                                &offsets,
                            );
                        } else {
                            // Fallback to non-instanced if instance buffer is missing
                            self.core.device.cmd_bind_vertex_buffers(
                                command_buffer,
                                0,
                                &[skinned.vertex_buffer],
                                &[0],
                            );
                        }
                    } else {
                        // Non-instanced: bind only vertex buffer
                        self.core.device.cmd_bind_vertex_buffers(
                            command_buffer,
                            0,
                            &[skinned.vertex_buffer],
                            &[0],
                        );
                    }
                    
                    // Bind index buffer
                    self.core.device.cmd_bind_index_buffer(
                        command_buffer,
                        skinned.index_buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                    
                    // Push time constant
                    let push_data = self.core.start_time.elapsed().as_secs_f32();
                    self.core.device.cmd_push_constants(
                        command_buffer,
                        pipeline.layout,
                        vk::ShaderStageFlags::VERTEX,
                        0,
                        std::slice::from_raw_parts(&push_data as *const f32 as *const u8, 4),
                    );
                    
                    // Draw indexed with appropriate instance count
                    self.core.device.cmd_draw_indexed(
                        command_buffer,
                        skinned.index_count,
                        skinned.instance_count,  // Use the instance count from skinned resources
                        0,
                        0,
                        0,
                    );
                } else {
                    println!("ERROR: Could not find {} pipeline!", pipeline_name);
                }
                
                self.core.device.cmd_end_render_pass(command_buffer);
                self.core.device
                    .end_command_buffer(command_buffer)
                    .expect("Failed to record command buffer");
            }
        }
    }
    
    fn record_command_buffer(&self, image_index: u32) {
        let command_buffer = self.core.command_buffers[image_index as usize];
        let framebuffer = self.core.framebuffers[image_index as usize];
        
        // Build render configuration
        let mut config = RenderConfig::default();
        
        // Set draw mode and resources based on what we have
        if let Some(ref buffers) = self.buffers {
            config.vertex_buffer = Some(buffers.vertex_buffer);
            
            if let Some(index_buffer) = buffers.index_buffer {
                config.index_buffer = Some(index_buffer);
                
                if self.instance_count > 1 {
                    config.draw_mode = DrawMode::IndexedInstanced {
                        index_count: buffers.index_count,
                        instance_count: self.instance_count,
                    };
                } else {
                    config.draw_mode = DrawMode::Indexed {
                        index_count: buffers.index_count,
                    };
                }
            }
        } else {
            // Simple rendering without buffers
            config.draw_mode = DrawMode::Simple {
                vertex_count: self.vertex_count,
            };
        }
        
        // Add texture descriptor if available
        if let Some(ref textures) = self.textures {
            config.descriptor_sets = &textures.descriptor_sets[image_index as usize..=image_index as usize];
        }
        
        // Add time push constant for simple animations
        let elapsed = self.core.get_elapsed_time();
        let time_data = [elapsed];
        let time_bytes = bytemuck::cast_slice(&time_data);
        config.push_constant_data = Some(time_bytes);
        
        // Use the unified command buffer recording
        record_command_buffer_unified(
            &self.core.device,
            command_buffer,
            self.core.render_pass,
            framebuffer,
            self.core.swapchain_extent,
            self.graphics_pipeline,
            self.pipeline_layout,
            &config,
            self.has_depth,
        );
    }
    
    fn record_command_buffer_instanced(&self, image_index: u32) {
        let command_buffer = self.core.command_buffers[image_index as usize];
        let framebuffer = self.core.framebuffers[image_index as usize];
        
        // Build render configuration
        let mut config = RenderConfig::default();
        config.clear_color = CLEAR_COLOR_MAGENTA;
        
        // Set resources
        if let Some(ref buffers) = self.buffers {
            config.vertex_buffer = Some(buffers.vertex_buffer);
            
            if let Some(index_buffer) = buffers.index_buffer {
                config.index_buffer = Some(index_buffer);
                
                // Use instanced drawing
                config.draw_mode = DrawMode::IndexedInstanced {
                    index_count: buffers.index_count,
                    instance_count: self.instance_count,
                };
            }
        }
        
        // Add texture descriptor if available
        if let Some(ref textures) = self.textures {
            config.descriptor_sets = &textures.descriptor_sets[image_index as usize..=image_index as usize];
        }
        
        // Add time push constant
        let elapsed = self.core.get_elapsed_time();
        let time_data = [elapsed];
        let time_bytes = bytemuck::cast_slice(&time_data);
        config.push_constant_data = Some(time_bytes);
        
        // Record command buffer with special handling for instance buffer
        unsafe {
            self.core.device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
                .expect("Failed to reset command buffer");
            
            let begin_info = vk::CommandBufferBeginInfo::default();
            self.core.device.begin_command_buffer(command_buffer, &begin_info)
                .expect("Failed to begin command buffer");
            
            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: config.clear_color,
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];
            
            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.core.render_pass)
                .framebuffer(framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.core.swapchain_extent,
                })
                .clear_values(&clear_values);
            
            self.core.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );
            
            self.core.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            );
            
            // Bind both vertex and instance buffers
            if let Some(ref buffers) = self.buffers {
                // Bind both vertex buffer (binding 0) and instance buffer (binding 1)
                if let Some(instance_buffer) = buffers.instance_buffer {
                    let vertex_buffers = [buffers.vertex_buffer, instance_buffer];
                    let offsets = [0, 0];
                    self.core.device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
                } else {
                    let vertex_buffers = [buffers.vertex_buffer];
                    let offsets = [0];
                    self.core.device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
                }
                
                if let Some(index_buffer) = buffers.index_buffer {
                    self.core.device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);
                }
            }
            
            // Bind descriptor sets if available
            if !config.descriptor_sets.is_empty() {
                self.core.device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_layout,
                    0,
                    config.descriptor_sets,
                    &[],
                );
            }
            
            // Push constants
            if let Some(push_data) = config.push_constant_data {
                self.core.device.cmd_push_constants(
                    command_buffer,
                    self.pipeline_layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    push_data,
                );
            }
            
            // Draw instanced
            if let DrawMode::IndexedInstanced { index_count, instance_count } = config.draw_mode {
                self.core.device.cmd_draw_indexed(
                    command_buffer,
                    index_count,
                    instance_count,
                    0,
                    0,
                    0,
                );
            }
            
            self.core.device.cmd_end_render_pass(command_buffer);
            self.core.device.end_command_buffer(command_buffer)
                .expect("Failed to record command buffer");
        }
    }
    
    fn record_command_buffer_multi_instance(&self, image_index: u32, instance_positions: &[[f32; 3]]) {
        let command_buffer = self.core.command_buffers[image_index as usize];
        let framebuffer = self.core.framebuffers[image_index as usize];
        
        unsafe {
            self.core.device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
                .expect("Failed to reset command buffer");
            
            let begin_info = vk::CommandBufferBeginInfo::default();
            self.core.device.begin_command_buffer(command_buffer, &begin_info)
                .expect("Failed to begin command buffer");
            
            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: CLEAR_COLOR_MAGENTA,
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];
            
            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.core.render_pass)
                .framebuffer(framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.core.swapchain_extent,
                })
                .clear_values(&clear_values);
            
            self.core.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );
            
            self.core.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            );
            
            // Bind vertex and index buffers if available
            if let Some(ref buffers) = self.buffers {
                let vertex_buffers = [buffers.vertex_buffer];
                let offsets = [0];
                self.core.device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
                
                if let Some(index_buffer) = buffers.index_buffer {
                    self.core.device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);
                }
            }
            
            // Bind descriptor sets if available
            if let Some(ref textures) = self.textures {
                let descriptor_sets = [textures.descriptor_sets[image_index as usize]];
                self.core.device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_layout,
                    0,
                    &descriptor_sets,
                    &[],
                );
            }
            
            // Get elapsed time
            let elapsed = self.core.get_elapsed_time();
            
            // Draw each instance with its position
            for position in instance_positions {
                let push_data = [elapsed, position[0], position[1], position[2]];
                let push_bytes = bytemuck::cast_slice(&push_data);
                
                self.core.device.cmd_push_constants(
                    command_buffer,
                    self.pipeline_layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    push_bytes,
                );
                
                if let Some(ref buffers) = self.buffers {
                    if buffers.index_buffer.is_some() {
                        self.core.device.cmd_draw_indexed(command_buffer, buffers.index_count, 1, 0, 0, 0);
                    }
                }
            }
            
            self.core.device.cmd_end_render_pass(command_buffer);
            self.core.device.end_command_buffer(command_buffer)
                .expect("Failed to record command buffer");
        }
    }
    
    fn record_command_buffer_with_view_proj(&self, image_index: u32, view_proj: Mat4) {
        let command_buffer = self.core.command_buffers[image_index as usize];
        let framebuffer = self.core.framebuffers[image_index as usize];
        
        // Prepare view-proj matrix for push constant
        let view_proj_array = view_proj.to_cols_array_2d();
        
        let mut config = RenderConfig::default();
        config.clear_color = CLEAR_COLOR_MAGENTA;
        
        // Set resources
        if let Some(ref buffers) = self.buffers {
            config.vertex_buffer = Some(buffers.vertex_buffer);
            
            if let Some(index_buffer) = buffers.index_buffer {
                config.index_buffer = Some(index_buffer);
                config.draw_mode = DrawMode::Indexed {
                    index_count: buffers.index_count,
                };
            }
        }
        
        // Use texture array descriptor if available
        let descriptor_sets_vec;
        if let Some(ref texture_arrays) = self.texture_arrays {
            descriptor_sets_vec = vec![texture_arrays.descriptor_set];
            config.descriptor_sets = &descriptor_sets_vec;
        }
        
        // Set push constants
        let push_bytes = bytemuck::cast_slice(&view_proj_array);
        config.push_constant_data = Some(push_bytes);
        
        record_command_buffer_unified(
            &self.core.device,
            command_buffer,
            self.core.render_pass,
            framebuffer,
            self.core.swapchain_extent,
            self.graphics_pipeline,
            self.pipeline_layout,
            &config,
            self.has_depth,
        );
    }
    
    fn record_command_buffer_with_push_data(&mut self, image_index: u32, view: Mat4, proj: Mat4) {
        self.record_command_buffer_with_push_data_and_egui(image_index, view, proj, None);
    }
    
    fn record_command_buffer_multi_mesh_with_egui(&mut self, image_index: u32, view: Mat4, proj: Mat4, egui_output: Option<egui::FullOutput>) {
        let command_buffer = self.core.command_buffers[image_index as usize];
        let framebuffer = self.core.framebuffers[image_index as usize];
        
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default();
            
            self.core.device
                .begin_command_buffer(command_buffer, &begin_info)
                .expect("Failed to begin command buffer");
            
            // Begin render pass
            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: CLEAR_COLOR_MAGENTA,
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];
            
            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.core.render_pass)
                .framebuffer(framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.core.swapchain_extent,
                })
                .clear_values(&clear_values);
            
            self.core.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );
            
            // Set viewport and scissor (do this once before the loop)
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.core.swapchain_extent.width as f32,
                height: self.core.swapchain_extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            self.core.device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.core.swapchain_extent,
            };
            self.core.device.cmd_set_scissor(command_buffer, 0, &[scissor]);
            
            // Track the currently bound pipeline to avoid redundant binds
            let mut current_pipeline_name: Option<String> = None;
            
            // Render each mesh with its transforms
            for (mesh_idx, mesh) in self.meshes.iter().enumerate() {
                // Skip meshes with no transforms and non-instanced meshes with no instances
                if !mesh.use_instancing && mesh.transforms.is_empty() {
                    continue;
                }
                if mesh.use_instancing && mesh.instance_count == 0 {
                    continue;
                }
                
                // Determine which pipeline to use for this mesh
                let actual_pipeline_name = mesh.pipeline_name.as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("default");
                
                // Debug log for colonist meshes
                if actual_pipeline_name.contains("colonist") || mesh_idx == 50 {
                    static mut COLONIST_LOG_COUNT: u32 = 0;
                    COLONIST_LOG_COUNT += 1;
                    if COLONIST_LOG_COUNT % 60 == 0 {
                        println!("Rendering mesh {}: pipeline={}, is_skinned={}, instance_count={}, use_instancing={}", 
                                 mesh_idx, actual_pipeline_name, mesh.is_skinned, mesh.instance_count, mesh.use_instancing);
                    }
                }
                
                // Switch pipeline if needed
                if current_pipeline_name.as_deref() != Some(actual_pipeline_name) {
                    let (pipeline, _pipeline_layout) = if let Some(pipeline_entry) = self.pipelines.get(actual_pipeline_name) {
                        (pipeline_entry.pipeline, pipeline_entry.layout)
                    } else {
                        // Fallback to default pipeline
                        println!("WARNING: Pipeline '{}' not found, using default", actual_pipeline_name);
                        (self.graphics_pipeline, self.pipeline_layout)
                    };
                    
                    self.core.device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline,
                    );
                    
                    current_pipeline_name = Some(actual_pipeline_name.to_string());
                }
                
                // Get the current pipeline layout for push constants
                let pipeline_layout = if let Some(pipeline_entry) = self.pipelines.get(actual_pipeline_name) {
                    pipeline_entry.layout
                } else {
                    self.pipeline_layout
                };
                
                // Check if using GPU instancing
                if mesh.use_instancing {
                    // TRUE GPU INSTANCING PATH
                    
                    // Bind vertex buffer at binding 0
                    self.core.device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[mesh.vertex_buffer],
                        &[0],
                    );
                    
                    // Bind instance buffer at binding 1 if available
                    if let Some(instance_buffer) = mesh.instance_buffer {
                        self.core.device.cmd_bind_vertex_buffers(
                            command_buffer,
                            1,
                            &[instance_buffer],
                            &[0],
                        );
                    }
                    
                    // Bind index buffer
                    self.core.device.cmd_bind_index_buffer(
                        command_buffer,
                        mesh.index_buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                    
                    // Bind descriptor sets if available
                    // For skinned meshes, bind the skinned descriptor sets
                    if mesh.is_skinned {
                        // Debug log for skinned mesh descriptor binding
                        if actual_pipeline_name.contains("colonist") {
                            static mut SKINNED_LOG_COUNT: u32 = 0;
                            SKINNED_LOG_COUNT += 1;
                            if SKINNED_LOG_COUNT % 60 == 0 {
                                println!("Binding skinned descriptors for mesh {}: has_sets={}, has_camera_buffer={}", 
                                         mesh_idx, 
                                         mesh.skinned_descriptor_sets.is_some(),
                                         mesh.camera_uniform_memory.is_some());
                            }
                        }
                        
                        // Update camera uniform buffer with current view/proj matrices
                        if let Some(camera_buffer_memory) = mesh.camera_uniform_memory {
                            let camera_uniforms = CameraUniforms {
                                view: view.to_cols_array(),
                                proj: proj.to_cols_array(),
                            };
                            
                            if let Ok(data) = self.core.device.map_memory(
                                    camera_buffer_memory,
                                    0,
                                    std::mem::size_of::<CameraUniforms>() as u64,
                                    vk::MemoryMapFlags::empty(),
                                ) {
                                    std::ptr::copy_nonoverlapping(
                                        &camera_uniforms as *const _ as *const u8,
                                        data as *mut u8,
                                        std::mem::size_of::<CameraUniforms>(),
                                    );
                                    self.core.device.unmap_memory(camera_buffer_memory);
                                } else {
                                    eprintln!("Failed to map camera buffer memory");
                                }
                        }
                        
                        if let Some(ref descriptor_sets) = mesh.skinned_descriptor_sets {
                            if descriptor_sets.len() > image_index as usize {
                                self.core.device.cmd_bind_descriptor_sets(
                                    command_buffer,
                                    vk::PipelineBindPoint::GRAPHICS,
                                    pipeline_layout,
                                    0,
                                    &descriptor_sets[image_index as usize..=image_index as usize],
                                    &[],
                                );
                            }
                        }
                    } else if let Some(ref textures) = mesh.texture_resources {
                        self.core.device.cmd_bind_descriptor_sets(
                            command_buffer,
                            vk::PipelineBindPoint::GRAPHICS,
                            pipeline_layout,
                            0,
                            &textures.descriptor_sets[image_index as usize..=image_index as usize],
                            &[],
                        );
                    } else if let Some(ref textures) = self.textures {
                        self.core.device.cmd_bind_descriptor_sets(
                            command_buffer,
                            vk::PipelineBindPoint::GRAPHICS,
                            pipeline_layout,
                            0,
                            &textures.descriptor_sets[image_index as usize..=image_index as usize],
                            &[],
                        );
                    }
                    
                    // Set push constants based on whether this is a skinned mesh
                    if mesh.is_skinned {
                        // Skinned shaders only expect time as push constant
                        let push_data = self.core.start_time.elapsed().as_secs_f32();
                        self.core.device.cmd_push_constants(
                            command_buffer,
                            pipeline_layout,
                            vk::ShaderStageFlags::VERTEX,
                            0,
                            std::slice::from_raw_parts(&push_data as *const f32 as *const u8, 4),
                        );
                    } else {
                        // Regular meshes use MVP push constants
                        let mvp = MvpPushConstants {
                            model: Mat4::IDENTITY.to_cols_array(),  // Model matrix handled by instance data
                            view: view.to_cols_array(),
                            proj: proj.to_cols_array(),
                            base_color: mesh.base_color,
                        };
                        
                        let push_bytes = bytemuck::bytes_of(&mvp);
                        
                        self.core.device.cmd_push_constants(
                            command_buffer,
                            pipeline_layout,
                            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                            0,
                            push_bytes,
                        );
                    }
                    
                    // Debug log draw call for colonist meshes
                    if actual_pipeline_name.contains("colonist") && mesh.use_instancing {
                        static mut DRAW_LOG_COUNT: u32 = 0;
                        DRAW_LOG_COUNT += 1;
                        if DRAW_LOG_COUNT % 60 == 0 {
                            println!("Drawing colonist mesh {}: index_count={}, instance_count={}, vertex_count={}", 
                                     mesh_idx, mesh.index_count, mesh.instance_count,
                                     mesh.index_count / 3); // Approximate vertex count
                            println!("  Using pipeline: {}", actual_pipeline_name);
                            println!("  Is skinned: {}", mesh.is_skinned);
                            println!("  Has descriptor sets: {}", mesh.skinned_descriptor_sets.is_some());
                        }
                    }
                    
                    // SINGLE DRAW CALL FOR ALL INSTANCES!
                    self.core.device.cmd_draw_indexed(
                        command_buffer,
                        mesh.index_count,
                        mesh.instance_count,  // Draw all instances in one call!
                        0,
                        0,
                        0,
                    );
                } else {
                    // INDIVIDUAL DRAW CALLS PATH (old behavior)
                    
                    // Bind vertex buffer
                    self.core.device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[mesh.vertex_buffer],
                        &[0],
                    );
                    
                    // Bind index buffer
                    self.core.device.cmd_bind_index_buffer(
                        command_buffer,
                        mesh.index_buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                    
                    // Bind descriptor sets if available
                    // For skinned meshes, bind the skinned descriptor sets
                    if mesh.is_skinned {
                        if let Some(ref descriptor_sets) = mesh.skinned_descriptor_sets {
                            self.core.device.cmd_bind_descriptor_sets(
                                command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                pipeline_layout,
                                0,
                                &descriptor_sets[image_index as usize..=image_index as usize],
                                &[],
                            );
                        }
                    } else if let Some(ref textures) = mesh.texture_resources {
                        self.core.device.cmd_bind_descriptor_sets(
                            command_buffer,
                            vk::PipelineBindPoint::GRAPHICS,
                            pipeline_layout,
                            0,
                            &textures.descriptor_sets[image_index as usize..=image_index as usize],
                            &[],
                        );
                    } else if let Some(ref textures) = self.textures {
                        self.core.device.cmd_bind_descriptor_sets(
                            command_buffer,
                            vk::PipelineBindPoint::GRAPHICS,
                            pipeline_layout,
                            0,
                            &textures.descriptor_sets[image_index as usize..=image_index as usize],
                            &[],
                        );
                    }
                    
                    // Draw each instance with its transform
                    for transform in &mesh.transforms {
                        let mvp = MvpPushConstants {
                            model: transform.to_cols_array(),
                            view: view.to_cols_array(),
                            proj: proj.to_cols_array(),
                            base_color: mesh.base_color,
                        };
                        
                        let push_bytes = bytemuck::bytes_of(&mvp);
                        
                        self.core.device.cmd_push_constants(
                            command_buffer,
                            pipeline_layout,
                            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                            0,
                            push_bytes,
                        );
                        
                        // Draw indexed
                        self.core.device.cmd_draw_indexed(
                            command_buffer,
                            mesh.index_count,
                            1,
                            0,
                            0,
                            0,
                        );
                    }
                }
            }
            
            // Fallback: render using the old buffers if meshes are empty but buffers exist
            if self.meshes.is_empty() && self.buffers.is_some() {
                // Use the old hardcoded model for backwards compatibility
                let model = Mat4::from_translation(Vec3::new(0.0, 0.0, -2.0)) * 
                            Mat4::from_scale(Vec3::splat(0.5));
                let mvp = MvpPushConstants {
                    model: model.to_cols_array(),
                    view: view.to_cols_array(),
                    proj: proj.to_cols_array(),
                    base_color: [1.0, 1.0, 1.0, 1.0], // Default white for legacy path
                };
                
                if let Some(ref buffers) = self.buffers {
                    self.core.device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[buffers.vertex_buffer],
                        &[0],
                    );
                    
                    if let Some(index_buffer) = buffers.index_buffer {
                        self.core.device.cmd_bind_index_buffer(
                            command_buffer,
                            index_buffer,
                            0,
                            vk::IndexType::UINT32,
                        );
                        
                        let push_bytes = bytemuck::bytes_of(&mvp);
                        self.core.device.cmd_push_constants(
                            command_buffer,
                            self.pipeline_layout,
                            vk::ShaderStageFlags::VERTEX,
                            0,
                            push_bytes,
                        );
                        
                        self.core.device.cmd_draw_indexed(
                            command_buffer,
                            buffers.index_count,
                            self.instance_count.max(1),
                            0,
                            0,
                            0,
                        );
                    }
                }
            }
            
            // Render egui if provided
            if let Some(egui_output) = egui_output {
                if let Some(ref mut egui_integration) = self.egui_integration {
                    // Update textures before rendering
                    if !egui_output.textures_delta.set.is_empty() {
                        if let Err(e) = egui_integration.renderer.set_textures(
                            self.core.graphics_queue,
                            self.core.command_pool,
                            egui_output.textures_delta.set.as_slice(),
                        ) {
                            eprintln!("Failed to set egui textures: {}", e);
                        }
                    }
                    
                    let clipped_primitives = egui_integration.context.tessellate(
                        egui_output.shapes,
                        egui_output.pixels_per_point,
                    );
                    
                    if let Err(e) = egui_integration.renderer.cmd_draw(
                        command_buffer,
                        self.core.swapchain_extent,
                        egui_output.pixels_per_point,
                        &clipped_primitives,
                    ) {
                        eprintln!("Failed to render egui: {}", e);
                    }
                    
                    // Free removed textures
                    if !egui_output.textures_delta.free.is_empty() {
                        if let Err(e) = egui_integration.renderer.free_textures(&egui_output.textures_delta.free) {
                            eprintln!("Failed to free egui textures: {}", e);
                        }
                    }
                }
            }
            
            self.core.device.cmd_end_render_pass(command_buffer);
            
            self.core.device
                .end_command_buffer(command_buffer)
                .expect("Failed to end command buffer");
        }
    }
    
    fn record_command_buffer_with_push_data_and_egui(&mut self, image_index: u32, view: Mat4, proj: Mat4, egui_output: Option<egui::FullOutput>) {
        let command_buffer = self.core.command_buffers[image_index as usize];
        let framebuffer = self.core.framebuffers[image_index as usize];
        
        // Use identity model matrix - let the view and proj matrices handle positioning
        let model = Mat4::IDENTITY;
        let mvp = MvpPushConstants {
            model: model.to_cols_array(),
            view: view.to_cols_array(),
            proj: proj.to_cols_array(),
            base_color: [1.0, 1.0, 1.0, 1.0], // Default white
        };
        
        let mut config = RenderConfig::default();
        config.clear_color = CLEAR_COLOR_MAGENTA;
        
        // Set resources
        if let Some(ref buffers) = self.buffers {
            config.vertex_buffer = Some(buffers.vertex_buffer);
            
            if let Some(index_buffer) = buffers.index_buffer {
                config.index_buffer = Some(index_buffer);
                
                if self.instance_count > 1 {
                    config.draw_mode = DrawMode::IndexedInstanced {
                        index_count: buffers.index_count,
                        instance_count: self.instance_count,
                    };
                } else {
                    config.draw_mode = DrawMode::Indexed {
                        index_count: buffers.index_count,
                    };
                }
            }
        }
        
        if let Some(ref textures) = self.textures {
            config.descriptor_sets = &textures.descriptor_sets[image_index as usize..=image_index as usize];
        }
        
        // Set push constants
        let mvp_array = [mvp];
        let push_bytes = bytemuck::cast_slice(&mvp_array);
        config.push_constant_data = Some(push_bytes);
        
        // Record commands with egui support
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default();
            
            self.core.device
                .begin_command_buffer(command_buffer, &begin_info)
                .expect("Failed to begin recording command buffer");
            
            let mut clear_values = vec![
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: config.clear_color,
                    },
                },
            ];
            
            if self.has_depth {
                clear_values.push(vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                });
            }
            
            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.core.render_pass)
                .framebuffer(framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.core.swapchain_extent,
                })
                .clear_values(&clear_values);
            
            self.core.device.cmd_begin_render_pass(command_buffer, &render_pass_info, vk::SubpassContents::INLINE);
            
            // Draw main geometry
            if self.graphics_pipeline != vk::Pipeline::null() {
                self.core.device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, self.graphics_pipeline);
                
                // Bind descriptor sets if available
                if !config.descriptor_sets.is_empty() {
                    self.core.device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout,
                        0,
                        config.descriptor_sets,
                        &[],
                    );
                }
                
                // Push constants
                if let Some(push_data) = config.push_constant_data {
                    self.core.device.cmd_push_constants(
                        command_buffer,
                        self.pipeline_layout,
                        vk::ShaderStageFlags::VERTEX,
                        0,
                        push_data,
                    );
                }
                
                // Bind vertex buffer
                if let Some(vertex_buffer) = config.vertex_buffer {
                    self.core.device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
                }
                
                // Draw based on mode
                match config.draw_mode {
                    DrawMode::Indexed { index_count } => {
                        if let Some(index_buffer) = config.index_buffer {
                            self.core.device.cmd_bind_index_buffer(
                                command_buffer,
                                index_buffer,
                                0,
                                vk::IndexType::UINT32,
                            );
                            self.core.device.cmd_draw_indexed(command_buffer, index_count, 1, 0, 0, 0);
                        }
                    }
                    DrawMode::IndexedInstanced { index_count, instance_count } => {
                        if let Some(index_buffer) = config.index_buffer {
                            self.core.device.cmd_bind_index_buffer(
                                command_buffer,
                                index_buffer,
                                0,
                                vk::IndexType::UINT32,
                            );
                            self.core.device.cmd_draw_indexed(command_buffer, index_count, instance_count, 0, 0, 0);
                        }
                    }
                    _ => {}
                }
            }
            
            // Draw egui if output is provided
            if let (Some(egui_integration), Some(output)) = (&mut self.egui_integration, egui_output) {
                // Paint egui inside the render pass
                if let Err(e) = egui_integration.paint(
                    command_buffer,
                    self.core.swapchain_extent,
                    output,
                ) {
                    eprintln!("Failed to paint egui: {}", e);
                }
            }
            
            self.core.device.cmd_end_render_pass(command_buffer);
            
            self.core.device
                .end_command_buffer(command_buffer)
                .expect("Failed to record command buffer");
        }
    }
    
    fn record_command_buffer_fluid(
        &self, 
        image_index: u32, 
        _view: Mat4, 
        _proj: Mat4,
        fluid_push_constants: &PushConstants,
    ) {
        let command_buffer = self.core.command_buffers[image_index as usize];
        let framebuffer = self.core.framebuffers[image_index as usize];
        
        let begin_info = vk::CommandBufferBeginInfo::default();
        
        unsafe {
            self.core.device
                .begin_command_buffer(command_buffer, &begin_info)
                .expect("Failed to begin recording command buffer");
            
            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0], // Black clear (sky will overwrite)
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];
            
            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.core.render_pass)
                .framebuffer(framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.core.swapchain_extent,
                })
                .clear_values(&clear_values);
            
            self.core.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );
            
            // First, render the sky background (if sky pipeline exists)
            if let Some(sky_pipeline_entry) = self.pipelines.get("sky") {
                // Bind sky pipeline
                self.core.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    sky_pipeline_entry.pipeline,
                );
                
                // Push constants for sky
                let push_bytes = bytemuck::bytes_of(fluid_push_constants);
                self.core.device.cmd_push_constants(
                    command_buffer,
                    sky_pipeline_entry.layout,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    push_bytes,
                );
                
                // Draw fullscreen triangle for sky (3 vertices, no vertex buffer needed)
                self.core.device.cmd_draw(command_buffer, 3, 1, 0, 0);
            }
            
            // Then render each mesh with the appropriate pipeline
            for (_mesh_idx, mesh) in self.meshes.iter().enumerate() {
                // Skip meshes with no transforms
                if !mesh.use_instancing && mesh.transforms.is_empty() {
                    continue;
                }
                
                // Determine which pipeline to use
                let pipeline_name = mesh.pipeline_name.as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("water"); // Default to water pipeline for fluid sim
                
                // Get the pipeline
                let (pipeline, pipeline_layout) = if let Some(pipeline_entry) = self.pipelines.get(pipeline_name) {
                    (pipeline_entry.pipeline, pipeline_entry.layout)
                } else {
                    // Fall back to default pipeline if not found
                    println!("WARNING: Pipeline '{}' not found, using default", pipeline_name);
                    continue;
                };
                
                // Bind the pipeline
                self.core.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline,
                );
                
                // Bind descriptor sets if this is a textured pipeline
                if let Some(textured_resources) = self.textured_pipelines.get(pipeline_name) {
                    self.core.device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline_layout,
                        0,
                        &[textured_resources.descriptor_sets[image_index as usize]],
                        &[],
                    );
                }
                
                // Push the fluid constants
                let push_bytes = bytemuck::bytes_of(fluid_push_constants);
                self.core.device.cmd_push_constants(
                    command_buffer,
                    pipeline_layout,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    push_bytes,
                );
                
                // Bind vertex and index buffers
                self.core.device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[mesh.vertex_buffer],
                    &[0],
                );
                
                self.core.device.cmd_bind_index_buffer(
                    command_buffer,
                    mesh.index_buffer,
                    0,
                    vk::IndexType::UINT32,
                );
                
                // Draw the mesh
                if mesh.use_instancing && mesh.instance_buffer.is_some() {
                    self.core.device.cmd_draw_indexed(
                        command_buffer,
                        mesh.index_count,
                        mesh.instance_count,
                        0,
                        0,
                        0,
                    );
                } else {
                    // Draw once for each transform
                    let num_instances = mesh.transforms.len().max(1) as u32;
                    self.core.device.cmd_draw_indexed(
                        command_buffer,
                        mesh.index_count,
                        num_instances,
                        0,
                        0,
                        0,
                    );
                }
            }
            
            self.core.device.cmd_end_render_pass(command_buffer);
            self.core.device
                .end_command_buffer(command_buffer)
                .expect("Failed to record command buffer");
        }
    }
    
    // Get render pass for external use
    pub fn get_render_pass(&self) -> vk::RenderPass {
        self.core.render_pass
    }
    
    // Get egui context for UI code
    pub fn get_egui_context(&mut self) -> Option<&egui::Context> {
        self.egui_integration.as_ref().map(|i| &i.context)
    }
    
    // Initialize egui integration
    pub fn initialize_egui(&mut self, render_pass: vk::RenderPass) -> Result<(), Box<dyn std::error::Error>> {
        let egui_integration = EguiIntegration::new(
            &self.core.instance,
            self.core.physical_device,
            self.core.device.clone(),
            render_pass,
            self.core.graphics_queue,
            self.core.command_pool,
        )?;
        
        self.egui_integration = Some(egui_integration);
        Ok(())
    }
    
    
    // Render frame with egui support
    pub fn render_frame_with_egui(
        &mut self,
        view: Mat4,
        proj: Mat4,
        egui_output: Option<egui::FullOutput>,
    ) {
        let image_index = match self.core.begin_frame() {
            Ok(index) => index,
            Err(e) => {
                eprintln!("Failed to begin frame: {}", e);
                return;
            }
        };
        
        // Check if we have meshes with transforms - if so, use multi-mesh rendering
        if !self.meshes.is_empty() && self.meshes.iter().any(|m| !m.transforms.is_empty()) {
            self.record_command_buffer_multi_mesh_with_egui(image_index, view, proj, egui_output);
        } else {
            // Use the old function for backwards compatibility
            self.record_command_buffer_with_push_data_and_egui(image_index, view, proj, egui_output);
        }
        
        if let Err(e) = self.core.end_frame(image_index) {
            eprintln!("Failed to end frame: {}", e);
        }
    }
    
    // Update egui swapchain when window resizes
    pub fn update_egui_swapchain(&mut self, width: u32, height: u32) {
        if let Some(egui_integration) = &mut self.egui_integration {
            egui_integration.update_swapchain(width, height);
        }
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.core.device.device_wait_idle();
            
            // Clean up egui integration
            if let Some(mut egui_integration) = self.egui_integration.take() {
                egui_integration.cleanup();
            }
            
            // Clean up texture resources
            if let Some(ref textures) = self.textures {
                self.core.device.destroy_sampler(textures.sampler, None);
                self.core.device.destroy_image_view(textures.image_view, None);
                self.core.device.destroy_image(textures.image, None);
                self.core.device.free_memory(textures.image_memory, None);
                self.core.device.destroy_descriptor_pool(textures.descriptor_pool, None);
                self.core.device.destroy_descriptor_set_layout(textures.descriptor_set_layout, None);
            }
            
            // Clean up texture array resources
            if let Some(ref texture_arrays) = self.texture_arrays {
                self.core.device.destroy_sampler(texture_arrays.texture_sampler, None);
                self.core.device.destroy_image_view(texture_arrays.texture_array_view, None);
                self.core.device.destroy_image(texture_arrays.texture_array, None);
                self.core.device.free_memory(texture_arrays.texture_array_memory, None);
                self.core.device.destroy_descriptor_pool(texture_arrays.descriptor_pool, None);
                self.core.device.destroy_descriptor_set_layout(texture_arrays.descriptor_set_layout, None);
            }
            
            // Clean up buffer resources
            if let Some(ref buffers) = self.buffers {
                self.core.device.destroy_buffer(buffers.vertex_buffer, None);
                self.core.device.free_memory(buffers.vertex_buffer_memory, None);
                
                if let Some(index_buffer) = buffers.index_buffer {
                    self.core.device.destroy_buffer(index_buffer, None);
                }
                if let Some(index_memory) = buffers.index_buffer_memory {
                    self.core.device.free_memory(index_memory, None);
                }
                
                if let Some(instance_buffer) = buffers.instance_buffer {
                    self.core.device.destroy_buffer(instance_buffer, None);
                }
                if let Some(instance_memory) = buffers.instance_buffer_memory {
                    self.core.device.free_memory(instance_memory, None);
                }
            }
            
            // Clean up mesh resources
            for mesh in &self.meshes {
                self.core.device.destroy_buffer(mesh.vertex_buffer, None);
                // Only free memory if not using pooled memory
                if let Some(memory) = mesh.vertex_buffer_memory {
                    self.core.device.free_memory(memory, None);
                }
                
                self.core.device.destroy_buffer(mesh.index_buffer, None);
                // Only free memory if not using pooled memory
                if let Some(memory) = mesh.index_buffer_memory {
                    self.core.device.free_memory(memory, None);
                }
                
                if let Some(instance_buffer) = mesh.instance_buffer {
                    self.core.device.destroy_buffer(instance_buffer, None);
                }
                if let Some(memory) = mesh.instance_buffer_memory {
                    self.core.device.free_memory(memory, None);
                }
                
                // Clean up joint buffer for skinned meshes
                if let Some(joint_buffer) = mesh.joint_buffer {
                    self.core.device.destroy_buffer(joint_buffer, None);
                }
                if let Some(memory) = mesh.joint_buffer_memory {
                    self.core.device.free_memory(memory, None);
                }
                
                // Clean up skinned mesh descriptor resources
                if let Some(camera_buffer) = mesh.camera_uniform_buffer {
                    self.core.device.destroy_buffer(camera_buffer, None);
                }
                if let Some(memory) = mesh.camera_uniform_memory {
                    self.core.device.free_memory(memory, None);
                }
                if let Some(pool) = mesh.skinned_descriptor_pool {
                    self.core.device.destroy_descriptor_pool(pool, None);
                }
                if let Some(layout) = mesh.skinned_descriptor_set_layout {
                    self.core.device.destroy_descriptor_set_layout(layout, None);
                }
            }
            
            // Clean up textured pipeline resources
            for (_, resources) in self.textured_pipelines.drain() {
                self.core.device.destroy_descriptor_pool(resources.descriptor_pool, None);
                self.core.device.destroy_descriptor_set_layout(resources.descriptor_set_layout, None);
            }
            
            // Clean up memory pool
            self.memory_pool.destroy();
            
            // Clean up pipeline
            destroy_pipeline(&self.core.device, self.graphics_pipeline, self.pipeline_layout);
        }
    }
}

// Helper struct for push constants
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct MvpPushConstants {
    model: [f32; 16],
    view: [f32; 16],
    proj: [f32; 16],
    base_color: [f32; 4], // Added base color for material-specific coloring
}
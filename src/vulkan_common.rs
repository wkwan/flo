use ash::{vk, Instance, Entry};
use ash::khr;
use std::collections::HashSet;
use std::mem;
use std::time::Instant;
use std::ffi::CString;
use bevy::window::RawHandleWrapperHolder;

use crate::constants::*;
use crate::memory_pool::{MemoryPoolManager, MemoryBlock};

pub struct QueueFamilyIndices {
    pub graphics_family: Option<u32>,
    pub present_family: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}

pub fn find_queue_families(
    instance: &Instance,
    surface_loader: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
    device: vk::PhysicalDevice,
) -> QueueFamilyIndices {
    let queue_families = unsafe { instance.get_physical_device_queue_family_properties(device) };
    
    let mut indices = QueueFamilyIndices {
        graphics_family: None,
        present_family: None,
    };
    
    for (i, queue_family) in queue_families.iter().enumerate() {
        if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            indices.graphics_family = Some(i as u32);
        }
        
        let present_support = unsafe {
            surface_loader
                .get_physical_device_surface_support(device, i as u32, surface)
                .unwrap_or(false)
        };
        
        if present_support {
            indices.present_family = Some(i as u32);
        }
        
        if indices.is_complete() {
            break;
        }
    }
    
    indices
}

pub fn pick_physical_device(
    instance: &Instance,
    surface_loader: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<(vk::PhysicalDevice, QueueFamilyIndices), Box<dyn std::error::Error>> {
    let devices = unsafe { instance.enumerate_physical_devices()? };
    
    for device in devices {
        let indices = find_queue_families(instance, surface_loader, surface, device);
        if indices.is_complete() {
            return Ok((device, indices));
        }
    }
    
    Err("Failed to find suitable GPU".into())
}

pub fn create_logical_device(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    indices: &QueueFamilyIndices,
    enable_anisotropy: bool,
) -> Result<ash::Device, Box<dyn std::error::Error>> {
    let mut unique_queue_families = HashSet::new();
    unique_queue_families.insert(indices.graphics_family.unwrap());
    unique_queue_families.insert(indices.present_family.unwrap());
    
    let queue_priorities = vec![1.0];
    let mut queue_create_infos = vec![];
    
    for queue_family in unique_queue_families {
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family)
            .queue_priorities(&queue_priorities);
        queue_create_infos.push(queue_create_info);
    }
    
    let mut device_features = vk::PhysicalDeviceFeatures::default();
    if enable_anisotropy {
        device_features = device_features.sampler_anisotropy(true);
    }
    
    let device_extensions = vec![khr::swapchain::NAME.as_ptr()];
    
    let create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_features(&device_features)
        .enabled_extension_names(&device_extensions);
    
    let device = unsafe { instance.create_device(physical_device, &create_info, None)? };
    
    Ok(device)
}

pub fn create_swapchain(
    _instance: &Instance,
    surface_loader: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    swapchain_loader: &khr::swapchain::Device,
    indices: &QueueFamilyIndices,
) -> Result<(vk::SwapchainKHR, Vec<vk::Image>, vk::Format, vk::Extent2D), Box<dyn std::error::Error>> {
    let capabilities = unsafe {
        surface_loader.get_physical_device_surface_capabilities(physical_device, surface)?
    };
    
    let formats = unsafe {
        surface_loader.get_physical_device_surface_formats(physical_device, surface)?
    };
    
    let present_modes = unsafe {
        surface_loader.get_physical_device_surface_present_modes(physical_device, surface)?
    };
    
    let surface_format = formats
        .iter()
        .find(|f| f.format == vk::Format::B8G8R8A8_SRGB && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
        .unwrap_or(&formats[0]);
    
    // Try to use MAILBOX (triple buffering) for best performance without tearing
    // Falls back to IMMEDIATE (no v-sync) if MAILBOX not available
    // Falls back to FIFO (v-sync) as last resort since it's always available
    let present_mode = present_modes
        .iter()
        .find(|&&mode| mode == vk::PresentModeKHR::MAILBOX)
        .or_else(|| present_modes.iter().find(|&&mode| mode == vk::PresentModeKHR::IMMEDIATE))
        .unwrap_or(&vk::PresentModeKHR::FIFO);
    
    println!("Selected present mode: {:?}", present_mode);
    
    let extent = capabilities.current_extent;
    
    let image_count = (capabilities.min_image_count + 1).min(
        if capabilities.max_image_count > 0 { capabilities.max_image_count } else { u32::MAX }
    );
    
    let mut create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT);
    
    let queue_family_indices = [indices.graphics_family.unwrap(), indices.present_family.unwrap()];
    
    if indices.graphics_family != indices.present_family {
        create_info = create_info
            .image_sharing_mode(vk::SharingMode::CONCURRENT)
            .queue_family_indices(&queue_family_indices);
    } else {
        create_info = create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE);
    }
    
    create_info = create_info
        .pre_transform(capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(*present_mode)
        .clipped(true);
    
    let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None)? };
    let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };
    
    Ok((swapchain, swapchain_images, surface_format.format, extent))
}

pub fn create_image_views(
    device: &ash::Device,
    swapchain_images: &[vk::Image],
    swapchain_format: vk::Format,
) -> Result<Vec<vk::ImageView>, Box<dyn std::error::Error>> {
    let mut image_views = Vec::with_capacity(swapchain_images.len());
    
    for &image in swapchain_images {
        let create_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(swapchain_format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        
        let image_view = unsafe { device.create_image_view(&create_info, None)? };
        image_views.push(image_view);
    }
    
    Ok(image_views)
}

pub fn find_depth_format(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<vk::Format, Box<dyn std::error::Error>> {
    let candidates = vec![
        vk::Format::D32_SFLOAT,
        vk::Format::D32_SFLOAT_S8_UINT,
        vk::Format::D24_UNORM_S8_UINT,
    ];
    
    for format in candidates {
        let props = unsafe {
            instance.get_physical_device_format_properties(physical_device, format)
        };
        
        if props.optimal_tiling_features.contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT) {
            return Ok(format);
        }
    }
    
    Err("Failed to find supported depth format".into())
}

pub fn create_depth_resources(
    instance: &Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    extent: vk::Extent2D,
) -> Result<(vk::Image, vk::DeviceMemory, vk::ImageView), Box<dyn std::error::Error>> {
    let depth_format = find_depth_format(instance, physical_device)?;
    
    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D {
            width: extent.width,
            height: extent.height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .format(depth_format)
        .tiling(vk::ImageTiling::OPTIMAL)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(vk::SampleCountFlags::TYPE_1);
    
    let image = unsafe { device.create_image(&image_info, None)? };
    
    let mem_requirements = unsafe { device.get_image_memory_requirements(image) };
    
    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_requirements.size)
        .memory_type_index(find_memory_type(
            instance,
            physical_device,
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?);
    
    let image_memory = unsafe { device.allocate_memory(&alloc_info, None)? };
    
    unsafe { device.bind_image_memory(image, image_memory, 0)? };
    
    let view_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(depth_format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::DEPTH,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });
    
    let image_view = unsafe { device.create_image_view(&view_info, None)? };
    
    Ok((image, image_memory, image_view))
}

pub fn find_memory_type(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> Result<u32, Box<dyn std::error::Error>> {
    let mem_properties = unsafe { instance.get_physical_device_memory_properties(physical_device) };
    
    for i in 0..mem_properties.memory_type_count {
        if (type_filter & (1 << i)) != 0 &&
           mem_properties.memory_types[i as usize].property_flags.contains(properties) {
            return Ok(i);
        }
    }
    
    Err("Failed to find suitable memory type".into())
}

pub fn create_framebuffers(
    device: &ash::Device,
    image_views: &[vk::ImageView],
    depth_image_view: vk::ImageView,
    render_pass: vk::RenderPass,
    extent: vk::Extent2D,
) -> Result<Vec<vk::Framebuffer>, Box<dyn std::error::Error>> {
    let mut framebuffers = Vec::with_capacity(image_views.len());
    
    for &image_view in image_views {
        let attachments = [image_view, depth_image_view];
        
        let framebuffer_info = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);
        
        let framebuffer = unsafe { device.create_framebuffer(&framebuffer_info, None)? };
        framebuffers.push(framebuffer);
    }
    
    Ok(framebuffers)
}

pub fn create_command_pool(
    device: &ash::Device,
    queue_family_index: u32,
) -> Result<vk::CommandPool, Box<dyn std::error::Error>> {
    let pool_info = vk::CommandPoolCreateInfo::default()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(queue_family_index);
    
    let command_pool = unsafe { device.create_command_pool(&pool_info, None)? };
    
    Ok(command_pool)
}

pub fn create_command_buffers(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    count: usize,
) -> Result<Vec<vk::CommandBuffer>, Box<dyn std::error::Error>> {
    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(count as u32);
    
    let command_buffers = unsafe { device.allocate_command_buffers(&alloc_info)? };
    
    Ok(command_buffers)
}

pub fn create_sync_objects(
    device: &ash::Device,
) -> Result<(Vec<vk::Semaphore>, Vec<vk::Semaphore>, Vec<vk::Fence>), Box<dyn std::error::Error>> {
    let semaphore_info = vk::SemaphoreCreateInfo::default();
    let fence_info = vk::FenceCreateInfo::default()
        .flags(vk::FenceCreateFlags::SIGNALED);
    
    let mut image_available_semaphores = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
    let mut render_finished_semaphores = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
    let mut in_flight_fences = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
    
    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        image_available_semaphores.push(unsafe { device.create_semaphore(&semaphore_info, None)? });
        render_finished_semaphores.push(unsafe { device.create_semaphore(&semaphore_info, None)? });
        in_flight_fences.push(unsafe { device.create_fence(&fence_info, None)? });
    }
    
    Ok((image_available_semaphores, render_finished_semaphores, in_flight_fences))
}

pub fn create_shader_module(device: &ash::Device, code: &[u8]) -> Result<vk::ShaderModule, Box<dyn std::error::Error>> {
    let code_u32: Vec<u32> = code.chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();
    
    let create_info = vk::ShaderModuleCreateInfo::default()
        .code(&code_u32);
    
    let shader_module = unsafe { device.create_shader_module(&create_info, None)? };
    
    Ok(shader_module)
}

pub fn create_buffer(
    instance: &Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory), Box<dyn std::error::Error>> {
    let buffer_info = vk::BufferCreateInfo::default()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    
    let buffer = unsafe { device.create_buffer(&buffer_info, None)? };
    
    let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
    
    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_requirements.size)
        .memory_type_index(find_memory_type(
            instance,
            physical_device,
            mem_requirements.memory_type_bits,
            properties,
        )?);
    
    let buffer_memory = unsafe { device.allocate_memory(&alloc_info, None)? };
    
    unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0)? };
    
    Ok((buffer, buffer_memory))
}

pub fn copy_buffer(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    src_buffer: vk::Buffer,
    dst_buffer: vk::Buffer,
    size: vk::DeviceSize,
) -> Result<(), Box<dyn std::error::Error>> {
    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(command_pool)
        .command_buffer_count(1);
    
    let command_buffer = unsafe { device.allocate_command_buffers(&alloc_info)?[0] };
    
    let begin_info = vk::CommandBufferBeginInfo::default()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    
    unsafe {
        device.begin_command_buffer(command_buffer, &begin_info)?;
        
        let copy_region = vk::BufferCopy::default()
            .size(size);
        
        device.cmd_copy_buffer(command_buffer, src_buffer, dst_buffer, &[copy_region]);
        
        device.end_command_buffer(command_buffer)?;
        
        let command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfo::default()
            .command_buffers(&command_buffers);
        
        device.queue_submit(queue, &[submit_info], vk::Fence::null())?;
        device.queue_wait_idle(queue)?;
        
        device.free_command_buffers(command_pool, &[command_buffer]);
    }
    
    Ok(())
}

pub fn create_vertex_buffer<T: Copy>(
    instance: &Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    vertices: &[T],
) -> Result<(vk::Buffer, vk::DeviceMemory), Box<dyn std::error::Error>> {
    let buffer_size = (mem::size_of::<T>() * vertices.len()) as vk::DeviceSize;
    
    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    
    unsafe {
        let data = device.map_memory(staging_buffer_memory, 0, buffer_size, vk::MemoryMapFlags::empty())?;
        std::ptr::copy_nonoverlapping(vertices.as_ptr() as *const u8, data as *mut u8, buffer_size as usize);
        device.unmap_memory(staging_buffer_memory);
    }
    
    let (vertex_buffer, vertex_buffer_memory) = create_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    
    copy_buffer(device, command_pool, queue, staging_buffer, vertex_buffer, buffer_size)?;
    
    unsafe {
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_buffer_memory, None);
    }
    
    Ok((vertex_buffer, vertex_buffer_memory))
}

pub fn create_index_buffer(
    instance: &Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    indices: &[u32],
) -> Result<(vk::Buffer, vk::DeviceMemory), Box<dyn std::error::Error>> {
    let buffer_size = (mem::size_of::<u32>() * indices.len()) as vk::DeviceSize;
    
    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    
    unsafe {
        let data = device.map_memory(staging_buffer_memory, 0, buffer_size, vk::MemoryMapFlags::empty())?;
        std::ptr::copy_nonoverlapping(indices.as_ptr() as *const u8, data as *mut u8, buffer_size as usize);
        device.unmap_memory(staging_buffer_memory);
    }
    
    let (index_buffer, index_buffer_memory) = create_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    
    copy_buffer(device, command_pool, queue, staging_buffer, index_buffer, buffer_size)?;
    
    unsafe {
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_buffer_memory, None);
    }
    
    Ok((index_buffer, index_buffer_memory))
}

// Pooled buffer creation functions
pub fn create_buffer_pooled(
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    instance: &Instance,
    memory_pool: &mut MemoryPoolManager,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, MemoryBlock), Box<dyn std::error::Error>> {
    let buffer_info = vk::BufferCreateInfo::default()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    
    let buffer = unsafe { device.create_buffer(&buffer_info, None)? };
    
    let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
    
    let memory_type_index = find_memory_type(
        instance,
        physical_device,
        mem_requirements.memory_type_bits,
        properties,
    )?;
    
    let memory_block = memory_pool.allocate_buffer(buffer, mem_requirements, memory_type_index)?;
    
    Ok((buffer, memory_block))
}

pub fn create_vertex_buffer_pooled<T: Copy>(
    instance: &Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    memory_pool: &mut MemoryPoolManager,
    vertices: &[T],
) -> Result<(vk::Buffer, MemoryBlock), Box<dyn std::error::Error>> {
    let buffer_size = (mem::size_of::<T>() * vertices.len()) as vk::DeviceSize;
    
    // Use reusable staging buffer from pool
    let (staging_buffer, staging_buffer_memory) = memory_pool.get_staging_buffer(
        instance,
        physical_device,
        buffer_size,
    )?;
    
    unsafe {
        let data = device.map_memory(staging_buffer_memory, 0, buffer_size, vk::MemoryMapFlags::empty())?;
        std::ptr::copy_nonoverlapping(vertices.as_ptr() as *const u8, data as *mut u8, buffer_size as usize);
        device.unmap_memory(staging_buffer_memory);
    }
    
    // Create vertex buffer using memory pool
    let (vertex_buffer, memory_block) = create_buffer_pooled(
        device,
        physical_device,
        instance,
        memory_pool,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    
    copy_buffer(device, command_pool, queue, staging_buffer, vertex_buffer, buffer_size)?;
    
    // Don't destroy staging buffer - it's reused!
    
    Ok((vertex_buffer, memory_block))
}

pub fn create_index_buffer_pooled(
    instance: &Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    memory_pool: &mut MemoryPoolManager,
    indices: &[u32],
) -> Result<(vk::Buffer, MemoryBlock), Box<dyn std::error::Error>> {
    let buffer_size = (mem::size_of::<u32>() * indices.len()) as vk::DeviceSize;
    
    // Use reusable staging buffer from pool
    let (staging_buffer, staging_buffer_memory) = memory_pool.get_staging_buffer(
        instance,
        physical_device,
        buffer_size,
    )?;
    
    unsafe {
        let data = device.map_memory(staging_buffer_memory, 0, buffer_size, vk::MemoryMapFlags::empty())?;
        std::ptr::copy_nonoverlapping(indices.as_ptr() as *const u8, data as *mut u8, buffer_size as usize);
        device.unmap_memory(staging_buffer_memory);
    }
    
    // Create index buffer using memory pool
    let (index_buffer, memory_block) = create_buffer_pooled(
        device,
        physical_device,
        instance,
        memory_pool,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    
    copy_buffer(device, command_pool, queue, staging_buffer, index_buffer, buffer_size)?;
    
    // Don't destroy staging buffer - it's reused!
    
    Ok((index_buffer, memory_block))
}

pub fn record_command_buffer_simple(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    extent: vk::Extent2D,
    graphics_pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    start_time: Instant,
    vertex_count: u32,
) {
    let begin_info = vk::CommandBufferBeginInfo::default();
    
    unsafe {
        device
            .begin_command_buffer(command_buffer, &begin_info)
            .expect("Failed to begin recording command buffer");
        
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: CLEAR_COLOR_MAGENTA,
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: DEPTH_CLEAR_VALUE,
                    stencil: STENCIL_CLEAR_VALUE,
                },
            },
        ];
        
        let render_pass_info = vk::RenderPassBeginInfo::default()
            .render_pass(render_pass)
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            })
            .clear_values(&clear_values);
        
        device.cmd_begin_render_pass(command_buffer, &render_pass_info, vk::SubpassContents::INLINE);
        
        if graphics_pipeline != vk::Pipeline::null() {
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, graphics_pipeline);
            
            let elapsed = start_time.elapsed().as_secs_f32();
            let time_data = [elapsed];
            let time_bytes = bytemuck::cast_slice(&time_data);
            
            device.cmd_push_constants(
                command_buffer,
                pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                time_bytes,
            );
            
            device.cmd_draw(command_buffer, vertex_count, 1, 0, 0);
        }
        
        device.cmd_end_render_pass(command_buffer);
        
        device
            .end_command_buffer(command_buffer)
            .expect("Failed to record command buffer");
    }
}

pub fn record_command_buffer_indexed(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    extent: vk::Extent2D,
    graphics_pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    index_count: u32,
    start_time: Instant,
) {
    let begin_info = vk::CommandBufferBeginInfo::default();
    
    unsafe {
        device
            .begin_command_buffer(command_buffer, &begin_info)
            .expect("Failed to begin recording command buffer");
        
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: CLEAR_COLOR_DEFAULT,
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: DEPTH_CLEAR_VALUE,
                    stencil: STENCIL_CLEAR_VALUE,
                },
            },
        ];
        
        let render_pass_info = vk::RenderPassBeginInfo::default()
            .render_pass(render_pass)
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            })
            .clear_values(&clear_values);
        
        device.cmd_begin_render_pass(command_buffer, &render_pass_info, vk::SubpassContents::INLINE);
        
        if graphics_pipeline != vk::Pipeline::null() {
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, graphics_pipeline);
            
            let vertex_buffers = [vertex_buffer];
            let offsets = [0];
            device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);
            
            let elapsed = start_time.elapsed().as_secs_f32();
            let time_data = [elapsed];
            let time_bytes = bytemuck::cast_slice(&time_data);
            
            device.cmd_push_constants(
                command_buffer,
                pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                time_bytes,
            );
            
            device.cmd_draw_indexed(command_buffer, index_count, 1, 0, 0, 0);
        }
        
        device.cmd_end_render_pass(command_buffer);
        
        device
            .end_command_buffer(command_buffer)
            .expect("Failed to record command buffer");
    }
}

pub struct VulkanCore {
    pub _entry: Entry,
    pub instance: Instance,
    pub surface: vk::SurfaceKHR,
    pub surface_loader: khr::surface::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_loader: khr::swapchain::Device,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,
    pub swapchain_image_views: Vec<vk::ImageView>,
    pub depth_image: vk::Image,
    pub depth_image_memory: vk::DeviceMemory,
    pub depth_image_view: vk::ImageView,
    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub current_frame: usize,
    pub start_time: Instant,
    pub queue_family_indices: QueueFamilyIndices,
}

impl VulkanCore {
    pub fn new(
        handle_wrapper: &RawHandleWrapperHolder,
        with_depth: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let entry = unsafe { Entry::load() }.expect("Failed to load Vulkan entry");
        
        let raw_handle = handle_wrapper.0.lock().unwrap();
        let raw_handle_ref = raw_handle.as_ref().expect("Window handle not available");
        
        let display_handle = raw_handle_ref.get_display_handle();
        let window_handle = raw_handle_ref.get_window_handle();
        
        let app_name = CString::new("Vulkan Bevy Renderer")?;
        let engine_name = CString::new("No Engine")?;
        
        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_3);
        
        let mut extensions = ash_window::enumerate_required_extensions(display_handle)?.to_vec();
        extensions.push(khr::surface::NAME.as_ptr());
        
        let layer_names: Vec<CString> = if ENABLE_VALIDATION_LAYERS {
            vec![CString::new("VK_LAYER_KHRONOS_validation")?]
        } else {
            vec![]
        };
        let layer_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();
        
        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layer_names_raw);
        
        let instance = unsafe { entry.create_instance(&create_info, None)? };
        
        let surface = unsafe {
            ash_window::create_surface(&entry, &instance, display_handle, window_handle, None)?
        };
        let surface_loader = khr::surface::Instance::new(&entry, &instance);
        
        let (physical_device, indices) = pick_physical_device(&instance, &surface_loader, surface)?;
        let device = create_logical_device(&instance, physical_device, &indices, false)?;
        
        let graphics_queue = unsafe { device.get_device_queue(indices.graphics_family.unwrap(), 0) };
        let present_queue = unsafe { device.get_device_queue(indices.present_family.unwrap(), 0) };
        
        let swapchain_loader = khr::swapchain::Device::new(&instance, &device);
        let (swapchain, swapchain_images, swapchain_format, swapchain_extent) = 
            create_swapchain(&instance, &surface_loader, surface, physical_device, &swapchain_loader, &indices)?;
        let swapchain_image_views = create_image_views(&device, &swapchain_images, swapchain_format)?;
        
        let (depth_image, depth_image_memory, depth_image_view) = if with_depth {
            create_depth_resources(&instance, &device, physical_device, swapchain_extent)?
        } else {
            (vk::Image::null(), vk::DeviceMemory::null(), vk::ImageView::null())
        };
        
        let render_pass = create_render_pass(&instance, &device, physical_device, swapchain_format, with_depth)?;
        
        let framebuffers = if with_depth {
            create_framebuffers(&device, &swapchain_image_views, depth_image_view, render_pass, swapchain_extent)?
        } else {
            create_framebuffers_no_depth(&device, &swapchain_image_views, render_pass, swapchain_extent)?
        };
        
        let command_pool = create_command_pool(&device, indices.graphics_family.unwrap())?;
        let command_buffers = create_command_buffers(&device, command_pool, swapchain_images.len())?;
        
        let (image_available_semaphores, render_finished_semaphores, in_flight_fences) = 
            create_sync_objects(&device)?;
        
        Ok(Self {
            _entry: entry,
            instance,
            surface,
            surface_loader,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swapchain,
            swapchain_loader,
            swapchain_images,
            swapchain_format,
            swapchain_extent,
            swapchain_image_views,
            depth_image,
            depth_image_memory,
            depth_image_view,
            render_pass,
            framebuffers,
            command_pool,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            current_frame: 0,
            start_time: Instant::now(),
            queue_family_indices: indices,
        })
    }
    
    pub fn begin_frame(&mut self) -> Result<u32, Box<dyn std::error::Error>> {
        unsafe {
            self.device.wait_for_fences(
                &[self.in_flight_fences[self.current_frame]], 
                true, 
                u64::MAX
            )?;
            
            let (image_index, _) = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null(),
            )?;
            
            self.device.reset_fences(&[self.in_flight_fences[self.current_frame]])?;
            self.device.reset_command_buffer(
                self.command_buffers[image_index as usize],
                vk::CommandBufferResetFlags::empty(),
            )?;
            
            Ok(image_index)
        }
    }
    
    pub fn end_frame(&mut self, image_index: u32) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = [self.command_buffers[image_index as usize]];
            let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];
            
            let submit_info = vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores);
            
            self.device.queue_submit(
                self.graphics_queue,
                &[submit_info],
                self.in_flight_fences[self.current_frame],
            )?;
            
            let swapchains = [self.swapchain];
            let image_indices = [image_index];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);
            
            self.swapchain_loader.queue_present(self.present_queue, &present_info)?;
            
            self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        }
        
        Ok(())
    }
    
    pub fn get_elapsed_time(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
    }
}

impl Drop for VulkanCore {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            
            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.device.destroy_semaphore(self.image_available_semaphores[i], None);
                self.device.destroy_semaphore(self.render_finished_semaphores[i], None);
                self.device.destroy_fence(self.in_flight_fences[i], None);
            }
            
            self.device.destroy_command_pool(self.command_pool, None);
            
            for &framebuffer in &self.framebuffers {
                self.device.destroy_framebuffer(framebuffer, None);
            }
            
            self.device.destroy_render_pass(self.render_pass, None);
            
            if self.depth_image_view != vk::ImageView::null() {
                self.device.destroy_image_view(self.depth_image_view, None);
                self.device.destroy_image(self.depth_image, None);
                self.device.free_memory(self.depth_image_memory, None);
            }
            
            for &image_view in &self.swapchain_image_views {
                self.device.destroy_image_view(image_view, None);
            }
            
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}

// Helper functions for common cleanup patterns
pub fn destroy_buffer(device: &ash::Device, buffer: vk::Buffer, memory: vk::DeviceMemory) {
    unsafe {
        device.destroy_buffer(buffer, None);
        device.free_memory(memory, None);
    }
}

pub fn destroy_image(device: &ash::Device, image: vk::Image, memory: vk::DeviceMemory, view: vk::ImageView) {
    unsafe {
        device.destroy_image_view(view, None);
        device.destroy_image(image, None);
        device.free_memory(memory, None);
    }
}

pub fn destroy_pipeline(device: &ash::Device, pipeline: vk::Pipeline, layout: vk::PipelineLayout) {
    unsafe {
        device.destroy_pipeline(pipeline, None);
        device.destroy_pipeline_layout(layout, None);
    }
}

pub fn wait_and_destroy_pipeline(device: &ash::Device, pipeline: vk::Pipeline, layout: vk::PipelineLayout) {
    unsafe {
        let _ = device.device_wait_idle();
        destroy_pipeline(device, pipeline, layout);
    }
}

// Descriptor set helper functions
pub fn create_descriptor_pool(
    device: &ash::Device,
    max_sets: u32,
    pool_sizes: &[vk::DescriptorPoolSize],
) -> Result<vk::DescriptorPool, Box<dyn std::error::Error>> {
    let pool_info = vk::DescriptorPoolCreateInfo::default()
        .pool_sizes(pool_sizes)
        .max_sets(max_sets);
    
    let descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };
    Ok(descriptor_pool)
}

pub fn create_descriptor_set_layout(
    device: &ash::Device,
    bindings: &[vk::DescriptorSetLayoutBinding],
) -> Result<vk::DescriptorSetLayout, Box<dyn std::error::Error>> {
    let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
        .bindings(bindings);
    
    let set_layout = unsafe { device.create_descriptor_set_layout(&layout_info, None)? };
    Ok(set_layout)
}

pub fn allocate_descriptor_sets(
    device: &ash::Device,
    descriptor_pool: vk::DescriptorPool,
    set_layouts: &[vk::DescriptorSetLayout],
) -> Result<Vec<vk::DescriptorSet>, Box<dyn std::error::Error>> {
    let alloc_info = vk::DescriptorSetAllocateInfo::default()
        .descriptor_pool(descriptor_pool)
        .set_layouts(set_layouts);
    
    let descriptor_sets = unsafe { device.allocate_descriptor_sets(&alloc_info)? };
    Ok(descriptor_sets)
}

pub fn update_descriptor_sets_texture(
    device: &ash::Device,
    descriptor_set: vk::DescriptorSet,
    image_view: vk::ImageView,
    sampler: vk::Sampler,
    binding: u32,
) {
    let image_info = vk::DescriptorImageInfo::default()
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .image_view(image_view)
        .sampler(sampler);
    
    let image_infos = [image_info];
    let descriptor_write = vk::WriteDescriptorSet::default()
        .dst_set(descriptor_set)
        .dst_binding(binding)
        .dst_array_element(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .image_info(&image_infos);
    
    unsafe {
        device.update_descriptor_sets(&[descriptor_write], &[]);
    }
}

pub fn create_framebuffers_no_depth(
    device: &ash::Device,
    image_views: &[vk::ImageView],
    render_pass: vk::RenderPass,
    extent: vk::Extent2D,
) -> Result<Vec<vk::Framebuffer>, Box<dyn std::error::Error>> {
    let mut framebuffers = Vec::with_capacity(image_views.len());
    
    for &image_view in image_views {
        let attachments = [image_view];
        
        let framebuffer_info = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);
        
        let framebuffer = unsafe { device.create_framebuffer(&framebuffer_info, None)? };
        framebuffers.push(framebuffer);
    }
    
    Ok(framebuffers)
}

pub fn create_render_pass(
    instance: &Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    swapchain_format: vk::Format,
    with_depth: bool,
) -> Result<vk::RenderPass, Box<dyn std::error::Error>> {
    let color_attachment = vk::AttachmentDescription::default()
        .format(swapchain_format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);
    
    let color_attachment_ref = vk::AttachmentReference::default()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
    
    let color_attachment_refs = [color_attachment_ref];
    
    let mut attachments = vec![color_attachment];
    let mut subpass_builder = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachment_refs);
    
    let depth_attachment_ref;
    if with_depth {
        let depth_format = find_depth_format(instance, physical_device)?;
        let depth_attachment = vk::AttachmentDescription::default()
            .format(depth_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
        
        attachments.push(depth_attachment);
        
        depth_attachment_ref = vk::AttachmentReference::default()
            .attachment(1)
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
        
        subpass_builder = subpass_builder.depth_stencil_attachment(&depth_attachment_ref);
    }
    
    let subpass = subpass_builder;
    
    let mut dependency = vk::SubpassDependency::default()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);
    
    if with_depth {
        dependency = dependency
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE);
    }
    
    let subpasses = [subpass];
    let dependencies = [dependency];
    let render_pass_info = vk::RenderPassCreateInfo::default()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);
    
    let render_pass = unsafe { device.create_render_pass(&render_pass_info, None)? };
    
    Ok(render_pass)
}


pub struct PipelineBuilder {
    device: ash::Device,
    vert_shader_code: Vec<u8>,
    frag_shader_code: Vec<u8>,
    vertex_binding_descriptions: Vec<vk::VertexInputBindingDescription>,
    vertex_attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
    push_constant_ranges: Vec<vk::PushConstantRange>,
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    extent: vk::Extent2D,
    render_pass: vk::RenderPass,
    with_depth_test: bool,
    cull_mode: vk::CullModeFlags,
    front_face: vk::FrontFace,
    polygon_mode: vk::PolygonMode,
    with_alpha_blending: bool,
}

impl PipelineBuilder {
    pub fn new(
        device: ash::Device,
        vert_shader_path: &str,
        frag_shader_path: &str,
        extent: vk::Extent2D,
        render_pass: vk::RenderPass,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let vert_shader_code = std::fs::read(vert_shader_path)?;
        let frag_shader_code = std::fs::read(frag_shader_path)?;
        
        Ok(Self {
            device,
            vert_shader_code,
            frag_shader_code,
            vertex_binding_descriptions: Vec::new(),
            vertex_attribute_descriptions: Vec::new(),
            push_constant_ranges: Vec::new(),
            descriptor_set_layouts: Vec::new(),
            extent,
            render_pass,
            with_depth_test: false,
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            polygon_mode: vk::PolygonMode::FILL,
            with_alpha_blending: false,
        })
    }
    
    pub fn with_vertex_input(
        mut self,
        binding_descriptions: Vec<vk::VertexInputBindingDescription>,
        attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
    ) -> Self {
        self.vertex_binding_descriptions = binding_descriptions;
        self.vertex_attribute_descriptions = attribute_descriptions;
        self
    }
    
    pub fn with_push_constants(mut self, ranges: Vec<vk::PushConstantRange>) -> Self {
        self.push_constant_ranges = ranges;
        self
    }
    
    pub fn with_descriptor_sets(mut self, layouts: Vec<vk::DescriptorSetLayout>) -> Self {
        self.descriptor_set_layouts = layouts;
        self
    }
    
    pub fn with_depth_test(mut self, enable: bool) -> Self {
        self.with_depth_test = enable;
        self
    }
    
    pub fn with_cull_mode(mut self, mode: vk::CullModeFlags) -> Self {
        self.cull_mode = mode;
        self
    }
    
    pub fn with_front_face(mut self, face: vk::FrontFace) -> Self {
        self.front_face = face;
        self
    }
    
    pub fn with_polygon_mode(mut self, mode: vk::PolygonMode) -> Self {
        self.polygon_mode = mode;
        self
    }
    
    pub fn with_alpha_blending(mut self, enable: bool) -> Self {
        self.with_alpha_blending = enable;
        self
    }
    
    pub fn build(self) -> Result<(vk::Pipeline, vk::PipelineLayout), Box<dyn std::error::Error>> {
        unsafe {
            let vert_shader_module = create_shader_module(&self.device, &self.vert_shader_code)?;
            let frag_shader_module = create_shader_module(&self.device, &self.frag_shader_code)?;
            
            let main_name = CString::new("main")?;
            
            let vert_shader_stage_info = vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_shader_module)
                .name(&main_name);
            
            let frag_shader_stage_info = vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_shader_module)
                .name(&main_name);
            
            let shader_stages = [vert_shader_stage_info, frag_shader_stage_info];
            
            let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_binding_descriptions(&self.vertex_binding_descriptions)
                .vertex_attribute_descriptions(&self.vertex_attribute_descriptions);
            
            let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                .primitive_restart_enable(false);
            
            let viewport = vk::Viewport::default()
                .x(0.0)
                .y(0.0)
                .width(self.extent.width as f32)
                .height(self.extent.height as f32)
                .min_depth(0.0)
                .max_depth(1.0);
            
            let scissor = vk::Rect2D::default()
                .offset(vk::Offset2D { x: 0, y: 0 })
                .extent(self.extent);
            
            let viewports = [viewport];
            let scissors = [scissor];
            let viewport_state = vk::PipelineViewportStateCreateInfo::default()
                .viewports(&viewports)
                .scissors(&scissors);
            
            let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .polygon_mode(self.polygon_mode)
                .line_width(1.0)
                .cull_mode(self.cull_mode)
                .front_face(self.front_face)
                .depth_bias_enable(false);
            
            let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
                .sample_shading_enable(false)
                .rasterization_samples(vk::SampleCountFlags::TYPE_1);
            
            let color_blend_attachment = if self.with_alpha_blending {
                vk::PipelineColorBlendAttachmentState::default()
                    .color_write_mask(vk::ColorComponentFlags::RGBA)
                    .blend_enable(true)
                    .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                    .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                    .color_blend_op(vk::BlendOp::ADD)
                    .src_alpha_blend_factor(vk::BlendFactor::ONE)
                    .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                    .alpha_blend_op(vk::BlendOp::ADD)
            } else {
                vk::PipelineColorBlendAttachmentState::default()
                    .color_write_mask(vk::ColorComponentFlags::RGBA)
                    .blend_enable(false)
            };
            
            let attachments = [color_blend_attachment];
            let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
                .logic_op_enable(false)
                .attachments(&attachments);
            
            let depth_stencil = if self.with_depth_test {
                vk::PipelineDepthStencilStateCreateInfo::default()
                    .depth_test_enable(true)
                    .depth_write_enable(true)
                    .depth_compare_op(vk::CompareOp::LESS)
                    .depth_bounds_test_enable(false)
                    .stencil_test_enable(false)
            } else {
                vk::PipelineDepthStencilStateCreateInfo::default()
            };
            
            let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&self.descriptor_set_layouts)
                .push_constant_ranges(&self.push_constant_ranges);
            
            let pipeline_layout = self.device.create_pipeline_layout(&pipeline_layout_info, None)?;
            
            let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
                .stages(&shader_stages)
                .vertex_input_state(&vertex_input_info)
                .input_assembly_state(&input_assembly)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterizer)
                .multisample_state(&multisampling)
                .depth_stencil_state(&depth_stencil)
                .color_blend_state(&color_blending)
                .layout(pipeline_layout)
                .render_pass(self.render_pass)
                .subpass(0);
            
            let pipelines = self.device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            ).map_err(|e| e.1)?;
            
            self.device.destroy_shader_module(vert_shader_module, None);
            self.device.destroy_shader_module(frag_shader_module, None);
            
            Ok((pipelines[0], pipeline_layout))
        }
    }
}

// Unified rendering configuration
pub enum DrawMode {
    Simple { vertex_count: u32 },
    Indexed { index_count: u32 },
    IndexedInstanced { index_count: u32, instance_count: u32 },
}

pub struct RenderConfig<'a> {
    pub draw_mode: DrawMode,
    pub vertex_buffer: Option<vk::Buffer>,
    pub index_buffer: Option<vk::Buffer>,
    pub descriptor_sets: &'a [vk::DescriptorSet],
    pub push_constant_data: Option<&'a [u8]>,
    pub push_constant_stages: vk::ShaderStageFlags,
    pub clear_color: [f32; 4],
}

impl<'a> Default for RenderConfig<'a> {
    fn default() -> Self {
        Self {
            draw_mode: DrawMode::Simple { vertex_count: 3 },
            vertex_buffer: None,
            index_buffer: None,
            descriptor_sets: &[],
            push_constant_data: None,
            push_constant_stages: vk::ShaderStageFlags::VERTEX,
            clear_color: CLEAR_COLOR_MAGENTA,
        }
    }
}

pub fn record_command_buffer_unified(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    extent: vk::Extent2D,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    config: &RenderConfig,
    has_depth: bool,
) {
    let begin_info = vk::CommandBufferBeginInfo::default();
    
    unsafe {
        device
            .begin_command_buffer(command_buffer, &begin_info)
            .expect("Failed to begin recording command buffer");
        
        let mut clear_values = vec![
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: config.clear_color,
                },
            },
        ];
        
        if has_depth {
            clear_values.push(vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: DEPTH_CLEAR_VALUE,
                    stencil: STENCIL_CLEAR_VALUE,
                },
            });
        }
        
        let render_pass_info = vk::RenderPassBeginInfo::default()
            .render_pass(render_pass)
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            })
            .clear_values(&clear_values);
        
        device.cmd_begin_render_pass(command_buffer, &render_pass_info, vk::SubpassContents::INLINE);
        
        if pipeline != vk::Pipeline::null() {
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
            
            // Bind descriptor sets if provided
            if !config.descriptor_sets.is_empty() {
                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline_layout,
                    0,
                    config.descriptor_sets,
                    &[],
                );
            }
            
            // Bind vertex buffer if provided
            if let Some(vertex_buffer) = config.vertex_buffer {
                let vertex_buffers = [vertex_buffer];
                let offsets = [0];
                device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            }
            
            // Bind index buffer if provided
            if let Some(index_buffer) = config.index_buffer {
                device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);
            }
            
            // Push constants if provided
            if let Some(push_data) = config.push_constant_data {
                device.cmd_push_constants(
                    command_buffer,
                    pipeline_layout,
                    config.push_constant_stages,
                    0,
                    push_data,
                );
            }
            
            // Draw based on mode
            match config.draw_mode {
                DrawMode::Simple { vertex_count } => {
                    device.cmd_draw(command_buffer, vertex_count, 1, 0, 0);
                }
                DrawMode::Indexed { index_count } => {
                    device.cmd_draw_indexed(command_buffer, index_count, 1, 0, 0, 0);
                }
                DrawMode::IndexedInstanced { index_count, instance_count } => {
                    device.cmd_draw_indexed(command_buffer, index_count, instance_count, 0, 0, 0);
                }
            }
        }
        
        device.cmd_end_render_pass(command_buffer);
        
        device
            .end_command_buffer(command_buffer)
            .expect("Failed to record command buffer");
    }
}


// Texture array helper functions for VulkanRenderer
pub fn create_textured_vertex_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    vertices: &[crate::mesh_textured::TexturedVertex],
) -> Result<(vk::Buffer, vk::DeviceMemory), Box<dyn std::error::Error>> {
    let buffer_size = (std::mem::size_of::<crate::mesh_textured::TexturedVertex>() * vertices.len()) as vk::DeviceSize;
    
    // Create staging buffer
    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    
    // Copy vertex data to staging buffer
    unsafe {
        let data = device.map_memory(staging_buffer_memory, 0, buffer_size, vk::MemoryMapFlags::empty())?;
        std::ptr::copy_nonoverlapping(vertices.as_ptr() as *const u8, data as *mut u8, buffer_size as usize);
        device.unmap_memory(staging_buffer_memory);
    }
    
    // Create vertex buffer
    let (vertex_buffer, vertex_buffer_memory) = create_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    
    // Copy from staging to vertex buffer
    copy_buffer(device, command_pool, graphics_queue, staging_buffer, vertex_buffer, buffer_size)?;
    
    // Clean up staging buffer
    unsafe {
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_buffer_memory, None);
    }
    
    Ok((vertex_buffer, vertex_buffer_memory))
}

use crate::texture::{begin_single_time_commands, end_single_time_commands};

pub fn create_texture_array(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    textures: &[crate::texture::TextureData],
    width: u32,
    height: u32,
) -> Result<(vk::Image, vk::DeviceMemory), Box<dyn std::error::Error>> {
    
    let layer_count = textures.len() as u32;
    let image_size = (width * height * 4) as vk::DeviceSize;
    let total_size = image_size * layer_count as vk::DeviceSize;
    
    // Create staging buffer
    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        device,
        physical_device,
        total_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    
    // Copy all texture data to staging buffer
    unsafe {
        let data = device.map_memory(staging_buffer_memory, 0, total_size, vk::MemoryMapFlags::empty())?;
        
        for (layer_idx, texture) in textures.iter().enumerate() {
            let layer_offset = layer_idx as vk::DeviceSize * image_size;
            let dst_ptr = (data as *mut u8).offset(layer_offset as isize);
            
            // Downsample texture if needed
            let src_width = texture.width;
            let src_height = texture.height;
            
            if src_width <= width && src_height <= height {
                // Texture fits, copy directly
                let src_pitch = src_width * 4;
                let dst_pitch = width * 4;
                
                for y in 0..src_height {
                    let src_offset = y * src_pitch;
                    let dst_offset = y * dst_pitch;
                    std::ptr::copy_nonoverlapping(
                        texture.pixels[src_offset as usize..].as_ptr(),
                        dst_ptr.offset(dst_offset as isize),
                        src_pitch as usize,
                    );
                }
                
                // Fill remaining rows with edge color
                if src_height < height {
                    let last_row_offset = (src_height - 1) * dst_pitch;
                    for y in src_height..height {
                        let dst_offset = y * dst_pitch;
                        std::ptr::copy_nonoverlapping(
                            dst_ptr.offset(last_row_offset as isize),
                            dst_ptr.offset(dst_offset as isize),
                            dst_pitch as usize,
                        );
                    }
                }
            } else {
                // Downsample texture using simple bilinear filtering
                let x_ratio = src_width as f32 / width as f32;
                let y_ratio = src_height as f32 / height as f32;
                
                for dst_y in 0..height {
                    for dst_x in 0..width {
                        let src_x = (dst_x as f32 * x_ratio) as u32;
                        let src_y = (dst_y as f32 * y_ratio) as u32;
                        
                        let src_x = src_x.min(src_width - 1);
                        let src_y = src_y.min(src_height - 1);
                        
                        let src_idx = ((src_y * src_width + src_x) * 4) as usize;
                        let dst_idx = ((dst_y * width + dst_x) * 4) as isize;
                        
                        if src_idx + 3 < texture.pixels.len() {
                            *dst_ptr.offset(dst_idx) = texture.pixels[src_idx];
                            *dst_ptr.offset(dst_idx + 1) = texture.pixels[src_idx + 1];
                            *dst_ptr.offset(dst_idx + 2) = texture.pixels[src_idx + 2];
                            *dst_ptr.offset(dst_idx + 3) = texture.pixels[src_idx + 3];
                        }
                    }
                }
            }
        }
        
        device.unmap_memory(staging_buffer_memory);
    }
    
    // Create image for texture array
    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(layer_count)
        .format(vk::Format::R8G8B8A8_SRGB)
        .tiling(vk::ImageTiling::OPTIMAL)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(vk::SampleCountFlags::TYPE_1);
    
    let texture_image = unsafe { device.create_image(&image_info, None)? };
    
    // Allocate memory for image
    let mem_requirements = unsafe { device.get_image_memory_requirements(texture_image) };
    let mem_type_index = find_memory_type(
        instance,
        physical_device,
        mem_requirements.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    
    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_requirements.size)
        .memory_type_index(mem_type_index);
    
    let texture_image_memory = unsafe { device.allocate_memory(&alloc_info, None)? };
    unsafe { device.bind_image_memory(texture_image, texture_image_memory, 0)? };
    
    // Transition image layout and copy from staging buffer
    transition_image_layout_array(
        device,
        command_pool,
        graphics_queue,
        texture_image,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        layer_count,
    )?;
    
    copy_buffer_to_image_array(
        device,
        command_pool,
        graphics_queue,
        staging_buffer,
        texture_image,
        width,
        height,
        layer_count,
    )?;
    
    transition_image_layout_array(
        device,
        command_pool,
        graphics_queue,
        texture_image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        layer_count,
    )?;
    
    // Clean up staging buffer
    unsafe {
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_buffer_memory, None);
    }
    
    Ok((texture_image, texture_image_memory))
}

pub fn transition_image_layout_array(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    layer_count: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    
    let command_buffer = begin_single_time_commands(device, command_pool)?;
    
    let (src_access_mask, dst_access_mask, src_stage, dst_stage) = match (old_layout, new_layout) {
        (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
            vk::AccessFlags::empty(),
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
        ),
        (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        ),
        _ => return Err("Unsupported layout transition".into()),
    };
    
    let barrier = vk::ImageMemoryBarrier::default()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count,
        })
        .src_access_mask(src_access_mask)
        .dst_access_mask(dst_access_mask);
    
    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            src_stage,
            dst_stage,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
    }
    
    end_single_time_commands(device, command_pool, graphics_queue, command_buffer)?;
    
    Ok(())
}

pub fn copy_buffer_to_image_array(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    buffer: vk::Buffer,
    image: vk::Image,
    width: u32,
    height: u32,
    layer_count: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    
    let command_buffer = begin_single_time_commands(device, command_pool)?;
    
    let layer_size = (width * height * 4) as vk::DeviceSize;
    
    for layer in 0..layer_count {
        let region = vk::BufferImageCopy::default()
            .buffer_offset(layer as vk::DeviceSize * layer_size)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: layer,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });
        
        unsafe {
            device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );
        }
    }
    
    end_single_time_commands(device, command_pool, graphics_queue, command_buffer)?;
    
    Ok(())
}

pub fn create_texture_array_view(
    device: &ash::Device,
    image: vk::Image,
    layer_count: u32,
) -> Result<vk::ImageView, Box<dyn std::error::Error>> {
    let view_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
        .format(vk::Format::R8G8B8A8_SRGB)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count,
        });
    
    let image_view = unsafe { device.create_image_view(&view_info, None)? };
    
    Ok(image_view)
}

// Texture image helper functions
pub fn create_texture_image(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    path: &str,
) -> Result<(vk::Image, vk::DeviceMemory), Box<dyn std::error::Error>> {
    let image_data = image::open(path)?.to_rgba8();
    let (width, height) = image_data.dimensions();
    let size = (width * height * 4) as vk::DeviceSize;
    
    // Create staging buffer
    let (staging_buffer, staging_memory) = create_buffer(
        instance,
        device,
        physical_device,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    
    // Copy image data to staging buffer
    unsafe {
        let data = device.map_memory(staging_memory, 0, size, vk::MemoryMapFlags::empty())?;
        std::ptr::copy_nonoverlapping(image_data.as_raw().as_ptr(), data as *mut u8, size as usize);
        device.unmap_memory(staging_memory);
    }
    
    // Create image
    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D { width, height, depth: 1 })
        .mip_levels(1)
        .array_layers(1)
        .format(vk::Format::R8G8B8A8_SRGB)
        .tiling(vk::ImageTiling::OPTIMAL)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(vk::SampleCountFlags::TYPE_1);
    
    let image = unsafe { device.create_image(&image_info, None)? };
    
    let mem_requirements = unsafe { device.get_image_memory_requirements(image) };
    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_requirements.size)
        .memory_type_index(find_memory_type(
            instance,
            physical_device,
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?);
    
    let image_memory = unsafe { device.allocate_memory(&alloc_info, None)? };
    unsafe { device.bind_image_memory(image, image_memory, 0)? };
    
    // Transition and copy
    transition_image_layout(device, command_pool, queue, image, vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL)?;
    copy_buffer_to_image(device, command_pool, queue, staging_buffer, image, width, height)?;
    transition_image_layout(device, command_pool, queue, image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)?;
    
    // Cleanup staging
    unsafe {
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_memory, None);
    }
    
    Ok((image, image_memory))
}

pub fn create_texture_image_view(
    device: &ash::Device,
    image: vk::Image,
) -> Result<vk::ImageView, Box<dyn std::error::Error>> {
    let view_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(vk::Format::R8G8B8A8_SRGB)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });
    
    let image_view = unsafe { device.create_image_view(&view_info, None)? };
    Ok(image_view)
}

pub fn create_texture_sampler(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
) -> Result<vk::Sampler, Box<dyn std::error::Error>> {
    let properties = unsafe { instance.get_physical_device_properties(physical_device) };
    
    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::REPEAT)
        .address_mode_v(vk::SamplerAddressMode::REPEAT)
        .address_mode_w(vk::SamplerAddressMode::REPEAT)
        .anisotropy_enable(true)
        .max_anisotropy(properties.limits.max_sampler_anisotropy)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR);
    
    let sampler = unsafe { device.create_sampler(&sampler_info, None)? };
    Ok(sampler)
}

pub fn transition_image_layout(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) -> Result<(), Box<dyn std::error::Error>> {
    
    let command_buffer = begin_single_time_commands(device, command_pool)?;
    
    let (src_access_mask, dst_access_mask, src_stage, dst_stage) = match (old_layout, new_layout) {
        (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
            vk::AccessFlags::empty(),
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
        ),
        (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        ),
        _ => return Err("Unsupported layout transition".into()),
    };
    
    let barrier = vk::ImageMemoryBarrier::default()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        })
        .src_access_mask(src_access_mask)
        .dst_access_mask(dst_access_mask);
    
    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            src_stage,
            dst_stage,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
    }
    
    end_single_time_commands(device, command_pool, queue, command_buffer)?;
    
    Ok(())
}

pub fn copy_buffer_to_image(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    buffer: vk::Buffer,
    image: vk::Image,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    
    let command_buffer = begin_single_time_commands(device, command_pool)?;
    
    let region = vk::BufferImageCopy::default()
        .buffer_offset(0)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_subresource(vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
        })
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(vk::Extent3D { width, height, depth: 1 });
    
    unsafe {
        device.cmd_copy_buffer_to_image(
            command_buffer,
            buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[region],
        );
    }
    
    end_single_time_commands(device, command_pool, queue, command_buffer)?;
    
    Ok(())
}

use ash::{vk, Instance};

pub struct TextureData {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl TextureData {
    pub fn new(pixels: Vec<u8>, width: u32, height: u32) -> Self {
        Self { pixels, width, height }
    }
    
    pub fn placeholder() -> Self {
        // Create a 2x2 purple texture as placeholder
        let pixels = vec![
            128, 0, 128, 255,  // Purple
            255, 0, 255, 255,  // Magenta
            255, 0, 255, 255,  // Magenta
            128, 0, 128, 255,  // Purple
        ];
        Self {
            pixels,
            width: 2,
            height: 2,
        }
    }
}

pub struct Texture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
}

impl Texture {
    pub fn from_file(
        instance: &Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Load image from file using image crate
        let img = image::open(path)?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let pixels = rgba.into_raw();
        
        let texture_data = TextureData::new(pixels, width, height);
        Self::create(instance, device, physical_device, command_pool, queue, &texture_data)
    }
    
    pub fn create(
        instance: &Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        texture_data: &TextureData,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let image_size = (texture_data.width * texture_data.height * 4) as vk::DeviceSize;
        
        // Create staging buffer
        let (staging_buffer, staging_memory) = create_buffer(
            instance,
            device,
            physical_device,
            image_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Copy data to staging buffer
        unsafe {
            let data = device.map_memory(staging_memory, 0, image_size, vk::MemoryMapFlags::empty())?;
            std::ptr::copy_nonoverlapping(texture_data.pixels.as_ptr(), data as *mut u8, image_size as usize);
            device.unmap_memory(staging_memory);
        }
        
        // Create image
        let (image, memory) = create_image(
            instance,
            device,
            physical_device,
            texture_data.width,
            texture_data.height,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        
        // Transition image layout and copy buffer to image
        transition_image_layout(
            device,
            command_pool,
            queue,
            image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        )?;
        
        copy_buffer_to_image(
            device,
            command_pool,
            queue,
            staging_buffer,
            image,
            texture_data.width,
            texture_data.height,
        )?;
        
        transition_image_layout(
            device,
            command_pool,
            queue,
            image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )?;
        
        // Cleanup staging buffer
        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_memory, None);
        }
        
        // Create image view
        let view = create_image_view(device, image, vk::Format::R8G8B8A8_SRGB)?;
        
        Ok(Self { image, memory, view })
    }
    
    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
        }
    }
}

pub fn create_texture_sampler(device: &ash::Device) -> Result<vk::Sampler, Box<dyn std::error::Error>> {
    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::REPEAT)
        .address_mode_v(vk::SamplerAddressMode::REPEAT)
        .address_mode_w(vk::SamplerAddressMode::REPEAT)
        .anisotropy_enable(true)
        .max_anisotropy(16.0)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .mip_lod_bias(0.0)
        .min_lod(0.0)
        .max_lod(0.0);
    
    let sampler = unsafe { device.create_sampler(&sampler_info, None)? };
    Ok(sampler)
}

fn create_buffer(
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

fn create_image(
    instance: &Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    width: u32,
    height: u32,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Image, vk::DeviceMemory), Box<dyn std::error::Error>> {
    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .format(format)
        .tiling(tiling)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(usage)
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
            properties,
        )?);
    
    let image_memory = unsafe { device.allocate_memory(&alloc_info, None)? };
    
    unsafe { device.bind_image_memory(image, image_memory, 0)? };
    
    Ok((image, image_memory))
}

fn transition_image_layout(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    image: vk::Image,
    _format: vk::Format,
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

fn copy_buffer_to_image(
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
    
    end_single_time_commands(device, command_pool, queue, command_buffer)?;
    
    Ok(())
}

pub fn begin_single_time_commands(
    device: &ash::Device,
    command_pool: vk::CommandPool,
) -> Result<vk::CommandBuffer, Box<dyn std::error::Error>> {
    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(command_pool)
        .command_buffer_count(1);
    
    let command_buffer = unsafe { device.allocate_command_buffers(&alloc_info)?[0] };
    
    let begin_info = vk::CommandBufferBeginInfo::default()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    
    unsafe {
        device.begin_command_buffer(command_buffer, &begin_info)?;
    }
    
    Ok(command_buffer)
}

pub fn end_single_time_commands(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    command_buffer: vk::CommandBuffer,
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
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

pub fn create_image_view(
    device: &ash::Device,
    image: vk::Image,
    format: vk::Format,
) -> Result<vk::ImageView, Box<dyn std::error::Error>> {
    let view_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
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

fn find_memory_type(
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
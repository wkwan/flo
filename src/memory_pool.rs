use ash::vk;
use std::collections::HashMap;

pub struct MemoryPool {
    device: ash::Device,
    allocations: Vec<MemoryAllocation>,
    free_regions: Vec<FreeRegion>,
    allocation_size: vk::DeviceSize,
    memory_type_index: u32,
    total_allocated: usize,
}

struct MemoryAllocation {
    memory: vk::DeviceMemory,
    #[allow(dead_code)]
    size: vk::DeviceSize,
}

#[derive(Clone)]
struct FreeRegion {
    allocation_index: usize,
    offset: vk::DeviceSize,
    size: vk::DeviceSize,
}

#[derive(Clone)]
pub struct MemoryBlock {
    pub memory: vk::DeviceMemory,
    pub offset: vk::DeviceSize,
    pub size: vk::DeviceSize,
    allocation_index: usize,
    pool_memory_type: u32,
}

impl MemoryPool {
    pub fn new(
        device: ash::Device,
        memory_type_index: u32,
        allocation_size: vk::DeviceSize,
    ) -> Self {
        Self {
            device,
            allocations: Vec::new(),
            free_regions: Vec::new(),
            allocation_size: allocation_size.max(256 * 1024 * 1024), // Min 256MB per allocation
            memory_type_index,
            total_allocated: 0,
        }
    }

    pub fn allocate(&mut self, size: vk::DeviceSize, alignment: vk::DeviceSize) -> Result<MemoryBlock, Box<dyn std::error::Error>> {
        let aligned_size = ((size + alignment - 1) / alignment) * alignment;
        
        // Try to find a free region that fits
        for (i, region) in self.free_regions.iter().enumerate() {
            let aligned_offset = ((region.offset + alignment - 1) / alignment) * alignment;
            let padding = aligned_offset - region.offset;
            
            if region.size >= aligned_size + padding {
                let block = MemoryBlock {
                    memory: self.allocations[region.allocation_index].memory,
                    offset: aligned_offset,
                    size: aligned_size,
                    allocation_index: region.allocation_index,
                    pool_memory_type: self.memory_type_index,
                };
                
                // Update or remove the free region
                if region.size > aligned_size + padding {
                    self.free_regions[i] = FreeRegion {
                        allocation_index: region.allocation_index,
                        offset: aligned_offset + aligned_size,
                        size: region.size - aligned_size - padding,
                    };
                } else {
                    self.free_regions.remove(i);
                }
                
                return Ok(block);
            }
        }
        
        // Need to allocate a new chunk
        let chunk_size = self.allocation_size.max(aligned_size);
        self.allocate_new_chunk(chunk_size)?;
        
        // Retry allocation (should succeed now)
        self.allocate(size, alignment)
    }

    fn allocate_new_chunk(&mut self, size: vk::DeviceSize) -> Result<(), Box<dyn std::error::Error>> {
        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(size)
            .memory_type_index(self.memory_type_index);
        
        let memory = unsafe { self.device.allocate_memory(&alloc_info, None)? };
        
        let allocation_index = self.allocations.len();
        
        self.allocations.push(MemoryAllocation {
            memory,
            size,
        });
        
        self.free_regions.push(FreeRegion {
            allocation_index,
            offset: 0,
            size,
        });
        
        self.total_allocated += 1;
        println!("Memory pool: Allocated chunk {} ({:.2} MB), total allocations: {}", 
                 allocation_index, size as f64 / (1024.0 * 1024.0), self.total_allocated);
        
        Ok(())
    }

    pub fn free(&mut self, block: MemoryBlock) {
        // Add the freed block back to free regions (simplified - doesn't coalesce)
        self.free_regions.push(FreeRegion {
            allocation_index: block.allocation_index,
            offset: block.offset,
            size: block.size,
        });
        
        // TODO: Implement coalescing of adjacent free regions
    }

    pub fn destroy(&mut self) {
        unsafe {
            for allocation in &self.allocations {
                self.device.free_memory(allocation.memory, None);
            }
        }
        self.allocations.clear();
        self.free_regions.clear();
    }
}

pub struct MemoryPoolManager {
    device: ash::Device,
    pools: HashMap<u32, MemoryPool>,
    staging_buffer: Option<vk::Buffer>,
    staging_memory: Option<vk::DeviceMemory>,
    staging_size: vk::DeviceSize,
}

impl MemoryPoolManager {
    pub fn new(device: ash::Device) -> Self {
        Self {
            device,
            pools: HashMap::new(),
            staging_buffer: None,
            staging_memory: None,
            staging_size: 0,
        }
    }
    
    pub fn get_staging_buffer(
        &mut self,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        required_size: vk::DeviceSize,
    ) -> Result<(vk::Buffer, vk::DeviceMemory), Box<dyn std::error::Error>> {
        // If we need a larger staging buffer, destroy the old one and create a new one
        if self.staging_size < required_size {
            // Clean up old staging buffer if it exists
            if let Some(buffer) = self.staging_buffer {
                unsafe {
                    self.device.destroy_buffer(buffer, None);
                }
            }
            if let Some(memory) = self.staging_memory {
                unsafe {
                    self.device.free_memory(memory, None);
                }
            }
            
            // Create a new staging buffer that's at least 16MB or the required size
            let size = required_size.max(16 * 1024 * 1024);
            
            let buffer_info = vk::BufferCreateInfo::default()
                .size(size)
                .usage(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            
            let buffer = unsafe { self.device.create_buffer(&buffer_info, None)? };
            
            let mem_requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };
            
            let memory_type_index = crate::vulkan_common::find_memory_type(
                instance,
                physical_device,
                mem_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            
            let alloc_info = vk::MemoryAllocateInfo::default()
                .allocation_size(size)
                .memory_type_index(memory_type_index);
            
            let memory = unsafe { self.device.allocate_memory(&alloc_info, None)? };
            unsafe { self.device.bind_buffer_memory(buffer, memory, 0)? };
            
            self.staging_buffer = Some(buffer);
            self.staging_memory = Some(memory);
            self.staging_size = size;
            
            println!("Created reusable staging buffer ({:.2} MB)", size as f64 / (1024.0 * 1024.0));
        }
        
        Ok((self.staging_buffer.unwrap(), self.staging_memory.unwrap()))
    }

    pub fn allocate_buffer(
        &mut self,
        buffer: vk::Buffer,
        memory_requirements: vk::MemoryRequirements,
        memory_type_index: u32,
    ) -> Result<MemoryBlock, Box<dyn std::error::Error>> {
        let pool = self.pools.entry(memory_type_index).or_insert_with(|| {
            MemoryPool::new(self.device.clone(), memory_type_index, 256 * 1024 * 1024)
        });
        
        let block = pool.allocate(memory_requirements.size, memory_requirements.alignment)?;
        
        unsafe {
            self.device.bind_buffer_memory(buffer, block.memory, block.offset)?;
        }
        
        Ok(block)
    }

    pub fn allocate_image(
        &mut self,
        image: vk::Image,
        memory_requirements: vk::MemoryRequirements,
        memory_type_index: u32,
    ) -> Result<MemoryBlock, Box<dyn std::error::Error>> {
        let pool = self.pools.entry(memory_type_index).or_insert_with(|| {
            MemoryPool::new(self.device.clone(), memory_type_index, 256 * 1024 * 1024)
        });
        
        let block = pool.allocate(memory_requirements.size, memory_requirements.alignment)?;
        
        unsafe {
            self.device.bind_image_memory(image, block.memory, block.offset)?;
        }
        
        Ok(block)
    }

    pub fn free_buffer(&mut self, block: MemoryBlock) {
        if let Some(pool) = self.pools.get_mut(&block.pool_memory_type) {
            pool.free(block);
        }
    }

    pub fn destroy(&mut self) {
        // Clean up staging buffer
        if let Some(buffer) = self.staging_buffer {
            unsafe {
                self.device.destroy_buffer(buffer, None);
            }
        }
        if let Some(memory) = self.staging_memory {
            unsafe {
                self.device.free_memory(memory, None);
            }
        }
        
        // Clean up pools
        for pool in self.pools.values_mut() {
            pool.destroy();
        }
        self.pools.clear();
    }

    pub fn get_stats(&self) -> String {
        let mut total_allocations = 0;
        let total_pools = self.pools.len();
        
        for (_type_index, pool) in &self.pools {
            total_allocations += pool.total_allocated;
        }
        
        format!("Memory pools: {}, Total GPU allocations: {}", total_pools, total_allocations)
    }
}
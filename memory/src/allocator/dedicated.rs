use std::{ops::Range, ptr::NonNull};

use ash::{
    version::DeviceV1_0,
    vk::{DeviceMemory, MemoryAllocateInfo, MemoryPropertyFlags},
};

use allocator::Allocator;
use block::Block;
use error::*;
use mapping::{mapped_fitting_range, MappedRange};
use memory::*;

/// Memory block allocated from `DedicatedAllocator`
#[derive(Debug)]
pub struct DedicatedBlock {
    memory: Memory,
    mapping: Option<(NonNull<u8>, Range<u64>)>,
}

unsafe impl Send for DedicatedBlock {}
unsafe impl Sync for DedicatedBlock {}

impl DedicatedBlock {
    /// Get inner memory.
    /// Panics if mapped.
    pub fn unwrap_memory(self) -> Memory {
        assert!(self.mapping.is_none());
        self.memory
    }

    /// Make unmapped block.
    pub fn from_memory(memory: Memory) -> Self {
        DedicatedBlock {
            memory,
            mapping: None,
        }
    }
}

impl Block for DedicatedBlock {
    #[inline]
    fn properties(&self) -> MemoryPropertyFlags {
        self.memory.properties()
    }

    #[inline]
    fn memory(&self) -> DeviceMemory {
        self.memory.raw()
    }

    #[inline]
    fn range(&self) -> Range<u64> {
        0..self.memory.size()
    }

    fn map<'a>(
        &'a mut self,
        device: &impl DeviceV1_0,
        range: Range<u64>,
    ) -> Result<MappedRange<'a>, MappingError> {
        assert!(
            range.start <= range.end,
            "Memory mapping region must have valid size"
        );

        unsafe {
            if let Some(ptr) = self
                .mapping
                .clone()
                .and_then(|mapping| mapped_fitting_range(mapping.0, mapping.1, range.clone()))
            {
                Ok(MappedRange::from_raw(&self.memory, ptr, range))
            } else {
                self.unmap(device);
                let mapping = MappedRange::new(&self.memory, device, range.clone())?;
                self.mapping = Some((mapping.ptr(), mapping.range()));
                Ok(mapping)
            }
        }
    }

    fn unmap(&mut self, device: &impl DeviceV1_0) {
        if self.mapping.take().is_some() {
            unsafe {
                trace!("Unmap memory: {:#?}", self.memory);
                device.unmap_memory(self.memory.raw());
            }
        }
    }
}

/// Dummy memory allocator that uses memory object per allocation requested.
///
/// This allocator suites best huge allocations.
/// From 32 MiB when GPU has 4-8 GiB memory total.
///
/// `Heaps` use this allocator when none of sub-allocators bound to the memory type
/// can handle size required.
#[derive(Debug)]
pub struct DedicatedAllocator {
    memory_type: u32,
    memory_properties: MemoryPropertyFlags,
    used: u64,
}

impl DedicatedAllocator {
    /// Get properties required by the allocator.
    pub fn properties_required() -> MemoryPropertyFlags {
        MemoryPropertyFlags::empty()
    }

    /// Create new `ArenaAllocator`
    /// for `memory_type` with `memory_properties` specified
    pub fn new(memory_type: u32, memory_properties: MemoryPropertyFlags) -> Self {
        DedicatedAllocator {
            memory_type,
            memory_properties,
            used: 0,
        }
    }
}

impl Allocator for DedicatedAllocator {
    type Block = DedicatedBlock;

    #[inline]
    fn alloc(
        &mut self,
        device: &impl DeviceV1_0,
        size: u64,
        _align: u64,
    ) -> Result<(DedicatedBlock, u64), MemoryError> {
        let memory = unsafe {
            Memory::from_raw(
                device.allocate_memory(
                    &MemoryAllocateInfo::builder()
                        .memory_type_index(self.memory_type)
                        .allocation_size(size)
                        .build(),
                    None,
                )?,
                size,
                self.memory_properties,
            )
        };

        self.used += size;

        Ok((DedicatedBlock::from_memory(memory), size))
    }

    #[inline]
    fn free(&mut self, device: &impl DeviceV1_0, mut block: DedicatedBlock) -> u64 {
        block.unmap(device);
        let size = block.memory.size();
        self.used -= size;
        unsafe {
            device.free_memory(block.memory.raw(), None);
        }
        size
    }
}

impl Drop for DedicatedAllocator {
    fn drop(&mut self) {
        assert_eq!(self.used, 0);
    }
}

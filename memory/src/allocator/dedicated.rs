use std::{ops::Range, ptr::NonNull};

use crate::{
    allocator::{Allocator, Kind},
    block::Block,
    mapping::{mapped_fitting_range, MappedRange},
    memory::*,
};

/// Memory block allocated from `DedicatedAllocator`
#[derive(Debug)]
pub struct DedicatedBlock<B: gfx_hal::Backend,> {
    memory: Memory<B>,
    mapping: Option<(NonNull<u8>, Range<u64>)>,
}

unsafe impl<B> Send for DedicatedBlock<B> where B: gfx_hal::Backend, {}
unsafe impl<B> Sync for DedicatedBlock<B> where B: gfx_hal::Backend, {}

impl<B> DedicatedBlock<B>
where
    B: gfx_hal::Backend,
{
    /// Get inner memory.
    /// Panics if mapped.
    pub fn unwrap_memory(self) -> Memory<B> {
        assert!(self.mapping.is_none());
        self.memory
    }

    /// Make unmapped block.
    pub fn from_memory(memory: Memory<B>) -> Self {
        DedicatedBlock {
            memory,
            mapping: None,
        }
    }
}

impl<B> Block<B> for DedicatedBlock<B>
where
    B: gfx_hal::Backend,
{
    #[inline]
    fn properties(&self) -> gfx_hal::memory::Properties {
        self.memory.properties()
    }

    #[inline]
    fn memory(&self) -> &B::Memory {
        self.memory.raw()
    }

    #[inline]
    fn range(&self) -> Range<u64> {
        0..self.memory.size()
    }

    fn map<'a>(
        &'a mut self,
        device: &impl gfx_hal::Device<B>,
        range: Range<u64>,
    ) -> Result<MappedRange<'a, B>, gfx_hal::mapping::Error> {
        assert!(
            range.start <= range.end,
            "Memory mapping region must have valid size"
        );

        if !self.memory.host_visible() {
            return Err(gfx_hal::mapping::Error::InvalidAccess);
        }

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

    fn unmap(&mut self, device: &impl gfx_hal::Device<B>) {
        if self.mapping.take().is_some() {
            unsafe {
                // trace!("Unmap memory: {:#?}", self.memory);
                device.unmap_memory(self.memory.raw());
            }
        }
    }
}

/// Dedicated memory allocator that uses memory object per allocation requested.
///
/// This allocator suites best huge allocations.
/// From 32 MiB when GPU has 4-8 GiB memory total.
///
/// `Heaps` use this allocator when none of sub-allocators bound to the memory type
/// can handle size required.
/// TODO: Check if resource prefers dedicated memory.
#[derive(Debug)]
pub struct DedicatedAllocator {
    memory_type: gfx_hal::MemoryTypeId,
    memory_properties: gfx_hal::memory::Properties,
    used: u64,
}

impl DedicatedAllocator {
    /// Get properties required by the allocator.
    pub fn properties_required() -> gfx_hal::memory::Properties {
        gfx_hal::memory::Properties::empty()
    }

    /// Create new `LinearAllocator`
    /// for `memory_type` with `memory_properties` specified
    pub fn new(memory_type: gfx_hal::MemoryTypeId, memory_properties: gfx_hal::memory::Properties) -> Self {
        DedicatedAllocator {
            memory_type,
            memory_properties,
            used: 0,
        }
    }
}

impl<B> Allocator<B> for DedicatedAllocator
where
    B: gfx_hal::Backend,
{
    type Block = DedicatedBlock<B>;

    fn kind() -> Kind {
        Kind::Dedicated
    }

    #[inline]
    fn alloc(
        &mut self,
        device: &impl gfx_hal::Device<B>,
        size: u64,
        _align: u64,
    ) -> Result<(DedicatedBlock<B>, u64), gfx_hal::device::AllocationError> {
        let memory = unsafe {
            Memory::from_raw(
                device.allocate_memory(
                    self.memory_type,
                    size,
                )?,
                size,
                self.memory_properties,
            )
        };

        self.used += size;

        Ok((DedicatedBlock::from_memory(memory), size))
    }

    #[inline]
    fn free(&mut self, device: &impl gfx_hal::Device<B>, mut block: DedicatedBlock<B>) -> u64 {
        block.unmap(device);
        let size = block.memory.size();
        self.used -= size;
        unsafe {
            device.free_memory(block.memory.into_raw());
        }
        size
    }
}

impl Drop for DedicatedAllocator {
    fn drop(&mut self) {
        if self.used > 0 {
            log::error!("Not all allocation from DedicatedAllocator was freed");
        }
    }
}

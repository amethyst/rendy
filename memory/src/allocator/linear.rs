use std::{collections::VecDeque, ops::Range, ptr::NonNull};

use {
    crate::{
        allocator::{Allocator, Kind},
        block::Block,
        mapping::*,
        memory::*,
        util::*,
    },
    rendy_core::hal::{Backend, device::Device as _},
};

/// Memory block allocated from `LinearAllocator`
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct LinearBlock<B: Backend> {
    // #[derivative(Debug(format_with = "::memory::memory_ptr_fmt"))]
    memory: *const Memory<B>,
    linear_index: u64,
    ptr: NonNull<u8>,
    range: Range<u64>,
    #[derivative(Debug = "ignore")]
    relevant: relevant::Relevant,
}

unsafe impl<B> Send for LinearBlock<B> where B: Backend {}
unsafe impl<B> Sync for LinearBlock<B> where B: Backend {}

impl<B> LinearBlock<B>
where
    B: Backend,
{
    fn shared_memory(&self) -> &Memory<B> {
        // Memory won't be freed until last block created from it deallocated.
        unsafe { &*self.memory }
    }

    fn size(&self) -> u64 {
        self.range.end - self.range.start
    }

    fn dispose(self) {
        self.relevant.dispose();
    }
}

impl<B> Block<B> for LinearBlock<B>
where
    B: Backend,
{
    #[inline]
    fn properties(&self) -> rendy_core::hal::memory::Properties {
        self.shared_memory().properties()
    }

    #[inline]
    fn memory(&self) -> &B::Memory {
        self.shared_memory().raw()
    }

    #[inline]
    fn range(&self) -> Range<u64> {
        self.range.clone()
    }

    #[inline]
    fn map<'a>(
        &'a mut self,
        _device: &B::Device,
        range: Range<u64>,
    ) -> Result<MappedRange<'a, B>, rendy_core::hal::mapping::Error> {
        assert!(
            range.start < range.end,
            "Memory mapping region must have valid size"
        );
        if !self.shared_memory().host_visible() {
            return Err(rendy_core::hal::mapping::Error::InvalidAccess);
        }

        if let Some((ptr, range)) = mapped_sub_range(self.ptr, self.range.clone(), range) {
            let mapping = unsafe { MappedRange::from_raw(self.shared_memory(), ptr, range) };
            Ok(mapping)
        } else {
            Err(rendy_core::hal::mapping::Error::OutOfBounds)
        }
    }

    #[inline]
    fn unmap(&mut self, _device: &B::Device) {
        debug_assert!(self.shared_memory().host_visible());
    }
}

/// Config for `LinearAllocator`.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LinearConfig {
    /// Size of the linear chunk.
    /// Keep it big.
    pub linear_size: u64,
}

/// Linear allocator that return memory from chunk sequentially.
/// It keeps only number of bytes allocated from each chunk.
/// Once chunk is exhausted it is placed into list.
/// When all blocks allocated from head of that list are freed,
/// head is freed as well.
///
/// This allocator suites best short-lived types of allocations.
/// Allocation strategy requires minimal overhead and implementation is fast.
/// But holding single block will completely stop memory recycling.
#[derive(Debug)]
pub struct LinearAllocator<B: Backend> {
    memory_type: rendy_core::hal::MemoryTypeId,
    memory_properties: rendy_core::hal::memory::Properties,
    linear_size: u64,
    offset: u64,
    lines: VecDeque<Line<B>>,
}

#[derive(derivative::Derivative)]
#[derivative(Debug)]
struct Line<B: Backend> {
    used: u64,
    free: u64,
    #[derivative(Debug = "ignore")]
    memory: Box<Memory<B>>,
    ptr: NonNull<u8>,
}

unsafe impl<B> Send for Line<B> where B: Backend {}
unsafe impl<B> Sync for Line<B> where B: Backend {}

impl<B> LinearAllocator<B>
where
    B: Backend,
{
    /// Get properties required by the `LinearAllocator`.
    pub fn properties_required() -> rendy_core::hal::memory::Properties {
        rendy_core::hal::memory::Properties::CPU_VISIBLE
    }

    /// Maximum allocation size.
    pub fn max_allocation(&self) -> u64 {
        self.linear_size / 2
    }

    /// Create new `LinearAllocator`
    /// for `memory_type` with `memory_properties` specified,
    /// with `LinearConfig` provided.
    pub fn new(
        memory_type: rendy_core::hal::MemoryTypeId,
        memory_properties: rendy_core::hal::memory::Properties,
        config: LinearConfig,
    ) -> Self {
        log::trace!(
            "Create new 'linear' allocator: type: '{:?}', properties: '{:#?}' config: '{:#?}'",
            memory_type,
            memory_properties,
            config
        );
        assert!(memory_properties.contains(Self::properties_required()));
        assert!(
            fits_usize(config.linear_size),
            "Linear size must fit in both usize and u64"
        );
        LinearAllocator {
            memory_type,
            memory_properties,
            linear_size: config.linear_size,
            offset: 0,
            lines: VecDeque::new(),
        }
    }

    /// Perform full cleanup of the memory allocated.
    pub fn dispose(mut self, device: &B::Device) {
        let _ = self.cleanup(device, 0);
        if !self.lines.is_empty() {
            log::error!(
                "Lines are not empty during allocator disposal. Lines: {:#?}",
                self.lines
            );
        }
    }

    fn cleanup(&mut self, device: &B::Device, off: usize) -> u64 {
        let mut freed = 0;
        while self.lines.len() > off {
            if self.lines[0].used > self.lines[0].free {
                break;
            }

            let line = self.lines.pop_front().unwrap();
            self.offset += 1;

            unsafe {
                // trace!("Unmap memory: {:#?}", line.memory);
                device.unmap_memory(line.memory.raw());

                freed += line.memory.size();
                device.free_memory(line.memory.into_raw());
            }
        }
        freed
    }
}

impl<B> Allocator<B> for LinearAllocator<B>
where
    B: Backend,
{
    type Block = LinearBlock<B>;

    fn kind() -> Kind {
        Kind::Linear
    }

    fn alloc(
        &mut self,
        device: &B::Device,
        size: u64,
        align: u64,
    ) -> Result<(LinearBlock<B>, u64), rendy_core::hal::device::AllocationError> {
        debug_assert!(self
            .memory_properties
            .contains(rendy_core::hal::memory::Properties::CPU_VISIBLE));

        assert!(size <= self.linear_size);
        assert!(align <= self.linear_size);

        let count = self.lines.len() as u64;
        if let Some(line) = self.lines.back_mut() {
            let aligned = aligned(line.used, align);
            let overhead = aligned - line.used;
            if self.linear_size - size > aligned {
                line.used = aligned + size;
                line.free += overhead;
                let (ptr, range) =
                    mapped_sub_range(line.ptr, 0..self.linear_size, aligned..aligned + size)
                        .expect("This sub-range must fit in line mapping");

                return Ok((
                    LinearBlock {
                        linear_index: self.offset + count - 1,
                        memory: &*line.memory,
                        ptr,
                        range,
                        relevant: relevant::Relevant,
                    },
                    0,
                ));
            }
        }

        let (memory, ptr) = unsafe {
            let raw = device.allocate_memory(self.memory_type, self.linear_size)?;

            let ptr = match device.map_memory(&raw, 0..self.linear_size) {
                Ok(ptr) => NonNull::new_unchecked(ptr),
                Err(rendy_core::hal::mapping::Error::OutOfMemory(error)) => {
                    device.free_memory(raw);
                    return Err(error.into());
                }
                Err(_) => panic!("Unexpected mapping failure"),
            };

            let memory = Memory::from_raw(raw, self.linear_size, self.memory_properties);

            (memory, ptr)
        };

        let line = Line {
            used: size,
            free: 0,
            ptr,
            memory: Box::new(memory),
        };

        let (ptr, range) = mapped_sub_range(ptr, 0..self.linear_size, 0..size)
            .expect("This sub-range must fit in line mapping");

        let block = LinearBlock {
            linear_index: self.offset + count,
            memory: &*line.memory,
            ptr,
            range,
            relevant: relevant::Relevant,
        };

        self.lines.push_back(line);
        Ok((block, self.linear_size))
    }

    fn free(&mut self, device: &B::Device, block: Self::Block) -> u64 {
        let index = block.linear_index - self.offset;
        assert!(
            fits_usize(index),
            "This can't exceed lines list length which fits into usize by definition"
        );
        let index = index as usize;
        assert!(
            index < self.lines.len(),
            "Can't be allocated from not yet created line"
        );
        {
            let ref mut line = self.lines[index];
            line.free += block.size();
        }
        block.dispose();

        self.cleanup(device, 1)
    }
}

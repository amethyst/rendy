//! Fast sub-allocator for short-living allocations.
//! Typically used for staging buffers.

use std::{collections::VecDeque, fmt::Debug, ops::Range, slice::from_raw_parts_mut, ptr::NonNull};
use hal;
use block::{Block, MappingError};
use memory::Memory;
use sub::SubAllocator;

pub struct ArenaBlock<T> {
    index: u64,
    memory: *const Memory<T>,
    mapping: Result<NonNull<u8>, MappingError>,
    range: Range<u64>,
}

impl<T> ArenaBlock<T> {
    fn shared_memory(&self) -> &Memory<T> {
        // Memory won't be freed until last block from it deallocated.
        unsafe { &*self.memory }
    }
}

impl<T> Block<T> for ArenaBlock<T>
where
    T: Debug + Send + Sync + 'static,
{
    /// Get memory properties of the block.
    fn properties(&self) -> hal::memory::Properties {
        self.shared_memory().properties
    }

    /// Get memory object.
    fn memory(&self) -> &T {
        self.shared_memory().raw()
    }

    /// Get memory range associated with this block.
    fn range(&self) -> Range<u64> {
        self.range.clone()
    }

    fn map<B>(&mut self, device: &B::Device, range: Range<u64>) -> Result<&mut [u8], MappingError>
    where
        B: hal::Backend<Memory = T>,
    {
        // Arena map whole memory on allocation if possible.
        let mapping = self.mapping.clone()?;

        // Check if specified range is not out of block bounds.
        if range.end < range.start || range.end > (self.range.end - self.range.start) {
            return Err(MappingError::OutOfBounds);
        }

        if range.start == range.end {
            return Ok(&mut [])
        }

        let start = range.start + self.range.start;
        let end = range.end + self.range.start;

        debug_assert!(super::fits_in_usize(start) && super::fits_in_usize(end), "Implied by check above because arena memory size must fit in usize");

        // Invalidate block sub-region.
        if !self.shared_memory().coherent() {
            hal::Device::invalidate_mapped_memory_ranges(device, Some((self.memory(), start .. end)));
        }

        unsafe {
            Ok(from_raw_parts_mut(mapping.as_ptr().add(start as usize), (end - start) as usize))
        }
    }

    fn unmap<B>(&mut self, device: &B::Device, range: Range<u64>)
    where
        B: hal::Backend<Memory = T>,
    {
        // Arena map whole memory on allocation if possible.
        if let Ok(_) = self.mapping.clone() {
            if !self.shared_memory().coherent() {
                // Clamp to this block's range.
                let range = super::clamp_range(range.start + self.range.start .. range.end + self.range.start, self.range.clone());
                // Invalidate block sub-region.
                hal::Device::flush_mapped_memory_ranges(device, Some((self.memory(), range.start .. range.end)));
            }
        }
    }
}

pub struct Arenas<T> {
    arena_size: u64,
    offset: u64,
    arenas: VecDeque<Arena<T>>,
}

impl<T> Arenas<T> {
    fn cleanup<B, F>(&mut self, device: &B::Device, mut external: F)
    where
        T: Send + Sync + Debug + 'static,
        B: hal::Backend<Memory = T>,
        F: FnMut(Memory<T>),
    {
        while self.arenas.len() > 1 {
            if self.arenas[0].used > self.arenas[0].free {
                break;
            }

            let arena = self.arenas.pop_front().unwrap();
            if let Ok(_) = arena.mapping {
                hal::Device::unmap_memory(device, arena.memory.raw());
            }

            external(*arena.memory);
        }
    }
}

impl<T> SubAllocator<T> for Arenas<T>
where
    T: Debug + Send + Sync + 'static,
{
    type Block = ArenaBlock<T>;

    fn sub_allocate<B, F, E>(&mut self, device: &B::Device, size: u64, align: u64, mut external: F) -> Result<Self::Block, E>
    where
        B: hal::Backend<Memory = T>,
        F: FnMut(u64) -> Result<Memory<T>, E>,
        E: From<hal::device::OutOfMemory>,
    {
        if size > self.arena_size {
            return Err(hal::device::OutOfMemory.into());
        }

        let count = self.arenas.len() as u64;
        if let Some(arena) = self.arenas.back_mut() {
            let aligned = super::aligned(arena.used, align);
            if self.arena_size - aligned > size {
                arena.used = aligned + size;
                return Ok(ArenaBlock {
                    index: self.offset + count - 1,
                    memory: &*arena.memory,
                    mapping: arena.mapping.clone(),
                    range: aligned .. arena.used,
                });
            }
        }

        let memory = external(self.arena_size)?;
        let mapping = if memory.cpu_visible() {
            match hal::Device::map_memory(device, memory.raw(), 0 .. self.arena_size) {
                Ok(ptr) => {
                    debug_assert!(!ptr.is_null());
                    Ok(unsafe {NonNull::new_unchecked(ptr)})
                }
                Err(hal::mapping::Error::InvalidAccess) => unreachable!("Memory properties checked"),
                Err(hal::mapping::Error::OutOfBounds) => unreachable!("range checked"),
                Err(hal::mapping::Error::OutOfMemory) => Err(MappingError::OutOfHostMemory),
            }
        } else {
            Err(MappingError::HostInvisible)
        };
        let arena = Arena {
            used: size,
            free: 0,
            mapping,
            memory: Box::new(memory),
        };

        let block = ArenaBlock {
            index: self.offset + count,
            memory: &*arena.memory,
            mapping: arena.mapping.clone(),
            range: 0 .. size,
        };
        self.arenas.push_back(arena);
        Ok(block)
    }

    fn free<B, F>(&mut self, device: &B::Device, block: Self::Block, external: F)
    where
        B: hal::Backend<Memory = T>,
        F: FnMut(Memory<T>),
    {
        {
            let index = block.index - self.offset;
            assert!(super::fits_in_usize(index));
            let index = index as usize;
            assert!(index < self.arenas.len());
            let ref mut arena = self.arenas[index];
            arena.free += block.range.end - block.range.start;
        }

        self.cleanup::<B, F>(device, external);
    }
}

struct Arena<T> {
    used: u64,
    free: u64,
    memory: Box<Memory<T>>,
    mapping: Result<NonNull<u8>, MappingError>,
}


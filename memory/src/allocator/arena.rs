//! Fast sub-allocator for short-living allocations.
//! Typically used for staging buffers.
//! This allocator allocate memory directly from device and maps whole range.

use std::{collections::VecDeque, ops::Range, slice::from_raw_parts_mut, ptr::NonNull};

use allocator::Allocator;
use block::Block;
use device::Device;
use error::*;
use map::*;
use memory::*;
use util;

pub struct ArenaBlock<T> {
    index: u64,
    memory: *const Memory<T>,
    mapping: NonNull<u8>,
    range: Range<u64>,
}

impl<T> ArenaBlock<T> {
    fn shared_memory(&self) -> &Memory<T> {
        // Memory won't be freed until last block created from it is deallocated.
        unsafe { &*self.memory }
    }
}

impl<T> Block<T> for ArenaBlock<T> {
    /// Get memory properties of the block.
    fn properties(&self) -> Properties {
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

    fn map<'a, D>(&'a mut self, device: &D, range: Range<u64>) -> Result<MappedRange<'a, T>, MappingError>
    where
        D: Device<T>,
    {
        // Check if specified range is not out of block bounds.
        if range.end < range.start || range.end > (self.range.end - self.range.start) {
            return Err(MappingError::OutOfBounds);
        }

        let start = range.start + self.range.start;
        let end = range.end + self.range.start;

        debug_assert!(util::fits_in_usize(start) && util::fits_in_usize(end), "Implied by check above because arena memory size must fit in usize");

        let coherent = maybe_coherent(self.shared_memory().host_coherent());

        Ok(unsafe {
            MappedRange {
                ptr: NonNull::new_unchecked(self.mapping.as_ptr().add(start as usize)),
                memory: self.shared_memory().raw(),
                offset: start,
                length: (end - start) as usize,
                coherent,
            }
        })
    }

    fn unmap<D>(&mut self, device: &D, range: Range<u64>)
    where
        D: Device<T>,
    {
    }
}

pub struct ArenaAllocator<T> {
    memory_type: u32,
    memory_properties: Properties,
    arena_size: u64,
    offset: u64,
    arenas: VecDeque<Arena<T>>,
}

struct Arena<T> {
    used: u64,
    free: u64,
    memory: Box<Memory<T>>,
    mapping: NonNull<u8>,
}

impl<T> ArenaAllocator<T> {
    fn cleanup<D>(&mut self, device: &D) -> u64
    where
        D: Device<T>,
    {
        let mut freed = 0;
        while self.arenas.len() > 1 {
            if self.arenas[0].used > self.arenas[0].free {
                break;
            }

            let arena = self.arenas.pop_front().unwrap();

            unsafe {
                device.unmap(arena.memory.raw());

                freed += arena.memory.size();
                device.free(arena.memory.into_raw());
            }
        }
        freed
    }
}

impl<T> Allocator<T> for ArenaAllocator<T> {
    type Block = ArenaBlock<T>;

    fn alloc<D>(&mut self, device: &D, size: u64, align: u64) -> Result<(ArenaBlock<T>, u64), MemoryError>
    where
        D: Device<T>,
    {
        debug_assert!(self.memory_properties.host_visible());

        if size > self.arena_size {
            return Err(OutOfMemoryError::OutOfDeviceMemory.into());
        }

        let count = self.arenas.len() as u64;
        if let Some(arena) = self.arenas.back_mut() {
            let aligned = util::aligned(arena.used, align);
            if self.arena_size - aligned > size {
                arena.used = aligned + size;
                return Ok((ArenaBlock {
                    index: self.offset + count - 1,
                    memory: &*arena.memory,
                    mapping: arena.mapping,
                    range: aligned .. arena.used,
                }, 0));
            }
        }

        let (memory, mapping) = unsafe {
            let raw = device.allocate(self.memory_type, self.arena_size)?;
            let mapping = match device.map(&raw, 0 .. self.arena_size) {
                Ok(mapping) => mapping,
                Err(error) => {
                    device.free(raw);
                    return Err(error.into())
                }
            };
            let memory = Memory::from_raw(raw, self.arena_size, self.memory_properties);
            (memory, mapping)
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
        Ok((block, self.arena_size))
    }

    fn free<D>(&mut self, device: &D, block: Self::Block) -> u64
    where   
        D: Device<T>,
    {
        {
            let index = block.index - self.offset;
            assert!(util::fits_in_usize(index));
            let index = index as usize;
            assert!(index < self.arenas.len());
            let ref mut arena = self.arenas[index];
            arena.free += block.range.end - block.range.start;
        }

        self.cleanup(device)
    }
}

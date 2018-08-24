//! Fast sub-allocator for short-living allocations.
//! Typically used for staging buffers.
//! This allocator allocate memory directly from device and maps whole range.

use std::{collections::VecDeque, ops::Range, slice::from_raw_parts_mut, ptr::NonNull};

use allocator::Allocator;
use block::Block;
use device::Device;
use error::*;
use mapping::*;
use memory::*;
use util::*;

#[derive(Debug)]
pub struct ArenaBlock<T> {
    memory: *const Memory<T>,
    arena_index: u64,
    ptr: NonNull<u8>,
    range: Range<u64>,
}

impl<T> ArenaBlock<T> {
    fn shared_memory(&self) -> &Memory<T> {
        // Memory won't be freed until last block created from it deallocated.
        unsafe { &*self.memory }
    }

    fn size(&self) -> u64 {
        self.range.end - self.range.start
    }
}

impl<T: 'static> Block for ArenaBlock<T> {

    type Memory = T;

    #[inline]
    fn properties(&self) -> Properties {
        self.shared_memory().properties
    }

    #[inline]
    fn memory(&self) -> &T {
        self.shared_memory().raw()
    }

    #[inline]
    fn range(&self) -> Range<u64> {
        self.range.clone()
    }

    #[inline]
    fn map<'a, D>(&'a mut self, _device: &D, range: Range<u64>) -> Result<MappedRange<'a, T>, MappingError> {
        debug_assert!(self.shared_memory().host_visible());
        let mapping = unsafe {
            MappedRange::from_raw(self.shared_memory(), self.ptr, self.range.clone())
        };
        Ok(mapping)
    }

    #[inline]
    fn unmap<D>(&mut self, _device: &D, range: Range<u64>) {
        debug_assert!(self.shared_memory().host_visible());
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ArenaConfig {
    /// Size of the arena chunk.
    /// Keep it big.
    pub arena_size: u64,
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
    ptr: NonNull<u8>,
}

impl<T: 'static> ArenaAllocator<T> {

    pub fn properties_required() -> Properties {
        Properties::HOST_VISIBLE
    }

    pub fn max_allocation(&self) -> u64 {
        self.arena_size / 2
    }

    pub fn new(
        memory_type: u32,
        memory_properties: Properties,
        config: ArenaConfig,
    ) -> Self {
        assert!(memory_properties.contains(Self::properties_required()));
        assert!(fits_usize(config.arena_size), "Arena size must fit in both usize and u64");
        ArenaAllocator {
            memory_type,
            memory_properties,
            arena_size: config.arena_size,
            offset: 0,
            arenas: VecDeque::new(),
        }
    }

    fn cleanup<D>(&mut self, device: &D) -> u64
    where
        D: Device<Memory = T>,
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

impl<T: 'static> Allocator for ArenaAllocator<T> {

    type Memory = T;

    type Block = ArenaBlock<T>;

    fn alloc<D>(&mut self, device: &D, size: u64, align: u64) -> Result<(ArenaBlock<T>, u64), MemoryError>
    where
        D: Device<Memory = T>,
    {
        debug_assert!(self.memory_properties.host_visible());

        if size > self.arena_size || align > self.arena_size {
            return Err(OutOfMemoryError::OutOfDeviceMemory.into());
        }

        let count = self.arenas.len() as u64;
        if let Some(arena) = self.arenas.back_mut() {
            let aligned = aligned(arena.used, align);
            if self.arena_size - size > aligned {
                arena.used = aligned + size;
                let (ptr, range) = mapped_sub_range(arena.ptr, 0 .. self.arena_size, aligned .. aligned + size).expect("This sub-range must fit in arena mapping");
                return Ok((
                    ArenaBlock {
                        arena_index: self.offset + count - 1,
                        memory: &*arena.memory,
                        ptr,
                        range, 
                    },
                0));
            }
        }

        let (memory, ptr) = unsafe {
            let raw = device.allocate(self.memory_type, self.arena_size)?;

            let ptr = match device.map(&raw, 0 .. self.arena_size) {
                Ok(ptr) => ptr,
                Err(error) => {
                    device.free(raw);
                    return Err(error.into())
                }
            };

            let memory = Memory::from_raw(raw, self.arena_size, self.memory_properties);

            (memory, ptr)
        };

        let arena = Arena {
            used: size,
            free: 0,
            ptr,
            memory: Box::new(memory),
        };

        let (ptr, range) = mapped_sub_range(ptr, 0 .. self.arena_size, 0 .. size).expect("This sub-range must fit in arena mapping");

        let block = ArenaBlock {
            arena_index: self.offset + count,
            memory: &*arena.memory,
            ptr,
            range,
        };

        self.arenas.push_back(arena);
        Ok((block, self.arena_size))
    }

    fn free<D>(&mut self, device: &D, block: Self::Block) -> u64
    where   
        D: Device<Memory = T>,
    {
        {
            let index = block.arena_index - self.offset;
            assert!(fits_usize(index), "This can't exceed arenas list length which fits into usize by definition");
            let index = index as usize;
            assert!(index < self.arenas.len(), "Can't be allocated from not yet created arena");
            let ref mut arena = self.arenas[index];
            arena.free += block.size();
        }

        self.cleanup(device)
    }
}

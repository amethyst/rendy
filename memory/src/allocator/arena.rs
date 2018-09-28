use std::{collections::VecDeque, fmt::Debug, ops::Range, ptr::NonNull};

use relevant::Relevant;

use allocator::Allocator;
use block::Block;
use device::Device;
use error::*;
use mapping::*;
use memory::*;
use util::*;

/// Memory block allocated from `ArenaAllocator`
#[derive(Derivative)]
#[derivative(Debug)]
pub struct ArenaBlock<T> {
    #[derivative(Debug(bound = "T: Debug", format_with = "super::memory_ptr_fmt"))]
    memory: *const Memory<T>,
    arena_index: u64,
    ptr: NonNull<u8>,
    range: Range<u64>,
    #[derivative(Debug = "ignore")]
    relevant: Relevant,
}

unsafe impl<T: Send> Send for ArenaBlock<T> {}
unsafe impl<T: Sync> Sync for ArenaBlock<T> {}

impl<T> ArenaBlock<T> {
    fn shared_memory(&self) -> &Memory<T> {
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

impl<T: 'static> Block for ArenaBlock<T> {
    type Memory = T;

    #[inline]
    fn properties(&self) -> Properties {
        self.shared_memory().properties()
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
    fn map<'a, D>(
        &'a mut self,
        _device: &D,
        range: Range<u64>,
    ) -> Result<MappedRange<'a, T>, MappingError> {
        assert!(
            range.start <= range.end,
            "Memory mapping region must have valid size"
        );
        debug_assert!(self.shared_memory().host_visible());

        if let Some((ptr, range)) = mapped_sub_range(self.ptr, self.range.clone(), range) {
            let mapping = unsafe { MappedRange::from_raw(self.shared_memory(), ptr, range) };
            Ok(mapping)
        } else {
            Err(MappingError::OutOfBounds)
        }
    }

    #[inline]
    fn unmap<D>(&mut self, _device: &D) {
        debug_assert!(self.shared_memory().host_visible());
    }
}

/// Config for `DynamicAllocator`.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ArenaConfig {
    /// Size of the arena chunk.
    /// Keep it big.
    pub arena_size: u64,
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
pub struct ArenaAllocator<T> {
    memory_type: u32,
    memory_properties: Properties,
    arena_size: u64,
    offset: u64,
    arenas: VecDeque<Arena<T>>,
}

#[derive(Derivative)]
#[derivative(Debug)]
struct Arena<T> {
    used: u64,
    free: u64,
    #[derivative(Debug = "ignore")]
    memory: Box<Memory<T>>,
    ptr: NonNull<u8>,
}

unsafe impl<T: Send> Send for Arena<T> {}
unsafe impl<T: Sync> Sync for Arena<T> {}

impl<T: 'static> ArenaAllocator<T> {
    /// Get properties required by the allocator.
    pub fn properties_required() -> Properties {
        Properties::HOST_VISIBLE
    }

    /// Maximum allocation size.
    pub fn max_allocation(&self) -> u64 {
        self.arena_size / 2
    }

    /// Create new `ArenaAllocator`
    /// for `memory_type` with `memory_properties` specified,
    /// with `ArenaConfig` provided.
    pub fn new(memory_type: u32, memory_properties: Properties, config: ArenaConfig) -> Self {
        assert!(memory_properties.contains(Self::properties_required()));
        assert!(
            fits_usize(config.arena_size),
            "Arena size must fit in both usize and u64"
        );
        ArenaAllocator {
            memory_type,
            memory_properties,
            arena_size: config.arena_size,
            offset: 0,
            arenas: VecDeque::new(),
        }
    }

    /// Perform full cleanup of the memory allocated.
    pub fn dispose<D>(mut self, device: &D)
    where
        D: Device<Memory = T>,
    {
        self.cleanup(device, 0);
        assert!(
            self.arenas.is_empty(),
            "Arenas are not empty during allocator disposal. Arenas: {:#?}",
            self.arenas
        );
    }

    fn cleanup<D>(&mut self, device: &D, off: usize) -> u64
    where
        D: Device<Memory = T>,
    {
        let mut freed = 0;
        while self.arenas.len() > off {
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

    fn alloc<D>(
        &mut self,
        device: &D,
        size: u64,
        align: u64,
    ) -> Result<(ArenaBlock<T>, u64), MemoryError>
    where
        D: Device<Memory = T>,
    {
        debug_assert!(self.memory_properties.host_visible());

        assert!(size <= self.arena_size);
        assert!(align <= self.arena_size);

        let count = self.arenas.len() as u64;
        if let Some(arena) = self.arenas.back_mut() {
            let aligned = aligned(arena.used, align);
            let overhead = aligned - arena.used;
            if self.arena_size - size > aligned {
                arena.used = aligned + size;
                arena.free += overhead;
                let (ptr, range) =
                    mapped_sub_range(arena.ptr, 0..self.arena_size, aligned..aligned + size)
                        .expect("This sub-range must fit in arena mapping");

                return Ok((
                    ArenaBlock {
                        arena_index: self.offset + count - 1,
                        memory: &*arena.memory,
                        ptr,
                        range,
                        relevant: Relevant,
                    },
                    0,
                ));
            }
        }

        let (memory, ptr) = unsafe {
            let raw = device.allocate(self.memory_type, self.arena_size)?;

            let ptr = match device.map(&raw, 0..self.arena_size) {
                Ok(ptr) => ptr,
                Err(error) => {
                    device.free(raw);
                    return Err(error.into());
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

        let (ptr, range) = mapped_sub_range(ptr, 0..self.arena_size, 0..size)
            .expect("This sub-range must fit in arena mapping");

        let block = ArenaBlock {
            arena_index: self.offset + count,
            memory: &*arena.memory,
            ptr,
            range,
            relevant: Relevant,
        };

        self.arenas.push_back(arena);
        Ok((block, self.arena_size))
    }

    fn free<D>(&mut self, device: &D, block: Self::Block) -> u64
    where
        D: Device<Memory = T>,
    {
        let index = block.arena_index - self.offset;
        assert!(
            fits_usize(index),
            "This can't exceed arenas list length which fits into usize by definition"
        );
        let index = index as usize;
        assert!(
            index < self.arenas.len(),
            "Can't be allocated from not yet created arena"
        );
        {
            let ref mut arena = self.arenas[index];
            arena.free += block.size();
        }
        block.dispose();

        self.cleanup(device, 1)
    }
}

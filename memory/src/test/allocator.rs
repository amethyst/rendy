#![allow(dead_code)]

use std::{cell::RefCell, collections::HashSet, ops::Range, ptr::NonNull};

use rand;
use veclist::VecList;

use allocator::{ArenaConfig, DynamicConfig};
use block::Block;
use device::Device;
use error::{AllocationError, MappingError, MemoryError, OutOfMemoryError};
use heaps::{Config, Heaps, MemoryBlock};
use memory::Properties;
use usage::*;

struct Inner {
    freed: HashSet<u64>,
    next: u64,
}

struct MockDevice(RefCell<Inner>);

impl MockDevice {
    fn new() -> Self {
        MockDevice(RefCell::new(Inner {
            freed: HashSet::new(),
            next: 0,
        }))
    }
}

impl Device for MockDevice {
    type Memory = u64;

    unsafe fn allocate(&self, _index: u32, _size: u64) -> Result<u64, AllocationError> {
        let mut inner = self.0.borrow_mut();
        let id = inner.next;
        inner.next = id + 1;
        Ok(id)
    }

    unsafe fn free(&self, memory: u64) {
        assert!(self.0.borrow_mut().freed.insert(memory), "Double-free");
    }

    unsafe fn map(&self, _memory: &u64, _range: Range<u64>) -> Result<NonNull<u8>, MappingError> {
        Ok(NonNull::dangling())
    }

    unsafe fn unmap(&self, _memory: &u64) {}

    unsafe fn invalidate<'a>(
        &self,
        _regions: impl IntoIterator<Item = (&'a u64, Range<u64>)>,
    ) -> Result<(), OutOfMemoryError> {
        unimplemented!()
    }
    unsafe fn flush<'a>(
        &self,
        _regions: impl IntoIterator<Item = (&'a u64, Range<u64>)>,
    ) -> Result<(), OutOfMemoryError> {
        unimplemented!()
    }
}

fn init() -> Heaps<u64> {
    let arena_config = ArenaConfig {
        arena_size: 32 * 1024,
    };
    let dynamic_config = DynamicConfig {
        blocks_per_chunk: 64,
        block_size_granularity: 256,
        max_block_size: 32 * 1024,
    };
    let small_dynamic_config = DynamicConfig {
        blocks_per_chunk: 64,
        block_size_granularity: 32,
        max_block_size: 1024,
    };

    unsafe {
        Heaps::new(
            vec![
                (
                    Properties::DEVICE_LOCAL,
                    0,
                    Config {
                        arena: None,
                        dynamic: Some(dynamic_config),
                    },
                ),
                (
                    Properties::HOST_VISIBLE | Properties::HOST_COHERENT | Properties::DEVICE_LOCAL,
                    1,
                    Config {
                        arena: None,
                        dynamic: Some(small_dynamic_config),
                    },
                ),
                (
                    Properties::HOST_VISIBLE | Properties::HOST_COHERENT,
                    2,
                    Config {
                        arena: Some(arena_config),
                        dynamic: Some(dynamic_config),
                    },
                ),
                (
                    Properties::HOST_VISIBLE | Properties::HOST_COHERENT | Properties::HOST_CACHED,
                    2,
                    Config {
                        arena: Some(arena_config),
                        dynamic: Some(dynamic_config),
                    },
                ),
            ],
            vec![16 * 1024 * 1024, 1 * 1024 * 1024, 32 * 1024 * 1024],
        )
    }
}

fn random_usage() -> UsageValue {
    match rand::random::<u8>() % 4 {
        0 => UsageValue::Data,
        1 => UsageValue::Download,
        2 => UsageValue::Upload,
        3 => UsageValue::Dynamic,
        _ => unreachable!(),
    }
}

#[derive(Debug)]
struct Allocation {
    mask: u32,
    usage: UsageValue,
    size: u64,
    align: u64,
}

impl Allocation {
    fn random() -> Self {
        let usage = random_usage();

        let mask = (rand::random::<u32>() % 3) | (1 << rand::random::<u32>() % 2);

        let mask = match usage {
            UsageValue::Data => mask,
            _ => mask << 1,
        };

        Allocation {
            mask,
            usage,
            size: 1 << (rand::random::<u32>() % 10),
            align: 1 << (rand::random::<u32>() % 10),
        }
    }

    fn allocate(
        &self,
        heaps: &mut Heaps<u64>,
        device: &MockDevice,
    ) -> Result<MemoryBlock<u64>, MemoryError> {
        let block = heaps.allocate(device, self.mask, self.usage, self.size, self.align)?;

        assert!(block.range().end - block.range().start >= self.size);
        assert_eq!(
            block.range().start % self.align,
            0,
            "Block: {:#?} allocated without requested align {}",
            block,
            self.align
        );
        assert!(self.usage.memory_fitness(block.properties()).is_some());
        assert_ne!((1 << block.memory_type()) & self.mask, 0);
        Ok(block)
    }
}

#[test]
fn heaps_init() {
    let heaps = init();
    drop(heaps);
}

#[test]
fn blocks_test() {
    let mut heaps = init();
    let ref device = MockDevice::new();
    let mut blocks = VecList::new();

    for _ in 0..32 {
        match rand::random::<u8>() % 2 {
            0 => {
                let allocation = Allocation::random();
                match allocation.allocate(&mut heaps, &device) {
                    Ok(block) => {
                        blocks.push(block);
                    }
                    Err(err) => {
                        panic!(
                            "Error({}) occurred for {:#?}. Blocks: {:#?}",
                            err, allocation, blocks
                        );
                    }
                }
            }
            _ if blocks.upper_bound() > 1 => {
                let index = rand::random::<usize>() % blocks.upper_bound();
                if let Some(block) = blocks.pop(index) {
                    heaps.free(device, block);
                }
            }
            _ => {}
        }
    }

    for i in 0..blocks.upper_bound() {
        if let Some(block) = blocks.pop(i) {
            heaps.free(device, block);
        }
    }

    drop(blocks);

    println!("Dropping Heaps");
    heaps.dispose(device);
}

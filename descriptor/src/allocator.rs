use {
    crate::{layout::*, ranges::*},
    gfx_hal::{
        device::OutOfMemory,
        pso::{AllocationError, DescriptorPool as _},
        Backend, Device,
    },
    smallvec::{smallvec, SmallVec},
    std::{
        cmp::{max, min},
        collections::{HashMap, VecDeque},
    },
};

const MIN_SETS: u32 = 64;

/// Descriptor set
#[derive(Debug)]
pub struct DescriptorSet<B: Backend> {
    raw: B::DescriptorSet,
    pool: u64,
    ranges: DescriptorRanges,
}

impl<B> DescriptorSet<B>
where
    B: Backend,
{
    /// Get raw set
    pub fn raw(&self) -> &B::DescriptorSet {
        &self.raw
    }
}

#[derive(Debug)]
struct Allocation<B: Backend> {
    sets: Vec<B::DescriptorSet>,
    pools: Vec<u64>,
}

#[derive(Debug)]
struct DescriptorPool<B: Backend> {
    raw: B::DescriptorPool,
    size: u32,
    free: u32,
}

unsafe fn allocate_from_pool<B: Backend>(
    raw: &mut B::DescriptorPool,
    layout: &B::DescriptorSetLayout,
    count: u32,
    allocation: &mut Vec<B::DescriptorSet>,
) -> Result<(), OutOfMemory> {
    let sets_were = allocation.len();
    raw.allocate_sets(std::iter::repeat(layout).take(count as usize), allocation)
        .map_err(|err| match err {
            AllocationError::OutOfHostMemory => OutOfMemory::OutOfHostMemory,
            AllocationError::OutOfDeviceMemory => OutOfMemory::OutOfDeviceMemory,
            err => {
                panic!("Unexpected error: {}", err);
            }
        })?;
    assert_eq!(allocation.len(), sets_were + count as usize);
    Ok(())
}

#[derive(Debug)]
struct DescriptorBucket<B: Backend> {
    pools_offset: u64,
    pools: VecDeque<DescriptorPool<B>>,
    total: u64,
}

impl<B> DescriptorBucket<B>
where
    B: Backend,
{
    fn new() -> Self {
        DescriptorBucket {
            pools_offset: 0,
            pools: VecDeque::new(),
            total: 0,
        }
    }

    unsafe fn dispose(mut self, device: &impl Device<B>) {
        if self.total > 0 {
            log::error!("Not all descriptor sets were deallocated");
        }

        if !self.pools.is_empty() {
            log::error!(
                "Descriptor pools are still in use during allocator disposal. {:?}",
                self.pools
            );
        }

        self.pools
            .drain(..)
            .for_each(|pool| device.destroy_descriptor_pool(pool.raw));
    }

    unsafe fn allocate(
        &mut self,
        device: &impl Device<B>,
        layout: &DescriptorSetLayout<B>,
        mut count: u32,
        allocation: &mut Allocation<B>,
    ) -> Result<(), OutOfMemory> {
        if count == 0 {
            return Ok(());
        }

        for (index, pool) in self.pools.iter_mut().enumerate().rev() {
            if pool.free == 0 {
                continue;
            }

            let allocate = min(pool.free, count);
            log::trace!("Allocate {} from exising pool", allocate);
            allocate_from_pool::<B>(&mut pool.raw, layout.raw(), allocate, &mut allocation.sets)?;
            allocation.pools.extend(
                std::iter::repeat(index as u64 + self.pools_offset).take(allocate as usize),
            );
            count -= allocate;
            pool.free -= allocate;
            self.total += allocate as u64;

            if count == 0 {
                return Ok(());
            }
        }

        if count > 0 {
            let size = max(MIN_SETS, (count - 1).next_power_of_two() * 2);
            let pool_ranges = layout.ranges() * size;
            log::trace!(
                "Create new pool with {} sets and {:?} descriptors. And allocate {} sets from it",
                size,
                pool_ranges,
                count
            );
            let mut raw = device.create_descriptor_pool(size as usize, &pool_ranges)?;
            allocate_from_pool::<B>(&mut raw, layout.raw(), count, &mut allocation.sets)?;
            allocation.pools.extend(
                std::iter::repeat(self.pools.len() as u64 + self.pools_offset).take(count as usize),
            );
            self.pools.push_back(DescriptorPool {
                raw,
                size,
                free: size - count,
            });
            self.total += count as u64;
        }

        Ok(())
    }

    unsafe fn free(&mut self, sets: impl IntoIterator<Item = B::DescriptorSet>, pool: u64) {
        let pool = &mut self.pools[(pool - self.pools_offset) as usize];
        let mut freed = 0;
        pool.raw.free_sets(sets.into_iter().inspect(|_| freed += 1));
        pool.free += freed;
        self.total -= freed as u64;
    }

    unsafe fn cleanup(&mut self, device: &impl Device<B>) {
        while let Some(pool) = self.pools.pop_front() {
            if pool.free < pool.size {
                self.pools.push_front(pool);
                break;
            }
            device.destroy_descriptor_pool(pool.raw);
            self.pools_offset += 1;
        }
    }
}

#[derive(Debug)]
pub struct DescriptorAllocator<B: Backend> {
    buckets: HashMap<DescriptorRanges, DescriptorBucket<B>>,
    allocation: Allocation<B>,
    relevant: relevant::Relevant,
    total: u64,
}

impl<B> DescriptorAllocator<B>
where
    B: Backend,
{
    pub fn new() -> Self {
        DescriptorAllocator {
            buckets: HashMap::new(),
            allocation: Allocation {
                sets: Vec::new(),
                pools: Vec::new(),
            },
            relevant: relevant::Relevant,
            total: 0,
        }
    }

    pub unsafe fn dispose(mut self, device: &impl Device<B>) {
        self.cleanup(device);
        self.buckets
            .drain()
            .for_each(|(_, bucket)| bucket.dispose(device));
        self.relevant.dispose();
    }

    pub unsafe fn allocate(
        &mut self,
        device: &impl Device<B>,
        layout: &DescriptorSetLayout<B>,
        count: u32,
        extend: &mut impl Extend<DescriptorSet<B>>,
    ) -> Result<(), OutOfMemory> {
        if count == 0 {
            return Ok(());
        }

        let layout_ranges = layout.ranges();
        log::trace!(
            "Allocating {} sets with layout {:?} @ {:?}",
            count,
            layout,
            layout_ranges
        );

        let bucket = self
            .buckets
            .entry(layout_ranges)
            .or_insert_with(|| DescriptorBucket::new());
        match bucket.allocate(device, layout, count, &mut self.allocation) {
            Ok(()) => {
                extend.extend(
                    Iterator::zip(
                        self.allocation.pools.drain(..),
                        self.allocation.sets.drain(..),
                    )
                    .map(|(pool, set)| DescriptorSet {
                        raw: set,
                        ranges: layout_ranges,
                        pool,
                    }),
                );
                Ok(())
            }
            Err(err) => {
                // Free sets allocated so far.
                let mut last = None;
                for (index, pool) in self.allocation.pools.drain(..).enumerate().rev() {
                    match last {
                        Some(last) if last == pool => {
                            // same pool, continue
                        }
                        Some(last) => {
                            // Free contiguous range of sets from one pool in one go.
                            bucket.free(self.allocation.sets.drain(index + 1..), last);
                        }
                        None => last = Some(pool),
                    }
                }

                if let Some(last) = last {
                    bucket.free(self.allocation.sets.drain(0..), last);
                }

                Err(err)
            }
        }
    }

    pub unsafe fn free(&mut self, sets: impl IntoIterator<Item = DescriptorSet<B>>) {
        let mut free: Option<(DescriptorRanges, u64, SmallVec<[B::DescriptorSet; 32]>)> = None;

        // Collect contig
        for set in sets {
            match &mut free {
                slot @ None => {
                    slot.replace((set.ranges, set.pool, smallvec![set.raw]));
                }
                Some((ranges, pool, sets)) if *ranges == set.ranges && *pool == set.pool => {
                    sets.push(set.raw);
                }
                Some((ranges, pool, sets)) => {
                    self.buckets
                        .get_mut(ranges)
                        .expect("Set should be allocated from this allocator")
                        .free(sets.drain(), *pool);
                    *pool = set.pool;
                    sets.push(set.raw);
                }
            }
        }

        if let Some((ranges, pool, sets)) = free {
            self.buckets
                .get_mut(&ranges)
                .expect("Set should be allocated from this allocator")
                .free(sets, pool);
        }
    }

    pub unsafe fn cleanup(&mut self, device: &impl Device<B>) {
        self.buckets
            .values_mut()
            .for_each(|bucket| bucket.cleanup(device));
    }
}

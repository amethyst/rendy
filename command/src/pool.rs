//! Pool module docs.

use ash::{version::DeviceV1_0, vk::{CommandBuffer, QueueFlags}};

use relevant::Relevant;

use crate::{
    buffer::*,
    capability::*,
    family::FamilyId,
};

/// Simple pool wrapper.
/// Doesn't provide any guarantees.
/// Wraps raw buffers into `Buffer`.
#[derive(Debug)]
pub struct Pool<P, C = QueueFlags, R = ()> {
    inner: P,
    capability: C,
    reset: R,
    family: FamilyId,
    relevant: Relevant,
}

impl<P, C, R> Pool<P, C, R> {
    /// Allocate new buffer.
    pub fn allocate_buffers<L: Level>(
        &mut self,
        device: &impl DeviceV1_0,
        level: L,
        count: usize,
    ) -> Vec<Buffer<C, InitialState, L, R>> {
        unimplemented!()
    }

    /// Free buffers.
    /// Buffers must be in droppable state.
    pub fn free_buffers(
        &mut self,
        device: &impl DeviceV1_0,
        buffers: Vec<Buffer<C, impl Droppable, impl Level, R>>,
    ) {
        unimplemented!()
    }

    /// Reset all buffers of this pool.
    pub unsafe fn reset(&mut self) {
        unimplemented!()
    }
}

impl<P, R> Pool<P, QueueFlags, R> {
    /// Convert capability level
    pub fn from_flags<C>(self) -> Result<Pool<P, C, R>, Self>
    where
        C: Capability,
    {
        if let Some(capability) = C::from_flags(self.capability) {
            Ok(Pool {
                inner: self.inner,
                capability,
                reset: self.reset,
                family: self.family,
                relevant: self.relevant,
            })
        } else {
            Err(self)
        }
    }
}

/// Command pool that owns allocated buffers.
/// It can be used to borrow buffers one by one.
/// All buffers will be reset together via pool.
/// Prior reset user must ensure all buffers are complete.
#[derive(Debug)]
pub struct OwningPool<P, B, C = QueueFlags, R = ()> {
    inner: Pool<P, C, R>,
    buffers: Vec<B>,
    next: usize,
}

impl<P, B, C, R> OwningPool<P, B, C, R> {
    /// Reserve at least `count` buffers.
    /// Allocate if there are not enough unused buffers.
    pub fn reserve(&mut self, count: usize) {
        unimplemented!()
    }

    /// Acquire command buffer from pool.
    /// The command buffer could be submitted only as part of submission for associated frame.
    /// TODO: Check that buffer cannot be moved out.
    pub fn acquire_buffer<L>(
        &mut self,
        device: &impl DeviceV1_0,
        level: L,
    ) -> Buffer<C, InitialState, L, R> {
        unimplemented!()
    }

    /// Reset all buffers at once.
    ///
    /// # Safety
    ///
    /// All buffers from this pool must be in resettable state.
    /// Any primary buffer that references secondary buffer from this pool will be invalidated.
    pub unsafe fn reset(&mut self) {
        unimplemented!()
    }
}

impl<P, B, R> OwningPool<P, B, QueueFlags, R> {
    /// Convert capability level
    pub fn from_flags<C>(self) -> Result<OwningPool<P, B, C, R>, Self>
    where
        C: Capability,
    {
        match self.inner.from_flags::<C>() {
            Ok(inner) => Ok(OwningPool {
                inner,
                buffers: self.buffers,
                next: self.next,
            }),
            Err(inner) => Err(OwningPool {
                inner,
                buffers: self.buffers,
                next: self.next,
            }),
        }
    }
}

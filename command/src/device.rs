//! Device module docs.

use std::{borrow::Borrow, fmt::Debug};

use resource;
use fence::FenceCreateInfo;

/// Abstract logical device.
/// It inherits methods to allocate memory and create resources.
pub trait Device: resource::Device {
    /// Semaphore type that can be used with this device.
    type Semaphore: Debug + 'static;

    /// Fence type that can be used with this device.
    type Fence: Debug + 'static;

    /// Finished command buffer that can be submitted to the queues of this device.
    type Submit: 'static;

    /// Command pool type that can be used with this device.
    type CommandPool: 'static;

    /// Command buffer type that can be used with this device.
    type CommandBuffer: CommandBuffer<Submit = Self::Submit> + 'static;

    /// Command queue type that can be used with this device.
    type CommandQueue: CommandQueue<
            Semaphore = Self::Semaphore,
            Fence = Self::Fence,
            Submit = Self::Submit,
        > + 'static;

    /// Create new fence.
    unsafe fn create_fence(&self, info: FenceCreateInfo) -> Self::Fence;

    /// Reset fence.
    unsafe fn reset_fence(&self, fence: &Self::Fence) {
        self.reset_fences(Some(fence))
    }

    /// Reset multiple fences at once.
    unsafe fn reset_fences<F>(&self, fences: F)
    where
        F: IntoIterator,
        F::Item: Borrow<Self::Fence>,
    {
        fences.into_iter().for_each(|fence| self.reset_fence(fence.borrow()));
    }
}

/// Abstract command buffer.
/// It defines all methods required to begin/end recording and record commands.
pub trait CommandBuffer {
    /// This type is `Device::CommandBuffer` of device that created pool from which this buffer is allocated.
    /// Raw command buffer can be cloned.
    type Submit;

    /// Get submittable object.
    /// Buffer must be in executable state.
    unsafe fn submit(&self) -> Self::Submit;
}

impl<'a, B: 'a> CommandBuffer for &'a mut B
where
    B: CommandBuffer,
{
    type Submit = B::Submit;

    unsafe fn submit(&self) -> B::Submit {
        B::submit(&**self)
    }
}

/// Abstract command queue.
/// It defines methods for submitting command buffers along with semaphores and fences.
pub trait CommandQueue {
    /// Semaphore type that can be used with this device.
    type Semaphore: Debug + 'static;

    /// Fence type that can be used with this device.
    type Fence: Debug + 'static;

    /// Finished command buffer that can be submitted to the queue.
    type Submit: 'static;
}

impl<'a, Q: 'a> CommandQueue for &'a mut Q
where
    Q: CommandQueue,
{
    type Semaphore = Q::Semaphore;
    type Fence = Q::Fence;
    type Submit = Q::Submit;
}

//! Device module docs.

use std::fmt::Debug;

use resource;

/// Abstract logical device.
/// It inherits methods to allocate memory and create resources.
pub trait Device: resource::Device {
    /// Semaphore type that can be used with this device.
    type Semaphore: Debug;

    /// Fence type that can be used with this device.
    type Fence: Debug;

    /// Finished command buffer that can be submitted to the queues of this device.
    type Submit: Debug;

    /// Command pool type that can be used with this device.
    type CommandPool;

    /// Command buffer type that can be used with this device.
    type CommandBuffer: CommandBuffer<Submit = Self::Submit> + Debug;

    /// Command queue type that can be used with this device.
    type CommandQueue: CommandQueue<Semaphore = Self::Semaphore, Fence = Self::Fence, Submit = Self::Submit> + Debug;
}

/// Abstract command buffer.
/// It defines all methods required to begin/end recording and record commands.
pub trait CommandBuffer {
    /// This type is `Device::CommandBuffer` of device that created pool from which this buffer is allocated.
    /// Raw command buffer can be cloned.
    type Submit: Debug;

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
    type Semaphore: Debug;

    /// Fence type that can be used with this device.
    type Fence: Debug;

    /// Finished command buffer that can be submitted to the queue.
    type Submit: Debug;
}

impl<'a, Q: 'a> CommandQueue for &'a mut Q
where
    Q: CommandQueue,
{
    type Semaphore = Q::Semaphore;
    type Fence = Q::Fence;
    type Submit = Q::Submit;
}
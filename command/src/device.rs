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

    /// Command pool type that can be used with this device.
    type CommandPool;

    /// Command buffer type that can be used with this device.
    type CommandBuffer: CommandBuffer + Debug;

    /// Command queue type that can be used with this device.
    type CommandQueue: CommandQueue<Semaphore = Self::Semaphore, Fence = Self::Fence, CommandBuffer = Self::CommandBuffer> + Debug;
}

/// Abstract command buffer.
/// It defines all methods required to begin/end recording and record commands.
pub trait CommandBuffer {}

impl<'a, B: 'a> CommandBuffer for &'a mut B
where
    B: CommandBuffer,
{}

/// Abstract command queue.
/// It defines methods for submitting command buffers along with semaphores and fences.
pub trait CommandQueue {
    /// Semaphore type that can be used with this device.
    type Semaphore: Debug;

    /// Fence type that can be used with this device.
    type Fence: Debug;

    /// Command buffer type that can be used with this device.
    type CommandBuffer: Debug;
}

impl<'a, Q: 'a> CommandQueue for &'a mut Q
where
    Q: CommandQueue,
{
    type Semaphore = Q::Semaphore;
    type Fence = Q::Fence;
    type CommandBuffer = Q::CommandBuffer;
}
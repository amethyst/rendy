
use {
    crate::family::QueueId,
    gfx_hal::{Backend, Device, queue::RawCommandQueue},
    std::{collections::VecDeque, ops::Range, cmp::max},
};

/// Fence is not set.
#[derive(Debug)]
pub struct UnarmedFence<B: Backend> {
    raw: B::Fence
}

impl<B> UnarmedFence<B>
where
    B: Backend,
{
    pub fn new(device: &impl Device<B>) -> Result<Self, gfx_hal::device::OutOfMemory> {
        Ok(UnarmedFence {
            raw: device.create_fence(false)?,
        })
    }

    /// Called by `Queue` after submission.
    pub(crate) fn arm(self, queue: QueueId, epoch: u64) -> ArmedFence<B> {
        ArmedFence {
            raw: self.raw,
            queue,
            epoch,
        }
    }

    pub fn raw(&self) -> &B::Fence {
        &self.raw
    }
}

/// Fence submitted.
#[derive(Debug)]
pub struct ArmedFence<B: Backend> {
    raw: B::Fence,
    queue: QueueId,
    epoch: u64,
}

impl<B> ArmedFence<B>
where
    B: Backend,
{
    pub fn raw(&self) -> &B::Fence {
        &self.raw
    }

    /// Get queue where this fence was submitted.
    pub fn queue(&self) -> QueueId {
        self.queue
    }

    /// Get epoch when this fence was submitted.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Check fence for completion.
    pub unsafe fn is_complete(&self, device: &impl Device<B>) -> Result<bool, gfx_hal::device::DeviceLost> {
        device.get_fence_status(&self.raw)
    }

    /// Try to convert fence to complete.
    pub unsafe fn complete(self, device: &impl Device<B>) -> Result<Result<CompleteFence<B>, Self>, (Self, gfx_hal::device::DeviceLost)> {
        match device.get_fence_status(&self.raw) {
            Ok(true) => Ok(Ok(CompleteFence {
                raw: self.raw,
                queue: self.queue,
                epoch: self.epoch,
            })),
            Ok(false) => Ok(Err(self)),
            Err(err) => Err((self, err)),
        }
    }

    /// Wait for fence to complete.
    pub unsafe fn wait_complete_for(self, device: &impl Device<B>, timeout_ns: u64) -> Result<Result<CompleteFence<B>, Self>, (Self, gfx_hal::device::OomOrDeviceLost)> {
        match device.wait_for_fence(&self.raw, timeout_ns) {
            Ok(true) => Ok(Ok(CompleteFence {
                raw: self.raw,
                queue: self.queue,
                epoch: self.epoch,
            })),
            Ok(false) => Ok(Err(self)),
            Err(err) => Err((self, err)),
        }
    }

    /// Wait for fence to complete.
    pub unsafe fn wait_complete(self, device: &impl Device<B>) -> Result<CompleteFence<B>, (Self, gfx_hal::device::OomOrDeviceLost)> {
        match device.wait_for_fence(&self.raw, !0) {
            Ok(true) => Ok(CompleteFence {
                raw: self.raw,
                queue: self.queue,
                epoch: self.epoch,
            }),
            Ok(false) => panic!("Waiting for !0 should never timeout"),
            Err(err) => Err((self, err))
        }
    }
}

/// Fence set.
#[derive(Debug)]
pub struct CompleteFence<B: Backend> {
    raw: B::Fence,
    queue: QueueId,
    epoch: u64,
}

impl<B> CompleteFence<B>
where
    B: Backend,
{
    pub fn raw(&self) -> &B::Fence {
        &self.raw
    }

    /// Get queue where this fence was submitted.
    pub fn queue(&self) -> QueueId {
        self.queue
    }

    /// Get epoch when this fence was submitted.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Reset fence.
    pub unsafe fn reset(self, device: &impl Device<B>) -> Result<UnarmedFence<B>, (Self, gfx_hal::device::OutOfMemory)> {
        match device.reset_fence(&self.raw) {
            Ok(()) => Ok(UnarmedFence {
                raw: self.raw,
            }),
            Err(err) => Err((self, err)),
        }
    }
}

/// Fence wrapper.
#[derive(Debug)]
pub enum Fence<B: Backend> {
    Unarmed(UnarmedFence<B>),
    Armed(ArmedFence<B>),
    Complete(CompleteFence<B>),
}

impl<B> Fence<B>
where
    B: Backend,
{
    pub fn new(device: &impl Device<B>) -> Result<Self, gfx_hal::device::OutOfMemory> {
        UnarmedFence::new(device).map(Fence::Unarmed)
    }

    pub fn is_unarmed(&self) -> bool {
        match self {
            Fence::Unarmed(_) => true,
            _ => false,
        }
    }

    pub fn is_armed(&self) -> bool {
        match self {
            Fence::Armed(_) => true,
            _ => false,
        }
    }

    pub fn is_complete(&self) -> bool {
        match self {
            Fence::Complete(_) => true,
            _ => false,
        }
    }

    pub fn unarmed(self) -> Result<UnarmedFence<B>, Self> {
        match self {
            Fence::Unarmed(fence) => Ok(fence),
            fence => Err(fence),
        }
    }

    pub fn armed(self) -> Result<ArmedFence<B>, Self> {
        match self {
            Fence::Armed(fence) => Ok(fence),
            fence => Err(fence),
        }
    }

    pub fn complete(self) -> Result<CompleteFence<B>, Self> {
        match self {
            Fence::Complete(fence) => Ok(fence),
            fence => Err(fence),
        }
    }

    /// Must be `Unarmed` before call.
    /// Becomes `Armed` after.
    pub(crate) fn arm(&mut self, queue: QueueId, epoch: u64) {
        match self {
            Fence::Unarmed(UnarmedFence { raw }) => unsafe  {
                let raw = std::ptr::read(raw);
                std::ptr::write(self, Fence::Armed(ArmedFence {
                    raw,
                    queue,
                    epoch,
                }));
            },
            _ => panic!("Must be Unarmed"),
        }
    }

    /// Wait for fence to complete.
    pub unsafe fn wait_complete(&mut self, device: &impl Device<B>, timeout_ns: u64) -> Result<bool, gfx_hal::device::OomOrDeviceLost> {
        match self {
            Fence::Armed(armed) => {
                match std::ptr::read(armed).wait_complete_for(device, timeout_ns) {
                    Ok(Ok(complete)) => {
                        std::ptr::write(self, Fence::Complete(complete));
                        Ok(true)
                    }
                    Ok(Err(armed)) => {
                        std::ptr::write(self, Fence::Armed(armed));
                        Ok(false)
                    }
                    Err((armed, err)) => {
                        std::ptr::write(self, Fence::Armed(armed));
                        Err(err)
                    }
                }
            },
            Fence::Complete(_) => Ok(true),
            _ => Ok(false),
        }
    }

    pub fn raw(&self) -> &B::Fence {
        match self {
            Fence::Unarmed(fence) => &fence.raw,
            Fence::Armed(fence) => &fence.raw,
            Fence::Complete(fence) => &fence.raw,
        }
    }
}
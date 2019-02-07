
use {
    crate::family::QueueId,
    gfx_hal::{Backend, Device, queue::RawCommandQueue},
    std::{collections::VecDeque, ops::Range, cmp::max},
};

/// Queue epoch is the point in particluar queue timeline when fence is submitted.
#[derive(Clone, Copy, Debug)]
pub struct FenceEpoch {
    /// Queue that signals fence.
    pub queue: QueueId,

    /// Queue epoch counter.
    pub epoch: u64,
}

#[derive(Clone, Copy, Debug)]
enum FenceState {
    Unsignaled,
    Signaled,
    Armed(FenceEpoch),
}

/// Fence wrapper.
#[derive(Debug)]
pub struct Fence<B: Backend> {
    raw: B::Fence,
    state: FenceState,
}

impl<B> Fence<B>
where
    B: Backend,
{
    pub fn new(device: &impl Device<B>, signaled: bool) -> Result<Self, gfx_hal::device::OutOfMemory> {
        let raw = device.create_fence(false)?;
        Ok(Fence {
            raw,
            state: if signaled { FenceState::Signaled } else { FenceState::Unsignaled },
        })
    }

    pub fn is_unsignaled(&self) -> bool {
        match self.state {
            FenceState::Unsignaled => true,
            _ => false,
        }
    }

    pub fn is_armed(&self) -> bool {
        match self.state {
            FenceState::Armed(_) => true,
            _ => false,
        }
    }

    pub fn is_signaled(&self) -> bool {
        match self.state {
            FenceState::Signaled => true,
            _ => false,
        }
    }

    /// Must be `Unsignaled` before call.
    /// Becomes `Armed` after.
    pub(crate) fn mark_armed(&mut self, epoch: FenceEpoch) {
        match self.state {
            FenceState::Unsignaled => {
                self.state = FenceState::Armed(epoch);
            },
            _ => panic!("Must be Unsignaled"),
        }
    }

    /// Reset signaled fence
    pub unsafe fn reset(&mut self, device: &impl Device<B>) -> Result<(), gfx_hal::device::OutOfMemory> {
        match self.state {
            FenceState::Signaled => {
                device.reset_fence(&self.raw)?;
                self.state = FenceState::Unsignaled;
                Ok(())
            }
            _ => panic!("Must be signaled"),
        }
    }

    /// Mark signaled fence as reset.
    pub unsafe fn mark_reset(&mut self) {
        match self.state {
            FenceState::Signaled => {
                self.state = FenceState::Unsignaled;
            }
            _ => panic!("Must be signaled"),
        }
    }

    /// Mark fence as signaled.
    pub unsafe fn mark_signaled(&mut self) -> FenceEpoch {
        match self.state {
            FenceState::Armed(epoch) => {
                self.state = FenceState::Signaled;
                epoch
            }
            _ => panic!("Must be armed"),
        }
    }

    /// Wait for fence to become signaled.
    /// On success returns complete fence epoch.
    /// On timeout returns `Ok(None)`
    pub unsafe fn wait_signaled(&mut self, device: &impl Device<B>, timeout_ns: u64) -> Result<Option<FenceEpoch>, gfx_hal::device::OomOrDeviceLost> {
        match self.state {
            FenceState::Armed(epoch) => {
                match device.wait_for_fence(&self.raw, timeout_ns) {
                    Ok(true) => {
                        self.state = FenceState::Signaled;
                        Ok(Some(epoch))
                    }
                    Ok(false) => Ok(None),
                    Err(err) => Err(err),
                }
            },
            _ => panic!("Must be armed"),
        }
    }

    pub fn raw(&self) -> &B::Fence {
        &self.raw
    }
    
    pub fn epoch(&self) -> FenceEpoch {
        match self.state {
            FenceState::Armed(epoch) =>  epoch,
            _ => panic!("Must be armed"),
        }
    }

    pub fn into_inner(self) -> B::Fence {
        match self.state {
            FenceState::Signaled | FenceState::Unsignaled => self.raw,
            _ => panic!("Armed fence must be waited upon before destroying"),
        }
    }
}

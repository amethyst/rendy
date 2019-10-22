use {
    crate::{
        family::QueueId,
        core::{device_owned, Device, DeviceId},
    },
    rendy_core::hal::{device::Device as _, Backend},
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
    Submitted(FenceEpoch),
}

/// Fence wrapper.
#[derive(Debug)]
pub struct Fence<B: Backend> {
    device: DeviceId,
    raw: B::Fence,
    state: FenceState,
}

device_owned!(Fence<B>);

impl<B> Fence<B>
where
    B: Backend,
{
    /// Create new fence in signaled or unsignaled state.
    pub fn new(device: &Device<B>, signaled: bool) -> Result<Self, rendy_core::hal::device::OutOfMemory> {
        let raw = device.raw().create_fence(false)?;
        Ok(Fence {
            device: device.id(),
            raw,
            state: if signaled {
                FenceState::Signaled
            } else {
                FenceState::Unsignaled
            },
        })
    }

    /// Check if fence was submitted.
    pub fn is_submitted(&self) -> bool {
        match self.state {
            FenceState::Submitted(_) => true,
            _ => false,
        }
    }

    /// Check if fence is signaled.
    pub fn is_signaled(&self) -> bool {
        match self.state {
            FenceState::Signaled => true,
            _ => false,
        }
    }

    /// Check if fence is unsignaled.
    /// It can be submitted as well.
    pub fn is_unsignaled(&self) -> bool {
        !self.is_signaled()
    }

    /// Panics if signaled or submitted.
    /// Becomes `Submitted` after.
    pub(crate) fn mark_submitted(&mut self, epoch: FenceEpoch) {
        match self.state {
            FenceState::Unsignaled => {
                self.state = FenceState::Submitted(epoch);
            }
            _ => panic!("Must be Unsignaled"),
        }
    }

    /// Reset signaled fence.
    /// Panics if not signaled.
    /// Becomes unsigneled.
    pub fn reset(&mut self, device: &Device<B>) -> Result<(), rendy_core::hal::device::OutOfMemory> {
        self.assert_device_owner(device);
        match self.state {
            FenceState::Signaled => {
                unsafe { device.reset_fence(&self.raw) }?;
                self.state = FenceState::Unsignaled;
                Ok(())
            }
            _ => panic!("Must be signaled"),
        }
    }

    /// Mark signaled fence as reset.
    /// Panics if not signaled.
    /// Becomes unsigneled.
    /// Fence must be reset using raw fence value.
    pub unsafe fn mark_reset(&mut self) {
        match self.state {
            FenceState::Signaled => {
                self.state = FenceState::Unsignaled;
            }
            _ => panic!("Must be signaled"),
        }
    }

    /// Mark fence as signaled.
    /// Panics if not submitted.
    /// Fence must be checked to be signaled using raw fence value.
    pub unsafe fn mark_signaled(&mut self) -> FenceEpoch {
        match self.state {
            FenceState::Submitted(epoch) => {
                self.state = FenceState::Signaled;
                epoch
            }
            _ => panic!("Must be submitted"),
        }
    }

    /// Wait for fence to become signaled.
    /// Panics if not submitted.
    /// Returns submission epoch on success.
    pub fn wait_signaled(
        &mut self,
        device: &Device<B>,
        timeout_ns: u64,
    ) -> Result<Option<FenceEpoch>, rendy_core::hal::device::OomOrDeviceLost> {
        self.assert_device_owner(device);

        match self.state {
            FenceState::Submitted(epoch) => {
                if unsafe { device.wait_for_fence(&self.raw, timeout_ns) }? {
                    self.state = FenceState::Signaled;
                    Ok(Some(epoch))
                } else {
                    Ok(None)
                }
            }
            _ => panic!("Must be submitted"),
        }
    }

    /// Check if fence has became signaled.
    /// Panics if not submitted.
    /// Returns submission epoch on success.
    pub fn check_signaled(
        &mut self,
        device: &Device<B>,
    ) -> Result<Option<FenceEpoch>, rendy_core::hal::device::DeviceLost> {
        self.assert_device_owner(device);

        match self.state {
            FenceState::Submitted(epoch) => {
                if unsafe { device.get_fence_status(&self.raw) }? {
                    self.state = FenceState::Signaled;
                    Ok(Some(epoch))
                } else {
                    Ok(None)
                }
            }
            _ => panic!("Must be submitted"),
        }
    }

    /// Get raw fence reference.
    /// Use `mark_*` functions to reflect stage changes.
    pub fn raw(&self) -> &B::Fence {
        &self.raw
    }

    /// Get submission epoch.
    /// Panics if not submitted.
    pub fn epoch(&self) -> FenceEpoch {
        match self.state {
            FenceState::Submitted(epoch) => epoch,
            _ => panic!("Must be submitted"),
        }
    }

    /// Unwrap raw fence value.
    /// Panics if submitted.
    pub fn into_inner(self) -> B::Fence {
        match self.state {
            FenceState::Signaled | FenceState::Unsignaled => self.raw,
            _ => panic!("Submitted fence must be waited upon before destroying"),
        }
    }
}

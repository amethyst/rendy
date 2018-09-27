//! Queue module docs.

use std::borrow::Borrow;

use buffer::Submit;
use device::CommandQueue;

/// Submission is a list of command buffers in executable state (in form of `Submit`s)
/// together with semaphores to wait and semaphores signal.
#[derive(Clone, Copy, Debug)]
pub struct Submission<W, L, S> {
    wait: W,
    buffers: L,
    signal: S,
}

/// Command queue with known capabilities.
#[derive(Debug)]
pub struct Queue<Q, C> {
    inner: Q,
    capability: C,
}

impl<Q, C> Queue<Q, C> {
    /// Submit command buffers to the queue.
    ///
    /// # Panics
    ///
    /// This function panics if a command buffer in submission was created from
    /// command pool associated with another queue family.
    ///
    /// # Safety
    ///
    /// User must ensure that for each semaphore to wait there must be queued signal of that semaphore.
    /// [See Vulkan spec for details](https://www.khronos.org/registry/vulkan/specs/1.1-extensions/html/vkspec.html#commandbuffers-submission-progress)
    ///
    pub unsafe fn submit<I, WI, BI, SI, W, B, S, F>(&mut self, submission: I, fence: Option<F>)
    where
        Q: CommandQueue,
        I: IntoIterator<Item = Submission<WI, BI, SI>>,
        WI: IntoIterator<Item = W>,
        BI: IntoIterator<Item = Submit<B>>,
        SI: IntoIterator<Item = S>,
        W: Borrow<Q::Semaphore>,
        B: Borrow<Q::Submit>,
        S: Borrow<Q::Semaphore>,
        F: Borrow<Q::Fence>,
    {
        unimplemented!()
    }
}

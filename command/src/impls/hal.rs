use std::borrow::Borrow;
use std::marker::PhantomData;

use hal;

use device::{CommandBuffer, CommandQueue, Device};
use fence;

impl<D, B> Device for (D, PhantomData<B>)
where
    B: hal::Backend,
    D: Borrow<B::Device>,
{
    type Semaphore = B::Semaphore;
    type Fence = B::Fence;
    type Submit = B::CommandBuffer;
    type CommandPool = B::CommandPool;
    type CommandBuffer = (B::CommandBuffer, PhantomData<B>);
    type CommandQueue = (B::CommandQueue, PhantomData<B>);

    unsafe fn create_fence(&self, info: fence::FenceCreateInfo) -> Self::Fence {
        hal::Device::create_fence(self.0.borrow(), info.flags.subset(fence::FenceCreateFlags::CREATE_SIGNALED))
    }
}

impl<C, B> CommandBuffer for (C, PhantomData<B>)
where
    B: hal::Backend,
    C: Borrow<B::CommandBuffer>,
{
    type Submit = B::CommandBuffer;

    unsafe fn submit(&self) -> Self::Submit {
        self.0.borrow().clone()
    }
}

impl<C, B> CommandQueue for (C, PhantomData<B>)
where
    B: hal::Backend,
    C: Borrow<B::CommandQueue>,
{
    type Semaphore = B::Semaphore;
    type Fence = B::Fence;
    type Submit = B::CommandBuffer;
}

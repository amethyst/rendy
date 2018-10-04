use ash::{
    self,
    version::{DeviceV1_0, FunctionPointers},
    vk,
};

use device::{CommandBuffer, CommandQueue, Device};

impl<V> Device for ash::Device<V>
where
    V: FunctionPointers,
    ash::Device<V>: DeviceV1_0,
{
    type Semaphore = vk::Semaphore;
    type Fence = vk::Fence;
    type Submit = vk::CommandBuffer;
    type CommandPool = (vk::DeviceFnV1_0, vk::CommandBuffer);
    type CommandBuffer = (vk::DeviceFnV1_0, vk::CommandBuffer);
    type CommandQueue = vk::Queue;
}

impl CommandBuffer for (vk::DeviceFnV1_0, vk::CommandBuffer) {
    type Submit = vk::CommandBuffer;

    unsafe fn submit(&self) -> Self::Submit {
        self.1
    }
}

impl CommandQueue for vk::Queue {
    type Semaphore = vk::Semaphore;
    type Fence = vk::Fence;
    type Submit = vk::CommandBuffer;
}

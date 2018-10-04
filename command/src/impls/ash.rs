use ash::{
    self,
    version::{DeviceV1_0, FunctionPointers},
    vk,
};

use device::{CommandBuffer, CommandQueue, Device};
use fence;

impl From<fence::FenceCreateFlags> for vk::FenceCreateFlags {
    fn from(flags: fence::FenceCreateFlags) -> Self {
        Self::from_flags(flags.bits()).expect("Unsupported flags")
    }
}

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

    unsafe fn create_fence(&self, info: fence::FenceCreateInfo) -> Self::Fence {
        use std::ptr::null;

        DeviceV1_0::create_fence(self, &vk::FenceCreateInfo {
            s_type: vk::StructureType::FenceCreateInfo,
            p_next: null(),
            flags: info.flags.into(),
        }, None).unwrap()
    }
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

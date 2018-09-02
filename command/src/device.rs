
use resource;

pub trait Device: resource::Device {
    type CommandBuffer;
    type CommandPool;
    type CommandQueue;
}


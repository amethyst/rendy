# Vulkan Buffers

This is a brief overview of Vulkan Buffers and how they are used within Rendy.

*Please note this is not intended to be a tutorial on Vulkan. It covers only enough to understand code written in Rendy*

## Buffers Overview

A buffer is just an area of memory in which to store data. The goal with buffers in Vulkan, in general, is to make data available to the GPU. An important concept to remember is that when working with a graphics device, it will have its own memory (RAM), that is separate from the RAM your system uses.

### Memory Visibility and Coherency

Can the CPU see the contents of the GPU's memory? Can the GPU access the system RAM? Good questions! The answer is, it depends. A graphics device will have capabilities, and it is up to you, intrepid programmer, to figure out what those are and code accordingly.

A piece of memory is referred to as a `heap`. A `heap` has a size in bytes, and a location. The location can be local to the graphics device, or not local. In most systems, there will be two `heaps`: one on the Vulkan device, and one on the system used by the CPU. When you want to use some memory from a `heap`, you allocate it as a certain type of memory. Each type has different properties; a summary is below.

*VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT*: If this is set, memory allocated with this type is the most efficient for device access. This bit _will only be set if the heap has the `VK_MEMORY_HEAP_DEVICE_LOCAL_BIT` set as well_.

*VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT*: If this is set, the memory is visible to the `host`; the system with the CPU. 

*VK_MEMORY_PROPERTY_HOST_COHERENT_BIT*: If this is set, host writes or device writes become visible to each other without explicit flush commands

*VK_MEMORY_PROPERTY_HOST_CACHED_BIT*: If this is set, the memory is cached on the `host`. _This memory may not be host coherent, but is usually faster than uncached memory_. This means that cached memory may not reflect the latest writes to it. 

*VK_MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT*: If this is set, the memory is visible to the device only. Despite its name, whatever memory is behind this allocation may or may not be lazily allocated.

### Why So Many?

Vulkan is meant to be cross-platform, and used not just for graphics, but for compute as well. We often think of a graphics engine being meant for a desktop-style system with one graphics card, there are many other configurations, such as:

1. A server with multiple graphics cards meant for high parallelizable computation (machine learning, neural net training, etc)
2. CPUs with the GPUs embedded on the chip, also known as SOCs (System on a Chip)
3. Devices with Unified Memory Access (UMA) where memory is non-local to _both_ the host and the device (weird, huh?)
4. A GPU with no memory
5. A desktop system with 2 or more graphics cards

### Allocation and Deallocation

When working with Vulkan, the programmer (that's you!), is responsible for requesting and freeing memory. This means it is possible to leak memory, even when using a safe language such as Rust. 

## Rendy

Memory management is an area where Rendy provides an abstraction layer. It has a memory manager called `Heaps`. You can find this module at `rendy/memory`. You will most often work with memory via the `Factory`. Using this module, you can request and free memory of various types and Rendy will handle the details.

### Buffer Example

Let's look at an example:

```rust
let buffer = factory
    .create_buffer(
        BufferInfo {
            size: buffer_frame_size(align) * frames as u64,
            usage: gfx_hal::buffer::Usage::UNIFORM
                | gfx_hal::buffer::Usage::INDIRECT
                | gfx_hal::buffer::Usage::VERTEX,
        },
        Dynamic,
    )
    .unwrap();
```

This creates a buffer with a size large enough to hold the data for multiple frames in our game, can store indirect draw commands, and can store vertex data. It also has the Rendy type of `Dynamic`, which means it can be used to send data back and forth between the CPU and GPU (bidirectional) rather than just one direction (unidirectional).

## Useful Links

[Usage Types in gfx_hal](https://docs.rs/gfx-hal/0.1.0/gfx_hal/buffer/struct.Usage.html)
[Vulkan Vertex Buffers](https://vulkan-tutorial.com/Vertex_buffers)
[Vulkan Indirect Draw](https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdDrawIndirect.html)



<p align="left">
  <img src="docs/logo.png" width="128px"/>
</p>

[![Build Status][s1]][tc]
[![Crates.io][s2]][ci]
[![docs page][docs-badge]][docs]
[![MIT/Apache][s3]][li]
![Lines of Code][s4]

[s1]: https://travis-ci.org/amethyst/rendy.svg?branch=master
[s2]: https://img.shields.io/crates/v/rendy.svg
[docs-badge]: https://img.shields.io/badge/docs-website-blue.svg
[docs]: https://docs.rs/rendy
[s3]: https://img.shields.io/badge/license-MIT%2FApache-blue.svg
[s4]: https://tokei.rs/b1/github/amethyst/rendy?category=code
[tc]: https://travis-ci.org/amethyst/rendy
[ci]: https://crates.io/crates/rendy/
[li]: COPYING

A rendering engine based on [`gfx-hal`], which mimics the [`Vulkan`] API.

## Building

This library requires standard build tools for the target platforms, except in the case of windows - the spirv-compiler feature requires Ninja to be installed for compilation. https://ninja-build.org

## Features

Most importantly `rendy` features safer API by checking important states and invariants.
It checks invariants statically using marker types and dynamically with stored values.

### Capability

Queue family capability defines what operation queues of the family supports.
`rendy` provides simple mechanisms to prevent recording unsupported commands.
A queue's capability level can be stored statically by marking the `Family` type with one of capability types: `Transfer`, `Graphics`, `Compute` or `General` (`Graphics` and `Compute` combined).
Alternatively the `Capability` type can be used instead of the marker type, this way actual capability level can be checked dynamically.

### Command buffer

`rendy` provides a handy wrapper named `CommandBuffer`. In contrast to its raw counterpart this wrapper
encodes crucial information about its state directly into the type.
This means users can't accidentally:
* record commands unsupported by queue family it belongs to.
* record commands when a command buffer is not in recording state.
* record render pass commands outside renderpass.
* forget to finish recording a buffer before submitting.
* resubmit a command buffer which was created for one time use.
* record execution of a primary buffer into a secondary buffer.
* etc

### Memory manager

`rendy`'s memory manager is called `Heaps`.
`Heaps` provides convenient methods to sub-allocate device-visible memory based on usage and visibility requirements. It also handles mapping for specific usage types.
**It is possible for [`gfx-hal`] to adopt [VMA]. In which case `rendy` will use it**

### Rendergraph

`rendy`'s rendergraph allows writing rendering code in simple modular style.
Note that this is not a scene graph offered by high-level graphics libraries, where nodes in
the graph correspond to complex objects in the world.  Instead it is a graph of render passes
with different properties.
This makes it much easier to compose a complex frame from simple parts.
A user defines nodes which declare which buffers and images it reads and writes and
the rendergraph takes responsibility for transient resource allocation and execution synchronization.
The user is responsible only for intra-node synchronization.

`DynNode` implementation - `RenderPassNode` can be constructed from `RenderGroup`s collected into subpasses.
`RenderPassNode` will do all work for render pass creating and inter-subpass synchronization.
There will be more `Node`, `DynNode` and `RenderGroup` implementations to further simplify usage and reduce boilerplate code required for various use cases.

### Cirques

This hybrid of circus and queue simplifies synchronizing host access to resources.
`Cirque` allocates copies of the resource from resource specific allocator
(e.g. `CommandPool` for `CommandBuffer`s, `Factory` for `Buffer`s)
and gives access to the unused copy.

### CPU-GPU data flow

Rendy can help to send data between device and host.
The `Factory` type can upload data to the device local memory choosing most appropriate technique for that.
* Memory mapping will be used if device local memory happens to be cpu-visible.
* Relatively small data will be uploaded directly to buffers.
* Staging buffer will be used for bigger uploads or any image uploads.
`Factory` will automatically insert synchronization commands according to user request.

### GPU-CPU data flow - **Not yet implemented**

### Data driven pipelines - **WIP**

We think it is possible in many common cases to feed GPU with data in semi-automatic mode.
`rendy::graph::node::render::RenderGroup` implementation will use `spirv-reflect` (or similar crate) to read layout information directly from shaders
and use it to automatically populate descriptors and set index/vertex buffers based on registered data encoders and provided scene instance.
Current *WIP* implementation will use `specs::World` as scene to render.

### Declarative pipelines - ***Planned***

Pipelines and descriptor sets has declarative nature and it is much easier to define them declaratively.
`rendy` provides a trait for this called `DescriptorSet`.
Deriving it will automatically generate code necessary for set creation, writing and binding.
Deriving the `GraphicsPipeline` trait will generate code for graphics pipeline creation and usage.
A similar `ComputePipeline` trait exists for compute pipelines.

#### Example

```rust
#[derive(DescriptorSet)]
struct Example {
    /// This field will be associated with binding 1 of type `VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER`.
    /// Actual `Buffer` will be allocated and kept updated by `Set<Example>`.
    #[descriptor(UniformBlock)]
    transform: mat4,

    /// This field will be associated with binding 2 of type `VK_DESCRIPTOR_TYPE_SAMPLED_IMAGE`.
    /// `ImageView` will be fetched from `Texture` which implements `Borrow<ImageView>`.
    #[descriptor(SampledImage)]
    texture: Texture,

    /// Raw `gfx-hal` objects can be used as well.
    /// But this field will make binding `Set<Example>` to a command buffer an unsafe operation
    /// since it is the user's job to ensure that this raw image view is valid during command buffer execution.
    #[descriptor(unsafe, SampledImage)]
    foo: RawImageView,
}
```

### Modularity

Most of the features provided by rendy can be used independently from each other
This helps to keep API clean and hopefully sound.
The top-level umbrella crate `rendy` has features for each subcrate so that they could be
enabled separately (enabling a subcrate will also enable its dependencies).

## Who is using it?

The first project to use `rendy` is expected to be the [`Amethyst`] project. Kindly open a PR or issue if you're aware of other projects using `rendy`.

## License

Licensed under either of

* Apache License, Version 2.0, ([license/APACHE](license/APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([license/MIT](license/MIT) or http://opensource.org/licenses/MIT)

at your option.

[`gfx-hal`]: https://github.com/gfx-rs/gfx
[`gfx-memory`]: https://github.com/gfx-rs/gfx-memory
[`gfx-render`]: https://github.com/gfx-rs/gfx-render
[`gfx-mesh`]: https://github.com/omni-viral/gfx-mesh
[`gfx-texture`]: https://github.com/omni-viral/gfx-texture
[`xfg`]: https://github.com/omni-viral/xfg-rs
[`Vulkan`]: https://www.khronos.org/vulkan/
[`Vulkan`-portability]: https://www.khronos.org/vulkan/portability-initiative
[`Amethyst`]: https://github.com/amethyst/amethyst
[VMA]: https://gpuopen.com/gaming-product/vulkan-memory-allocator/

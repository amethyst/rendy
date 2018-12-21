
# Rendy

[![Build Status][s1]][tc]
[![Crates.io][s2]][ci]
[![docs page][docs-badge]][docs]
[![MIT/Apache][s3]][li]
![Lines of Code][s4]

[s1]: https://travis-ci.org/omni-viral/rendy.svg?branch=master
[s2]: https://img.shields.io/crates/v/rendy.svg
[docs-badge]: https://img.shields.io/badge/docs-website-blue.svg
[docs]: https://omni-viral.github.io/rendy-docs/rendy/index.html
[s3]: https://img.shields.io/badge/license-MIT%2FApache-blue.svg
[s4]: https://tokei.rs/b1/github/omni-viral/rendy?category=code
[tc]: https://travis-ci.org/omni-viral/rendy
[ci]: https://crates.io/crates/rendy/
[li]: COPYING

Yet another [`Vulkan`] based rendering engine.
Actually it is based on [`gfx-hal`] that mimics [`Vulkan`] API.

## Features

Most importantly `rendy` features safer API by checking important states and invariants.
It checks invariants statically using marker types and dynamically with stored values.

### Capability

Queue family capability defines what operation queues of the family supports.
`rendy` provides simple mechanism to prevent recording unsupported commands.
Capability level can be stored statically by marking `Family` type with one of capability types: `Transfer`, `Graphics`, `Compute` or `General` (`Graphics` and `Compute` combined).
Alternatively `Capability` type can be used instead of marker type, this way actual capability level can be checked dynamically.

### Command buffer

`rendy` provides handy wrapper named `CommandBuffer`. In contrast to raw counterpart this wrapper
encodes crutial information about its state directly into type level.
This means user can't accidentially:
* record command unsupported by queue family it belongs to.
* record command when command buffer is not in recording state.
* record render pass command outside renderpass.
* forget to finish recording buffer before submitting.
* resubmit command buffer which was created for one time use.
* record execution of primary buffer into secondary buffer.
* etc

### Memory manager

`rendy`'s memory manager is called `Heaps`.
`Heaps` provides convenient methods to sub-allocate device-visible memory based on usage and visibility requirements. It also handles mapping for specific usage types.
**It is possible for [`gfx-hal`] to adopt VMA. In which case `rendy` will use it**

### Framegraph

`rendy`'s framegraph allow writing rendering code in simple modular style.
Making it much easier to composite complex frame from simple parts.
User defines nodes which declare buffers and images it reads and writes.
Framegraph takes responsibility for resource allocation and execution synchronization.
User is responsible only for intra-node synchronization.

### Cirques

This hybrid of circus and queue simplifies synchronizing host access to resources.
`Cirque` allocates copies of the resource from resource spicific allocator
(e.g. `CommandPool` for `CommandBuffer`s, `Factory` for `Buffer`s)
and gives access to the unused copy.

### CPU-GPU data flow - ***Not yet implemented***

Rendy can help to send data between device and host.
`Factory` can upload data to the device local memory choosing most appropriate technique for that.
* Memory mapping will be used if device local memory happens to be cpu-visible.
* Relatively small data will be uploaded directly to buffers.
* Staging buffer will be used for bigger uploads or any image uploads.
`Factoy` will automatically insert synchronization commands according to user request.

### Layouts - ***Not yet implemented. More experiments required***

Pipelines and descriptor sets has declarative nature and it is much easier to define them declaratively.
`rendy` provides `DescriptorSet` trait.
Deriving it will automatically generate code necessary for set creation, writing and binding.
Deriving `GraphicsPipeline` trait will generate code for graphics pipeline creation and usage.
Similar `ComputePipeline` trait exists for compute pipelines.

#### Example

```rust
#[derive(DescritorSet)]
struct Example {
    /// This field will be associated with binding 1 of type `VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER`.
    /// Actual `Buffer` will be allocated and kept updated by `Set<Example>`.
    #[descriptor(UniformBlock)]
    transform: mat4,

    /// This field will be associated with binding 2 of type `VK_DESCRIPTOR_TYPE_SAMPLED_IMAGE`.
    /// `ImageView` will be fetched from `Texture` which implements `Borrow<ImageView>`.
    #[descriptor(SampledImage)]
    texture: Texture,

    /// Raw Vulkan objects can be used as well.
    /// But this field will make binding of `Set<Example>` to command buffer to require unsafe operation
    /// since it is user job to ensure that this raw image view is valid during command buffer execution.
    #[descriptor(unsafe, SampledImage)]
    foo: RawImageView,
}
```

### Shader reflection - ***Not yet implemented***

`rendy` will use `spirv-relfect` or similiar crate to read layout information directly from shaders
and use it to automatically populate descriptors and set index/vertex buffers based on registered data sources.

### Modularity

Most of the features provided by rendy can be used independently from others.
This helps to keep API clean and hopefuly sound.
Top-level umbrela crate `rendy` has feature for each subcrates so that they could be enabled separately (subcrate will also enable its depenencies).

## Why another renderer

There is no fully-featured modern renderers written in Rust. So this project aims to be the one of the first.
`rendy` will be used by [`Amethyst`] project.

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

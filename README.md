
# Rendy

Yet another [`Vulkan`] based rendering engine.

## Features

`rendy` features safer API by checking important states and invariants.
It can check invariants statically using marker types and dynamically with stored values.

### Capability

Queue family capability defines what operation queues of the family supports.
`rendy` provides simple mechanism to prevent recording unsupported commands.
Capability level can be stored statically by marking `Family` type with one of capability types: `Transfer`, `Graphics`, `Compute` or `General` (`Graphics` and `Compute` combined).
Alternatively `Capability` type can be used instead of marker type, this way actual capability level can be checked dynamically.

### Memory manager

`rendy`'s memory manager is called `Heaps`.
`Heaps` provides convenient methods to sub-allocate device-visible memory based on usage and visibility requirements. It also handles mapping for specific usage types.

### Objects lifetime - ***Not yet implemented***

`rendy` provide tools to track resource usage in order to automatically destroy them after last use.
Once resource is referenced in recorded command it won't be destroyed immediately after handle dropped but after command is complete. For performance reasons tracking mechanism can choose later destruction time than necessary to save few ticks.

### CPU-GPU data flow - ***Not yet implemented***

Rendy can help to send data between device and host.
`Factory` can upload data to the device local memory choosing most appropriate technique for that.
* Memory mapping will be used if device local memory happens to be cpu-visible.
* Relatively small data will be uploaded directly to buffers.
* Staging buffer will be used for bigger uploads or any image uploads.
`Factoy` will automatically insert synchronization commands according to user request.

### Layouts - ***Not yet implemented***

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

### Framegraph - ***Not yet implemented***

`rendy`'s framegraph allow writing rendering code in simple modular style.
Making it much easier to composite complex frame from simple parts.
User defines nodes which declare buffers and images it reads and writes.
Framegraph takes responsibility for resource allocation and execution synchronization.
User is responsible only for intra-node synchronization.

### Modularity

Most of the features provided by rendy can be used independently from others.
Most notably `rendy-memory` crate doesn't depend on any other rendy crate.

## Why another render

There is no fully-featured modern renderers written in Rust. So this project aims to be the one of the first.
Once `rendy` will be able to render simple scenes it probably will be integrated as rendering engine into [`amethyst`].

### How it started

`rendy` is my rethinking of libraries I wrote for [`gfx-hal`] project:
* [`gfx-memory`]
* [`gfx-render`]
* [`gfx-mesh`]
* [`gfx-texture`]
* [`xfg`]

Those libraries can be seen as draft for `rendy`.

## License

Licensed under either of

* Apache License, Version 2.0, ([license/APACHE](license/APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([license/MIT](license/MIT) or http://opensource.org/licenses/MIT)

at your option.

[`ash`]: https://github.com/MaikKlein/ash
[`gfx-hal`]: https://github.com/gfx-rs/gfx
[`gfx-memory`]: https://github.com/gfx-rs/gfx-memory
[`gfx-render`]: https://github.com/gfx-rs/gfx-render
[`gfx-mesh`]: https://github.com/omni-viral/gfx-mesh
[`gfx-texture`]: https://github.com/omni-viral/gfx-texture
[`xfg`]: https://github.com/omni-viral/xfg-rs
[`Vulkan`]: https://www.khronos.org/vulkan/
[`Vulkan`-portability]: https://www.khronos.org/vulkan/portability-initiative
[`amethyst`]: https://github.com/amethyst/amethyst

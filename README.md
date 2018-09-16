
# Rendy

Yet another [`Vulkan`] based rendering engine.

## Features

Rendy features safer API by checking important states and invariants.
It can check invariants statically using marker types and dynamically with stored values.

### Capability

Queue family capability defines what operation queues of the family supports.
Capability level can be stored statically by marking `Family` type with one of capability types: `Transfer`, `Graphics`, `Compute` or `General`.
Alternatively `Capability` type can be used instead of marker type so that `Family` instance can be checked for capability level dynamically.

### Memory manager

`Heaps` provides convenient methods to sub-allocate device-visible memory based on usage and visibility requirements.

### Objects lifetime - ***Not yet implemented***

Rendy provide tools to track resource usage in order to automatically destroy them after last use.

### CPU-GPU data flow - ***Not yet implemented***

`Factory` can upload data to the device local memory choosing most appropriate technique for that.
* Memory mapping will be used if device local memory happens to be cpu-visible.
* Relatively small data will be uploaded directly to buffers.
* Staging buffer will be used for bigger uploads or any image uploads.
`Factoy` will automatically insert synchronization commands according to user request.

### Descriptors - ***Not yet implemented***

Descriptor sets can be automated by deriving `DescriptorSet` implementation for structures with descriptors or uniform block as fields.

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

Rendy's framegraph allow writing rendering code in simple modular style.
Making it much easier to composite complex frame from simple parts.
User defines nodes which declare buffers and images it reads and writes.
Framegraph takes responsibility for resource allocation and execution synchronization.
User is responsible only for intra-node synchronization.

## Why another render?

There is no fully-featured modern renderers written in Rust. So this project aims to be the one of the first.
Once `rendy` will be able to render simple scenes it probably will be integrated as rendering engine into [`amethyst`].

### How it started.

`rendy` is my rethinking of libraries I wrote for [`gfx-hal`] project:
* [`gfx-memory`]
* [`gfx-render`]
* [`gfx-mesh`]
* [`gfx-texture`]
* [`xfg`]

Those libraries can be seen as draft for `rendy`.

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

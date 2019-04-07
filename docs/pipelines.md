# Graphics Pipelines

This document covers the key points of a Graphics Pipeline as used by rendy and gfx-hal; they are essentially the same as a Vulkan graphics pipeline.

*Please note this is not intended to be a tutorial on Vulkan. It covers only enough to understand code written in Rendy*

## Pipeline Overview

When you want to draw a 3D shape on someone's monitor, say a cube, you are really just drawing some number of triangles.

## A Cube

For a basic cube, each side is probably made up of 2 triangles, for a total of 12 triangles. When you position those triangles properly in 3D space, they form a cube that has 6 sides and 8 vertices (a place where edges meet).

All this is represented in some coordinate space in your computer's memory as a series of (x, y, z) coordinates in that space. Note that all of this has nothing to do with actually _drawing_ that cube. You could have the coordinate space and things in it moving around and never draw it to screen. Some simulations work this way.

## Changing the Cube

To draw your cube, your series of coordinates will go through a series of transformations. This series of transformations is known as a Pipeline in Vulkan, and it means the same thing in Rendy. Some of these will add color to your cube, apply textures, and so on.

## Pipeline Stages

At a very high level, a Pipeline consists of:

1. Input Assembler
2. Shaders
3. Rasterization
4. Fragment Shaders

### Input Assembler

Every Pipeline has an `Input Assembler`. This stage takes the vertices that you want to render (that's the series of (x, y, z) coordinates I mentioned above) and assembles them into one of the Vulkan Primitives. These Primitives are things like Triangles, Lines, or Points.

By default, Rendy uses `TriangleList`, where it assumes the vertex data will be a list containing coordinates. It will take the first three, and make a triangle out of it. Then the next three, and so on.

### Shader Stages

There are different types of shaders, and different stages for them. They should occur in the order below.

#### Vertex Shader

In this stage, vertices _can_ be transformed. Their position can be changed, as can their attributes, which is data associated with a specific vertex.

#### Tesselation Shader (Optional)

A full description of tesselation is beyond the scope of this document. See [this](https://vulkan.lunarg.com/doc/view/1.0.33.0/linux/vkspec.chunked/ch21.html) page for more info.

#### Geometry Shader (Optional)

From [the API docs](https://vulkan.lunarg.com/doc/view/1.0.33.0/linux/vkspec.chunked/ch22.html):

The geometry shader operates on a group of vertices and their associated data assembled from a single input primitive, and emits zero or more output primitives and the group of vertices and their associated data required for each output primitive. 

Geometry shading is enabled when a geometry shader is included in the pipeline.

### Rasterization

Once all the shader stages have been completed and the primitives are 
assembled, they are _rasterized_. From [the API docs](https://vulkan.lunarg.com/doc/view/1.0.33.0/linux/vkspec.chunked/ch24.html):

> Rasterization is the process by which a primitive is converted to a 
> two-dimensional image. Each point of this image contains associated data 
> such as depth, color, or other attributes.
>
> Rasterizing a primitive begins by determining which squares of an integer
> grid in framebuffer coordinates are occupied by the primitive, and 
> assigning one or more depth values to each such square.

Now we're making progress! Only 28,233 more stages to go!

### Fragment Shader

Yes, this is a shader stage that isn't with the rest of the shader stages. Shocking.

From [the API docs](http://vulkan-spec-chunked.ahcox.com/ch24.html):
> As something is rasterized, the rasterizer produces a series of framebuffer addresses and values using a two-dimensional 
> description of a point, line segment, or triangle. A grid square, including its (x, y) framebuffer coordinates, z coordinate > (depth), and associated data added by fragment shaders is called a _fragment_.

You probably guessed that a fragment shader alters these fragments.

Once they are done, a final series of `framebuffer operations`, such as color blending, sends the final color to the framebuffer.

### Framebuffer Operations

After the fragment shaders are done, other things read the updated data to come up with the final color, transparency, and all that which we will skip. You don't have to do any of that. With a few exceptions (such as altering the blend operation in the pipeline), fragment shaders are the only way to affect those.

## Back to Rendy

In Rendy, we describe one of these pipelines using two `Trait`s: a Pipeline Descriptor, and a Pipeline. 

### SimpleGraphicsPipelineDesc

This `Trait` is defined in `graph/src/node/render/group/simple.rs`, and you will use it to set up the resources your pipeline will use. The `Desc` is short for `Description`.

With this `Trait`, you describe your graphics pipeline to Vulkan/Metal/whatever, such as the vertices you want to draw, how to blend colors, how much memory will be needed for buffers, and all that good stuff.

A Rust `Associated Type` specifies the `SimpleGraphicsPipeline` to use:

```rust
/// Descriptor for simple graphics pipeline implementation.
pub trait SimpleGraphicsPipelineDesc<B: Backend, T: ?Sized>: std::fmt::Debug {
    /// Simple graphics pipeline implementation
    type Pipeline: SimpleGraphicsPipeline<B, T>;
    /// ...
```

### SimpleGraphicsPipeline

This `Trait` is also defined in `graph/src/node/render/group/simple.rs`. A Rust `Associated Type` specifies the `SimpleGraphicsPipelineDesc` to use:

```rust
pub trait SimpleGraphicsPipeline<B: Backend, T: ?Sized>:
    std::fmt::Debug + Sized + Send + Sync + 'static
{
    /// This pipeline descriptor.
    type Desc: SimpleGraphicsPipelineDesc<B, T, Pipeline = Self>;
    /// ...
}
```

In this, you tell the Pipeline _how_ to use all the resources you described in the `SimpleGraphicsPipelineDesc`. 

## References

The above is a _drastically_ simplified description of what is happening, but the goal is to help put these `Trait`s into proper context so you can more easily understand the code.

Once you are ready, you can check out this full diagram of a Pipeline: https://vulkan.lunarg.com/doc/view/1.0.33.0/linux/vkspec.chunked/ch09.html

And then, once you've recovered, the entire chapter on Pipelines in the Vulkan spec is here: https://vulkan.lunarg.com/doc/view/1.0.33.0/linux/vkspec.chunked/ch09.html.

A similar document to this by @termhn (one of the primary Rendy contributors) can be found [here](https://github.com/termhn/gfx-hal-tutorial/blob/master/articles/zero-to-voxel-render-part1.md#the-graphics-pipeline).

Good Luck....

_...the final piece of the note appears to be stained with a mixture of tears and something darker, possibly blood..._

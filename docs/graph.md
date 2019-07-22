# Summary

This document gives an overview of the Framegraph system in Rendy. 

## What is a Framegraph

Let's start at the end for this explanation. What you see on your monitor is the end result of a complicated series of transformations to data as it makes its way from your running application to your monitor.

This series of transformations is often referred to as a _rendering pipeline_; note that this is separate from another meaning of _rendering pipeline_ which is used to encompass the GPU hardware pipeline (all shader stages and everything in between). 

These _rendering pipelines_ can be very simple, or they can be very complex. When they are very complex, it can be useful to add a layer of abstraction to make them easier to conceptualize.

One way of doing this is with a _framegraph_ (an alternate name sometimes used is _render graph_). Consider a simple rendering pipeline that looks like this:

```
┌────────────┐    ┌────────────┐                                                                            
│  Previous  │    │  Compute   │                                                                            
│   Frame    │───▶│  Average   │────────────────────────────────┐                                           
│            │    │ Brightness │                                │                                           
└────────────┘    └────────────┘                                │                                           
                                                                ▼                                           
┌────────────┐    ┌────────────┐     ┌────────────┐      ┌────────────┐     ┌────────────┐    ┌────────────┐
│   Depth    │    │            │     │            │      │            │     │            │    │            │
│  Pre-Pass  │───▶│ PBR Shader │────▶│   Tonemap  │ ────▶│   Dither   │────▶│ Lens Flare │───▶│   Output   │
│            │    │            │     │            │      │            │     │            │    │            │
└────────────┘    └────────────┘     └────────────┘      └────────────┘     └────────────┘    └────────────┘
                         ▲                                                                                  
┌────────────┐           │                                                                                  
│ Shadow Map │           │                                                                                  
│    Pass    │───────────┘                                                                                  
│            │                                                                                              
└────────────┘                                                                                              
```

This could be applied to one model in your frame. Expand this out to much more complex pipelines, across complex scenes with many models, and you can see how it can get complicated.

### Synchronization

Aside from providing a higher-level view of your graphics pipeline, a render graph can also handle _synchronization_. Imagine you have 3 shaders, A, B, and C. C requires both A and B to have processed a pixel before it can process it.

A graph can abstract the synchronization primitives needed to ensure C doesn't get the data before both A and B are done.

### Ordering

Similar to synchronization, a render graph can provide ordering. If you have shaders A -> B -> C, you may not want them to ever run in the order B -> C -> A.

### Managed Primitives

Beneath the graph, you have all the primitives you would normally find:

* Buffers
* Semaphores
* Images
* Swap Chains
* etc...

The render graph just wraps them up neatly into a

## Render Group

A render group is a collection of pipelines (in the Vulkan sense of the word) that are processed in a `Subpass`.

## Subpass

A `Subpass` is a child of a `Pass`. Currently, a `Pass` can have only one `Subpass`. More may be allowed in the future, but one is always required. They are an opportunity for performance optimization when the output of one render group is used directly as the input to another one; not sampled, but literally passing the fragment.

An additional benefit to having and using `Subpasses` now is that the API will not require breaking changes to support more than one `Subpass` per `Pass`. 

## RenderPass

In Rendy, a `RenderPass` is a combination of a `Pass` and a `Node`, described below.

## Pass

A `Pass`, currently can have only one `Subpass`, and a `Pass` will usually belong to a `Node`.

## Node

A `Node` contains 0 or more things that provide a set of self-contained (i.e. internally synchronized) submissions to a queue each frame. This is _usually_ a `RenderPass`, but does not have to be. The intended usage of a `Node` is that it should only consist of one thing, though it can technically contain more.

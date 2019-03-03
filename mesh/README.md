
# `rendy-mesh`

Helper crate for `gfx-hal` to create and use meshes with vertex semantics.

# Vertex semantics

Vertex formats usually has semantics attached to field names.
This crate provides traits and types to have semantics explicitly defined on the type level.

`Position`, `Normal`, `TexCoord` etc. are attributes that have unambiguous semantics.
Users can define their own attribute types by implementing the `Attribute` trait.

While the attribute type on its own is a trivial vertex format (with single attribute), complex vertex formats are created by composing attribute types.

The `WithAttribute` trait allows to get formatting info for individual attributes defined in a vertex format.
The `Query` trait allows to get formatting info for several attributes at once.

`VertexFormat` queried from vertex formats can be used to build graphics pipelines and bind required vertex buffers from mesh to command buffer.

To define a custom vertex format type, the `AsVertexFormat` trait must be implemented providing a `VertexFormat` associated constant.

`WithAttribute` can be implemented also for all attributes and `VertexFormat` associated constant in `AsVertexFormat` can be defined more clearly utilizing `WithAttribute` implementation.
`Query` is automatically implemented.

# Mesh

`Mesh` is a collection of vertex buffers and optionally an index buffer together with vertex formats of the buffers and index type. Also there is a primitive type specified which defines how vertices form primitives (lines, triangles etc).
To create instances of `Mesh` you need to use `MeshBuilder`.

1. Fill `MeshBuilder` with typed vertex data.
1. Provide the index data.
1. Set the primitive type (Triangles list by default).
1. Call `MeshBuilder::build`. It uses `Factory` from `gfx-render` to create buffers and upload data.

Here is your fresh new `Mesh`. Or an `Error` from `gfx-render`.

To bind vertex buffers to a command buffer use `Mesh::bind` with a sorted array of `VertexFormat`s (the same that was used to setup the graphics pipeline).

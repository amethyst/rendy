//!
//! Manage vertex and index buffers of single objects with ease.
//!

use std::{
    borrow::Cow,
    mem::size_of,
};

use crate::{
    factory::Factory,
    resource::buffer::{Buffer, VertexBuffer as UsageVertexBuffer, IndexBuffer as UsageIndexBuffer},
    util::{cast_cow, is_slice_sorted, is_slice_sorted_by_key},
    vertex::{AsVertex, VertexFormat},
};

/// Vertex buffer with it's format
#[derive(Debug)]
pub struct VertexBuffer<B: gfx_hal::Backend> {
    buffer: Buffer<B>,
    format: VertexFormat<'static>,
    len: u32,
}

/// Index buffer with it's type
#[derive(Debug)]
pub struct IndexBuffer<B: gfx_hal::Backend> {
    buffer: Buffer<B>,
    index_type: gfx_hal::IndexType,
    len: u32,
}

/// Abstracts over two types of indices and their absence.
#[derive(Debug)]
pub enum Indices<'a> {
    /// No indices.
    None,

    /// `u16` per index.
    U16(Cow<'a, [u16]>),

    /// `u32` per index.
    U32(Cow<'a, [u32]>),
}

impl From<Vec<u16>> for Indices<'static> {
    fn from(vec: Vec<u16>) -> Self {
        Indices::U16(vec.into())
    }
}

impl<'a> From<&'a [u16]> for Indices<'a> {
    fn from(slice: &'a [u16]) -> Self {
        Indices::U16(slice.into())
    }
}

impl<'a> From<Cow<'a, [u16]>> for Indices<'a> {
    fn from(cow: Cow<'a, [u16]>) -> Self {
        Indices::U16(cow)
    }
}

impl From<Vec<u32>> for Indices<'static> {
    fn from(vec: Vec<u32>) -> Self {
        Indices::U32(vec.into())
    }
}

impl<'a> From<&'a [u32]> for Indices<'a> {
    fn from(slice: &'a [u32]) -> Self {
        Indices::U32(slice.into())
    }
}

impl<'a> From<Cow<'a, [u32]>> for Indices<'a> {
    fn from(cow: Cow<'a, [u32]>) -> Self {
        Indices::U32(cow)
    }
}

/// Generics-free mesh builder.
/// Useful for creating mesh from non-predefined set of data.
/// Like from glTF.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MeshBuilder<'a> {
    vertices: smallvec::SmallVec<[(Cow<'a, [u8]>, VertexFormat<'static>); 16]>,
    indices: Option<(Cow<'a, [u8]>, gfx_hal::IndexType)>,
    prim: gfx_hal::Primitive,
}

impl<'a> MeshBuilder<'a> {
    /// Create empty builder.
    pub fn new() -> Self {
        MeshBuilder {
            vertices: smallvec::SmallVec::new(),
            indices: None,
            prim: gfx_hal::Primitive::TriangleList,
        }
    }

    /// Set indices buffer to the `MeshBuilder`
    pub fn with_indices<I>(mut self, indices: I) -> Self
    where
        I: Into<Indices<'a>>,
    {
        self.set_indices(indices);
        self
    }

    /// Set indices buffer to the `MeshBuilder`
    pub fn set_indices<I>(&mut self, indices: I) -> &mut Self
    where
        I: Into<Indices<'a>>,
    {
        self.indices = match indices.into() {
            Indices::None => None,
            Indices::U16(i) => Some((cast_cow(i), gfx_hal::IndexType::U16)),
            Indices::U32(i) => Some((cast_cow(i), gfx_hal::IndexType::U32)),
        };
        self
    }

    /// Add another vertices to the `MeshBuilder`
    pub fn with_vertices<V, D>(mut self, vertices: D) -> Self
    where
        V: AsVertex + 'a,
        D: Into<Cow<'a, [V]>>,
    {
        self.add_vertices(vertices);
        self
    }

    /// Add another vertices to the `MeshBuilder`
    pub fn add_vertices<V, D>(&mut self, vertices: D) -> &mut Self
    where
        V: AsVertex + 'a,
        D: Into<Cow<'a, [V]>>,
    {
        self.vertices.push((cast_cow(vertices.into()), V::VERTEX));
        self
    }

    /// Sets the primitive type of the mesh.
    ///
    /// By default, meshes are constructed as triangle lists.
    pub fn with_prim_type(mut self, prim: gfx_hal::Primitive) -> Self {
        self.prim = prim;
        self
    }

    /// Sets the primitive type of the mesh.
    ///
    /// By default, meshes are constructed as triangle lists.
    pub fn set_prim_type(&mut self, prim: gfx_hal::Primitive) -> &mut Self {
        self.prim = prim;
        self
    }

    /// Builds and returns the new mesh.
    pub fn build<B>(&self, family: gfx_hal::queue::QueueFamilyId, factory: &mut Factory<B>) -> Result<Mesh<B>, failure::Error>
    where
        B: gfx_hal::Backend,
    {
        Ok(Mesh {
            vbufs: self
                .vertices
                .iter()
                .map(|(vertices, format)| {
                    let len = vertices.len() as u32 / format.stride;
                    Ok(VertexBuffer {
                        buffer: {
                            let mut buffer = factory.create_buffer(
                                1,
                                vertices.len() as _,
                                UsageVertexBuffer,
                            )?;
                            unsafe {
                                // New buffer can't be touched by device yet.
                                factory.upload_buffer(
                                    &mut buffer,
                                    0,
                                    vertices,
                                    family,
                                    gfx_hal::buffer::Access::VERTEX_BUFFER_READ,
                                )?;
                            }
                            buffer
                        },
                        format: format.clone(),
                        len,
                    })
                }).collect::<Result<_, failure::Error>>()?,
            ibuf: match self.indices {
                None => None,
                Some((ref indices, index_type)) => {
                    let stride = match index_type {
                        gfx_hal::IndexType::U16 => size_of::<u16>(),
                        gfx_hal::IndexType::U32 => size_of::<u32>(),
                    };
                    let len = indices.len() as u32 / stride as u32;
                    Some(IndexBuffer {
                        buffer: {
                            let mut buffer = factory.create_buffer(
                                1,
                                indices.len() as _,
                                UsageIndexBuffer,
                            )?;
                            unsafe {
                                // New buffer can't be touched by device yet.
                                factory.upload_buffer(
                                    &mut buffer,
                                    0,
                                    indices,
                                    family,
                                    gfx_hal::buffer::Access::INDEX_BUFFER_READ,
                                )?;
                            }
                            buffer
                        },
                        index_type,
                        len,
                    })
                }
            },
            prim: self.prim,
        })
    }
}

/// Single mesh is a collection of buffers that provides available attributes.
/// Exactly one mesh is used per drawing call in common.
#[derive(Debug)]
pub struct Mesh<B: gfx_hal::Backend> {
    vbufs: Vec<VertexBuffer<B>>,
    ibuf: Option<IndexBuffer<B>>,
    prim: gfx_hal::Primitive,
}

impl<B> Mesh<B>
where
    B: gfx_hal::Backend,
{
    /// Build new mesh with `MeshBuilder`
    pub fn builder<'a>() -> MeshBuilder<'a> {
        MeshBuilder::new()
    }

    /// gfx_hal::Primitive type of the `Mesh`
    pub fn primitive(&self) -> gfx_hal::Primitive {
        self.prim
    }

    /// Bind buffers to specified attribute locations.
    pub fn bind<'a>(
        &'a self,
        formats: &[VertexFormat<'_>],
    ) -> Result<Bind<'a, B>, Incompatible> {
        debug_assert!(is_slice_sorted(formats));
        debug_assert!(is_slice_sorted_by_key(&self.vbufs, |vbuf| &vbuf.format));
        
        let mut vertex = smallvec::SmallVec::new();

        let mut next = 0;
        let mut vertex_count = None;
        for format in formats {
            if let Some(index) = find_compatible_buffer(&self.vbufs[next..], format) {
                // Ensure buffer is valid
                vertex.push((self.vbufs[index].buffer.raw(), 0));
                next = index + 1;
                assert!(vertex_count.is_none() || vertex_count == Some(self.vbufs[index].len));
                vertex_count = Some(self.vbufs[index].len);
            } else {
                // Can't bind
                return Err(Incompatible);
            }
        }
        Ok(match self.ibuf.as_ref() {
            Some(ibuf) => Bind::Indexed {
                buffer: ibuf.buffer.raw(),
                offset: 0,
                index_type: ibuf.index_type,
                count: ibuf.len,
                vertex,
            },
            None => Bind::Unindexed {
                count: vertex_count.unwrap_or(0),
                vertex,
            },
        })
    }
}

/// failure::Error type returned by `Mesh::bind` in case of mesh's vertex buffers are incompatible with requested vertex formats.
#[derive(Clone, Copy, Debug)]
pub struct Incompatible;

/// Result of buffers bindings.
/// It only contains `IndexBufferView` (if index buffers exists)
/// and vertex count.
/// Vertex buffers are in separate `VertexBufferSet`
#[derive(Clone, Debug)]
pub enum Bind<'a, B: gfx_hal::Backend> {
    /// Indexed binding.
    Indexed {
        /// The buffer to bind.
        buffer: &'a B::Buffer,
        /// The offset into the buffer to start at.
        offset: u64,
        /// The type of the table elements (`u16` or `u32`).
        index_type: gfx_hal::IndexType,
        /// Indices count to use in `draw_indexed` method.
        count: u32,
        /// Vertex buffers.
        vertex: smallvec::SmallVec<[(&'a B::Buffer, u64); 16]>,
    },
    /// Not indexed binding.
    Unindexed {
        /// Vertex count to use in `draw` method.
        count: u32,
        /// Vertex buffers.
        vertex: smallvec::SmallVec<[(&'a B::Buffer, u64); 16]>,
    },
}

impl<'a, B> Bind<'a, B>
where
    B: gfx_hal::Backend,
{
    /// Record drawing command for this biding.
    pub unsafe fn draw_raw(&self, encoder: &mut impl gfx_hal::command::RawCommandBuffer<B>) {
        match self {
            &Bind::Indexed {
                buffer,
                offset,
                index_type,
                count,
                ref vertex,
            } => {
                encoder.bind_vertex_buffers(0, vertex.iter().cloned());
                encoder.bind_index_buffer(gfx_hal::buffer::IndexBufferView {
                    buffer,
                    offset,
                    index_type,
                });
                encoder.draw_indexed(0..count, 0, 0..1);
            }
            &Bind::Unindexed { ref vertex, count } => {
                encoder.bind_vertex_buffers(0, vertex.iter().cloned());
                encoder.draw(0..count, 0..1);
            }
        }
    }
}

/// Helper function to find buffer with compatible format.
fn find_compatible_buffer<B>(vbufs: &[VertexBuffer<B>], format: &VertexFormat<'_>) -> Option<usize>
where
    B: gfx_hal::Backend,
{
    debug_assert!(is_slice_sorted_by_key(&*format.attributes, |a| a.offset));
    for (i, vbuf) in vbufs.iter().enumerate() {
        debug_assert!(is_slice_sorted_by_key(&*vbuf.format.attributes, |a| a.offset));
        if is_compatible(&vbuf.format, format) {
            return Some(i);
        }
    }
    None
}

/// Check is vertex format `left` is compatible with `right`.
/// `left` must have same `stride` and contain all attributes from `right`.
fn is_compatible(left: &VertexFormat<'_>, right: &VertexFormat<'_>) -> bool {
    if left.stride != right.stride {
        return false;
    }

    // Don't start searching from index 0 because attributes are sorted
    let mut skip = 0;
    right.attributes.iter().all(|r| {
        left.attributes[skip..]
            .iter()
            .position(|l| *l == *r)
            .map_or(false, |p| {
                skip += p;
                true
            })
    })
}

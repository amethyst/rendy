//!
//! Manage vertex and index buffers of single objects with ease.
//!

use std::{borrow::Cow, cmp::min, mem::size_of};

use crate::{
    command::{EncoderCommon, Graphics, QueueId, Supports},
    factory::{BufferState, Factory},
    resource::buffer::{
        Buffer, IndexBuffer as UsageIndexBuffer, VertexBuffer as UsageVertexBuffer,
    },
    util::{cast_cow, is_slice_sorted, is_slice_sorted_by_key},
    vertex::{AsVertex, VertexFormat},
};

/// Vertex buffer with it's format
#[derive(Debug)]
pub struct VertexBuffer<B: gfx_hal::Backend> {
    buffer: Buffer<B>,
    format: VertexFormat<'static>,
}

/// Index buffer with it's type
#[derive(Debug)]
pub struct IndexBuffer<B: gfx_hal::Backend> {
    buffer: Buffer<B>,
    index_type: gfx_hal::IndexType,
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
    pub fn build<B>(
        &self,
        queue: QueueId,
        factory: &mut Factory<B>,
    ) -> Result<Mesh<B>, failure::Error>
    where
        B: gfx_hal::Backend,
    {
        let mut len = u32::max_value();
        Ok(Mesh {
            vbufs: self
                .vertices
                .iter()
                .map(|(vertices, format)| {
                    len = min(len, vertices.len() as u32 / format.stride);
                    Ok(VertexBuffer {
                        buffer: {
                            let mut buffer =
                                factory.create_buffer(1, vertices.len() as _, UsageVertexBuffer)?;
                            unsafe {
                                // New buffer can't be touched by device yet.
                                factory.upload_buffer(
                                    &mut buffer,
                                    0,
                                    vertices,
                                    None,
                                    BufferState::new(queue)
                                        .with_access(gfx_hal::buffer::Access::VERTEX_BUFFER_READ),
                                )?;
                            }
                            buffer
                        },
                        format: format.clone(),
                    })
                })
                .collect::<Result<_, failure::Error>>()?,
            ibuf: match self.indices {
                None => None,
                Some((ref indices, index_type)) => {
                    let stride = match index_type {
                        gfx_hal::IndexType::U16 => size_of::<u16>(),
                        gfx_hal::IndexType::U32 => size_of::<u32>(),
                    };
                    len = indices.len() as u32 / stride as u32;
                    Some(IndexBuffer {
                        buffer: {
                            let mut buffer =
                                factory.create_buffer(1, indices.len() as _, UsageIndexBuffer)?;
                            unsafe {
                                // New buffer can't be touched by device yet.
                                factory.upload_buffer(
                                    &mut buffer,
                                    0,
                                    indices,
                                    None,
                                    BufferState::new(queue)
                                        .with_access(gfx_hal::buffer::Access::INDEX_BUFFER_READ),
                                )?;
                            }
                            buffer
                        },
                        index_type,
                    })
                }
            },
            prim: self.prim,
            len,
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
    len: u32,
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

    /// Get number of vertices in mesh.
    pub fn len(&self) -> u32 {
        self.len
    }

    /// Bind buffers to specified attribute locations.
    pub fn bind<'a, C>(
        &'a self,
        formats: &[VertexFormat<'_>],
        encoder: &mut EncoderCommon<'_, B, C>,
    ) -> Result<u32, Incompatible>
    where
        C: Supports<Graphics>,
    {
        debug_assert!(is_slice_sorted(formats));
        debug_assert!(is_slice_sorted_by_key(&self.vbufs, |vbuf| &vbuf.format));

        let mut vertex = smallvec::SmallVec::<[_; 16]>::new();

        let mut next = 0;
        for format in formats {
            if let Some(index) = find_compatible_buffer(&self.vbufs[next..], format) {
                // Ensure buffer is valid
                vertex.push((self.vbufs[index].buffer.raw(), 0));
                next = index + 1;
            } else {
                // Can't bind
                return Err(Incompatible);
            }
        }
        match self.ibuf.as_ref() {
            Some(ibuf) => {
                encoder.bind_index_buffer(ibuf.buffer.raw(), 0, ibuf.index_type);
                encoder.bind_vertex_buffers(0, vertex.iter().cloned());
            }
            None => {
                encoder.bind_vertex_buffers(0, vertex.iter().cloned());
            }
        }

        Ok(self.len)
    }
}

/// failure::Error type returned by `Mesh::bind` in case of mesh's vertex buffers are incompatible with requested vertex formats.
#[derive(Clone, Copy, Debug)]
pub struct Incompatible;

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

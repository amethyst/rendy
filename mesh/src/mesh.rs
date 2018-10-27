//!
//! Manage vertex and index buffers of single objects with ease.
//!

use std::borrow::Cow;
use std::mem::size_of;

use failure::Error;

use ash::{
    vk::{
        AccessFlags,
        BufferCreateInfo,
        BufferUsageFlags,
        IndexType,
        PrimitiveTopology,
    },
};
use smallvec::SmallVec;

use command::FamilyId;
use memory::usage::Data;
use resource::Buffer;
use factory::Factory;

use utils::{cast_cow, is_slice_sorted, is_slice_sorted_by_key};
use vertex::{AsVertex, VertexFormat};

/// Vertex buffer with it's format
#[derive(Debug)]
pub struct VertexBuffer {
    buffer: Buffer,
    format: VertexFormat<'static>,
    len: u32,
}

/// Index buffer with it's type
#[derive(Debug)]
pub struct IndexBuffer {
    buffer: Buffer,
    index_type: IndexType,
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MeshBuilder<'a> {
    vertices: SmallVec<[(Cow<'a, [u8]>, VertexFormat<'static>); 16]>,
    indices: Option<(Cow<'a, [u8]>, IndexType)>,
    prim: PrimitiveTopology,
}

impl<'a> MeshBuilder<'a> {
    /// Create empty builder.
    pub fn new() -> Self {
        MeshBuilder {
            vertices: SmallVec::new(),
            indices: None,
            prim: PrimitiveTopology::TRIANGLE_LIST,
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
            Indices::U16(i) => Some((cast_cow(i), IndexType::UINT16)),
            Indices::U32(i) => Some((cast_cow(i), IndexType::UINT32)),
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
        self.vertices
            .push((cast_cow(vertices.into()), V::VERTEX_FORMAT));
        self
    }

    /// Sets the primitive type of the mesh.
    ///
    /// By default, meshes are constructed as triangle lists.
    pub fn with_prim_type(mut self, prim: PrimitiveTopology) -> Self {
        self.prim = prim;
        self
    }

    /// Sets the primitive type of the mesh.
    ///
    /// By default, meshes are constructed as triangle lists.
    pub fn set_prim_type(&mut self, prim: PrimitiveTopology) -> &mut Self {
        self.prim = prim;
        self
    }

    /// Builds and returns the new mesh.
    pub fn build(
        &self,
        family: FamilyId,
        factory: &mut Factory,
    ) -> Result<Mesh, Error> {
        Ok(Mesh {
            vbufs: self
                .vertices
                .iter()
                .map(|(vertices, format)| {
                    let len = vertices.len() as u32 / format.stride;
                    Ok(VertexBuffer {
                        buffer: {
                            let mut buffer = factory.create_buffer(
                                BufferCreateInfo::builder()
                                    .size(vertices.len() as _)
                                    .usage(BufferUsageFlags::VERTEX_BUFFER | BufferUsageFlags::TRANSFER_DST)
                                    .build(),
                                1,
                                Data,
                            )?;
                            factory.upload_buffer(
                                &mut buffer,
                                0,
                                vertices,
                                family,
                                AccessFlags::VERTEX_ATTRIBUTE_READ,
                            )?;
                            buffer
                        },
                        format: format.clone(),
                        len,
                    })
                })
                .collect::<Result<_, Error>>()?,
            ibuf: match self.indices {
                None => None,
                Some((ref indices, index_type)) => {
                    let stride = match index_type {
                        IndexType::UINT16 => size_of::<u16>(),
                        IndexType::UINT32 => size_of::<u32>(),
                        _ => unreachable!(),
                    };
                    let len = indices.len() as u32 / stride as u32;
                    Some(IndexBuffer {
                        buffer: {
                            let mut buffer = factory.create_buffer(
                                BufferCreateInfo::builder()
                                    .size(indices.len() as _)
                                    .usage(BufferUsageFlags::INDEX_BUFFER | BufferUsageFlags::TRANSFER_DST)
                                    .build(),
                                1,
                                Data,
                            )?;
                            // factory.upload_buffer(
                            //     &mut buffer,
                            //     family,
                            //     AccessFlags::INDEX_BUFFER_READ,
                            //     0,
                            //     &indices,
                            // )?;
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
pub struct Mesh {
    vbufs: Vec<VertexBuffer>,
    ibuf: Option<IndexBuffer>,
    prim: PrimitiveTopology,
}

impl Mesh {
    /// Build new mesh with `HMeshBuilder`
    pub fn new<'a>() -> MeshBuilder<'a> {
        MeshBuilder::new()
    }

    /// PrimitiveTopology type of the `Mesh`
    pub fn primitive(&self) -> PrimitiveTopology {
        self.prim
    }

    // /// Bind buffers to specified attribute locations.
    // pub fn bind<'a>(
    //     &'a self,
    //     formats: &[VertexFormat],
    //     vertex: &mut VertexBufferSet<'a, B>,
    // ) -> Result<Bind<'a, B>, Incompatible> {
    //     debug_assert!(is_slice_sorted(formats));
    //     debug_assert!(is_slice_sorted_by_key(&self.vbufs, |vbuf| &vbuf.format));
    //     debug_assert!(vertex.0.is_empty());

    //     let mut next = 0;
    //     let mut vertex_count = None;
    //     for format in formats {
    //         if let Some(index) = find_compatible_buffer(&self.vbufs[next..], format) {
    //             // Ensure buffer is valid
    //             vertex.0.push((self.vbufs[index].buffer.raw(), 0));
    //             next = index + 1;
    //             assert!(vertex_count.is_none() || vertex_count == Some(self.vbufs[index].len));
    //             vertex_count = Some(self.vbufs[index].len);
    //         } else {
    //             // Can't bind
    //             return Err(Incompatible);
    //         }
    //     }
    //     Ok(self
    //         .ibuf
    //         .as_ref()
    //         .map(|ibuf| Bind::Indexed {
    //             buffer: ibuf.buffer.raw(),
    //             offset: 0,
    //             index_type: ibuf.index_type,
    //             count: ibuf.len,
    //         })
    //         .unwrap_or(Bind::Unindexed {
    //             count: vertex_count.unwrap_or(0),
    //         }))
    // }
}

/// Error type returned by `Mesh::bind` in case of mesh's vertex buffers are incompatible with requested vertex formats.
#[derive(Clone, Copy, Debug)]
pub struct Incompatible;

/// Result of buffers bindings.
/// It only contains `IndexBufferView` (if index buffers exists)
/// and vertex count.
/// Vertex buffers are in separate `VertexBufferSet`
#[derive(Copy, Clone, Debug)]
pub enum Bind<'a> {
    /// Indexed binding.
    Indexed {
        /// The buffer to bind.
        buffer: &'a Buffer,
        /// The offset into the buffer to start at.
        offset: u64,
        /// The type of the table elements (`u16` or `u32`).
        index_type: IndexType,
        /// Indices count to use in `draw_indexed` method.
        count: u32,
    },
    /// Not indexed binding.
    Unindexed {
        /// Vertex count to use in `draw` method.
        count: u32,
    },
}

// impl<'a> Bind<'a> {
//     /// Record drawing command for this biding.
//     pub fn draw(&self, vertex: VertexBufferSet, encoder: &mut RenderSubpassCommon) {
//         encoder.bind_vertex_buffers(0, vertex);
//         match *self {
//             Bind::Indexed {
//                 buffer,
//                 offset,
//                 index_type,
//                 count,
//             } => {
//                 encoder.bind_index_buffer(IndexBufferView {
//                     buffer,
//                     offset,
//                     index_type,
//                 });
//                 encoder.draw_indexed(0..count, 0, 0..1);
//             }
//             Bind::Unindexed { count } => {
//                 encoder.draw(0..count, 0..1);
//             }
//         }
//     }
// }

/// Helper function to find buffer with compatible format.
fn find_compatible_buffer(vbufs: &[VertexBuffer], format: &VertexFormat) -> Option<usize> {
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
fn is_compatible(left: &VertexFormat, right: &VertexFormat) -> bool {
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

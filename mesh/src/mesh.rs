//!
//! Manage vertex and index buffers of single objects with ease.
//!

use crate::{
    command::{EncoderCommon, Graphics, QueueId, RenderPassEncoder, Supports},
    core::cast_cow,
    factory::{BufferState, Factory, UploadError},
    memory::{Data, Upload, Write},
    resource::{Buffer, BufferInfo, Escape},
    AsVertex, VertexFormat,
};
use rendy_core::hal::adapter::PhysicalDevice;
use std::{borrow::Cow, mem::size_of};

/// Vertex buffer with it's format
#[derive(Debug)]
pub struct VertexBufferLayout {
    offset: u64,
    format: VertexFormat,
}

/// Index buffer with it's type
#[derive(Debug)]
pub struct IndexBuffer<B: rendy_core::hal::Backend> {
    buffer: Escape<Buffer<B>>,
    index_type: rendy_core::hal::IndexType,
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
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MeshBuilder<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    vertices: smallvec::SmallVec<[RawVertices<'a>; 16]>,
    #[cfg_attr(feature = "serde", serde(borrow))]
    indices: Option<RawIndices<'a>>,
    prim: rendy_core::hal::pso::Primitive,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct RawVertices<'a> {
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes", borrow))]
    vertices: Cow<'a, [u8]>,
    format: VertexFormat,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct RawIndices<'a> {
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes", borrow))]
    indices: Cow<'a, [u8]>,
    index_type: rendy_core::hal::IndexType,
}

fn index_stride(index_type: rendy_core::hal::IndexType) -> usize {
    match index_type {
        rendy_core::hal::IndexType::U16 => size_of::<u16>(),
        rendy_core::hal::IndexType::U32 => size_of::<u32>(),
    }
}

impl<'a> MeshBuilder<'a> {
    /// Create empty builder.
    pub fn new() -> Self {
        MeshBuilder {
            vertices: smallvec::SmallVec::new(),
            indices: None,
            prim: rendy_core::hal::pso::Primitive::TriangleList,
        }
    }

    /// Convert builder into fully owned type. This forces internal vertex and index buffers
    /// to be cloned, which allows borrowed source buffers to be released.
    pub fn into_owned(self) -> MeshBuilder<'static> {
        MeshBuilder {
            vertices: self
                .vertices
                .into_iter()
                .map(|v| RawVertices {
                    vertices: Cow::Owned(v.vertices.into_owned()),
                    format: v.format,
                })
                .collect(),
            indices: self.indices.map(|i| RawIndices {
                indices: Cow::Owned(i.indices.into_owned()),
                index_type: i.index_type,
            }),
            prim: self.prim,
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
                Indices::U16(i) => Some(RawIndices {
                    indices: cast_cow(i),
                    index_type: rendy_core::hal::IndexType::U16,
                }),
                Indices::U32(i) => Some(RawIndices {
                    indices: cast_cow(i),
                    index_type: rendy_core::hal::IndexType::U32,
                }),
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
            self.vertices.push(RawVertices {
                vertices: cast_cow(vertices.into()),
                format: V::vertex(),
            });
            self
        }

        /// Sets the primitive type of the mesh.
        ///
        /// By default, meshes are constructed as triangle lists.
        pub fn with_prim_type(mut self, prim: rendy_core::hal::pso::Primitive) -> Self {
            self.prim = prim;
            self
        }

        /// Sets the primitive type of the mesh.
        ///
        /// By default, meshes are constructed as triangle lists.
        pub fn set_prim_type(&mut self, prim: rendy_core::hal::pso::Primitive) -> &mut Self {
            self.prim = prim;
            self
        }

        /// Builds and returns the new mesh.
        ///
        /// A mesh expects all vertex buffers to have the same number of elements.
        /// If those are not equal, the length of smallest vertex buffer is selected,
        /// effectively discaring extra data from larger buffers.
        ///
        /// Note that contents of index buffer is not validated.
        pub fn build<B>(&self, queue: QueueId, factory: &Factory<B>) -> Result<Mesh<B>, UploadError>
        where
            B: rendy_core::hal::Backend,
        {
            let align = factory.physical().limits().non_coherent_atom_size;
            let mut len = self
                .vertices
                .iter()
                .map(|v| v.vertices.len() as u32 / v.format.stride)
                .min()
                .unwrap_or(0);

            let buffer_size = self
                .vertices
                .iter()
                .map(|v| (v.format.stride * len) as usize)
                .sum();

            let aligned_size = align_by(align, buffer_size) as u64;

            let mut staging = factory
                .create_buffer(
                    BufferInfo {
                        size: aligned_size,
                        usage: rendy_core::hal::buffer::Usage::TRANSFER_SRC,
                    },
                    Upload,
                )
                .map_err(UploadError::Create)?;

            let mut buffer = factory
                .create_buffer(
                    BufferInfo {
                        size: buffer_size as _,
                        usage: rendy_core::hal::buffer::Usage::VERTEX
                            | rendy_core::hal::buffer::Usage::TRANSFER_DST,
                    },
                    Data,
                )
                .map_err(UploadError::Create)?;

            let mut mapped = staging
                .map(factory, 0..aligned_size)
                .map_err(UploadError::Map)?;
            let mut writer =
                unsafe { mapped.write(factory, 0..aligned_size) }.map_err(UploadError::Map)?;
            let staging_slice = unsafe { writer.slice() };

            let mut offset = 0usize;
            let mut vertex_layouts: Vec<_> = self
                .vertices
                .iter()
                .map(|RawVertices { vertices, format }| {
                    let size = (format.stride * len) as usize;
                    staging_slice[offset..offset + size].copy_from_slice(&vertices[0..size]);
                    let this_offset = offset as u64;
                    offset += size;
                    VertexBufferLayout {
                        offset: this_offset,
                        format: format.clone(),
                    }
                })
                .collect();

            drop(writer);
            drop(mapped);

            vertex_layouts.sort_unstable_by(|a, b| a.format.cmp(&b.format));

            let index_buffer = match self.indices {
                None => None,
                Some(RawIndices {
                    ref indices,
                    index_type,
                }) => {
                    len = (indices.len() / index_stride(index_type)) as u32;
                    let mut buffer = factory
                        .create_buffer(
                            BufferInfo {
                                size: indices.len() as _,
                                usage: rendy_core::hal::buffer::Usage::INDEX
                                    | rendy_core::hal::buffer::Usage::TRANSFER_DST,
                            },
                            Data,
                        )
                        .map_err(UploadError::Create)?;
                    unsafe {
                        // New buffer can't be touched by device yet.
                        factory.upload_buffer(
                            &mut buffer,
                            0,
                            &indices,
                            None,
                            BufferState::new(queue)
                                .with_access(rendy_core::hal::buffer::Access::INDEX_BUFFER_READ)
                                .with_stage(rendy_core::hal::pso::PipelineStage::VERTEX_INPUT),
                        )?;
                    }

                    Some(IndexBuffer { buffer, index_type })
                }
            };

            unsafe {
                factory
                    .upload_from_staging_buffer(
                        &mut buffer,
                        0,
                        staging,
                        None,
                        BufferState::new(queue)
                            .with_access(rendy_core::hal::buffer::Access::VERTEX_BUFFER_READ)
                            .with_stage(rendy_core::hal::pso::PipelineStage::VERTEX_INPUT),
                    )
                    .map_err(UploadError::Upload)?;
            }

            Ok(Mesh {
                vertex_layouts,
                index_buffer,
                vertex_buffer: buffer,
                prim: self.prim,
                len,
            })
        }
    }

    fn align_by(align: usize, value: usize) -> usize {
        ((value + align - 1) / align) * align
    }

    /// Single mesh is a collection of buffer ranges that provides available attributes.
    /// Usually exactly one mesh is used per draw call.
    #[derive(Debug)]
    pub struct Mesh<B: rendy_core::hal::Backend> {
        vertex_buffer: Escape<Buffer<B>>,
        vertex_layouts: Vec<VertexBufferLayout>,
        index_buffer: Option<IndexBuffer<B>>,
        prim: rendy_core::hal::pso::Primitive,
        len: u32,
    }

    impl<B> Mesh<B>
    where
        B: rendy_core::hal::Backend,
    {
        /// Build new mesh with `MeshBuilder`
        pub fn builder<'a>() -> MeshBuilder<'a> {
            MeshBuilder::new()
        }

        /// rendy_core::hal::pso::Primitive type of the `Mesh`
        pub fn primitive(&self) -> rendy_core::hal::pso::Primitive {
            self.prim
        }

        /// Returns the number of vertices that will be drawn
        /// in the mesh.  For a mesh with no index buffer,
        /// this is the same as the number of vertices, or for
        /// a mesh with indices, this is the same as the number
        /// of indices.
        pub fn len(&self) -> u32 {
            self.len
        }

        fn get_vertex_iter<'a>(
            &'a self,
            formats: &[VertexFormat],
        ) -> Result<impl Iterator<Item = (&'a B::Buffer, u64)> + ExactSizeIterator, Incompatible> {
            debug_assert!(is_slice_sorted(formats), "Formats: {:#?}", formats);
            debug_assert!(is_slice_sorted_by_key(&self.vertex_layouts, |l| &l.format));

            let mut vertex = smallvec::SmallVec::<[_; 16]>::new();

            let mut next = 0;
            for format in formats {
                if let Some(index) = find_compatible_buffer(&self.vertex_layouts[next..], format) {
                    next += index;
                    vertex.push(self.vertex_layouts[next].offset);
                } else {
                    // Can't bind
                    return Err(Incompatible {
                        not_found: format.clone(),
                        in_formats: self
                            .vertex_layouts
                            .iter()
                            .map(|l| l.format.clone())
                            .collect(),
                    });
                }
            }

            let buffer = self.vertex_buffer.raw();
            Ok(vertex.into_iter().map(move |offset| (buffer, offset)))
        }

        /// Bind buffers to specified attribute locations.
        pub fn bind<C>(
        &self,
        first_binding: u32,
        formats: &[VertexFormat],
        encoder: &mut EncoderCommon<'_, B, C>,
    ) -> Result<u32, Incompatible>
    where
        C: Supports<Graphics>,
    {
        let vertex_iter = self.get_vertex_iter(formats)?;
        match self.index_buffer.as_ref() {
            Some(index_buffer) => unsafe {
                encoder.bind_index_buffer(index_buffer.buffer.raw(), 0, index_buffer.index_type);
                encoder.bind_vertex_buffers(first_binding, vertex_iter);
            },
            None => unsafe {
                encoder.bind_vertex_buffers(first_binding, vertex_iter);
            },
        }

        Ok(self.len)
    }

    /// Bind buffers to specified attribute locations and issue draw calls with given instance range.
    pub fn bind_and_draw(
        &self,
        first_binding: u32,
        formats: &[VertexFormat],
        instance_range: std::ops::Range<u32>,
        encoder: &mut RenderPassEncoder<'_, B>,
    ) -> Result<u32, Incompatible> {
        let vertex_iter = self.get_vertex_iter(formats)?;
        unsafe {
            match self.index_buffer.as_ref() {
                Some(index_buffer) => {
                    encoder.bind_index_buffer(
                        index_buffer.buffer.raw(),
                        0,
                        index_buffer.index_type,
                    );
                    encoder.bind_vertex_buffers(first_binding, vertex_iter);
                    encoder.draw_indexed(0..self.len, 0, instance_range);
                }
                None => {
                    encoder.bind_vertex_buffers(first_binding, vertex_iter);
                    encoder.draw(0..self.len, instance_range);
                }
            }
        }

        Ok(self.len)
    }
}

/// Error type returned by `Mesh::bind` in case of mesh's vertex buffers are incompatible with requested vertex formats.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompatible {
    /// Format that was queried but was not found
    pub not_found: VertexFormat,
    /// List of formats that were available at query time
    pub in_formats: Vec<VertexFormat>,
}

impl std::fmt::Display for Incompatible {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Vertex format {:?} is not compatible with any of {:?}.",
            self.not_found, self.in_formats
        )
    }
}
impl std::error::Error for Incompatible {}

/// Helper function to find buffer with compatible format.
fn find_compatible_buffer(
    vertex_layouts: &[VertexBufferLayout],
    format: &VertexFormat,
) -> Option<usize> {
    debug_assert!(is_slice_sorted(&*format.attributes));
    for (i, layout) in vertex_layouts.iter().enumerate() {
        debug_assert!(is_slice_sorted(&*layout.format.attributes));
        if is_compatible(&layout.format, format) {
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
            .position(|l| l == r)
            .map_or(false, |p| {
                skip += p;
                true
            })
    })
}

/// Chech if slice o f ordered values is sorted.
fn is_slice_sorted<T: Ord>(slice: &[T]) -> bool {
    is_slice_sorted_by_key(slice, |i| i)
}

/// Check if slice is sorted using ordered key and key extractor
fn is_slice_sorted_by_key<'a, T, K: Ord>(slice: &'a [T], f: impl Fn(&'a T) -> K) -> bool {
    if let Some((first, slice)) = slice.split_first() {
        let mut cmp = f(first);
        for item in slice {
            let item = f(item);
            if cmp > item {
                return false;
            }
            cmp = item;
        }
    }
    true
}

impl<'a, A> From<Vec<A>> for MeshBuilder<'a>
where
    A: AsVertex + 'a,
{
    fn from(vertices: Vec<A>) -> Self {
        MeshBuilder::new().with_vertices(vertices)
    }
}

macro_rules! impl_builder_from_vec {
    ($($from:ident),*) => {
        impl<'a, $($from,)*> From<($(Vec<$from>,)*)> for MeshBuilder<'a>
        where
            $($from: AsVertex + 'a,)*
        {
            fn from(vertices: ($(Vec<$from>,)*)) -> Self {
                #[allow(unused_mut)]
                let mut builder = MeshBuilder::new();
                #[allow(non_snake_case)]
                let ($($from,)*) = vertices;
                $(builder.add_vertices($from);)*
                builder
            }
        }

        impl_builder_from_vec!(@ $($from),*);
    };
    (@) => {};
    (@ $head:ident $(,$tail:ident)*) => {
        impl_builder_from_vec!($($tail),*);
    };
}

impl_builder_from_vec!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

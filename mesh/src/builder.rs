
use crate::{
    index_stride,
    command::QueueId,
    core::{cast_arbitrary_slice, cast_vec, cast_cow, hal::{Backend, IndexType}},
    factory::{BufferState, Factory, UploadError},
    memory::{Data, Upload, Write},
    resource::BufferInfo,
    AsVertex, VertexFormat,
    r#static::{IndexBuffer, Mesh, VertexBufferLayout}, align_by,
    dynamic::{DynamicMesh, DynamicVertices, DynamicIndices},
};
use rendy_core::hal::adapter::PhysicalDevice;
use std::{any::TypeId, borrow::Cow, mem::{align_of, MaybeUninit}};

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
    bytes: Cow<'a, [u8]>,
    format: VertexFormat,
    align: usize,
    ty: TypeId,
}

#[derive(Clone, Copy)] #[repr(C, align(1))] struct Aligned1(u8);
#[derive(Clone, Copy)] #[repr(C, align(2))] struct Aligned2(u8);
#[derive(Clone, Copy)] #[repr(C, align(4))] struct Aligned4(u8);
#[derive(Clone, Copy)] #[repr(C, align(8))] struct Aligned8(u8);
#[derive(Clone, Copy)] #[repr(C, align(16))] struct Aligned16(u8);
#[derive(Clone, Copy)] #[repr(C, align(32))] struct Aligned32(u8);
#[derive(Clone, Copy)] #[repr(C, align(64))] struct Aligned64(u8);
#[derive(Clone, Copy)] #[repr(C, align(128))] struct Aligned128(u8);
#[derive(Clone, Copy)] #[repr(C, align(256))] struct Aligned256(u8);
#[derive(Clone, Copy)] #[repr(C, align(512))] struct Aligned512(u8);

impl RawVertices<'_> {
    fn into_owned(self) -> RawVertices<'static> {
        let bytes = match self.bytes {
            Cow::Borrowed(bytes) => {
                unsafe { match self.align {
                    1 => cast_vec(Vec::<Aligned1>::from(cast_arbitrary_slice(bytes))),
                    2 => cast_vec(Vec::<Aligned2>::from(cast_arbitrary_slice(bytes))),
                    4 => cast_vec(Vec::<Aligned4>::from(cast_arbitrary_slice(bytes))),
                    8 => cast_vec(Vec::<Aligned8>::from(cast_arbitrary_slice(bytes))),
                    16 => cast_vec(Vec::<Aligned16>::from(cast_arbitrary_slice(bytes))),
                    32 => cast_vec(Vec::<Aligned32>::from(cast_arbitrary_slice(bytes))),
                    64 => cast_vec(Vec::<Aligned64>::from(cast_arbitrary_slice(bytes))),
                    128 => cast_vec(Vec::<Aligned128>::from(cast_arbitrary_slice(bytes))),
                    256 => cast_vec(Vec::<Aligned256>::from(cast_arbitrary_slice(bytes))),
                    512 => cast_vec(Vec::<Aligned512>::from(cast_arbitrary_slice(bytes))),
                    _ => panic!("Too aligned"),
                } }
            },
            Cow::Owned(owned) => owned,
        };

        RawVertices {
            bytes: Cow::Owned(bytes),
            format: self.format,
            align: self.align,
            ty: self.ty,
        }
    }

    fn into_dynamic(self) -> DynamicVertices {
        let bytes = match self.bytes {
            Cow::Borrowed(bytes) => {
                unsafe { match self.align {
                    1 => cast_vec(Vec::<Aligned1>::from(cast_arbitrary_slice(bytes))),
                    2 => cast_vec(Vec::<Aligned2>::from(cast_arbitrary_slice(bytes))),
                    4 => cast_vec(Vec::<Aligned4>::from(cast_arbitrary_slice(bytes))),
                    8 => cast_vec(Vec::<Aligned8>::from(cast_arbitrary_slice(bytes))),
                    16 => cast_vec(Vec::<Aligned16>::from(cast_arbitrary_slice(bytes))),
                    32 => cast_vec(Vec::<Aligned32>::from(cast_arbitrary_slice(bytes))),
                    64 => cast_vec(Vec::<Aligned64>::from(cast_arbitrary_slice(bytes))),
                    128 => cast_vec(Vec::<Aligned128>::from(cast_arbitrary_slice(bytes))),
                    256 => cast_vec(Vec::<Aligned256>::from(cast_arbitrary_slice(bytes))),
                    512 => cast_vec(Vec::<Aligned512>::from(cast_arbitrary_slice(bytes))),
                    _ => panic!("Too aligned"),
                } }
            },
            Cow::Owned(owned) => owned,
        };

        DynamicVertices {
            bytes,
            dirty: Vec::new(),
            ty: self.ty,
            offset: 0,
            size: 0,
            format: self.format,
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct RawIndices<'a> {
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes", borrow))]
    bytes: Cow<'a, [u8]>,
    ty: IndexType,
}

impl RawIndices<'_> {
    fn into_owned(self) -> RawIndices<'static> {
        let bytes = match self.bytes {
            Cow::Borrowed(bytes) => {
                unsafe { match self.ty {
                    IndexType::U16 => cast_vec(Vec::<u16>::from(cast_arbitrary_slice(bytes))),
                    IndexType::U32 => cast_vec(Vec::<u32>::from(cast_arbitrary_slice(bytes))),
                } }
            },
            Cow::Owned(owned) => owned,
        };

        RawIndices {
            bytes: Cow::Owned(bytes),
            ty: self.ty,
        }
    }

    fn into_dynamic(self) -> DynamicIndices {
        let bytes = match self.bytes {
            Cow::Borrowed(bytes) => {
                unsafe { match self.ty {
                    IndexType::U16 => cast_vec(Vec::<u16>::from(cast_arbitrary_slice(bytes))),
                    IndexType::U32 => cast_vec(Vec::<u32>::from(cast_arbitrary_slice(bytes))),
                } }
            },
            Cow::Owned(owned) => owned,
        };

        DynamicIndices {
            bytes,
            dirty: Vec::new(),
            ty: self.ty,
        }
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
                .map(RawVertices::into_owned)
                .collect(),
            indices: self.indices.map(RawIndices::into_owned),
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
                bytes: cast_cow(i),
                ty: IndexType::U16,
            }),
            Indices::U32(i) => Some(RawIndices {
                bytes: cast_cow(i),
                ty: IndexType::U32,
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
            bytes: cast_cow(vertices.into()),
            format: V::vertex(),
            align: align_of::<V>(),
            ty: TypeId::of::<V>(),
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
        B: Backend,
    {
        let align = factory.physical().limits().non_coherent_atom_size;
        let mut len = self
            .vertices
            .iter()
            .map(|v| v.bytes.len() / v.format.stride as usize)
            .min()
            .unwrap_or(0);

        let buffer_size = self
            .vertices
            .iter()
            .map(|v| v.format.stride as usize * len)
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
        let mut writer = unsafe {
            // New staging buffer cannot be accessed by device.
            mapped.write(factory, 0..aligned_size)
        }.map_err(UploadError::Map)?;

        let staging_slice: &mut [MaybeUninit<u8>] = unsafe {
            // Slize is never read.
            writer.slice()
        };

        let mut offset = 0usize;
        let mut vertex_layouts: Vec<_> = self
            .vertices
            .iter()
            .map(|v| {
                let size = v.format.stride as usize * len;
                unsafe {
                    debug_assert!(v.bytes.len() >= size); // "Ensured by `len` calculation
                    // `staging_slice` size is sum of all `size`s in this loop + alignment.
                    std::ptr::copy_nonoverlapping(v.bytes.as_ptr(), staging_slice.as_mut_ptr().add(offset) as *mut u8, size);
                }
                let this_offset = offset;
                offset += size;
                VertexBufferLayout {
                    offset: this_offset,
                    format: v.format.clone(),
                }
            })
            .collect();

        drop(staging_slice);
        drop(writer);
        drop(mapped);

        unsafe {
            factory
                .upload_from_staging_buffer(
                    &mut buffer,
                    staging,
                    None,
                    BufferState::new(queue)
                        .with_access(rendy_core::hal::buffer::Access::VERTEX_BUFFER_READ)
                        .with_stage(rendy_core::hal::pso::PipelineStage::VERTEX_INPUT),
                    Some(rendy_core::hal::command::BufferCopy {
                        src: 0,
                        dst: 0,
                        size: buffer_size as u64,
                    })
                )
                .map_err(UploadError::Upload)?;
        }

        vertex_layouts.sort_unstable_by(|a, b| a.format.cmp(&b.format));

        let index_buffer = match self.indices {
            None => None,
            Some(RawIndices {
                ref bytes,
                ty,
            }) => {
                len = bytes.len() / index_stride(ty);
                let mut buffer = factory
                    .create_buffer(
                        BufferInfo {
                            size: bytes.len() as _,
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
                        &bytes,
                        None,
                        BufferState::new(queue)
                            .with_access(rendy_core::hal::buffer::Access::INDEX_BUFFER_READ)
                            .with_stage(rendy_core::hal::pso::PipelineStage::VERTEX_INPUT),
                    )?;
                }

                Some(IndexBuffer { buffer, ty })
            }
        };

        Ok(Mesh {
            vertex_layouts,
            index_buffer,
            vertex_buffer: buffer,
            prim: self.prim,
            len: len as u32,
        })
    }

    /// Builds and returns the new dynamic mesh.
    /// 
    /// A mesh expects all vertex buffers to have the same number of elements.
    /// If those are not equal, the length of smallest vertex buffer is selected
    /// 
    /// Note that contents of index buffer is not validated.
    /// 
    /// In addition dynamic mesh can be modified and new vertices added.
    /// Set of vertex attributes or presense of index buffer cannot be changed.
    /// To apply modifications to underlying GPU buffers `DynamicMesh::update` must be called.
    pub fn build_dynamic<B>(self, queue: QueueId, factory: &Factory<B>) -> Result<DynamicMesh<B>, UploadError>
    where
        B: Backend,
    {
        let mesh = self.build(queue, factory)?;

        Ok(DynamicMesh {
            mesh,
            vertices: self.vertices.into_iter().map(RawVertices::into_dynamic).collect(),
            indices: self.indices.map(RawIndices::into_dynamic),
        })
    }
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

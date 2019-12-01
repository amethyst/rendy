
use crate::{
    builder::MeshBuilder,
    index_stride, align_by,
    command::QueueId,
    core::{cast_arbitrary_slice_mut, hal::{Backend, IndexType}},
    factory::{BufferState, Factory, UploadError},
    memory::{Data, Upload, Write},
    resource::BufferInfo,
    AsVertex, VertexFormat,
    r#static::{Mesh, IndexBuffer},
};
use rendy_core::hal::adapter::PhysicalDevice;
use std::{any::TypeId, mem::{size_of, align_of, MaybeUninit}, ops::{Deref, Range}};

pub(crate) struct DynamicVertices {
    pub(crate) bytes: Vec<u8>,
    pub(crate) dirty: Vec<Range<usize>>,
    pub(crate) ty: TypeId,
    pub(crate) offset: usize,
    pub(crate) size: usize,
    pub(crate) format: VertexFormat,
}

pub(crate)struct DynamicIndices {
    pub(crate) bytes: Vec<u8>,
    pub(crate) dirty: Vec<Range<usize>>,
    pub(crate) ty: IndexType,
}

/// Single mesh is a collection of buffer ranges that provides available attributes.
/// Usually exactly one mesh is used per draw call.
///
/// Dynamic mesh also allows modifying vertices and indices between frames.
pub struct DynamicMesh<B: Backend> {
    pub(crate) mesh: Mesh<B>,
    pub(crate) vertices: Vec<DynamicVertices>,
    pub(crate) indices: Option<DynamicIndices>,
}

#[derive(Clone, Copy, Debug)]
pub enum VerticesModifyError {
    OutOfRange {
        count: usize,
        requested: usize,
    },
    TypeMismatch {
        expected: TypeId,
        found: TypeId,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum IndicesModifyError {
    NoIndexBuffer,
    TypeMismatch {
        expected: IndexType,
        found: IndexType,
    },
}

impl<B> Deref for DynamicMesh<B>
where
    B: Backend,
{
    type Target = Mesh<B>;

    fn deref(&self) -> &Mesh<B> {
        &self.mesh
    }
}

impl<B> AsRef<Mesh<B>> for DynamicMesh<B>
where
    B: Backend,
{
    fn as_ref(&self) -> &Mesh<B> {
        &self.mesh
    }
}

impl<B> std::borrow::Borrow<Mesh<B>> for DynamicMesh<B>
where
    B: Backend,
{
    fn borrow(&self) -> &Mesh<B> {
        &self.mesh
    }
}

impl<B> DynamicMesh<B>
where
    B: Backend,
{
    /// Build new mesh with `MeshBuilder`
    pub fn builder<'a>() -> MeshBuilder<'a> {
        MeshBuilder::new()
    }

    /// Acquire slice of vertices from vertex buffer at `index`.
    /// All vertices from specifed range will be marked dirty and flushed on `DynamicMesh::update`.
    /// If requested range is larger than vertex array then it will be resized, filling new values with `fill` before returning.
    /// 
    /// # Panics
    ///
    /// This function will panic if wrong vertex type is requested.
    pub fn modify_vertices<V>(&mut self, index: usize, range: Range<u32>, fill: V) -> Result<&mut [V], VerticesModifyError>
    where
        V: AsVertex,
    {
        let len = self.vertices.len();
        let vertices = self.vertices.get_mut(index).ok_or_else(||
            VerticesModifyError::OutOfRange {
                count: len,
                requested: index,
            }
        )?;

        if vertices.ty != TypeId::of::<V>() {
            return Err(VerticesModifyError::TypeMismatch {
                expected: TypeId::of::<V>(),
                found: vertices.ty,
            });
        }

        let bytes_range = range.start as usize * size_of::<V>() .. range.end as usize * size_of::<V>();
        vertices.dirty.push(bytes_range.clone());

        if vertices.bytes.len() < bytes_range.end {
            let additional_bytes = bytes_range.end - vertices.bytes.len();
            assert_eq!(additional_bytes % size_of::<V>(), 0);
            vertices.bytes.reserve_exact(additional_bytes);
            unsafe {
                let ptr = vertices.bytes.as_mut_ptr().add(vertices.bytes.len());
                assert_eq!(ptr as usize % align_of::<V>(), 0, "Vector contains `V`s and so must be aligned");
                let ptr = ptr as *mut V;
                let additional = additional_bytes / size_of::<V>();
                for ptr in std::iter::successors(Some(ptr), |p| Some(p.add(1))).take(additional) {
                    std::ptr::write(ptr, fill);
                }
                vertices.bytes.set_len(bytes_range.end);
            }
        }

        Ok(unsafe {
            cast_arbitrary_slice_mut(&mut vertices.bytes[bytes_range])
        })
    }

    /// Acquire slice of indices from index buffer at `index`.
    /// All indices from specifed range will be marked dirty and flushed on `DynamicMesh::update`.
    /// If requested range is larger than index array then it will be resized, filling new values with `0` before returning.
    /// 
    /// # Panics
    ///
    /// This function will panic if wrong index type is requested.
    pub fn modify_indices_16(&mut self, range: Range<u32>) -> Result<&mut [u16], IndicesModifyError> {
        let indices = self.indices.as_mut().ok_or(IndicesModifyError::NoIndexBuffer)?;

        if indices.ty != IndexType::U16 {
            return Err(IndicesModifyError::TypeMismatch {
                expected: IndexType::U16,
                found: indices.ty,
            });
        }

        let bytes_range = range.start as usize * size_of::<u16>() .. range.end as usize * size_of::<u16>();

        if indices.bytes.len() < bytes_range.end {
            indices.bytes.resize(bytes_range.end, 0);
        }

        indices.dirty.push(bytes_range.clone());

        if indices.bytes.len() < bytes_range.end {
            let additional_bytes = bytes_range.end - indices.bytes.len();
            assert_eq!(additional_bytes % size_of::<u16>(), 0);
            indices.bytes.reserve_exact(additional_bytes);
            unsafe {
                let ptr = indices.bytes.as_mut_ptr().add(indices.bytes.len());
                std::ptr::write_bytes(ptr, 0, additional_bytes);
                indices.bytes.set_len(bytes_range.end);
            }
        }

        Ok(unsafe {
            cast_arbitrary_slice_mut(&mut indices.bytes[bytes_range])
        })
    }

    /// Acquire slice of indices from index buffer at `index`.
    /// All indices from specifed range will be marked dirty and flushed on `DynamicMesh::update`.
    /// If requested range is larger than index array then it will be resized, filling new values with `0` before returning.
    /// 
    /// # Panics
    ///
    /// This function will panic if wrong index type is requested.
    pub fn modify_indices_32(&mut self, range: Range<u32>) -> Result<&mut [u32], IndicesModifyError> {
        let indices = self.indices.as_mut().ok_or(IndicesModifyError::NoIndexBuffer)?;

        if indices.ty != IndexType::U32 {
            return Err(IndicesModifyError::TypeMismatch {
                expected: IndexType::U32,
                found: indices.ty,
            });
        }

        let bytes_range = range.start as usize * size_of::<u32>() .. range.end as usize * size_of::<u32>();

        if indices.bytes.len() < bytes_range.end {
            indices.bytes.resize(bytes_range.end, 0);
        }

        indices.dirty.push(bytes_range.clone());

        if indices.bytes.len() < bytes_range.end {
            let additional_bytes = bytes_range.end - indices.bytes.len();
            assert_eq!(additional_bytes % size_of::<u32>(), 0);
            indices.bytes.reserve_exact(additional_bytes);
            unsafe {
                let ptr = indices.bytes.as_mut_ptr().add(indices.bytes.len());
                std::ptr::write_bytes(ptr, 0, additional_bytes);
                indices.bytes.set_len(bytes_range.end);
            }
        }

        Ok(unsafe {
            cast_arbitrary_slice_mut(&mut indices.bytes[bytes_range])
        })
    }

    pub fn update(&mut self, queue: QueueId, factory: &Factory<B>) -> Result<(), UploadError> {
        let len = self.vertices
            .iter()
            .map(|v| v.bytes.len() / v.format.stride as usize)
            .min()
            .unwrap_or(0);

        if self.vertices.iter().all(|v| v.size >= len * v.format.stride as usize) {
            self.update_vertex_buffer(queue, factory)?;
        } else {
            self.build_vertex_buffer(queue, factory)?;
        }

        if let Some(indices) = &mut self.indices {
            match &self.mesh.index_buffer {
                Some(index_buffer) if index_buffer.buffer.size() >= indices.bytes.len() as u64 => {
                    self.update_index_buffer(queue, factory)?;    
                },
                _ => {
                    self.build_index_buffer(queue, factory)?;
                }
            }
        }

        Ok(())
    }

    fn build_vertex_buffer(&mut self, queue: QueueId, factory: &Factory<B>) -> Result<(), UploadError> {
        let align = factory.physical().limits().non_coherent_atom_size;
        let len = self.vertices
            .iter()
            .map(|v| v.bytes.len() / v.format.stride as usize)
            .min()
            .unwrap_or(0);

        let buffer_size = self.vertices
            .iter()
            .map(|v| v.format.stride as usize * len)
            .sum();

        let staging_size = align_by(align, buffer_size) as u64;

        let mut staging = factory
            .create_buffer(
                BufferInfo {
                    size: staging_size,
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
            .map(factory, 0..staging_size)
            .map_err(UploadError::Map)?;
        let mut writer =
            unsafe { mapped.write(factory, 0..staging_size) }.map_err(UploadError::Map)?;
        let staging_slice: &mut [MaybeUninit<u8>] = unsafe { writer.slice() };

        let mut offset = 0usize;
        for v in &mut self.vertices {
            let size = v.format.stride as usize * len;
            unsafe {
                debug_assert!(v.bytes.len() >= size); // "Ensured by `len` calculation
                // `staging_slice` size is sum of all `size`s in this loop + alignment.
                std::ptr::copy_nonoverlapping(v.bytes.as_ptr(), staging_slice.as_mut_ptr().add(offset) as *mut u8, size);
            }
            v.offset = offset;
            v.size = size;
            offset += size;
        }

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

        assert_eq!(self.mesh.vertex_layouts.len(), self.vertices.len());
        for (l, v) in self.mesh.vertex_layouts.iter_mut().zip(&self.vertices) {
            assert_eq!(l.format, v.format);
            l.offset = v.offset;
        }
        self.mesh.vertex_buffer = buffer;
        if self.indices.is_none() {
            self.mesh.len = len as u32;
        }

        Ok(())
    }

    fn update_vertex_buffer(&mut self, queue: QueueId, factory: &Factory<B>) -> Result<(), UploadError> {
        let dirty_bytes_total = self.vertices.iter_mut().flat_map(|v| {
            v.dirty.sort_by_key(|d| d.start);

            let mut j = 0;
            for i in 1 .. v.dirty.len() {
                if v.dirty[j].end >= v.dirty[i].start {
                    // merge
                    v.dirty[j].end = std::cmp::max(v.dirty[j].end, v.dirty[i].end);
                } else {
                    // next
                    j += 1;
                }
            }

            v.dirty.truncate(j);
            v.dirty.iter().map(|d| d.end - d.start)
        }).sum();

        if dirty_bytes_total == 0 {
            return Ok(());
        }

        let align = factory.physical().limits().non_coherent_atom_size;
        let staging_size = align_by(align, dirty_bytes_total) as u64;

        let mut staging = factory
            .create_buffer(
                BufferInfo {
                    size: staging_size,
                    usage: rendy_core::hal::buffer::Usage::TRANSFER_SRC,
                },
                Upload,
            )
            .map_err(UploadError::Create)?;

        let mut mapped = staging
            .map(factory, 0..staging_size)
            .map_err(UploadError::Map)?;
        let mut writer =
            unsafe { mapped.write(factory, 0..staging_size) }.map_err(UploadError::Map)?;
        let staging_slice: &mut [MaybeUninit<u8>] = unsafe { writer.slice() };

        let mut offset = 0usize;
        for v in &self.vertices {
            for d in &v.dirty {
                let size = d.end - d.start;
                unsafe {
                    debug_assert!(v.bytes.len() >= d.end); // "Ensured by `len` calculation
                    // `staging_slice` size is sum of all `size`s in this loop + alignment.
                    std::ptr::copy_nonoverlapping(v.bytes.as_ptr().add(d.start), staging_slice.as_mut_ptr().add(offset) as *mut u8, size);
                }
                offset += size;
            }
        }

        drop(staging_slice);
        drop(writer);
        drop(mapped);

        let state = BufferState::new(queue)
            .with_access(rendy_core::hal::buffer::Access::VERTEX_BUFFER_READ)
            .with_stage(rendy_core::hal::pso::PipelineStage::VERTEX_INPUT);

        let mut offset = 0usize;
        unsafe {
            factory
                .upload_from_staging_buffer(
                    &mut self.mesh.vertex_buffer,
                    staging,
                    Some(state),
                    state,
                    self.vertices.iter().flat_map(|v| v.dirty.iter()).map(|d| {
                        let size = d.end - d.start;
                        offset += size;
                        rendy_core::hal::command::BufferCopy {
                            src: (offset - size) as u64,
                            dst: d.start as u64,
                            size: size as u64,
                        }
                    }),
                )
                .map_err(UploadError::Upload)?;
        }

        for v in &mut self.vertices {
            v.dirty.clear();
        }

        Ok(())
    }

    fn build_index_buffer(&mut self, queue: QueueId, factory: &Factory<B>) -> Result<(), UploadError> {
        if let Some(indices) = &mut self.indices {
            let len = (indices.bytes.len() / index_stride(indices.ty)) as u32;
            let mut buffer = factory
                .create_buffer(
                    BufferInfo {
                        size: indices.bytes.len() as _,
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
                    &indices.bytes,
                    None,
                    BufferState::new(queue)
                        .with_access(rendy_core::hal::buffer::Access::INDEX_BUFFER_READ)
                        .with_stage(rendy_core::hal::pso::PipelineStage::VERTEX_INPUT),
                )?;
            }

            self.mesh.index_buffer = Some(IndexBuffer { buffer, ty: indices.ty });
            self.mesh.len = len;
        }

        Ok(())
    }

    fn update_index_buffer(&mut self, queue: QueueId, factory: &Factory<B>) -> Result<(), UploadError> {
        let indices = self.indices.as_mut().unwrap();
        let index_buffer = self.mesh.index_buffer.as_mut().unwrap();

        indices.dirty.sort_by_key(|d| d.start);

        let mut j = 0;
        for i in 1 .. indices.dirty.len() {
            if indices.dirty[j].end >= indices.dirty[i].start {
                // merge
                indices.dirty[j].end = std::cmp::max(indices.dirty[j].end, indices.dirty[i].end);
            } else {
                // next
                j += 1;
            }
        }

        indices.dirty.truncate(j);
        let dirty_bytes_total = indices.dirty.iter().map(|d| d.end - d.start).sum();
    
        if dirty_bytes_total == 0 {
            return Ok(());
        }

        let align = factory.physical().limits().non_coherent_atom_size;
        let staging_size = align_by(align, dirty_bytes_total) as u64;

        let mut staging = factory
            .create_buffer(
                BufferInfo {
                    size: staging_size,
                    usage: rendy_core::hal::buffer::Usage::TRANSFER_SRC,
                },
                Upload,
            )
            .map_err(UploadError::Create)?;

        let mut mapped = staging
            .map(factory, 0..staging_size)
            .map_err(UploadError::Map)?;
        let mut writer =
            unsafe { mapped.write(factory, 0..staging_size) }.map_err(UploadError::Map)?;
        let staging_slice: &mut [MaybeUninit<u8>] = unsafe { writer.slice() };

        let mut offset = 0usize;
        for d in &indices.dirty {
            let size = d.end - d.start;
            unsafe {
                debug_assert!(indices.bytes.len() >= d.end); // "Ensured by `len` calculation
                // `staging_slice` size is sum of all `size`s in this loop + alignment.
                std::ptr::copy_nonoverlapping(indices.bytes.as_ptr().add(d.start), staging_slice.as_mut_ptr().add(offset) as *mut u8, size);
            }
            offset += size;
        }

        drop(staging_slice);
        drop(writer);
        drop(mapped);

        let state = BufferState::new(queue)
            .with_access(rendy_core::hal::buffer::Access::INDEX_BUFFER_READ)
            .with_stage(rendy_core::hal::pso::PipelineStage::VERTEX_INPUT);

        let mut offset = 0usize;
        unsafe {
            factory
                .upload_from_staging_buffer(
                    &mut index_buffer.buffer,
                    staging,
                    Some(state),
                    state,
                    indices.dirty.iter().map(|d| {
                        let size = d.end - d.start;
                        offset += size;
                        rendy_core::hal::command::BufferCopy {
                            src: (offset - size) as u64,
                            dst: d.start as u64,
                            size: size as u64,
                        }
                    }),
                )
                .map_err(UploadError::Upload)?;
        }

        indices.dirty.clear();
        Ok(())
    }
}



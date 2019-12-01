
use crate::{
    Incompatible, is_slice_sorted, is_slice_sorted_by_key, is_compatible,
    command::{EncoderCommon, Graphics, RenderPassEncoder, Supports},
    core::hal::Backend,
    resource::{Buffer, Escape},
    VertexFormat,
    builder::MeshBuilder,
};

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

/// Vertex buffer with it's format
#[derive(Debug)]
pub(crate) struct VertexBufferLayout {
    pub(crate) offset: usize,
    pub(crate) format: VertexFormat,
}

/// Index buffer with it's type
#[derive(Debug)]
pub(crate) struct IndexBuffer<B: Backend> {
    pub(crate) buffer: Escape<Buffer<B>>,
    pub(crate) ty: rendy_core::hal::IndexType,
}


/// Single mesh is a collection of buffer ranges that provides available attributes.
/// Usually exactly one mesh is used per draw call.
#[derive(Debug)]
pub struct Mesh<B: Backend> {
    pub(crate) vertex_buffer: Escape<Buffer<B>>,
    pub(crate) vertex_layouts: Vec<VertexBufferLayout>,
    pub(crate) index_buffer: Option<IndexBuffer<B>>,
    pub(crate) prim: rendy_core::hal::pso::Primitive,
    pub(crate) len: u32,
}

impl<B> Mesh<B>
where
    B: Backend,
{
    /// Build new mesh with `MeshBuilder`
    pub fn builder<'a>() -> MeshBuilder<'a> {
        MeshBuilder::new()
    }

    /// Primitive type of the `Mesh`
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
    ) -> Result<impl IntoIterator<Item = (&'a B::Buffer, u64)>, Incompatible> {
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
        Ok(vertex.into_iter().map(move |offset| (buffer, offset as u64)))
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
                encoder.bind_index_buffer(index_buffer.buffer.raw(), 0, index_buffer.ty);
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
                        index_buffer.ty,
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

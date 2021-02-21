// use std::fmt;

/// Memory object wrapper.
/// Contains size and properties of the memory.
#[derive(Debug)]
pub struct Memory<B: gfx_hal::Backend> {
    raw: B::Memory,
    size: u64,
    properties: gfx_hal::memory::Properties,
    non_coherent_atom_size: u64,
    relevant: relevant::Relevant,
}

impl<B> Memory<B>
where
    B: gfx_hal::Backend,
{
    /// Get memory properties.
    pub fn properties(&self) -> gfx_hal::memory::Properties {
        self.properties
    }

    /// Get memory size.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get raw memory.
    pub fn raw(&self) -> &B::Memory {
        &self.raw
    }
    /// Get raw mutable memory.
    pub fn raw_mut(&mut self) -> &mut B::Memory {
        &mut self.raw
    }

    /// Unwrap raw memory.
    pub fn into_raw(self) -> B::Memory {
        self.relevant.dispose();
        self.raw
    }

    pub(crate) fn non_coherent_atom_size(&self) -> u64 {
        debug_assert!(
            self.host_visible() && !self.host_coherent(),
            "Irrelevent and shouldn't be called",
        );
        self.non_coherent_atom_size
    }

    /// Create memory from raw object.
    ///
    /// # Safety
    ///
    /// TODO:
    pub unsafe fn from_raw(
        raw: B::Memory,
        size: u64,
        properties: gfx_hal::memory::Properties,
        non_coherent_atom_size: u64,
    ) -> Self {
        Memory {
            properties,
            raw,
            size,
            non_coherent_atom_size,
            relevant: relevant::Relevant,
        }
    }

    /// Check if this memory is host-visible and can be mapped.
    /// `memory.host_visible()` is equivalent to `memory.properties().contains(Properties::CPU_VISIBLE)`
    pub fn host_visible(&self) -> bool {
        self.properties
            .contains(gfx_hal::memory::Properties::CPU_VISIBLE)
    }

    /// Check if this memory is host-coherent and doesn't require invalidating or flushing.
    /// `memory.host_coherent()` is equivalent to `memory.properties().contains(Properties::COHERENT)`
    pub fn host_coherent(&self) -> bool {
        self.properties
            .contains(gfx_hal::memory::Properties::COHERENT)
    }
}

// pub(crate) fn memory_ptr_fmt(
//     memory: &*const Memory,
//     fmt: &mut fmt::Formatter<'_>,
// ) -> Result<(), fmt::Error> {
//     unsafe {
//         if fmt.alternate() {
//             write!(fmt, "*const {:#?}", **memory)
//         } else {
//             write!(fmt, "*const {:?}", **memory)
//         }
//     }
// }

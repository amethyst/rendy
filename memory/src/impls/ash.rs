use ash::{
    self, version::{DeviceV1_0, FunctionPointers},
};
use device::Device;
use error::*;
use smallvec::SmallVec;
use std::{
    ops::Range, ptr::{null, null_mut, NonNull},
};

impl From<ash::vk::Result> for OutOfMemoryError {
    fn from(result: ash::vk::Result) -> OutOfMemoryError {
        match result {
            ash::vk::Result::Success => panic!("Unexpected success"),
            ash::vk::Result::ErrorOutOfHostMemory => OutOfMemoryError::OutOfHostMemory,
            ash::vk::Result::ErrorOutOfDeviceMemory => OutOfMemoryError::OutOfDeviceMemory,
            _ => panic!("unexpected error"),
        }
    }
}

impl From<ash::vk::Result> for MappingError {
    fn from(result: ash::vk::Result) -> MappingError {
        match result {
            ash::vk::Result::Success => panic!("Unexpected success"),
            ash::vk::Result::ErrorOutOfHostMemory => OutOfMemoryError::OutOfHostMemory.into(),
            ash::vk::Result::ErrorOutOfDeviceMemory => OutOfMemoryError::OutOfDeviceMemory.into(),
            ash::vk::Result::ErrorMemoryMapFailed => MappingError::MappingFailed,
            _ => panic!("unexpected error"),
        }
    }
}

impl From<ash::vk::Result> for AllocationError {
    fn from(result: ash::vk::Result) -> AllocationError {
        match result {
            ash::vk::Result::Success => panic!("Unexpected success"),
            ash::vk::Result::ErrorOutOfHostMemory => OutOfMemoryError::OutOfHostMemory.into(),
            ash::vk::Result::ErrorOutOfDeviceMemory => OutOfMemoryError::OutOfDeviceMemory.into(),
            _ => panic!("unexpected error"),
        }
    }
}

impl From<ash::vk::Result> for MemoryError {
    fn from(result: ash::vk::Result) -> MemoryError {
        match result {
            ash::vk::Result::Success => panic!("Unexpected success"),
            ash::vk::Result::ErrorOutOfHostMemory => OutOfMemoryError::OutOfHostMemory.into(),
            ash::vk::Result::ErrorOutOfDeviceMemory => OutOfMemoryError::OutOfDeviceMemory.into(),
            ash::vk::Result::ErrorMemoryMapFailed => MappingError::MappingFailed.into(),
            _ => panic!("unexpected error"),
        }
    }
}

impl<V> Device for ash::Device<V>
where
    V: FunctionPointers,
    ash::Device<V>: DeviceV1_0,
{
    type Memory = ash::vk::DeviceMemory;

    unsafe fn allocate(
        &self,
        index: u32,
        size: u64,
    ) -> Result<ash::vk::DeviceMemory, AllocationError> {
        Ok(self.allocate_memory(
            &ash::vk::MemoryAllocateInfo {
                s_type: ash::vk::StructureType::MemoryAllocateInfo,
                p_next: null(),
                allocation_size: size,
                memory_type_index: index,
            },
            None,
        )?)
    }

    unsafe fn free(&self, memory: ash::vk::DeviceMemory) {
        self.free_memory(memory, None);
    }

    unsafe fn map(
        &self,
        memory: &ash::vk::DeviceMemory,
        range: Range<u64>,
    ) -> Result<NonNull<u8>, MappingError> {
        let ptr = self.map_memory(
            *memory,
            range.start,
            range.end - range.start,
            ash::vk::MemoryMapFlags::empty(),
        )?;
        debug_assert_ne!(ptr, null_mut());
        Ok(NonNull::new_unchecked(ptr as *mut u8))
    }

    unsafe fn unmap(&self, memory: &ash::vk::DeviceMemory) {
        self.unmap_memory(*memory)
    }

    unsafe fn invalidate<'a>(
        &self,
        regions: impl IntoIterator<Item = (&'a ash::vk::DeviceMemory, Range<u64>)>,
    ) -> Result<(), OutOfMemoryError> {
        let ranges = regions
            .into_iter()
            .map(|(memory, range)| ash::vk::MappedMemoryRange {
                s_type: ash::vk::StructureType::MappedMemoryRange,
                p_next: null(),
                memory: *memory,
                offset: range.start,
                size: range.end - range.start,
            })
            .collect::<SmallVec<[_; 32]>>();
        self.invalidate_mapped_memory_ranges(&ranges)?;
        Ok(())
    }

    unsafe fn flush<'a>(
        &self,
        regions: impl IntoIterator<Item = (&'a ash::vk::DeviceMemory, Range<u64>)>,
    ) -> Result<(), OutOfMemoryError> {
        let ranges = regions
            .into_iter()
            .map(|(memory, range)| ash::vk::MappedMemoryRange {
                s_type: ash::vk::StructureType::MappedMemoryRange,
                p_next: null(),
                memory: *memory,
                offset: range.start,
                size: range.end - range.start,
            })
            .collect::<SmallVec<[_; 32]>>();
        self.flush_mapped_memory_ranges(&ranges)?;
        Ok(())
    }
}

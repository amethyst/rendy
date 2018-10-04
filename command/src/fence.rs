

bitflags!{
    /// Flags to specify initial state and behavior of the fence.
    #[derive(Default)]
    pub struct FenceCreateFlags: u32 {
        /// Create fence in signaled state.
        const CREATE_SIGNALED = 0x00000001;
    }
}

/// Create info for fence.
#[derive(Clone, Copy, Debug, Default)]
pub struct FenceCreateInfo {
    /// Creation flags.
    pub flags: FenceCreateFlags,
}


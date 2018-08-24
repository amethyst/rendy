

bitflags! {
    pub struct Flags: u32 {
        const TRANSFER_SRC = 0x00000001;
        const TRANSFER_DST = 0x00000002;
        const SAMPLED = 0x00000004;
        const STORAGE = 0x00000008;
        const COLOR_ATTACHMENT = 0x00000010;
        const DEPTH_STENCIL_ATTACHMENT = 0x00000020;
        const TRANSIENT_ATTACHMENT = 0x00000040;
        const INPUT_ATTACHMENT = 0x00000080;
    }
}

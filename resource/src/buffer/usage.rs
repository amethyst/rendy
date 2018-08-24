
bitflags! {
    pub struct Flags: u32 {
        const TRANSFER_SRC = 0x00000001;
        const TRANSFER_DST = 0x00000002;
        const UNIFORM_TEXEL_BUFFER = 0x00000004;
        const STORAGE_TEXEL_BUFFER = 0x00000008;
        const UNIFORM_BUFFER = 0x00000010;
        const STORAGE_BUFFER = 0x00000020;
        const INDEX_BUFFER = 0x00000040;
        const VERTEX_BUFFER = 0x00000080;
        const INDIRECT_BUFFER = 0x00000100;
    }
}

pub trait Usage {
    fn flags(&self) -> Flags;
}

#[derive(Debug)]
pub struct UsageValue(Flags);

impl Usage for UsageValue {
    fn flags(&self) -> Flags {
        self.0
    }
}

#[derive(Debug)]
pub struct VertexBuffer;

impl Usage for VertexBuffer {
    fn flags(&self) -> Flags {
        Flags::TRANSFER_DST | Flags::VERTEX_BUFFER
    }
}

#[derive(Debug)]
pub struct IndexBuffer;

impl Usage for IndexBuffer {
    fn flags(&self) -> Flags {
        Flags::TRANSFER_DST | Flags::INDEX_BUFFER
    }
}

#[derive(Debug)]
pub struct UniformBuffer;

impl Usage for UniformBuffer {
    fn flags(&self) -> Flags {
        Flags::UNIFORM_BUFFER
    }
}

#[derive(Debug)]
pub struct UploadBuffer;

impl Usage for UploadBuffer {
    fn flags(&self) -> Flags {
        Flags::TRANSFER_SRC
    }
}

#[derive(Debug)]
pub struct DownloadBuffer;

impl Usage for DownloadBuffer {
    fn flags(&self) -> Flags {
        Flags::TRANSFER_DST
    }
}

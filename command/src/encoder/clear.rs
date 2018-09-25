

/// Value to clear color.
#[derive(Clone, Copy, Debug)]
pub enum ClearColor {
    /// Floating pointer clear value.
    Float([f32; 4]),

    /// Signed integer pointer clear value.
    Int([i32; 4]),

    /// Unsigned integer pointer clear value.
    UInt([u32; 4]),
}

/// Value to clear depth and stencil.
#[derive(Clone, Copy, Debug)]
pub struct ClearDepthStencil {
    /// Depth clear value.
    pub depth: f32,

    /// Stencil clear value.
    pub stencil: u32,
}

/// Value to clear image.
#[derive(Clone, Copy, Debug)]
pub enum ClearValue {
    /// Color clear value.
    Color(ClearColor),

    /// Depth-stencil clear value.
    DepthStencil(ClearDepthStencil),
}

impl From<ClearColor> for ClearValue {
    fn from(value: ClearColor) -> ClearValue {
        ClearValue::Color(value)
    }
}

impl From<ClearDepthStencil> for ClearValue {
    fn from(value: ClearDepthStencil) -> ClearValue {
        ClearValue::DepthStencil(value)
    }
}


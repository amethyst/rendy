//! Typed pixel formats.

/// Normalized unsigned integer representation
#[derive(Clone, Copy, Debug, Default)]
pub struct Unorm;

/// Normalized signed integer representation
#[derive(Clone, Copy, Debug, Default)]
pub struct Inorm;

/// Unsigned integer representation
#[derive(Clone, Copy, Debug, Default)]
pub struct Uint;

/// Signed integer representation
#[derive(Clone, Copy, Debug, Default)]
pub struct Int;

/// ???
#[derive(Clone, Copy, Debug, Default)]
pub struct Uscaled;

/// ???
#[derive(Clone, Copy, Debug, Default)]
pub struct Iscaled;

/// `Unorm` but in with srgb gammar correction.
#[derive(Clone, Copy, Debug, Default)]
pub struct Srgb;

/// Floating point representation.
#[derive(Clone, Copy, Debug, Default)]
pub struct Float;

/// 8 bits marker type
#[derive(Clone, Copy, Debug, Default)]
pub struct _8;

/// 16 bits marker type
#[derive(Clone, Copy, Debug, Default)]
pub struct _16;

/// 32 bits marker type
#[derive(Clone, Copy, Debug, Default)]
pub struct _32;

/// 64 bits marker type
#[derive(Clone, Copy, Debug, Default)]
pub struct _64;

/// Size of the channel.
pub trait ChannelSize {
    /// Channel representation.
    const SIZE: u32;
}

impl ChannelSize for _8 {
    const SIZE: u32 = 1;
}
impl ChannelSize for _16 {
    const SIZE: u32 = 2;
}
impl ChannelSize for _32 {
    const SIZE: u32 = 4;
}
impl ChannelSize for _64 {
    const SIZE: u32 = 8;
}

/// Channel representation.
pub trait ChannelRepr<S> {
    /// Channel representation.
    type Repr: Sized + std::fmt::Debug + Default + Copy + Send + Sync + 'static;
}

macro_rules! impl_channel_repr {
    ($($type:ident * $size:ident = $repr:ident;)*) => {
        $(
            impl ChannelRepr<$size> for $type { type Repr = $repr; }
        )*
    };
}

impl_channel_repr! {
    Unorm * _8 = u8;
    Inorm * _8 = u8;
    Uint * _8 = u8;
    Int * _8 = u8;
    Uscaled * _8 = u8;
    Iscaled * _8 = u8;
    Srgb * _8 = u8;

    Unorm * _16 = u16;
    Inorm * _16 = u16;
    Uint * _16 = u16;
    Int * _16 = u16;
    Uscaled * _16 = u16;
    Iscaled * _16 = u16;
    Srgb * _16 = u16;

    Unorm * _32 = u32;
    Inorm * _32 = u32;
    Uint * _32 = u32;
    Int * _32 = u32;
    Uscaled * _32 = u32;
    Iscaled * _32 = u32;
    Srgb * _32 = u32;
    Float * _32 = f32;

    Unorm * _64 = u64;
    Inorm * _64 = u64;
    Uint * _64 = u64;
    Int * _64 = u64;
    Uscaled * _64 = u64;
    Iscaled * _64 = u64;
    Srgb * _64 = u64;
    Float * _64 = f64;
}

/// Read channel.
#[derive(Clone, Copy, Debug, Default)]
pub struct R;

/// Read-green channels.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rg;

/// Read-green-blue channels.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rgb;

/// Read-green-blue-alpha channels.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rgba;

/// Blue-green-red channels.
#[derive(Clone, Copy, Debug, Default)]
pub struct Bgr;

/// Blue-green-red-alpha channels.
#[derive(Clone, Copy, Debug, Default)]
pub struct Bgra;

/// Alpha-blue-green-red channels.
#[derive(Clone, Copy, Debug, Default)]
pub struct Abgr;

/// Pixel representation.
pub trait PixelRepr<S, T> {
    /// Pixel representation.
    type Repr: Sized + std::fmt::Debug + Default + Copy + Send + Sync + 'static;
}

macro_rules! num_channels {
    (R) => {
        1
    };
    (Rg) => {
        2
    };
    (Rgb) => {
        3
    };
    (Rgba) => {
        4
    };
    (Bgr) => {
        3
    };
    (Bgra) => {
        4
    };
    (Abgr) => {
        4
    };
}

macro_rules! impl_pixel_repr {
    ($($channels:ident;)*) => {
        $(
            impl<S, T> PixelRepr<S, T> for $channels
            where
                S: ChannelSize,
                T: ChannelRepr<S>,
            {
                type Repr = [<T as ChannelRepr<S>>::Repr; num_channels!($channels)];
            }
        )*
    };
}

impl_pixel_repr! {
    R;
    Rg;
    Rgb;
    Rgba;
    Bgr;
    Bgra;
    Abgr;
}

/// One pixel
#[derive(derivative::Derivative)]
#[derivative(
    Clone(bound = ""),
    Copy(bound = ""),
    Debug(bound = ""),
    Default(bound = "")
)]
#[repr(transparent)]
pub struct Pixel<C, S, T>
where
    C: PixelRepr<S, T>,
{
    /// Pixel representation.
    pub repr: <C as PixelRepr<S, T>>::Repr,
}

/// Pixel trait.
pub trait AsPixel: Copy + std::fmt::Debug + Default + Send + Sync + 'static {
    /// Name of the pixel type.
    const NAME: &'static str;

    /// Size of the pixel.
    const SIZE: u32;

    /// Pixel format.
    const FORMAT: gfx_hal::format::Format;
}

macro_rules! impl_pixel {
    ($($alias:ident = $channels:ident $size:ident $type:ident;)*) => {
        $(
            /// Pixel type alias.
            pub type $alias = Pixel<$channels, $size, $type>;

            impl AsPixel for $alias {
                const NAME: &'static str = stringify!($alias);
                const SIZE: u32 = num_channels!($channels) * <$size as ChannelSize>::SIZE;
                const FORMAT: gfx_hal::format::Format = gfx_hal::format::Format::$alias;
            }
        )*
    };
}

impl_pixel! {
    R8Unorm = R _8 Unorm;
    R8Inorm = R _8 Inorm;
    R8Uscaled = R _8 Uscaled;
    R8Iscaled = R _8 Iscaled;
    R8Uint = R _8 Uint;
    R8Int = R _8 Int;
    R8Srgb = R _8 Srgb;
    Rg8Unorm = Rg _8 Unorm;
    Rg8Inorm = Rg _8 Inorm;
    Rg8Uscaled = Rg _8 Uscaled;
    Rg8Iscaled = Rg _8 Iscaled;
    Rg8Uint = Rg _8 Uint;
    Rg8Int = Rg _8 Int;
    Rg8Srgb = Rg _8 Srgb;
    Rgb8Unorm = Rgb _8 Unorm;
    Rgb8Inorm = Rgb _8 Inorm;
    Rgb8Uscaled = Rgb _8 Uscaled;
    Rgb8Iscaled = Rgb _8 Iscaled;
    Rgb8Uint = Rgb _8 Uint;
    Rgb8Int = Rgb _8 Int;
    Rgb8Srgb = Rgb _8 Srgb;
    Bgr8Unorm = Bgr _8 Unorm;
    Bgr8Inorm = Bgr _8 Inorm;
    Bgr8Uscaled = Bgr _8 Uscaled;
    Bgr8Iscaled = Bgr _8 Iscaled;
    Bgr8Uint = Bgr _8 Uint;
    Bgr8Int = Bgr _8 Int;
    Bgr8Srgb = Bgr _8 Srgb;
    Rgba8Unorm = Rgba _8 Unorm;
    Rgba8Inorm = Rgba _8 Inorm;
    Rgba8Uscaled = Rgba _8 Uscaled;
    Rgba8Iscaled = Rgba _8 Iscaled;
    Rgba8Uint = Rgba _8 Uint;
    Rgba8Int = Rgba _8 Int;
    Rgba8Srgb = Rgba _8 Srgb;
    Bgra8Unorm = Bgra _8 Unorm;
    Bgra8Inorm = Bgra _8 Inorm;
    Bgra8Uscaled = Bgra _8 Uscaled;
    Bgra8Iscaled = Bgra _8 Iscaled;
    Bgra8Uint = Bgra _8 Uint;
    Bgra8Int = Bgra _8 Int;
    Bgra8Srgb = Bgra _8 Srgb;
    Abgr8Unorm = Abgr _8 Unorm;
    Abgr8Inorm = Abgr _8 Inorm;
    Abgr8Uscaled = Abgr _8 Uscaled;
    Abgr8Iscaled = Abgr _8 Iscaled;
    Abgr8Uint = Abgr _8 Uint;
    Abgr8Int = Abgr _8 Int;
    Abgr8Srgb = Abgr _8 Srgb;
    R16Unorm = R _16 Unorm;
    R16Inorm = R _16 Inorm;
    R16Uscaled = R _16 Uscaled;
    R16Iscaled = R _16 Iscaled;
    R16Uint = R _16 Uint;
    R16Int = R _16 Int;
    // R16Float = R _16 Float;
    Rg16Unorm = Rg _16 Unorm;
    Rg16Inorm = Rg _16 Inorm;
    Rg16Uscaled = Rg _16 Uscaled;
    Rg16Iscaled = Rg _16 Iscaled;
    Rg16Uint = Rg _16 Uint;
    Rg16Int = Rg _16 Int;
    // Rg16Float = Rg _16 Float;
    Rgb16Unorm = Rgb _16 Unorm;
    Rgb16Inorm = Rgb _16 Inorm;
    Rgb16Uscaled = Rgb _16 Uscaled;
    Rgb16Iscaled = Rgb _16 Iscaled;
    Rgb16Uint = Rgb _16 Uint;
    Rgb16Int = Rgb _16 Int;
    // Rgb16Float = Rgb _16 Float;
    Rgba16Unorm = Rgba _16 Unorm;
    Rgba16Inorm = Rgba _16 Inorm;
    Rgba16Uscaled = Rgba _16 Uscaled;
    Rgba16Iscaled = Rgba _16 Iscaled;
    Rgba16Uint = Rgba _16 Uint;
    Rgba16Int = Rgba _16 Int;
    // Rgba16Float = Rgba _16 Float;
    R32Uint = R _32 Uint;
    R32Int = R _32 Int;
    R32Float = R _32 Float;
    Rg32Uint = Rg _32 Uint;
    Rg32Int = Rg _32 Int;
    Rg32Float = Rg _32 Float;
    Rgb32Uint = Rgb _32 Uint;
    Rgb32Int = Rgb _32 Int;
    Rgb32Float = Rgb _32 Float;
    Rgba32Uint = Rgba _32 Uint;
    Rgba32Int = Rgba _32 Int;
    Rgba32Float = Rgba _32 Float;
    R64Uint = R _64 Uint;
    R64Int = R _64 Int;
    R64Float = R _64 Float;
    Rg64Uint = Rg _64 Uint;
    Rg64Int = Rg _64 Int;
    Rg64Float = Rg _64 Float;
    Rgb64Uint = Rgb _64 Uint;
    Rgb64Int = Rgb _64 Int;
    Rgb64Float = Rgb _64 Float;
    Rgba64Uint = Rgba _64 Uint;
    Rgba64Int = Rgba _64 Int;
    Rgba64Float = Rgba _64 Float;
}

#[cfg(feature = "palette")]
mod palette_pixel {
    use super::*;

    macro_rules! impl_from_palette {
        (@impl R, $palette:expr) => {{
            let (r,) = $palette.into_components();
            [r.convert()]
        }};
        (@impl Rg, $palette:expr) => {{
            let (r, g) = $palette.into_components();
            [r.convert(), g.convert()]
        }};
        (@impl Rgb, $palette:expr) => {{
            let (r, g, b) = $palette.into_components();
            [r.convert(), g.convert(), b.convert()]
        }};
        (@impl Rgba, $palette:expr) => {{
            let (r, g, b, a) = $palette.into_components();
            [r.convert(), g.convert(), b.convert(), a.convert()]
        }};
        ($($palette:ident => $channels:ident $repr:ident;)*) => {$(
            impl<S, T> From<palette::$palette<T>> for Pixel<$channels, S, $repr>
            where
                S: ChannelSize,
                $repr: ChannelRepr<S>,
                T: palette::Component,
                <$repr as ChannelRepr<S>>::Repr: palette::Component,
            {
                fn from(palette: palette::$palette<T>) -> Self {
                    Self {
                        repr: impl_from_palette!(@impl $channels, palette),
                    }
                }
            }
        )*};
    }

    impl_from_palette! {
        Srgb => Rgb Srgb;
        Srgba => Rgba Srgb;
        LinSrgb => Rgb Unorm;
        LinSrgba => Rgba Unorm;
        SrgbLuma => R Srgb;
        SrgbLumaa => Rg Srgb;
    }
}

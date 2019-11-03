//! Typed pixel formats.
//! More information on these can be found [here](https://vulkan.lunarg.com/doc/view/1.0.30.0/linux/vkspec.chunked/ch31s03.html#VkFormat)
//!

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

/// Unsigned scaled integer representation
#[derive(Clone, Copy, Debug, Default)]
pub struct Uscaled;

/// Signed scaled integer representation
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

/// Byte size of each channel in the image, such as Red, Green,
/// or other channels depending on the format.
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

/// Channel representation as a Rust type
pub trait ChannelRepr<S> {
    /// Newtype to reduce verbosity of representing a Channel in Rust
    type Repr: Sized + std::fmt::Debug + Default + Copy + Send + Sync + 'static;
}

/// Generates an impl for a Channel
macro_rules! impl_channel_repr {
    ($($type:ident * $size:ident = $repr:ident;)*) => {
        $(
            impl ChannelRepr<$size> for $type { type Repr = $repr; }
        )*
    };
}

// Actually generates the impl for the below types
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

/// Red channel.
#[derive(Clone, Copy, Debug, Default)]
pub struct R;

/// Red-green channels.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rg;

/// Red-green-blue channels.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rgb;

/// Red-green-blue-alpha channels.
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

/// Pixel representation as a Rust type
pub trait PixelRepr<S, T> {
    /// Newtype to reduce verbosity of representing a Pixel in Rust
    type Repr: Sized + std::fmt::Debug + Default + Copy + Send + Sync + 'static;
}

/// Returns the number of channels for common RGBA combinations
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

/// Generates the Pixel impl for various Channels
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

// Actually use the macro to generate the implementations
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
#[repr(transparent)]
pub struct Pixel<C, S, T>
where
    C: PixelRepr<S, T>,
{
    /// Pixel representation.
    pub repr: <C as PixelRepr<S, T>>::Repr,
}

impl<C, S, T> Copy for Pixel<C, S, T> where C: PixelRepr<S, T> {}

impl<C, S, T> Clone for Pixel<C, S, T>
where
    C: PixelRepr<S, T>,
{
    fn clone(&self) -> Self {
        Pixel {
            repr: self.repr.clone(),
        }
    }
}

impl<C, S, T> std::fmt::Debug for Pixel<C, S, T>
where
    C: PixelRepr<S, T>,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Pixel").field("repr", &self.repr).finish()
    }
}

impl<C, S, T> Default for Pixel<C, S, T>
where
    C: PixelRepr<S, T>,
{
    fn default() -> Self {
        Pixel {
            repr: C::Repr::default(),
        }
    }
}

/// AsPixel trait for extracting the underlying data representation information from a Rust data type
/// # Example
/// ```rust,no-run
/// struct Rgba([u8; 4]);
/// ```
pub trait AsPixel: Copy + std::fmt::Debug + Default + Send + Sync + 'static {
    /// Name of the pixel type.
    const NAME: &'static str;

    /// Size of the pixel.
    const SIZE: u32;

    /// Pixel format.
    const FORMAT: rendy_core::hal::format::Format;
}

macro_rules! impl_pixel {
    ($($alias:ident = $channels:ident $size:ident $type:ident;)*) => {
        $(
            /// Pixel type alias.
            pub type $alias = Pixel<$channels, $size, $type>;

            impl AsPixel for $alias {
                const NAME: &'static str = stringify!($alias);
                const SIZE: u32 = num_channels!($channels) * <$size as ChannelSize>::SIZE;
                const FORMAT: rendy_core::hal::format::Format = rendy_core::hal::format::Format::$alias;
            }
        )*
    };
}

// Actually implement AsPixel for all the formats
// TODO: Implement AsPixel for the Float; they are commented out until then
impl_pixel! {
    R8Unorm = R _8 Unorm;
    R8Snorm = R _8 Inorm;
    R8Uscaled = R _8 Uscaled;
    R8Sscaled = R _8 Iscaled;
    R8Uint = R _8 Uint;
    R8Sint = R _8 Int;
    R8Srgb = R _8 Srgb;
    Rg8Unorm = Rg _8 Unorm;
    Rg8Snorm = Rg _8 Inorm;
    Rg8Uscaled = Rg _8 Uscaled;
    Rg8Sscaled = Rg _8 Iscaled;
    Rg8Uint = Rg _8 Uint;
    Rg8Sint = Rg _8 Int;
    Rg8Srgb = Rg _8 Srgb;
    Rgb8Unorm = Rgb _8 Unorm;
    Rgb8Snorm = Rgb _8 Inorm;
    Rgb8Uscaled = Rgb _8 Uscaled;
    Rgb8Sscaled = Rgb _8 Iscaled;
    Rgb8Uint = Rgb _8 Uint;
    Rgb8Sint = Rgb _8 Int;
    Rgb8Srgb = Rgb _8 Srgb;
    Bgr8Unorm = Bgr _8 Unorm;
    Bgr8Snorm = Bgr _8 Inorm;
    Bgr8Uscaled = Bgr _8 Uscaled;
    Bgr8Sscaled = Bgr _8 Iscaled;
    Bgr8Uint = Bgr _8 Uint;
    Bgr8Sint = Bgr _8 Int;
    Bgr8Srgb = Bgr _8 Srgb;
    Rgba8Unorm = Rgba _8 Unorm;
    Rgba8Snorm = Rgba _8 Inorm;
    Rgba8Uscaled = Rgba _8 Uscaled;
    Rgba8Sscaled = Rgba _8 Iscaled;
    Rgba8Uint = Rgba _8 Uint;
    Rgba8Sint = Rgba _8 Int;
    Rgba8Srgb = Rgba _8 Srgb;
    Bgra8Unorm = Bgra _8 Unorm;
    Bgra8Snorm = Bgra _8 Inorm;
    Bgra8Uscaled = Bgra _8 Uscaled;
    Bgra8Sscaled = Bgra _8 Iscaled;
    Bgra8Uint = Bgra _8 Uint;
    Bgra8Sint = Bgra _8 Int;
    Bgra8Srgb = Bgra _8 Srgb;
    Abgr8Unorm = Abgr _8 Unorm;
    Abgr8Snorm = Abgr _8 Inorm;
    Abgr8Uscaled = Abgr _8 Uscaled;
    Abgr8Sscaled = Abgr _8 Iscaled;
    Abgr8Uint = Abgr _8 Uint;
    Abgr8Sint = Abgr _8 Int;
    Abgr8Srgb = Abgr _8 Srgb;
    R16Unorm = R _16 Unorm;
    R16Snorm = R _16 Inorm;
    R16Uscaled = R _16 Uscaled;
    R16Sscaled = R _16 Iscaled;
    R16Uint = R _16 Uint;
    R16Sint = R _16 Int;
    // R16Sfloat = R _16 Float;
    Rg16Unorm = Rg _16 Unorm;
    Rg16Snorm = Rg _16 Inorm;
    Rg16Uscaled = Rg _16 Uscaled;
    Rg16Sscaled = Rg _16 Iscaled;
    Rg16Uint = Rg _16 Uint;
    Rg16Sint = Rg _16 Int;
    // Rg16Sfloat = Rg _16 Float;
    Rgb16Unorm = Rgb _16 Unorm;
    Rgb16Snorm = Rgb _16 Inorm;
    Rgb16Uscaled = Rgb _16 Uscaled;
    Rgb16Sscaled = Rgb _16 Iscaled;
    Rgb16Uint = Rgb _16 Uint;
    Rgb16Sint = Rgb _16 Int;
    // Rgb16Sfloat = Rgb _16 Float;
    Rgba16Unorm = Rgba _16 Unorm;
    Rgba16Snorm = Rgba _16 Inorm;
    Rgba16Uscaled = Rgba _16 Uscaled;
    Rgba16Sscaled = Rgba _16 Iscaled;
    Rgba16Uint = Rgba _16 Uint;
    Rgba16Sint = Rgba _16 Int;
    // Rgba16Sfloat = Rgba _16 Float;
    R32Uint = R _32 Uint;
    R32Sint = R _32 Int;
    R32Sfloat = R _32 Float;
    Rg32Uint = Rg _32 Uint;
    Rg32Sint = Rg _32 Int;
    Rg32Sfloat = Rg _32 Float;
    Rgb32Uint = Rgb _32 Uint;
    Rgb32Sint = Rgb _32 Int;
    Rgb32Sfloat = Rgb _32 Float;
    Rgba32Uint = Rgba _32 Uint;
    Rgba32Sint = Rgba _32 Int;
    Rgba32Sfloat = Rgba _32 Float;
    R64Uint = R _64 Uint;
    R64Sint = R _64 Int;
    R64Sfloat = R _64 Float;
    Rg64Uint = Rg _64 Uint;
    Rg64Sint = Rg _64 Int;
    Rg64Sfloat = Rg _64 Float;
    Rgb64Uint = Rgb _64 Uint;
    Rgb64Sint = Rgb _64 Int;
    Rgb64Sfloat = Rgb _64 Float;
    Rgba64Uint = Rgba _64 Uint;
    Rgba64Sint = Rgba _64 Int;
    Rgba64Sfloat = Rgba _64 Float;
}

#[cfg(feature = "palette")]
mod palette_pixel {
    //! A palette_pixel represents is a type that represents a single color value
    //! in a color space.
    //!
    use palette::{
        encoding,
        luma::{Luma, LumaStandard, Lumaa},
        rgb::{Rgb, RgbStandard, Rgba},
        white_point::D65,
        Component,
    };

    macro_rules! impl_from_palette {
        (# $color:ident R as $encoding:path) => {
            {
                let f = $color.into_format();
                let _: (f32,) = f.into_components();
                let (r,) = f.into_encoding::<$encoding>().into_format().into_components();
                Self { repr: [r] }
            }
        };
        (# $color:ident Rg as $encoding:path) => {
            {
                let f = $color.into_format();
                let _: (f32,f32) = f.into_components();
                let (r,g) = f.into_encoding::<$encoding>().into_format().into_components();
                Self { repr: [r,g] }
            }
        };
        (# $color:ident Rgb as $encoding:path) => {
            {
                let f = $color.into_format();
                let _: (f32,f32,f32) = f.into_components();
                let (r,g,b) = f.into_encoding::<$encoding>().into_format().into_components();
                Self { repr: [r,g,b] }
            }
        };
        (# $color:ident Rgba as $encoding:path) => {
            {
                let f = $color.into_format();
                let _: (f32,f32,f32,f32) = f.into_components();
                let (r,g,b,a) = f.into_encoding::<$encoding>().into_format().into_components();
                Self { repr: [r,g,b,a] }
            }
        };

        ($($container:path as $encoding:path : $standard:path => $channels:ident $($repr:ident)|+),* $(,)*) => {$($(
            impl<S, T, B> From<$container> for super::Pixel<super::$channels, B, super::$repr>
            where
                S: $standard,
                T: Component,
                B: super::ChannelSize,
                super::$repr: super::ChannelRepr<B>,
                <super::$repr as super::ChannelRepr<B>>::Repr: Component,
            {
                fn from(color: $container) -> Self {
                    impl_from_palette!(# color $channels as $encoding)
                }
            }
        )+)*};
    }

    impl_from_palette! {
        Rgb<S, T> as encoding::Srgb: RgbStandard<Space = encoding::Srgb> => Rgb Srgb,
        Rgba<S, T> as encoding::Srgb: RgbStandard<Space = encoding::Srgb> => Rgba Srgb,
        Luma<S, T> as encoding::Srgb: LumaStandard<WhitePoint = D65> => R Srgb,
        Lumaa<S, T> as encoding::Srgb: LumaStandard<WhitePoint = D65> => Rg Srgb,

        Rgb<S, T> as encoding::Linear<encoding::Srgb>: RgbStandard<Space = encoding::Srgb> => Rgb Unorm | Float,
        Rgba<S, T> as encoding::Linear<encoding::Srgb>: RgbStandard<Space = encoding::Srgb> => Rgba Unorm | Float,

        Luma<S, T> as encoding::Linear<D65>: LumaStandard<WhitePoint = D65> => R Unorm | Float,
        Lumaa<S, T> as encoding::Linear<D65>: LumaStandard<WhitePoint = D65> => Rg Unorm | Float,
    }
}

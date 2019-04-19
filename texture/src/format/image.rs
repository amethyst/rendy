//! Module that turns an image into a `Texture`

use crate::{pixel, TextureBuilder};
use derivative::Derivative;

// reexport for easy usage in ImageTextureConfig
pub use image::ImageFormat;

#[derive(Derivative, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derivative(Default)]
pub enum Repr {
    Unorm,
    Inorm,
    Uscaled,
    Iscaled,
    Uint,
    Int,
    #[derivative(Default)]
    Srgb,
}

/// Stores details about how the data is laid out
struct DataLayout {
    /// distance between lines in texels
    pub width: u32,
    /// distance between layers/planes in texel lines
    pub height: u32,
}

#[derive(Derivative, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derivative(Default)]
/// Enumerates the kinds of `Texture`s
pub enum TextureKind {
    D1,
    D1Array,
    #[derivative(Default)]
    D2 {
        #[derivative(Default(value = "1"))]
        samples: u8,
    },
    D2Array {
        samples: u8,
        layers: u16,
    },
    D3 {
        depth: u32,
    },
    Cube,
    CubeArray {
        layers: u16,
    },
}

impl TextureKind {
    fn layout_and_kind(&self, width: u32, height: u32) -> (gfx_hal::image::Kind, DataLayout) {
        use gfx_hal::image::Kind::*;
        match self {
            TextureKind::D1 => (
                D1(width * height, 1),
                DataLayout { width: width * height, height: 1 }
            ),
            TextureKind::D1Array => (
                D1(width, height as u16),
                DataLayout { width, height: 1 }
            ),
            TextureKind::D2 { samples } => (
                D2(width, height, 1, *samples),
                DataLayout { width, height }
            ),
            TextureKind::D2Array {
                samples,
                layers,
            } => (
                D2(
                    width,
                    height / *layers as u32,
                    *layers,
                    *samples,
                ),
                DataLayout { width, height: height / *layers as u32 }
            ),
            TextureKind::D3 { depth } => (
                D3(
                    width,
                    height / *depth,
                    *depth,
                ),
                DataLayout { width, height: height / *depth }
            ),
            TextureKind::Cube => (
                D2(
                    width,
                    height / 6,
                    6,
                    1,
                ),
                DataLayout { width, height: height / 6 }
            ),
            TextureKind::CubeArray { layers } => (
                D2(
                    width,
                    height / (*layers as u32 * 6),
                    layers * 6,
                    1,
                ),
                DataLayout { width, height: height / (*layers as u32 * 6) }
            ),
        }
    }

    fn view_kind(&self) -> gfx_hal::image::ViewKind {
        use gfx_hal::image::ViewKind;
        match self {
            TextureKind::D1 => ViewKind::D1,
            TextureKind::D1Array { .. } => ViewKind::D1Array,
            TextureKind::D2 { .. } => ViewKind::D2,
            TextureKind::D2Array { .. } => ViewKind::D2Array,
            TextureKind::D3 { .. } => ViewKind::D3,
            TextureKind::Cube { .. } => ViewKind::Cube,
            TextureKind::CubeArray { .. } => ViewKind::CubeArray,
        }
    }
}

#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derivative(Default)]
pub struct ImageTextureConfig {
    /// Interpret the image as given format.
    /// When `None`, format is determined automatically based on magic bytes.
    /// Automatic method doesn't support TGA format.
    #[cfg_attr(feature = "serde", serde(with = "serde_image_format"))]
    pub format: Option<ImageFormat>,
    pub repr: Repr,
    pub kind: TextureKind,
    #[derivative(Default(
        value = "gfx_hal::image::SamplerInfo::new(gfx_hal::image::Filter::Linear, gfx_hal::image::WrapMode::Clamp)"
    ))]
    pub sampler_info: gfx_hal::image::SamplerInfo,
}

#[cfg(feature = "serde")]
mod serde_image_format {
    //! Module for enabline serde to serialize and deserialize image formats
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    #[serde(remote = "image::ImageFormat")]
    enum SerdeImageFormat {
        PNG,
        JPEG,
        GIF,
        WEBP,
        PNM,
        TIFF,
        TGA,
        BMP,
        ICO,
        HDR,
    }

    #[derive(Serialize, Deserialize)]
    struct Helper(#[serde(with = "SerdeImageFormat")] image::ImageFormat);

    pub fn serialize<S: Serializer>(
        value: &Option<image::ImageFormat>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.map(Helper).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<image::ImageFormat>, D::Error> {
        Ok(Option::deserialize(deserializer)?.map(|Helper(format)| format))
    }
}

macro_rules! dyn_format {
    ($channel:ident, $size:ident, $repr:ident) => {
        <pixel::Pixel<pixel::$channel, pixel::$size, pixel::$repr> as pixel::AsPixel>::FORMAT
    };
    ($channel:ident, $size:ident, $repr:expr) => {{
        match $repr {
            Repr::Unorm => dyn_format!($channel, $size, Unorm),
            Repr::Inorm => dyn_format!($channel, $size, Inorm),
            Repr::Uscaled => dyn_format!($channel, $size, Uscaled),
            Repr::Iscaled => dyn_format!($channel, $size, Iscaled),
            Repr::Uint => dyn_format!($channel, $size, Uint),
            Repr::Int => dyn_format!($channel, $size, Int),
            Repr::Srgb => dyn_format!($channel, $size, Srgb),
        }
    }};
}

/// Attempts to load a Texture from an image.
pub fn load_from_image(
    bytes: &[u8],
    config: ImageTextureConfig,
) -> Result<TextureBuilder<'static>, failure::Error> {
    use gfx_hal::format::{Component, Swizzle};
    use image::{DynamicImage, GenericImageView};

    let image_format = config
        .format
        .map_or_else(|| image::guess_format(bytes), |f| Ok(f))?;
    let image = image::load_from_memory_with_format(bytes, image_format)?;

    let (w, h) = image.dimensions();
    let (kind, layout) = config.kind.layout_and_kind(w, h);

    let (vec, format, swizzle) = match image {
        DynamicImage::ImageLuma8(img) => (
            img.into_vec(),
            dyn_format!(R, _8, config.repr),
            Swizzle(Component::R, Component::R, Component::R, Component::One),
        ),
        DynamicImage::ImageLumaA8(img) => (
            img.into_vec(),
            dyn_format!(Rg, _8, config.repr),
            Swizzle(Component::R, Component::R, Component::R, Component::G),
        ),
        DynamicImage::ImageRgb8(img) => (
            img.into_vec(),
            dyn_format!(Rgb, _8, config.repr),
            Swizzle::NO,
        ),
        DynamicImage::ImageRgba8(img) => (
            img.into_vec(),
            dyn_format!(Rgba, _8, config.repr),
            Swizzle::NO,
        ),
        DynamicImage::ImageBgr8(img) => (
            img.into_vec(),
            dyn_format!(Bgr, _8, config.repr),
            Swizzle::NO,
        ),
        DynamicImage::ImageBgra8(img) => (
            img.into_vec(),
            dyn_format!(Bgra, _8, config.repr),
            Swizzle::NO,
        ),
    };

    Ok(TextureBuilder::new()
        .with_raw_data(vec, format)
        .with_swizzle(swizzle)
        .with_data_width(layout.width)
        .with_data_height(layout.height)
        .with_kind(kind)
        .with_view_kind(config.kind.view_kind())
        .with_sampler_info(config.sampler_info))
}

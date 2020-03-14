//! Module that turns an image into a `Texture`

use crate::{pixel, MipLevels, TextureBuilder};

use std::convert::TryFrom;
use std::num::NonZeroU8;

// reexport for easy usage in ImageTextureConfig
pub use image::ImageFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Repr {
    Float,
    Unorm,
    Inorm,
    Uscaled,
    Iscaled,
    Uint,
    Int,
    Srgb,
}

impl Default for Repr {
    fn default() -> Self {
        Repr::Srgb
    }
}

/// A description how to interpret loaded texture.
/// Defines the dimensionality and layer count of textures to load.
///
/// When loading more than one layer, the loaded image is vertically
/// divided into mutiple subimages. The layer width is preserved and
/// it's height is a fraction of image's original height.
///
/// 1D arrays are treated as a sequence of rows, each being an array entry.
/// 1D images are treated as a single sequence of pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TextureKind {
    D1,
    D1Array,
    D2,
    D2Array { layers: u16 },
    D3 { depth: u32 },
    Cube,
    CubeArray { layers: u16 },
}

impl Default for TextureKind {
    fn default() -> Self {
        TextureKind::D2
    }
}

impl TextureKind {
    fn gfx_kind(&self, width: u32, height: u32) -> rendy_core::hal::image::Kind {
        use rendy_core::hal::image::Kind::*;
        match self {
            TextureKind::D1 => D1(width * height, 1),
            TextureKind::D1Array => D1(width, height as u16),
            TextureKind::D2 => D2(width, height, 1, 1),
            TextureKind::D2Array { layers } => D2(width, height / *layers as u32, *layers, 1),
            TextureKind::D3 { depth } => D3(width, height / *depth, *depth),
            TextureKind::Cube => D2(width, height / 6, 6, 1),
            TextureKind::CubeArray { layers } => {
                D2(width, height / (*layers as u32 * 6), layers * 6, 1)
            }
        }
    }

    fn view_kind(&self) -> rendy_core::hal::image::ViewKind {
        use rendy_core::hal::image::ViewKind;
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct ImageTextureConfig {
    /// Interpret the image as given format.
    /// When `None`, format is determined automatically based on magic bytes.
    /// Automatic method doesn't support TGA format.
    #[cfg_attr(feature = "serde", serde(with = "serde_image_format"))]
    pub format: Option<ImageFormat>,
    pub repr: Repr,
    pub kind: TextureKind,
    pub sampler_info: rendy_core::hal::image::SamplerDesc,
    /// Automatically generate mipmaps for this image
    pub generate_mips: bool,
    /// Premultiply the alpha channel of the image, if there is one. Note that this
    /// means an image stored with non-premultiplied alpha will become premultiplied,
    /// rather than indicating that the supplied image is premultiplied to begin with.
    pub premultiply_alpha: bool,
}

impl Default for ImageTextureConfig {
    fn default() -> Self {
        ImageTextureConfig {
            format: None,
            repr: Repr::default(),
            kind: TextureKind::default(),
            sampler_info: rendy_core::hal::image::SamplerDesc::new(
                rendy_core::hal::image::Filter::Linear,
                rendy_core::hal::image::WrapMode::Clamp,
            ),
            generate_mips: false,
            premultiply_alpha: false,
        }
    }
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
            Repr::Float => unimplemented!(),
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

/// pixel.channel_count() must be 4
fn premultiply_alpha_4channel<P: image::Pixel<Subpixel = u8>>(pixel: &mut P) {
    let channels_mut = pixel.channels_mut();
    let alpha = channels_mut[3] as f32 / 255.0;
    channels_mut[0] = (channels_mut[0] as f32 * alpha).min(255.0).max(0.0) as u8;
    channels_mut[1] = (channels_mut[1] as f32 * alpha).min(255.0).max(0.0) as u8;
    channels_mut[2] = (channels_mut[2] as f32 * alpha).min(255.0).max(0.0) as u8;
}

/// pixel.channel_count() must be 2
fn premultiply_alpha_2channel<P: image::Pixel<Subpixel = u8>>(pixel: &mut P) {
    let channels_mut = pixel.channels_mut();
    let alpha = channels_mut[1] as f32 / 255.0;
    channels_mut[0] = (channels_mut[0] as f32 * alpha).min(255.0).max(0.0) as u8;
}

/// pixel.channel_count() must be 2
fn premultiply_alpha_2channel_u16<P: image::Pixel<Subpixel = u16>>(pixel: &mut P) {
    let channels_mut = pixel.channels_mut();
    let alpha = channels_mut[1] as f32 / 255.0;
    channels_mut[0] = (channels_mut[0] as f32 * alpha).min(255.0).max(0.0) as u16;
}

fn vec_16_to_vec_8(vec_u16: Vec<u16>) -> Vec<u8> {
    vec_u16
        .into_iter()
        .flat_map(|value: u16| {
            let high: u8 =
                TryFrom::<u16>::try_from(value & 0x00ffu16).expect("Unreachable: within u8 range");
            let low: u8 = TryFrom::<u16>::try_from((value >> 8) & 0x00ffu16)
                .expect("Unreachable: within u8 range");
            std::iter::once(low).chain(std::iter::once(high))
        })
        .collect::<Vec<u8>>()
}

/// Attempts to load a Texture from an image.
pub fn load_from_image<R>(
    mut reader: R,
    config: ImageTextureConfig,
) -> Result<TextureBuilder<'static>, image::ImageError>
where
    R: std::io::BufRead + std::io::Seek,
{
    use image::{DynamicImage, GenericImageView};
    use rendy_core::hal::format::{Component, Swizzle};

    let image_format = config.format.map_or_else(
        || {
            let r = reader.by_ref();
            // Longest size of image crate supported magic bytes
            let mut format_magic_bytes = [0u8; 10];
            r.read_exact(&mut format_magic_bytes)?;
            r.seek(std::io::SeekFrom::Current(-10))?;
            image::guess_format(&format_magic_bytes)
        },
        |f| Ok(f),
    )?;

    let (w, h, vec, format, swizzle) = match (image_format, config.repr) {
        (image::ImageFormat::Hdr, Repr::Float) => {
            let decoder = image::hdr::HdrDecoder::new(reader)?;
            let metadata = decoder.metadata();
            let (w, h) = (metadata.width, metadata.height);

            let format = rendy_core::hal::format::Format::Rgb32Sfloat;
            let vec = crate::core::cast_vec(decoder.read_image_hdr()?);
            let swizzle = Swizzle::NO;
            (w, h, vec, format, swizzle)
        }
        _ => {
            let image = image::load(reader, image_format)?;

            let (w, h) = image.dimensions();

            let (vec, format, swizzle) = match image {
                DynamicImage::ImageLuma8(img) => (
                    img.into_vec(),
                    dyn_format!(R, _8, config.repr),
                    Swizzle(Component::R, Component::R, Component::R, Component::One),
                ),
                DynamicImage::ImageLuma16(img) => {
                    let vec_u16 = img.into_vec();
                    let vec_u8 = vec_16_to_vec_8(vec_u16);
                    (
                        vec_u8,
                        dyn_format!(R, _8, config.repr),
                        Swizzle(Component::R, Component::R, Component::R, Component::G),
                    )
                }
                DynamicImage::ImageLumaA8(mut img) => {
                    if config.premultiply_alpha {
                        for pixel in img.pixels_mut() {
                            premultiply_alpha_2channel(pixel);
                        }
                    }
                    (
                        img.into_vec(),
                        dyn_format!(Rg, _8, config.repr),
                        Swizzle(Component::R, Component::R, Component::R, Component::G),
                    )
                }
                DynamicImage::ImageLumaA16(mut img) => {
                    if config.premultiply_alpha {
                        for pixel in img.pixels_mut() {
                            premultiply_alpha_2channel_u16(pixel);
                        }
                    }
                    let vec_u16 = img.into_vec();
                    let vec_u8 = vec_16_to_vec_8(vec_u16);
                    (
                        vec_u8,
                        dyn_format!(Rg, _8, config.repr),
                        Swizzle(Component::R, Component::R, Component::R, Component::G),
                    )
                }
                DynamicImage::ImageRgb8(img) => (
                    img.into_vec(),
                    dyn_format!(Rgb, _8, config.repr),
                    Swizzle::NO,
                ),
                DynamicImage::ImageRgb16(img) => (
                    vec_16_to_vec_8(img.into_vec()),
                    dyn_format!(Rgb, _8, config.repr),
                    Swizzle::NO,
                ),
                DynamicImage::ImageRgba16(mut img) => {
                    if config.premultiply_alpha {
                        for pixel in img.pixels_mut() {
                            premultiply_alpha_2channel_u16(pixel);
                        }
                    }
                    let vec_u16 = img.into_vec();
                    let vec_u8 = vec_16_to_vec_8(vec_u16);
                    (
                        vec_u8,
                        dyn_format!(Rg, _8, config.repr),
                        Swizzle(Component::R, Component::R, Component::R, Component::G),
                    )
                }
                DynamicImage::ImageRgba8(mut img) => {
                    if config.premultiply_alpha {
                        for pixel in img.pixels_mut() {
                            premultiply_alpha_4channel(pixel);
                        }
                    }
                    (
                        img.into_vec(),
                        dyn_format!(Rgba, _8, config.repr),
                        Swizzle::NO,
                    )
                }
                DynamicImage::ImageBgr8(img) => (
                    img.into_vec(),
                    dyn_format!(Bgr, _8, config.repr),
                    Swizzle::NO,
                ),
                DynamicImage::ImageBgra8(mut img) => {
                    if config.premultiply_alpha {
                        for pixel in img.pixels_mut() {
                            premultiply_alpha_4channel(pixel);
                        }
                    }
                    (
                        img.into_vec(),
                        dyn_format!(Bgra, _8, config.repr),
                        Swizzle::NO,
                    )
                }
            };
            (w, h, vec, format, swizzle)
        }
    };

    let kind = config.kind.gfx_kind(w, h);
    let extent = kind.extent();

    let mips = if config.generate_mips {
        MipLevels::GenerateAuto
    } else {
        MipLevels::Levels(NonZeroU8::new(1).unwrap())
    };

    Ok(TextureBuilder::new()
        .with_raw_data(vec, format)
        .with_swizzle(swizzle)
        .with_data_width(extent.width)
        .with_data_height(extent.height)
        .with_mip_levels(mips)
        .with_kind(kind)
        .with_premultiplied_alpha(config.premultiply_alpha)
        .with_view_kind(config.kind.view_kind())
        .with_sampler_info(config.sampler_info))
}

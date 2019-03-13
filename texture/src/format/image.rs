use crate::{pixel, TextureBuilder};
use derivative::Derivative;

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
    Float,
}

/// Determines the way layers are being stored in source image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LayerLayout {
    Row,
    Column,
}

struct DataLayout {
    /// distance between lines in texels
    pub line_stride: u32,
    /// distance between layers/planes in texels
    pub layer_stride: u32,
}

impl LayerLayout {
    fn layer_width(&self, image_width: u32, layers: u32) -> u32 {
        match self {
            LayerLayout::Row => image_width / layers,
            LayerLayout::Column => image_width,
        }
    }

    fn layer_height(&self, image_height: u32, layers: u32) -> u32 {
        match self {
            LayerLayout::Row => image_height,
            LayerLayout::Column => image_height / layers,
        }
    }

    fn data_layout(&self, image_width: u32, image_height: u32, layers: u32) -> DataLayout {
        match self {
            LayerLayout::Row => DataLayout {
                line_stride: image_width,
                layer_stride: image_width / layers,
            },
            LayerLayout::Column => DataLayout {
                line_stride: image_width,
                layer_stride: image_width * image_height / layers,
            },
        }
    }
}

#[derive(Derivative, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derivative(Default)]
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
        layout: LayerLayout,
    },
    D3 {
        depth: u32,
        layout: LayerLayout,
    },
    Cube {
        layout: LayerLayout,
    },
    CubeArray {
        layers: u16,
        layout: LayerLayout,
    },
}

impl TextureKind {
    fn layout_and_kind(&self, width: u32, height: u32) -> (gfx_hal::image::Kind, DataLayout) {
        use gfx_hal::image::Kind::*;
        match self {
            TextureKind::D1 => (
                D1(width * height, 1),
                LayerLayout::Column.data_layout(width * height, 1, 1),
            ),
            TextureKind::D1Array => (
                D1(width, height as u16),
                LayerLayout::Column.data_layout(width, 1, height),
            ),
            TextureKind::D2 { samples } => (
                D2(width, height, 1, *samples),
                LayerLayout::Column.data_layout(width, height, 1),
            ),
            TextureKind::D2Array {
                samples,
                layers,
                layout,
            } => (
                D2(
                    layout.layer_width(width, *layers as u32),
                    layout.layer_height(height, *layers as u32),
                    *layers,
                    *samples,
                ),
                layout.data_layout(width, height, *layers as u32),
            ),
            TextureKind::D3 { depth, layout } => (
                D3(
                    layout.layer_width(width, *depth),
                    layout.layer_height(height, *depth),
                    *depth,
                ),
                layout.data_layout(width, height, *depth),
            ),
            TextureKind::Cube { layout } => (
                D2(
                    layout.layer_width(width, 6),
                    layout.layer_height(height, 6),
                    6,
                    1,
                ),
                layout.data_layout(width, height, 6),
            ),
            TextureKind::CubeArray { layers, layout } => (
                D2(
                    layout.layer_width(width, *layers as u32 * 6),
                    layout.layer_height(height, *layers as u32 * 6),
                    layers * 6,
                    1,
                ),
                layout.data_layout(width, height, *layers as u32 * 6),
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derivative(Default)]
pub struct ImageTextureConfig {
    /// Interpret the image as given format.
    /// When `None`, format is determined automatically based on magic bytes.
    /// Automatic method doesn't support TGA format.
    #[cfg_attr(feature = "serde", serde(with = "serde_image_format"))]
    pub format: Option<image::ImageFormat>,
    /// The representation that will be used for the image on the GPU.
    /// When `None`, a default is chosen based on the image format.
    pub repr: Option<Repr>,
    pub kind: TextureKind,
    #[derivative(Default(value = "gfx_hal::image::Filter::Linear"))]
    pub filter: gfx_hal::image::Filter,
}

#[cfg(feature = "serde")]
mod serde_image_format {
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
    ($channel:ident, $size:ident, dyn $repr:expr) => {{
        match $repr {
            Repr::Float => unreachable!(),
            Repr::Unorm => dyn_format!($channel, $size, Unorm),
            Repr::Inorm => dyn_format!($channel, $size, Inorm),
            Repr::Uscaled => dyn_format!($channel, $size, Uscaled),
            Repr::Iscaled => dyn_format!($channel, $size, Iscaled),
            Repr::Uint => dyn_format!($channel, $size, Uint),
            Repr::Int => dyn_format!($channel, $size, Int),
            Repr::Srgb => dyn_format!($channel, $size, Srgb),
        }
    }};
    ($channel:ident, $size:ident, $repr:ident) => {
        <pixel::Pixel<pixel::$channel, pixel::$size, pixel::$repr> as pixel::AsPixel>::FORMAT
    };
}

pub fn load_from_image<R, B, U> (
    mut reader: R,
    config: ImageTextureConfig,
    usage: U,
    physical: &dyn gfx_hal::PhysicalDevice<B>,
) -> Result<TextureBuilder<'static>, failure::Error>
where
    R: std::io::BufRead + std::io::Seek,
    B: gfx_hal::Backend,
    U: crate::resource::image::Usage
{
    use gfx_hal::format::{Component, Swizzle};
    use image::{DynamicImage, GenericImageView};

    let image_format = if let Some(f) = config.format {
        f
    } else {
        // max length of image crate supported magic bytes
        let mut format_buf = [0u8; 10]; 
        reader.read_exact(&mut format_buf)?;
        let format = image::guess_format(&format_buf);
        reader.seek(std::io::SeekFrom::Current(-10))?;
        format?
    };

    let (w, h, vec, format, swizzle) = match (image_format, config.repr) {
        (image::ImageFormat::HDR, None) | (image::ImageFormat::HDR, Some(Repr::Float)) => {
            let decoder = image::hdr::HDRDecoder::new(reader)?;
            let metadata = decoder.metadata();
            let (w, h) = (metadata.width, metadata.height);

            let vec = decoder.read_image_hdr()?;

            let format = dyn_format!(Rgb, _32, Float);
            let properties = physical.format_properties(Some(format));
            let (vec, format) = if properties.optimal_tiling.contains(usage.features()) {
                (crate::util::cast_vec(vec), format)
            } else {
                let format = dyn_format!(Rgba, _32, Float);
                let properties = physical.format_properties(Some(format));
                if !properties.optimal_tiling.contains(usage.features()) {
                    failure::bail!("Physical device does not support required usage for image's format")
                }
                let vec = vec
                    .into_iter()
                    .map(|rgb| {
                        image::Rgba {
                            data: [rgb.data[0], rgb.data[1], rgb.data[2], 0.0]
                        }
                    })
                    .collect::<Vec<_>>();
                (crate::util::cast_vec(vec), format)
            };
            let swizzle = Swizzle::NO;
            (w, h, vec, format, swizzle)
        },
        (_, Some(Repr::Float)) => {
            failure::bail!("Attempting to load non-HDR format with Float repr")
        }
        _ => {
            let image = image::load(reader, image_format)?;

            let (w, h) = image.dimensions();

            let (vec, format, swizzle) = match image {
                DynamicImage::ImageLuma8(img) => (
                    img.into_vec(),
                    dyn_format!(R, _8, dyn config.repr.unwrap_or_default()),
                    Swizzle(Component::R, Component::R, Component::R, Component::One),
                ),
                DynamicImage::ImageLumaA8(img) => (
                    img.into_vec(),
                    dyn_format!(Rg, _8, dyn config.repr.unwrap_or_default()),
                    Swizzle(Component::R, Component::R, Component::R, Component::G),
                ),
                DynamicImage::ImageRgb8(img) => (
                    img.into_vec(),
                    dyn_format!(Rgb, _8, dyn config.repr.unwrap_or_default()),
                    Swizzle::NO,
                ),
                DynamicImage::ImageRgba8(img) => (
                    img.into_vec(),
                    dyn_format!(Rgba, _8, dyn config.repr.unwrap_or_default()),
                    Swizzle::NO,
                ),
                DynamicImage::ImageBgr8(img) => (
                    img.into_vec(),
                    dyn_format!(Bgr, _8, dyn config.repr.unwrap_or_default()),
                    Swizzle::NO,
                ),
                DynamicImage::ImageBgra8(img) => (
                    img.into_vec(),
                    dyn_format!(Bgra, _8, dyn config.repr.unwrap_or_default()),
                    Swizzle::NO,
                ),
            };
            (w, h, vec, format, swizzle)
        }
    };

    let (kind, layout) = config.kind.layout_and_kind(w, h);

    Ok(TextureBuilder::new()
        .with_raw_data(vec, format)
        .with_swizzle(swizzle)
        .with_data_width(layout.line_stride)
        .with_data_height(layout.layer_stride)
        .with_kind(kind)
        .with_view_kind(config.kind.view_kind())
        .with_filter(config.filter))
}

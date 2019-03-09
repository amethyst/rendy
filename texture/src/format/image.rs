use crate::{pixel, TextureBuilder};
use derivative::Derivative;
pub use image::ImageFormat;

#[derive(Derivative)]
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

/// Determines the way layers are being stored in source image.
pub enum LayerLayout {
    Row,
    Column,
}

impl LayerLayout {
    pub fn layer_width(&self, image_width: u32, layers: u32) -> u32 {
        match self {
            LayerLayout::Row => image_width / layers,
            LayerLayout::Column => image_width,
        }
    }

    pub fn layer_height(&self, image_height: u32, layers: u32) -> u32 {
        match self {
            LayerLayout::Row => image_height,
            LayerLayout::Column => image_height / layers,
        }
    }
}

#[derive(Derivative)]
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
    fn img_kind(&self, width: u32, height: u32) -> gfx_hal::image::Kind {
        use gfx_hal::image::Kind::*;
        match self {
            TextureKind::D1 => D1(width * height, 1),
            TextureKind::D1Array => D1(width, height as u16),
            TextureKind::D2 { samples } => D2(width, height, 1, *samples),
            TextureKind::D2Array {
                samples,
                layers,
                layout,
            } => D2(
                layout.layer_width(width, *layers as u32),
                layout.layer_height(height, *layers as u32),
                *layers,
                *samples,
            ),
            TextureKind::D3 { depth, layout } => D3(
                layout.layer_width(width, *depth),
                layout.layer_height(height, *depth),
                *depth,
            ),
            TextureKind::Cube { layout } => D2(
                layout.layer_width(width, 6),
                layout.layer_height(height, 6),
                6,
                1,
            ),
            TextureKind::CubeArray { layers, layout } => D2(
                layout.layer_width(width, *layers as u32 * 6),
                layout.layer_height(height, *layers as u32 * 6),
                layers * 6,
                1,
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

#[derive(Derivative)]
#[derivative(Default)]
pub struct ImageTextureConfig {
    /// Interpret the image as given format.
    /// When `None`, format is determined automatically based on magic bytes.
    /// Automatic method doesn't support TGA format.
    format: Option<ImageFormat>,
    repr: Repr,
    kind: TextureKind,
    #[derivative(Default(value = "gfx_hal::image::Filter::Linear"))]
    filter: gfx_hal::image::Filter,
}

macro_rules! dyn_format {
    ($channel:ty, $size:ty, $repr:expr) => {{
        use pixel::{AsPixel, Pixel};
        match $repr {
            Repr::Unorm => <Pixel<$channel, $size, pixel::Unorm> as AsPixel>::FORMAT,
            Repr::Inorm => <Pixel<$channel, $size, pixel::Inorm> as AsPixel>::FORMAT,
            Repr::Uscaled => <Pixel<$channel, $size, pixel::Uscaled> as AsPixel>::FORMAT,
            Repr::Iscaled => <Pixel<$channel, $size, pixel::Iscaled> as AsPixel>::FORMAT,
            Repr::Uint => <Pixel<$channel, $size, pixel::Uint> as AsPixel>::FORMAT,
            Repr::Int => <Pixel<$channel, $size, pixel::Int> as AsPixel>::FORMAT,
            Repr::Srgb => <Pixel<$channel, $size, pixel::Srgb> as AsPixel>::FORMAT,
        }
    }};
}

pub fn load_from_image(
    bytes: &[u8],
    config: ImageTextureConfig,
) -> Result<TextureBuilder<'static>, failure::Error> {
    use image::{DynamicImage, GenericImageView};

    let image_format = config
        .format
        .map_or_else(|| image::guess_format(bytes), |f| Ok(f))?;
    let image = image::load_from_memory_with_format(bytes, image_format)?;

    let mut builder = TextureBuilder::new();

    let (w, h) = image.dimensions();
    builder.set_data_width(w);
    builder.set_data_height(h);
    builder.set_kind(config.kind.img_kind(w, h));
    builder.set_view_kind(config.kind.view_kind());
    builder.set_filter(config.filter);

    use pixel::{Bgr, Bgra, Rg, Rgb, Rgba, R, _8};
    match image {
        DynamicImage::ImageLuma8(img) => {
            builder.set_raw_data(img.into_vec(), dyn_format!(R, _8, config.repr))
        }
        DynamicImage::ImageLumaA8(img) => {
            builder.set_raw_data(img.into_vec(), dyn_format!(Rg, _8, config.repr))
        }
        DynamicImage::ImageRgb8(img) => {
            builder.set_raw_data(img.into_vec(), dyn_format!(Rgb, _8, config.repr))
        }
        DynamicImage::ImageRgba8(img) => {
            builder.set_raw_data(img.into_vec(), dyn_format!(Rgba, _8, config.repr))
        }
        DynamicImage::ImageBgr8(img) => {
            builder.set_raw_data(img.into_vec(), dyn_format!(Bgr, _8, config.repr))
        }
        DynamicImage::ImageBgra8(img) => {
            builder.set_raw_data(img.into_vec(), dyn_format!(Bgra, _8, config.repr))
        }
    };

    Ok(builder)
}

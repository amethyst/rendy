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

macro_rules! set_data {
    ($builder:expr, $repr:expr, $img:expr) => {
        match $repr {
            Repr::Unorm => $builder.set_data(img_into_vec::<pixel::Unorm, _>($img)),
            Repr::Inorm => $builder.set_data(img_into_vec::<pixel::Inorm, _>($img)),
            Repr::Uscaled => $builder.set_data(img_into_vec::<pixel::Uscaled, _>($img)),
            Repr::Iscaled => $builder.set_data(img_into_vec::<pixel::Iscaled, _>($img)),
            Repr::Uint => $builder.set_data(img_into_vec::<pixel::Uint, _>($img)),
            Repr::Int => $builder.set_data(img_into_vec::<pixel::Int, _>($img)),
            Repr::Srgb => $builder.set_data(img_into_vec::<pixel::Srgb, _>($img)),
        }
    };
}

pub fn load_from_image(
    bytes: &[u8],
    config: ImageTextureConfig,
) -> Result<TextureBuilder<'static>, failure::Error> {
    use image::{DynamicImage, GenericImageView};

    let format = config
        .format
        .map_or_else(|| image::guess_format(bytes), |f| Ok(f))?;
    let image = image::load_from_memory_with_format(bytes, format)?;

    let mut builder = TextureBuilder::new();

    let (w, h) = image.dimensions();
    builder.set_data_width(w);
    builder.set_data_height(h);
    builder.set_kind(config.kind.img_kind(w, h));
    builder.set_view_kind(config.kind.view_kind());
    builder.set_filter(config.filter);

    match image {
        DynamicImage::ImageLuma8(img) => set_data!(builder, config.repr, img),
        DynamicImage::ImageLumaA8(img) => set_data!(builder, config.repr, img),
        DynamicImage::ImageRgb8(img) => set_data!(builder, config.repr, img),
        DynamicImage::ImageRgba8(img) => set_data!(builder, config.repr, img),
        DynamicImage::ImageBgr8(img) => set_data!(builder, config.repr, img),
        DynamicImage::ImageBgra8(img) => set_data!(builder, config.repr, img),
    };

    Ok(builder)
}

// Types that are representing identical memory layout
trait CastPixel<R>: image::Pixel + 'static {
    type Into: pixel::AsPixel;
}

trait IntoChannels<S, T>
where
    S: pixel::ChannelSize,
    T: pixel::ChannelRepr<S>,
{
    type Channels: pixel::PixelRepr<S, T>;
}

macro_rules! map_channels {
    {$($colors:ident => $channels:ident),*,} => {
        $(
            impl<S, T> IntoChannels<S::Size, T> for image::$colors<S>
            where
                S: IntoChannelSize + image::Primitive,
                T: pixel::ChannelRepr<S::Size>,
            {
                type Channels = pixel::$channels;
            }
        )*
    }
}

map_channels! {
    Rgba => Rgba,
    Bgra => Bgra,
    Rgb => Rgb,
    Bgr => Bgr,
    Luma => R,
    LumaA => Rg,
}

impl<T, R> CastPixel<R> for T
where
    R: pixel::ChannelRepr<<<T as image::Pixel>::Subpixel as IntoChannelSize>::Size> + 'static,
    T: IntoChannels<<<T as image::Pixel>::Subpixel as IntoChannelSize>::Size, R>
        + image::Pixel
        + 'static,
    T::Subpixel: IntoChannelSize,
    pixel::Pixel<
        <T as IntoChannels<<<T as image::Pixel>::Subpixel as IntoChannelSize>::Size, R>>::Channels,
        <<T as image::Pixel>::Subpixel as IntoChannelSize>::Size,
        R,
    >: pixel::AsPixel,
{
    type Into = pixel::Pixel<
        <T as IntoChannels<<<T as image::Pixel>::Subpixel as IntoChannelSize>::Size, R>>::Channels,
        <<T as image::Pixel>::Subpixel as IntoChannelSize>::Size,
        R,
    >;
}

trait IntoChannelSize {
    type Size: pixel::ChannelSize;
}

impl IntoChannelSize for u8 {
    type Size = pixel::_8;
}
impl IntoChannelSize for u16 {
    type Size = pixel::_16;
}
impl IntoChannelSize for u32 {
    type Size = pixel::_32;
}
impl IntoChannelSize for u64 {
    type Size = pixel::_64;
}

fn img_into_vec<
    R: pixel::ChannelRepr<<<P as image::Pixel>::Subpixel as IntoChannelSize>::Size>,
    P: CastPixel<R>,
>(
    img: image::ImageBuffer<P, Vec<<P as image::Pixel>::Subpixel>>,
) -> Vec<P::Into>
where
    <P as image::Pixel>::Subpixel: IntoChannelSize,
{
    let len = (img.width() * img.height()) as usize;
    let mut raw = img.into_raw();
    let ptr = raw.as_mut_ptr() as *mut P::Into;

    let pixel_size = std::mem::size_of::<P::Into>();

    // When original vector's capacity is not divisible by new type size,
    // a reallocation is necessary. Otherwise vector cast can be done
    // and ownership can be transferred without copying.
    if (raw.capacity() % pixel_size) == 0 {
        let capacity = raw.capacity() / pixel_size;
        debug_assert!(capacity >= len);
        unsafe {
            let new_vec = Vec::from_raw_parts(ptr, len, capacity);
            std::mem::forget(raw);
            new_vec
        }
    } else {
        unsafe {
            let slice = std::slice::from_raw_parts(ptr, len);
            slice.to_owned()
        }
    }
}
